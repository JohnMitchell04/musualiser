use std::{ fs::File, io::BufReader, sync::{ Arc, Mutex }, time::Duration, path::PathBuf };
use rodio::{ Decoder, OutputStream, source::Source, Sink, OutputStreamHandle };
use rustfft::{ FftPlanner, num_complex::Complex, Fft };

use crate::FFT_FREQUENCY;

/// Holds all information needed for the FFT.
struct FftFilter<I> {
    input: I,
    internal_vector: Vec<Complex<f32>>,
    output_vector: Arc<Mutex<Vec<(Complex<f32>, f32)>>>,
    counter: u16,
    filter: Arc<dyn Fft<f32>>
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
            self.internal_vector.push(Complex { re: sample, im: 0.0});
        }

        self.counter += 1;

        // If we have enough samples to perform an FFT, then do so
        if self.internal_vector.len() == (self.input.sample_rate() / FFT_FREQUENCY) as usize {
            // Perform FFT and place into our output vector
            self.perform_fft();

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
    /// * `output_vector` - Is the thread safe vector to place processed FFT data into.
    /// 
    /// * `filter` - Is the FFT algorithm to use.
    pub fn new(input: I, output_vector: Arc<Mutex<Vec<(Complex<f32>, f32)>>>, filter: Arc<dyn Fft<f32>>) -> Self {
        let counter: u16 = 0;
        let internal_vector = Vec::with_capacity(input.sample_rate() as usize / FFT_FREQUENCY as usize);

        FftFilter { input, internal_vector, output_vector, counter, filter }
    }

    /// Performs the FFT on the internal vector and places the result into the output vector.
    pub fn perform_fft(&mut self) {
        // Copy data into temporary array for FFT process
        let mut temp = self.internal_vector.clone();

        // Perform FFT
        self.filter.process(&mut temp);
        temp.drain((temp.len() / 2)..temp.len());
        
        // Calculate the frequency for each bin
        let step = self.sample_rate() as f32 / temp.len() as f32;
        let mut transformed_data = Vec::new();
        for (index, amp) in temp.iter().enumerate() {
            let fr = index as f32 * step;
            transformed_data.push((*amp, fr));
        }

        // Remove inaudible frequencies
        transformed_data.retain(|&x| x.1 > 20.0 && x.1 < 20000.0);

        let mut lock = self.output_vector.lock().unwrap();
        let data = &mut *lock;
        *data = transformed_data.clone();
    }
}

/// Holds all necessary information for the audio manager.
pub struct FileAudioManager {
    sink: Sink,
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sample_destination: Arc<Mutex<Vec<(Complex<f32>, f32)>>>,
    fft_planner: FftPlanner<f32>,
    opened_songs: Vec<PathBuf>,
    selected_song_idx: usize
}

impl FileAudioManager {
    /// Initialises and creates a new audio manager.
    /// 
    /// # Arguments
    /// 
    /// * `sample_destination`- Is the thread safe vector to place processed FFT data into,
    pub fn new(sample_destination: Arc<Mutex<Vec<(Complex<f32>, f32)>>>) -> Self {
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
    }

    /// Returns the vector of songs that have been opened.
    pub fn opened_songs(&self) -> Vec<String> {
        self.opened_songs.iter().map(|path| { String::from(path.to_str().unwrap()) }).collect()
    }

    /// Returns the currently playing song index.
    pub fn selected_song_index(&self) -> usize {
        self.selected_song_idx
    }

    /// Chnage the currently selected song. Will open and play the new audio.
    /// 
    /// # Arguments
    ///
    /// * `index` - Is the index of the song to change to.
    pub fn change_current_song(&mut self, index: usize) {
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
