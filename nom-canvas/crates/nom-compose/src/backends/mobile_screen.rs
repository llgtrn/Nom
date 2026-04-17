#![deny(unsafe_code)]
use crate::backends::ComposeResult;
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;

/// Specification for a mobile screen capture stub.
#[derive(Debug, Clone)]
pub struct MobileScreenSpec {
    pub width: u32,
    pub height: u32,
    /// Platform hint: "ios" | "android"
    pub platform: String,
    pub scale_factor: f32,
}

impl MobileScreenSpec {
    /// Returns the logical (point-based) size: (width / scale_factor, height / scale_factor).
    pub fn logical_size(&self) -> (u32, u32) {
        let sf = self.scale_factor.max(1.0);
        (
            (self.width as f32 / sf) as u32,
            (self.height as f32 / sf) as u32,
        )
    }
}

pub struct MobileScreenBackend;

impl MobileScreenBackend {
    pub fn compose(
        spec: &MobileScreenSpec,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> ComposeResult {
        sink.emit(ComposeEvent::Started {
            backend: "mobile_screen".into(),
            entity_id: spec.platform.clone(),
        });

        let (lw, lh) = spec.logical_size();
        let json = serde_json::json!({
            "width": spec.width,
            "height": spec.height,
            "platform": spec.platform,
            "scale_factor": spec.scale_factor,
            "logical_width": lw,
            "logical_height": lh,
        });
        let bytes = json.to_string().into_bytes();

        sink.emit(ComposeEvent::Progress {
            percent: 0.5,
            stage: "capturing mobile screen".into(),
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
    fn mobile_screen_logical_size() {
        let spec = MobileScreenSpec {
            width: 1170,
            height: 2532,
            platform: "ios".into(),
            scale_factor: 3.0,
        };
        let (lw, lh) = spec.logical_size();
        assert_eq!(lw, 390);
        assert_eq!(lh, 844);
    }

    #[test]
    fn mobile_screen_compose_produces_artifact() {
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 1080,
            height: 1920,
            platform: "android".into(),
            scale_factor: 2.0,
        };
        let result = MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink);
        assert!(result.is_ok());

        let (lw, lh) = spec.logical_size();
        let expected_json = serde_json::json!({
            "width": 1080u32,
            "height": 1920u32,
            "platform": "android",
            "scale_factor": 2.0f32,
            "logical_width": lw,
            "logical_height": lh,
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
    fn mobile_screen_backend_kind() {
        // MobileScreenSpec platform and scale_factor preserved.
        let spec = MobileScreenSpec {
            width: 750,
            height: 1334,
            platform: "ios".into(),
            scale_factor: 2.0,
        };
        assert_eq!(spec.platform, "ios");
        assert!((spec.scale_factor - 2.0).abs() < f32::EPSILON);
        let (lw, lh) = spec.logical_size();
        assert_eq!(lw, 375);
        assert_eq!(lh, 667);
    }

    #[test]
    fn mobile_screen_backend_compose_ok() {
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 360,
            height: 800,
            platform: "android".into(),
            scale_factor: 1.0,
        };
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }
}
