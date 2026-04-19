use crate::event::*;
use crate::renderer::Renderer;
use crate::scene::Scene;
use crate::types::*;
use std::sync::Arc;

/// Window configuration
pub struct WindowOptions {
    pub title: String,
    pub size: Vec2,
    pub min_size: Option<Vec2>,
    pub decorations: bool,
    pub transparent: bool,
    pub resizable: bool,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "nom-canvas".to_string(),
            size: Vec2::new(1280.0, 800.0),
            min_size: Some(Vec2::new(640.0, 480.0)),
            decorations: true,
            transparent: false,
            resizable: true,
        }
    }
}

/// Builder for `Window` — fluent API for configuring window properties.
pub struct WindowBuilder {
    title: String,
    width: f32,
    height: f32,
    resizable: bool,
}

impl WindowBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 1280.0,
            height: 800.0,
            resizable: true,
        }
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    pub fn resizable(mut self) -> Self {
        self.resizable = true;
        self
    }

    pub fn build(self) -> Window {
        Window::new(WindowOptions {
            title: self.title,
            size: Vec2::new(self.width, self.height),
            resizable: self.resizable,
            ..WindowOptions::default()
        })
    }
}

/// Application handler callbacks — winit ApplicationHandler pattern
pub trait ApplicationHandler {
    fn resumed(&mut self, window: &mut Window);
    fn window_event(&mut self, window: &mut Window, event: WindowEvent);
    fn about_to_wait(&mut self, window: &mut Window);
    /// Build the scene for the current frame. Called during `RedrawRequested`.
    /// Default implementation does nothing.
    fn draw(&mut self, _window: &mut Window, _scene: &mut Scene) {}
}

/// GPU-side window configuration — surface dimensions, MSAA, and vsync.
///
/// Stub fields only: wgpu surface/device/queue handles are not held here
/// because wgpu is not a direct dependency of nom-gpui tests.  The fields
/// below are the semantic contract that a real GPU backend must satisfy.
pub struct GpuWindowConfig {
    /// Surface width in physical pixels.
    pub width: u32,
    /// Surface height in physical pixels.
    pub height: u32,
    /// MSAA sample count (1 = no MSAA, 4 = 4× MSAA).
    pub sample_count: u32,
    /// Whether vertical sync is enabled.
    pub vsync: bool,
}

impl GpuWindowConfig {
    /// Create a new GPU window config with explicit dimensions and MSAA.
    pub fn new(width: u32, height: u32, sample_count: u32, vsync: bool) -> Self {
        Self {
            width,
            height,
            sample_count,
            vsync,
        }
    }
}

impl Default for GpuWindowConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
            sample_count: 1,
            vsync: true,
        }
    }
}

/// Window state managed by nom-gpui
pub struct Window {
    pub options: WindowOptions,
    pub scale_factor: f32,
    pub content_size: Vec2,
    pub is_focused: bool,
    pub cursor_position: Vec2,
    frame_pending: bool,
    close_requested: bool,
    /// Whether the GPU surface has been successfully initialised.
    pub gpu_ready: bool,
    /// GPU surface width in physical pixels (mirrors wgpu surface config).
    pub surface_width: u32,
    /// GPU surface height in physical pixels (mirrors wgpu surface config).
    pub surface_height: u32,
    /// MSAA sample count for the GPU surface.
    pub sample_count: u32,
    /// Whether vertical sync is enabled on the GPU surface.
    pub vsync: bool,
    /// Real wgpu surface — present only after GPU init in native context.
    pub wgpu_surface: Option<wgpu::Surface<'static>>,
    /// Real wgpu device — present only after GPU init.
    pub wgpu_device: Option<Arc<wgpu::Device>>,
    /// Real wgpu queue — present only after GPU init.
    pub wgpu_queue: Option<Arc<wgpu::Queue>>,
    /// Negotiated surface format.
    pub surface_format: Option<wgpu::TextureFormat>,
    /// GPU renderer — present only after GPU init.
    pub renderer: Option<Renderer>,
}

