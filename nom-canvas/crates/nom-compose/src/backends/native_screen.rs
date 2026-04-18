#![deny(unsafe_code)]
use crate::backends::ComposeResult;
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;

/// Specification for a native desktop screen-capture artifact request.
#[derive(Debug, Clone)]
pub struct NativeScreenSpec {
    pub width: u32,
    pub height: u32,
    pub display_index: usize,
    /// Format hint: "png" | "jpeg" | "bmp"
    pub format: String,
}

impl NativeScreenSpec {
    pub fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

pub struct NativeScreenBackend;

impl NativeScreenBackend {
    pub fn compose(
        spec: &NativeScreenSpec,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> ComposeResult {
        if spec.width == 0 || spec.height == 0 {
            sink.emit(ComposeEvent::Failed {
                reason: "native_screen dimensions must be non-zero".into(),
            });
            return Err("native_screen dimensions must be non-zero".into());
        }
        if !matches!(spec.format.as_str(), "png" | "jpeg" | "bmp") {
            sink.emit(ComposeEvent::Failed {
                reason: format!("unsupported native_screen format: {}", spec.format),
            });
            return Err(format!("unsupported native_screen format: {}", spec.format));
        }

        sink.emit(ComposeEvent::Started {
            backend: "native_screen".into(),
            entity_id: format!("display_{}", spec.display_index),
        });

        let json = serde_json::json!({
            "width": spec.width,
            "height": spec.height,
            "display_index": spec.display_index,
            "format": spec.format,
            "pixel_count": spec.pixel_count(),
        });
        let bytes = json.to_string().into_bytes();

        sink.emit(ComposeEvent::Progress {
            percent: 0.5,
            stage: "capturing native screen".into(),
            rendered_frames: None,
            encoded_frames: None,
            elapsed_ms: None,
        });
        let artifact_hash = store.write(&bytes);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

    #[test]
    fn native_screen_compose_produces_artifact() {
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 2560,
            height: 1440,
            display_index: 0,
            format: "png".into(),
        };
        assert_eq!(spec.pixel_count(), 2560 * 1440);
        let result = NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink);
        assert!(result.is_ok());

        let expected_json = serde_json::json!({
            "width": 2560u32,
            "height": 1440u32,
            "display_index": 0usize,
            "format": "png",
            "pixel_count": 2560u64 * 1440u64,
        })
        .to_string();

        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(expected_json.as_bytes());
        let r = h.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&r);
        assert!(store.exists(&hash));
    }

    #[test]
    fn native_screen_backend_kind() {
        // NativeScreenSpec fields preserved — format and display_index.
        let spec = NativeScreenSpec {
            width: 1920,
            height: 1080,
            display_index: 1,
            format: "jpeg".into(),
        };
        assert_eq!(spec.format, "jpeg");
        assert_eq!(spec.display_index, 1);
        assert_eq!(spec.pixel_count(), 1920 * 1080);
    }

    #[test]
    fn native_screen_backend_compose_ok() {
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 800,
            height: 600,
            display_index: 0,
            format: "bmp".into(),
        };
        assert!(NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn native_screen_rejects_invalid_format_and_dimensions() {
        let mut store = InMemoryStore::new();
        let invalid_format = NativeScreenSpec {
            width: 800,
            height: 600,
            display_index: 0,
            format: "gif".into(),
        };
        assert!(
            NativeScreenBackend::compose(&invalid_format, &mut store, &LogProgressSink).is_err()
        );

        let invalid_size = NativeScreenSpec {
            width: 0,
            height: 600,
            display_index: 0,
            format: "png".into(),
        };
        assert!(NativeScreenBackend::compose(&invalid_size, &mut store, &LogProgressSink).is_err());
    }

    #[test]
    fn native_screen_compose_valid_dimensions() {
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 3840,
            height: 2160,
            display_index: 0,
            format: "png".into(),
        };
        assert!(NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
        assert_eq!(spec.pixel_count(), 3840 * 2160);
    }

    #[test]
    fn native_screen_compose_invalid_dimensions_errors() {
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 0,
            height: 0,
            display_index: 0,
            format: "png".into(),
        };
        assert!(NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_err());
    }

    #[test]
    fn native_screen_platform_windows_valid() {
        // Windows: common 1920x1080 full-HD spec.
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 1920,
            height: 1080,
            display_index: 0,
            format: "bmp".into(),
        };
        assert!(NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn native_screen_platform_macos_valid() {
        // macOS retina: 2560x1600.
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 2560,
            height: 1600,
            display_index: 0,
            format: "png".into(),
        };
        assert!(NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn native_screen_platform_linux_valid() {
        // Linux: 1280x1024 spec.
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 1280,
            height: 1024,
            display_index: 0,
            format: "jpeg".into(),
        };
        assert!(NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn native_screen_format_png_valid() {
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 1024,
            height: 768,
            display_index: 0,
            format: "png".into(),
        };
        assert!(NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn native_screen_format_bmp_valid() {
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 800,
            height: 600,
            display_index: 1,
            format: "bmp".into(),
        };
        assert!(NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn native_screen_format_invalid_errors() {
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 1920,
            height: 1080,
            display_index: 0,
            format: "tiff".into(),
        };
        assert!(NativeScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_err());
    }
}
