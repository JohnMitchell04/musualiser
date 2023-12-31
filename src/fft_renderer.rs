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
    pub fn new(glow_context: Rc<glow::Context>, input_data: Arc<Mutex<Vec<Complex<f32>>>>, mut textures: imgui::Textures<glow::Texture>) -> Self {
        // Create dummy initial texture
        let width = 1;
        let height = 1;
    
        let mut data = Vec::with_capacity(width * height);
        for i in 0..width {
            for j in 0..height {
                data.push(i as u8);
                data.push(j as u8);
                data.push((i + j) as u8);
            }
        }
    
        let texture = unsafe { glow_context.create_texture() }.expect("Unable to create GL texture");
    
        unsafe {
            glow_context.bind_texture(glow::TEXTURE_2D, Some(texture));
            glow_context.tex_parameter_i32(
                glow::TEXTURE_2D, 
                glow::TEXTURE_MIN_FILTER, 
                glow::LINEAR as _
            );
            glow_context.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as _,
            );
            glow_context.tex_image_2d(
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

        let texture_id = textures.insert(texture);

        FftRenderer { glow_context, input_data, textures, texture_id }
    }

    pub fn render_fft(&mut self) {
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
                Some(&data),
            )
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