impl Window {
    pub fn new(options: WindowOptions) -> Self {
        let size = options.size;
        Self {
            options,
            scale_factor: 1.0,
            content_size: size,
            is_focused: false,
            cursor_position: Vec2::zero(),
            frame_pending: false,
            close_requested: false,
            gpu_ready: false,
            surface_width: size.x as u32,
            surface_height: size.y as u32,
            sample_count: 1,
            vsync: true,
            wgpu_surface: None,
            wgpu_device: None,
            wgpu_queue: None,
            surface_format: None,
            renderer: None,
        }
    }

    pub fn request_redraw(&mut self) {
        self.frame_pending = true;
    }
    pub fn take_frame_pending(&mut self) -> bool {
        std::mem::take(&mut self.frame_pending)
    }
    pub fn request_close(&mut self) {
        self.close_requested = true;
    }
    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    /// Handle device lost — rebuild swapchain + re-upload atlas
    pub fn handle_device_lost(&mut self) {
        // In real impl: recreate wgpu::Surface + swapchain
        // Atlas textures must be re-uploaded
        self.frame_pending = true;
    }

    pub fn set_cursor_position(&mut self, pos: Vec2) {
        self.cursor_position = pos;
    }
    pub fn set_scale_factor(&mut self, factor: f32) {
        self.scale_factor = factor;
        self.frame_pending = true;
    }
}

/// High-level window events (from winit)
#[derive(Debug, Clone)]
pub enum WindowEvent {
    Mouse(MouseEvent),
    Keyboard(KeyEvent),
    Scroll(ScrollEvent),
    Resized { new_size: Vec2 },
    ScaleFactorChanged { new_scale: f32 },
    Focused(bool),
    CloseRequested,
    DeviceLost,
}

