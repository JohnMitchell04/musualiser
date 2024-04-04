use std::borrow::Cow;
use imgui::{Key, Ui};
use rfd::FileDialog;

mod application;
mod file_audio_manager;
mod app_audio_manager;
mod fft_renderer;

const FFT_FREQUENCY: u32 = 5;

fn main() {
    // Initialise app and helpers
    let app = application::Application::new();

    // Run app
    app.main_loop(application_loop);
}

/// Function passed to the main application loop detailing the UI.
///
/// # Arguments
///
/// * `ui` - Is the ImGui UI class that provides access to UI widgets and functions.
/// 
/// * `renderer` - Is the FFT Renderer class that creates the visualisation from audio data .
/// 
/// * `audio_manager` - Is the Audio Manager class that handles playing audio.
fn application_loop(_: &mut bool, ui: &mut Ui, renderer: &mut fft_renderer::FftRenderer, audio_manager: &mut file_audio_manager::FileAudioManager) {
    // Window for displaying the visualisation
    ui.window("Visualisation").size([400.0, 400.0], imgui::Condition::FirstUseEver).title_bar(false).build(|| {
        let draw_list = ui.get_window_draw_list();
        let size = ui.content_region_avail();
        let origin = ui.cursor_screen_pos();
        renderer.render(draw_list, size, origin);
    });

    // Window for controlling currently selected and open songs
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
        imgui::ListBox::build_simple(list_box, ui, &mut selected_item, &items, &label_function);

        width_specifier.end();

        // Update the current song
        if items.len() != 0 {
            audio_manager.change_current_song(selected_item);
        }

        // Open file dialogue for the user to select a song
        if ui.button_with_size("Select Songs", [window_size[0], 10.0]) {
            // TODO: Deal with errors this could throw
            let opened_songs = FileDialog::new()
                .add_filter("audio", &["mp3", "wav", ])
                .set_directory("/")
                .pick_files()
                .unwrap();

            audio_manager.update_open_songs(opened_songs);
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