use nom_gpui::scene::{Quad, Scene};
use nom_gpui::types::{Bounds, Hsla, Pixels, Point, Size};
use nom_gpui::window::{ApplicationHandler, Window, WindowEvent, run_application};

struct FirstPaintApp;

impl ApplicationHandler for FirstPaintApp {
    fn resumed(&mut self, window: &mut Window) {
        window.request_redraw();
    }

    fn window_event(&mut self, window: &mut Window, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            window.request_close();
        }
    }

    fn about_to_wait(&mut self, window: &mut Window) {
        if !window.close_requested() {
            window.request_redraw();
        }
    }

    fn draw(&mut self, _window: &mut Window, scene: &mut Scene) {
        scene.push_quad(Quad {
            bounds: Bounds::new(
                Point::new(Pixels(100.0), Pixels(100.0)),
                Size::new(Pixels(200.0), Pixels(200.0)),
            ),
            background: Some(Hsla::new(0.0, 1.0, 0.5, 1.0)),
            ..Default::default()
        });
    }
}

fn main() {
    let options = nom_gpui::window::WindowOptions {
        title: "AD-RENDER-DEMO — First Paint".to_string(),
        ..nom_gpui::window::WindowOptions::default()
    };
    run_application(options, FirstPaintApp);
}
