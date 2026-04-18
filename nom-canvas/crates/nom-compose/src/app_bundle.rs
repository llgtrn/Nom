//! App bundle types for cross-platform build output.

#[derive(Debug, Clone, PartialEq)]
pub enum BundleTarget {
    Windows,
    Linux,
    MacOs,
    Wasm,
    Android,
    Ios,
}

impl BundleTarget {
    pub fn extension(&self) -> &'static str {
        match self {
            BundleTarget::Windows => "exe",
            BundleTarget::Linux => "",
            BundleTarget::MacOs => "app",
            BundleTarget::Wasm => "wasm",
            BundleTarget::Android => "apk",
            BundleTarget::Ios => "ipa",
        }
    }

    pub fn is_mobile(&self) -> bool {
        matches!(self, BundleTarget::Android | BundleTarget::Ios)
    }
}

pub struct BundleManifest {
    pub app_name: String,
    pub version: String,
    pub targets: Vec<BundleTarget>,
    pub entry_hash: u64,
}

impl BundleManifest {
    pub fn has_target(&self, t: &BundleTarget) -> bool {
        self.targets.contains(t)
    }

    pub fn target_count(&self) -> usize {
        self.targets.len()
    }
}

pub struct BundleArtifact {
    pub target: BundleTarget,
    pub path: String,
    pub size_bytes: u64,
}

impl BundleArtifact {
    pub fn is_large(&self) -> bool {
        self.size_bytes > 10_000_000
    }

    pub fn filename(&self) -> String {
        let ext = self.target.extension();
        // Extract name and version from path: "dist/{name}-{version}.{ext}" or "dist/{name}-{version}"
        // Build from path by splitting on '/' and taking the last segment, stripping extension.
        let basename = self
            .path
            .split('/')
            .last()
            .unwrap_or(&self.path);
        // Strip existing extension suffix if present to get "name-version"
        let stem = if ext.is_empty() {
            basename.to_string()
        } else {
            let suffix = format!(".{}", ext);
            if basename.ends_with(&suffix) {
                basename[..basename.len() - suffix.len()].to_string()
            } else {
                basename.to_string()
            }
        };
        if ext.is_empty() {
            stem
        } else {
            format!("{}.{}", stem, ext)
        }
    }
}

pub struct BundleBuilder {
    pub manifest: BundleManifest,
}

impl BundleBuilder {
    pub fn new(manifest: BundleManifest) -> Self {
        Self { manifest }
    }

    pub fn build_for(&self, t: BundleTarget) -> BundleArtifact {
        let ext = t.extension();
        let path = if ext.is_empty() {
            format!("dist/{}-{}", self.manifest.app_name, self.manifest.version)
        } else {
            format!(
                "dist/{}-{}.{}",
                self.manifest.app_name, self.manifest.version, ext
            )
        };
        BundleArtifact {
            target: t,
            path,
            size_bytes: 1024 * 1024,
        }
    }

    pub fn build_all(&self) -> Vec<BundleArtifact> {
        self.manifest
            .targets
            .iter()
            .cloned()
            .map(|t| self.build_for(t))
            .collect()
    }
}

pub struct BundleOutput {
    pub artifacts: Vec<BundleArtifact>,
}

impl BundleOutput {
    pub fn total_size(&self) -> u64 {
        self.artifacts.iter().map(|a| a.size_bytes).sum()
    }

    pub fn for_target(&self, t: &BundleTarget) -> Option<&BundleArtifact> {
        self.artifacts.iter().find(|a| &a.target == t)
    }
}

#[cfg(test)]
mod app_bundle_tests {
    use super::*;

    #[test]
    fn target_extension() {
        assert_eq!(BundleTarget::Windows.extension(), "exe");
        assert_eq!(BundleTarget::Linux.extension(), "");
        assert_eq!(BundleTarget::MacOs.extension(), "app");
        assert_eq!(BundleTarget::Wasm.extension(), "wasm");
        assert_eq!(BundleTarget::Android.extension(), "apk");
        assert_eq!(BundleTarget::Ios.extension(), "ipa");
    }

