//! Knowledge graph for .nomtu relationships.
//!
//! Builds a graph from NomtuEntry bodies, tracking which functions call
//! which others, what imports they need, and grouping them into semantic
//! communities via label propagation.

use std::collections::{HashMap, HashSet, VecDeque};

use nom_types::NomtuEntry;
use serde::{Deserialize, Serialize};

// ── Node & Edge types ────────────────────────────────────────────────

/// A node in the .nomtu knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomtuNode {
    pub word: String,
    pub variant: Option<String>,
    pub language: String,
    pub kind: String,
    pub body_hash: Option<String>,
}

/// Relationship between two .nomtu entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomtuEdge {
    pub from_word: String,
    pub from_variant: Option<String>,
    pub to_word: String,
    pub to_variant: Option<String>,
    pub edge_type: EdgeType,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeType {
    /// Function calls another function.
    Calls,
    /// Function imports/uses a type or module.
    Imports,
    /// Struct implements a trait/interface.
    Implements,
    /// Requires this to compile.
    DependsOn,
    /// Semantically similar (same concept, different impl).
    SimilarTo,
}

/// A community of related .nomtu entries.
#[derive(Debug, Clone)]
pub struct Community {
    pub id: String,
    pub label: String,
    pub members: Vec<String>,
    pub cohesion: f64,
}

// ── Graph ────────────────────────────────────────────────────────────

/// The .nomtu knowledge graph.
pub struct NomtuGraph {
    nodes: Vec<NomtuNode>,
    edges: Vec<NomtuEdge>,
    /// Index: word -> node indices.
    word_index: HashMap<String, Vec<usize>>,
    /// Bodies keyed by (word, variant) for analysis.
    bodies: HashMap<(String, Option<String>), String>,
    /// Languages keyed by (word, variant).
    languages: HashMap<(String, Option<String>), String>,
}

