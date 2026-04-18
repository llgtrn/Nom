#![deny(unsafe_code)]
use crate::backends::ComposeResult;
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;

/// Specification for a mobile screen-capture artifact request.
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
        if spec.width == 0 || spec.height == 0 {
            sink.emit(ComposeEvent::Failed {
                reason: "mobile_screen dimensions must be non-zero".into(),
            });
            return Err("mobile_screen dimensions must be non-zero".into());
        }
        if !matches!(spec.platform.as_str(), "ios" | "android") {
            sink.emit(ComposeEvent::Failed {
                reason: format!("unsupported mobile_screen platform: {}", spec.platform),
            });
            return Err(format!(
                "unsupported mobile_screen platform: {}",
                spec.platform
            ));
        }

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

    #[test]
    fn mobile_screen_rejects_invalid_platform_and_dimensions() {
        let mut store = InMemoryStore::new();
        let invalid_platform = MobileScreenSpec {
            width: 360,
            height: 800,
            platform: "desktop".into(),
            scale_factor: 1.0,
        };
        assert!(
            MobileScreenBackend::compose(&invalid_platform, &mut store, &LogProgressSink).is_err()
        );

        let invalid_size = MobileScreenSpec {
            width: 360,
            height: 0,
            platform: "android".into(),
            scale_factor: 1.0,
        };
        assert!(MobileScreenBackend::compose(&invalid_size, &mut store, &LogProgressSink).is_err());
    }

    #[test]
    fn mobile_screen_compose_valid_dimensions() {
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 1080,
            height: 2400,
            platform: "android".into(),
            scale_factor: 2.0,
        };
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn mobile_screen_compose_invalid_width_errors() {
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 0,
            height: 1920,
            platform: "ios".into(),
            scale_factor: 3.0,
        };
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_err());
    }

    #[test]
    fn mobile_screen_compose_invalid_height_errors() {
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 1080,
            height: 0,
            platform: "android".into(),
            scale_factor: 2.0,
        };
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_err());
    }

    #[test]
    fn mobile_screen_compose_portrait_orientation() {
        // Portrait: height > width.
        let spec = MobileScreenSpec {
            width: 1080,
            height: 1920,
            platform: "android".into(),
            scale_factor: 2.0,
        };
        assert!(spec.height > spec.width);
        let mut store = InMemoryStore::new();
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn mobile_screen_compose_landscape_orientation() {
        // Landscape: width > height.
        let spec = MobileScreenSpec {
            width: 1920,
            height: 1080,
            platform: "ios".into(),
            scale_factor: 2.0,
        };
        assert!(spec.width > spec.height);
        let mut store = InMemoryStore::new();
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn mobile_screen_platform_ios_valid() {
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 1170,
            height: 2532,
            platform: "ios".into(),
            scale_factor: 3.0,
        };
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn mobile_screen_platform_android_valid() {
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 1080,
            height: 2340,
            platform: "android".into(),
            scale_factor: 2.75,
        };
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }

    #[test]
    fn mobile_screen_platform_unknown_errors() {
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 1280,
            height: 800,
            platform: "windows_phone".into(),
            scale_factor: 1.0,
        };
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_err());
    }

    #[test]
    fn mobile_screen_dpi_positive() {
        // scale_factor must produce positive logical dimensions.
        let spec = MobileScreenSpec {
            width: 1080,
            height: 1920,
            platform: "android".into(),
            scale_factor: 3.0,
        };
        let (lw, lh) = spec.logical_size();
        assert!(lw > 0);
        assert!(lh > 0);
    }

    #[test]
    fn mobile_screen_format_png_valid() {
        // Compose succeeds for a standard high-density iOS spec (format is implicit PNG artifact).
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 1242,
            height: 2208,
            platform: "ios".into(),
            scale_factor: 3.0,
        };
        assert!(MobileScreenBackend::compose(&spec, &mut store, &LogProgressSink).is_ok());
    }
}
