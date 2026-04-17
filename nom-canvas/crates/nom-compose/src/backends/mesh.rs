//! 3D mesh composition backend (glTF 2.0 export).
//!
//! Spec + validation only.  Actual glTF serialization lives in a runtime
//! crate that can optionally be swapped between a pure-Rust writer and
//! the `gltf-json` crate.
#![deny(unsafe_code)]

use crate::backend_trait::{
    CompositionBackend, ComposeError, ComposeOutput, ComposeSpec, InterruptFlag, ProgressSink,
};
use crate::kind::NomKind;

// ─── export format ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Gltf,
    Glb,
}

impl ExportFormat {
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Gltf => "model/gltf+json",
            Self::Glb => "model/gltf-binary",
        }
    }

    pub fn is_binary(self) -> bool {
        matches!(self, Self::Glb)
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Gltf => "gltf",
            Self::Glb => "glb",
        }
    }
}

// ─── mesh geometry ───────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub struct MeshGeometry {
    pub name: String,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub has_normals: bool,
    pub has_uvs: bool,
    pub has_colors: bool,
}

impl MeshGeometry {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vertex_count: 0,
            triangle_count: 0,
            has_normals: false,
            has_uvs: false,
            has_colors: false,
        }
    }

    pub fn with_vertex_count(mut self, n: u32) -> Self {
        self.vertex_count = n;
        self
    }

    pub fn with_triangle_count(mut self, n: u32) -> Self {
        self.triangle_count = n;
        self
    }

    pub fn with_normals(mut self) -> Self {
        self.has_normals = true;
        self
    }

    pub fn with_uvs(mut self) -> Self {
        self.has_uvs = true;
        self
    }

    pub fn with_colors(mut self) -> Self {
        self.has_colors = true;
        self
    }

    /// Approximate vertex-buffer size in bytes assuming f32 components:
    /// 3 floats position + 3 floats normal + 2 floats uv + 4 floats color.
    pub fn vertex_buffer_bytes(&self) -> u64 {
        let per_vertex_floats: u64 = 3
            + if self.has_normals { 3 } else { 0 }
            + if self.has_uvs { 2 } else { 0 }
            + if self.has_colors { 4 } else { 0 };
        self.vertex_count as u64 * per_vertex_floats * 4
    }

    /// Index-buffer size in bytes: u32 indices, 3 per triangle.
    pub fn index_buffer_bytes(&self) -> u64 {
        self.triangle_count as u64 * 3 * 4
    }
}

// ─── material ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialRef {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

// ─── animation ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationClip {
    pub name: String,
    pub duration_seconds: f32,
    pub channel_count: u32,
}

// ─── scene spec ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub struct MeshSceneSpec {
    pub meshes: Vec<MeshGeometry>,
    pub materials: Vec<MaterialRef>,
    pub animations: Vec<AnimationClip>,
    pub export: ExportFormat,
    pub origin_name: String,
}

impl MeshSceneSpec {
    pub fn new(origin_name: impl Into<String>) -> Self {
        Self {
            meshes: vec![],
            materials: vec![],
            animations: vec![],
            export: ExportFormat::Glb,
            origin_name: origin_name.into(),
        }
    }

    pub fn add_mesh(&mut self, m: MeshGeometry) {
        self.meshes.push(m);
    }

    pub fn add_material(&mut self, m: MaterialRef) {
        self.materials.push(m);
    }

    pub fn add_animation(&mut self, a: AnimationClip) {
        self.animations.push(a);
    }

    pub fn with_export(mut self, fmt: ExportFormat) -> Self {
        self.export = fmt;
        self
    }

    pub fn total_vertex_bytes(&self) -> u64 {
        self.meshes.iter().map(|m| m.vertex_buffer_bytes()).sum()
    }

    pub fn total_index_bytes(&self) -> u64 {
        self.meshes.iter().map(|m| m.index_buffer_bytes()).sum()
    }
}

// ─── validation errors ───────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum MeshError {
    #[error("empty mesh list (at least one MeshGeometry required)")]
    EmptyMeshList,
    #[error("mesh '{0}' has zero vertices")]
    ZeroVertices(String),
    #[error("mesh '{0}' has zero triangles")]
    ZeroTriangles(String),
}

