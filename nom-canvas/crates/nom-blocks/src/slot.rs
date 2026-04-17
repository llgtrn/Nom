#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::block_model::NomtuRef;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SlotValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Ref(NomtuRef),
    List(Vec<SlotValue>),
    Blob { hash: [u8; 32], mime: String },
}

impl SlotValue {
    pub fn as_text(&self) -> Option<&str> {
        if let SlotValue::Text(t) = self { Some(t) } else { None }
    }
    pub fn as_number(&self) -> Option<f64> {
        if let SlotValue::Number(n) = self { Some(*n) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let SlotValue::Bool(b) = self { Some(*b) } else { None }
    }
    pub fn as_ref(&self) -> Option<&NomtuRef> {
        if let SlotValue::Ref(r) = self { Some(r) } else { None }
    }
}

/// Confidence scale: 1.0=explicit, 0.8=inferred from grammar, 0.6=heuristic
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlotBinding {
    pub clause_name: String,
    pub grammar_shape: String,
    pub value: SlotValue,
    pub is_required: bool,
    pub confidence: f32,
    pub reason: String,
}

impl SlotBinding {
    pub fn explicit(clause_name: impl Into<String>, grammar_shape: impl Into<String>, value: SlotValue) -> Self {
        Self {
            clause_name: clause_name.into(),
            grammar_shape: grammar_shape.into(),
            value,
            is_required: true,
            confidence: 1.0,
            reason: "explicit user-set".into(),
        }
    }

    pub fn inferred(clause_name: impl Into<String>, grammar_shape: impl Into<String>, value: SlotValue) -> Self {
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
}
