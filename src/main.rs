use std::{ fs::File, sync::{ Arc, Mutex} };
use glow::HasContext;
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
    //let song = File::open("./test/titanium-170190.mp3").unwrap();
    //audio_manager.add_song(song);
    //audio_manager.play();

    let mut textures = imgui::Textures::<glow::Texture>::default();

    let width = 100;
    let height = 100;

    let mut data = Vec::with_capacity(width * height);
    for i in 0..width {
        for j in 0..height {
            data.push(i as u8);
            data.push(j as u8);
            data.push((i + j) as u8);
        }
    }

    let gl = app.glow_context();
    let texture = unsafe { gl.create_texture() }.expect("Unable to create GL texture");

    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_parameter_i32(
            glow::TEXTURE_2D, 
            glow::TEXTURE_MIN_FILTER, 
            glow::LINEAR as _
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as _,
        );
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGB as _,
            width as _,
            height as _,
            0,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            Some(&data),
        )
    }

    let id = textures.insert(texture);

    app.main_loop(move |_, ui| {
        //ui.show_demo_window(&mut true);
        ui.window("Test Texture")
            .size([400.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Hello textures!");
                ui.text("Some generated texture");
                imgui::Image::new(id, [100.0, 100.0]).build(ui);
            });

        // if ui.is_key_pressed(Key::Space) {
        //     if audio_manager.is_paused() {
        //         audio_manager.pause();
        //     } else {
        //         audio_manager.play();
        //     }
        // }

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