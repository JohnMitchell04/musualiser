use std::{num::NonZeroU32, time::Instant};
use glow::HasContext;
use glutin::surface::*;
use imgui_winit_support::winit::event;

mod initialisation;

fn main() {
    // Create the window and other components to be used by our application
    let (event_loop, window, surface, context) = initialisation::create_window();

    // Initialise imgui for our window
    let (mut winit_platform, mut imgui_context) = initialisation::imgui_init(&window);

    // Get the OpenGL context from glow
    let glow = initialisation::glow_context(&context);

    // Initialise the imgui renderer
    let mut ig_renderer = imgui_glow_renderer::AutoRenderer::initialize(glow, &mut imgui_context)
        .expect("Failed to create renderer");
    
    // Create tracker for performance from last frame
    let mut last_frame = Instant::now();

    // Start the event loop
    event_loop.run(move |event, window_target| {
        match event {
            // For any event we update the imgui context with the new time and update the last frame
            event::Event::NewEvents(_) => {
                let now = Instant::now();
                imgui_context.io_mut().update_delta_time(now.duration_since(last_frame));
                last_frame = now;
            }
            // If we are about to do nothing, request a redraw
            event::Event::AboutToWait => {
                let _ = winit_platform.prepare_frame(imgui_context.io_mut(), &window);
                window.request_redraw();
            }
            // When a redraw is requested
            event::Event::WindowEvent { event: event::WindowEvent::RedrawRequested, .. } => {
                // Clear the colour buffer
                unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

                // Get the UI and tell it to display the demo window
                let ui = imgui_context.frame();
                ui.show_demo_window(&mut true);

                // Prepare the render on winit 
                winit_platform.prepare_render(ui, &window);
                let draw_data = imgui_context.render();

                // Tell imgui to render
                ig_renderer.render(draw_data).expect("Error rendering imgui");


                surface.swap_buffers(&context).expect("Failed to swap buffers");
            }
            event::Event::WindowEvent { event: event::WindowEvent::CloseRequested, .. } => {
                window_target.exit();
            }
            event::Event::WindowEvent { event: event::WindowEvent::Resized(new_size), .. } => {
                if new_size.width > 0 && new_size.height > 0 {
                    surface.resize(
                        &context,
                        NonZeroU32::new(new_size.width).unwrap(),
                        NonZeroU32::new(new_size.height).unwrap()
                    );
                }
                winit_platform.handle_event(imgui_context.io_mut(), &window, &event);
            }
            event => {
                winit_platform.handle_event(imgui_context.io_mut(), &window, &event);
            }
        }  
    }).expect("Event loop error");
}