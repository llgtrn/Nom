use crate::event::*;
use crate::types::*;

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
}

/// Window state managed by nom-gpui
pub struct Window {
    pub options: WindowOptions,
    pub scale_factor: f32,
    pub content_size: Vec2,
    pub is_focused: bool,
    pub cursor_position: Vec2,
    // In real impl: winit::window::Window + wgpu swap chain
    frame_pending: bool,
    close_requested: bool,
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

    let os_window = builder.build(&event_loop).expect("create native window");
    let mut window = Window::new(options);
    window.scale_factor = os_window.scale_factor() as f32;
    let size = os_window.inner_size();
    window.content_size = Vec2::new(size.width as f32, size.height as f32);
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
        assert!(!w.take_frame_pending(), "new window must start with no pending frame");
    }

    #[test]
    fn event_loop_active_after_request_redraw() {
        // After request_redraw() the window enters active state (frame pending).
        let mut w = Window::new(WindowOptions::default());
        w.take_frame_pending(); // clear initial state
        w.request_redraw();
        assert!(w.take_frame_pending(), "request_redraw must activate frame-pending");
        // After take, it returns to idle.
        assert!(!w.take_frame_pending(), "after take, window returns to idle");
    }

    #[test]
    fn keyboard_event_shift_modifier_combination() {
        use crate::event::{Key, KeyEvent, Modifiers};
        let ev = KeyEvent::Pressed {
            key: Key::Char('S'),
            modifiers: Modifiers { shift: true, ctrl: false, alt: false, meta: false },
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
            modifiers: Modifiers { shift: true, ctrl: true, alt: false, meta: false },
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
        let modifiers = Modifiers { shift: true, ctrl: true, alt: true, meta: true };
        let ev = KeyEvent::Pressed { key: Key::Return, modifiers };
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
            modifiers: Modifiers { shift: false, ctrl: false, alt: true, meta: false },
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
        assert_eq!(w.content_size, new_size, "content_size must update after resize");
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
        assert!(w.take_frame_pending(), "scale factor change must trigger redraw");
    }

    #[test]
    fn window_close_requested_flag() {
        let mut w = Window::new(WindowOptions::default());
        assert!(!w.close_requested());
        w.request_close();
        assert!(w.close_requested(), "after request_close the flag must be set");
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
        let w = WindowBuilder::new("custom").width(2560.0).height(1440.0).build();
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
        let ev = KeyEvent::Input { text: "nom".to_string() };
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
}
