use std::borrow::Cow;
use imgui::{Key, Ui};
use rfd::FileDialog;

mod application;
mod common_audio_manager;
mod file_audio_manager;
mod app_audio_manager;
mod fft_renderer;

use fft_renderer::FftRenderer;
use file_audio_manager::FileAudioManager;
use app_audio_manager::AppAudioManager;

/// This is number of "full" FFTs to perform per second, in order to
/// smooth the visualisation, a windowing of 25% is used which means the
/// number of FFTs calculated is actually 4 times this.
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
/// * `file_audio_manager` - Is the Audio Manager class that handles playing audio from files.
fn application_loop(_: &mut bool, ui: &mut Ui, renderer: &mut FftRenderer, file_audio_manager: &mut FileAudioManager, app_audio_manager: &mut AppAudioManager) {
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

        // App audio
        let app_audio = app_audio_manager.is_playing();
        let mut value = app_audio;
        if ui.checkbox("App Audio", &mut value) {
            if app_audio {
                app_audio_manager.stop();
            } else {
                app_audio_manager.start();
            }
        }

        // Create dropdown of applications
        if value {
            let width_specifier = ui.push_item_width(-1.0);
            let list_box = imgui::ListBox::new("##source_list_box");

            // Add all currently opened applications and get selected application
            let items = app_audio_manager.opened_applications();
            let names: Vec<String> = items.iter().map(|item| item.0.clone()).collect();
            let curr_pid = app_audio_manager.current_pid();
            let mut index = if let Some(curr_pid) = curr_pid {
                items.iter().enumerate().position(|(_, (_, pid))| curr_pid == *pid).unwrap()
            } else {
                0
            };

            // Build list box
            fn label_function<'b>(item: &'b String) -> Cow<'b, str> { Cow::from(item.as_str()) }
            imgui::ListBox::build_simple(list_box, ui, &mut index, &names, &label_function);

            width_specifier.end();

            // Update app audio manager if needed
            app_audio_manager.update(items[index].clone());
        }

        // Only allow the user to select file audio if app audio is not playing
        if !(app_audio) {
            let width_specifier = ui.push_item_width(-1.0);
            let list_box = imgui::ListBox::new("##song_list_box");

            // Add all currently opened songs and get selected song
            let items = file_audio_manager.opened_songs();
            let mut selected_item = file_audio_manager.selected_song_index();

            // Build list box
            fn label_function<'b>(item: &'b String) -> Cow<'b, str> { Cow::from(item.as_str()) }
            let window_size = ui.content_region_avail();
            imgui::ListBox::build_simple(list_box, ui, &mut selected_item, &items, &label_function);

            width_specifier.end();

            // Update the current song
            if items.len() != 0 {
                file_audio_manager.change_current_song(selected_item);
            }

            // Open file dialogue for the user to select a song
            if ui.button_with_size("Select Songs", [window_size[0], 10.0]) {
                // TODO: Deal with errors this could throw
                let opened_songs = FileDialog::new()
                    .add_filter("audio", &["mp3", "wav", ])
                    .set_directory("/")
                    .pick_files()
                    .unwrap();

                file_audio_manager.update_open_songs(opened_songs);

                // Reset the selected song
                file_audio_manager.change_current_song(0);
            }   
        }
    });

    // Pause/Play currently selected song
    if ui.is_key_pressed(Key::Space) {
        if !file_audio_manager.is_paused() {
            file_audio_manager.pause();
        } else {
            file_audio_manager.play();
        }
    }
}