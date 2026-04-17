#![deny(unsafe_code)]
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

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

pub struct WebScreenBackend;

impl WebScreenBackend {
    pub fn compose(input: WebScreenInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> WebScreenResult {
        sink.emit(ComposeEvent::Started { backend: "web_screen".into(), entity_id: input.entity.id.clone() });
        // Stub: produce deterministic screenshot bytes from url + dimensions
        let screenshot = format!(
            "[stub] screenshot {}x{} url={}\n",
            input.viewport_w, input.viewport_h, input.url
        );
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "capturing".into() });
        let artifact_hash = store.write(screenshot.as_bytes());
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });
        WebScreenResult { artifact_hash, duration_ms: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;
    #[test]
    fn web_screen_compose_basic() {
        let mut store = InMemoryStore::new();
        let input = WebScreenInput {
            entity: NomtuRef { id: "ws1".into(), word: "homepage".into(), kind: "screen".into() },
            url: "https://example.com".into(),
            viewport_w: 1280,
            viewport_h: 720,
        };
        let result = WebScreenBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&result.artifact_hash));
        let data = store.read(&result.artifact_hash).unwrap();
        assert!(data.starts_with(b"[stub]"));
    }
}
