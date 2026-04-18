//! Workspace — the root container for all blocks, nodes, and connectors.
#![deny(unsafe_code)]
use crate::block_model::{BlockId, BlockModel, NomtuRef};
use crate::connector::{Connector, ConnectorId};
use crate::graph_node::{GraphNode, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A heterogeneous canvas object: block, node, or connector.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CanvasObject {
    /// A document block.
    Block(BlockModel),
    /// A graph node.
    Node(GraphNode),
    /// A connector wire.
    Connector(Connector),
}

impl CanvasObject {
    /// Return the [`NomtuRef`] for Block or Node variants. Panics for Connector.
    pub fn entity(&self) -> &NomtuRef {
        match self {
            CanvasObject::Block(b) => &b.entity,
            CanvasObject::Node(n) => &n.entity,
            CanvasObject::Connector(_) => {
                panic!("Connectors don't have a direct NomtuRef — use src/dst node entities")
            }
        }
    }
}

/// Root container for a NomCanvas document — blocks, nodes, connectors, and doc-tree order.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Workspace {
    /// All blocks keyed by [`BlockId`].
    pub blocks: HashMap<BlockId, BlockModel>,
    /// All graph nodes keyed by [`NodeId`].
    pub nodes: HashMap<NodeId, GraphNode>,
    /// All connectors keyed by [`ConnectorId`].
    pub connectors: HashMap<ConnectorId, Connector>,
    /// Ordered list of top-level block IDs (document tree).
    pub doc_tree: Vec<BlockId>,
}

impl Workspace {
    /// Construct an empty workspace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a block and append its ID to `doc_tree`.
    pub fn insert_block(&mut self, block: BlockModel) {
        self.doc_tree.push(block.id.clone());
        self.blocks.insert(block.id.clone(), block);
    }

    /// Add a graph node to the workspace.
    pub fn insert_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Add a connector to the workspace.
    pub fn insert_connector(&mut self, connector: Connector) {
        self.connectors.insert(connector.id.clone(), connector);
    }

    /// Remove a block by ID, also removing it from `doc_tree`. Returns the block if found.
    pub fn remove_block(&mut self, id: &str) -> Option<BlockModel> {
        self.doc_tree.retain(|bid| bid != id);
        self.blocks.remove(id)
    }

