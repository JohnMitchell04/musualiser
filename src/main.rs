use std::{ fs::File, sync::{ Arc, Mutex}, borrow::Cow };
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
    let audio_manager = audio_manager::AudioManager::new(shared_samples.clone());

    let fft_renderer = fft_renderer::FftRenderer::new(app.glow_context(), shared_samples, textures);

    app.main_loop(fft_renderer, audio_manager, move |_, ui, fft_renderer, audio_manager| {
        ui.window("Visualisation").size([400.0, 400.0], imgui::Condition::FirstUseEver).build(|| {
            let window_size = ui.content_region_avail();
            fft_renderer.render(window_size);
            imgui::Image::new(fft_renderer.get_texture_id(), window_size).build(ui);
        });

        ui.window("Songs").size([200.0, 200.0], imgui::Condition::FirstUseEver).build(|| {
            let window_size = ui.content_region_avail();

            // Function to create labels from items in list box
            fn label_function<'b>(item: &'b &str) -> Cow<'b, str> {
                Cow::from(*item)
            }

            ui.text("Songs");
            let width_specifier = ui.push_item_width(-1.0);
            // List box of songs
            let list_box = imgui::ListBox::new("##song_list_box");

            // Add all the opened songs
            let items = audio_manager.selected_songs();
            let mut selected_item = 0;

            // Create list box
            imgui::ListBox::build_simple(
                list_box,
                ui,
                &mut selected_item,
                &items,
                &label_function
            );
            width_specifier.end();

            // Update the current song if needed
            audio_manager.update_current_song(items[selected_item], selected_item);

            if ui.button_with_size("Select Songs", [window_size[0], 10.0]) {
                audio_manager.select_songs();
            }
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