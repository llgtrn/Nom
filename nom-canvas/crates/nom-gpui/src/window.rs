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
        }
    }

    pub fn request_redraw(&mut self) {
        self.frame_pending = true;
    }
    pub fn take_frame_pending(&mut self) -> bool {
        std::mem::take(&mut self.frame_pending)
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

/// Run the application event loop (stub — real impl uses winit EventLoop)
pub fn run_application<H: ApplicationHandler>(options: WindowOptions, mut handler: H) {
    let mut window = Window::new(options);
    handler.resumed(&mut window);
    // In real impl: starts winit event loop
    // Here: single synthetic frame for testing
    handler.about_to_wait(&mut window);
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
}
