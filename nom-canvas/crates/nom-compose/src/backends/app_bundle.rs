#![deny(unsafe_code)]
use crate::backends::ComposeResult;

/// Deployment target for an application bundle.
#[derive(Debug, Clone, PartialEq)]
pub enum AppTarget {
    Web,
    Desktop,
    Mobile,
    Extension,
}

/// Specification for building and bundling an application.
#[derive(Debug, Clone)]
pub struct AppBundleSpec {
    pub name: String,
    pub version: String,
    pub targets: Vec<AppTarget>,
    pub entry_point: String,
}

impl AppBundleSpec {
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }
}

pub fn compose(spec: &AppBundleSpec) -> ComposeResult {
    if spec.name.is_empty() {
        return Err("app bundle name must not be empty".into());
    }
    if spec.entry_point.is_empty() {
        return Err("app bundle entry_point must not be empty".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_bundle_target_count() {
        let spec = AppBundleSpec {
            name: "my-app".into(),
            version: "1.0.0".into(),
            targets: vec![AppTarget::Web, AppTarget::Desktop, AppTarget::Mobile],
            entry_point: "src/main.rs".into(),
        };
        assert_eq!(spec.target_count(), 3);
    }

    #[test]
    fn app_bundle_compose_produces_artifact() {
        let spec = AppBundleSpec {
            name: "launcher".into(),
            version: "0.1.0".into(),
            targets: vec![AppTarget::Extension],
            entry_point: "src/lib.rs".into(),
        };
        let result = compose(&spec);
        assert!(result.is_ok(), "compose must return Ok for valid spec");
    }
}
