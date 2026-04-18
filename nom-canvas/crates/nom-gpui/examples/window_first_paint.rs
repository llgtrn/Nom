use nom_gpui::types::Vec2;
use nom_gpui::window::{run_application, ApplicationHandler, Window, WindowEvent, WindowOptions};
use std::time::{Duration, Instant};

struct FirstPaintHarness {
    started: Instant,
    saw_wait: bool,
}

impl FirstPaintHarness {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            saw_wait: false,
        }
    }
}

impl ApplicationHandler for FirstPaintHarness {
    fn resumed(&mut self, window: &mut Window) {
        window.request_redraw();
    }

    fn window_event(&mut self, window: &mut Window, event: WindowEvent) {
        if matches!(event, WindowEvent::CloseRequested) {
            window.request_close();
        }
    }

    fn about_to_wait(&mut self, window: &mut Window) {
        if self.saw_wait && self.started.elapsed() >= Duration::from_millis(1200) {
            window.request_close();
        } else {
            self.saw_wait = true;
            window.request_redraw();
        }
    }
}

fn main() {
    run_application(
        WindowOptions {
            title: "NomCanvas Visual QA".into(),
            size: Vec2::new(640.0, 420.0),
            min_size: Some(Vec2::new(320.0, 240.0)),
            decorations: true,
            transparent: false,
            resizable: false,
        },
        FirstPaintHarness::new(),
    );
}
