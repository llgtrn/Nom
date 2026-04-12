//! `nom-app` — app-composition kinds per §5.12.
//!
//! An app is a hash closure rooted at an `AppManifest`. The manifest
//! names entry points, data sources, pages, and configuration; the
//! closure walk from that hash pulls in all code and media the app
//! needs. `nom app build <manifest_hash>` materializes the whole
//! thing per-target platform (via `nom-ux` specialization edges per
//! §5.11.6 for UX, and codec/container closures per §5.16.12 for
//! embedded media).
//!
//! This crate is the Phase-5 §5.12 scaffold. Actual manifest parsing
//! + ingestion of real apps arrives incrementally; the kinds and
//! builder shapes below define the surface.

use thiserror::Error;

/// Composition kind tags for app-layer entries.
///
/// Each constant is the canonical `EntryKind::as_str()` value for its
/// variant — single source of truth lives in [`nom_types::EntryKind`]
/// (iter 16 landed the promotion). This module exists so app-layer
/// code can write `app_kind::APP_MANIFEST` instead of
/// `EntryKind::AppManifest.as_str()`, keeping call sites short.
pub mod app_kind {
    use nom_types::EntryKind;

    pub const APP_MANIFEST: &str = EntryKind::AppManifest.as_str();
    pub const DATA_SOURCE: &str = EntryKind::DataSource.as_str();
    pub const QUERY: &str = EntryKind::Query.as_str();
    pub const APP_ACTION: &str = EntryKind::AppAction.as_str();
    pub const APP_VARIABLE: &str = EntryKind::AppVariable.as_str();
    pub const PAGE: &str = EntryKind::Page.as_str();

    /// Returns true if `s` matches one of the six app-layer
    /// [`EntryKind`] string tags. Delegates to
    /// [`EntryKind::from_str`] so new EntryKind additions don't need
    /// a parallel match here.
    pub fn is_known(s: &str) -> bool {
        matches!(
            EntryKind::from_str(s),
            EntryKind::AppManifest
                | EntryKind::DataSource
                | EntryKind::Query
                | EntryKind::AppAction
                | EntryKind::AppVariable
                | EntryKind::Page
        )
    }
}

/// An app-manifest entry. Its body is the serialized manifest (JSON
/// today; canonical textual form later). Referenced entries become
/// the app's closure.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppManifest {
    /// Entry hash of this manifest. Canonical app id.
    pub manifest_hash: String,
    /// Human-readable name. Not identity — identity is the hash.
    pub name: String,
    /// Default target platform for `nom app build` when no flag given.
    /// Stored as a string for stable JSON serialization; use
    /// [`AppManifest::default_target_platform`] for the typed form.
    pub default_target: String,
    /// Hash of the root page entry.
    pub root_page_hash: String,
    /// Hashes of data-source entries, in declaration order.
    pub data_sources: Vec<String>,
    /// Hashes of action entries the app can invoke.
    pub actions: Vec<String>,
    /// Hashes of media entries (icons, fonts, sounds) to bundle.
    pub media_assets: Vec<String>,
    /// Free-form settings (env vars, feature flags, policy tags).
    pub settings: serde_json::Value,
}

impl AppManifest {
    /// Parse `default_target` into a typed [`nom_ux::Platform`].
    /// Returns `None` if the string isn't a recognized platform tag.
    /// Call sites should prefer this over raw-string matching.
    pub fn default_target_platform(&self) -> Option<nom_ux::Platform> {
        nom_ux::platform_from_str(&self.default_target)
    }
}

/// Errors produced by `nom-app`.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("manifest references missing entry: {0}")]
    MissingReference(String),
    #[error("manifest parse failed: {0}")]
    ParseFailed(String),
    #[error("target platform not supported by this manifest: {0}")]
    UnsupportedTarget(String),
    #[error("builder not yet implemented for target: {0}")]
    BuilderNotYetImplemented(String),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_kind_is_known_recognizes_all_variants() {
        for k in [
            app_kind::APP_MANIFEST,
            app_kind::DATA_SOURCE,
            app_kind::QUERY,
            app_kind::APP_ACTION,
            app_kind::APP_VARIABLE,
            app_kind::PAGE,
        ] {
            assert!(app_kind::is_known(k));
        }
        assert!(!app_kind::is_known("not_an_app_kind"));
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let m = AppManifest {
            manifest_hash: "m_abc".into(),
            name: "todo_list_app".into(),
            default_target: "web".into(),
            root_page_hash: "p_home".into(),
            data_sources: vec!["ds_todos".into()],
            actions: vec!["a_add".into(), "a_delete".into()],
            media_assets: vec!["icon_checkbox".into()],
            settings: serde_json::json!({"theme":"dark"}),
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: AppManifest = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn default_target_parses_to_platform() {
        let mut m = AppManifest {
            manifest_hash: "h".into(),
            name: "n".into(),
            default_target: "web".into(),
            root_page_hash: "p".into(),
            data_sources: vec![],
            actions: vec![],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        };
        assert_eq!(m.default_target_platform(), Some(nom_ux::Platform::Web));
        m.default_target = "desktop".into();
        assert_eq!(m.default_target_platform(), Some(nom_ux::Platform::Desktop));
        m.default_target = "garbage".into();
        assert_eq!(m.default_target_platform(), None);
    }
}