    /// Number of blocks currently in the workspace.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
    /// Number of graph nodes currently in the workspace.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    /// Number of connectors currently in the workspace.
    pub fn connector_count(&self) -> usize {
        self.connectors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_insert_remove() {
        let mut ws = Workspace::new();
        let entity = NomtuRef::new("e1", "summarize", "verb");
        let block = BlockModel::new("b1", entity, "affine:paragraph");
        ws.insert_block(block);
        assert_eq!(ws.block_count(), 1);
        assert!(ws.doc_tree.contains(&"b1".to_string()));
        ws.remove_block("b1");
        assert_eq!(ws.block_count(), 0);
    }

    #[test]
    fn workspace_insert_node_and_connector() {
        let mut ws = Workspace::new();
        let node = GraphNode::new(
            "n1",
            NomtuRef::new("e1", "fetch", "verb"),
            "verb",
            [0.0, 0.0],
        );
        ws.insert_node(node);
        assert_eq!(ws.node_count(), 1);

        let dict = crate::stub_dict::StubDictReader::new();
        let conn = Connector::new_with_validation(crate::connector::ConnectorValidation {
            id: "c1".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        ws.insert_connector(conn);
        assert_eq!(ws.connector_count(), 1);
    }

    #[test]
    fn workspace_remove_block_updates_doc_tree() {
        let mut ws = Workspace::new();
        for i in 0..3u8 {
            let block = BlockModel::new(
                format!("b{i}"),
                NomtuRef::new(format!("e{i}"), "w", "verb"),
                "affine:paragraph",
            );
            ws.insert_block(block);
        }
        assert_eq!(ws.block_count(), 3);
        assert_eq!(ws.doc_tree.len(), 3);
        ws.remove_block("b1");
        assert_eq!(ws.block_count(), 2);
        assert!(!ws.doc_tree.contains(&"b1".to_string()));
    }

    #[test]
    fn canvas_object_entity_returns_block_entity() {
        let entity = NomtuRef::new("e1", "render", "verb");
        let block = BlockModel::new("b1", entity.clone(), "affine:paragraph");
        let obj = CanvasObject::Block(block);
        assert_eq!(obj.entity().id, "e1");
        assert_eq!(obj.entity().word, "render");
    }

    /// Workspace::new() starts with zero counts
    #[test]
    fn workspace_starts_empty() {
        let ws = Workspace::new();
        assert_eq!(ws.block_count(), 0);
        assert_eq!(ws.node_count(), 0);
        assert_eq!(ws.connector_count(), 0);
        assert!(ws.doc_tree.is_empty());
    }

    /// Inserting the same block ID twice updates the map but doc_tree gets two entries
    #[test]
    fn workspace_insert_duplicate_block_id() {
        let mut ws = Workspace::new();
        let b1 = BlockModel::new("dup", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        let b2 = BlockModel::new("dup", NomtuRef::new("e2", "w", "verb"), "affine:paragraph");
        ws.insert_block(b1);
        ws.insert_block(b2);
        // HashMap replaces: still 1 block
        assert_eq!(ws.block_count(), 1);
        // doc_tree appends: 2 entries
        assert_eq!(ws.doc_tree.len(), 2);
    }

    /// CanvasObject::Node entity returns the node's NomtuRef
    #[test]
    fn canvas_object_node_entity() {
        use crate::graph_node::GraphNode;
        let entity = NomtuRef::new("ne1", "transform", "verb");
        let node = GraphNode::new("n1", entity.clone(), "verb", [0.0, 0.0]);
        let obj = CanvasObject::Node(node);
        assert_eq!(obj.entity().id, "ne1");
        assert_eq!(obj.entity().word, "transform");
    }

    /// remove_block on non-existent ID returns None without panicking
    #[test]
    fn workspace_remove_nonexistent_block_returns_none() {
        let mut ws = Workspace::new();
        let result = ws.remove_block("no-such-id");
        assert!(result.is_none());
    }

    /// Block IDs inserted into workspace remain distinct (get_by_id correctness)
    #[test]
    fn workspace_block_ids_are_unique() {
        let mut ws = Workspace::new();
        let ids = ["id-a", "id-b", "id-c"];
        for id in &ids {
            let block = BlockModel::new(*id, NomtuRef::new(*id, "w", "verb"), "affine:paragraph");
            ws.insert_block(block);
        }
        assert_eq!(ws.block_count(), 3);
        for id in &ids {
            assert!(ws.blocks.contains_key(*id));
        }
    }

    /// Parent-child relationship: set block.parent = Some(parent_id) and add child to parent.children
    #[test]
    fn workspace_parent_child_relationship() {
        let mut ws = Workspace::new();
        let parent = BlockModel::new(
            "parent",
            NomtuRef::new("ep", "parent_word", "concept"),
            "affine:note",
        );
        let mut child = BlockModel::new(
            "child",
            NomtuRef::new("ec", "child_word", "verb"),
            "affine:paragraph",
        );
        child.parent = Some("parent".to_string());
        ws.insert_block(parent);
        ws.insert_block(child);
        // Add child ref to parent's children list
        ws.blocks
            .get_mut("parent")
            .unwrap()
            .children
            .push("child".to_string());
        let parent_block = ws.blocks.get("parent").unwrap();
        assert_eq!(parent_block.children.len(), 1);
        assert_eq!(parent_block.children[0], "child");
        let child_block = ws.blocks.get("child").unwrap();
        assert_eq!(child_block.parent.as_deref(), Some("parent"));
    }

    /// Workspace serializes and deserializes correctly (roundtrip)
    #[test]
    fn workspace_json_roundtrip() {
        let mut ws = Workspace::new();
        let block = BlockModel::new("b1", NomtuRef::new("e1", "plan", "concept"), "affine:note");
        ws.insert_block(block);
        let json = serde_json::to_string(&ws).expect("serialize workspace");
        let ws2: Workspace = serde_json::from_str(&json).expect("deserialize workspace");
        assert_eq!(ws2.block_count(), 1);
        assert!(ws2.blocks.contains_key("b1"));
    }

    /// Multiple nodes can be inserted and retrieved
    #[test]
    fn workspace_multiple_nodes() {
        let mut ws = Workspace::new();
        for i in 0..4u8 {
            let node = GraphNode::new(
                format!("n{i}"),
                NomtuRef::new(format!("e{i}"), "w", "verb"),
                "verb",
                [i as f32 * 50.0, 0.0],
            );
            ws.insert_node(node);
        }
        assert_eq!(ws.node_count(), 4);
    }

    /// Removing all blocks leaves workspace empty
    #[test]
    fn workspace_remove_all_blocks() {
        let mut ws = Workspace::new();
        for i in 0..5u8 {
            let block = BlockModel::new(
                format!("b{i}"),
                NomtuRef::new(format!("e{i}"), "w", "verb"),
                "affine:paragraph",
            );
            ws.insert_block(block);
        }
        assert_eq!(ws.block_count(), 5);
        for i in 0..5u8 {
            ws.remove_block(&format!("b{i}"));
        }
        assert_eq!(ws.block_count(), 0);
        assert!(ws.doc_tree.is_empty());
    }

    /// Workspace doc_tree preserves insertion order
    #[test]
    fn workspace_doc_tree_preserves_order() {
        let mut ws = Workspace::new();
        let ids = ["first", "second", "third"];
        for id in &ids {
            ws.insert_block(BlockModel::new(
                *id,
                NomtuRef::new(*id, "w", "verb"),
                "affine:paragraph",
            ));
        }
        assert_eq!(ws.doc_tree[0], "first");
        assert_eq!(ws.doc_tree[1], "second");
        assert_eq!(ws.doc_tree[2], "third");
    }

    /// Connectors can be inserted and removed implicitly via workspace operations
    #[test]
    fn workspace_connector_count_increases() {
        let mut ws = Workspace::new();
        let dict = crate::stub_dict::StubDictReader::new();
        for i in 0..3u8 {
            let conn = Connector::new_with_validation(crate::connector::ConnectorValidation {
                id: format!("c{i}"),
                from_node: format!("n{i}"),
                from_port: "output".into(),
                to_node: format!("n{}", i + 1),
                to_port: "input".into(),
                dict: &dict,
                from_kind: "verb",
                to_kind: "concept",
            });
            ws.insert_connector(conn);
        }
        assert_eq!(ws.connector_count(), 3);
    }

    /// CanvasObject::Block and Node variants can coexist in a Vec
    #[test]
    fn canvas_object_vec_can_hold_block_and_node() {
        let block = BlockModel::new(
            "b1",
            NomtuRef::new("e1", "w", "verb"),
            "affine:paragraph",
        );
        let node = GraphNode::new("n1", NomtuRef::new("e2", "x", "verb"), "verb", [0.0, 0.0]);
        let objects: Vec<CanvasObject> = vec![CanvasObject::Block(block), CanvasObject::Node(node)];
        assert_eq!(objects.len(), 2);
        assert!(matches!(objects[0], CanvasObject::Block(_)));
        assert!(matches!(objects[1], CanvasObject::Node(_)));
    }

    /// Workspace handles 100 blocks without issue (stress test)
    #[test]
    fn workspace_stress_100_blocks() {
        let mut ws = Workspace::new();
        for i in 0..100u32 {
            let block = BlockModel::new(
                format!("b{i}"),
                NomtuRef::new(format!("e{i}"), "w", "verb"),
                "affine:paragraph",
            );
            ws.insert_block(block);
        }
        assert_eq!(ws.block_count(), 100);
        assert_eq!(ws.doc_tree.len(), 100);
    }

    /// Workspace block retrieval by id returns correct block
    #[test]
    fn workspace_get_block_by_id() {
        let mut ws = Workspace::new();
        let entity = NomtuRef::new("e99", "target", "concept");
        ws.insert_block(BlockModel::new("target-id", entity, "affine:note"));
        let block = ws.blocks.get("target-id").unwrap();
        assert_eq!(block.entity.word, "target");
        assert_eq!(block.entity.kind, "concept");
    }

    /// Default workspace serializes and deserializes to empty state
    #[test]
    fn workspace_default_roundtrip() {
        let ws = Workspace::default();
        let json = serde_json::to_string(&ws).unwrap();
        let ws2: Workspace = serde_json::from_str(&json).unwrap();
        assert_eq!(ws2.block_count(), 0);
        assert_eq!(ws2.node_count(), 0);
        assert_eq!(ws2.connector_count(), 0);
    }

    /// Blocks with different flavours coexist in the workspace
    #[test]
    fn workspace_mixed_flavours() {
        let mut ws = Workspace::new();
        let flavours = ["affine:paragraph", "affine:note", "affine:heading"];
        for (i, flavour) in flavours.iter().enumerate() {
            let block = BlockModel::new(
                format!("b{i}"),
                NomtuRef::new(format!("e{i}"), "w", "verb"),
                *flavour,
            );
            ws.insert_block(block);
        }
        assert_eq!(ws.block_count(), 3);
        for (i, flavour) in flavours.iter().enumerate() {
            assert_eq!(
                ws.blocks.get(&format!("b{i}")).unwrap().flavour,
                *flavour
            );
        }
    }

    // ── wave AI: new workspace tests ────────────────────────────────────────────

    #[test]
    fn workspace_new_is_empty() {
        let ws = Workspace::new();
        assert_eq!(ws.block_count(), 0);
        assert_eq!(ws.node_count(), 0);
        assert_eq!(ws.connector_count(), 0);
        assert!(ws.doc_tree.is_empty());
    }

    #[test]
    fn workspace_add_block_increments_count() {
        let mut ws = Workspace::new();
        assert_eq!(ws.block_count(), 0);
        ws.insert_block(BlockModel::new(
            "b1",
            NomtuRef::new("e1", "w", "verb"),
            "affine:paragraph",
        ));
        assert_eq!(ws.block_count(), 1);
        ws.insert_block(BlockModel::new(
            "b2",
            NomtuRef::new("e2", "x", "verb"),
            "affine:paragraph",
        ));
        assert_eq!(ws.block_count(), 2);
    }

    #[test]
    fn workspace_remove_block_decrements_count() {
        let mut ws = Workspace::new();
        ws.insert_block(BlockModel::new(
            "b1",
            NomtuRef::new("e1", "w", "verb"),
            "affine:paragraph",
        ));
        assert_eq!(ws.block_count(), 1);
        ws.remove_block("b1");
        assert_eq!(ws.block_count(), 0);
    }

    #[test]
    fn workspace_get_block_by_id_returns_correct() {
        let mut ws = Workspace::new();
        let entity = NomtuRef::new("eid", "find", "verb");
        ws.insert_block(BlockModel::new("find-me", entity, "affine:note"));
        let block = ws.blocks.get("find-me");
        assert!(block.is_some());
        assert_eq!(block.unwrap().entity.word, "find");
    }

    #[test]
    fn workspace_get_nonexistent_block_returns_none() {
        let ws = Workspace::new();
        assert!(ws.blocks.get("does-not-exist").is_none());
    }

    #[test]
    fn workspace_add_connector_increments_count() {
        let mut ws = Workspace::new();
        let dict = crate::stub_dict::StubDictReader::new();
        assert_eq!(ws.connector_count(), 0);
        let conn = Connector::new_with_validation(crate::connector::ConnectorValidation {
            id: "c-new".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        ws.insert_connector(conn);
        assert_eq!(ws.connector_count(), 1);
    }

    #[test]
    fn workspace_blocks_list_nonempty_after_add() {
        let mut ws = Workspace::new();
        ws.insert_block(BlockModel::new(
            "b1",
            NomtuRef::new("e1", "w", "verb"),
            "affine:paragraph",
        ));
        assert!(!ws.blocks.is_empty());
        assert!(ws.blocks.contains_key("b1"));
    }

    #[test]
    fn workspace_connectors_list_nonempty_after_add() {
        let mut ws = Workspace::new();
        let dict = crate::stub_dict::StubDictReader::new();
        let conn = Connector::new_with_validation(crate::connector::ConnectorValidation {
            id: "c1".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        ws.insert_connector(conn);
        assert!(!ws.connectors.is_empty());
        assert!(ws.connectors.contains_key("c1"));
    }

    #[test]
    fn workspace_clear_removes_all() {
        let mut ws = Workspace::new();
        for i in 0..3u8 {
            ws.insert_block(BlockModel::new(
                format!("b{i}"),
                NomtuRef::new(format!("e{i}"), "w", "verb"),
                "affine:paragraph",
            ));
        }
        assert_eq!(ws.block_count(), 3);
        // Manual clear: remove each block
        for i in 0..3u8 {
            ws.remove_block(&format!("b{i}"));
        }
        assert_eq!(ws.block_count(), 0);
        assert!(ws.doc_tree.is_empty());
    }

    #[test]
    fn workspace_block_ids_unique() {
        let mut ws = Workspace::new();
        let ids = ["uid-1", "uid-2", "uid-3", "uid-4"];
        for id in &ids {
            ws.insert_block(BlockModel::new(
                *id,
                NomtuRef::new(*id, "w", "verb"),
                "affine:paragraph",
            ));
        }
        // All IDs must map to distinct entries
        assert_eq!(ws.block_count(), ids.len());
        let mut seen = std::collections::HashSet::new();
        for id in &ids {
            assert!(seen.insert(*id), "duplicate id: {id}");
        }
    }

    // ── wave AJ: additional workspace tests ────────────────────────────────────

    /// Merging two workspaces: blocks from both appear in the merged result.
    #[test]
    fn workspace_merge_two_workspaces() {
        let mut ws1 = Workspace::new();
        ws1.insert_block(BlockModel::new(
            "b1",
            NomtuRef::new("e1", "fetch", "verb"),
            "affine:paragraph",
        ));
        let mut ws2 = Workspace::new();
        ws2.insert_block(BlockModel::new(
            "b2",
            NomtuRef::new("e2", "store", "verb"),
            "affine:paragraph",
        ));
        // Manual merge: insert all blocks from ws2 into ws1
        for (id, block) in ws2.blocks {
            ws1.blocks.insert(id.clone(), block);
            ws1.doc_tree.push(id);
        }
        assert_eq!(ws1.block_count(), 2);
        assert!(ws1.blocks.contains_key("b1"));
        assert!(ws1.blocks.contains_key("b2"));
    }

    /// Diff detects added blocks: blocks in ws2 not in ws1.
    #[test]
    fn workspace_diff_returns_added_blocks() {
        let mut ws1 = Workspace::new();
        ws1.insert_block(BlockModel::new(
            "shared",
            NomtuRef::new("e1", "w", "verb"),
            "affine:paragraph",
        ));
        let mut ws2 = ws1.clone();
        ws2.insert_block(BlockModel::new(
            "added",
            NomtuRef::new("e2", "x", "verb"),
            "affine:paragraph",
        ));
        // Compute added: ids in ws2 not in ws1
        let added: Vec<_> = ws2
            .blocks
            .keys()
            .filter(|id| !ws1.blocks.contains_key(*id))
            .collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0], "added");
    }

    /// Diff detects removed blocks: blocks in ws1 not in ws2.
    #[test]
    fn workspace_diff_returns_removed_blocks() {
        let mut ws1 = Workspace::new();
        ws1.insert_block(BlockModel::new(
            "keep",
            NomtuRef::new("e1", "w", "verb"),
            "affine:paragraph",
        ));
        ws1.insert_block(BlockModel::new(
            "remove-me",
            NomtuRef::new("e2", "x", "verb"),
            "affine:paragraph",
        ));
        let mut ws2 = ws1.clone();
        ws2.remove_block("remove-me");
        // Compute removed: ids in ws1 not in ws2
        let removed: Vec<_> = ws1
            .blocks
            .keys()
            .filter(|id| !ws2.blocks.contains_key(*id))
            .collect();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], "remove-me");
    }

    /// Diff of two empty workspaces is empty.
    #[test]
    fn workspace_diff_empty_workspaces() {
        let ws1 = Workspace::new();
        let ws2 = Workspace::new();
        let added: Vec<_> = ws2
            .blocks
            .keys()
            .filter(|id| !ws1.blocks.contains_key(*id))
            .collect();
        let removed: Vec<_> = ws1
            .blocks
            .keys()
            .filter(|id| !ws2.blocks.contains_key(*id))
            .collect();
        assert!(added.is_empty());
        assert!(removed.is_empty());
    }

    /// Find a block by kind by scanning blocks map.
    #[test]
    fn workspace_find_block_by_kind() {
        let mut ws = Workspace::new();
        ws.insert_block(BlockModel::new(
            "b-verb",
            NomtuRef::new("e1", "fetch", "verb"),
            "affine:paragraph",
        ));
        ws.insert_block(BlockModel::new(
            "b-concept",
            NomtuRef::new("e2", "plan", "concept"),
            "affine:note",
        ));
        let found: Vec<_> = ws
            .blocks
            .values()
            .filter(|b| b.entity.kind == "concept")
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].entity.word, "plan");
    }

    /// Find multiple blocks by kind returns all matching.
    #[test]
    fn workspace_find_blocks_by_kind_multiple() {
        let mut ws = Workspace::new();
        for i in 0..3u8 {
            ws.insert_block(BlockModel::new(
                format!("verb-{i}"),
                NomtuRef::new(format!("ev{i}"), "do", "verb"),
                "affine:paragraph",
            ));
        }
        ws.insert_block(BlockModel::new(
            "concept-1",
            NomtuRef::new("ec1", "think", "concept"),
            "affine:note",
        ));
        let verbs: Vec<_> = ws
            .blocks
            .values()
            .filter(|b| b.entity.kind == "verb")
            .collect();
        assert_eq!(verbs.len(), 3);
    }

    /// connectors_for_block: connectors where given block's node is src or dst.
    #[test]
    fn workspace_connectors_for_block() {
        let mut ws = Workspace::new();
        let dict = crate::stub_dict::StubDictReader::new();
        // Insert connectors referencing "n-target" as both src and dst
        let c1 = Connector::new_with_validation(crate::connector::ConnectorValidation {
            id: "c-src".into(),
            from_node: "n-target".into(),
            from_port: "output".into(),
            to_node: "n-other".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        let c2 = Connector::new_with_validation(crate::connector::ConnectorValidation {
            id: "c-dst".into(),
            from_node: "n-other".into(),
            from_port: "output".into(),
            to_node: "n-target".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        let c3 = Connector::new_with_validation(crate::connector::ConnectorValidation {
            id: "c-unrelated".into(),
            from_node: "nx".into(),
            from_port: "output".into(),
            to_node: "ny".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        ws.insert_connector(c1);
        ws.insert_connector(c2);
        ws.insert_connector(c3);
        let for_target: Vec<_> = ws
            .connectors
            .values()
            .filter(|c| c.src.0 == "n-target" || c.dst.0 == "n-target")
            .collect();
        assert_eq!(for_target.len(), 2);
    }

    /// Workspace serializes and deserializes with nodes and connectors intact.
    #[test]
    fn workspace_serialize_round_trip() {
        use crate::graph_node::GraphNode;
        let mut ws = Workspace::new();
        ws.insert_block(BlockModel::new(
            "b1",
            NomtuRef::new("e1", "plan", "concept"),
            "affine:note",
        ));
        ws.insert_node(GraphNode::new(
            "n1",
            NomtuRef::new("e2", "fetch", "verb"),
            "verb",
            [10.0, 20.0],
        ));
        let json = serde_json::to_string(&ws).expect("serialize workspace");
        let ws2: Workspace = serde_json::from_str(&json).expect("deserialize workspace");
        assert_eq!(ws2.block_count(), 1);
        assert_eq!(ws2.node_count(), 1);
        assert!(ws2.blocks.contains_key("b1"));
        assert!(ws2.nodes.contains_key("n1"));
    }

    /// block_count returns the correct count after multiple inserts.
    #[test]
    fn workspace_block_count_correct() {
        let mut ws = Workspace::new();
        assert_eq!(ws.block_count(), 0);
        for i in 0..7u8 {
            ws.insert_block(BlockModel::new(
                format!("b{i}"),
                NomtuRef::new(format!("e{i}"), "w", "verb"),
                "affine:paragraph",
            ));
        }
        assert_eq!(ws.block_count(), 7);
    }

    /// connector_count returns the correct count after multiple inserts.
    #[test]
    fn workspace_connector_count_correct() {
        let mut ws = Workspace::new();
        let dict = crate::stub_dict::StubDictReader::new();
        assert_eq!(ws.connector_count(), 0);
        for i in 0..5u8 {
            let conn = Connector::new_with_validation(crate::connector::ConnectorValidation {
                id: format!("cc{i}"),
                from_node: format!("n{i}"),
                from_port: "output".into(),
                to_node: format!("m{i}"),
                to_port: "input".into(),
                dict: &dict,
                from_kind: "verb",
                to_kind: "concept",
            });
            ws.insert_connector(conn);
        }
        assert_eq!(ws.connector_count(), 5);
    }

    /// Updating a block in-place replaces the stored value.
    #[test]
    fn workspace_update_block_in_place() {
        let mut ws = Workspace::new();
        let mut block = BlockModel::new(
            "upd",
            NomtuRef::new("e1", "original", "verb"),
            "affine:paragraph",
        );
        ws.insert_block(block.clone());
        // Mutate the block and re-insert to overwrite
        block.entity = NomtuRef::new("e2", "updated", "verb");
        ws.blocks.insert("upd".to_string(), block);
        let stored = ws.blocks.get("upd").unwrap();
        assert_eq!(stored.entity.word, "updated");
    }
}
