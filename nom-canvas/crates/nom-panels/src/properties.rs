//! Properties inspector panel — fields for the currently selected block.

use smallvec::SmallVec;

/// The kind of a property field, determining its editor widget.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Text,
    Number,
    Boolean,
    Color,
    Select(Vec<String>),
}

/// The runtime value of a property field.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    /// RGBA, each component in [0.0, 1.0].
    Color([f32; 4]),
}

/// A single property field shown in the inspector.
#[derive(Debug, Clone)]
pub struct PropertyField {
    pub name: String,
    pub kind: FieldKind,
    pub value: FieldValue,
}

impl PropertyField {
    pub fn new(name: impl Into<String>, kind: FieldKind, value: FieldValue) -> Self {
        Self {
            name: name.into(),
            kind,
            value,
        }
    }
}

/// Property inspector panel state.
#[derive(Debug)]
pub struct PropertyInspector {
    pub selected_block: Option<String>,
    pub fields: SmallVec<[PropertyField; 8]>,
}

impl PropertyInspector {
    pub fn new() -> Self {
        Self {
            selected_block: None,
            fields: SmallVec::new(),
        }
    }

    /// Select a block by its id and clear the current fields.
    pub fn select(&mut self, block_id: impl Into<String>) {
        self.selected_block = Some(block_id.into());
        self.fields.clear();
    }

    /// Clear selection.
    pub fn deselect(&mut self) {
        self.selected_block = None;
        self.fields.clear();
    }

    /// Look up a field by name.
    pub fn field_by_name(&self, name: &str) -> Option<&PropertyField> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Stub paint method — rendering lives in the GPU layer.
    pub fn paint(&self) {}
}

impl Default for PropertyInspector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_by_default() {
        let pi = PropertyInspector::new();
        assert!(pi.selected_block.is_none());
        assert!(pi.fields.is_empty());
    }

    #[test]
    fn field_lookup_by_name() {
        let mut pi = PropertyInspector::new();
        pi.fields.push(PropertyField::new(
            "opacity",
            FieldKind::Number,
            FieldValue::Number(1.0),
        ));
        assert!(pi.field_by_name("opacity").is_some());
        assert!(pi.field_by_name("missing").is_none());
    }

    #[test]
    fn select_clears_fields() {
        let mut pi = PropertyInspector::new();
        pi.fields.push(PropertyField::new(
            "x",
            FieldKind::Number,
            FieldValue::Number(0.0),
        ));
        pi.select("block-1");
        assert!(pi.fields.is_empty());
        assert_eq!(pi.selected_block.as_deref(), Some("block-1"));
    }

    #[test]
    fn deselect_clears_all() {
        let mut pi = PropertyInspector::new();
        pi.select("block-1");
        pi.deselect();
        assert!(pi.selected_block.is_none());
    }
}
