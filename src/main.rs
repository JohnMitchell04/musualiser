use std::{fs::File, sync::{ Arc, Mutex}};
use imgui::Key;

mod application;
mod audio_manager;

fn main() {
    // Create our application
    let app = application::initialise_appplication();

    let samples = Vec::new();
    let mutex = Mutex::new(samples);
    let shared_samples = Arc::new(mutex);

    // Create audio manager
    let mut audio_manager = audio_manager::AudioManager::new(shared_samples.clone());

    // Add test song
    let song = File::open("./test/titanium-170190.mp3").unwrap();
    audio_manager.add_song(song);
    audio_manager.play();

    app.main_loop(move |_, ui| {
        ui.show_demo_window(&mut true);

        if ui.is_key_pressed(Key::Space) {
            if audio_manager.is_paused() {
                audio_manager.pause();
            } else {
                audio_manager.play();
            }
        }

        // If an FFT has been calculated, display it
        let mut lock = shared_samples.lock().unwrap();
        if !(*lock).is_empty() {
            // Normalise data

            // Draw data

            // Clear output vector
            (*lock).clear();
        }
    });
}