use std::{ fs::File, io::BufReader, path::PathBuf, sync::{ mpsc::Sender, Arc }, time::Duration };
use rodio::{ Decoder, OutputStream, source::Source, Sink, OutputStreamHandle };
use rustfft::{ FftPlanner, num_complex::Complex, Fft };

use crate::FFT_FREQUENCY;
use crate::common_audio_manager::FftHandler;

// TODO: Look into using rodio's buffer to handle audio data

/// Holds all information needed for the FFT filter over a Rodio stream.
struct FftFilter<I> {
    input: I,
    internal_vector: Vec<f32>,
    counter: u16,
    handler: FftHandler,
}

impl<I> Iterator for FftFilter<I>
where I: Source<Item = f32>, {
    type Item = f32;

    /// Handle each sample in the audio data and perform an FFT when enough samples have been collected.
    fn next(&mut self) -> Option<f32> {
        let sample = match self.input.next() {
            None => return None,
            Some(s) => s,
        };

        // Ensure we only get samples from the first channel
        if self.counter % self.input.channels() == 0 {
            self.internal_vector.push(sample);
        }

        self.counter += 1;

        // If we have enough samples to perform an FFT, then do so
        if self.internal_vector.len() == (self.input.sample_rate() / FFT_FREQUENCY) as usize {
            self.handler.perform_fft(self.internal_vector.as_slice());

            // Remove the first quarter of the samples, this is done to smooth the visualisation by creating overlapping windows
            self.internal_vector.drain(0..self.internal_vector.len() / 4);
            self.counter = self.internal_vector.len() as u16;
        }

        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> ExactSizeIterator for FftFilter<I> where I: Source<Item = f32> + ExactSizeIterator {}

// Required for rodio Sink
impl<I> Source for FftFilter<I>
where I: Source<Item = f32>, {
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.input.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
} 

impl <I> FftFilter<I>
where I: Source<Item = f32>, {
    /// Return a new FFT filter.
    /// 
    /// # Arguments
    /// 
    /// * `input` - Is the audio source to perform the FFT on.
    /// 
    /// * `sample_destination` - Is the sender to the renderer.
    /// 
    /// * `filter` - Is the FFT algorithm to use.
    pub fn new(input: I, sample_destination: Sender<Vec<(Complex<f32>, f32)>>, filter: Arc<dyn Fft<f32>>) -> Self {
        let counter: u16 = 0;
        let internal_vector = Vec::with_capacity(input.sample_rate() as usize / FFT_FREQUENCY as usize);
        let handler = FftHandler::new(sample_destination, input.sample_rate(), filter);

        FftFilter { input, internal_vector, counter, handler}
    }
}

/// Holds all necessary information for the file audio manager.
pub struct FileAudioManager {
    sink: Sink,
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sample_destination: Sender<Vec<(Complex<f32>, f32)>>,
    fft_planner: FftPlanner<f32>,
    opened_songs: Vec<PathBuf>,
    selected_song_idx: usize
}

impl FileAudioManager {
    /// Initialises and creates a new audio manager to handle file playback and FFT processing.
    /// 
    /// # Arguments
    /// 
    /// * `sample_destination`- Is the sender to the renderer.
    pub fn new(sample_destination: Sender<Vec<(Complex<f32>, f32)>>) -> Self {
        // Initialise rodio
        let (_stream, stream_handle) = OutputStream::try_default().expect("Failed to get audio output device: ");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink: ");

        let fft_planner = FftPlanner::new();
        let opened_songs = Vec::new();
        let selected_song_idx = usize::MAX;

        FileAudioManager { sink, _stream, _stream_handle: stream_handle, sample_destination, fft_planner, opened_songs, selected_song_idx }
    }

    /// Update list of currently opened songs.
    pub fn update_open_songs(&mut self, opened_songs: Vec<PathBuf>) {
        self.opened_songs = opened_songs;
        self.opened_songs.insert(0, PathBuf::from("Stop"));
    }

    /// Returns the vector of songs that have been opened.
    pub fn opened_songs(&self) -> Vec<String> {
        self.opened_songs.iter().map(|path| { String::from(path.to_str().unwrap()) }).collect()
    }

    /// Returns the currently playing song index.
    pub fn selected_song_index(&self) -> usize {
        self.selected_song_idx
    }

    /// Change the currently selected song. Will open and play the new audio.
    /// 
    /// # Arguments
    ///
    /// * `index` - Is the index of the song to change to.
    pub fn change_current_song(&mut self, index: usize) {
        if index == 0 {
            self.clear_queue();
            self.selected_song_idx = index;
            return;
        }

        if index == self.selected_song_idx { return }

        // Changing song, so clear sink and update currently playing
        self.clear_queue();
        self.selected_song_idx = index;

        let file = File::open(&self.opened_songs[index]).unwrap();

        // TODO: Deal with errors

        self.add_song(file);
        self.play();
    }

    /// Clears all audio in the current sink.
    pub fn clear_queue(&mut self) {
        self.sink.clear();
    }

    /// Returns whether the sink is paused.
    pub fn is_paused(&mut self) -> bool {
        self.sink.is_paused()
    }

    /// Pauses the audio sink.
    pub fn pause(&mut self) {
        self.sink.pause();
    }

    /// Plays the audio sink.
    pub fn play(&mut self) {
        self.sink.play();
    }

    /// Adds a specified audio file to the audio manager, while applying necessary filters and converting data.
    /// 
    /// # Arguments
    /// 
    /// * `song` - Is the file object of the audio being added.
    fn add_song(&mut self, song: File) {
        let reader = BufReader::new(song);
        let source = Decoder::new(reader)
            .unwrap()
            .pausable(false)
            .convert_samples();

        // Plan FFT algorithm to use
        let fft = self.fft_planner.plan_fft_forward((source.sample_rate() / FFT_FREQUENCY) as usize);

        // Apply FFT filter to song and add to sink
        let filter = FftFilter::new(source, self.sample_destination.clone(), fft);
        self.sink.append(filter);
    }
}
