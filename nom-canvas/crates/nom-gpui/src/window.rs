use crate::types::*;
use crate::event::*;

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

    pub fn request_redraw(&mut self) { self.frame_pending = true; }
    pub fn take_frame_pending(&mut self) -> bool { std::mem::take(&mut self.frame_pending) }

    /// Handle device lost — rebuild swapchain + re-upload atlas
    pub fn handle_device_lost(&mut self) {
        // In real impl: recreate wgpu::Surface + swapchain
        // Atlas textures must be re-uploaded
        self.frame_pending = true;
    }

    pub fn set_cursor_position(&mut self, pos: Vec2) { self.cursor_position = pos; }
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
}
