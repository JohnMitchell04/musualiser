use std::num::NonZeroU32;
use glutin::{ 
    context::{ ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext },
    config::ConfigTemplateBuilder,
    display::{ GetGlDisplay, GlDisplay },
    surface::{ Surface, SurfaceAttributesBuilder, WindowSurface }
};
use imgui_winit_support::{
    winit::{
        event_loop::EventLoop,
        window::WindowBuilder,
        dpi::LogicalSize,
        window::Window,
    },
    WinitPlatform
};
use raw_window_handle::HasRawWindowHandle;
use imgui::Context;

pub struct Application {
    pub event_loop: EventLoop<()>,
    pub window: Window,
    pub surface: Surface<WindowSurface>,
    pub context: PossiblyCurrentContext,
    pub winit_platform: WinitPlatform,
    pub imgui_context: Context,
    pub ig_renderer: imgui_glow_renderer::AutoRenderer
}

pub fn initialise_appplication() -> Application {
    // Create the window and other components to be used by our application
    let (event_loop, window, surface, context) = create_window();

    // Initialise imgui for our window
    let (winit_platform, mut imgui_context) = imgui_init(&window);

    // Get the OpenGL context from glow
    let glow = glow_context(&context);

    // Initialise the imgui renderer
    let ig_renderer = imgui_glow_renderer::AutoRenderer::initialize(glow, &mut imgui_context)
        .expect("Failed to create renderer");

    Application {event_loop, window, surface, context, winit_platform, imgui_context, ig_renderer}
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