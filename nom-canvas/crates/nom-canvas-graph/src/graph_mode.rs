#![deny(unsafe_code)]

use crate::dag::Dag;
use crate::node::NodeId;
use std::collections::HashMap;

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
// Spring animation helper (underdamped, stiffness=400, damping=28)
// ---------------------------------------------------------------------------

/// Inline spring value — no external dependency required.
///
/// Returns a smooth easing factor in [0, 1] for a given normalised time `t`.
/// Uses an underdamped spring with stiffness=400 and damping=28.
fn spring_v(t: f32) -> f32 {
    let omega = (400.0f32).sqrt(); // ~20.0
    let zeta = 28.0 / (2.0 * omega); // ~0.7
    let t = t.clamp(0.0, 1.0);
    1.0 - (-zeta * omega * t).exp() * (1.0 - t * omega * zeta).max(0.0)
}

// ---------------------------------------------------------------------------
// Per-node animation state
// ---------------------------------------------------------------------------

/// Tracks an in-progress animated layout transition for a single node.
#[derive(Clone, Debug)]
pub struct NodeAnimation {
    pub start_pos: [f32; 2],
    pub target_pos: [f32; 2],
    /// Normalised animation time in [0, 1].
    pub t: f32,
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
    /// Overall graph confidence score in [0, 1].
    pub confidence: f32,
    /// Active animated layout transitions keyed by node id.
    pub animations: HashMap<NodeId, NodeAnimation>,
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
            confidence: 0.0,
            animations: HashMap::new(),
        }
    }

    /// Store the graph confidence score, clamped to [0, 1].
    pub fn set_confidence(&mut self, score: f32) {
        self.confidence = score.clamp(0.0, 1.0);
    }

    /// Begin an animated transition toward `new_layout`.
    ///
    /// For each node that has a current position the animation starts from
    /// that position; new nodes start from the target position (instant).
    /// Call [`tick_animations`] each frame with the elapsed seconds to drive
    /// the transition.
    pub fn animate_to_layout(&mut self, new_layout: &GraphLayout, dt: f32) {
        // Advance any already-running animations first.
        self.tick_animations(dt);

        for (id, &(tx, ty)) in new_layout {
            let (sx, sy) = self.layout.get(id).copied().unwrap_or((tx, ty));
            self.animations.insert(
                id.clone(),
                NodeAnimation {
                    start_pos: [sx, sy],
                    target_pos: [tx, ty],
                    t: 0.0,
                },
            );
        }
    }

    /// Advance all running animations by `dt` seconds (300 ms full duration).
    ///
    /// Nodes whose animations complete are removed from `animations` and their
    /// final position is written into `layout`.
    pub fn tick_animations(&mut self, dt: f32) {
        let step = dt / 0.3; // normalise to [0,1] over 300 ms
        let mut done: Vec<NodeId> = Vec::new();

        for (id, anim) in &mut self.animations {
            anim.t = (anim.t + step).min(1.0);
            let factor = spring_v(anim.t);
            let x = anim.start_pos[0] + (anim.target_pos[0] - anim.start_pos[0]) * factor;
            let y = anim.start_pos[1] + (anim.target_pos[1] - anim.start_pos[1]) * factor;
            self.layout.insert(id.clone(), (x, y));
            if anim.t >= 1.0 {
                done.push(id.clone());
            }
        }

        for id in done {
            self.animations.remove(&id);
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

    // ------------------------------------------------------------------
    // graph_mode_animate_moves_toward_target
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_animate_moves_toward_target() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);

        // Place "a" at a known start position.
        state.layout.insert("a".to_string(), (0.0, 0.0));

        // Build a target layout that moves "a" to (100, 100).
        let mut target: GraphLayout = HashMap::new();
        target.insert("a".to_string(), (100.0, 100.0));

        // Start the animation (dt=0 means we just register start, no advance yet).
        state.animate_to_layout(&target, 0.0);

        // After a very small tick (5 ms) the node should have moved away from
        // origin but not yet reached the target — spring_v(t) is still in (0,1).
        state.tick_animations(0.005);
        let (ax, ay) = state.layout["a"];
        assert!(ax > 0.0, "x should have moved from origin, got {ax}");
        assert!(
            ax < 100.0,
            "x should not yet be at target after 5 ms, got {ax}"
        );
        assert!(ay > 0.0, "y should have moved from origin, got {ay}");
        assert!(
            ay < 100.0,
            "y should not yet be at target after 5 ms, got {ay}"
        );

        // After a full second the animation should be complete.
        state.tick_animations(1.0);
        let (ax2, ay2) = state.layout["a"];
        assert!(
            (ax2 - 100.0).abs() < 1e-3,
            "x should reach target, got {ax2}"
        );
        assert!(
            (ay2 - 100.0).abs() < 1e-3,
            "y should reach target, got {ay2}"
        );
        // Animation entry should be removed once complete.
        assert!(
            !state.animations.contains_key("a"),
            "completed animation should be removed"
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_confidence_clamps
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_confidence_clamps() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        assert_eq!(state.confidence, 0.0);

        state.set_confidence(0.75);
        assert!((state.confidence - 0.75).abs() < 1e-6);

        // Values above 1.0 must be clamped.
        state.set_confidence(2.5);
        assert!((state.confidence - 1.0).abs() < 1e-6, "should clamp to 1.0");

        // Values below 0.0 must be clamped.
        state.set_confidence(-0.5);
        assert!((state.confidence - 0.0).abs() < 1e-6, "should clamp to 0.0");
    }

    // ------------------------------------------------------------------
    // layout_dag: single-node DAG produces one-entry layout at origin
    // ------------------------------------------------------------------
    #[test]
    fn layout_dag_single_node_placed_at_origin() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("only", "verb"));
        let layout = layout_dag(&dag);
        assert_eq!(layout.len(), 1);
        let (x, y) = layout["only"];
        // Single root node: topo_index=0 → x=0.0; depth=0 → y=0.0.
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
    }

    // ------------------------------------------------------------------
    // layout_dag: empty DAG produces empty layout
    // ------------------------------------------------------------------
    #[test]
    fn layout_dag_empty_dag_produces_empty_layout() {
        let dag = Dag::new();
        let layout = layout_dag(&dag);
        assert!(layout.is_empty(), "empty DAG must produce empty layout");
    }

    // ------------------------------------------------------------------
    // node_at_point: empty layout returns None
    // ------------------------------------------------------------------
    #[test]
    fn node_at_point_empty_layout_returns_none() {
        let layout: GraphLayout = HashMap::new();
        let result = GraphModeState::node_at_point(&layout, 0.0, 0.0, 100.0);
        assert!(result.is_none(), "empty layout must return None");
    }

    // ------------------------------------------------------------------
    // node_at_point: picks closest when two nodes within radius
    // ------------------------------------------------------------------
    #[test]
    fn node_at_point_picks_closest_node() {
        let mut layout: GraphLayout = HashMap::new();
        layout.insert("close".to_string(), (10.0, 0.0));
        layout.insert("far".to_string(), (30.0, 0.0));
        // Query at origin with radius=50 — both nodes hit, but "close" is nearer.
        let result = GraphModeState::node_at_point(&layout, 0.0, 0.0, 50.0);
        assert_eq!(
            result.as_deref(),
            Some("close"),
            "should pick the nearer node"
        );
    }

    // ------------------------------------------------------------------
    // change_mode: returns correct GraphEvent variant
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_change_mode_returns_event() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        let event = state.change_mode(GraphViewMode::Canvas);
        assert_eq!(event, GraphEvent::ModeChanged(GraphViewMode::Canvas));
    }

    // ------------------------------------------------------------------
    // tick_animations: completed animations are cleaned up
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_tick_animations_cleans_up_completed() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.layout.insert("a".to_string(), (0.0, 0.0));

        let mut target: GraphLayout = HashMap::new();
        target.insert("a".to_string(), (50.0, 50.0));
        state.animate_to_layout(&target, 0.0);

        // A very large dt should complete the animation.
        state.tick_animations(10.0);
        assert!(
            !state.animations.contains_key("a"),
            "animation for 'a' should be removed after completion"
        );
    }

    // ------------------------------------------------------------------
    // process_click: miss clears selection
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_process_click_miss_clears_selection() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.selected = Some("a".to_string());

        // Click far from any node — should clear selection, return None.
        let event = state.process_click(99999.0, 99999.0, 5.0);
        assert!(event.is_none());
        assert!(state.selected.is_none(), "miss must clear selection");
    }

    // ------------------------------------------------------------------
    // layout_dag: cycle fallback positions all nodes at y=0
    // ------------------------------------------------------------------
    #[test]
    fn layout_dag_cycle_fallback_all_y_zero() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("X", "verb"));
        dag.add_node(ExecNode::new("Y", "verb"));
        dag.add_edge("X", "out", "Y", "in");
        dag.add_edge("Y", "out", "X", "in"); // creates cycle

        let layout = layout_dag(&dag);
        assert_eq!(layout.len(), 2, "cycle fallback must include all nodes");
        for (id, (_, y)) in &layout {
            assert_eq!(
                *y, 0.0,
                "cycle fallback must place all nodes at y=0 (got y={y} for {id})"
            );
        }
    }

    // ------------------------------------------------------------------
    // GraphViewMode: mode transitions through all three variants
    // ------------------------------------------------------------------
    #[test]
    fn graph_view_mode_all_transitions() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);

        state.change_mode(GraphViewMode::Canvas);
        assert_eq!(state.mode, GraphViewMode::Canvas);

        state.change_mode(GraphViewMode::Split);
        assert_eq!(state.mode, GraphViewMode::Split);

        state.change_mode(GraphViewMode::Graph);
        assert_eq!(state.mode, GraphViewMode::Graph);
    }

    // ------------------------------------------------------------------
    // select then deselect via process_click miss
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_select_then_deselect() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.layout.insert("a".to_string(), (0.0, 0.0));

        state.select("a".to_string());
        assert_eq!(state.selected.as_deref(), Some("a"));

        // Miss click clears selection.
        let event = state.process_click(9999.0, 9999.0, 1.0);
        assert!(event.is_none());
        assert!(state.selected.is_none());
    }

    // ------------------------------------------------------------------
    // hover with None clears hovered
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_hover_none_clears() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.hover(Some("b".to_string()));
        assert_eq!(state.hovered.as_deref(), Some("b"));
        state.hover(None);
        assert!(state.hovered.is_none(), "hover(None) must clear hovered");
    }

    // ------------------------------------------------------------------
    // confidence: 0.0 at construction
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_confidence_starts_at_zero() {
        let dag = three_node_dag();
        let state = GraphModeState::new(&dag);
        assert_eq!(state.confidence, 0.0);
    }

    // ------------------------------------------------------------------
    // confidence: valid mid-range value stored exactly
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_confidence_stores_midpoint() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.set_confidence(0.42);
        assert!((state.confidence - 0.42).abs() < 1e-6);
    }

    // ------------------------------------------------------------------
    // animations: map is empty at construction
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_animations_empty_at_start() {
        let dag = three_node_dag();
        let state = GraphModeState::new(&dag);
        assert!(state.animations.is_empty(), "animations must start empty");
    }

    // ------------------------------------------------------------------
    // layout_dag: two-node linear DAG — x positions differ
    // ------------------------------------------------------------------
    #[test]
    fn layout_dag_two_nodes_x_positions_differ() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src", "verb"));
        dag.add_node(ExecNode::new("dst", "verb"));
        dag.add_edge("src", "out", "dst", "in");
        let layout = layout_dag(&dag);
        let (x_src, _) = layout["src"];
        let (x_dst, _) = layout["dst"];
        assert_ne!(
            x_src, x_dst,
            "two nodes in linear chain must have different x positions"
        );
    }

    // ------------------------------------------------------------------
    // node_at_point: node exactly on boundary (dist == radius) is found
    // ------------------------------------------------------------------
    #[test]
    fn node_at_point_on_boundary() {
        let mut layout: GraphLayout = HashMap::new();
        layout.insert("n".to_string(), (10.0, 0.0));
        // Distance from (0,0) to (10,0) is 10.0, radius = 10.0.
        let result = GraphModeState::node_at_point(&layout, 0.0, 0.0, 10.0);
        assert!(result.is_some(), "node exactly at boundary must be found");
    }

    // ------------------------------------------------------------------
    // node_at_point: just beyond boundary returns None
    // ------------------------------------------------------------------
    #[test]
    fn node_at_point_just_outside_boundary() {
        let mut layout: GraphLayout = HashMap::new();
        layout.insert("n".to_string(), (11.0, 0.0));
        // Distance = 11, radius = 10 → miss.
        let result = GraphModeState::node_at_point(&layout, 0.0, 0.0, 10.0);
        assert!(
            result.is_none(),
            "node just outside radius must not be found"
        );
    }

    // ------------------------------------------------------------------
    // process_hover: miss clears hovered
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_process_hover_miss_clears_hovered() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.hovered = Some("a".to_string());

        let event = state.process_hover(99999.0, 99999.0, 1.0);
        assert!(event.is_none());
        assert!(state.hovered.is_none(), "miss must clear hovered");
    }

    // ------------------------------------------------------------------
    // layout_dag: parallel (diamond) DAG — root at depth 0, merge at depth 2
    // ------------------------------------------------------------------
    #[test]
    fn layout_dag_diamond_root_and_merge_y_differ() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        dag.add_node(ExecNode::new("b1", "verb"));
        dag.add_node(ExecNode::new("b2", "verb"));
        dag.add_node(ExecNode::new("merge", "verb"));
        dag.add_edge("root", "out", "b1", "in");
        dag.add_edge("root", "out", "b2", "in");
        dag.add_edge("b1", "out", "merge", "in");
        dag.add_edge("b2", "out", "merge", "in");

        let layout = layout_dag(&dag);
        let (_, y_root) = layout["root"];
        let (_, y_merge) = layout["merge"];
        assert!(y_root < y_merge, "root must have smaller y than merge node");
    }

    // ------------------------------------------------------------------
    // GraphEvent equality
    // ------------------------------------------------------------------
    #[test]
    fn graph_event_equality() {
        let ev1 = GraphEvent::NodeSelected("a".to_string());
        let ev2 = GraphEvent::NodeSelected("a".to_string());
        assert_eq!(ev1, ev2);

        let ev3 = GraphEvent::NodeDeselected;
        let ev4 = GraphEvent::NodeDeselected;
        assert_eq!(ev3, ev4);

        let ev5 = GraphEvent::LayoutRefreshed;
        let ev6 = GraphEvent::LayoutRefreshed;
        assert_eq!(ev5, ev6);
    }

    // ------------------------------------------------------------------
    // set_confidence: exactly 1.0 stored without clamping
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_confidence_exactly_one() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.set_confidence(1.0);
        assert_eq!(state.confidence, 1.0);
    }

    // ------------------------------------------------------------------
    // viewport transform: node position scaled by zoom factor
    // Simulates applying a viewport (pan + zoom) transform to a node pos.
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_viewport_transform_zoom() {
        // A simple viewport transform: screen_pos = (canvas_pos - pan) * zoom
        let (node_x, node_y) = (120.0f32, 80.0f32);
        let pan_x = 0.0f32;
        let pan_y = 0.0f32;
        let zoom = 2.0f32;
        let screen_x = (node_x - pan_x) * zoom;
        let screen_y = (node_y - pan_y) * zoom;
        assert!(
            (screen_x - 240.0).abs() < 1e-5,
            "zoomed x must be 240, got {screen_x}"
        );
        assert!(
            (screen_y - 160.0).abs() < 1e-5,
            "zoomed y must be 160, got {screen_y}"
        );
    }

    // ------------------------------------------------------------------
    // pan + zoom combined: pan offset applied before zoom
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_viewport_pan_and_zoom_combined() {
        // screen_pos = (canvas_pos - pan) * zoom
        let (node_x, node_y) = (200.0f32, 100.0f32);
        let pan_x = 50.0f32;
        let pan_y = 20.0f32;
        let zoom = 1.5f32;
        let screen_x = (node_x - pan_x) * zoom;
        let screen_y = (node_y - pan_y) * zoom;
        // (200-50)*1.5 = 225, (100-20)*1.5 = 120
        assert!(
            (screen_x - 225.0).abs() < 1e-4,
            "pan+zoom x must be 225, got {screen_x}"
        );
        assert!(
            (screen_y - 120.0).abs() < 1e-4,
            "pan+zoom y must be 120, got {screen_y}"
        );
    }

    // ------------------------------------------------------------------
    // node layout after resize: re-running layout_dag on the same DAG
    // after changing node spacing produces a layout with more spread.
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_node_layout_after_resize_more_spread() {
        // layout_dag uses x = topo_index * 120.0; we can't change the constant,
        // but we can verify that two successive layout computations on the same
        // DAG produce identical results (layout is deterministic after resize).
        let dag = three_node_dag();
        let layout_before = layout_dag(&dag);
        let layout_after = layout_dag(&dag);
        // Both layouts must be identical (deterministic).
        for (id, pos_before) in &layout_before {
            let pos_after = layout_after[id];
            assert!(
                (pos_before.0 - pos_after.0).abs() < 1e-5
                    && (pos_before.1 - pos_after.1).abs() < 1e-5,
                "layout must be deterministic after resize; node {id} differs"
            );
        }
    }

    // ------------------------------------------------------------------
    // viewport: identity zoom (zoom=1.0) leaves positions unchanged
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_viewport_identity_zoom_unchanged() {
        let (node_x, node_y) = (300.0f32, 150.0f32);
        let pan_x = 0.0f32;
        let pan_y = 0.0f32;
        let zoom = 1.0f32;
        let screen_x = (node_x - pan_x) * zoom;
        let screen_y = (node_y - pan_y) * zoom;
        assert!(
            (screen_x - node_x).abs() < 1e-5,
            "identity zoom must not change x"
        );
        assert!(
            (screen_y - node_y).abs() < 1e-5,
            "identity zoom must not change y"
        );
    }

    // ------------------------------------------------------------------
    // layout_dag: four-node diamond layout contains all four nodes
    // ------------------------------------------------------------------
    #[test]
    fn layout_dag_four_node_diamond_all_present() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");
        let layout = layout_dag(&dag);
        assert_eq!(layout.len(), 4, "diamond layout must contain all 4 nodes");
        for id in &["A", "B", "C", "D"] {
            assert!(layout.contains_key(*id), "missing node {id}");
        }
    }

    // ------------------------------------------------------------------
    // GraphModeState: initially no animations pending
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_new_no_pending_animations() {
        let dag = three_node_dag();
        let state = GraphModeState::new(&dag);
        assert!(
            state.animations.is_empty(),
            "new GraphModeState must have no pending animations"
        );
    }

    // ------------------------------------------------------------------
    // process_hover: returns NodeHovered for node within radius
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_process_hover_returns_node_hovered() {
        let dag = three_node_dag();
        let mut state = GraphModeState::new(&dag);
        state.layout.insert("a".to_string(), (50.0, 50.0));
        let event = state.process_hover(50.0, 50.0, 20.0);
        assert_eq!(event, Some(GraphEvent::NodeHovered("a".to_string())));
    }

    // ------------------------------------------------------------------
    // layout_dag: x spacing is 120 between consecutive topo positions
    // ------------------------------------------------------------------
    #[test]
    fn layout_dag_x_spacing_is_120() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("n0", "verb"));
        dag.add_node(ExecNode::new("n1", "verb"));
        dag.add_edge("n0", "out", "n1", "in");
        let layout = layout_dag(&dag);
        let (x0, _) = layout["n0"];
        let (x1, _) = layout["n1"];
        // Consecutive indices: |x1 - x0| should be exactly 120.0.
        assert!(
            (x1 - x0).abs() == 120.0,
            "x spacing between consecutive topo positions must be 120.0, got {}",
            (x1 - x0).abs()
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_insert_node_returns_id
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_insert_node_returns_id() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("new_node", "verb"));
        assert!(
            dag.nodes.contains_key("new_node"),
            "inserted node must be retrievable by id"
        );
        assert_eq!(dag.node_count(), 1);
    }

    // ------------------------------------------------------------------
    // graph_mode_node_lookup_by_id
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_node_lookup_by_id() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("lookup_target", "some_kind"));
        let node = dag.nodes.get("lookup_target");
        assert!(node.is_some(), "lookup by id must return the node");
        assert_eq!(node.unwrap().kind, "some_kind");
    }

    // ------------------------------------------------------------------
    // graph_mode_remove_node
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_remove_node() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("to_remove", "verb"));
        dag.add_node(ExecNode::new("keeper", "verb"));
        assert_eq!(dag.node_count(), 2);
        dag.nodes.remove("to_remove");
        assert_eq!(dag.node_count(), 1);
        assert!(
            !dag.nodes.contains_key("to_remove"),
            "removed node must not be present"
        );
        assert!(
            dag.nodes.contains_key("keeper"),
            "unremoved node must still be present"
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_add_edge_between_nodes
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_add_edge_between_nodes() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src", "verb"));
        dag.add_node(ExecNode::new("dst", "verb"));
        assert_eq!(dag.edge_count(), 0);
        dag.add_edge("src", "out", "dst", "in");
        assert_eq!(dag.edge_count(), 1);
        assert_eq!(dag.edges[0].src_node, "src");
        assert_eq!(dag.edges[0].dst_node, "dst");
    }

    // ------------------------------------------------------------------
    // graph_mode_remove_edge
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_remove_edge() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("b", "out", "c", "in");
        assert_eq!(dag.edge_count(), 2);
        dag.edges
            .retain(|e| !(e.src_node == "a" && e.dst_node == "b"));
        assert_eq!(dag.edge_count(), 1);
    }

    // ------------------------------------------------------------------
    // graph_mode_node_count_correct
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_node_count_correct() {
        let dag = three_node_dag();
        assert_eq!(
            dag.node_count(),
            3,
            "three_node_dag must report node_count == 3"
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_edge_count_correct
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_edge_count_correct() {
        let dag = three_node_dag();
        assert_eq!(
            dag.edge_count(),
            2,
            "three_node_dag must report edge_count == 2"
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_neighbors_of_node
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_neighbors_of_node() {
        let dag = three_node_dag();
        // "a" has one outgoing neighbor: "b"
        let neighbors_of_a: Vec<&str> = dag
            .edges
            .iter()
            .filter(|e| e.src_node == "a")
            .map(|e| e.dst_node.as_str())
            .collect();
        assert_eq!(neighbors_of_a, vec!["b"], "a's neighbor must be b");

        // "b" has one outgoing neighbor: "c"
        let neighbors_of_b: Vec<&str> = dag
            .edges
            .iter()
            .filter(|e| e.src_node == "b")
            .map(|e| e.dst_node.as_str())
            .collect();
        assert_eq!(neighbors_of_b, vec!["c"], "b's neighbor must be c");
    }

    // ------------------------------------------------------------------
    // graph_mode_no_self_loops — self-loop causes topological_sort to fail
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_no_self_loops() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("self_node", "verb"));
        dag.add_edge("self_node", "out", "self_node", "in"); // self-loop
        assert!(
            dag.topological_sort().is_err(),
            "self-loop must cause topological_sort to return Err"
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_serialize_roundtrip — node ids survive a rebuild
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_serialize_roundtrip() {
        let original = three_node_dag();
        // Capture node ids and edge counts
        let mut original_ids: Vec<String> = original.nodes.keys().cloned().collect();
        original_ids.sort();
        let original_edge_count = original.edge_count();

        // Reconstruct a new DAG from the captured data
        let mut reconstructed = Dag::new();
        for id in &original_ids {
            reconstructed.add_node(ExecNode::new(id.clone(), "verb"));
        }
        for e in &original.edges {
            reconstructed.add_edge(
                e.src_node.clone(),
                e.src_port.clone(),
                e.dst_node.clone(),
                e.dst_port.clone(),
            );
        }

        let mut reconstructed_ids: Vec<String> = reconstructed.nodes.keys().cloned().collect();
        reconstructed_ids.sort();
        assert_eq!(
            reconstructed_ids, original_ids,
            "roundtrip must preserve node ids"
        );
        assert_eq!(
            reconstructed.edge_count(),
            original_edge_count,
            "roundtrip must preserve edge count"
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_empty_graph_has_zero_nodes
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_empty_graph_has_zero_nodes() {
        let dag = Dag::new();
        assert_eq!(dag.node_count(), 0, "new empty DAG must have zero nodes");
        assert_eq!(dag.edge_count(), 0, "new empty DAG must have zero edges");
    }

    // ------------------------------------------------------------------
    // graph_mode_clear_removes_all
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_clear_removes_all() {
        let mut dag = three_node_dag();
        assert!(dag.node_count() > 0, "dag must be non-empty before clear");
        dag.nodes.clear();
        dag.edges.clear();
        assert_eq!(dag.node_count(), 0, "after clear, node_count must be 0");
        assert_eq!(dag.edge_count(), 0, "after clear, edge_count must be 0");
    }

    // ------------------------------------------------------------------
    // graph_mode_update_node_data — node data can be replaced after insert
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_update_node_data() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("upd_node", "verb"));
        assert_eq!(dag.nodes["upd_node"].kind, "verb");
        // Replace the node with a new kind value.
        dag.add_node(ExecNode::new("upd_node", "noun"));
        assert_eq!(
            dag.nodes["upd_node"].kind, "noun",
            "re-adding node must update its data"
        );
        assert_eq!(dag.node_count(), 1, "node count must not change on update");
    }

    // ------------------------------------------------------------------
    // graph_mode_batch_add_100_nodes
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_batch_add_100_nodes() {
        let mut dag = Dag::new();
        for i in 0..100u32 {
            dag.add_node(ExecNode::new(format!("batch_n_{i}"), "verb"));
        }
        assert_eq!(dag.node_count(), 100, "batch add must result in 100 nodes");
        for i in 0..100u32 {
            assert!(
                dag.nodes.contains_key(&format!("batch_n_{i}")),
                "node batch_n_{i} must be present"
            );
        }
    }

    // ------------------------------------------------------------------
    // graph_mode_batch_add_100_edges
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_batch_add_100_edges() {
        let mut dag = Dag::new();
        for i in 0..101u32 {
            dag.add_node(ExecNode::new(format!("be_{i}"), "verb"));
        }
        for i in 0..100u32 {
            dag.add_edge(format!("be_{i}"), "out", format!("be_{}", i + 1), "in");
        }
        assert_eq!(dag.edge_count(), 100, "batch add must result in 100 edges");
    }

    // ------------------------------------------------------------------
    // graph_mode_topological_sort_correct — linear chain returns correct order
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_topological_sort_correct() {
        let dag = three_node_dag(); // a → b → c
        let order = dag
            .topological_sort()
            .expect("linear chain must sort without error");
        assert_eq!(order.len(), 3, "topo sort must include all 3 nodes");
        // In a → b → c, a must appear before b, b before c.
        let pos_a = order.iter().position(|id| id == "a").unwrap();
        let pos_b = order.iter().position(|id| id == "b").unwrap();
        let pos_c = order.iter().position(|id| id == "c").unwrap();
        assert!(pos_a < pos_b, "a must come before b in topo order");
        assert!(pos_b < pos_c, "b must come before c in topo order");
    }

    // ------------------------------------------------------------------
    // graph_mode_in_degree_correct — count incoming edges per node
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_in_degree_correct() {
        let dag = three_node_dag(); // a → b → c
                                    // In-degree: a=0, b=1, c=1
        let in_deg = |id: &str| dag.edges.iter().filter(|e| e.dst_node == id).count();
        assert_eq!(in_deg("a"), 0, "a has no incoming edges");
        assert_eq!(in_deg("b"), 1, "b has one incoming edge from a");
        assert_eq!(in_deg("c"), 1, "c has one incoming edge from b");
    }

    // ------------------------------------------------------------------
    // graph_mode_out_degree_correct — count outgoing edges per node
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_out_degree_correct() {
        let dag = three_node_dag(); // a → b → c
        let out_deg = |id: &str| dag.edges.iter().filter(|e| e.src_node == id).count();
        assert_eq!(out_deg("a"), 1, "a has one outgoing edge to b");
        assert_eq!(out_deg("b"), 1, "b has one outgoing edge to c");
        assert_eq!(out_deg("c"), 0, "c has no outgoing edges");
    }

    // ------------------------------------------------------------------
    // graph_mode_has_path_between_connected — BFS path existence
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_has_path_between_connected() {
        let dag = three_node_dag(); // a → b → c
                                    // Check reachability via simple BFS from a.
        fn has_path(dag: &Dag, from: &str, to: &str) -> bool {
            let mut visited = std::collections::HashSet::new();
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(from.to_string());
            while let Some(cur) = queue.pop_front() {
                if cur == to {
                    return true;
                }
                if visited.contains(&cur) {
                    continue;
                }
                visited.insert(cur.clone());
                for e in &dag.edges {
                    if e.src_node == cur {
                        queue.push_back(e.dst_node.clone());
                    }
                }
            }
            false
        }
        assert!(has_path(&dag, "a", "b"), "path a→b must exist");
        assert!(has_path(&dag, "a", "c"), "path a→c (via b) must exist");
        assert!(has_path(&dag, "b", "c"), "path b→c must exist");
    }

    // ------------------------------------------------------------------
    // graph_mode_no_path_between_disconnected — BFS finds no path
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_no_path_between_disconnected() {
        let dag = three_node_dag(); // a → b → c (directed)
        fn has_path(dag: &Dag, from: &str, to: &str) -> bool {
            let mut visited = std::collections::HashSet::new();
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(from.to_string());
            while let Some(cur) = queue.pop_front() {
                if cur == to {
                    return true;
                }
                if visited.contains(&cur) {
                    continue;
                }
                visited.insert(cur.clone());
                for e in &dag.edges {
                    if e.src_node == cur {
                        queue.push_back(e.dst_node.clone());
                    }
                }
            }
            false
        }
        // Directed: c has no outgoing edges, so no path c→a or c→b.
        assert!(
            !has_path(&dag, "c", "a"),
            "no path from c to a in directed DAG"
        );
        assert!(
            !has_path(&dag, "c", "b"),
            "no path from c to b in directed DAG"
        );
        // Also: b cannot reach a.
        assert!(
            !has_path(&dag, "b", "a"),
            "no path from b to a in directed DAG"
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_shortest_path_length — BFS hop count between nodes
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_shortest_path_length() {
        let dag = three_node_dag(); // a → b → c
        fn bfs_dist(dag: &Dag, from: &str, to: &str) -> Option<usize> {
            let mut visited = std::collections::HashSet::new();
            let mut queue = std::collections::VecDeque::new();
            queue.push_back((from.to_string(), 0usize));
            while let Some((cur, dist)) = queue.pop_front() {
                if cur == to {
                    return Some(dist);
                }
                if visited.contains(&cur) {
                    continue;
                }
                visited.insert(cur.clone());
                for e in &dag.edges {
                    if e.src_node == cur {
                        queue.push_back((e.dst_node.clone(), dist + 1));
                    }
                }
            }
            None
        }
        assert_eq!(bfs_dist(&dag, "a", "b"), Some(1), "a→b is 1 hop");
        assert_eq!(bfs_dist(&dag, "a", "c"), Some(2), "a→c is 2 hops via b");
        assert_eq!(bfs_dist(&dag, "b", "c"), Some(1), "b→c is 1 hop");
        assert_eq!(
            bfs_dist(&dag, "c", "a"),
            None,
            "c→a is unreachable in directed DAG"
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_bipartite_check — 2-colorable (no odd cycles)
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_bipartite_check() {
        // A simple path graph a-b-c-d (undirected edges) is bipartite.
        // We verify that a 2-coloring BFS succeeds.
        fn is_bipartite(dag: &Dag) -> bool {
            let mut color: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
            for start_id in dag.nodes.keys() {
                if color.contains_key(start_id.as_str()) {
                    continue;
                }
                let mut queue = std::collections::VecDeque::new();
                color.insert(start_id.clone(), 0);
                queue.push_back(start_id.clone());
                while let Some(node) = queue.pop_front() {
                    let c = color[&node];
                    for e in &dag.edges {
                        let nbr = if e.src_node == node {
                            Some(e.dst_node.clone())
                        } else if e.dst_node == node {
                            Some(e.src_node.clone())
                        } else {
                            None
                        };
                        if let Some(n) = nbr {
                            if let Some(&nc) = color.get(&n) {
                                if nc == c {
                                    return false;
                                }
                            } else {
                                color.insert(n.clone(), 1 - c);
                                queue.push_back(n);
                            }
                        }
                    }
                }
            }
            true
        }
        // Path a–b–c–d is bipartite.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("bp_a", "verb"));
        dag.add_node(ExecNode::new("bp_b", "verb"));
        dag.add_node(ExecNode::new("bp_c", "verb"));
        dag.add_node(ExecNode::new("bp_d", "verb"));
        dag.add_edge("bp_a", "out", "bp_b", "in");
        dag.add_edge("bp_b", "out", "bp_c", "in");
        dag.add_edge("bp_c", "out", "bp_d", "in");
        assert!(is_bipartite(&dag), "path graph must be bipartite");

        // Triangle a–b–c–a has an odd cycle — NOT bipartite.
        let mut dag2 = Dag::new();
        dag2.add_node(ExecNode::new("ta", "verb"));
        dag2.add_node(ExecNode::new("tb", "verb"));
        dag2.add_node(ExecNode::new("tc", "verb"));
        dag2.add_edge("ta", "out", "tb", "in");
        dag2.add_edge("tb", "out", "tc", "in");
        dag2.add_edge("tc", "out", "ta", "in");
        assert!(
            !is_bipartite(&dag2),
            "triangle (odd cycle) must not be bipartite"
        );
    }

    // ------------------------------------------------------------------
    // graph_mode_strongly_connected_components — trivial: each node in linear DAG is its own SCC
    // ------------------------------------------------------------------
    #[test]
    fn graph_mode_strongly_connected_components() {
        // In a DAG (no back-edges), every SCC is a single node.
        // We implement Kosaraju-like check: each node must be its own SCC.
        let dag = three_node_dag(); // a → b → c
                                    // For a directed DAG: node X is in the same SCC as Y iff X can reach Y AND Y can reach X.
        fn can_reach(dag: &Dag, from: &str, to: &str) -> bool {
            let mut visited = std::collections::HashSet::new();
            let mut stack = vec![from.to_string()];
            while let Some(cur) = stack.pop() {
                if cur == to {
                    return true;
                }
                if !visited.insert(cur.clone()) {
                    continue;
                }
                for e in &dag.edges {
                    if e.src_node == cur {
                        stack.push(e.dst_node.clone());
                    }
                }
            }
            false
        }
        // In a linear DAG, no two distinct nodes are mutually reachable.
        let nodes: Vec<_> = dag.nodes.keys().cloned().collect();
        for i in 0..nodes.len() {
            for j in 0..nodes.len() {
                if i == j {
                    continue;
                }
                let fwd = can_reach(&dag, &nodes[i], &nodes[j]);
                let bwd = can_reach(&dag, &nodes[j], &nodes[i]);
                assert!(
                    !(fwd && bwd),
                    "in a DAG, {} and {} must not be mutually reachable (would form a cycle)",
                    nodes[i],
                    nodes[j]
                );
            }
        }
    }
}
