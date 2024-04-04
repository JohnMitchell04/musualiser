use std::{thread, collections::VecDeque, slice};
use wasapi::*;

use crate::FFT_FREQUENCY;

// Capture loop, capture samples and send in chunks of "chunksize" frames to channel
fn capture_loop() {
    // Get device and client
    let device = get_default_device(&Direction::Capture).unwrap();
    let mut audio_client = device.get_iaudioclient().unwrap();

    // Set desired format
    let desired_format = WaveFormat::new(32, 32, &SampleType::Float, 44100, 1, None);
    let chunksize = 44100 / FFT_FREQUENCY as usize;

    // Gather information about format and client
    let blockalign = desired_format.get_blockalign();
    let (_def_time, min_time) = audio_client.get_periods().unwrap();

    // Initialize client
    audio_client.initialize_client(
        &desired_format,
        min_time,
        &Direction::Capture,
        &ShareMode::Shared,
        true,
    ).unwrap();

    // Gather information about client
    let h_event = audio_client.set_get_eventhandle().unwrap();
    let buffer_frame_count = audio_client.get_bufferframecount().unwrap();
    let capture_client = audio_client.get_audiocaptureclient().unwrap();

    // Create queue for sending samples
    let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
        100 * blockalign as usize * (1024 + 2 * buffer_frame_count as usize),
    );

    // Start the client stream
    audio_client.start_stream().unwrap();
    loop {
        // If we have enough samples, send a chunk
        while sample_queue.len() > (blockalign as usize * chunksize) {
            // Allocate memory as f32 to ensure correct alignment
            let mut data = vec![0.0; blockalign as usize * chunksize / 4];

            // Reinterpret the memory as u8
            let data_ptr = data.as_mut_ptr() as *mut u8;
            let chunk = unsafe { slice::from_raw_parts_mut(data_ptr, blockalign as usize * chunksize) };

            // Fill the chunk with samples
            for element in chunk.iter_mut() {
                *element = sample_queue.pop_front().unwrap();
            }

            // Convert back to f32
            let data_ptr = data_ptr as *const f32;
            let final_data = unsafe { slice::from_raw_parts(data_ptr, blockalign as usize * chunksize / 4) };
        }

        // Read from device to queue
        capture_client.read_from_device_to_deque(blockalign as usize, &mut sample_queue).unwrap();
        if h_event.wait_for_event(1000000).is_err() {
            audio_client.stop_stream().unwrap();
            break;
        }
    }
}

/// Holds all necessary information for the audio manager.
pub struct AppAudioManager {
    _handle: thread::JoinHandle<()>,
}

impl AppAudioManager {
    pub fn new() -> Self {
        // Capture
        let _handle = thread::Builder::new()
            .name("Capture".to_string())
            .spawn(move || {
                // Initialise COM
                initialize_mta().unwrap();
                capture_loop();
            }).unwrap();

        AppAudioManager { _handle }
    }
}