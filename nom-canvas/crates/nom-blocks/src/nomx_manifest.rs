/// NomxDep — a dependency declared in a .nomx manifest (name, version, content hash).
#[derive(Debug, Clone, PartialEq)]
pub struct NomxDep {
    /// Dependency name.
    pub name: String,
    /// Declared version string.
    pub version: String,
    /// FNV-1a content hash; 0 means unpinned.
    pub content_hash: u64,
}

impl NomxDep {
    /// Create a new dependency entry.
    pub fn new(name: impl Into<String>, version: impl Into<String>, content_hash: u64) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            content_hash,
        }
    }

    /// Returns `true` when `content_hash` is non-zero (i.e. the dep is pinned to a
    /// specific content-addressed artifact).
    pub fn is_pinned(&self) -> bool {
        self.content_hash != 0
    }
}

/// NomxManifest — workspace manifest struct for a .nomx AppManifest.
#[derive(Debug, Clone)]
pub struct NomxManifest {
    /// Manifest name (workspace / app identifier).
    pub name: String,
    /// Manifest version string.
    pub version: String,
    /// Declared dependencies.
    pub deps: Vec<NomxDep>,
}

impl NomxManifest {
    /// Create an empty manifest.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            deps: Vec::new(),
        }
    }

    /// Append a dependency.
    pub fn add_dep(&mut self, dep: NomxDep) {
        self.deps.push(dep);
    }

    /// Total number of dependencies.
    pub fn dep_count(&self) -> usize {
        self.deps.len()
    }

    /// Returns references to all pinned dependencies (where `is_pinned()` is true).
    pub fn pinned_deps(&self) -> Vec<&NomxDep> {
        self.deps.iter().filter(|d| d.is_pinned()).collect()
    }

    /// Serialise to a simple text representation.
    /// Format: `manifest name=X version=Y deps=N`
    pub fn to_nomx_string(&self) -> String {
        format!(
            "manifest name={} version={} deps={}",
            self.name, self.version, self.deps.len()
        )
    }
}

/// NomxModuleEdge — a directed edge in the module dependency graph.
#[derive(Debug, Clone, PartialEq)]
pub struct NomxModuleEdge {
    /// Source module identifier.
    pub from_module: String,
    /// Target module identifier.
    pub to_module: String,
    /// Edge type label (e.g. `"HasFlowArtifact"`).
    pub edge_type: String,
}

impl NomxModuleEdge {
    /// Create a new module edge.
    pub fn new(
        from_module: impl Into<String>,
        to_module: impl Into<String>,
        edge_type: impl Into<String>,
    ) -> Self {
        Self {
            from_module: from_module.into(),
            to_module: to_module.into(),
            edge_type: edge_type.into(),
        }
    }
}

/// NomxModuleGraph — module dependency graph backed by `HasFlowArtifact` edges.
#[derive(Debug, Default)]
pub struct NomxModuleGraph {
    /// Module node names.
    pub nodes: Vec<String>,
    /// Directed edges between modules.
    pub edges: Vec<NomxModuleEdge>,
}

impl NomxModuleGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a module node (duplicates are allowed; deduplication is the caller's concern).
    pub fn add_node(&mut self, name: impl Into<String>) {
        self.nodes.push(name.into());
    }

    /// Append a directed edge.
    pub fn add_edge(&mut self, edge: NomxModuleEdge) {
        self.edges.push(edge);
    }

    /// Number of registered nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// All edges whose `from_module` matches `module`.
    pub fn edges_from(&self, module: &str) -> Vec<&NomxModuleEdge> {
        self.edges
            .iter()
            .filter(|e| e.from_module == module)
            .collect()
    }
}

#[cfg(test)]
mod nomx_manifest_tests {
    use super::*;

    #[test]
    fn nomx_dep_is_pinned_true() {
        let dep = NomxDep::new("nom-core", "0.1.0", 0xdeadbeef_cafebabe);
        assert!(dep.is_pinned(), "dep with non-zero hash must be pinned");
    }

    #[test]
    fn nomx_dep_is_pinned_false() {
        let dep = NomxDep::new("nom-core", "0.1.0", 0);
        assert!(!dep.is_pinned(), "dep with hash=0 must NOT be pinned");
    }

    #[test]
    fn nomx_manifest_add_and_count() {
        let mut m = NomxManifest::new("my-app", "1.0.0");
        assert_eq!(m.dep_count(), 0);
        m.add_dep(NomxDep::new("a", "1.0", 1));
        m.add_dep(NomxDep::new("b", "2.0", 0));
        assert_eq!(m.dep_count(), 2);
    }

    #[test]
    fn nomx_manifest_pinned_deps_filter() {
        let mut m = NomxManifest::new("app", "0.0.1");
        m.add_dep(NomxDep::new("pinned", "1.0", 42));
        m.add_dep(NomxDep::new("floating", "1.0", 0));
        m.add_dep(NomxDep::new("also-pinned", "2.0", 99));
        let pinned = m.pinned_deps();
        assert_eq!(pinned.len(), 2);
        assert!(pinned.iter().all(|d| d.is_pinned()));
    }

    #[test]
    fn nomx_manifest_to_nomx_string() {
        let mut m = NomxManifest::new("workspace", "3.1.4");
        m.add_dep(NomxDep::new("x", "1.0", 1));
        m.add_dep(NomxDep::new("y", "2.0", 2));
        let s = m.to_nomx_string();
        assert_eq!(s, "manifest name=workspace version=3.1.4 deps=2");
    }

    #[test]
    fn nomx_module_graph_add_nodes() {
        let mut g = NomxModuleGraph::new();
        g.add_node("alpha");
        g.add_node("beta");
        g.add_node("gamma");
        assert_eq!(g.node_count(), 3);
    }

    #[test]
    fn nomx_module_graph_add_edge_and_count() {
        let mut g = NomxModuleGraph::new();
        g.add_node("a");
        g.add_node("b");
        g.add_edge(NomxModuleEdge::new("a", "b", "HasFlowArtifact"));
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn nomx_module_graph_edges_from() {
        let mut g = NomxModuleGraph::new();
        g.add_node("src");
        g.add_node("dst1");
        g.add_node("dst2");
        g.add_node("other");
        g.add_edge(NomxModuleEdge::new("src", "dst1", "HasFlowArtifact"));
        g.add_edge(NomxModuleEdge::new("src", "dst2", "HasFlowArtifact"));
        g.add_edge(NomxModuleEdge::new("other", "dst1", "HasFlowArtifact"));
        let from_src = g.edges_from("src");
        assert_eq!(from_src.len(), 2);
        assert!(from_src.iter().all(|e| e.from_module == "src"));
    }

    #[test]
    fn nomx_manifest_new_has_zero_deps() {
        let m = NomxManifest::new("empty", "0.0.0");
        assert_eq!(m.dep_count(), 0, "fresh manifest must have zero deps");
        assert!(m.pinned_deps().is_empty());
    }
}
