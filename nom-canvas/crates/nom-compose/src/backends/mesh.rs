#![deny(unsafe_code)]
use crate::backends::ComposeResult;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

/// Primitive topology for a mesh.
#[derive(Debug, Clone, PartialEq)]
pub enum MeshPrimitive {
    Triangles,
    Lines,
    Points,
}

/// Specification for a glTF/OBJ/PLY mesh geometry artifact.
#[derive(Debug, Clone)]
pub struct MeshSpec {
    pub name: String,
    pub vertex_count: usize,
    pub face_count: usize,
    pub primitive: MeshPrimitive,
    /// Format hint: "gltf" | "obj" | "ply"
    pub format: String,
}

impl MeshSpec {
    /// Returns face_count for triangle meshes; 0 for non-triangle primitives.
    pub fn triangle_count(&self) -> usize {
        if self.primitive == MeshPrimitive::Triangles {
            self.face_count
        } else {
            0
        }
    }
}

pub struct MeshBackend;

impl MeshBackend {
    pub fn compose(spec: &MeshSpec, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> ComposeResult {
        sink.emit(ComposeEvent::Started { backend: "mesh".into(), entity_id: spec.name.clone() });

        let json = serde_json::json!({
            "name": spec.name,
            "vertex_count": spec.vertex_count,
            "face_count": spec.face_count,
            "primitive": format!("{:?}", spec.primitive),
            "format": spec.format,
        });
        let bytes = json.to_string().into_bytes();

        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "serializing mesh spec".into() });
        let artifact_hash = store.write(&bytes);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

    #[test]
    fn mesh_spec_triangle_count() {
        let tri = MeshSpec {
            name: "cube".into(),
            vertex_count: 8,
            face_count: 12,
            primitive: MeshPrimitive::Triangles,
            format: "gltf".into(),
        };
        assert_eq!(tri.triangle_count(), 12);

        let lines = MeshSpec { primitive: MeshPrimitive::Lines, ..tri.clone() };
        assert_eq!(lines.triangle_count(), 0);

        let points = MeshSpec { primitive: MeshPrimitive::Points, ..tri.clone() };
        assert_eq!(points.triangle_count(), 0);
    }

    #[test]
    fn mesh_compose_produces_artifact() {
        let mut store = InMemoryStore::new();
        let spec = MeshSpec {
            name: "sphere".into(),
            vertex_count: 100,
            face_count: 196,
            primitive: MeshPrimitive::Triangles,
            format: "obj".into(),
        };
        let result = MeshBackend::compose(&spec, &mut store, &LogProgressSink);
        assert!(result.is_ok());

        let expected_json = serde_json::json!({
            "name": "sphere",
            "vertex_count": 100,
            "face_count": 196,
            "primitive": "Triangles",
            "format": "obj",
        }).to_string();

        use sha2::{Sha256, Digest};
        let mut h = Sha256::new();
        h.update(expected_json.as_bytes());
        let r = h.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&r);
        assert!(store.exists(&hash));
    }
}
