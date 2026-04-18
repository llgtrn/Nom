#![deny(unsafe_code)]

/// Target surface for a screen capture operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenTarget {
    Window,
    Monitor,
    Region,
    Headless,
}

impl ScreenTarget {
    /// Returns true when the target supports user interaction.
    pub fn is_interactive(&self) -> bool {
        matches!(self, ScreenTarget::Window | ScreenTarget::Region)
    }

    /// Returns a canonical lowercase name for the target.
    pub fn target_name(&self) -> &'static str {
        match self {
            ScreenTarget::Window => "window",
            ScreenTarget::Monitor => "monitor",
            ScreenTarget::Region => "region",
            ScreenTarget::Headless => "headless",
        }
    }
}

/// Describes the pixel dimensions and DPI scale of a capture.
#[derive(Debug, Clone)]
pub struct CaptureResolution {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
}

impl CaptureResolution {
    /// Width in logical (CSS/DIP) pixels.
    pub fn logical_width(&self) -> f32 {
        self.width as f32 / self.scale_factor
    }

    /// Total number of physical pixels.
    pub fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// Raw pixel data returned by a capture operation.
#[derive(Debug, Clone)]
pub struct CaptureBuffer {
    pub data: Vec<u8>,
    pub resolution: CaptureResolution,
    pub format: String,
}

impl CaptureBuffer {
    /// Number of bytes stored in `data`.
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }

    /// True when `data` contains no bytes.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Expected byte length for RGBA (4 bytes per pixel).
    pub fn expected_size(&self) -> u64 {
        self.resolution.pixel_count() * 4
    }
}

/// Combines a capture target with the desired resolution.
#[derive(Debug, Clone)]
pub struct ScreenCapture {
    pub target: ScreenTarget,
    pub resolution: CaptureResolution,
}

impl ScreenCapture {
    /// Returns an empty `CaptureBuffer` with this capture's resolution and rgba format.
    pub fn capture_stub(&self) -> CaptureBuffer {
        CaptureBuffer {
            data: Vec::new(),
            resolution: self.resolution.clone(),
            format: "rgba".to_string(),
        }
    }

    /// Human-readable description: `"<target>@<width>x<height>"`.
    pub fn description(&self) -> String {
        format!(
            "{}@{}x{}",
            self.target.target_name(),
            self.resolution.width,
            self.resolution.height
        )
    }
}

/// Collects capture buffers produced during a session.
#[derive(Debug, Default)]
pub struct NativeScreenBackend {
    pub captures: Vec<CaptureBuffer>,
}

impl NativeScreenBackend {
    pub fn add_capture(&mut self, b: CaptureBuffer) {
        self.captures.push(b);
    }

    /// Sum of `size_bytes()` across all stored buffers.
    pub fn total_size_bytes(&self) -> usize {
        self.captures.iter().map(|b| b.size_bytes()).sum()
    }

    /// Count of buffers where `!is_empty()`.
    pub fn non_empty_count(&self) -> usize {
        self.captures.iter().filter(|b| !b.is_empty()).count()
    }
}

#[cfg(test)]
mod native_screen_tests {
    use super::*;

    fn resolution(w: u32, h: u32, scale: f32) -> CaptureResolution {
        CaptureResolution { width: w, height: h, scale_factor: scale }
    }

    #[test]
    fn target_is_interactive() {
        assert!(ScreenTarget::Window.is_interactive());
        assert!(ScreenTarget::Region.is_interactive());
        assert!(!ScreenTarget::Monitor.is_interactive());
        assert!(!ScreenTarget::Headless.is_interactive());
    }

    #[test]
    fn target_target_name() {
        assert_eq!(ScreenTarget::Window.target_name(), "window");
        assert_eq!(ScreenTarget::Monitor.target_name(), "monitor");
        assert_eq!(ScreenTarget::Region.target_name(), "region");
        assert_eq!(ScreenTarget::Headless.target_name(), "headless");
    }

    #[test]
    fn resolution_logical_width() {
        let r = resolution(1920, 1080, 2.0);
        assert!((r.logical_width() - 960.0).abs() < f32::EPSILON);
    }

    #[test]
    fn resolution_pixel_count() {
        let r = resolution(1920, 1080, 1.0);
        assert_eq!(r.pixel_count(), 1920 * 1080);
    }

    #[test]
    fn buffer_size_bytes() {
        let buf = CaptureBuffer {
            data: vec![0u8; 12],
            resolution: resolution(1, 1, 1.0),
            format: "rgba".to_string(),
        };
        assert_eq!(buf.size_bytes(), 12);
    }

    #[test]
    fn buffer_expected_size() {
        let buf = CaptureBuffer {
            data: Vec::new(),
            resolution: resolution(100, 50, 1.0),
            format: "rgba".to_string(),
        };
        assert_eq!(buf.expected_size(), 100 * 50 * 4);
    }

    #[test]
    fn capture_description_format() {
        let cap = ScreenCapture {
            target: ScreenTarget::Window,
            resolution: resolution(1280, 720, 1.0),
        };
        assert_eq!(cap.description(), "window@1280x720");
    }

    #[test]
    fn capture_stub_is_empty() {
        let cap = ScreenCapture {
            target: ScreenTarget::Headless,
            resolution: resolution(800, 600, 1.0),
        };
        let buf = cap.capture_stub();
        assert!(buf.is_empty());
        assert_eq!(buf.format, "rgba");
    }

    #[test]
    fn backend_non_empty_count() {
        let mut backend = NativeScreenBackend::default();
        let res = resolution(4, 4, 1.0);
        backend.add_capture(CaptureBuffer {
            data: vec![0u8; 64],
            resolution: res.clone(),
            format: "rgba".to_string(),
        });
        backend.add_capture(CaptureBuffer {
            data: Vec::new(),
            resolution: res.clone(),
            format: "rgba".to_string(),
        });
        backend.add_capture(CaptureBuffer {
            data: vec![1u8; 32],
            resolution: res.clone(),
            format: "rgba".to_string(),
        });
        assert_eq!(backend.non_empty_count(), 2);
        assert_eq!(backend.total_size_bytes(), 64 + 32);
    }
}
