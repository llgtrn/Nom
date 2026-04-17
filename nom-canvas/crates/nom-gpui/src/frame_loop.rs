#![cfg(feature = "native")]
//! Winit 0.30 event loop integration for `nom-gpui`.
//!
//! [`App`] implements `winit::application::ApplicationHandler` so the caller
//! only needs to:
//!
//! 1. Construct a [`crate::context::GpuContext`].
//! 2. Implement [`FrameHandler`] for the per-frame draw logic.
//! 3. Call `App::new(gpu, handler).run(event_loop)`.
//!
//! Present cadence is driven entirely by the OS via `PresentMode::Fifo` (vsync).
//! No manual 60 fps tick is installed.

#![deny(unsafe_code)]

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

use crate::context::GpuContext;
use crate::element::ElementId;
use crate::window::{WindowError, WindowSurface};

// ── ElementStateMap ───────────────────────────────────────────────────────────

/// Persistent cross-frame state keyed by [`ElementId`].
///
/// Elements look up or insert their own typed state each frame. State survives
/// across redraws until explicitly removed.
pub struct ElementStateMap {
    map: HashMap<ElementId, Box<dyn Any>>,
}

impl ElementStateMap {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    /// Get or insert state of type `T` for `id`.
    ///
    /// If no entry exists for `id`, calls `init()` to create the initial value.
    /// Returns a mutable reference to the stored `T`.
    ///
    /// # Panics
    ///
    /// Panics if an entry already exists but was stored as a different type.
    pub fn get_or_insert<T: Any, F: FnOnce() -> T>(&mut self, id: ElementId, init: F) -> &mut T {
        self.map
            .entry(id)
            .or_insert_with(|| Box::new(init()))
            .downcast_mut::<T>()
            .expect("ElementStateMap: type mismatch for element id")
    }

    /// Remove and return the raw boxed state for `id`, if present.
    pub fn remove(&mut self, id: ElementId) -> Option<Box<dyn Any>> {
        self.map.remove(&id)
    }

    /// Number of tracked elements.
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

// ── FrameHandler ──────────────────────────────────────────────────────────────

/// Trait implemented by the application to receive per-frame callbacks.
///
/// All methods are called on the main thread by the winit event loop.
pub trait FrameHandler: 'static {
    /// Called once per frame.
    ///
    /// The implementation should record draw commands into `view` using the
    /// GPU resources it holds. The texture will be presented to the display
    /// after this method returns.
    fn draw(&mut self, view: &wgpu::TextureView, window: &Window);

    /// Called when the window is resized (including on the initial show).
    ///
    /// The default implementation is a no-op; override when the handler owns
    /// resources (e.g. depth textures) that must be rebuilt on resize.
    fn resize(&mut self, _size: (u32, u32)) {}

    /// Called when the user closes the window.
    ///
    /// Return `true` to allow the application to quit (default). Return `false`
    /// to keep the window open (e.g. while showing a save dialog).
    fn close_requested(&mut self) -> bool {
        true
    }
}

// ── App ───────────────────────────────────────────────────────────────────────

/// Top-level winit application that owns the GPU context, window, and surface.
///
/// Construct with [`App::new`] then hand to [`EventLoop::run_app`] via
/// [`App::run`].
pub struct App<H: FrameHandler> {
    handler: H,
    window: Option<Arc<Window>>,
    surface: Option<WindowSurface>,
    gpu: GpuContext,
    /// Persistent element state, keyed by [`ElementId`].
    /// Available for future wave-3 integration with the element lifecycle.
    pub element_state: ElementStateMap,
}

impl<H: FrameHandler> App<H> {
    /// Create the application.
    ///
    /// The window and surface are created lazily inside
    /// `ApplicationHandler::resumed` so that the event loop is already active
    /// when wgpu receives the surface handle — the winit 0.30 requirement.
    pub fn new(gpu: GpuContext, handler: H) -> Self {
        Self {
            handler,
            window: None,
            surface: None,
            gpu,
            element_state: ElementStateMap::new(),
        }
    }

    /// Enter the winit event loop.
    ///
    /// This call blocks until the application exits (e.g. after
    /// [`FrameHandler::close_requested`] returns `true`).
    pub fn run(mut self, event_loop: EventLoop<()>) -> Result<(), WindowError> {
        // `run_app` consumes the event loop; errors are propagated via the
        // `WindowError::EventLoop` variant.
        event_loop.run_app(&mut self)?;
        Ok(())
    }
}

// ── ApplicationHandler ────────────────────────────────────────────────────────

