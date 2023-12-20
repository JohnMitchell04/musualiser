use std::{
    fs::File,
    io::BufReader,
    sync::Arc,
    time::Duration
};
use rodio::{Decoder, OutputStream, source::Source, Sink};
use imgui::Key;

mod initialisation;

struct FftFilter<I> {
    input: I,
    vector: Arc<Vec<f32>>
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

fn fft_filter<I>(input: I, vector: Arc<Vec<f32>>) -> FftFilter<I>
where I: Source<Item = f32>, {
    FftFilter { input, vector }
} 

fn main() {
    // Create our application
    let app = initialisation::initialise_appplication();

    // Try to get the default sound device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    let samples = Vec::new();
    let shared_samples = Arc::new(samples);
    
    // Load file
    let file = BufReader::new(File::open("./test/titanium-170190.mp3").unwrap());
    let source = Decoder::new(file).unwrap();
    let test = source.pausable(false);
    let filter = fft_filter(test.convert_samples(), shared_samples);
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.append(filter);

    app.main_loop(move |_, ui| {
        ui.show_demo_window(&mut true);

        if ui.is_key_pressed(Key::Space) {
            if !sink.is_paused() {
                sink.pause();
            } else {
                sink.play();
            }
        }
    });
}