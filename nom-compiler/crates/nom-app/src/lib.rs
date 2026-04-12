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

/// A compiled-app output aspect.
///
/// Compiling an `AppManifest` never produces a single "god file".
/// Instead, each aspect of the app is serialized to its own artifact,
/// mirroring the 26-peer-crate discipline of the compiler itself.
/// Keeping aspects in separate files means each concern (security,
/// UX, env, business logic, benchmarks, …) can be audited, swapped,
/// hashed, and cached independently.
///
/// Aspect → default file-stem mapping is given by
/// [`OutputAspect::file_stem`]. The file extension is picked by
/// [`OutputAspect::extension`] (mostly `.json`; the core executable
/// takes `.bin` / `.wasm` / `.exe` / `.apk` per target platform).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum OutputAspect {
    /// Core executable (bitcode-linked closure → platform binary).
    Core,
    /// Authorization + security policy: capabilities, sandbox, secrets.
    Security,
    /// Screens, pages, user-flows, design-rule bindings.
    Ux,
    /// Runtime environment: OS target, arch, DB engine, env vars.
    Env,
    /// Business-logic rules: contracts, validations, invariants.
    BizLogic,
    /// Benchmark criteria: perf budgets, regression gates.
    Bench,
    /// Request/response schemas: API endpoints, payload shapes.
    Response,
    /// Flow artifacts: recorded user-journey + middleware tape.
    Flow,
    /// Optimization directives: inline hints, specialization budget.
    Optimize,
    /// Acceptance criteria: success predicates, test oracles.
    Criteria,
}

impl OutputAspect {
    /// Every aspect in declaration order. Use for fan-out iteration.
    pub const ALL: &'static [OutputAspect] = &[
        OutputAspect::Core,
        OutputAspect::Security,
        OutputAspect::Ux,
        OutputAspect::Env,
        OutputAspect::BizLogic,
        OutputAspect::Bench,
        OutputAspect::Response,
        OutputAspect::Flow,
        OutputAspect::Optimize,
        OutputAspect::Criteria,
    ];

    /// File stem (no extension). Combined with [`extension`] gives the
    /// default path under the app's output directory.
    pub fn file_stem(self) -> &'static str {
        match self {
            OutputAspect::Core => "app",
            OutputAspect::Security => "app.security",
            OutputAspect::Ux => "app.ux",
            OutputAspect::Env => "app.env",
            OutputAspect::BizLogic => "app.bizlogic",
            OutputAspect::Bench => "app.bench",
            OutputAspect::Response => "app.response",
            OutputAspect::Flow => "app.flow",
            OutputAspect::Optimize => "app.optimize",
            OutputAspect::Criteria => "app.criteria",
        }
    }

    /// Extension for this aspect. Core defers to the target platform;
    /// all other aspects are JSON manifests today.
    pub fn extension(self, target: Option<nom_ux::Platform>) -> &'static str {
        match self {
            OutputAspect::Core => match target {
                Some(nom_ux::Platform::Web) => "wasm",
                Some(nom_ux::Platform::Mobile) => "apk",
                Some(nom_ux::Platform::Desktop) | None => "bin",
            },
            _ => "json",
        }
    }

    /// Default relative path for this aspect under the app output dir.
    pub fn default_path(self, target: Option<nom_ux::Platform>) -> String {
        format!("{}.{}", self.file_stem(), self.extension(target))
    }
}

/// One artifact emitted by `compile_app_to_artifacts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub aspect: OutputAspect,
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Compile an app manifest into a fan-out of per-aspect artifacts.
///
/// Scaffold only: returns one empty `Artifact` per aspect at its
/// default path. Real per-aspect population lands incrementally as
/// each aspect's querying + serialization is implemented (Core pulls
/// the bc closure; Security reads concept membership for "security"
/// + kind=Concept entries; Ux serializes Screen/Page/UserFlow; etc.).
pub fn compile_app_to_artifacts(manifest: &AppManifest) -> Result<Vec<Artifact>, AppError> {
    let target = manifest.default_target_platform();
    Ok(OutputAspect::ALL
        .iter()
        .map(|&aspect| Artifact {
            aspect,
            path: aspect.default_path(target),
            bytes: Vec::new(),
        })
        .collect())
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
    fn output_aspect_all_has_ten_variants() {
        assert_eq!(OutputAspect::ALL.len(), 10);
    }

    #[test]
    fn output_aspect_extension_picks_platform_binary() {
        use nom_ux::Platform;
        assert_eq!(OutputAspect::Core.extension(Some(Platform::Web)), "wasm");
        assert_eq!(OutputAspect::Core.extension(Some(Platform::Mobile)), "apk");
        assert_eq!(OutputAspect::Core.extension(Some(Platform::Desktop)), "bin");
        assert_eq!(OutputAspect::Core.extension(None), "bin");
        assert_eq!(OutputAspect::Security.extension(Some(Platform::Web)), "json");
    }

    #[test]
    fn compile_emits_one_artifact_per_aspect() {
        let m = AppManifest {
            manifest_hash: "h".into(),
            name: "n".into(),
            default_target: "web".into(),
            root_page_hash: "p".into(),
            data_sources: vec![],
            actions: vec![],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        };
        let artifacts = compile_app_to_artifacts(&m).unwrap();
        assert_eq!(artifacts.len(), OutputAspect::ALL.len());
        let core = artifacts.iter().find(|a| a.aspect == OutputAspect::Core).unwrap();
        assert_eq!(core.path, "app.wasm");
        let sec = artifacts.iter().find(|a| a.aspect == OutputAspect::Security).unwrap();
        assert_eq!(sec.path, "app.security.json");
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
