use std::sync::{ Arc, Mutex };
use rustfft::num_complex::Complex;
use imgui::{DrawListMut, ImColor32};
use splines::{Key, Spline};

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
    pub fn render(&mut self, draw_list: DrawListMut<'_>, size: [f32; 2], origin: [f32; 2]) {
        // If the size of the window has changed, the data needs to be recalculated
        if self.current_size != size {
            self.resize(size);
        }

        let mut lock = self.input_data.lock().unwrap();
        let data = &mut *lock;
        if !data.is_empty() {
            self.current_render_data = self.preprocess_data(data);

            // Data is no longer needed
            (*data).clear();
            drop(lock);

            self.current_render_data = self.interpolate_data();
        }

        // Draw bezier curves for the visualisation
        for set in self.current_render_data.windows(4).step_by(3) {
            if set.len() < 4 {
                break;
            }

            draw_list.add_bezier_curve(
                [origin[0] + set[0][0], origin[1] + set[0][1]],
                [origin[0] + set[1][0], origin[1] + set[1][1]],
                [origin[0] + set[2][0], origin[1] + set[2][1]],
                [origin[0] + set[3][0], origin[1] + set[3][1]],
                ImColor32::from_rgba(255, 255, 255, 255)
            ).build();
        }
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

    /// Performs necessary preprocessing.
    /// 
    /// This includes calculating the mel of each frequency, averaging the data into 100 chunks, and scaling the data between 0 and 1.
    /// 
    /// # Arguments
    /// 
    /// * `lock` - Is the lock on the input data.
    fn preprocess_data(&self, data: &Vec<(Complex<f32>, f32)>) -> Vec<[f32; 2]> {
        let width = self.current_size[0];
        let height = self.current_size[1];

        // Calculate the mel of each frequency
        let mel_data: Vec<(Complex<f32>, f32)> = data.iter().map(|&x| (x.0, 1127.0 * (1.0 + x.1 / 700.0).ln())).collect();

        // Average data into arbitrary number of 150 chunks
        let per_chunk = mel_data.len() / 150;
        let mut averaged_data = Vec::with_capacity(150);
        for chunk in mel_data.chunks(per_chunk) {
            let mut sum = 0.0;
            let mut i = 0;
            for value in chunk {
                sum += value.0.norm();
                i += 1;
            }

            averaged_data.push(sum as f32 / i as f32);
        }

        // Normalise data between 0 and 1, scale it to the height of the window and invert for visualisation
        let largest = averaged_data.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
        let processed_data: Vec<f32> = averaged_data.iter().map(|x| height - ((x / largest) * height) - 1 as f32).collect();

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

        final_data
    }

    /// Performs an interpolation of the data which is used to create a smooth visualisation.
    /// 
    /// The data is interpolated using a Catmull-Rom spline.
    fn interpolate_data(&self) -> Vec<[f32; 2]> {
        // Create spline keys
        let mut keys = Vec::with_capacity(self.current_render_data.len());
        for value in self.current_render_data.iter() {
            keys.push(Key::new(value[0], value[1], splines::Interpolation::CatmullRom));
        }

        // Create spline
        let spline = Spline::from_vec(keys);
        let mut sampled_data = Vec::with_capacity(self.current_render_data.len() * 3);

        // Skip the first and last point during interpolation as it leads to odd behaviour
        sampled_data.push(self.current_render_data[0]);
        for set in self.current_render_data.chunks(2).skip(1).rev().skip(1).rev() {
            if set.len() < 2 {
                break;
            }

            let step = (set[1][0] - set[0][0]) / 3.0;
            let x1 = set[0][0] + step;
            let x2 = set[0][0] + step * 2.0;

            // Add two interpolations between points to smooth the visualisation
            sampled_data.push(set[0]);
            sampled_data.push([x1, match spline.sample(x1) {
                Some(value) => value,
                None => 0.0
            }]);
            sampled_data.push([x2, match spline.sample(x2) {
                Some(value) => value,
                None => 0.0
            }]);
            sampled_data.push(set[1]);
        }
        sampled_data.push(*self.current_render_data.last().unwrap());

        sampled_data
    }
}