impl NomtuGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            word_index: HashMap::new(),
            bodies: HashMap::new(),
            languages: HashMap::new(),
        }
    }

    /// Populate the graph from a slice of NomtuEntry.
    pub fn from_entries(entries: &[NomtuEntry]) -> Self {
        let mut graph = Self::new();
        for entry in entries {
            let node = NomtuNode {
                word: entry.word.clone(),
                variant: entry.variant.clone(),
                language: entry.language.clone(),
                kind: entry.kind.clone(),
                body_hash: entry.hash.clone(),
            };
            graph.add_node(node);
            if let Some(body) = &entry.body {
                let key = (entry.word.clone(), entry.variant.clone());
                graph.bodies.insert(key.clone(), body.clone());
                graph.languages.insert(key, entry.language.clone());
            }
        }
        graph
    }

    pub fn add_node(&mut self, node: NomtuNode) {
        let idx = self.nodes.len();
        self.word_index
            .entry(node.word.clone())
            .or_default()
            .push(idx);
        self.nodes.push(node);
    }

    pub fn add_edge(&mut self, edge: NomtuEdge) {
        self.edges.push(edge);
    }

    pub fn nodes(&self) -> &[NomtuNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[NomtuEdge] {
        &self.edges
    }

    // ── Edge builders ────────────────────────────────────────────────

    /// Build edges by analyzing .nomtu bodies for call patterns.
    /// Scans each body for function call patterns (e.g., `foo(`, `bar::baz(`)
    /// and creates Calls edges to matching .nomtu entries.
    pub fn build_call_edges(&mut self) {
        let known_words: HashSet<&str> = self.word_index.keys().map(|s| s.as_str()).collect();
        let bodies: Vec<_> = self
            .bodies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for ((word, variant), body) in &bodies {
            let calls = extract_calls_from_body(body);
            for call in calls {
                // Match against known words
                let call_word = call.split("::").last().unwrap_or(&call);
                if call_word == word {
                    continue; // skip self-calls
                }
                if known_words.contains(call_word) {
                    self.edges.push(NomtuEdge {
                        from_word: word.clone(),
                        from_variant: variant.clone(),
                        to_word: call_word.to_string(),
                        to_variant: None,
                        edge_type: EdgeType::Calls,
                        confidence: 0.8,
                    });
                }
            }
        }
    }

    /// Detect imports from body text (use statements, import statements).
    /// Creates Imports edges.
    pub fn build_import_edges(&mut self) {
        let known_words: HashSet<&str> = self.word_index.keys().map(|s| s.as_str()).collect();
        let bodies: Vec<_> = self
            .bodies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let languages: HashMap<_, _> = self.languages.clone();

        for ((word, variant), body) in &bodies {
            let lang = languages
                .get(&(word.clone(), variant.clone()))
                .map(|s| s.as_str())
                .unwrap_or("rust");
            let imports = extract_imports_from_body(body, lang);
            for (module, symbol) in imports {
                // Try to match the symbol or module against known words
                let target = if !symbol.is_empty() && known_words.contains(symbol.as_str()) {
                    symbol.clone()
                } else if known_words.contains(module.as_str()) {
                    module.clone()
                } else {
                    continue;
                };
                if target == *word {
                    continue; // skip self-imports
                }
                self.edges.push(NomtuEdge {
                    from_word: word.clone(),
                    from_variant: variant.clone(),
                    to_word: target,
                    to_variant: None,
                    edge_type: EdgeType::Imports,
                    confidence: 0.9,
                });
            }
        }
    }

    // ── Queries ──────────────────────────────────────────────────────

    /// Find all dependencies of a .nomtu (transitive closure of Calls + Imports).
    pub fn dependencies(&self, word: &str, variant: Option<&str>) -> Vec<&NomtuNode> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        let start_key = make_key(word, variant);
        visited.insert(start_key.clone());
        queue.push_back(start_key);

        while let Some(current) = queue.pop_front() {
            for edge in &self.edges {
                let from_key = make_key(&edge.from_word, edge.from_variant.as_deref());
                if from_key != current {
                    continue;
                }
                if !matches!(
                    edge.edge_type,
                    EdgeType::Calls | EdgeType::Imports | EdgeType::DependsOn
                ) {
                    continue;
                }
                let to_key = make_key(&edge.to_word, edge.to_variant.as_deref());
                if visited.insert(to_key.clone()) {
                    queue.push_back(to_key);
                }
            }
        }

        // Remove the start node itself
        visited.remove(&make_key(word, variant));

        // Collect matching nodes
        self.nodes
            .iter()
            .filter(|n| visited.contains(&make_key(&n.word, n.variant.as_deref())))
            .collect()
    }

    /// Detect communities using label propagation.
    /// Groups .nomtu that frequently call each other into semantic domains.
    pub fn detect_communities(&self) -> Vec<Community> {
        if self.nodes.is_empty() {
            return Vec::new();
        }

        // Build adjacency list from edges
        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
        for edge in &self.edges {
            let from_indices = self
                .word_index
                .get(&edge.from_word)
                .cloned()
                .unwrap_or_default();
            let to_indices = self
                .word_index
                .get(&edge.to_word)
                .cloned()
                .unwrap_or_default();
            for &fi in &from_indices {
                for &ti in &to_indices {
                    if fi != ti {
                        adj.entry(fi).or_default().push(ti);
                        adj.entry(ti).or_default().push(fi);
                    }
                }
            }
        }

        // Label propagation: each node starts in its own community
        let n = self.nodes.len();
        let mut labels: Vec<usize> = (0..n).collect();

        // Iterate until stable (max 20 iterations)
        for _ in 0..20 {
            let mut changed = false;
            for i in 0..n {
                let neighbors = match adj.get(&i) {
                    Some(ns) => ns,
                    None => continue,
                };
                if neighbors.is_empty() {
                    continue;
                }
                // Find most frequent label among neighbors
                let mut freq: HashMap<usize, usize> = HashMap::new();
                for &nb in neighbors {
                    *freq.entry(labels[nb]).or_default() += 1;
                }
                let best_label = *freq.iter().max_by_key(|&(_, count)| *count).unwrap().0;
                if labels[i] != best_label {
                    labels[i] = best_label;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // Group nodes by label
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, &label) in labels.iter().enumerate() {
            groups.entry(label).or_default().push(i);
        }

        // Build Community structs
        groups
            .into_iter()
            .enumerate()
            .map(|(idx, (_label, member_indices))| {
                let members: Vec<String> = member_indices
                    .iter()
                    .map(|&i| self.nodes[i].word.clone())
                    .collect();
                let total_edges = self.count_internal_edges(&member_indices);
                let max_edges = member_indices.len() * (member_indices.len().saturating_sub(1)) / 2;
                let cohesion = if max_edges > 0 {
                    total_edges as f64 / max_edges as f64
                } else {
                    1.0
                };
                // Label from most common kind in the community
                let label_str = self.dominant_kind(&member_indices);
                Community {
                    id: format!("community-{idx}"),
                    label: label_str,
                    members,
                    cohesion,
                }
            })
            .collect()
    }

    /// Find entry points (nodes with high out-degree, low in-degree).
    pub fn entry_points(&self) -> Vec<&NomtuNode> {
        let mut out_degree: HashMap<String, usize> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for edge in &self.edges {
            *out_degree.entry(edge.from_word.clone()).or_default() += 1;
            *in_degree.entry(edge.to_word.clone()).or_default() += 1;
        }

        let mut candidates: Vec<(&NomtuNode, f64)> = self
            .nodes
            .iter()
            .map(|n| {
                let out = *out_degree.get(&n.word).unwrap_or(&0) as f64;
                let inp = *in_degree.get(&n.word).unwrap_or(&0) as f64;
                // Score: high out, low in
                let score = if inp == 0.0 { out + 1.0 } else { out / inp };
                (n, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.into_iter().map(|(n, _)| n).collect()
    }

    /// Trace an execution flow from an entry point (BFS, max depth).
    pub fn trace_flow(&self, start_word: &str, max_depth: usize) -> Vec<&NomtuNode> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        let mut result: Vec<&NomtuNode> = Vec::new();

        visited.insert(start_word.to_string());
        queue.push_back((start_word.to_string(), 0));

        // Add the start node itself
        if let Some(indices) = self.word_index.get(start_word) {
            if let Some(&idx) = indices.first() {
                result.push(&self.nodes[idx]);
            }
        }

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in &self.edges {
                if edge.from_word != current {
                    continue;
                }
                if !matches!(edge.edge_type, EdgeType::Calls | EdgeType::Imports) {
                    continue;
                }
                if visited.insert(edge.to_word.clone()) {
                    if let Some(indices) = self.word_index.get(&edge.to_word) {
                        if let Some(&idx) = indices.first() {
                            result.push(&self.nodes[idx]);
                        }
                    }
                    queue.push_back((edge.to_word.clone(), depth + 1));
                }
            }
        }

        result
    }

    // ── Helpers ──────────────────────────────────────────────────────

    fn count_internal_edges(&self, member_indices: &[usize]) -> usize {
        let member_words: HashSet<&str> = member_indices
            .iter()
            .map(|&i| self.nodes[i].word.as_str())
            .collect();
        self.edges
            .iter()
            .filter(|e| {
                member_words.contains(e.from_word.as_str())
                    && member_words.contains(e.to_word.as_str())
            })
            .count()
    }

    fn dominant_kind(&self, member_indices: &[usize]) -> String {
        let mut freq: HashMap<&str, usize> = HashMap::new();
        for &i in member_indices {
            *freq.entry(&self.nodes[i].kind).or_default() += 1;
        }
        freq.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(kind, _)| kind.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

impl Default for NomtuGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ── Body analysis helpers ────────────────────────────────────────────

fn make_key(word: &str, variant: Option<&str>) -> String {
    match variant {
        Some(v) => format!("{word}::{v}"),
        None => word.to_string(),
    }
}

/// Extract function call patterns from body text.
fn extract_calls_from_body(body: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' && i > 0 {
            // Walk backwards to find the function name
            let end = i;
            let mut start = i - 1;
            while start > 0
                && (chars[start].is_alphanumeric() || chars[start] == '_' || chars[start] == ':')
            {
                start -= 1;
            }
            // Adjust if we stopped on a non-matching char
            if !chars[start].is_alphanumeric() && chars[start] != '_' && chars[start] != ':' {
                start += 1;
            }
            if start < end {
                let name: String = chars[start..end].iter().collect();
                if !name.is_empty()
                    && name.chars().next().is_some_and(|c| c.is_alphabetic())
                    // Skip common keywords
                    && !matches!(
                        name.as_str(),
                        "if" | "for" | "while" | "match" | "return" | "fn" | "let" | "mut"
                            | "pub" | "impl" | "struct" | "enum" | "use" | "mod" | "crate"
                            | "self" | "Self" | "super" | "where" | "async" | "await"
                    )
                {
                    calls.push(name);
                }
            }
        }
        i += 1;
    }
    calls
}

/// Extract imports from body text based on language.
fn extract_imports_from_body(body: &str, language: &str) -> Vec<(String, String)> {
    let mut imports = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        match language {
            "rust" => {
                if let Some(path) = trimmed
                    .strip_prefix("use ")
                    .and_then(|s| s.strip_suffix(';'))
                {
                    let parts: Vec<&str> = path.split("::").collect();
                    if parts.len() >= 2 {
                        imports.push((parts[0].to_string(), parts.last().unwrap().to_string()));
                    }
                }
            }
            "python" => {
                if let Some(rest) = trimmed.strip_prefix("import ") {
                    imports.push((rest.trim().to_string(), String::new()));
                } else if let Some(rest) = trimmed.strip_prefix("from ") {
                    let parts: Vec<&str> = rest.splitn(2, " import ").collect();
                    if parts.len() == 2 {
                        imports.push((parts[0].to_string(), parts[1].to_string()));
                    }
                }
            }
            "javascript" | "typescript" => {
                if trimmed.contains("import ") || trimmed.contains("require(") {
                    for quote in ['"', '\''] {
                        if let Some(start) = trimmed.find(quote) {
                            if let Some(end) = trimmed[start + 1..].find(quote) {
                                imports.push((
                                    trimmed[start + 1..start + 1 + end].to_string(),
                                    String::new(),
                                ));
                            }
                        }
                    }
                }
            }
            "go" => {
                if trimmed.starts_with("import ") || trimmed.starts_with('"') {
                    if let Some(start) = trimmed.find('"') {
                        if let Some(end) = trimmed[start + 1..].find('"') {
                            imports.push((
                                trimmed[start + 1..start + 1 + end].to_string(),
                                String::new(),
                            ));
                        }
                    }
                }
            }
            "c" | "cpp" => {
                if trimmed.starts_with("#include") {
                    if let Some(start) = trimmed.find(|c: char| c == '<' || c == '"') {
                        if let Some(end) = trimmed[start + 1..].find(|c: char| c == '>' || c == '"')
                        {
                            imports.push((
                                trimmed[start + 1..start + 1 + end].to_string(),
                                String::new(),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    imports
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nom_types::NomtuEntry;

    fn sample_entry(word: &str, variant: Option<&str>, body: &str, language: &str) -> NomtuEntry {
        NomtuEntry {
            word: word.to_string(),
            variant: variant.map(|s| s.to_string()),
            language: language.to_string(),
            kind: "function".to_string(),
            body: Some(body.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn build_call_edges_detects_calls() {
        let entries = vec![
            sample_entry("foo", None, "fn foo() { bar(42); baz(1, 2); }", "rust"),
            sample_entry("bar", None, "fn bar(x: i32) { }", "rust"),
            sample_entry("baz", None, "fn baz(a: i32, b: i32) { }", "rust"),
        ];
        let mut graph = NomtuGraph::from_entries(&entries);
        graph.build_call_edges();

        let call_edges: Vec<_> = graph
            .edges()
            .iter()
            .filter(|e| e.edge_type == EdgeType::Calls)
            .collect();
        assert!(
            call_edges.len() >= 2,
            "expected at least 2 call edges, got {}",
            call_edges.len()
        );

        let targets: Vec<&str> = call_edges.iter().map(|e| e.to_word.as_str()).collect();
        assert!(targets.contains(&"bar"), "expected call to bar");
        assert!(targets.contains(&"baz"), "expected call to baz");
    }

    #[test]
    fn build_import_edges_detects_rust_imports() {
        let entries = vec![
            sample_entry(
                "handler",
                None,
                "use std::io;\nuse crate::auth;\nfn handler() { }",
                "rust",
            ),
            sample_entry("auth", None, "fn auth() { }", "rust"),
        ];
        let mut graph = NomtuGraph::from_entries(&entries);
        graph.build_import_edges();

        let import_edges: Vec<_> = graph
            .edges()
            .iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .collect();
        assert!(
            !import_edges.is_empty(),
            "expected at least one import edge"
        );
        assert_eq!(import_edges[0].to_word, "auth");
    }

    #[test]
    fn dependencies_returns_transitive() {
        let entries = vec![
            sample_entry("a", None, "fn a() { b(1); }", "rust"),
            sample_entry("b", None, "fn b(x: i32) { c(x); }", "rust"),
            sample_entry("c", None, "fn c(x: i32) { }", "rust"),
        ];
        let mut graph = NomtuGraph::from_entries(&entries);
        graph.build_call_edges();

        let deps = graph.dependencies("a", None);
        let dep_words: Vec<&str> = deps.iter().map(|n| n.word.as_str()).collect();
        assert!(dep_words.contains(&"b"), "expected b in deps of a");
        assert!(
            dep_words.contains(&"c"),
            "expected c in deps of a (transitive)"
        );
    }

    #[test]
    fn detect_communities_groups_connected_nodes() {
        let entries = vec![
            sample_entry("a", None, "fn a() { b(1); }", "rust"),
            sample_entry("b", None, "fn b(x: i32) { a(1); }", "rust"),
            sample_entry("c", None, "fn c() { d(1); }", "rust"),
            sample_entry("d", None, "fn d(x: i32) { c(1); }", "rust"),
        ];
        let mut graph = NomtuGraph::from_entries(&entries);
        graph.build_call_edges();

        let communities = graph.detect_communities();
        // a-b should be in one community, c-d in another (or they might merge)
        assert!(!communities.is_empty(), "expected at least one community");
    }

    #[test]
    fn trace_flow_respects_max_depth() {
        let entries = vec![
            sample_entry("a", None, "fn a() { b(1); }", "rust"),
            sample_entry("b", None, "fn b(x: i32) { c(x); }", "rust"),
            sample_entry("c", None, "fn c(x: i32) { d(x); }", "rust"),
            sample_entry("d", None, "fn d(x: i32) { }", "rust"),
        ];
        let mut graph = NomtuGraph::from_entries(&entries);
        graph.build_call_edges();

        let flow = graph.trace_flow("a", 1);
        // depth 1: a + direct callees (b)
        assert!(
            flow.len() <= 2,
            "expected at most 2 nodes at depth 1, got {}",
            flow.len()
        );
    }

    #[test]
    fn entry_points_finds_roots() {
        let entries = vec![
            sample_entry("main", None, "fn main() { handler(1); }", "rust"),
            sample_entry(
                "handler",
                None,
                "fn handler(x: i32) { db_query(x); }",
                "rust",
            ),
            sample_entry("db_query", None, "fn db_query(x: i32) { }", "rust"),
        ];
        let mut graph = NomtuGraph::from_entries(&entries);
        graph.build_call_edges();

        let eps = graph.entry_points();
        assert!(!eps.is_empty());
        // main should be first (highest out/in ratio)
        assert_eq!(eps[0].word, "main");
    }
}
