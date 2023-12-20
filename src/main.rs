use std::{num::NonZeroU32, time::Instant};
use glow::HasContext;
use glutin::surface::*;
use imgui_winit_support::winit::event;

mod initialisation;

fn main() {
    // Create our application
    let mut app = initialisation::initialise_appplication();

    let mut last_frame = Instant::now();

    // Start the event loop
    app.event_loop.run(move |event, window_target| {
        match event {
            // For any event we update the imgui context with the new time and update the last frame
            event::Event::NewEvents(_) => {
                let now = Instant::now();
                app.imgui_context.io_mut().update_delta_time(now.duration_since(last_frame));
                last_frame = now;
            }
            // If we are about to do nothing, request a redraw
            event::Event::AboutToWait => {
                let _ = app.winit_platform.prepare_frame(app.imgui_context.io_mut(), &app.window);
                app.window.request_redraw();
            }
            // When a redraw is requested
            event::Event::WindowEvent { event: event::WindowEvent::RedrawRequested, .. } => {
                // Clear the colour buffer
                unsafe { app.ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

                // Get the UI and tell it to display the demo window
                let ui = app.imgui_context.frame();
                ui.show_demo_window(&mut true);

                // Prepare the render on winit 
                app.winit_platform.prepare_render(ui, &app.window);
                let draw_data = app.imgui_context.render();

                // Tell imgui to render
                app.ig_renderer.render(draw_data).expect("Error rendering imgui");


                app.surface.swap_buffers(&app.context).expect("Failed to swap buffers");
            }
            event::Event::WindowEvent { event: event::WindowEvent::CloseRequested, .. } => {
                window_target.exit();
            }
            event::Event::WindowEvent { event: event::WindowEvent::Resized(new_size), .. } => {
                if new_size.width > 0 && new_size.height > 0 {
                    app.surface.resize(
                        &app.context,
                        NonZeroU32::new(new_size.width).unwrap(),
                        NonZeroU32::new(new_size.height).unwrap()
                    );
                }
                app.winit_platform.handle_event(app.imgui_context.io_mut(), &app.window, &event);
            }
            event => {
                app.winit_platform.handle_event(app.imgui_context.io_mut(), &app.window, &event);
            }
        }  
    }).expect("Event loop error");
}