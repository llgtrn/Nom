#![deny(unsafe_code)]
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

pub struct ScenarioScene {
    pub id: String,
    pub description: String,
    pub actors: Vec<String>,
    pub duration_ms: u64,
}

pub struct ScenarioInput {
    pub entity: NomtuRef,
    pub title: String,
    pub scenes: Vec<ScenarioScene>,
}

pub struct ScenarioOutput {
    pub artifact_hash: [u8; 32],
    pub scene_count: usize,
}

pub struct ScenarioBackend;

impl ScenarioBackend {
    pub fn compose(input: ScenarioInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> ScenarioOutput {
        sink.emit(ComposeEvent::Started { backend: "scenario".into(), entity_id: input.entity.id.clone() });
        let total = input.scenes.len();
        let mut content = format!("# {}\n\n", input.title);
        for (i, scene) in input.scenes.iter().enumerate() {
            content.push_str(&format!(
                "## Scene {}: {}\n{}\nActors: {}\nDuration: {}ms\n\n",
                i + 1,
                scene.id,
                scene.description,
                scene.actors.join(", "),
                scene.duration_ms,
            ));
            sink.emit(ComposeEvent::Progress {
                percent: (i + 1) as f32 / total.max(1) as f32,
                stage: scene.id.clone(),
            });
        }
        let hash = store.write(content.as_bytes());
        sink.emit(ComposeEvent::Completed { artifact_hash: hash, byte_size: content.len() as u64 });
        ScenarioOutput { artifact_hash: hash, scene_count: total }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

    #[test]
    fn scenario_compose() {
        let mut store = InMemoryStore::new();
        let out = ScenarioBackend::compose(ScenarioInput {
            entity: NomtuRef { id: "s1".into(), word: "login-flow".into(), kind: "concept".into() },
            title: "Login".into(),
            scenes: vec![ScenarioScene {
                id: "open".into(),
                description: "user opens app".into(),
                actors: vec!["user".into()],
                duration_ms: 500,
            }],
        }, &mut store, &LogProgressSink);
        assert_eq!(out.scene_count, 1);
        assert!(store.exists(&out.artifact_hash));
    }

    #[test]
    fn scenario_empty() {
        let mut store = InMemoryStore::new();
        let out = ScenarioBackend::compose(ScenarioInput {
            entity: NomtuRef { id: "s2".into(), word: "empty".into(), kind: "concept".into() },
            title: "Empty".into(),
            scenes: vec![],
        }, &mut store, &LogProgressSink);
        assert_eq!(out.scene_count, 0);
        assert!(store.exists(&out.artifact_hash));
    }
}
