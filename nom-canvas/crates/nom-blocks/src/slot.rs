//! Slot value and binding types.
#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};

/// A typed value that can be stored in a block slot.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SlotValue {
    /// UTF-8 text.
    Text(String),
    /// 64-bit floating-point number.
    Number(f64),
    /// Boolean flag.
    Bool(bool),
    /// Reference to another DB entry.
    Ref(NomtuRef),
    /// Ordered list of slot values.
    List(Vec<SlotValue>),
    /// Raw binary blob identified by a 32-byte hash.
    Blob {
        /// Content-addressed hash.
        hash: [u8; 32],
        /// MIME type string.
        mime: String,
    },
}

impl SlotValue {
    /// Extract the text value if this is a [`SlotValue::Text`].
    pub fn as_text(&self) -> Option<&str> {
        if let SlotValue::Text(t) = self {
            Some(t)
        } else {
            None
        }
    }
    /// Extract the numeric value if this is a [`SlotValue::Number`].
    pub fn as_number(&self) -> Option<f64> {
        if let SlotValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }
    /// Extract the boolean value if this is a [`SlotValue::Bool`].
    pub fn as_bool(&self) -> Option<bool> {
        if let SlotValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
    /// Extract the entity reference if this is a [`SlotValue::Ref`].
    pub fn as_ref(&self) -> Option<&NomtuRef> {
        if let SlotValue::Ref(r) = self {
            Some(r)
        } else {
            None
        }
    }
}

/// Confidence scale: 1.0=explicit, 0.8=inferred from grammar, 0.6=heuristic
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlotBinding {
    /// Name of the clause/slot this binding targets.
    pub clause_name: String,
    /// Grammar type tag for the slot.
    pub grammar_shape: String,
    /// Current value stored in the slot.
    pub value: SlotValue,
    /// Whether the slot is required by the grammar.
    pub is_required: bool,
    /// Confidence score for this binding (0.0–1.0).
    pub confidence: f32,
    /// Reason string explaining the binding source.
    pub reason: String,
}

impl SlotBinding {
    /// Construct an explicit (user-set) binding with confidence 1.0.
    pub fn explicit(
        clause_name: impl Into<String>,
        grammar_shape: impl Into<String>,
        value: SlotValue,
    ) -> Self {
        Self {
            clause_name: clause_name.into(),
            grammar_shape: grammar_shape.into(),
            value,
            is_required: true,
            confidence: 1.0,
            reason: "explicit user-set".into(),
        }
    }

    /// Construct an inferred binding (from grammar) with confidence 0.8.
    pub fn inferred(
        clause_name: impl Into<String>,
        grammar_shape: impl Into<String>,
        value: SlotValue,
    ) -> Self {
        Self {
            clause_name: clause_name.into(),
            grammar_shape: grammar_shape.into(),
            value,
            is_required: false,
            confidence: 0.8,
            reason: "inferred from grammar".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_value_accessors() {
        let sv = SlotValue::Text("hello".into());
        assert_eq!(sv.as_text(), Some("hello"));
        assert_eq!(sv.as_number(), None);
    }

    #[test]
    fn slot_binding_confidence() {
        let sb = SlotBinding::explicit("input", "text", SlotValue::Text("x".into()));
        assert_eq!(sb.confidence, 1.0);
        let sb2 = SlotBinding::inferred("output", "concept", SlotValue::Bool(true));
        assert_eq!(sb2.confidence, 0.8);
    }

    /// SlotValue::Number accessor returns correct value
    #[test]
    fn slot_value_number_accessor() {
        let sv = SlotValue::Number(std::f64::consts::PI);
        assert!((sv.as_number().unwrap() - std::f64::consts::PI).abs() < f64::EPSILON);
        assert!(sv.as_text().is_none());
        assert!(sv.as_bool().is_none());
    }

    /// SlotValue::Bool accessor returns correct value
    #[test]
    fn slot_value_bool_accessor() {
        let sv = SlotValue::Bool(true);
        assert_eq!(sv.as_bool(), Some(true));
        assert!(sv.as_text().is_none());
        assert!(sv.as_number().is_none());
    }

    /// SlotValue::Ref accessor returns the NomtuRef
    #[test]
    fn slot_value_ref_accessor() {
        use crate::block_model::NomtuRef;
        let r = NomtuRef::new("id1", "fetch", "verb");
        let sv = SlotValue::Ref(r.clone());
        let got = sv.as_ref().unwrap();
        assert_eq!(got.id, "id1");
        assert_eq!(got.word, "fetch");
    }

    /// SlotValue::List can nest other SlotValues
    #[test]
    fn slot_value_list_nesting() {
        let list = SlotValue::List(vec![
            SlotValue::Text("a".into()),
            SlotValue::Number(1.0),
            SlotValue::Bool(false),
        ]);
        if let SlotValue::List(items) = &list {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].as_text(), Some("a"));
        } else {
            panic!("expected List variant");
        }
    }

    /// SlotBinding::explicit sets is_required true; inferred sets it false
    #[test]
    fn slot_binding_is_required_flag() {
        let explicit = SlotBinding::explicit("port", "text", SlotValue::Text("v".into()));
        assert!(explicit.is_required);
        let inferred = SlotBinding::inferred("port", "text", SlotValue::Text("v".into()));
        assert!(!inferred.is_required);
    }

    /// SlotBinding reason strings differ between explicit and inferred
    #[test]
    fn slot_binding_reason_strings() {
        let explicit = SlotBinding::explicit("a", "text", SlotValue::Bool(true));
        assert!(explicit.reason.contains("explicit"));
        let inferred = SlotBinding::inferred("b", "text", SlotValue::Bool(false));
        assert!(inferred.reason.contains("inferred") || inferred.reason.contains("grammar"));
    }
}
