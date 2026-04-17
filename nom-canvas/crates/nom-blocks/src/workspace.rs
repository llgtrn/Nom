#![deny(unsafe_code)]
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::block_model::{BlockId, BlockModel, NomtuRef};
use crate::graph_node::{GraphNode, NodeId};
use crate::connector::{Connector, ConnectorId};

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
            CanvasObject::Connector(_) => panic!("Connectors don't have a direct NomtuRef — use src/dst node entities"),
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

    pub fn block_count(&self) -> usize { self.blocks.len() }
    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn connector_count(&self) -> usize { self.connectors.len() }
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

        let conn = Connector::new_stub("c1", "n1", "output", "n2", "input");
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
}
