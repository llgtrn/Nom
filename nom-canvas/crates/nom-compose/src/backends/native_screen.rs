#![deny(unsafe_code)]
use crate::backends::ComposeResult;
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;

/// Specification for a native desktop screenshot/capture stub.
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
}