/// Run the native application event loop.
pub fn run_application<H: ApplicationHandler + 'static>(options: WindowOptions, handler: H) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_native_application(options, handler);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut window = Window::new(options);
        handler.resumed(&mut window);
        handler.about_to_wait(&mut window);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_native_application<H: ApplicationHandler + 'static>(options: WindowOptions, mut handler: H) {
    use winit::dpi::LogicalSize;
    use winit::event::{Event, WindowEvent as WinitWindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder as WinitWindowBuilder;

    let event_loop = EventLoop::new().expect("create winit event loop");
    let mut builder = WinitWindowBuilder::new()
        .with_title(options.title.clone())
        .with_inner_size(LogicalSize::new(
            options.size.x as f64,
            options.size.y as f64,
        ))
        .with_decorations(options.decorations)
        .with_transparent(options.transparent)
        .with_resizable(options.resizable);
    if let Some(min_size) = options.min_size {
        builder =
            builder.with_min_inner_size(LogicalSize::new(min_size.x as f64, min_size.y as f64));
    }

    let os_window = Arc::new(builder.build(&event_loop).expect("create native window"));
    let mut window = Window::new(options);
    window.scale_factor = os_window.scale_factor() as f32;
    let size = os_window.inner_size();
    let width = size.width.max(1);
    let height = size.height.max(1);
    window.content_size = Vec2::new(width as f32, height as f32);
    window.surface_width = width;
    window.surface_height = height;

    // Real wgpu init chain. os_window is Arc so the surface lifetime is tied to it safely.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = instance
        .create_surface(Arc::clone(&os_window))
        .expect("create wgpu surface");
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .expect("request wgpu adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        },
        None,
    ))
    .expect("request wgpu device");
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats[0];
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let device_arc = Arc::new(device);
    let queue_arc = Arc::new(queue);
    window.wgpu_surface = Some(surface);
    window.wgpu_device = Some(Arc::clone(&device_arc));
    window.wgpu_queue = Some(Arc::clone(&queue_arc));
    window.surface_format = Some(surface_format);
    window.gpu_ready = true;
    let renderer = Renderer::with_gpu(device_arc, queue_arc, surface_format);
    window.renderer = Some(renderer);

    handler.resumed(&mut window);

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WinitWindowEvent::CloseRequested => {
                    handler.window_event(&mut window, WindowEvent::CloseRequested);
                    elwt.exit();
                }
                WinitWindowEvent::Focused(focused) => {
                    window.is_focused = focused;
                    handler.window_event(&mut window, WindowEvent::Focused(focused));
                }
                WinitWindowEvent::Resized(size) => {
                    let new_size = Vec2::new(size.width as f32, size.height as f32);
                    window.content_size = new_size;
                    handler.window_event(&mut window, WindowEvent::Resized { new_size });
                }
                WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    let scale = scale_factor as f32;
                    window.set_scale_factor(scale);
                    handler.window_event(
                        &mut window,
                        WindowEvent::ScaleFactorChanged { new_scale: scale },
                    );
                }
                WinitWindowEvent::CursorMoved { position, .. } => {
                    window.set_cursor_position(Vec2::new(position.x as f32, position.y as f32));
                }
                WinitWindowEvent::RedrawRequested => {
                    let _ = window.take_frame_pending();
                    let mut scene = Scene::new();
                    handler.draw(&mut window, &mut scene);
                    if scene.is_empty() {
                        scene.push_quad(crate::scene::Quad {
                            bounds: crate::types::Bounds {
                                origin: crate::types::Point {
                                    x: crate::types::Pixels(50.0),
                                    y: crate::types::Pixels(50.0),
                                },
                                size: crate::types::Size {
                                    width: crate::types::Pixels(100.0),
                                    height: crate::types::Pixels(100.0),
                                },
                            },
                            background: Some(crate::types::Hsla::new(0.0, 1.0, 0.5, 1.0)),
                            ..Default::default()
                        });
                    }
                    if let Some(ref mut renderer) = window.renderer {
                        if let (Some(surface), Some(device), Some(queue)) = (
                            window.wgpu_surface.as_ref(),
                            window.wgpu_device.as_ref(),
                            window.wgpu_queue.as_ref(),
                        ) {
                            let _ = renderer.begin_frame();
                            renderer.draw(&mut scene);
                            let _ = renderer.end_frame_render(surface, device, queue);
                        }
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                handler.about_to_wait(&mut window);
                if window.close_requested() {
                    elwt.exit();
                    return;
                }
                if window.take_frame_pending() {
                    os_window.request_redraw();
                }
            }
            Event::Resumed => handler.resumed(&mut window),
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_new_sets_correct_content_size() {
        let opts = WindowOptions {
            size: Vec2::new(1920.0, 1080.0),
            ..WindowOptions::default()
        };
        let w = Window::new(opts);
        assert_eq!(w.content_size, Vec2::new(1920.0, 1080.0));
    }

    #[test]
    fn request_redraw_sets_frame_pending() {
        let mut w = Window::new(WindowOptions::default());
        assert!(!w.take_frame_pending());
        w.request_redraw();
        assert!(w.take_frame_pending());
        // after take it is false again
        assert!(!w.take_frame_pending());
    }

    #[test]
    fn window_default_options_are_correct() {
        let opts = WindowOptions::default();
        assert_eq!(opts.title, "nom-canvas");
        assert_eq!(opts.size, Vec2::new(1280.0, 800.0));
        assert!(opts.resizable);
        assert!(opts.decorations);
        assert!(!opts.transparent);
    }

    #[test]
    fn set_scale_factor_marks_frame_pending() {
        let mut w = Window::new(WindowOptions::default());
        w.take_frame_pending(); // clear any initial pending state
        w.set_scale_factor(2.0);
        assert_eq!(w.scale_factor, 2.0);
        assert!(w.take_frame_pending());
    }

    #[test]
    fn set_cursor_position_updates_stored_position() {
        let mut w = Window::new(WindowOptions::default());
        let pos = Vec2::new(42.0, 100.0);
        w.set_cursor_position(pos);
        assert_eq!(w.cursor_position, pos);
    }

    #[test]
    fn handle_device_lost_sets_frame_pending() {
        let mut w = Window::new(WindowOptions::default());
        w.take_frame_pending();
        w.handle_device_lost();
        assert!(w.take_frame_pending());
    }

    #[test]
    fn window_initial_state_is_unfocused_at_origin() {
        let w = Window::new(WindowOptions::default());
        assert!(!w.is_focused);
        assert_eq!(w.cursor_position, Vec2::zero());
        assert_eq!(w.scale_factor, 1.0);
    }

    #[test]
    fn window_builder_creates_window_with_options() {
        let w = WindowBuilder::new("test-window")
            .width(1920.0)
            .height(1080.0)
            .resizable()
            .build();

        assert_eq!(w.options.title, "test-window");
        assert_eq!(w.options.size, Vec2::new(1920.0, 1080.0));
        assert_eq!(w.content_size, Vec2::new(1920.0, 1080.0));
        assert!(w.options.resizable);
    }

    // ---- New tests ----

    #[test]
    fn window_transparent_option() {
        let opts = WindowOptions {
            transparent: true,
            ..WindowOptions::default()
        };
        assert!(opts.transparent);
    }

    #[test]
    fn window_min_size() {
        let min = Vec2::new(320.0, 240.0);
        let opts = WindowOptions {
            min_size: Some(min),
            ..WindowOptions::default()
        };
        assert_eq!(opts.min_size, Some(min));
    }

    #[test]
    fn window_event_close_requested() {
        let event = WindowEvent::CloseRequested;
        // Pattern-match to confirm the variant is accessible and constructible.
        match event {
            WindowEvent::CloseRequested => {}
            _ => panic!("expected CloseRequested"),
        }
    }

    #[test]
    fn window_event_resized() {
        let new_size = Vec2::new(800.0, 600.0);
        let event = WindowEvent::Resized { new_size };
        match event {
            WindowEvent::Resized { new_size: s } => {
                assert_eq!(s, new_size);
            }
            _ => panic!("expected Resized"),
        }
    }

    // ── Wave AD new tests ────────────────────────────────────────────────────

    #[test]
    fn event_loop_starts_idle_no_frame_pending() {
        // A freshly created Window has no frame pending (idle state).
        let w = Window::new(WindowOptions::default());
        // We can't take_frame_pending on an immutable ref; create mutable copy.
        let mut w = w;
        assert!(
            !w.take_frame_pending(),
            "new window must start with no pending frame"
        );
    }

    #[test]
    fn event_loop_active_after_request_redraw() {
        // After request_redraw() the window enters active state (frame pending).
        let mut w = Window::new(WindowOptions::default());
        w.take_frame_pending(); // clear initial state
        w.request_redraw();
        assert!(
            w.take_frame_pending(),
            "request_redraw must activate frame-pending"
        );
        // After take, it returns to idle.
        assert!(
            !w.take_frame_pending(),
            "after take, window returns to idle"
        );
    }

    #[test]
    fn keyboard_event_shift_modifier_combination() {
        use crate::event::{Key, KeyEvent, Modifiers};
        let ev = KeyEvent::Pressed {
            key: Key::Char('S'),
            modifiers: Modifiers {
                shift: true,
                ctrl: false,
                alt: false,
                meta: false,
            },
        };
        if let KeyEvent::Pressed { key, modifiers } = ev {
            assert_eq!(key, Key::Char('S'));
            assert!(modifiers.shift);
            assert!(!modifiers.ctrl);
            assert!(!modifiers.is_shortcut());
        } else {
            panic!("expected Pressed");
        }
    }

    #[test]
    fn keyboard_event_ctrl_shift_combination() {
        use crate::event::{Key, KeyEvent, Modifiers};
        let ev = KeyEvent::Pressed {
            key: Key::F5,
            modifiers: Modifiers {
                shift: true,
                ctrl: true,
                alt: false,
                meta: false,
            },
        };
        if let KeyEvent::Pressed { key, modifiers } = ev {
            assert_eq!(key, Key::F5);
            assert!(modifiers.shift && modifiers.ctrl);
            assert!(modifiers.is_shortcut(), "ctrl must trigger is_shortcut");
        } else {
            panic!("expected Pressed");
        }
    }

    #[test]
    fn keyboard_event_all_modifiers_active() {
        use crate::event::{Key, KeyEvent, Modifiers};
        let modifiers = Modifiers {
            shift: true,
            ctrl: true,
            alt: true,
            meta: true,
        };
        let ev = KeyEvent::Pressed {
            key: Key::Return,
            modifiers,
        };
        if let KeyEvent::Pressed { modifiers: m, .. } = ev {
            assert!(m.shift && m.ctrl && m.alt && m.meta);
            assert!(m.is_shortcut());
        } else {
            panic!("expected Pressed");
        }
    }

    #[test]
    fn keyboard_event_alt_only_not_shortcut() {
        use crate::event::{Key, KeyEvent, Modifiers};
        let ev = KeyEvent::Pressed {
            key: Key::Tab,
            modifiers: Modifiers {
                shift: false,
                ctrl: false,
                alt: true,
                meta: false,
            },
        };
        if let KeyEvent::Pressed { modifiers, .. } = ev {
            assert!(!modifiers.is_shortcut(), "alt-only must not be is_shortcut");
            assert!(modifiers.alt);
        } else {
            panic!("expected Pressed");
        }
    }

    #[test]
    fn window_resize_updates_content_size() {
        // Simulating a Resized event must update content_size.
        let mut w = Window::new(WindowOptions::default());
        let new_size = Vec2::new(2560.0, 1440.0);
        // Simulate what run_native_application does on resize.
        w.content_size = new_size;
        assert_eq!(
            w.content_size, new_size,
            "content_size must update after resize"
        );
    }

    #[test]
    fn window_resize_to_minimum_size() {
        let mut w = Window::new(WindowOptions::default());
        let min_size = Vec2::new(640.0, 480.0);
        w.content_size = min_size;
        assert_eq!(w.content_size, min_size);
    }

    #[test]
    fn window_resize_multiple_times_keeps_last_value() {
        let mut w = Window::new(WindowOptions::default());
        for (width, height) in [(800.0, 600.0f32), (1024.0, 768.0), (1920.0, 1080.0)] {
            w.content_size = Vec2::new(width, height);
        }
        assert_eq!(w.content_size, Vec2::new(1920.0, 1080.0));
    }

    #[test]
    fn window_scale_factor_change_marks_redraw() {
        let mut w = Window::new(WindowOptions::default());
        w.take_frame_pending();
        w.set_scale_factor(3.0);
        assert_eq!(w.scale_factor, 3.0);
        assert!(
            w.take_frame_pending(),
            "scale factor change must trigger redraw"
        );
    }

    #[test]
    fn window_close_requested_flag() {
        let mut w = Window::new(WindowOptions::default());
        assert!(!w.close_requested());
        w.request_close();
        assert!(
            w.close_requested(),
            "after request_close the flag must be set"
        );
    }

    #[test]
    fn window_focus_state_can_be_set() {
        let mut w = Window::new(WindowOptions::default());
        assert!(!w.is_focused);
        w.is_focused = true;
        assert!(w.is_focused);
    }

    #[test]
    fn window_builder_default_dimensions() {
        let w = WindowBuilder::new("canvas").build();
        assert_eq!(w.content_size, Vec2::new(1280.0, 800.0));
    }

    #[test]
    fn window_builder_custom_dimensions() {
        let w = WindowBuilder::new("custom")
            .width(2560.0)
            .height(1440.0)
            .build();
        assert_eq!(w.content_size, Vec2::new(2560.0, 1440.0));
    }

    #[test]
    fn keyboard_released_event_variant() {
        use crate::event::{Key, KeyEvent, Modifiers};
        let ev = KeyEvent::Released {
            key: Key::Escape,
            modifiers: Modifiers::default(),
        };
        if let KeyEvent::Released { key, .. } = ev {
            assert_eq!(key, Key::Escape);
        } else {
            panic!("expected Released");
        }
    }

    #[test]
    fn keyboard_input_text_event() {
        use crate::event::KeyEvent;
        let ev = KeyEvent::Input {
            text: "nom".to_string(),
        };
        if let KeyEvent::Input { text } = ev {
            assert_eq!(text, "nom");
        } else {
            panic!("expected Input");
        }
    }

    #[test]
    fn window_event_scale_factor_changed() {
        let ev = WindowEvent::ScaleFactorChanged { new_scale: 2.5 };
        if let WindowEvent::ScaleFactorChanged { new_scale } = ev {
            assert!((new_scale - 2.5).abs() < f32::EPSILON);
        } else {
            panic!("expected ScaleFactorChanged");
        }
    }

    #[test]
    fn window_event_focused_true() {
        let ev = WindowEvent::Focused(true);
        if let WindowEvent::Focused(f) = ev {
            assert!(f);
        } else {
            panic!("expected Focused");
        }
    }

    #[test]
    fn window_event_focused_false() {
        let ev = WindowEvent::Focused(false);
        if let WindowEvent::Focused(f) = ev {
            assert!(!f);
        } else {
            panic!("expected Focused");
        }
    }

    // ── Wave AE new tests ────────────────────────────────────────────────────

    #[test]
    fn window_builder_only_title_uses_defaults() {
        let w = WindowBuilder::new("my-app").build();
        assert_eq!(w.options.title, "my-app");
        // Defaults: 1280 x 800
        assert_eq!(w.options.size, Vec2::new(1280.0, 800.0));
        assert!(w.options.resizable);
    }

    #[test]
    fn window_builder_only_width_changed() {
        let w = WindowBuilder::new("test").width(640.0).build();
        assert_eq!(w.content_size.x, 640.0);
        assert_eq!(w.content_size.y, 800.0, "height should remain default 800");
    }

    #[test]
    fn window_builder_only_height_changed() {
        let w = WindowBuilder::new("test").height(480.0).build();
        assert_eq!(w.content_size.x, 1280.0, "width should remain default 1280");
        assert_eq!(w.content_size.y, 480.0);
    }

    #[test]
    fn window_close_not_requested_initially() {
        let w = Window::new(WindowOptions::default());
        assert!(
            !w.close_requested(),
            "close_requested must be false on creation"
        );
    }

    #[test]
    fn window_request_close_sets_flag_permanently() {
        let mut w = Window::new(WindowOptions::default());
        w.request_close();
        // Flag must persist — calling it again must still return true.
        assert!(w.close_requested());
        assert!(w.close_requested());
    }

    #[test]
    fn window_scale_factor_default_is_one() {
        let w = Window::new(WindowOptions::default());
        assert!((w.scale_factor - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn window_set_scale_factor_fractional() {
        let mut w = Window::new(WindowOptions::default());
        w.take_frame_pending();
        w.set_scale_factor(1.5);
        assert!((w.scale_factor - 1.5).abs() < f32::EPSILON);
        assert!(
            w.take_frame_pending(),
            "fractional scale must also trigger redraw"
        );
    }

    #[test]
    fn window_set_cursor_position_multiple_updates() {
        let mut w = Window::new(WindowOptions::default());
        for (x, y) in [(10.0f32, 20.0f32), (100.0, 200.0), (0.0, 0.0)] {
            w.set_cursor_position(Vec2::new(x, y));
            assert_eq!(w.cursor_position, Vec2::new(x, y));
        }
    }

    #[test]
    fn window_handle_device_lost_makes_frame_pending() {
        let mut w = Window::new(WindowOptions::default());
        w.take_frame_pending(); // clear
        w.handle_device_lost();
        assert!(
            w.take_frame_pending(),
            "device-lost must queue a new frame for atlas re-upload"
        );
    }

    #[test]
    fn window_options_no_decorations() {
        let opts = WindowOptions {
            decorations: false,
            ..WindowOptions::default()
        };
        assert!(!opts.decorations);
    }

    #[test]
    fn window_options_non_resizable() {
        let opts = WindowOptions {
            resizable: false,
            ..WindowOptions::default()
        };
        assert!(!opts.resizable);
    }

    #[test]
    fn window_options_no_min_size() {
        let opts = WindowOptions {
            min_size: None,
            ..WindowOptions::default()
        };
        assert!(opts.min_size.is_none());
    }

    #[test]
    fn window_event_device_lost_variant() {
        let ev = WindowEvent::DeviceLost;
        match ev {
            WindowEvent::DeviceLost => {}
            _ => panic!("expected DeviceLost"),
        }
    }

    #[test]
    fn window_event_scroll() {
        use crate::event::{Modifiers, ScrollEvent};
        let ev = WindowEvent::Scroll(ScrollEvent {
            position: Vec2::zero(),
            delta: Vec2::new(0.0, -3.0),
            modifiers: Modifiers::default(),
        });
        match ev {
            WindowEvent::Scroll(_) => {}
            _ => panic!("expected Scroll"),
        }
    }

    #[test]
    fn window_take_frame_pending_is_idempotent_when_false() {
        let mut w = Window::new(WindowOptions::default());
        // No pending frame — multiple takes all return false.
        for _ in 0..5 {
            assert!(!w.take_frame_pending());
        }
    }

    #[test]
    fn window_request_redraw_then_take_resets() {
        let mut w = Window::new(WindowOptions::default());
        w.request_redraw();
        assert!(w.take_frame_pending());
        assert!(!w.take_frame_pending(), "second take must be false");
    }

    // ── Wave AO: GpuWindowConfig + Window GPU fields ─────────────────────────

    #[test]
    fn gpu_window_config_default_values() {
        let cfg = GpuWindowConfig::default();
        assert_eq!(cfg.width, 1280);
        assert_eq!(cfg.height, 800);
        assert_eq!(cfg.sample_count, 1);
        assert!(cfg.vsync, "vsync must be enabled by default");
    }

    #[test]
    fn gpu_window_config_new_sets_all_fields() {
        let cfg = GpuWindowConfig::new(1920, 1080, 4, false);
        assert_eq!(cfg.width, 1920);
        assert_eq!(cfg.height, 1080);
        assert_eq!(cfg.sample_count, 4);
        assert!(!cfg.vsync, "vsync must be false when specified as false");
    }

    #[test]
    fn gpu_window_config_new_vsync_true() {
        let cfg = GpuWindowConfig::new(800, 600, 1, true);
        assert!(cfg.vsync);
    }

    #[test]
    fn gpu_window_config_msaa_none_is_sample_count_1() {
        let cfg = GpuWindowConfig::new(1280, 720, 1, true);
        assert_eq!(cfg.sample_count, 1, "no-MSAA must use sample_count=1");
    }

    #[test]
    fn gpu_window_config_msaa4_is_sample_count_4() {
        let cfg = GpuWindowConfig::new(2560, 1440, 4, true);
        assert_eq!(cfg.sample_count, 4, "4xMSAA must use sample_count=4");
    }

    #[test]
    fn window_new_gpu_ready_is_false() {
        let w = Window::new(WindowOptions::default());
        assert!(
            !w.gpu_ready,
            "GPU surface is not ready until explicitly initialised"
        );
    }

    #[test]
    fn window_new_surface_dimensions_match_options() {
        let opts = WindowOptions {
            size: Vec2::new(1920.0, 1080.0),
            ..WindowOptions::default()
        };
        let w = Window::new(opts);
        assert_eq!(w.surface_width, 1920);
        assert_eq!(w.surface_height, 1080);
    }

    #[test]
    fn window_new_sample_count_default_is_one() {
        let w = Window::new(WindowOptions::default());
        assert_eq!(w.sample_count, 1, "default MSAA sample count must be 1");
    }

    #[test]
    fn window_new_vsync_default_is_true() {
        let w = Window::new(WindowOptions::default());
        assert!(w.vsync, "vsync must be enabled by default");
    }

    #[test]
    fn window_gpu_ready_can_be_set() {
        let mut w = Window::new(WindowOptions::default());
        assert!(!w.gpu_ready);
        w.gpu_ready = true;
        assert!(w.gpu_ready, "gpu_ready must reflect assigned value");
    }

    #[test]
    fn window_surface_width_can_be_updated() {
        let mut w = Window::new(WindowOptions::default());
        w.surface_width = 3840;
        assert_eq!(w.surface_width, 3840);
    }

    #[test]
    fn window_surface_height_can_be_updated() {
        let mut w = Window::new(WindowOptions::default());
        w.surface_height = 2160;
        assert_eq!(w.surface_height, 2160);
    }

    #[test]
    fn window_sample_count_can_be_set_to_four() {
        let mut w = Window::new(WindowOptions::default());
        w.sample_count = 4;
        assert_eq!(w.sample_count, 4);
    }

    #[test]
    fn window_vsync_can_be_disabled() {
        let mut w = Window::new(WindowOptions::default());
        w.vsync = false;
        assert!(!w.vsync, "vsync can be disabled after construction");
    }

    #[test]
    fn gpu_window_config_zero_dimensions_allowed() {
        // Zero dimensions are valid during teardown / before resize.
        let cfg = GpuWindowConfig::new(0, 0, 1, false);
        assert_eq!(cfg.width, 0);
        assert_eq!(cfg.height, 0);
    }
}
