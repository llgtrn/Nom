/// Describes the type of a property field.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyKind {
    Text,
    Number,
    Boolean,
    Color,
    Enum,
}

impl PropertyKind {
    /// Returns `true` for Text, Number, and Boolean kinds.
    pub fn is_primitive(&self) -> bool {
        matches!(self, PropertyKind::Text | PropertyKind::Number | PropertyKind::Boolean)
    }

    /// Human-readable display name for this kind.
    pub fn display_name(&self) -> &'static str {
        match self {
            PropertyKind::Text => "Text",
            PropertyKind::Number => "Number",
            PropertyKind::Boolean => "Boolean",
            PropertyKind::Color => "Color",
            PropertyKind::Enum => "Enum",
        }
    }
}

/// Holds the current value of a property field.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Color(String),
    Enum(String),
}

impl PropertyValue {
    /// Returns a string representation of the value.
    pub fn as_string(&self) -> String {
        match self {
            PropertyValue::Text(v) => v.clone(),
            PropertyValue::Number(v) => format!("{}", v),
            PropertyValue::Bool(v) => if *v { "true".to_string() } else { "false".to_string() },
            PropertyValue::Color(v) => v.clone(),
            PropertyValue::Enum(v) => v.clone(),
        }
    }
}

/// A single named property with a kind, value, and editability flag.
#[derive(Debug, Clone)]
pub struct PropertyField {
    pub name: String,
    pub kind: PropertyKind,
    pub value: PropertyValue,
    pub read_only: bool,
}

impl PropertyField {
    /// Returns `true` when the field can be edited (i.e. not read-only).
    pub fn is_editable(&self) -> bool {
        !self.read_only
    }
}

/// A labeled collection of property fields.
#[derive(Debug, Clone)]
pub struct PropertyGroup {
    pub label: String,
    pub fields: Vec<PropertyField>,
}

impl PropertyGroup {
    /// Appends a field to this group.
    pub fn add_field(&mut self, f: PropertyField) {
        self.fields.push(f);
    }

    /// Returns references to all editable fields in this group.
    pub fn editable_fields(&self) -> Vec<&PropertyField> {
        self.fields.iter().filter(|f| f.is_editable()).collect()
    }
}

/// Top-level property panel containing one or more groups.
#[derive(Debug, Clone, Default)]
pub struct PropertyPanel {
    pub groups: Vec<PropertyGroup>,
}

impl PropertyPanel {
    /// Appends a group to this panel.
    pub fn add_group(&mut self, g: PropertyGroup) {
        self.groups.push(g);
    }

    /// Returns the total number of fields across all groups.
    pub fn total_fields(&self) -> usize {
        self.groups.iter().map(|g| g.fields.len()).sum()
    }

    /// Searches all groups for a field with the given name.
    pub fn find_field(&self, name: &str) -> Option<&PropertyField> {
        self.groups
            .iter()
            .flat_map(|g| g.fields.iter())
            .find(|f| f.name == name)
    }
}

#[cfg(test)]
mod property_panel_tests {
    use super::*;

    #[test]
    fn kind_is_primitive() {
        assert!(PropertyKind::Text.is_primitive());
        assert!(PropertyKind::Number.is_primitive());
        assert!(PropertyKind::Boolean.is_primitive());
        assert!(!PropertyKind::Color.is_primitive());
        assert!(!PropertyKind::Enum.is_primitive());
    }

    #[test]
    fn kind_display_name() {
        assert_eq!(PropertyKind::Text.display_name(), "Text");
        assert_eq!(PropertyKind::Number.display_name(), "Number");
        assert_eq!(PropertyKind::Boolean.display_name(), "Boolean");
        assert_eq!(PropertyKind::Color.display_name(), "Color");
        assert_eq!(PropertyKind::Enum.display_name(), "Enum");
    }

    #[test]
    fn value_as_string_number() {
        assert_eq!(PropertyValue::Number(3.14).as_string(), "3.14");
        assert_eq!(PropertyValue::Number(0.0).as_string(), "0");
        assert_eq!(PropertyValue::Number(42.0).as_string(), "42");
    }

    #[test]
    fn value_as_string_bool_true() {
        assert_eq!(PropertyValue::Bool(true).as_string(), "true");
    }

    #[test]
    fn value_as_string_bool_false() {
        assert_eq!(PropertyValue::Bool(false).as_string(), "false");
    }

    #[test]
    fn field_is_editable() {
        let editable = PropertyField {
            name: "x".to_string(),
            kind: PropertyKind::Number,
            value: PropertyValue::Number(1.0),
            read_only: false,
        };
        let readonly = PropertyField {
            name: "y".to_string(),
            kind: PropertyKind::Text,
            value: PropertyValue::Text("v".to_string()),
            read_only: true,
        };
        assert!(editable.is_editable());
        assert!(!readonly.is_editable());
    }

    #[test]
    fn group_editable_fields_count() {
        let mut group = PropertyGroup { label: "g".to_string(), fields: vec![] };
        group.add_field(PropertyField {
            name: "a".to_string(),
            kind: PropertyKind::Text,
            value: PropertyValue::Text("v".to_string()),
            read_only: false,
        });
        group.add_field(PropertyField {
            name: "b".to_string(),
            kind: PropertyKind::Boolean,
            value: PropertyValue::Bool(true),
            read_only: true,
        });
        group.add_field(PropertyField {
            name: "c".to_string(),
            kind: PropertyKind::Number,
            value: PropertyValue::Number(5.0),
            read_only: false,
        });
        assert_eq!(group.editable_fields().len(), 2);
    }

    #[test]
    fn panel_total_fields() {
        let mut panel = PropertyPanel::default();
        let mut g1 = PropertyGroup { label: "g1".to_string(), fields: vec![] };
        g1.add_field(PropertyField {
            name: "f1".to_string(),
            kind: PropertyKind::Text,
            value: PropertyValue::Text("a".to_string()),
            read_only: false,
        });
        g1.add_field(PropertyField {
            name: "f2".to_string(),
            kind: PropertyKind::Number,
            value: PropertyValue::Number(1.0),
            read_only: false,
        });
        let mut g2 = PropertyGroup { label: "g2".to_string(), fields: vec![] };
        g2.add_field(PropertyField {
            name: "f3".to_string(),
            kind: PropertyKind::Boolean,
            value: PropertyValue::Bool(false),
            read_only: false,
        });
        panel.add_group(g1);
        panel.add_group(g2);
        assert_eq!(panel.total_fields(), 3);
    }

    #[test]
    fn panel_find_field_found() {
        let mut panel = PropertyPanel::default();
        let mut g = PropertyGroup { label: "g".to_string(), fields: vec![] };
        g.add_field(PropertyField {
            name: "target".to_string(),
            kind: PropertyKind::Color,
            value: PropertyValue::Color("#fff".to_string()),
            read_only: false,
        });
        panel.add_group(g);
        let found = panel.find_field("target");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "target");
    }

    #[test]
    fn panel_find_field_not_found() {
        let panel = PropertyPanel::default();
        assert!(panel.find_field("missing").is_none());
    }
}