impl<H: FrameHandler> ApplicationHandler for App<H> {
    /// Called by winit when the application is (re-)activated.
    ///
    /// On the very first call this creates the window and surface. On mobile /
    /// Wayland it can be called again after a suspension; in that case the
    /// existing window is reused.
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            // Already initialised; nothing to do on re-resume.
            return;
        }

        let attrs = WindowAttributes::default().with_title("NomCanvas");
        let window = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("nom-gpui: failed to create window: {e}");
                el.exit();
                return;
            }
        };

        let surface = match WindowSurface::new(
            &self.gpu.instance,
            &self.gpu.adapter,
            &self.gpu.device,
            Arc::clone(&window),
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("nom-gpui: failed to create surface: {e}");
                el.exit();
                return;
            }
        };

        // Notify the handler of the initial window size.
        let phys = window.inner_size();
        self.handler.resize((phys.width, phys.height));

        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn window_event(
        &mut self,
        el: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            // ── Draw ──────────────────────────────────────────────────────────
            WindowEvent::RedrawRequested => {
                // Device-loss recovery.
                if self.gpu.is_device_lost() {
                    if let Err(e) = pollster::block_on(self.gpu.recover()) {
                        eprintln!("nom-gpui: device recovery failed: {e}");
                        el.exit();
                        return;
                    }
                    // Reconfigure surface with the recovered device.
                    if let (Some(surface), Some(window)) = (&mut self.surface, &self.window) {
                        let phys = window.inner_size();
                        surface.resize(&self.gpu.device, (phys.width, phys.height));
                    }
                }

                let surface = match self.surface.as_mut() {
                    Some(s) => s,
                    None => return,
                };
                let window = match self.window.as_ref() {
                    Some(w) => w,
                    None => return,
                };

                let frame = match surface.acquire() {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("nom-gpui: surface acquire error: {e}");
                        window.request_redraw();
                        return;
                    }
                };

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                self.handler.draw(&view, window);
                frame.present();
                window.request_redraw();
            }

            // ── Resize ────────────────────────────────────────────────────────
            WindowEvent::Resized(physical_size) => {
                let new_size = (physical_size.width, physical_size.height);
                if let Some(surface) = self.surface.as_mut() {
                    surface.resize(&self.gpu.device, new_size);
                }
                self.handler.resize(new_size);
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }

            // ── Close ─────────────────────────────────────────────────────────
            WindowEvent::CloseRequested => {
                if self.handler.close_requested() {
                    el.exit();
                }
            }

            _ => {}
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal no-op handler used in unit tests.
    struct NoopHandler;

    impl FrameHandler for NoopHandler {
        fn draw(&mut self, _view: &wgpu::TextureView, _window: &Window) {}
    }

    // --- ElementStateMap tests ---

    #[test]
    fn element_state_get_or_insert_creates_then_returns_same() {
        let mut map = ElementStateMap::new();
        let id = ElementId(42);
        // First call creates the entry.
        {
            let v = map.get_or_insert(id, || 0u32);
            assert_eq!(*v, 0);
            *v = 7;
        }
        // Second call returns the same mutated value.
        {
            let v = map.get_or_insert(id, || 99u32);
            assert_eq!(*v, 7, "should return existing state, not re-initialize");
        }
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn element_state_remove_clears_entry() {
        let mut map = ElementStateMap::new();
        let id = ElementId(1);
        map.get_or_insert(id, || String::from("hello"));
        assert_eq!(map.len(), 1);
        let removed = map.remove(id);
        assert!(removed.is_some());
        assert_eq!(map.len(), 0);
        // A second remove returns None.
        assert!(map.remove(id).is_none());
    }

    /// Verify default trait implementations compile and behave correctly.
    #[test]
    fn frame_handler_defaults_are_correct() {
        let mut h = NoopHandler;
        assert!(h.close_requested(), "default close_requested must return true");
        h.resize((800, 600)); // must not panic
    }

    /// Integration guard: try to open an event loop. If the platform denies
    /// it (CI without a display server), skip silently.
    #[test]
    fn window_surface_constructs_when_display_available() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: no display server (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        build_test_event_loop();
    }

    #[cfg(target_os = "windows")]
    fn build_test_event_loop() {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        let Ok(event_loop) = EventLoop::builder().with_any_thread(true).build() else {
            eprintln!("SKIP: no display in this environment");
            return;
        };
        let _ = event_loop;
    }

    #[cfg(not(target_os = "windows"))]
    fn build_test_event_loop() {
        let Ok(event_loop) = EventLoop::new() else {
            eprintln!("SKIP: no display in this environment");
            return;
        };
        let _ = event_loop;
    }

    /// `WindowError::NoFormat` must be constructible at the type level.
    #[test]
    fn window_error_variants_exist() {
        let _err: Result<(), WindowError> = Err(WindowError::NoFormat);
    }
}