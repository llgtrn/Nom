//! Plugin manifest registry for external composition backends.
//!
//! Reads manifest entries from memory or a descriptor file; stores
//! `PluginManifest` records that tell the dispatcher which library
//! should handle which `NomKind`.  Actual dynamic loading via `libloading`
//! lives in a separate runtime crate.
#![deny(unsafe_code)]

use crate::kind::NomKind;
use std::collections::HashMap;
use std::path::PathBuf;

pub type PluginId = String;

#[derive(Clone, Debug, PartialEq)]
pub struct PluginManifest {
    pub id: PluginId,
    pub name: String,
    pub version: String,
    pub author: String,
    pub provides: Vec<NomKind>,
    pub library_path: PathBuf,
    pub capabilities: Vec<String>,
    pub min_nom_version: String,
    pub trusted: bool,
}

impl PluginManifest {
    pub fn new(
        id: impl Into<PluginId>,
        name: impl Into<String>,
        version: impl Into<String>,
        library_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            author: String::new(),
            provides: Vec::new(),
            library_path: library_path.into(),
            capabilities: Vec::new(),
            min_nom_version: "0.1.0".to_string(),
            trusted: false,
        }
    }

    pub fn provides_kind(mut self, kind: NomKind) -> Self {
        self.provides.push(kind);
        self
    }

    pub fn with_capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.push(cap.into());
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    pub fn trust(mut self) -> Self {
        self.trusted = true;
        self
    }

    pub fn handles(&self, kind: NomKind) -> bool {
        self.provides.contains(&kind)
    }
}

