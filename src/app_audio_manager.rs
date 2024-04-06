use std::{collections::VecDeque, slice, sync::{mpsc::{self, Receiver, Sender}, Arc, Mutex}, thread::{self, JoinHandle}};
use rustfft::{num_complex::Complex, FftPlanner};
use wasapi::*;

use crate::FFT_FREQUENCY;
use crate::common_audio_manager::FftHandler;

struct AudioThread {
    audio_client: AudioClient,
    format: WaveFormat,
    playing: bool,
    handler: FftHandler,
}

impl AudioThread {
    pub fn new(sample_destination: Sender<Vec<(Complex<f32>, f32)>>, fft_planner: Arc<Mutex<FftPlanner<f32>>>) -> Self {
        // Get device and client
        let device = get_default_device(&Direction::Capture).unwrap();
        let mut audio_client = device.get_iaudioclient().unwrap();

        // Get default format
        let format = AudioClient::get_mixformat(&audio_client).unwrap();

        // Gather information about format and client
        let (_def_time, min_time) = audio_client.get_periods().unwrap();

        // Initialize client
        audio_client.initialize_client(
            &format,
            min_time,
            &Direction::Capture,
            &ShareMode::Shared,
            true,
        ).unwrap();

        // Plan FFT for this format
        let fft = fft_planner.lock().unwrap().plan_fft_forward(format.get_samplespersec() as usize);

        // Create the FFT handler
        let handler = FftHandler::new(sample_destination, format.get_samplespersec(), fft);

        AudioThread { audio_client, format, playing: false, handler }
    }

    pub fn capture_loop(&mut self, receiver: mpsc::Receiver<bool>) {
        // Gather information about client
        let event_handler = self.audio_client.set_get_eventhandle().unwrap();
        let buffer_frame_count = self.audio_client.get_bufferframecount().unwrap();
        let capture_client = self.audio_client.get_audiocaptureclient().unwrap();

        // Gather information about format
        let block_align = self.format.get_blockalign();
        let chunk_size = 44100 / FFT_FREQUENCY as usize;
        let channels = self.format.get_nchannels();
    
        // Create queue for sending samples
        let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
            100 * block_align as usize * (1024 + 2 * buffer_frame_count as usize),
        );
    
        // Block until we first want to start stream
        match receiver.recv() {
            Ok(value) => { if value { self.audio_client.start_stream().unwrap(); self.playing = true; } }
            Err(_) => { return; }
        }

        // Main loop
        loop {
            // If we have enough samples, send a chunk
            while sample_queue.len() > (block_align as usize * chunk_size) {
                // Allocate memory as f32 to ensure correct alignment
                let mut data = vec![0.0; block_align as usize * chunk_size / 4];
    
                // Reinterpret the memory as u8
                let data_ptr = data.as_mut_ptr() as *mut u8;
                let chunk = unsafe { slice::from_raw_parts_mut(data_ptr, block_align as usize * chunk_size) };
    
                // Fill the chunk with samples
                let mut sum: usize = 0;
                for element in chunk.iter_mut() {
                    // We don't want to send samples from all channels
                    if sum % channels as usize == 0 {
                        *element = sample_queue.pop_front().unwrap();
                    }
                    sum += 1;
                }
    
                // Convert back to f32
                let data_ptr = data_ptr as *const f32;
                let final_data = unsafe { slice::from_raw_parts(data_ptr, block_align as usize * chunk_size / 4) };

                // Perform FFT on the data
                self.handler.perform_fft(final_data);
            }
    
            // Read from device to queue
            capture_client.read_from_device_to_deque(block_align as usize, &mut sample_queue).unwrap();
            if event_handler.wait_for_event(1000000).is_err() {
                self.audio_client.stop_stream().unwrap();
                break;
            }
    
            // Check if we should stop the stream
            match receiver.try_recv() {
                Ok(value) => { if !value {self.audio_client.stop_stream().unwrap(); self.playing = false; } }
                // Main thread has disconnected so end here
                Err(mpsc::TryRecvError::Disconnected) => { break; },
                _ => {}
            }

            // If we are stopped, wait for command to start again
            if !self.playing {
                match receiver.recv() {
                    Ok(value) => { if value { self.audio_client.start_stream().unwrap(); self.playing = true; } }
                    // Main thread has disconnected so end here
                    Err(_) => { break; }
                }
            }
        }
    }
}

/// Holds all necessary information for the app audio manager.
pub struct AppAudioManager {
    current_handle: JoinHandle<()>,
    sample_destination: Sender<Vec<(Complex<f32>, f32)>>,
    playing: bool,
    transmit: Sender<bool>,
    fft_planner: Arc<Mutex<FftPlanner<f32>>>,
}

impl AppAudioManager {
    /// Initialises and creates a new app audio manager.
    /// 
    /// # Arguments
    /// 
    /// * `sample_destination` - Is the destination to send the samples for rendering.
    pub fn new(sample_destination: Sender<Vec<(Complex<f32>, f32)>>) -> Self {
        // Create FFT planner
        let fft_planner = Arc::new(Mutex::new(FftPlanner::new()));
        let fft_planner_clone = fft_planner.clone();

        // Communications channel for control over the thread
        let (transmit, receive): (Sender<bool>, Receiver<bool>) = mpsc::channel();
        let sample_destination_clone = sample_destination.clone();

        // Create the audio thread
        let current_handle = thread::Builder::new()
            .name("Capture".to_string())
            .spawn(move || {
                let mut audio_thread = AudioThread::new(sample_destination_clone, fft_planner_clone);
                audio_thread.capture_loop(receive);
            }).unwrap();

        AppAudioManager { current_handle, sample_destination, playing: false, transmit, fft_planner }
    }

    /// Starts the audio stream passing samples to the FFT processor.
    /// 
    /// If the audio thread has died for some reason, it will be revived.
    pub fn start(&mut self) {
        match self.transmit.send(true) {
            Ok(_) => { self.playing = true; }
            // Thread has died for some reason
            Err(_) => { self.revive_thread(); }
        }
    }

    /// Stops the audio stream passing samples to the FFT processor.
    /// 
    /// If the audio thread has died for some reason, it will be revived.
    pub fn stop(&mut self) {
        match self.transmit.send(false) {
            Ok(_) => { self.playing = false; }
            // Thread has died for some reason
            Err(_) => { self.revive_thread(); }
        }
        self.playing = false;
    }

    /// Revives the audio thread if it has died.
    fn revive_thread(&mut self) {
        // Communications channel for control over the thread
        let (transmit, receive): (Sender<bool>, Receiver<bool>) = mpsc::channel();

        let fft_planner = self.fft_planner.clone();
        let sample_destination = self.sample_destination.clone();

        self.current_handle = thread::Builder::new()
            .name("Capture".to_string())
            .spawn(move || {
                let mut audio_thread = AudioThread::new(sample_destination, fft_planner);
                audio_thread.capture_loop(receive);
            }).unwrap();

        self.transmit = transmit;
    }
}