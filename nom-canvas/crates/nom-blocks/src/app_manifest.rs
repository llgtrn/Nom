//! AppManifest — Cargo-style dependency manifest for workspace .nomx manifests.
//!
//! Models a manifest with named deps (each carrying a FNV-1a hash), and a
//! `ManifestGraph` that aggregates multiple manifests via HasFlowArtifact edges.

/// A single dependency declared in a workspace manifest.
pub struct ManifestDep {
    /// Dependency name.
    pub name: String,
    /// Declared version string.
    pub version: String,
    /// FNV-1a hash of name+version bytes.
    pub hash: u64,
}

impl ManifestDep {
    /// Create a new `ManifestDep`, computing the FNV-1a hash over `name+version`.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        let name = name.into();
        let version = version.into();
        let hash = fnv1a(name.as_bytes().iter().chain(version.as_bytes().iter()).copied());
        Self { name, version, hash }
    }

    /// Return the version string as a `&str`.
    pub fn version_str(&self) -> &str {
        &self.version
    }
}

/// Application manifest for a .nomx workspace entry.
pub struct AppManifest {
    /// Application name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// FNV-1a hash of the application name (entry hash).
    pub entry_hash: u64,
    /// Declared dependencies.
    pub deps: Vec<ManifestDep>,
}

impl AppManifest {
    /// Create a new `AppManifest`; `entry_hash` is FNV-1a of `name`.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        let name = name.into();
        let version = version.into();
        let entry_hash = fnv1a(name.as_bytes().iter().copied());
        Self { name, version, entry_hash, deps: Vec::new() }
    }

    /// Append a dependency.
    pub fn add_dep(&mut self, dep: ManifestDep) {
        self.deps.push(dep);
    }

    /// Number of declared dependencies.
    pub fn dep_count(&self) -> usize {
        self.deps.len()
    }

    /// Find a dependency by exact name.
    pub fn find_dep(&self, name: &str) -> Option<&ManifestDep> {
        self.deps.iter().find(|d| d.name == name)
    }

    /// Render as a .nomx header line: `app <name> v<version> [<dep_count>]`.
    pub fn to_nomx_header(&self) -> String {
        format!("app {} v{} [{}]", self.name, self.version, self.dep_count())
    }
}

/// Module graph that aggregates manifests via HasFlowArtifact edges.
pub struct ManifestGraph {
    /// All manifests tracked in this graph.
    pub manifests: Vec<AppManifest>,
}

impl ManifestGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self { manifests: Vec::new() }
    }

    /// Add a manifest to the graph.
    pub fn add_manifest(&mut self, manifest: AppManifest) {
        self.manifests.push(manifest);
    }

    /// Number of manifests in the graph.
    pub fn manifest_count(&self) -> usize {
        self.manifests.len()
    }

    /// Find a manifest by exact application name.
    pub fn find_by_name(&self, name: &str) -> Option<&AppManifest> {
        self.manifests.iter().find(|m| m.name == name)
    }

    /// Sum of all `dep_count()` values across every manifest.
    pub fn total_deps(&self) -> usize {
        self.manifests.iter().map(|m| m.dep_count()).sum()
    }
}

impl Default for ManifestGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// FNV-1a hash over an iterator of bytes.
fn fnv1a(bytes: impl Iterator<Item = u8>) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

#[cfg(test)]
mod app_manifest_tests {
    use super::*;

    #[test]
    fn manifest_dep_new_fields() {
        let dep = ManifestDep::new("serde", "1.0.0");
        assert_eq!(dep.name, "serde");
        assert_eq!(dep.version, "1.0.0");
    }

    #[test]
    fn manifest_dep_fnv_hash_is_deterministic() {
        let dep1 = ManifestDep::new("nom-core", "0.2.0");
        let dep2 = ManifestDep::new("nom-core", "0.2.0");
        assert_eq!(dep1.hash, dep2.hash);
        // Different input must produce different hash
        let dep3 = ManifestDep::new("nom-core", "0.3.0");
        assert_ne!(dep1.hash, dep3.hash);
    }

    #[test]
    fn app_manifest_new_fields() {
        let m = AppManifest::new("my-app", "1.2.3");
        assert_eq!(m.name, "my-app");
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.dep_count(), 0);
        // entry_hash must be non-zero for a non-empty name
        assert_ne!(m.entry_hash, 0);
    }

    #[test]
    fn add_dep_increments_dep_count() {
        let mut m = AppManifest::new("app", "0.1.0");
        assert_eq!(m.dep_count(), 0);
        m.add_dep(ManifestDep::new("tokio", "1.0.0"));
        assert_eq!(m.dep_count(), 1);
        m.add_dep(ManifestDep::new("serde", "1.0.0"));
        assert_eq!(m.dep_count(), 2);
    }

    #[test]
    fn find_dep_found() {
        let mut m = AppManifest::new("app", "0.1.0");
        m.add_dep(ManifestDep::new("tokio", "1.0.0"));
        let found = m.find_dep("tokio");
        assert!(found.is_some());
        assert_eq!(found.unwrap().version_str(), "1.0.0");
    }

    #[test]
    fn find_dep_not_found() {
        let m = AppManifest::new("app", "0.1.0");
        assert!(m.find_dep("missing").is_none());
    }

    #[test]
    fn to_nomx_header_format() {
        let mut m = AppManifest::new("canvas", "2.0.0");
        m.add_dep(ManifestDep::new("blocks", "1.0.0"));
        m.add_dep(ManifestDep::new("graph", "0.5.0"));
        assert_eq!(m.to_nomx_header(), "app canvas v2.0.0 [2]");
    }

    #[test]
    fn manifest_graph_total_deps() {
        let mut graph = ManifestGraph::new();
        let mut a = AppManifest::new("app-a", "1.0.0");
        a.add_dep(ManifestDep::new("x", "1.0.0"));
        a.add_dep(ManifestDep::new("y", "1.0.0"));
        let mut b = AppManifest::new("app-b", "2.0.0");
        b.add_dep(ManifestDep::new("z", "3.0.0"));
        graph.add_manifest(a);
        graph.add_manifest(b);
        assert_eq!(graph.total_deps(), 3);
    }

    #[test]
    fn manifest_graph_find_by_name() {
        let mut graph = ManifestGraph::new();
        graph.add_manifest(AppManifest::new("alpha", "0.1.0"));
        graph.add_manifest(AppManifest::new("beta", "0.2.0"));
        let found = graph.find_by_name("beta");
        assert!(found.is_some());
        assert_eq!(found.unwrap().version, "0.2.0");
        assert!(graph.find_by_name("gamma").is_none());
    }
}
