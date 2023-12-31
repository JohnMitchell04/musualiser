use std::{ rc::Rc, time::Instant, num::NonZeroU32 };
use glutin::{ 
    context::{ ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext },
    config::ConfigTemplateBuilder,
    display::{ GetGlDisplay, GlDisplay },
    surface::{ Surface, SurfaceAttributesBuilder, WindowSurface, GlSurface }
};
use glow::HasContext;
use imgui_winit_support::{
    winit::{
        event_loop::EventLoop,
        window::WindowBuilder,
        dpi::LogicalSize,
        window::Window,
        event
    },
    WinitPlatform
};
use raw_window_handle::HasRawWindowHandle;
use imgui::Ui;

use crate::fft_renderer;

// Struct holding all necessary information about our application
pub struct Application {
    event_loop: EventLoop<()>,
    window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    winit_platform: WinitPlatform,
    imgui_context: imgui::Context,
    glow_context: Rc<glow::Context>,
    ig_renderer: imgui_glow_renderer::Renderer
}

impl Application {
    // Main event loop that takes a customisable UI function
    pub fn main_loop<F: FnMut(&mut bool, &mut Ui, &mut fft_renderer::FftRenderer)>(self, mut fft_renderer: fft_renderer::FftRenderer, mut run_ui: F) {
        let Application {
            event_loop,
            window,
            surface,
            context,
            mut winit_platform,
            mut imgui_context,
            glow_context,
            mut ig_renderer
        } = self;
        let mut last_frame = Instant::now();

        // Start the event loop
        event_loop.run(move |event, window_target| {
            match event {
                // For all events we update the imgui context with the new time and update the last frame
                event::Event::NewEvents(_) => {
                    let now = Instant::now();
                    imgui_context.io_mut().update_delta_time(now.duration_since(last_frame));
                    last_frame = now;
                }
                // If we are about to do nothing, request a redraw
                event::Event::AboutToWait => {
                    let _ = winit_platform
                        .prepare_frame(imgui_context.io_mut(), &window)
                        .expect("Failed to prepare frame");
                    window.request_redraw();
                }
                // When a redraw is requested
                event::Event::WindowEvent { event: event::WindowEvent::RedrawRequested, .. } => {
                    // Clear the colour buffer
                    unsafe { glow_context.clear(glow::COLOR_BUFFER_BIT) };

                    // Get the UI and run the passed UI
                    let ui = imgui_context.frame();
                    let mut run = true;
                    run_ui(&mut run, ui, &mut fft_renderer);
                    if !run {
                        window_target.exit();
                    }

                    // Prepare the render on winit 
                    winit_platform.prepare_render(ui, &window);

                    // Render the imgui scene and return the draw data
                    let draw_data = imgui_context.render();

                    // Tell the renderer to draw the imgui data
                    ig_renderer.render::<imgui::Textures<glow::Texture>>(&glow_context, fft_renderer.get_textures(), draw_data).expect("Error rendering imgui");
                    surface.swap_buffers(&context).expect("Failed to swap buffers");
                }
                // Exit when requested
                event::Event::WindowEvent { event: event::WindowEvent::CloseRequested, .. } => {
                    window_target.exit();
                }
                // When resize is requested, ensure everything is done correctly
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
                // Other events do not affect us and can be passed to winit
                event => {
                    winit_platform.handle_event(imgui_context.io_mut(), &window, &event);
                }
            }  
        }).expect("Event loop error");
    }

    pub fn glow_context(&self) -> Rc<glow::Context> {
        self.glow_context.clone()
    }
}

pub fn initialise_appplication() -> (Application, imgui::Textures<glow::Texture>) {
    // Create the window and other components to be used by our application
    let (event_loop, window, surface, context) = create_window();

    // Initialise imgui for our window
    let (winit_platform, mut imgui_context) = imgui_init(&window);

    // Get the OpenGL context from glow
    let glow_context = Rc::new(glow_context(&context));

    // Enable sRGB support
    unsafe { glow_context.enable(glow::FRAMEBUFFER_SRGB); }

    // Create texture mapping
    let mut textures = imgui::Textures::<glow::Texture>::default();

    // Initialise the imgui renderer
    let ig_renderer = imgui_glow_renderer::Renderer::initialize(
        &glow_context,
        &mut imgui_context, 
        &mut textures, 
        false
    ).expect("Failed to create renderer");

    (Application {event_loop, window, surface, context, winit_platform, imgui_context, glow_context, ig_renderer }, textures)
}

fn create_window() -> (EventLoop<()>, Window, Surface<WindowSurface>, PossiblyCurrentContext) {
    // Create a new event loop
    let event_loop: EventLoop<()> = EventLoop::new().unwrap();

    // Create a window builder with specified title and dimensions
    let window_builder: WindowBuilder = WindowBuilder::new()
        .with_title("Musualiser")
        .with_inner_size(LogicalSize::new(640, 480));

    // Build window and config with the window builder
    let (window, cfg) = glutin_winit::DisplayBuilder::new()
        .with_window_builder(Some(window_builder))
        .build(&event_loop, ConfigTemplateBuilder::new(), |mut config| {
            config.next().unwrap()
        })
        .expect("Failed to create OpenGL Window");

    let window: Window = window.unwrap();

    // Get the current context attributes for the window
    let context_attribs = ContextAttributesBuilder::new()
        .build(Some(window.raw_window_handle()));

    // Create the context from the context attributes
    let context: glutin::context::NotCurrentContext = unsafe {
        cfg.display()
            .create_context(&cfg, &context_attribs)
            .expect("Failed to create OpenGL context")
    };
    
    // Get the surface attributes for our window
    let surface_attribs = SurfaceAttributesBuilder::<WindowSurface>::new()
        .with_srgb(Some(true))
        .build(window.raw_window_handle(), NonZeroU32::new(640).unwrap(), NonZeroU32::new(480).unwrap());

    // Create the surface for the window
    let surface: Surface<WindowSurface> = unsafe {
        cfg.display()
            .create_window_surface(&cfg, &surface_attribs)
            .expect("Failed to create OpenGL surface")
    };

    // Create the context for the surface
    let context: PossiblyCurrentContext = context
        .make_current(&surface)
        .expect("Failed to make OpenGL context current");

    (event_loop, window, surface, context)
}

fn imgui_init(window: &Window) -> (WinitPlatform, imgui::Context) {
    // Create the imgui context
    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    // Initialise the winit platform
    let mut winit_platform = WinitPlatform::init(&mut imgui_context);
    winit_platform.attach_window(imgui_context.io_mut(), window, imgui_winit_support::HiDpiMode::Rounded);

    // Add default fonts to the imgui context
    imgui_context.fonts().add_font(&[imgui::FontSource::DefaultFontData { config: None }]);
    imgui_context.io_mut().font_global_scale = (1.0 / winit_platform.hidpi_factor()) as f32;

    (winit_platform, imgui_context)
}

fn glow_context(context: &PossiblyCurrentContext) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function_cstr(|s| context.display().get_proc_address(s).cast())
    }
}