#![deny(unsafe_code)]
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

pub struct TransformInput {
    pub entity: NomtuRef,
    pub input_hash: [u8; 32],
    pub transform_kind: String,
}

pub struct TransformOutput {
    pub artifact_hash: [u8; 32],
    pub byte_size: u64,
}

pub struct TransformBackend;

impl TransformBackend {
    pub fn compose(input: TransformInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> TransformOutput {
        sink.emit(ComposeEvent::Started { backend: "transform".into(), entity_id: input.entity.id.clone() });
        let source = store.read(&input.input_hash).unwrap_or_default();
        let transformed = Self::apply(&input.transform_kind, &source);
        let artifact_hash = store.write(&transformed);
        let byte_size = transformed.len() as u64;
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });
        TransformOutput { artifact_hash, byte_size }
    }

    fn apply(kind: &str, data: &[u8]) -> Vec<u8> {
        match kind {
            "uppercase" => data.iter().map(|b| b.to_ascii_uppercase()).collect(),
            "lowercase" => data.iter().map(|b| b.to_ascii_lowercase()).collect(),
            "reverse" => { let mut v = data.to_vec(); v.reverse(); v },
            _ => data.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

    #[test]
    fn transform_uppercase() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"hello");
        let out = TransformBackend::compose(TransformInput {
            entity: NomtuRef { id: "t1".into(), word: "xform".into(), kind: "concept".into() },
            input_hash,
            transform_kind: "uppercase".into(),
        }, &mut store, &LogProgressSink);
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"HELLO");
    }

    #[test]
    fn transform_unknown_passthrough() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"data");
        let out = TransformBackend::compose(TransformInput {
            entity: NomtuRef { id: "t2".into(), word: "xform".into(), kind: "concept".into() },
            input_hash,
            transform_kind: "noop".into(),
        }, &mut store, &LogProgressSink);
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"data");
    }
}
