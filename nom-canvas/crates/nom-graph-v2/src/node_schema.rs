pub type NodeId = u64;

#[derive(Debug, Clone)]
pub struct NodeSchema {
    pub id: NodeId,
    pub class_type: String,
    pub input_types: Vec<(String, String)>,
    pub output_types: Vec<String>,
    pub output_is_list: Vec<bool>,
    pub has_side_effects: bool,
}

impl NodeSchema {
    pub fn new(id: NodeId, class_type: impl Into<String>) -> Self {
        Self {
            id,
            class_type: class_type.into(),
            input_types: Vec::new(),
            output_types: Vec::new(),
            output_is_list: Vec::new(),
            has_side_effects: false,
        }
    }

    pub fn with_input(mut self, name: impl Into<String>, ty: impl Into<String>) -> Self {
        self.input_types.push((name.into(), ty.into()));
        self
    }

    pub fn with_output(mut self, ty: impl Into<String>) -> Self {
        self.output_types.push(ty.into());
        self.output_is_list.push(false);
        self
    }

    pub fn side_effect(mut self) -> Self {
        self.has_side_effects = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let s = NodeSchema::new(1, "LoadImage");
        assert_eq!(s.id, 1);
        assert_eq!(s.class_type, "LoadImage");
        assert!(s.input_types.is_empty());
        assert!(!s.has_side_effects);
    }

    #[test]
    fn builder_chain() {
        let s = NodeSchema::new(2, "CLIPTextEncode")
            .with_input("text", "STRING")
            .with_input("clip", "CLIP")
            .with_output("CONDITIONING");
        assert_eq!(s.input_types.len(), 2);
        assert_eq!(s.output_types[0], "CONDITIONING");
    }

    #[test]
    fn side_effect_flag() {
        let s = NodeSchema::new(3, "SaveImage").side_effect();
        assert!(s.has_side_effects);
    }

    #[test]
    fn output_is_list_tracks_outputs() {
        let s = NodeSchema::new(4, "Multi").with_output("A").with_output("B");
        assert_eq!(s.output_is_list.len(), 2);
    }
}
