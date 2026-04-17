#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::NomtuRef;

// ---------------------------------------------------------------------------
// Domain model
// ---------------------------------------------------------------------------

/// Specification for a browser screenshot capture.
#[derive(Debug, Clone)]
pub struct ScreenshotSpec {
    pub url: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub wait_ms: u32,
    pub full_page: bool,
}

impl ScreenshotSpec {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            viewport_width: 1280,
            viewport_height: 720,
            wait_ms: 1000,
            full_page: false,
        }
    }

    /// Total pixel count for the viewport.
    pub fn pixel_count(&self) -> u64 {
        self.viewport_width as u64 * self.viewport_height as u64
    }
}

// ---------------------------------------------------------------------------
// Typed input/result kept for existing callers
// ---------------------------------------------------------------------------

pub struct WebScreenInput {
    pub entity: NomtuRef,
    pub url: String,
    pub viewport_w: u32,
    pub viewport_h: u32,
}

pub struct WebScreenResult {
    pub artifact_hash: [u8; 32],
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Backend
// ---------------------------------------------------------------------------

pub struct WebScreenBackend;

impl WebScreenBackend {
    /// Legacy typed-input compose used by existing callers.
    pub fn compose(
        input: WebScreenInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> WebScreenResult {
        sink.emit(ComposeEvent::Started {
            backend: "web_screen".into(),
            entity_id: input.entity.id.clone(),
        });

        let spec = ScreenshotSpec {
            url: input.url,
            viewport_width: input.viewport_w,
            viewport_height: input.viewport_h,
            wait_ms: 1000,
            full_page: false,
        };

        let json = Self::spec_to_json(&spec);
        sink.emit(ComposeEvent::Progress {
            percent: 0.5,
            stage: "capturing".into(),
        });
        let artifact_hash = store.write(json.as_bytes());
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        });
        WebScreenResult {
            artifact_hash,
            duration_ms: 0,
        }
    }

    /// String-input compose: builds a `ScreenshotSpec` from the URL string,
    /// serialises it to JSON, and writes it to the artifact store.
    pub fn compose_str(
        &self,
        input: &str,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> String {
        sink.emit(ComposeEvent::Started {
            backend: "web_screen".into(),
            entity_id: String::new(),
        });

        let spec = ScreenshotSpec::new(input);
        let json = Self::spec_to_json(&spec);

        sink.emit(ComposeEvent::Progress {
            percent: 0.5,
            stage: "capturing".into(),
        });
        let hash = store.write(json.as_bytes());
        let byte_size = store.byte_size(&hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed {
            artifact_hash: hash,
            byte_size,
        });
        format!(
            "{:x}",
            hash.iter()
                .take(4)
                .fold(0u64, |acc, &b| acc * 256 + b as u64)
        )
    }

    /// Serialise a `ScreenshotSpec` to a JSON string using serde_json.
    fn spec_to_json(spec: &ScreenshotSpec) -> String {
        serde_json::json!({
            "url": spec.url,
            "viewport_width": spec.viewport_width,
            "viewport_height": spec.viewport_height,
            "wait_ms": spec.wait_ms,
            "full_page": spec.full_page,
        })
        .to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::{LogProgressSink, VecProgressSink};
    use crate::store::InMemoryStore;

    #[test]
    fn web_screen_spec_pixel_count() {
        let spec = ScreenshotSpec::new("https://example.com");
        assert_eq!(spec.pixel_count(), 1280 * 720);

        let wide = ScreenshotSpec {
            viewport_width: 1920,
            viewport_height: 1080,
            ..spec.clone()
        };
        assert_eq!(wide.pixel_count(), 1920 * 1080);
    }

    #[test]
    fn web_screen_compose_produces_artifact() {
        let mut store = InMemoryStore::new();
        let result =
            WebScreenBackend.compose_str("https://example.com", &mut store, &LogProgressSink);
        // Must return a non-empty hash hex string.
        assert!(!result.is_empty());
        // The stored payload must contain the URL and be valid JSON.
        let expected_json = serde_json::json!({
            "url": "https://example.com",
            "viewport_width": 1280,
            "viewport_height": 720,
            "wait_ms": 1000,
            "full_page": false,
        })
        .to_string();
        let hash_bytes = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(expected_json.as_bytes());
            let r = h.finalize();
            let mut b = [0u8; 32];
            b.copy_from_slice(&r);
            b
        };
        assert!(store.exists(&hash_bytes));
        let data = store.read(&hash_bytes).unwrap();
        assert_eq!(data, expected_json.as_bytes());
    }

    #[test]
    fn web_screen_compose_emits_progress() {
        let mut store = InMemoryStore::new();
        let sink = VecProgressSink::new();
        WebScreenBackend.compose_str("https://example.com", &mut store, &sink);
        let events = sink.take();
        // Must have at least: Started, Progress, Completed.
        assert!(
            events.len() >= 3,
            "expected >=3 events, got {}",
            events.len()
        );
        assert!(matches!(events[0], ComposeEvent::Started { .. }));
        let has_progress = events
            .iter()
            .any(|e| matches!(e, ComposeEvent::Progress { .. }));
        assert!(has_progress, "no Progress event emitted");
        let has_completed = events
            .iter()
            .any(|e| matches!(e, ComposeEvent::Completed { .. }));
        assert!(has_completed, "no Completed event emitted");
    }

    #[test]
    fn web_screen_backend_kind() {
        // ScreenshotSpec carries "web_screen" semantics; verify defaults.
        let spec = ScreenshotSpec::new("https://nom.dev");
        assert_eq!(spec.viewport_width, 1280);
        assert_eq!(spec.viewport_height, 720);
        assert_eq!(spec.wait_ms, 1000);
        assert!(!spec.full_page);
    }

    #[test]
    fn web_screen_backend_compose_ok() {
        let mut store = InMemoryStore::new();
        let result =
            WebScreenBackend.compose_str("https://nom.dev/canvas", &mut store, &LogProgressSink);
        assert!(!result.is_empty());
    }
}
