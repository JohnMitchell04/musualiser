use std::sync::{mpsc::Sender, Arc};
use rustfft::{num_complex::Complex, Fft};

/// Holds all information needed for the calculating the FFT and sending the data to its destination.
pub struct FftHandler {
    sample_destination: Sender<Vec<(Complex<f32>, f32)>>,
    sample_rate: u32,
    fft: Arc<dyn Fft<f32>>,
}

impl FftHandler {
    /// Create a new FFT handler.
    pub fn new(sample_destination: Sender<Vec<(Complex<f32>, f32)>>, sample_rate: u32, fft: Arc<dyn Fft<f32>>) -> Self {
        FftHandler { sample_destination, sample_rate, fft }
    }

    /// Performs the FFT on the provided data and sends the FFT data to the renderer.
    /// 
    /// # Arguments 
    ///
    /// * `data` - Is the audio data to perform the FFT on.
    pub fn perform_fft(&mut self, data: &[f32]) {
        // Create vec for processed data
        let mut processed_data = data.iter().map(|&x| Complex::new(x, 0.0)).collect::<Vec<Complex<f32>>>();

        // Perform FFT
        self.fft.process(&mut processed_data);
        processed_data.drain((processed_data.len() / 2)..processed_data.len());
        
        // Calculate the frequency for each bin
        let step = self.sample_rate as f32 / processed_data.len() as f32;
        let mut transformed_data = Vec::new();
        for (index, amp) in processed_data.iter().enumerate() {
            let fr = index as f32 * step;
            transformed_data.push((*amp, fr));
        }

        // Remove inaudible frequencies
        transformed_data.retain(|&x| x.1 > 20.0 && x.1 < 20000.0);

        // Send data to visualisation renderer, this should always succeed if our program is still running
        let _ = self.sample_destination.send(transformed_data);
    }
}