#[derive(Default)]
pub struct PluginRegistry {
    plugins: HashMap<PluginId, PluginManifest>,
    /// Resolution order for kinds provided by multiple plugins. When a kind
    /// has several candidates, the first entry in this Vec wins.
    kind_preferences: HashMap<NomKind, Vec<PluginId>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, manifest: PluginManifest) -> Result<(), PluginError> {
        if self.plugins.contains_key(&manifest.id) {
            return Err(PluginError::DuplicateId(manifest.id.clone()));
        }
        for kind in &manifest.provides {
            self.kind_preferences
                .entry(*kind)
                .or_default()
                .push(manifest.id.clone());
        }
        self.plugins.insert(manifest.id.clone(), manifest);
        Ok(())
    }

    pub fn unregister(&mut self, id: &str) -> bool {
        let removed = self.plugins.remove(id).is_some();
        if removed {
            for prefs in self.kind_preferences.values_mut() {
                prefs.retain(|p| p != id);
            }
            self.kind_preferences.retain(|_, v| !v.is_empty());
        }
        removed
    }

    pub fn get(&self, id: &str) -> Option<&PluginManifest> {
        self.plugins.get(id)
    }

    pub fn plugins_for_kind(&self, kind: NomKind) -> Vec<&PluginManifest> {
        self.kind_preferences
            .get(&kind)
            .map(|ids| ids.iter().filter_map(|id| self.plugins.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn preferred_for_kind(&self, kind: NomKind) -> Option<&PluginManifest> {
        self.plugins_for_kind(kind).into_iter().next()
    }

    /// Move `id` to the head of the preference list for `kind`, making it the
    /// new preferred provider.
    pub fn prefer(&mut self, kind: NomKind, id: &str) -> Result<(), PluginError> {
        let prefs = self
            .kind_preferences
            .get_mut(&kind)
            .ok_or(PluginError::UnknownKind(kind))?;
        let pos = prefs
            .iter()
            .position(|p| p == id)
            .ok_or_else(|| PluginError::KindNotProvided(id.to_string(), kind))?;
        let entry = prefs.remove(pos);
        prefs.insert(0, entry);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin id '{0}' already registered")]
    DuplicateId(PluginId),
    #[error("no plugin registered for kind {0:?}")]
    UnknownKind(NomKind),
    #[error("plugin '{0}' does not provide kind {1:?}")]
    KindNotProvided(PluginId, NomKind),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manifest(id: &str) -> PluginManifest {
        PluginManifest::new(id, "Test Plugin", "1.0.0", "/tmp/test.so")
    }

    #[test]
    fn manifest_new_defaults() {
        let m = make_manifest("alpha");
        assert_eq!(m.id, "alpha");
        assert_eq!(m.version, "1.0.0");
        assert!(m.provides.is_empty());
        assert!(m.capabilities.is_empty());
        assert!(!m.trusted);
        assert!(m.author.is_empty());
        assert_eq!(m.min_nom_version, "0.1.0");
    }

    #[test]
    fn manifest_builder_chain() {
        let m = make_manifest("beta")
            .provides_kind(NomKind::MediaVideo)
            .provides_kind(NomKind::MediaAudio)
            .with_capability("gpu")
            .with_author("Alice")
            .trust();
        assert_eq!(m.provides, vec![NomKind::MediaVideo, NomKind::MediaAudio]);
        assert_eq!(m.capabilities, vec!["gpu"]);
        assert_eq!(m.author, "Alice");
        assert!(m.trusted);
    }

    #[test]
    fn handles_matches_listed_kinds_only() {
        let m = make_manifest("gamma").provides_kind(NomKind::ScreenWeb);
        assert!(m.handles(NomKind::ScreenWeb));
        assert!(!m.handles(NomKind::ScreenNative));
        assert!(!m.handles(NomKind::MediaVideo));
    }

    #[test]
    fn registry_new_is_empty() {
        let reg = PluginRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn register_and_get_round_trip() {
        let mut reg = PluginRegistry::new();
        let m = make_manifest("delta").provides_kind(NomKind::DataQuery);
        reg.register(m).unwrap();
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
        let got = reg.get("delta").unwrap();
        assert_eq!(got.id, "delta");
    }

    #[test]
    fn register_duplicate_id_returns_error() {
        let mut reg = PluginRegistry::new();
        reg.register(make_manifest("dup")).unwrap();
        let err = reg.register(make_manifest("dup")).unwrap_err();
        assert!(matches!(err, PluginError::DuplicateId(id) if id == "dup"));
    }

    #[test]
    fn unregister_removes_plugin_and_preferences() {
        let mut reg = PluginRegistry::new();
        reg.register(make_manifest("eps").provides_kind(NomKind::MediaImage))
            .unwrap();
        assert!(reg.unregister("eps"));
        assert!(reg.get("eps").is_none());
        assert!(reg.plugins_for_kind(NomKind::MediaImage).is_empty());
        assert!(!reg.unregister("eps"));
    }

    #[test]
    fn plugins_for_kind_empty_when_unknown() {
        let reg = PluginRegistry::new();
        assert!(reg.plugins_for_kind(NomKind::MediaVideo).is_empty());
    }

    #[test]
    fn plugins_for_kind_returns_all_registered() {
        let mut reg = PluginRegistry::new();
        reg.register(make_manifest("p1").provides_kind(NomKind::ScreenNative))
            .unwrap();
        reg.register(make_manifest("p2").provides_kind(NomKind::ScreenNative))
            .unwrap();
        let results = reg.plugins_for_kind(NomKind::ScreenNative);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn preferred_for_kind_follows_registration_order() {
        let mut reg = PluginRegistry::new();
        reg.register(make_manifest("first").provides_kind(NomKind::DataTransform))
            .unwrap();
        reg.register(make_manifest("second").provides_kind(NomKind::DataTransform))
            .unwrap();
        let preferred = reg.preferred_for_kind(NomKind::DataTransform).unwrap();
        assert_eq!(preferred.id, "first");
    }

    #[test]
    fn prefer_moves_to_head() {
        let mut reg = PluginRegistry::new();
        reg.register(make_manifest("x1").provides_kind(NomKind::ConceptDocument))
            .unwrap();
        reg.register(make_manifest("x2").provides_kind(NomKind::ConceptDocument))
            .unwrap();
        reg.prefer(NomKind::ConceptDocument, "x2").unwrap();
        let preferred = reg.preferred_for_kind(NomKind::ConceptDocument).unwrap();
        assert_eq!(preferred.id, "x2");
    }

    #[test]
    fn prefer_unknown_kind_returns_error() {
        let mut reg = PluginRegistry::new();
        let err = reg
            .prefer(NomKind::ScenarioWorkflow, "nobody")
            .unwrap_err();
        assert!(matches!(err, PluginError::UnknownKind(NomKind::ScenarioWorkflow)));
    }

    #[test]
    fn prefer_wrong_plugin_id_returns_kind_not_provided() {
        let mut reg = PluginRegistry::new();
        reg.register(make_manifest("correct").provides_kind(NomKind::Media3D))
            .unwrap();
        let err = reg.prefer(NomKind::Media3D, "wrong").unwrap_err();
        assert!(matches!(err, PluginError::KindNotProvided(id, NomKind::Media3D) if id == "wrong"));
    }

    #[test]
    fn len_and_is_empty_update_with_register_unregister() {
        let mut reg = PluginRegistry::new();
        assert_eq!(reg.len(), 0);
        assert!(reg.is_empty());
        reg.register(make_manifest("z")).unwrap();
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
        reg.unregister("z");
        assert_eq!(reg.len(), 0);
        assert!(reg.is_empty());
    }
}
