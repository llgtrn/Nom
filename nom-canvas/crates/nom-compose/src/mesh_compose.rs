/// MeshVertex — a 3D vertex with position and normal.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
}

impl MeshVertex {
    /// Create a vertex at (x, y, z) with default normal (0, 0, 1).
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, nx: 0.0, ny: 0.0, nz: 1.0 }
    }

    /// Override the normal vector.
    pub fn with_normal(mut self, nx: f32, ny: f32, nz: f32) -> Self {
        self.nx = nx;
        self.ny = ny;
        self.nz = nz;
        self
    }

    /// Euclidean distance from origin: sqrt(x² + y² + z²).
    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Euclidean distance to another vertex.
    pub fn distance_to(&self, other: &MeshVertex) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// MeshFace — a triangular face referencing 3 vertex indices.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshFace {
    pub v0: u32,
    pub v1: u32,
    pub v2: u32,
}

impl MeshFace {
    pub fn new(v0: u32, v1: u32, v2: u32) -> Self {
        Self { v0, v1, v2 }
    }

    /// Return the three indices as an array.
    pub fn indices(&self) -> [u32; 3] {
        [self.v0, self.v1, self.v2]
    }

    /// Return the largest of the three vertex indices.
    pub fn max_index(&self) -> u32 {
        self.v0.max(self.v1).max(self.v2)
    }
}

/// Mesh — a named collection of vertices and triangular faces.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub faces: Vec<MeshFace>,
    pub name: String,
}

impl Mesh {
    pub fn new(name: impl Into<String>) -> Self {
        Self { vertices: Vec::new(), faces: Vec::new(), name: name.into() }
    }

    pub fn add_vertex(&mut self, v: MeshVertex) {
        self.vertices.push(v);
    }

    pub fn add_face(&mut self, f: MeshFace) {
        self.faces.push(f);
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// A mesh is valid when every face index is within bounds.
    pub fn is_valid(&self) -> bool {
        let vc = self.vertex_count() as u32;
        self.faces.iter().all(|f| f.v0 < vc && f.v1 < vc && f.v2 < vc)
    }
}

/// MeshComposer — assembles common mesh primitives.
pub struct MeshComposer;

impl MeshComposer {
    pub fn new() -> Self {
        Self
    }

    /// Build a unit triangle: vertices at (0,0,0), (1,0,0), (0,1,0).
    pub fn create_triangle(&self, name: &str) -> Mesh {
        let mut m = Mesh::new(name);
        m.add_vertex(MeshVertex::new(0.0, 0.0, 0.0));
        m.add_vertex(MeshVertex::new(1.0, 0.0, 0.0));
        m.add_vertex(MeshVertex::new(0.0, 1.0, 0.0));
        m.add_face(MeshFace::new(0, 1, 2));
        m
    }

    /// Build a unit quad (two triangles): corners at (0,0,0),(1,0,0),(1,1,0),(0,1,0).
    pub fn create_quad(&self, name: &str) -> Mesh {
        let mut m = Mesh::new(name);
        m.add_vertex(MeshVertex::new(0.0, 0.0, 0.0));
        m.add_vertex(MeshVertex::new(1.0, 0.0, 0.0));
        m.add_vertex(MeshVertex::new(1.0, 1.0, 0.0));
        m.add_vertex(MeshVertex::new(0.0, 1.0, 0.0));
        m.add_face(MeshFace::new(0, 1, 2));
        m.add_face(MeshFace::new(0, 2, 3));
        m
    }

    /// Return the canonical vertex count for a named primitive.
    pub fn vertex_count_for_primitive(&self, primitive: &str) -> usize {
        match primitive {
            "triangle" => 3,
            "quad" => 4,
            _ => 0,
        }
    }
}

impl Default for MeshComposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod mesh_compose_tests {
    use super::*;

    #[test]
    fn mesh_vertex_magnitude() {
        let v = MeshVertex::new(3.0, 4.0, 0.0);
        let mag = v.magnitude();
        assert!((mag - 5.0).abs() < 1e-5, "magnitude of (3,4,0) must be 5, got {mag}");
    }

    #[test]
    fn mesh_vertex_distance_to() {
        let a = MeshVertex::new(0.0, 0.0, 0.0);
        let b = MeshVertex::new(1.0, 0.0, 0.0);
        let d = a.distance_to(&b);
        assert!((d - 1.0).abs() < 1e-5, "distance must be 1.0, got {d}");
    }

    #[test]
    fn mesh_face_max_index() {
        let f = MeshFace::new(2, 7, 5);
        assert_eq!(f.max_index(), 7);
    }

    #[test]
    fn mesh_add_and_count() {
        let mut m = Mesh::new("test");
        m.add_vertex(MeshVertex::new(0.0, 0.0, 0.0));
        m.add_vertex(MeshVertex::new(1.0, 0.0, 0.0));
        m.add_vertex(MeshVertex::new(0.0, 1.0, 0.0));
        m.add_face(MeshFace::new(0, 1, 2));
        assert_eq!(m.vertex_count(), 3);
        assert_eq!(m.face_count(), 1);
    }

    #[test]
    fn mesh_is_valid_true() {
        let mut m = Mesh::new("valid");
        m.add_vertex(MeshVertex::new(0.0, 0.0, 0.0));
        m.add_vertex(MeshVertex::new(1.0, 0.0, 0.0));
        m.add_vertex(MeshVertex::new(0.0, 1.0, 0.0));
        m.add_face(MeshFace::new(0, 1, 2));
        assert!(m.is_valid(), "mesh with in-bounds indices must be valid");
    }

    #[test]
    fn mesh_is_valid_false() {
        let mut m = Mesh::new("invalid");
        m.add_vertex(MeshVertex::new(0.0, 0.0, 0.0));
        // face references index 5 which does not exist
        m.add_face(MeshFace::new(0, 0, 5));
        assert!(!m.is_valid(), "mesh with out-of-bounds index must be invalid");
    }

    #[test]
    fn mesh_composer_create_triangle() {
        let composer = MeshComposer::new();
        let tri = composer.create_triangle("tri");
        assert_eq!(tri.vertex_count(), 3);
        assert_eq!(tri.face_count(), 1);
        assert!(tri.is_valid());
        assert_eq!(tri.faces[0].indices(), [0, 1, 2]);
    }

    #[test]
    fn mesh_composer_create_quad() {
        let composer = MeshComposer::new();
        let quad = composer.create_quad("quad");
        assert_eq!(quad.vertex_count(), 4);
        assert_eq!(quad.face_count(), 2);
        assert!(quad.is_valid());
    }

    #[test]
    fn mesh_composer_vertex_count_for_primitive() {
        let composer = MeshComposer::new();
        assert_eq!(composer.vertex_count_for_primitive("triangle"), 3);
        assert_eq!(composer.vertex_count_for_primitive("quad"), 4);
        assert_eq!(composer.vertex_count_for_primitive("unknown"), 0);
    }
}
