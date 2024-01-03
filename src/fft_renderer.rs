use std::{ rc::Rc, sync::{ Arc, Mutex }};
use glow::HasContext;
use rustfft::num_complex::Complex;

pub struct FftRenderer {
    glow_context: Rc<glow::Context>,
    input_data: Arc<Mutex<Vec<Complex<f32>>>>,
    textures: imgui::Textures<glow::Texture>,
    texture_id: imgui::TextureId
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

        FftRenderer { glow_context, input_data, textures, texture_id }
    }

    pub fn render_fft(&mut self, size: [f32; 2]) {
        // If we are still waiting on the next FFT, don't calculate a new display
        let mut lock = self.input_data.lock().unwrap();
        if (*lock).is_empty() { return; }

        let data = &mut (*lock);

        // Normalise data
        let mut normalised: Vec<f64> = data.iter().map(|x| (x.norm() as f64 / data.len() as f64)).collect();

        // Scale data between 0 and 1
        let largest = normalised.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
        normalised = normalised.iter().map(|x| (x / largest)).collect();

        // Calculate data used in the rendering
        let width = size[0].floor() as usize;
        let height = size[1].floor() as usize;

        let bar_width = (width as f64 / normalised.len() as f64).floor() as usize;

        // Draw black background
        let mut draw_data = Vec::with_capacity(width * height);
        for _i in 0..width {
            for _j in 0..height {
                draw_data.push(0);
                draw_data.push(0);
                draw_data.push(0);
            }
        }

        // TODO: I believe that (need to check): Currently the scale of frequencies changes per FFT, make sure it is constant

        // Draw FFT over background
        for (index, frequency) in normalised.iter().enumerate() {
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
    
        let texture = unsafe { self.glow_context.create_texture() }.expect("Unable to create GL texture");
    
        unsafe {
            self.glow_context.bind_texture(glow::TEXTURE_2D, Some(texture));
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
            )
        }

        self.textures.replace(self.texture_id, texture);

        // Clear FFT data for next time
        data.clear();
    }

    pub fn get_textures(&self) -> &imgui::Textures<glow::Texture> {
        &self.textures
    }

    pub fn get_texture_id(&self) -> imgui::TextureId {
        self.texture_id
    }
}