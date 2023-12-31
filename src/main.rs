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
        ui.window("Visualisation").size([400.0, 400.0], imgui::Condition::FirstUseEver).build(|| {
            let window_size = ui.content_region_avail();
            fft_renderer.render_fft(window_size);
            imgui::Image::new(fft_renderer.get_texture_id(), window_size).build(ui);
        });

        if ui.is_key_pressed(Key::Space) {
            if !audio_manager.is_paused() {
                audio_manager.pause();
            } else {
                audio_manager.play();
            }
        }
    });
}