use std::{ rc::Rc, sync::{ Arc, Mutex }};
use glow::HasContext;
use rustfft::num_complex::Complex;

/// Holds all necessary information for the visualisation renderer
pub struct FftRenderer {
    glow_context: Rc<glow::Context>,
    input_data: Arc<Mutex<Vec<(Complex<f32>, f64)>>>,
    current_render_data: Vec<f64>,
    textures: imgui::Textures<glow::Texture>,
    texture_id: imgui::TextureId,
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
    pub fn new(glow_context: Rc<glow::Context>, input_data: Arc<Mutex<Vec<(Complex<f32>, f64)>>>, mut textures: imgui::Textures<glow::Texture>) -> Self {
        // An initial dummy texture has to be created that can be replaced later
        let texture = unsafe { glow_context.create_texture() }.expect("Unable to create GL texture");
        let texture_id = textures.insert(texture);
        let current_size = [0.0, 0.0];
        let current_render_data = Vec::new();

        FftRenderer { glow_context, input_data, current_render_data, textures, texture_id, current_size }
    }

    /// Handle rendering, if the size of the window hasn't changed or the data is the same this is unecessary and skipped.
    /// 
    /// # Arguments
    /// 
    /// * `size` - Is the size of the render window.
    pub fn render(&mut self, size: [f32; 2]) {
        // TODO: When moving to a windowed model, seperate into pre process step
        let mut lock = self.input_data.lock().unwrap();
        if !(*lock).is_empty() { 
            let data = &mut (*lock);

            // Calculate the mel of each frequency
            let mel_data: Vec<(Complex<f32>, f64)> = data.iter().map(|&x| (x.0, 1127.0 * (1.0 + x.1 / 700.0).ln())).collect();

            // Lock is no longer needed
            data.clear();
            drop(lock);

            // Average data into arbitrary number of 100 chunks
            let mel_diff = (mel_data.last().unwrap().1 - mel_data.first().unwrap().1) / 100.0;
            let mut i = 0;
            let mut j = 0;
            let mut averaged_data = Vec::new();
            while i < mel_data.len() {
                let mut sum = 0.0;

                while j < mel_data.len() && mel_data[j].1 < mel_data[i].1 + mel_diff  {
                    sum += mel_data[j].0.norm();
                    j += 1;
                }

                averaged_data.push(sum as f64 / (j - i) as f64);
                i = j;
            }

            // Scale data between 0 and 1
            let largest = averaged_data.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
            self.current_render_data = averaged_data.iter().map(|x| (x / largest)).collect();

            // Update size just in case
            self.current_size = size;
            self.render_fft();
        } else if size != self.current_size {
            // Don't need the lock so it can be released
            drop(lock);

            self.current_size = size;
            self.render_fft();
        }
    }

    /// Create the visualisation for the audio data.
    pub fn render_fft(&mut self) {
        let width = self.current_size[0] as usize;
        let height = self.current_size[1] as usize;

        // Draw black background
        let mut draw_data = Vec::with_capacity(width * height); // Not sure why this isn't muliplied by 3
        for _i in 0..width {
            for _j in 0..height {
                draw_data.push(0);
                draw_data.push(0);
                draw_data.push(0);
            }
        }

        // // In case this render is being performed before first FFT has been calculated
        // if !self.current_render_data.is_empty() {
        //     // Average data
        //     // Potetially do in powers of 2

        //     // Calculate difference table

        //     // Interpolate for each x value

        //     // TODO: Potentially make line thicker
        // }

        ///////////////////// OLD IMPLEMENTATION /////////////////////
        // In case this render is being performed before first FFT has been calculated
        if !self.current_render_data.is_empty() {
            let bar_width = width / self.current_render_data.len();

            // TODO: I believe that (need to check): Currently the scale of frequencies changes per FFT, make sure it is constant

            // Draw FFT over background
            for (index, frequency) in self.current_render_data.iter().enumerate() {
                let bar_height = (height as f64 * frequency).round() as usize;

                // Not sure why it must start from 1 and 2 for the height
                // but it prevents a white bar at the top and side
                for i in 1..bar_width {
                    let x = ((index * bar_width) + i) * 3;

                    for j in 2..bar_height {
                        let y = (height - j) * width * 3;

                        // Draw bar as white
                        draw_data[y + x] = 255;
                        draw_data[y + x + 1] = 255;
                        draw_data[y + x + 2] = 255;
                    }
                }
            }
        }
    
        let texture = unsafe { self.glow_context.create_texture() }.expect("Unable to create GL texture: ");

        // TODO: Look into using tex_sub_image_2d - should be more efficient
    
        unsafe {
            self.glow_context.bind_texture(glow::TEXTURE_2D, Some(texture));
            self.glow_context.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            self.glow_context.tex_parameter_i32(
                glow::TEXTURE_2D, 
                glow::TEXTURE_MIN_FILTER, 
                glow::LINEAR as _
            );
            self.glow_context.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as _
            );
            self.glow_context.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as _,
                width as _,
                height as _,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                Some(&draw_data)
            );
        }

        self.textures.replace(self.texture_id, texture);
    }

    /// Returns the texture mapping that the visualisation texture is added to.
    pub fn get_textures(&self) -> &imgui::Textures<glow::Texture> {
        &self.textures
    }

    /// Returns the texture ID of the generated visualisation texture.
    pub fn get_texture_id(&self) -> imgui::TextureId {
        self.texture_id
    }
}