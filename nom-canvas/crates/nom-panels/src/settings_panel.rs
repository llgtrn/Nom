//! Settings panel view-model.
#![deny(unsafe_code)]

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsSection {
    Editor,
    Appearance,
    Network,
    Collaboration,
    Advanced,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Choice(String),
    Multiline(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SettingSchema {
    pub key: String,
    pub section: SettingsSection,
    pub label: String,
    pub description: Option<String>,
    pub default: SettingValue,
    pub choices: Option<Vec<String>>,
    pub experimental: bool,
}

impl SettingSchema {
    pub fn new(
        key: impl Into<String>,
        section: SettingsSection,
        label: impl Into<String>,
        default: SettingValue,
    ) -> Self {
        Self {
            key: key.into(),
            section,
            label: label.into(),
            description: None,
            default,
            choices: None,
            experimental: false,
        }
    }

    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }

    pub fn with_choices(mut self, choices: Vec<String>) -> Self {
        self.choices = Some(choices);
        self
    }

    pub fn experimental(mut self) -> Self {
        self.experimental = true;
        self
    }
}

#[derive(Default)]
pub struct SettingsPanel {
    schemas: Vec<SettingSchema>,
    values: HashMap<String, SettingValue>,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("unknown setting key '{0}'")]
    UnknownKey(String),
    #[error("invalid value for setting '{0}' (type mismatch)")]
    TypeMismatch(String),
    #[error("value not in allowed choices for setting '{0}'")]
    NotInChoices(String),
    #[error("duplicate setting key '{0}'")]
    DuplicateKey(String),
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, schema: SettingSchema) -> Result<(), SettingsError> {
        if self.schemas.iter().any(|s| s.key == schema.key) {
            return Err(SettingsError::DuplicateKey(schema.key));
        }
        self.schemas.push(schema);
        Ok(())
    }

    pub fn schema(&self, key: &str) -> Option<&SettingSchema> {
        self.schemas.iter().find(|s| s.key == key)
    }

    pub fn set(&mut self, key: &str, value: SettingValue) -> Result<(), SettingsError> {
        let schema = self
            .schemas
            .iter()
            .find(|s| s.key == key)
            .ok_or_else(|| SettingsError::UnknownKey(key.to_string()))?;

        let ok = match (&schema.default, &value) {
            (SettingValue::Bool(_), SettingValue::Bool(_)) => true,
            (SettingValue::Int(_), SettingValue::Int(_)) => true,
            (SettingValue::Float(_), SettingValue::Float(_)) => true,
            (SettingValue::Text(_), SettingValue::Text(_)) => true,
            (SettingValue::Choice(_), SettingValue::Choice(_)) => true,
            (SettingValue::Multiline(_), SettingValue::Multiline(_)) => true,
            _ => false,
        };
        if !ok {
            return Err(SettingsError::TypeMismatch(key.to_string()));
        }

        if let SettingValue::Choice(ref v) = value {
            if let Some(ref choices) = schema.choices {
                if !choices.contains(v) {
                    return Err(SettingsError::NotInChoices(key.to_string()));
                }
            }
        }

        self.values.insert(key.to_string(), value);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&SettingValue> {
        self.values.get(key).or_else(|| {
            self.schemas
                .iter()
                .find(|s| s.key == key)
                .map(|s| &s.default)
        })
    }

    pub fn reset(&mut self, key: &str) -> bool {
        self.values.remove(key).is_some()
    }

    pub fn reset_all(&mut self) {
        self.values.clear();
    }

    pub fn settings_in_section(&self, section: SettingsSection) -> Vec<&SettingSchema> {
        self.schemas.iter().filter(|s| s.section == section).collect()
    }

    pub fn is_modified(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn modified_keys(&self) -> Vec<&String> {
        self.values.keys().collect()
    }

    pub fn schema_count(&self) -> usize {
        self.schemas.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bool_schema() -> SettingSchema {
        SettingSchema::new("editor.word_wrap", SettingsSection::Editor, "Word Wrap", SettingValue::Bool(false))
    }

    fn make_choice_schema() -> SettingSchema {
        SettingSchema::new("appearance.theme", SettingsSection::Appearance, "Theme", SettingValue::Choice("dark".into()))
            .with_choices(vec!["dark".into(), "light".into(), "solarized".into()])
    }

    // 1. SettingSchema::new defaults
    #[test]
    fn schema_new_defaults() {
        let s = make_bool_schema();
        assert_eq!(s.key, "editor.word_wrap");
        assert_eq!(s.section, SettingsSection::Editor);
        assert_eq!(s.label, "Word Wrap");
        assert_eq!(s.default, SettingValue::Bool(false));
        assert!(s.description.is_none());
        assert!(s.choices.is_none());
        assert!(!s.experimental);
    }

    // 2. Builder chain
    #[test]
    fn schema_builder_chain() {
        let s = SettingSchema::new("adv.key", SettingsSection::Advanced, "Adv", SettingValue::Int(0))
            .with_description("An advanced setting")
            .with_choices(vec!["a".into(), "b".into()])
            .experimental();
        assert_eq!(s.description.as_deref(), Some("An advanced setting"));
        assert_eq!(s.choices.as_ref().unwrap().len(), 2);
        assert!(s.experimental);
    }

    // 3. SettingsPanel::new empty
    #[test]
    fn panel_new_empty() {
        let p = SettingsPanel::new();
        assert_eq!(p.schema_count(), 0);
        assert!(p.modified_keys().is_empty());
    }

    // 4. register success + duplicate rejected
    #[test]
    fn register_success_and_duplicate() {
        let mut p = SettingsPanel::new();
        assert!(p.register(make_bool_schema()).is_ok());
        assert_eq!(p.schema_count(), 1);
        let err = p.register(make_bool_schema()).unwrap_err();
        assert!(matches!(err, SettingsError::DuplicateKey(_)));
    }

    // 5. schema lookup hit + miss
    #[test]
    fn schema_lookup() {
        let mut p = SettingsPanel::new();
        p.register(make_bool_schema()).unwrap();
        assert!(p.schema("editor.word_wrap").is_some());
        assert!(p.schema("nonexistent").is_none());
    }

    // 6. get returns default when unset
    #[test]
    fn get_returns_default_when_unset() {
        let mut p = SettingsPanel::new();
        p.register(make_bool_schema()).unwrap();
        assert_eq!(p.get("editor.word_wrap"), Some(&SettingValue::Bool(false)));
    }

    // 7. set type match + get returns new value
    #[test]
    fn set_type_match_and_get() {
        let mut p = SettingsPanel::new();
        p.register(make_bool_schema()).unwrap();
        p.set("editor.word_wrap", SettingValue::Bool(true)).unwrap();
        assert_eq!(p.get("editor.word_wrap"), Some(&SettingValue::Bool(true)));
    }

    // 8. set type mismatch -> TypeMismatch
    #[test]
    fn set_type_mismatch() {
        let mut p = SettingsPanel::new();
        p.register(make_bool_schema()).unwrap();
        let err = p.set("editor.word_wrap", SettingValue::Int(1)).unwrap_err();
        assert!(matches!(err, SettingsError::TypeMismatch(_)));
    }

    // 9. set Choice not in choices -> NotInChoices
    #[test]
    fn set_choice_not_in_choices() {
        let mut p = SettingsPanel::new();
        p.register(make_choice_schema()).unwrap();
        let err = p.set("appearance.theme", SettingValue::Choice("neon".into())).unwrap_err();
        assert!(matches!(err, SettingsError::NotInChoices(_)));
    }

    // 10. set unknown key -> UnknownKey
    #[test]
    fn set_unknown_key() {
        let mut p = SettingsPanel::new();
        let err = p.set("no.such.key", SettingValue::Bool(true)).unwrap_err();
        assert!(matches!(err, SettingsError::UnknownKey(_)));
    }

    // 11. reset removes override
    #[test]
    fn reset_removes_override() {
        let mut p = SettingsPanel::new();
        p.register(make_bool_schema()).unwrap();
        p.set("editor.word_wrap", SettingValue::Bool(true)).unwrap();
        assert!(p.is_modified("editor.word_wrap"));
        assert!(p.reset("editor.word_wrap"));
        assert!(!p.is_modified("editor.word_wrap"));
        assert_eq!(p.get("editor.word_wrap"), Some(&SettingValue::Bool(false)));
    }

    // 12. reset_all clears
    #[test]
    fn reset_all_clears() {
        let mut p = SettingsPanel::new();
        p.register(make_bool_schema()).unwrap();
        p.set("editor.word_wrap", SettingValue::Bool(true)).unwrap();
        p.reset_all();
        assert!(p.modified_keys().is_empty());
    }

    // 13. settings_in_section filters
    #[test]
    fn settings_in_section_filters() {
        let mut p = SettingsPanel::new();
        p.register(make_bool_schema()).unwrap();
        p.register(make_choice_schema()).unwrap();
        p.register(
            SettingSchema::new("network.timeout", SettingsSection::Network, "Timeout", SettingValue::Int(30))
        ).unwrap();
        assert_eq!(p.settings_in_section(SettingsSection::Editor).len(), 1);
        assert_eq!(p.settings_in_section(SettingsSection::Appearance).len(), 1);
        assert_eq!(p.settings_in_section(SettingsSection::Network).len(), 1);
        assert_eq!(p.settings_in_section(SettingsSection::Collaboration).len(), 0);
    }

    // 14. is_modified / modified_keys
    #[test]
    fn is_modified_and_modified_keys() {
        let mut p = SettingsPanel::new();
        p.register(make_bool_schema()).unwrap();
        assert!(!p.is_modified("editor.word_wrap"));
        p.set("editor.word_wrap", SettingValue::Bool(true)).unwrap();
        assert!(p.is_modified("editor.word_wrap"));
        assert_eq!(p.modified_keys().len(), 1);
    }

    // 15. schema_count
    #[test]
    fn schema_count_tracks_registrations() {
        let mut p = SettingsPanel::new();
        assert_eq!(p.schema_count(), 0);
        p.register(make_bool_schema()).unwrap();
        assert_eq!(p.schema_count(), 1);
        p.register(make_choice_schema()).unwrap();
        assert_eq!(p.schema_count(), 2);
    }
}
