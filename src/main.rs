use std::{ sync::{ Arc, Mutex}, borrow::Cow };
use imgui::{Key, Ui};

mod application;
mod audio_manager;
mod fft_renderer;

fn main() {
    // Initialise app and helpers
    let (app, textures) = application::initialise_appplication();
    let shared_samples = Arc::new(Mutex::new(Vec::new())); // TODO: Look into removing
    let audio_manager = audio_manager::AudioManager::new(shared_samples.clone());
    let fft_renderer = fft_renderer::FftRenderer::new(app.glow_context(), shared_samples, textures);

    // Run app
    app.main_loop(fft_renderer, audio_manager, application_loop);
}

/// Function passed to the main application loop detailing the UI
///
/// # Arguments
///
/// * `ui` - The ImGui UI class that allows creating the UI
/// * `renderer` - The FFT Renderer class that creates the visualisation from audio data 
/// * `audio_manager` - The Audio Manager class that handles playing audio
fn application_loop(_: &mut bool, ui: &mut Ui, renderer: &mut fft_renderer::FftRenderer, audio_manager: &mut audio_manager::AudioManager) {
    // Window for displaying the visualisation
    ui.window("Visualisation").size([400.0, 400.0], imgui::Condition::FirstUseEver).build(|| {
        let window_size = ui.content_region_avail();
        renderer.render(window_size);
        imgui::Image::new(renderer.get_texture_id(), window_size).build(ui);
    });

    // Window or controlling currently selected and open songs
    ui.window("Songs").size([200.0, 200.0], imgui::Condition::FirstUseEver).build(|| {
        ui.text("Songs");
        let width_specifier = ui.push_item_width(-1.0);
        let list_box = imgui::ListBox::new("##song_list_box");

        // Add all currently opened songs and get selected song
        let items = audio_manager.opened_songs();
        let mut selected_item = audio_manager.selected_song_index();

        // Build list box
        fn label_function<'b>(item: &'b String) -> Cow<'b, str> { Cow::from(item.as_str()) }
        let window_size = ui.content_region_avail();
        imgui::ListBox::build_simple(
            list_box,
            ui,
            &mut selected_item,
            &items,
            &label_function
        );

        width_specifier.end();

        // Update the current song
        if items.len() != 0 {
            audio_manager.update_current_song(&items[selected_item], selected_item);
        }

        // Add songs
        if ui.button_with_size("Select Songs", [window_size[0], 10.0]) {
            audio_manager.open_songs();
        }
    });

    // Pause/Play currently selected song
    if ui.is_key_pressed(Key::Space) {
        if !audio_manager.is_paused() {
            audio_manager.pause();
        } else {
            audio_manager.play();
        }
    }
}