// ─── validation ──────────────────────────────────────────────────────────────

pub fn validate(spec: &MeshSceneSpec) -> Result<(), MeshError> {
    if spec.meshes.is_empty() {
        return Err(MeshError::EmptyMeshList);
    }
    for m in &spec.meshes {
        if m.vertex_count == 0 {
            return Err(MeshError::ZeroVertices(m.name.clone()));
        }
        if m.triangle_count == 0 {
            return Err(MeshError::ZeroTriangles(m.name.clone()));
        }
    }
    Ok(())
}

// ─── stub backend ────────────────────────────────────────────────────────────

pub struct StubMeshBackend;

impl CompositionBackend for StubMeshBackend {
    fn kind(&self) -> NomKind {
        NomKind::Media3D
    }

    fn name(&self) -> &str {
        "stub-mesh"
    }

    fn compose(
        &self,
        _spec: &ComposeSpec,
        _progress: &dyn ProgressSink,
        _interrupt: &InterruptFlag,
    ) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput {
            bytes: Vec::new(),
            mime_type: "model/gltf-binary".to_string(),
            cost_cents: 0,
        })
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_trait::InterruptFlag;

    struct NoopSink;
    impl ProgressSink for NoopSink {
        fn notify(&self, _percent: u32, _message: &str) {}
    }

    // ExportFormat
    #[test]
    fn export_format_gltf_mime() {
        assert_eq!(ExportFormat::Gltf.mime_type(), "model/gltf+json");
    }

    #[test]
    fn export_format_glb_mime() {
        assert_eq!(ExportFormat::Glb.mime_type(), "model/gltf-binary");
    }

    #[test]
    fn export_format_is_binary() {
        assert!(!ExportFormat::Gltf.is_binary());
        assert!(ExportFormat::Glb.is_binary());
    }

    #[test]
    fn export_format_extension() {
        assert_eq!(ExportFormat::Gltf.extension(), "gltf");
        assert_eq!(ExportFormat::Glb.extension(), "glb");
    }

    // MeshGeometry defaults
    #[test]
    fn mesh_geometry_new_defaults() {
        let m = MeshGeometry::new("cube");
        assert_eq!(m.name, "cube");
        assert_eq!(m.vertex_count, 0);
        assert_eq!(m.triangle_count, 0);
        assert!(!m.has_normals);
        assert!(!m.has_uvs);
        assert!(!m.has_colors);
    }

    // Builder chain
    #[test]
    fn mesh_geometry_builder_chain() {
        let m = MeshGeometry::new("sphere")
            .with_vertex_count(64)
            .with_triangle_count(128)
            .with_normals()
            .with_uvs()
            .with_colors();
        assert_eq!(m.vertex_count, 64);
        assert_eq!(m.triangle_count, 128);
        assert!(m.has_normals);
        assert!(m.has_uvs);
        assert!(m.has_colors);
    }

    // vertex_buffer_bytes base: 100 verts * 3 floats * 4 bytes = 1200
    #[test]
    fn vertex_buffer_bytes_base() {
        let m = MeshGeometry::new("t").with_vertex_count(100);
        assert_eq!(m.vertex_buffer_bytes(), 1200);
    }

    // vertex_buffer_bytes with all flags: 100 * (3+3+2+4) * 4 = 100*12*4 = 4800
    #[test]
    fn vertex_buffer_bytes_all_flags() {
        let m = MeshGeometry::new("t")
            .with_vertex_count(100)
            .with_normals()
            .with_uvs()
            .with_colors();
        assert_eq!(m.vertex_buffer_bytes(), 4800);
    }

    // index_buffer_bytes: 10 triangles * 3 * 4 = 120
    #[test]
    fn index_buffer_bytes_arithmetic() {
        let m = MeshGeometry::new("t").with_triangle_count(10);
        assert_eq!(m.index_buffer_bytes(), 120);
    }

    // MeshSceneSpec defaults
    #[test]
    fn mesh_scene_spec_new_defaults() {
        let s = MeshSceneSpec::new("scene");
        assert_eq!(s.origin_name, "scene");
        assert!(s.meshes.is_empty());
        assert!(s.materials.is_empty());
        assert!(s.animations.is_empty());
        assert_eq!(s.export, ExportFormat::Glb);
    }

    // add_mesh / add_material / add_animation
    #[test]
    fn mesh_scene_spec_add_entries() {
        let mut s = MeshSceneSpec::new("s");
        s.add_mesh(MeshGeometry::new("m1").with_vertex_count(4).with_triangle_count(2));
        s.add_material(MaterialRef {
            name: "mat".into(),
            base_color: [1.0, 0.0, 0.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
        });
        s.add_animation(AnimationClip {
            name: "walk".into(),
            duration_seconds: 1.2,
            channel_count: 3,
        });
        assert_eq!(s.meshes.len(), 1);
        assert_eq!(s.materials.len(), 1);
        assert_eq!(s.animations.len(), 1);
    }

    // with_export
    #[test]
    fn mesh_scene_spec_with_export() {
        let s = MeshSceneSpec::new("s").with_export(ExportFormat::Gltf);
        assert_eq!(s.export, ExportFormat::Gltf);
    }

    // total_vertex_bytes + total_index_bytes across two meshes
    #[test]
    fn total_bytes_sum() {
        let mut s = MeshSceneSpec::new("s");
        // mesh A: 100 verts * 3 * 4 = 1200 verts; 10 tris * 3 * 4 = 120 idx
        s.add_mesh(MeshGeometry::new("a").with_vertex_count(100).with_triangle_count(10));
        // mesh B: 50 verts * 3 * 4 = 600 verts; 20 tris * 3 * 4 = 240 idx
        s.add_mesh(MeshGeometry::new("b").with_vertex_count(50).with_triangle_count(20));
        assert_eq!(s.total_vertex_bytes(), 1800);
        assert_eq!(s.total_index_bytes(), 360);
    }

    // validate Ok
    #[test]
    fn validate_ok() {
        let mut s = MeshSceneSpec::new("s");
        s.add_mesh(MeshGeometry::new("m").with_vertex_count(3).with_triangle_count(1));
        assert!(validate(&s).is_ok());
    }

    // validate EmptyMeshList
    #[test]
    fn validate_empty_mesh_list() {
        let s = MeshSceneSpec::new("s");
        let err = validate(&s).unwrap_err();
        assert!(matches!(err, MeshError::EmptyMeshList));
        assert!(err.to_string().contains("empty mesh list"));
    }

    // validate ZeroVertices
    #[test]
    fn validate_zero_vertices() {
        let mut s = MeshSceneSpec::new("s");
        s.add_mesh(MeshGeometry::new("bad").with_triangle_count(1));
        let err = validate(&s).unwrap_err();
        assert!(matches!(err, MeshError::ZeroVertices(_)));
        assert!(err.to_string().contains("bad"));
    }

    // validate ZeroTriangles
    #[test]
    fn validate_zero_triangles() {
        let mut s = MeshSceneSpec::new("s");
        s.add_mesh(MeshGeometry::new("bad").with_vertex_count(3));
        let err = validate(&s).unwrap_err();
        assert!(matches!(err, MeshError::ZeroTriangles(_)));
        assert!(err.to_string().contains("bad"));
    }

    // StubMeshBackend
    #[test]
    fn stub_mesh_backend_kind_and_name() {
        let b = StubMeshBackend;
        assert_eq!(b.kind(), NomKind::Media3D);
        assert_eq!(b.name(), "stub-mesh");
    }

    #[test]
    fn stub_mesh_backend_compose() {
        let b = StubMeshBackend;
        let spec = ComposeSpec {
            kind: NomKind::Media3D,
            params: vec![],
        };
        let sink = NoopSink;
        let flag = InterruptFlag::new();
        let out = b.compose(&spec, &sink, &flag).unwrap();
        assert_eq!(out.mime_type, "model/gltf-binary");
        assert_eq!(out.cost_cents, 0);
        assert!(out.bytes.is_empty());
    }
}
