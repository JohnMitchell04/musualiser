use std::sync::{ Arc, Mutex };
use rustfft::num_complex::Complex;
use imgui::{DrawListMut, ImColor32};

/// Holds all necessary information for the visualisation renderer.
pub struct FftRenderer {
    input_data: Arc<Mutex<Vec<(Complex<f32>, f32)>>>,
    current_render_data: Vec<[f32; 2]>,
    current_size: [f32; 2]
}

impl FftRenderer {
    /// Initialises and creates a new visualisation renderer.
    /// 
    /// # Arguments
    /// 
    /// * `glow_context` - Is the OpenGL context.
    /// 
    /// * `input_data` - Is the Fourier transformed audio input data.
    /// 
    /// * `textures` - Is the texture mapping.
    pub fn new(input_data: Arc<Mutex<Vec<(Complex<f32>, f32)>>>) -> Self {
        let current_size = [0.0, 0.0];
        let current_render_data = Vec::new();

        FftRenderer { input_data, current_render_data, current_size }
    }

    /// Handle rendering, if the size of the window hasn't changed or the data is the same this is unecessary and skipped.
    /// 
    /// # Arguments
    /// 
    /// * `size` - Is the size of the render window.
    pub fn render(&mut self, draw_list: DrawListMut<'_>, size: [f32; 2]) {
        if self.current_size != size {
            // If the size of the window has changed, the data needs to be recalculated
            self.resize(size);
        }

        self.data_preprocess();

        // TODO: Find out why we are not getting 100 chunks
        // TODO: Think about interpolating with splines and then drawing with bezier curves for a smoother visualisation

        // Draw bezier curves for the visualisation
        for set in self.current_render_data.windows(4).step_by(3) {
            if set.len() < 4 {
                break;
            }
            draw_list.add_bezier_curve(
                set[0],
                set[1],
                set[2],
                set[3],
                ImColor32::from_rgba(255, 255, 255, 255)
            ).build();
        }
    }

    // /// Create the visualisation for the audio data.
    // pub fn render_fft(&mut self) {
    //     let width = self.current_size[0] as usize;
    //     let height = self.current_size[1] as usize;

    //     // Draw black background
    //     let mut draw_data = Vec::with_capacity(width * height); // Not sure why this isn't muliplied by 3
    //     for _i in 0..width {
    //         for _j in 0..height {
    //             draw_data.push(0);
    //             draw_data.push(0);
    //             draw_data.push(0);
    //         }
    //     }

    //     // In case this render is being performed before first FFT has been calculated
    //     if !self.current_render_data.is_empty() {
    //         // Draw FFT over background
    //         for (index, frequency) in self.current_render_data.iter().enumerate() {
    //             let bar_height = (height as f64 * frequency).floor() as usize;

    //             let x = index * 3;
                
    //             // I have no idea why this is out of bounds nor do I understand how this texture stuff works at all
    //             let test = (bar_height as i32 - 2).clamp(0, height as i32 - 1) as usize;

    //             for j in test..bar_height {
    //                 let y = (height as i32 - 2 - j as i32).clamp(0, height as i32 - 1) as usize * width * 3;

    //                 // Draw bar as white
    //                 draw_data[y + x] = 255;
    //                 draw_data[y + x + 1] = 255;
    //                 draw_data[y + x + 2] = 255;
    //             }
    //         }
    //     }
    
    //     let texture = unsafe { self.glow_context.create_texture() }.expect("Unable to create GL texture: ");

    //     // TODO: Look into using tex_sub_image_2d - should be more efficient
    
    //     unsafe {
    //         self.glow_context.bind_texture(glow::TEXTURE_2D, Some(texture));
    //         self.glow_context.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
    //         self.glow_context.tex_parameter_i32(
    //             glow::TEXTURE_2D, 
    //             glow::TEXTURE_MIN_FILTER, 
    //             glow::LINEAR as _
    //         );
    //         self.glow_context.tex_parameter_i32(
    //             glow::TEXTURE_2D,
    //             glow::TEXTURE_MAG_FILTER,
    //             glow::LINEAR as _
    //         );
    //         self.glow_context.tex_image_2d(
    //             glow::TEXTURE_2D,
    //             0,
    //             glow::RGB as _,
    //             width as _,
    //             height as _,
    //             0,
    //             glow::RGB,
    //             glow::UNSIGNED_BYTE,
    //             Some(&draw_data)
    //         );
    //     }

