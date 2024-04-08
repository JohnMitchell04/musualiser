use std::{collections::VecDeque, sync::{mpsc::{self, Receiver, Sender, TryRecvError}, Arc, Condvar, Mutex}, thread::{self, JoinHandle}};
use rustfft::{num_complex::Complex, FftPlanner};
use wasapi::*;

use crate::FFT_FREQUENCY;
use crate::common_audio_manager::FftHandler;

// TODO: Maybe set thread priority to high

struct AudioThread {
    device_id: String,
    audio_client: AudioClient,
    format: WaveFormat,
    playing: Arc<(Mutex<bool>, Condvar)>,
    device_change: Sender<bool>,
    handler: FftHandler,
}

impl AudioThread {
    pub fn new(sample_destination: Sender<Vec<(Complex<f32>, f32)>>, fft_planner: Arc<Mutex<FftPlanner<f32>>>, playing: Arc<(Mutex<bool>, Condvar)>, device_change: Sender<bool>) -> Self {
        // Get device and client
        let device = get_default_device(&Direction::Render).unwrap();
        let device_id = device.get_id().unwrap();
        let mut audio_client = device.get_iaudioclient().unwrap();

        // Set desired format
        let format = WaveFormat::new(32, 32, &SampleType::Float, 44100, 1, None);

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
        let fft = fft_planner.lock().unwrap().plan_fft_forward(format.get_samplespersec() as usize / FFT_FREQUENCY as usize);

        // Create the FFT handler
        let handler = FftHandler::new(sample_destination, format.get_samplespersec(), fft);

        AudioThread { device_id, audio_client, format, playing, handler, device_change }
    }

    pub fn capture_loop(&mut self,) {
        // Gather information about client
        let event_handler = self.audio_client.set_get_eventhandle().unwrap();
        let buffer_frame_count = self.audio_client.get_bufferframecount().unwrap();
        let capture_client = self.audio_client.get_audiocaptureclient().unwrap();

        // Gather information about format
        let block_align = self.format.get_blockalign();
        let chunk_size = self.format.get_samplespersec() as usize / FFT_FREQUENCY as usize;
    
        // Create queue for sending samples
        let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
            100 * block_align as usize * (1024 + 2 * buffer_frame_count as usize),
        );

        // Block until we first want to start stream
        let (lock, cvar) = &*self.playing;
        let mut playing = lock.lock().unwrap();
        while !*playing {
            playing = cvar.wait(playing).unwrap();
        }
        drop(playing);

        self.audio_client.start_stream().unwrap();

        // Allocate memory as f32 to ensure correct alignment
        let mut data = Vec::with_capacity(chunk_size);

        // Main loop
        loop {
            while !sample_queue.is_empty() {
                // Temporary buffer for samples
                let mut temp = Vec::with_capacity(std::cmp::min(sample_queue.len(), (chunk_size - data.len()) * 4));

                // Try to fill the chunk with samples
                while sample_queue.len() > 0 && temp.len() < temp.capacity() {
                    temp.push(sample_queue.pop_front().unwrap());
                }

                // Convert to f32
                for chunk in temp.chunks(4) {
                    let mut sample = [0; 4];
                    sample.copy_from_slice(chunk);
                    let sample = f32::from_ne_bytes(sample);
                    data.push(sample);
                }

                // Ensure the chunk is filled before further processing
                if data.len() != chunk_size {
                    break;
                }

                // Perform FFT on the data
                self.handler.perform_fft(data.as_slice());

                // Remove the first quarter of the samples, this is done to smooth the visualisation by creating overlapping windows
                data.drain(0..data.len() / 4);
            }
    
            // Read from device to queue
            capture_client.read_from_device_to_deque(block_align as usize, &mut sample_queue).unwrap();
            if event_handler.wait_for_event(1000000).is_err() {
                self.audio_client.stop_stream().unwrap();
                break;
            }

            // If this device is no longer active, we should kill this thread and a new one should be started with the new device
            let test = get_default_device(&Direction::Render).unwrap().get_id().unwrap();
            if self.device_id != test {
                let _ = self.device_change.send(true);
                return;
            }
    
            // Check if we should stop the stream and if so, wait for command to start again
            let (lock, cvar) = &*self.playing;
            let mut playing = lock.lock().unwrap();

            if !*playing {
                self.audio_client.stop_stream().unwrap();
                while !*playing {
                    playing = cvar.wait(playing).unwrap();
                }

                self.audio_client.start_stream().unwrap()
            }
        }
    }
}

/// Holds all necessary information for the app audio manager.
pub struct AppAudioManager {
    current_handle: JoinHandle<()>,
    sample_destination: Sender<Vec<(Complex<f32>, f32)>>,
    playing: Arc<(Mutex<bool>, Condvar)>,
    fft_planner: Arc<Mutex<FftPlanner<f32>>>,
    device_change: Receiver<bool>,
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

        // Condvar for thread control
        let playing = Arc::new((Mutex::new(false), Condvar::new()));
        let playing_clone = playing.clone();

        // Communications channel for reviving thread on device change
        let (transmit, device_change): (Sender<bool>, Receiver<bool>) = mpsc::channel();

        let sample_destination_clone = sample_destination.clone();

        // Create the audio thread
        let current_handle = thread::Builder::new()
            .name("Capture".to_string())
            .spawn(move || {
                let mut audio_thread = AudioThread::new(sample_destination_clone, fft_planner_clone, playing_clone, transmit);
                audio_thread.capture_loop();
            }).unwrap();

        AppAudioManager { current_handle, sample_destination, playing, fft_planner, device_change }
    }

    /// Starts the audio stream passing samples to the FFT processor.
    /// 
    /// If the audio thread has died for some reason, it will be revived.
    pub fn start(&mut self) {
        let (lock, cvar) = &*self.playing;
        let mut playing = lock.lock().unwrap();
        *playing = true;
        cvar.notify_all();
    }

    /// Stops the audio stream passing samples to the FFT processor.
    /// 
    /// If the audio thread has died for some reason, it will be revived.
    pub fn stop(&mut self) {
        let (lock, cvar) = &*self.playing;
        let mut playing = lock.lock().unwrap();
        *playing = false;
        cvar.notify_all();
    }

    /// Checks if the audio thread is still alive.
    /// 
    /// If the audio thread has died for some reason, it will be revived in its previous state.
    pub fn check_device(&mut self) {
        match self.device_change.try_recv() {
            Ok(value) => { if value { self.revive_thread(); } }
            Err(TryRecvError::Empty) => {}
            // Thread has died and not informed us for some reason
            Err(TryRecvError::Disconnected) => { self.revive_thread(); }
        }
    }

    /// Returns whether the audio is currently playing.
    pub fn is_playing(&self) -> bool {
        let (lock, _) = &*self.playing;
        let playing = lock.lock().unwrap();
        *playing
    }

    /// Revives the audio thread if it has died.
    fn revive_thread(&mut self) {
        let fft_planner = self.fft_planner.clone();
        let sample_destination = self.sample_destination.clone();
        let playing = self.playing.clone();

        // Communications channel for reviving thread on device change
        let (transmit, device_change): (Sender<bool>, Receiver<bool>) = mpsc::channel();
        self.device_change = device_change;

        self.current_handle = thread::Builder::new()
            .name("Capture".to_string())
            .spawn(move || {
                let mut audio_thread = AudioThread::new(sample_destination, fft_planner, playing, transmit);
                audio_thread.capture_loop();
            }).unwrap();
    }
}