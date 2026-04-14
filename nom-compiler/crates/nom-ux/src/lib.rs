//! `nom-ux` — UX as first-class dict unit per §5.11 / §5.11.6.
//!
//! UX entries describe screens, flows, interactions, and visual
//! patterns as dict entries. Per §4.4.6 their **bodies are `.bc`**
//! (compiled UI components from Dioxus / React / etc.); the
//! declarative kind/edge model on top of the bitcode bodies survives
//! as analysis metadata.
//!
//! This crate is the Phase-5 §5.11 scaffold. Functional extraction
//! (ingest React app → produce Screen + UserFlow entries) arrives
//! incrementally via per-ecosystem extractors
//! (`src/extractors/{react,vue,svelte,flutter,swiftui,jetpackcompose}.rs`
//! per §5.11.5).
//!
//! The first implemented surface is the §5.11.6 **platform-specialization
//! compile path**: given a screen hash and a target platform, resolve the
//! matching `ui_runtime_launch` specialization.

use thiserror::Error;

/// Target platform for a UX build, per §5.11.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Platform {
    Web,
    Desktop,
    Mobile,
}

impl Platform {
    /// Every platform in a stable order. Mirrors
    /// [`nom_media::Modality::ALL`] and [`nom_types::body_kind::ALL`]
    /// for consistent enumeration across the scaffolded crates.
    pub const ALL: &'static [Platform] = &[Platform::Web, Platform::Desktop, Platform::Mobile];

    /// The word name of the `ui_runtime_launch` specialization for
    /// this platform. The resolver walks `Specializes` edges from the
    /// abstract `ui_runtime_launch` to the concrete variant matching
    /// this word.
    pub const fn runtime_launch_word(self) -> &'static str {
        match self {
            Platform::Web => "ui_runtime_launch_web",
            Platform::Desktop => "ui_runtime_launch_desktop",
            Platform::Mobile => "ui_runtime_launch_mobile",
        }
    }

    /// Primary artifact-path extension for this target.
    /// `web → .wasm`, `desktop → .exe` (Windows) / platform native,
    /// `mobile → .apk`/`.ipa`.
    pub fn artifact_extension(self) -> &'static str {
        match self {
            Platform::Web => "wasm",
            Platform::Desktop => {
                if cfg!(target_os = "windows") {
                    "exe"
                } else if cfg!(target_os = "macos") {
                    "app"
                } else {
                    ""
                }
            }
            Platform::Mobile => "apk", // Android default; iOS override at build time.
        }
    }
}

/// Parse `--target <web|desktop|mobile>` flag values. Case-insensitive.
pub fn platform_from_str(s: &str) -> Option<Platform> {
    match s.to_ascii_lowercase().as_str() {
        "web" => Some(Platform::Web),
        "desktop" | "native" => Some(Platform::Desktop),
        "mobile" | "android" | "ios" => Some(Platform::Mobile),
        _ => None,
    }
}

/// Errors produced by `nom-ux`. Minimal until real extractor work
/// starts — each extractor PR grows this enum as needed.
#[derive(Debug, Error)]
pub enum UxError {
    #[error("unknown target platform: {0} (expected web|desktop|mobile)")]
    UnknownPlatform(String),
    #[error("no {0} specialization in dict (walk Specializes from ui_runtime_launch)")]
    MissingSpecialization(String),
    #[error("capability unavailable on platform: {cap} on {platform:?} (NOM-U02)")]
    UnavailableCapability { cap: String, platform: Platform },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_maps_to_runtime_launch_word() {
        assert_eq!(Platform::Web.runtime_launch_word(), "ui_runtime_launch_web");
        assert_eq!(
            Platform::Desktop.runtime_launch_word(),
            "ui_runtime_launch_desktop"
        );
        assert_eq!(
            Platform::Mobile.runtime_launch_word(),
            "ui_runtime_launch_mobile"
        );
    }

    #[test]
    fn platform_from_str_case_insensitive() {
        assert_eq!(platform_from_str("web"), Some(Platform::Web));
        assert_eq!(platform_from_str("WEB"), Some(Platform::Web));
        assert_eq!(platform_from_str("desktop"), Some(Platform::Desktop));
        assert_eq!(platform_from_str("native"), Some(Platform::Desktop));
        assert_eq!(platform_from_str("ios"), Some(Platform::Mobile));
        assert_eq!(platform_from_str("android"), Some(Platform::Mobile));
        assert_eq!(platform_from_str("watchos"), None);
    }

    #[test]
    fn web_artifact_is_wasm_regardless_of_host() {
        assert_eq!(Platform::Web.artifact_extension(), "wasm");
        assert_eq!(Platform::Mobile.artifact_extension(), "apk");
    }

    #[test]
    fn platform_all_covers_every_variant() {
        for p in Platform::ALL {
            // Exhaustive-match sentinel: a new Platform variant
            // breaks this match at compile time until ALL is
            // updated. Matches iter 24's pattern in nom-media.
            let _: () = match p {
                Platform::Web | Platform::Desktop | Platform::Mobile => (),
            };
        }
        assert_eq!(Platform::ALL.len(), 3);
    }
}
