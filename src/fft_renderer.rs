use std::{ rc::Rc, sync::{ Arc, Mutex }};
use glow::HasContext;
use rustfft::num_complex::Complex;

pub struct FftRenderer {
    glow_context: Rc<glow::Context>,
    input_data: Arc<Mutex<Vec<Complex<f32>>>>,
    current_render_data: Vec<f64>,
    textures: imgui::Textures<glow::Texture>,
    texture_id: imgui::TextureId,
    current_size: [f32; 2]
}

impl FftRenderer {
    pub fn new(
        glow_context: Rc<glow::Context>,
        input_data: Arc<Mutex<Vec<Complex<f32>>>>,
        mut textures: imgui::Textures<glow::Texture>
    ) -> Self {
        // Create and add dummy initial texture
        let texture = unsafe { glow_context.create_texture() }.expect("Unable to create GL texture");
        let texture_id = textures.insert(texture);
        let current_size = [0.0, 0.0];
        let current_render_data = Vec::new();

        FftRenderer { glow_context, input_data, current_render_data, textures, texture_id, current_size }
    }

    pub fn render(&mut self, size: [f32; 2]) {
        let mut lock = self.input_data.lock().unwrap();
        if !(*lock).is_empty() { 
            // Update internal data
            let data = &mut (*lock);

            // Calculate modulus of complex data
            self.current_render_data = data.iter()
                .map(|x| (x.norm() as f64 / self.current_render_data.len() as f64))
                .collect();

            // FFT can now be processed, so clear data and release lock
            data.clear();
            drop(lock);

            // Scale data between 0 and 1
            let largest = self.current_render_data.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
            self.current_render_data = self.current_render_data.iter().map(|x| (x / largest)).collect();

            // Update size just in case
            self.current_size = size;

            // Render
            self.render_fft();
        }
        else if size != self.current_size {
            // Don't need the lock so it can be released
            drop(lock);

            // Update size
            self.current_size = size;

            // Render
            self.render_fft();
        }
    }

    pub fn render_fft(&mut self) {
        // Dimensions
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

        // In case this render is being performed before first FFT has been calculated
        if !self.current_render_data.is_empty() {
            // Calculate width of each bar
            let bar_width = width / self.current_render_data.len();

            // TODO: I believe that (need to check): Currently the scale of frequencies changes per FFT, make sure it is constant

            // Draw FFT over background
            for (index, frequency) in self.current_render_data.iter().enumerate() {
                // Calculate bar height
                let bar_height = (height as f64 * frequency).round() as usize;

                for i in 0..bar_width {
                    let x = ((index * bar_width) + i) * 3;

                    for j in 0..bar_height {
                        let y = (height - (j + 1)) * width * 3;

                        // Draw bar as white
                        draw_data[y + x] = 255;
                        draw_data[y + x + 1] = 255;
                        draw_data[y + x + 2] = 255;
                    }
                }
            }
        }
    
        let texture = unsafe { self.glow_context.create_texture() }.expect("Unable to create GL texture");

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
                glow::LINEAR as _,
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
                Some(&draw_data),
            );
        }

        self.textures.replace(self.texture_id, texture);
    }

    pub fn get_textures(&self) -> &imgui::Textures<glow::Texture> {
        &self.textures
    }

    pub fn get_texture_id(&self) -> imgui::TextureId {
        self.texture_id
    }
}