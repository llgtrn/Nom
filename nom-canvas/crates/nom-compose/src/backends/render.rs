#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::NomtuRef;
use std::collections::HashMap;

pub struct RenderInput {
    pub entity: NomtuRef,
    pub template: String,
    pub variables: HashMap<String, String>,
}

pub struct RenderOutput {
    pub artifact_hash: [u8; 32],
    pub rendered: String,
}

pub struct RenderBackend;

impl RenderBackend {
    pub fn compose(
        input: RenderInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> RenderOutput {
        sink.emit(ComposeEvent::Started {
            backend: "render".into(),
            entity_id: input.entity.id.clone(),
        });
        let mut rendered = input.template.clone();
        for (key, value) in &input.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            rendered = rendered.replace(&placeholder, value);
        }
        let artifact_hash = store.write(rendered.as_bytes());
        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size: rendered.len() as u64,
        });
        RenderOutput {
            artifact_hash,
            rendered,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

    #[test]
    fn render_substitution() {
        let mut store = InMemoryStore::new();
        let mut vars = HashMap::new();
        vars.insert("name".into(), "Nom".into());
        vars.insert("version".into(), "1.0".into());
        let out = RenderBackend::compose(
            RenderInput {
                entity: NomtuRef {
                    id: "r1".into(),
                    word: "render".into(),
                    kind: "concept".into(),
                },
                template: "Hello {{name}} v{{version}}!".into(),
                variables: vars,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.rendered, "Hello Nom v1.0!");
        assert!(store.exists(&out.artifact_hash));
    }

    #[test]
    fn render_no_vars() {
        let mut store = InMemoryStore::new();
        let out = RenderBackend::compose(
            RenderInput {
                entity: NomtuRef {
                    id: "r2".into(),
                    word: "render".into(),
                    kind: "concept".into(),
                },
                template: "static content".into(),
                variables: HashMap::new(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.rendered, "static content");
    }
}
