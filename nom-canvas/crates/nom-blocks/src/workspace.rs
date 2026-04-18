#![deny(unsafe_code)]
use crate::block_model::{BlockId, BlockModel, NomtuRef};
use crate::connector::{Connector, ConnectorId};
use crate::graph_node::{GraphNode, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CanvasObject {
    Block(BlockModel),
    Node(GraphNode),
    Connector(Connector),
}

impl CanvasObject {
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Workspace {
    pub blocks: HashMap<BlockId, BlockModel>,
    pub nodes: HashMap<NodeId, GraphNode>,
    pub connectors: HashMap<ConnectorId, Connector>,
    pub doc_tree: Vec<BlockId>,
}

impl Workspace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_block(&mut self, block: BlockModel) {
        self.doc_tree.push(block.id.clone());
        self.blocks.insert(block.id.clone(), block);
    }

    pub fn insert_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn insert_connector(&mut self, connector: Connector) {
        self.connectors.insert(connector.id.clone(), connector);
    }

    pub fn remove_block(&mut self, id: &str) -> Option<BlockModel> {
        self.doc_tree.retain(|bid| bid != id);
        self.blocks.remove(id)
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
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
}
