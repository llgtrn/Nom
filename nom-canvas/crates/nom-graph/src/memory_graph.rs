use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryTier {
    Hot,
    Warm,
    Cold,
    Archive,
}

impl MemoryTier {
    pub fn retention_days(&self) -> u32 {
        match self {
            MemoryTier::Hot => 1,
            MemoryTier::Warm => 7,
            MemoryTier::Cold => 30,
            MemoryTier::Archive => 365,
        }
    }

    pub fn is_accessible(&self) -> bool {
        match self {
            MemoryTier::Hot | MemoryTier::Warm | MemoryTier::Cold => true,
            MemoryTier::Archive => false,
        }
    }
}

pub struct MemoryNode {
    pub id: u64,
    pub tier: MemoryTier,
    pub key: String,
    pub value_hash: u64,
    pub access_count: u32,
}

impl MemoryNode {
    pub fn record_access(&mut self) {
        self.access_count += 1;
    }

    pub fn is_hot(&self) -> bool {
        self.access_count > 10
    }
}

pub struct MemoryEdge {
    pub from_id: u64,
    pub to_id: u64,
    pub strength: f32,
}

impl MemoryEdge {
    pub fn is_strong(&self) -> bool {
        self.strength > 0.7
    }

    pub fn edge_key(&self) -> String {
        format!("{}→{}", self.from_id, self.to_id)
    }
}

pub struct MemoryGraph {
    pub nodes: HashMap<u64, MemoryNode>,
    pub edges: Vec<MemoryEdge>,
}

impl MemoryGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn insert_node(&mut self, n: MemoryNode) {
        self.nodes.insert(n.id, n);
    }

    pub fn add_edge(&mut self, e: MemoryEdge) {
        self.edges.push(e);
    }

    pub fn neighbors(&self, id: u64) -> Vec<&MemoryNode> {
        self.edges
            .iter()
            .filter(|e| e.from_id == id && e.is_strong())
            .filter_map(|e| self.nodes.get(&e.to_id))
            .collect()
    }

    pub fn hot_nodes(&self) -> Vec<&MemoryNode> {
        self.nodes.values().filter(|n| n.is_hot()).collect()
    }
}

impl Default for MemoryGraph {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemoryQuery {
    pub tier_filter: Option<MemoryTier>,
    pub min_access: u32,
}

impl MemoryQuery {
    pub fn matches(&self, n: &MemoryNode) -> bool {
        let tier_ok = match &self.tier_filter {
            Some(t) => t == &n.tier && n.tier.is_accessible(),
            None => true,
        };
        tier_ok && n.access_count >= self.min_access
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_retention_days() {
        assert_eq!(MemoryTier::Hot.retention_days(), 1);
        assert_eq!(MemoryTier::Warm.retention_days(), 7);
        assert_eq!(MemoryTier::Cold.retention_days(), 30);
        assert_eq!(MemoryTier::Archive.retention_days(), 365);
    }

    #[test]
    fn test_tier_is_accessible_archive_false() {
        assert!(MemoryTier::Hot.is_accessible());
        assert!(MemoryTier::Warm.is_accessible());
        assert!(MemoryTier::Cold.is_accessible());
        assert!(!MemoryTier::Archive.is_accessible());
    }

    #[test]
    fn test_node_record_access_and_is_hot() {
        let mut node = MemoryNode {
            id: 1,
            tier: MemoryTier::Hot,
            key: "k".to_string(),
            value_hash: 42,
            access_count: 10,
        };
        assert!(!node.is_hot());
        node.record_access();
        assert_eq!(node.access_count, 11);
        assert!(node.is_hot());
    }

    #[test]
    fn test_edge_is_strong() {
        let strong = MemoryEdge { from_id: 1, to_id: 2, strength: 0.8 };
        let weak = MemoryEdge { from_id: 1, to_id: 3, strength: 0.7 };
        assert!(strong.is_strong());
        assert!(!weak.is_strong());
    }

    #[test]
    fn test_edge_key_format() {
        let edge = MemoryEdge { from_id: 5, to_id: 9, strength: 0.9 };
        assert_eq!(edge.edge_key(), "5→9");
    }

    #[test]
    fn test_graph_insert_and_neighbors_via_strong_edge() {
        let mut graph = MemoryGraph::new();
        graph.insert_node(MemoryNode { id: 1, tier: MemoryTier::Hot, key: "a".to_string(), value_hash: 0, access_count: 0 });
        graph.insert_node(MemoryNode { id: 2, tier: MemoryTier::Warm, key: "b".to_string(), value_hash: 0, access_count: 0 });
        graph.insert_node(MemoryNode { id: 3, tier: MemoryTier::Cold, key: "c".to_string(), value_hash: 0, access_count: 0 });
        // strong edge 1→2, weak edge 1→3
        graph.add_edge(MemoryEdge { from_id: 1, to_id: 2, strength: 0.9 });
        graph.add_edge(MemoryEdge { from_id: 1, to_id: 3, strength: 0.5 });

        let neighbors = graph.neighbors(1);
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].id, 2);
    }

    #[test]
    fn test_graph_hot_nodes() {
        let mut graph = MemoryGraph::new();
        graph.insert_node(MemoryNode { id: 1, tier: MemoryTier::Hot, key: "a".to_string(), value_hash: 0, access_count: 11 });
        graph.insert_node(MemoryNode { id: 2, tier: MemoryTier::Hot, key: "b".to_string(), value_hash: 0, access_count: 5 });

        let hot = graph.hot_nodes();
        assert_eq!(hot.len(), 1);
        assert_eq!(hot[0].id, 1);
    }

    #[test]
    fn test_query_matches_tier_filter() {
        let node_hot = MemoryNode { id: 1, tier: MemoryTier::Hot, key: "x".to_string(), value_hash: 0, access_count: 0 };
        let node_warm = MemoryNode { id: 2, tier: MemoryTier::Warm, key: "y".to_string(), value_hash: 0, access_count: 0 };
        let node_archive = MemoryNode { id: 3, tier: MemoryTier::Archive, key: "z".to_string(), value_hash: 0, access_count: 0 };

        let query = MemoryQuery { tier_filter: Some(MemoryTier::Hot), min_access: 0 };
        assert!(query.matches(&node_hot));
        assert!(!query.matches(&node_warm));

        // Archive with matching tier filter → false because is_accessible() = false
        let query_archive = MemoryQuery { tier_filter: Some(MemoryTier::Archive), min_access: 0 };
        assert!(!query_archive.matches(&node_archive));
    }

    #[test]
    fn test_query_matches_min_access() {
        let node = MemoryNode { id: 1, tier: MemoryTier::Warm, key: "k".to_string(), value_hash: 0, access_count: 5 };

        let q_pass = MemoryQuery { tier_filter: None, min_access: 5 };
        let q_fail = MemoryQuery { tier_filter: None, min_access: 6 };
        assert!(q_pass.matches(&node));
        assert!(!q_fail.matches(&node));
    }
}
