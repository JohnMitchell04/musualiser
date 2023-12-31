use std::{ fs::File, sync::{ Arc, Mutex} };
use imgui::Key;

mod application;
mod audio_manager;
mod fft_renderer;

fn main() {
    // Create our application
    let (app, textures) = application::initialise_appplication();

    // Create shared data for audio manager and FFT renderer
    let shared_samples = Arc::new(Mutex::new(Vec::new()));

    // Create audio manager
    let mut audio_manager = audio_manager::AudioManager::new(shared_samples.clone());

    // Add test song
    let song = File::open("./test/titanium-170190.mp3").unwrap();
    audio_manager.add_song(song);
    audio_manager.play();

    let fft_renderer = fft_renderer::FftRenderer::new(app.glow_context(), shared_samples, textures);

    app.main_loop(fft_renderer, move |_, ui, fft_renderer| {
        ui.show_demo_window(&mut true);
        ui.window("Test Texture")
            .size([400.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Hello textures!");
                ui.text("Some generated texture");
                fft_renderer.render_fft();
                imgui::Image::new(fft_renderer.get_texture_id(), [100.0, 100.0]).build(ui);
            });

        if ui.is_key_pressed(Key::Space) {
            if !audio_manager.is_paused() {
                audio_manager.pause();
            } else {
                audio_manager.play();
            }
        }

        // // If an FFT has been calculated, display it
        // let mut lock = shared_samples.lock().unwrap();
        // if !(*lock).is_empty() {
        //     // Normalise data
        //     let mut normalised: Vec<f64> = (*lock)
        //         .iter()
        //         .map(|x| (x.norm() as f64 / (*lock).len() as f64))
        //         .collect();

        //     // Scale data between 0 and 1
        //     let largest = normalised.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
        //     normalised = normalised.iter().map(|x| (x / largest)).collect();

        //     // Draw data


        //     // Clear output vector
        //     (*lock).clear();
        // }
    });
}