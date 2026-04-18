#[derive(Debug, Clone, PartialEq)]
pub enum IntentKind {
    Query,
    Compose,
    Transform,
    Route,
    Evaluate,
}

impl IntentKind {
    pub fn is_data_flow(&self) -> bool {
        matches!(self, IntentKind::Query | IntentKind::Transform | IntentKind::Evaluate)
    }

    pub fn kind_code(&self) -> u8 {
        match self {
            IntentKind::Query => 0,
            IntentKind::Compose => 1,
            IntentKind::Transform => 2,
            IntentKind::Route => 3,
            IntentKind::Evaluate => 4,
        }
    }
}

pub struct IntentNode {
    pub id: u64,
    pub kind: IntentKind,
    pub label: String,
    pub weight: f32,
}

impl IntentNode {
    pub fn is_heavy(&self) -> bool {
        self.weight > 0.5
    }

    pub fn node_key(&self) -> String {
        format!("intent:{}:{}", self.id, self.label)
    }
}

pub struct IntentEdge {
    pub from_id: u64,
    pub to_id: u64,
    pub confidence: f32,
    pub label: String,
}

impl IntentEdge {
    pub fn is_confident(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }

    pub fn edge_key(&self) -> String {
        format!("{}->{}", self.from_id, self.to_id)
    }
}

pub struct IntentGraph {
    pub nodes: Vec<IntentNode>,
    pub edges: Vec<IntentEdge>,
}

impl IntentGraph {
    pub fn add_node(&mut self, n: IntentNode) {
        self.nodes.push(n);
    }

    pub fn add_edge(&mut self, e: IntentEdge) {
        self.edges.push(e);
    }

    pub fn neighbors(&self, id: u64) -> Vec<&IntentNode> {
        let to_ids: Vec<u64> = self.edges
            .iter()
            .filter(|e| e.from_id == id)
            .map(|e| e.to_id)
            .collect();
        self.nodes
            .iter()
            .filter(|n| to_ids.contains(&n.id))
            .collect()
    }

    pub fn high_confidence_edges(&self, threshold: f32) -> Vec<&IntentEdge> {
        self.edges.iter().filter(|e| e.is_confident(threshold)).collect()
    }
}

pub struct IntentGraphQuery {
    pub kind_filter: Option<IntentKind>,
    pub min_weight: f32,
}

impl IntentGraphQuery {
    pub fn matches_node(&self, n: &IntentNode) -> bool {
        let kind_ok = match &self.kind_filter {
            Some(k) => &n.kind == k,
            None => true,
        };
        kind_ok && n.weight >= self.min_weight
    }
}

#[cfg(test)]
mod intent_graph_tests {
    use super::*;

    #[test]
    fn kind_is_data_flow() {
        assert!(IntentKind::Query.is_data_flow());
        assert!(IntentKind::Transform.is_data_flow());
        assert!(IntentKind::Evaluate.is_data_flow());
        assert!(!IntentKind::Compose.is_data_flow());
        assert!(!IntentKind::Route.is_data_flow());
    }

    #[test]
    fn kind_code_compose_is_1() {
        assert_eq!(IntentKind::Compose.kind_code(), 1);
    }

    #[test]
    fn node_is_heavy() {
        let heavy = IntentNode { id: 1, kind: IntentKind::Query, label: "a".into(), weight: 0.6 };
        let light = IntentNode { id: 2, kind: IntentKind::Query, label: "b".into(), weight: 0.5 };
        assert!(heavy.is_heavy());
        assert!(!light.is_heavy());
    }

    #[test]
    fn node_node_key() {
        let n = IntentNode { id: 42, kind: IntentKind::Route, label: "router".into(), weight: 0.1 };
        assert_eq!(n.node_key(), "intent:42:router");
    }

    #[test]
    fn edge_is_confident_threshold() {
        let e = IntentEdge { from_id: 1, to_id: 2, confidence: 0.8, label: "link".into() };
        assert!(e.is_confident(0.8));
        assert!(e.is_confident(0.5));
        assert!(!e.is_confident(0.9));
    }

    #[test]
    fn edge_edge_key() {
        let e = IntentEdge { from_id: 3, to_id: 7, confidence: 0.5, label: "x".into() };
        assert_eq!(e.edge_key(), "3->7");
    }

    #[test]
    fn graph_neighbors() {
        let mut g = IntentGraph { nodes: vec![], edges: vec![] };
        g.add_node(IntentNode { id: 1, kind: IntentKind::Query, label: "n1".into(), weight: 0.2 });
        g.add_node(IntentNode { id: 2, kind: IntentKind::Compose, label: "n2".into(), weight: 0.7 });
        g.add_node(IntentNode { id: 3, kind: IntentKind::Route, label: "n3".into(), weight: 0.3 });
        g.add_edge(IntentEdge { from_id: 1, to_id: 2, confidence: 0.9, label: "e1".into() });
        g.add_edge(IntentEdge { from_id: 2, to_id: 3, confidence: 0.4, label: "e2".into() });
        let neighbors = g.neighbors(1);
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].id, 2);
    }

    #[test]
    fn graph_high_confidence_edges_count() {
        let mut g = IntentGraph { nodes: vec![], edges: vec![] };
        g.add_edge(IntentEdge { from_id: 1, to_id: 2, confidence: 0.9, label: "a".into() });
        g.add_edge(IntentEdge { from_id: 2, to_id: 3, confidence: 0.4, label: "b".into() });
        g.add_edge(IntentEdge { from_id: 3, to_id: 4, confidence: 0.75, label: "c".into() });
        let high = g.high_confidence_edges(0.7);
        assert_eq!(high.len(), 2);
    }

    #[test]
    fn query_matches_node_with_kind_filter() {
        let q = IntentGraphQuery { kind_filter: Some(IntentKind::Transform), min_weight: 0.3 };
        let matching = IntentNode { id: 10, kind: IntentKind::Transform, label: "t".into(), weight: 0.5 };
        let wrong_kind = IntentNode { id: 11, kind: IntentKind::Query, label: "q".into(), weight: 0.5 };
        let too_light = IntentNode { id: 12, kind: IntentKind::Transform, label: "t2".into(), weight: 0.1 };
        assert!(q.matches_node(&matching));
        assert!(!q.matches_node(&wrong_kind));
        assert!(!q.matches_node(&too_light));
    }
}
