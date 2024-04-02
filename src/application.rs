use std::{ sync::{ Arc, Mutex}, time::Instant, num::NonZeroU32 };
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

use crate::{fft_renderer, audio_manager};

/// Holds all necessary information about our application.
pub struct Application {
    event_loop: EventLoop<()>,
    window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    winit_platform: WinitPlatform,
    imgui_context: imgui::Context,
    // glow_context: glow::Context,
    ig_renderer: imgui_glow_renderer::AutoRenderer,
    visualisation_renderer: fft_renderer::FftRenderer,
    audio_manager: audio_manager::AudioManager
}

impl Application {
    /// Initialises and creates a new application.
    pub fn new() -> Self {
        let (event_loop, window, surface, context) = Self::create_window();
        let (winit_platform, mut imgui_context) = Self::imgui_init(&window);
        let glow_context = Self::glow_context(&context);

        // Initialise provided ImGui renderer
        let ig_renderer = imgui_glow_renderer::AutoRenderer::initialize(glow_context, &mut imgui_context)
            .expect("Failed to create ImGui renderer: ");

        let shared_samples = Arc::new(Mutex::new(Vec::new()));
        let visualisation_renderer = fft_renderer::FftRenderer::new(shared_samples.clone());
        let audio_manager = audio_manager::AudioManager::new(shared_samples.clone(), 5);

        Application { event_loop, window, surface, context, winit_platform, imgui_context, ig_renderer, visualisation_renderer, audio_manager }
    }

    /// Start the main application loop with the provided UI descriptor function.
    /// 
    /// * `run_ui` - Is the function detailing the UI and its functionality.
    pub fn main_loop<F: FnMut(&mut bool, &mut Ui, &mut fft_renderer::FftRenderer, &mut audio_manager::AudioManager)>(self, mut run_ui: F) {
        let Application {
            event_loop,
            window,
            surface,
            context,
            mut winit_platform,
            mut imgui_context,
            // glow_context,
            mut ig_renderer,
            mut visualisation_renderer,
            mut audio_manager
        } = self;
        let mut last_frame = Instant::now();

        event_loop.run(move |event, window_target| {
            match event {
                // For all events we update the ImGui context and last frame with the new time
                event::Event::NewEvents(_) => {
                    let now = Instant::now();
                    imgui_context.io_mut().update_delta_time(now.duration_since(last_frame));
                    last_frame = now;
                }
                // If the app is about to do nothing, request a redraw
                event::Event::AboutToWait => {
                    // TODO: Can potentially recover from this so maybe change away from expect
                    winit_platform.prepare_frame(imgui_context.io_mut(), &window).expect("Failed to prepare frame: ");
                    window.request_redraw();
                }
                // Handle a redraw
                event::Event::WindowEvent { event: event::WindowEvent::RedrawRequested, .. } => {
                    unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

                    let ui = imgui_context.frame();
                    let mut run = true;
                    run_ui(&mut run, ui, &mut visualisation_renderer, &mut audio_manager);
                    if !run {
                        window_target.exit();
                    }

                    // // Prepare winit backend and create ImGui draw data then render
                    winit_platform.prepare_render(ui, &window);
                    let draw_data = imgui_context.render();
                    ig_renderer.render(draw_data).expect("Error rendering imgui");
                    // // TODO: Can potentially recover from this so maybe change away from expect
                    surface.swap_buffers(&context).expect("Failed to swap buffers: ");
                }
                // Exit when requested
                event::Event::WindowEvent { event: event::WindowEvent::CloseRequested, .. } => {
                    window_target.exit();
                }
                // When resize is requested, ensure everything is done correctly
                event::Event::WindowEvent { event: event::WindowEvent::Resized(new_size), .. } => {
                    if new_size.width > 0 && new_size.height > 0 {
                        surface.resize(&context, NonZeroU32::new(new_size.width).unwrap(), NonZeroU32::new(new_size.height).unwrap());
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

    /// Returns assorted variables for the window created for the application.
    /// 
    /// * `EventLoop` - Is the provides access to system and window events for this application.
    /// 
    /// * `Window` - Is the application window.
    /// 
    /// * `Surface` - Is the OpenGL surface for the application.
    ///
    /// * `PossiblyCurrentContext`- Is the OpenGL context that is possibly on the current thread.
    fn create_window() -> (EventLoop<()>, Window, Surface<WindowSurface>, PossiblyCurrentContext) {
        // Build OpenGL window and config with the winit window builder, making sure to attach the event loop
        let event_loop = EventLoop::new().unwrap();
        let window_builder = WindowBuilder::new()
            .with_title("Musualiser")
            .with_inner_size(LogicalSize::new(640, 480));

        let (window, cfg) = glutin_winit::DisplayBuilder::new()
            .with_window_builder(Some(window_builder))
            .build(&event_loop, ConfigTemplateBuilder::new(), |mut config| { config.next().unwrap() })
            .expect("Failed to create OpenGL Window: ");

        let window = window.unwrap();

        // Create the OpenGL context using the context attributes
        let context_attribs = ContextAttributesBuilder::new()
            .build(Some(window.raw_window_handle()));

        let context = unsafe {
            cfg.display()
                .create_context(&cfg, &context_attribs)
                .expect("Failed to create OpenGL context: ")
        };
        
        // Create the OpenGL surface for the window using the surface atrributes
        let surface_attribs = SurfaceAttributesBuilder::<WindowSurface>::new()
            .with_srgb(Some(true))
            .build(window.raw_window_handle(), NonZeroU32::new(640).unwrap(), NonZeroU32::new(480).unwrap());

        let surface: Surface<WindowSurface> = unsafe {
            cfg.display()
                .create_window_surface(&cfg, &surface_attribs)
                .expect("Failed to create OpenGL surface")
        };

        // Create the context on the current thread
        let context: PossiblyCurrentContext = context
            .make_current(&surface)
            .expect("Failed to make OpenGL context current");

        (event_loop, window, surface, context)
    }

    /// Returns assorted variables for the window created for the application.
    /// 
    /// * `WinitPlatform` - Is the winit backend used by ImGui.
    /// 
    /// * `Context` - Is the ImGui context.
    /// 
    /// # Arguments
    /// 
    /// * `window` - Is the previously created window ImGui will be running in.
    fn imgui_init(window: &Window) -> (WinitPlatform, imgui::Context) {
        // Create the imgui context
        let mut imgui_context = imgui::Context::create();
        imgui_context.set_ini_filename(None); // TODO: Add support for ini files

        // Initialise the ImGui winit platform backend
        let mut winit_platform = WinitPlatform::init(&mut imgui_context);
        winit_platform.attach_window(imgui_context.io_mut(), window, imgui_winit_support::HiDpiMode::Rounded);

        // Add default fonts to the imgui context
        imgui_context.fonts().add_font(&[imgui::FontSource::DefaultFontData { config: None }]);
        imgui_context.io_mut().font_global_scale = (1.0 / winit_platform.hidpi_factor()) as f32;

        (winit_platform, imgui_context)
    }

    /// Returns the OpenGL context.
    /// 
    /// # Arguments
    /// 
    /// * `context` - Is the thread current context.
    fn glow_context(context: &PossiblyCurrentContext) -> glow::Context {
        unsafe {
            glow::Context::from_loader_function_cstr(|s| context.display().get_proc_address(s).cast())
        }
    }
}