    #[test]
    fn target_is_mobile() {
        assert!(BundleTarget::Android.is_mobile());
        assert!(BundleTarget::Ios.is_mobile());
        assert!(!BundleTarget::Windows.is_mobile());
        assert!(!BundleTarget::Linux.is_mobile());
        assert!(!BundleTarget::MacOs.is_mobile());
        assert!(!BundleTarget::Wasm.is_mobile());
    }

    #[test]
    fn manifest_has_target() {
        let manifest = BundleManifest {
            app_name: "myapp".to_string(),
            version: "1.0.0".to_string(),
            targets: vec![BundleTarget::Windows, BundleTarget::Wasm],
            entry_hash: 42,
        };
        assert!(manifest.has_target(&BundleTarget::Windows));
        assert!(manifest.has_target(&BundleTarget::Wasm));
        assert!(!manifest.has_target(&BundleTarget::Linux));
        assert!(!manifest.has_target(&BundleTarget::Ios));
    }

    #[test]
    fn artifact_is_large_false_at_1mb() {
        let artifact = BundleArtifact {
            target: BundleTarget::Windows,
            path: "dist/myapp-1.0.0.exe".to_string(),
            size_bytes: 1024 * 1024, // 1 MB
        };
        assert!(!artifact.is_large());
    }

    #[test]
    fn artifact_filename_with_ext() {
        let artifact = BundleArtifact {
            target: BundleTarget::Windows,
            path: "dist/myapp-1.0.0.exe".to_string(),
            size_bytes: 1024 * 1024,
        };
        assert_eq!(artifact.filename(), "myapp-1.0.0.exe");
    }

    #[test]
    fn artifact_filename_without_ext_linux() {
        let artifact = BundleArtifact {
            target: BundleTarget::Linux,
            path: "dist/myapp-1.0.0".to_string(),
            size_bytes: 1024 * 1024,
        };
        assert_eq!(artifact.filename(), "myapp-1.0.0");
    }

    #[test]
    fn builder_build_for_stub_size() {
        let manifest = BundleManifest {
            app_name: "myapp".to_string(),
            version: "1.0.0".to_string(),
            targets: vec![BundleTarget::Windows],
            entry_hash: 0,
        };
        let builder = BundleBuilder::new(manifest);
        let artifact = builder.build_for(BundleTarget::Windows);
        assert_eq!(artifact.size_bytes, 1024 * 1024);
        assert_eq!(artifact.path, "dist/myapp-1.0.0.exe");
    }

    #[test]
    fn builder_build_all_count_matches_targets() {
        let manifest = BundleManifest {
            app_name: "myapp".to_string(),
            version: "2.0.0".to_string(),
            targets: vec![BundleTarget::Windows, BundleTarget::Linux, BundleTarget::Wasm],
            entry_hash: 99,
        };
        let builder = BundleBuilder::new(manifest);
        let artifacts = builder.build_all();
        assert_eq!(artifacts.len(), 3);
    }

    #[test]
    fn output_total_size() {
        let artifacts = vec![
            BundleArtifact { target: BundleTarget::Windows, path: "dist/a.exe".to_string(), size_bytes: 500_000 },
            BundleArtifact { target: BundleTarget::Wasm, path: "dist/a.wasm".to_string(), size_bytes: 300_000 },
        ];
        let output = BundleOutput { artifacts };
        assert_eq!(output.total_size(), 800_000);
    }

    #[test]
    fn output_for_target_found() {
        let artifacts = vec![
            BundleArtifact { target: BundleTarget::Windows, path: "dist/a.exe".to_string(), size_bytes: 500_000 },
            BundleArtifact { target: BundleTarget::MacOs, path: "dist/a.app".to_string(), size_bytes: 700_000 },
        ];
        let output = BundleOutput { artifacts };
        let found = output.for_target(&BundleTarget::MacOs);
        assert!(found.is_some());
        assert_eq!(found.unwrap().size_bytes, 700_000);
        assert!(output.for_target(&BundleTarget::Linux).is_none());
    }
}
