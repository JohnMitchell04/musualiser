use std::{
    fs::File,
    io::BufReader,
    sync::{ Arc, Mutex },
    time::Duration, path::PathBuf
};
use rodio::{Decoder, OutputStream, source::Source, Sink, OutputStreamHandle};
use rustfft::{FftPlanner, num_complex::Complex, Fft};
use rfd::FileDialog;

struct FftFilter<I> {
    input: I,
    internal_vector: Vec<Complex<f32>>,
    output_vector: Arc<Mutex<Vec<Complex<f32>>>>,
    counter: u16,
    filter: Arc<dyn Fft<f32>>
}

impl<I> FftFilter<I> {
    /// Returns a reference to the inner source.
    pub fn inner(&self) -> &I {
        &self.input
    }

    /// Returns a mutable reference to the inner source.
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Returns the inner source.
    pub fn into_inner(self) -> I {
        self.input
    }
}

impl<I> Iterator for FftFilter<I>
where I: Source<Item = f32>, {
    type Item = f32;

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
        if self.counter / self.input.channels() == (self.input.sample_rate() / 30) as u16 {
            // Perform FFT and place into our output vector
            self.perform_fft();

            // Reset counter and vector
            self.internal_vector.clear();
            self.counter = 0;
        }

        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> ExactSizeIterator for FftFilter<I> where I: Source<Item = f32> + ExactSizeIterator {}

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
    pub fn new(input: I, output_vector: Arc<Mutex<Vec<Complex<f32>>>>, filter: Arc<dyn Fft<f32>>) -> Self {
        let counter: u16 = 0;
        let internal_vector = Vec::new();

        FftFilter { input, internal_vector, output_vector, counter, filter }
    }

    pub fn perform_fft(&mut self) {
        // Aquire lock for output data
        let mut lock = self.output_vector.lock().unwrap();
        let data = &mut *lock;

        // Copy internal data into output vector to perform FFT in place
        *data = self.internal_vector.clone();

        // Process data
        self.filter.process(data);

        // Only the second half of the data is needed after the FFT
        data.drain(0..(data.len() / 2));
    }
}

pub struct AudioManager {
    sink: Sink,
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sample_destination: Arc<Mutex<Vec<Complex<f32>>>>,
    fft_planner: FftPlanner<f32>,
    currently_playing: String,
    selected_songs: Vec<PathBuf>
}

impl AudioManager {
    pub fn new(sample_destination: Arc<Mutex<Vec<Complex<f32>>>>) -> Self {
        // Try to get the default sound device
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();

        // Create sink
        let sink = Sink::try_new(&stream_handle).unwrap();

        // Create FFT planner
        let fft_planner = FftPlanner::new();

        let currently_playing = String::from("");
        let selected_songs = Vec::new();

        AudioManager { 
            sink,
            _stream,
            _stream_handle: stream_handle,
            sample_destination,
            fft_planner,
            currently_playing,
            selected_songs
        }
    }

    pub fn select_songs(&mut self) {
        // TODO: Deal with errors this could throw
        self.selected_songs = FileDialog::new()
            .add_filter("audio", &["mp3", "wav", ])
            .set_directory("/")
            .pick_files()
            .unwrap();
    }

    pub fn selected_songs(&self) -> Vec<&str> {
        self.selected_songs.iter().map(|path| { path.to_str().unwrap() }).collect()
    }

    pub fn update_current_song(&mut self, song: &str, index: usize) {
        // Already playing
        if song == self.currently_playing { return }

        // Changing song, so clear current song
        self.sink.clear();

        // Add new song
        let file = File::open(&self.selected_songs[index]).unwrap();

        // TODO: Deal with errors

        self.add_song(file);
    }

    pub fn clear_queue(&mut self) {
        self.sink.clear();
    }

    pub fn is_paused(&mut self) -> bool {
        self.sink.is_paused()
    }

    pub fn pause(&mut self) {
        self.sink.pause();
    }

    pub fn play(&mut self) {
        self.sink.play();
    }

    fn add_song(&mut self, song: File) {
        // Read song file
        let reader = BufReader::new(song);

        // Decode file, make pausable and convert samples
        let source = Decoder::new(reader)
            .unwrap()
            .pausable(false)
            .convert_samples();

        // Plan FFT
        let fft = self.fft_planner.plan_fft_forward((source.sample_rate() / 30) as usize);

        // Add FFT filter and add to sink
        let filter = FftFilter::new(source, self.sample_destination.clone(), fft);
        self.sink.append(filter);
    }
}

