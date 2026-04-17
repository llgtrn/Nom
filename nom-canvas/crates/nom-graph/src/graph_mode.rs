#![deny(unsafe_code)]

use std::collections::HashMap;
use crate::dag::Dag;
use crate::node::NodeId;

// ---------------------------------------------------------------------------
// View mode
// ---------------------------------------------------------------------------

/// Which view the canvas is currently showing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphViewMode {
    /// Full canvas / free-layout view.
    Canvas,
    /// Pure graph / DAG view.
    Graph,
    /// Canvas and graph side-by-side.
    Split,
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Maps each node to a 2-D position (x, y) on the canvas plane.
pub type GraphLayout = HashMap<NodeId, (f32, f32)>;

/// Compute a grid layout from a DAG using a topological sort.
///
/// Nodes are sorted topologically; within each depth level they are placed
/// left-to-right by their index in the sorted order.
///
/// Position formula:
///   x = topo_index * 120.0
///   y = depth * 80.0
///
/// "Depth" is the longest path from any source node.  For a fallen-back
/// linear sort (cycle-free), we compute depths via BFS from roots.
/// If the DAG contains a cycle topological_sort returns Err; we fall back
/// to index-only placement (y = 0.0 for all).
pub fn layout_dag(dag: &Dag) -> GraphLayout {
    let topo = match dag.topological_sort() {
        Ok(order) => order,
        Err(all) => {
            // Cycle fallback: grid row 0, columns by node id sorted
            let mut ids: Vec<NodeId> = all;
            ids.sort();
            return ids
                .into_iter()
                .enumerate()
                .map(|(i, id)| (id, (i as f32 * 120.0, 0.0)))
                .collect();
        }
    };

    // Build adjacency (src -> [dst]) from edges for depth computation.
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &dag.edges {
        children
            .entry(edge.src_node.as_str())
            .or_default()
            .push(edge.dst_node.as_str());
    }

    // Longest-path depth via single forward pass over the topo order.
    let mut depth: HashMap<&str, usize> = HashMap::new();
    for id in &topo {
        let d = *depth.get(id.as_str()).unwrap_or(&0);
        if let Some(dsts) = children.get(id.as_str()) {
            for dst in dsts {
                let entry = depth.entry(dst).or_insert(0);
                if *entry < d + 1 {
                    *entry = d + 1;
                }
            }
        }
    }

    topo.into_iter()
        .enumerate()
        .map(|(i, id)| {
            let d = depth.get(id.as_str()).copied().unwrap_or(0);
            (id, (i as f32 * 120.0, d as f32 * 80.0))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Full UI state for graph-mode interaction.
#[derive(Clone, Debug)]
pub struct GraphModeState {
    pub mode: GraphViewMode,
    pub layout: GraphLayout,
    /// Currently selected node, if any.
    pub selected: Option<NodeId>,
    /// Node under the pointer, if any.
    pub hovered: Option<NodeId>,
}

impl GraphModeState {
    /// Create a new state from a DAG.  Starts in `Graph` mode with a
    /// freshly computed layout and nothing selected or hovered.
    pub fn new(dag: &Dag) -> Self {
        Self {
            mode: GraphViewMode::Graph,
            layout: layout_dag(dag),
            selected: None,
            hovered: None,
        }
    }

    /// Set the selected node.
    pub fn select(&mut self, id: NodeId) {
        self.selected = Some(id);
    }

    /// Set (or clear) the hovered node.
    pub fn hover(&mut self, id: Option<NodeId>) {
        self.hovered = id;
    }

    /// Find the nearest node in `layout` whose position is within `radius`
    /// of the point `(x, y)`.  Returns the node's id, or `None` if no node
    /// is close enough.
    pub fn node_at_point(layout: &GraphLayout, x: f32, y: f32, radius: f32) -> Option<NodeId> {
        let r2 = radius * radius;
        let mut best: Option<(f32, &NodeId)> = None;
        for (id, &(nx, ny)) in layout {
            let dx = nx - x;
            let dy = ny - y;
            let dist2 = dx * dx + dy * dy;
            if dist2 <= r2 {
                match best {
                    None => best = Some((dist2, id)),
                    Some((bd, _)) if dist2 < bd => best = Some((dist2, id)),
                    _ => {}
                }
            }
        }
        best.map(|(_, id)| id.clone())
    }
}

// ---------------------------------------------------------------------------
// NomtuRef carrier — every graph node may optionally reference a nomtu entry
// ---------------------------------------------------------------------------

/// A thin wrapper that pairs an `ExecNode` id with an optional nomtu entry
/// reference (a 64-bit hash).  This lives here rather than in `node.rs` so
/// that the DAG execution layer stays free of canvas/UI concerns.
#[derive(Clone, Debug, Default)]
pub struct NomtuRef {
    pub node_id: NodeId,
    pub nomtu_ref: Option<u64>,
}

impl NomtuRef {
    pub fn new(node_id: impl Into<String>, nomtu_ref: Option<u64>) -> Self {
        Self { node_id: node_id.into(), nomtu_ref }
    }
}

// ---------------------------------------------------------------------------
// Canvas events
// ---------------------------------------------------------------------------

/// Events emitted by graph-mode interaction.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphEvent {
    NodeSelected(NodeId),
    NodeHovered(NodeId),
    NodeDeselected,
    ModeChanged(GraphViewMode),
    LayoutRefreshed,
}

impl GraphModeState {
    /// Hit-test a click at `(x, y)` with `radius`.
    ///
    /// If a node is found, it becomes selected and `NodeSelected` is returned.
    /// Otherwise the selection is cleared and `None` is returned.
    pub fn process_click(&mut self, x: f32, y: f32, radius: f32) -> Option<GraphEvent> {
        match Self::node_at_point(&self.layout, x, y, radius) {
            Some(id) => {
                self.selected = Some(id.clone());
                Some(GraphEvent::NodeSelected(id))
            }
            None => {
                self.selected = None;
                None
            }
        }
    }

    /// Hit-test a pointer move at `(x, y)` with `radius`.
    ///
    /// Returns `NodeHovered` if a node is within radius, otherwise `None`.
    /// Also updates `self.hovered`.
    pub fn process_hover(&mut self, x: f32, y: f32, radius: f32) -> Option<GraphEvent> {
        let hit = Self::node_at_point(&self.layout, x, y, radius);
        self.hovered = hit.clone();
        hit.map(GraphEvent::NodeHovered)
    }

    /// Change the current view mode and return a `ModeChanged` event.
    pub fn change_mode(&mut self, mode: GraphViewMode) -> GraphEvent {
        self.mode = mode.clone();
        GraphEvent::ModeChanged(mode)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::ExecNode;

    fn three_node_dag() -> Dag {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("b", "out", "c", "in");
        dag
    }

    // ------------------------------------------------------------------
    // layout_dag: all nodes must appear in the returned map
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_layout_has_all_nodes() {
        let dag = three_node_dag();
        let layout = layout_dag(&dag);
        assert_eq!(layout.len(), 3, "layout must contain every node");
        assert!(layout.contains_key("a"), "missing node a");
        assert!(layout.contains_key("b"), "missing node b");
        assert!(layout.contains_key("c"), "missing node c");
    }

    // ------------------------------------------------------------------
    // select: sets the `selected` field
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_select_sets_selected() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        assert!(state.selected.is_none(), "selected should start as None");
        state.select("b".to_string());
        assert_eq!(state.selected.as_deref(), Some("b"));
        state.select("c".to_string());
        assert_eq!(state.selected.as_deref(), Some("c"));
    }

    // ------------------------------------------------------------------
    // node_at_point: finds nearby node, rejects too-far nodes
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_node_at_point_finds_nearby() {
        let mut layout: GraphLayout = HashMap::new();
        layout.insert("n1".to_string(), (0.0, 0.0));
        layout.insert("n2".to_string(), (200.0, 0.0));

        // Exactly at n1 — should find n1
        let hit = GraphModeState::node_at_point(&layout, 0.0, 0.0, 50.0);
        assert_eq!(hit.as_deref(), Some("n1"));

        // Close to n2 — should find n2
        let hit2 = GraphModeState::node_at_point(&layout, 195.0, 5.0, 50.0);
        assert_eq!(hit2.as_deref(), Some("n2"));

        // Between both but beyond radius of either — should be None
        let miss = GraphModeState::node_at_point(&layout, 100.0, 0.0, 40.0);
        assert!(miss.is_none(), "point halfway between nodes should miss");
    }

    // ------------------------------------------------------------------
    // Extra: mode starts as Graph
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_state_starts_in_graph_mode() {
        let dag = three_node_dag();
        let state = GraphModeState::new(&dag);
        assert_eq!(state.mode, GraphViewMode::Graph);
        assert!(state.selected.is_none());
        assert!(state.hovered.is_none());
    }

    // ------------------------------------------------------------------
    // Extra: hover sets and clears
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_hover_sets_and_clears() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.hover(Some("a".to_string()));
        assert_eq!(state.hovered.as_deref(), Some("a"));
        state.hover(None);
        assert!(state.hovered.is_none());
    }

    // ------------------------------------------------------------------
    // Extra: layout positions differ for nodes at different depths
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_layout_depths_differ() {
        let dag = three_node_dag();
        let layout = layout_dag(&dag);
        // a is depth 0, b is depth 1, c is depth 2
        let (_, ya) = layout["a"];
        let (_, yb) = layout["b"];
        let (_, yc) = layout["c"];
        assert!(ya < yb, "b should be below a");
        assert!(yb < yc, "c should be below b");
    }

    // ------------------------------------------------------------------
    // graph_event_click_selects_node
    // ------------------------------------------------------------------
    #[test]
    fn graph_event_click_selects_node() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);

        // Inject a known layout position for "a".
        state.layout.insert("a".to_string(), (0.0, 0.0));

        let event = state.process_click(0.0, 0.0, 50.0);
        assert_eq!(event, Some(GraphEvent::NodeSelected("a".to_string())));
        assert_eq!(state.selected.as_deref(), Some("a"));

        // Click far from every node — deselects.
        let miss = state.process_click(9999.0, 9999.0, 10.0);
        assert!(miss.is_none());
        assert!(state.selected.is_none());
    }

    // ------------------------------------------------------------------
    // graph_event_hover_returns_event
    // ------------------------------------------------------------------
    #[test]
    fn graph_event_hover_returns_event() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.layout.insert("b".to_string(), (100.0, 100.0));

        let event = state.process_hover(100.0, 100.0, 30.0);
        assert_eq!(event, Some(GraphEvent::NodeHovered("b".to_string())));
        assert_eq!(state.hovered.as_deref(), Some("b"));

        // Move away — no event, hovered cleared.
        let miss = state.process_hover(500.0, 500.0, 10.0);
        assert!(miss.is_none());
        assert!(state.hovered.is_none());
    }

    // ------------------------------------------------------------------
    // graph_event_mode_change
    // ------------------------------------------------------------------
    #[test]
    fn graph_event_mode_change() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        assert_eq!(state.mode, GraphViewMode::Graph);

        let event = state.change_mode(GraphViewMode::Canvas);
        assert_eq!(event, GraphEvent::ModeChanged(GraphViewMode::Canvas));
        assert_eq!(state.mode, GraphViewMode::Canvas);

        let event2 = state.change_mode(GraphViewMode::Split);
        assert_eq!(event2, GraphEvent::ModeChanged(GraphViewMode::Split));
        assert_eq!(state.mode, GraphViewMode::Split);
    }
}
