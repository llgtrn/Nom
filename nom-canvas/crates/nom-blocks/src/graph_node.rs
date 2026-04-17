//! Graph node (DAG compute node) block schema.
#![deny(unsafe_code)]

use crate::block_model::BlockId;
use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{GRAPH_NODE, MEDIA_IMAGE, NOMX, PROSE, SURFACE};

pub type FractionalIndex = String;

#[derive(Clone, Debug, PartialEq)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub id: String,
    pub name: String,
    pub direction: PortDirection,
    pub kind: String,
    pub is_list: bool,
    pub required: bool,
}

impl Port {
    pub fn input(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            direction: PortDirection::Input,
            kind: kind.into(),
            is_list: false,
            required: true,
        }
    }

    pub fn output(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            direction: PortDirection::Output,
            kind: kind.into(),
            is_list: false,
            required: false,
        }
    }

    pub fn list_of(mut self) -> Self {
        self.is_list = true;
        self
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNodeProps {
    pub xywh: String,
    pub index: FractionalIndex,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub kind: String,
    pub config: String,
    pub child_element_ids: Vec<BlockId>,
}

impl GraphNodeProps {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            xywh: "0 0 200 100".to_owned(),
            index: "a0".to_owned(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            kind: kind.into(),
            config: "{}".to_owned(),
            child_element_ids: Vec::new(),
        }
    }

    pub fn with_input(mut self, port: Port) -> Self {
        self.inputs.push(port);
        self
    }

    pub fn with_output(mut self, port: Port) -> Self {
        self.outputs.push(port);
        self
    }

    pub fn find_port(&self, id: &str) -> Option<&Port> {
        self.inputs
            .iter()
            .chain(self.outputs.iter())
            .find(|p| p.id == id)
    }

    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }

    pub fn required_inputs(&self) -> impl Iterator<Item = &Port> {
        self.inputs.iter().filter(|p| p.required)
    }
}

pub fn graph_node_schema() -> BlockSchema {
    BlockSchema {
        flavour: GRAPH_NODE,
        version: 1,
        role: Role::Hub,
        parents: &[SURFACE],
        children: &[PROSE, NOMX, MEDIA_IMAGE],
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub from_node: BlockId,
    pub from_port: String,
    pub to_node: BlockId,
    pub to_port: String,
}

impl Edge {
    pub fn new(
        from_node: BlockId,
        from_port: impl Into<String>,
        to_node: BlockId,
        to_port: impl Into<String>,
    ) -> Self {
        Self {
            from_node,
            from_port: from_port.into(),
            to_node,
            to_port: to_port.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::Role;

    #[test]
    fn port_input_defaults_required_true() {
        let p = Port::input("x", "X", "int");
        assert_eq!(p.direction, PortDirection::Input);
        assert!(p.required);
        assert!(!p.is_list);
    }

    #[test]
    fn port_output_defaults_required_false() {
        let p = Port::output("y", "Y", "float");
        assert_eq!(p.direction, PortDirection::Output);
        assert!(!p.required);
        assert!(!p.is_list);
    }

    #[test]
    fn list_of_chains() {
        let p = Port::input("a", "A", "tensor").list_of();
        assert!(p.is_list);
        assert!(p.required);
    }

    #[test]
    fn optional_clears_required() {
        let p = Port::input("b", "B", "any").optional();
        assert!(!p.required);
    }

    #[test]
    fn graph_node_props_new_empty_ports() {
        let props = GraphNodeProps::new("add");
        assert_eq!(props.kind, "add");
        assert_eq!(props.inputs.len(), 0);
        assert_eq!(props.outputs.len(), 0);
        assert_eq!(props.child_element_ids.len(), 0);
        assert_eq!(props.xywh, "0 0 200 100");
        assert_eq!(props.index, "a0");
    }

    #[test]
    fn with_input_with_output_append() {
        let props = GraphNodeProps::new("mul")
            .with_input(Port::input("a", "A", "float"))
            .with_input(Port::input("b", "B", "float"))
            .with_output(Port::output("out", "Out", "float"));
        assert_eq!(props.input_count(), 2);
        assert_eq!(props.output_count(), 1);
    }

    #[test]
    fn find_port_searches_inputs_then_outputs() {
        let props = GraphNodeProps::new("op")
            .with_input(Port::input("in0", "In0", "int"))
            .with_output(Port::output("out0", "Out0", "int"));
        assert!(props.find_port("in0").is_some());
        assert!(props.find_port("out0").is_some());
        assert!(props.find_port("missing").is_none());
        assert_eq!(props.find_port("in0").unwrap().direction, PortDirection::Input);
        assert_eq!(props.find_port("out0").unwrap().direction, PortDirection::Output);
    }

    #[test]
    fn required_inputs_filters_optional() {
        let props = GraphNodeProps::new("op")
            .with_input(Port::input("req", "Req", "int"))
            .with_input(Port::input("opt", "Opt", "int").optional());
        let required: Vec<_> = props.required_inputs().collect();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].id, "req");
    }

    #[test]
    fn graph_node_schema_role_hub() {
        let schema = graph_node_schema();
        assert_eq!(schema.role, Role::Hub);
        assert_eq!(schema.flavour, GRAPH_NODE);
        assert_eq!(schema.version, 1);
        assert!(schema.parents.contains(&SURFACE));
        assert!(schema.children.contains(&PROSE));
        assert!(schema.children.contains(&NOMX));
        assert!(schema.children.contains(&MEDIA_IMAGE));
    }

    #[test]
    fn edge_new_constructs_correctly() {
        let e = Edge::new(1u64, "out0", 2u64, "in0");
        assert_eq!(e.from_node, 1);
        assert_eq!(e.from_port, "out0");
        assert_eq!(e.to_node, 2);
        assert_eq!(e.to_port, "in0");
    }
}