    //     self.textures.replace(self.texture_id, texture);
    // }

    // /// Returns the texture mapping that the visualisation texture is added to.
    // pub fn get_textures(&self) -> &imgui::Textures<glow::Texture> {
    //     &self.textures
    // }

    // /// Returns the texture ID of the generated visualisation texture.
    // pub fn get_texture_id(&self) -> imgui::TextureId {
    //     self.texture_id
    // }

    // TODO: Add a resize function for changes in window size, just needs to recalculate height and x coords

    /// Performs necessary preprocessing.
    /// 
    /// This includes calculating the mel of each frequency, averaging the data into 100 chunks, and scaling the data between 0 and 1.
    /// 
    /// # Arguments
    /// 
    /// * `lock` - Is the lock on the input data.
    fn data_preprocess(&mut self) {
        let lock = self.input_data.lock().unwrap();
        let data = &*lock;
        if lock.is_empty() {
            return;
        }

        let width = self.current_size[0];
        let height = self.current_size[1];

        // Calculate the mel of each frequency
        let mel_data: Vec<(Complex<f32>, f32)> = data.iter().map(|&x| (x.0, 1127.0 * (1.0 + x.1 / 700.0).ln())).collect();

        // Average data into arbitrary number of 150 chunks
        let mel_diff = (mel_data.last().unwrap().1 - mel_data.first().unwrap().1) / 150.0;
        let mut i = 0;
        let mut j = 0;
        let mut averaged_data = Vec::new();
        while i < mel_data.len() {
            let mut sum = 0.0;

            while j < mel_data.len() && mel_data[j].1 < mel_data[i].1 + mel_diff {
                sum += mel_data[j].0.norm();
                j += 1;
            }

            averaged_data.push(sum as f32 / (j - i) as f32);
            i = j;
        }

        // Scale data between 0 and 1 and then scale it to the height of the window
        let largest = averaged_data.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
        let processed_data: Vec<f32> = averaged_data.iter().map(|x| (x / largest) * height).collect();

        // Get x coords of each point
        let mut x_values = Vec::with_capacity(averaged_data.len());
        let step = width / averaged_data.len() as f32;
        for i in 0..averaged_data.len() {
            x_values.push(step * i as f32);
        }

        // Combine x and y coords
        let mut final_data = Vec::with_capacity(processed_data.len());
        for i in 0..processed_data.len() {
            final_data.push([x_values[i], processed_data[i]])
        }

        self.current_render_data = final_data.clone();
    }

    /// Resizes the visualisation to fit the new window size.
    /// 
    /// # Arguments
    /// 
    /// * `size` - Is the new size of the window.
    fn resize(&mut self, size: [f32; 2]) {
        let width_factor = size[0] / self.current_size[0];
        let height_factor = size[1] / self.current_size[1];

        self.current_render_data = self.current_render_data.iter().map(|x| [x[0] * width_factor, x[1] * height_factor]).collect();
        self.current_size = size;
    }

    // /// Returns an interpolation of the data which is used to create a smooth visualisation.
    // /// 
    // /// The data is interpolated using a Catmull-Rom spline.
    // fn data_interpolation(&self) -> Vec<f64> {
    //     // Get the x values for each data point
    //     let mut x_values = Vec::with_capacity(self.current_render_data.len());
    //     let step = self.current_size[0] as f64 / self.current_render_data.len() as f64;
    //     for i in 0..self.current_render_data.len() {
    //         x_values.push(step * i as f64);
    //     }

    //     // Create spline keys
    //     let mut keys = Vec::with_capacity(self.current_render_data.len());
    //     for (index, value) in x_values.iter().enumerate() {
    //         keys.push(Key::new(*value, self.current_render_data[index], splines::Interpolation::CatmullRom));
    //     }

    //     // Create spline and sample it
    //     let spline = Spline::from_vec(keys);
    //     let mut values = Vec::with_capacity(self.current_size[0] as usize);
    //     for i in 0..self.current_size[0] as usize {
    //         values.push(match spline.clamped_sample(i as f64) {
    //             Some(x) => x,
    //             None => 0.0
    //         });
    //     }

    //     //values = values.iter().map(|x| { x * self.current_size[1] as f64 }).collect();

    //     values
    // }
}