//! Configuration loading for the Nom compiler workspace.
//!
//! Reads workspace and donor configuration from TOML files.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

pub const DEFAULT_INVENTORY_PATH: &str = "data/reports/inventory.json";

#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    pub name: String,
    pub root: PathBuf,
    pub default_inventory_path: PathBuf,
    pub default_languages: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub donors: Vec<DonorConfig>,
}

#[derive(Debug, Clone)]
pub struct DonorConfig {
    pub name: String,
    pub kind: String,
    pub path: PathBuf,
    pub priority: u32,
    pub roles: Vec<String>,
    pub exclude: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceConfigFile {
    project: ProjectSection,
    scan: Option<ScanSection>,
    donors: Vec<DonorReference>,
}

#[derive(Debug, Deserialize)]
struct ProjectSection {
    name: String,
    inventory_output: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ScanSection {
    default_languages: Option<Vec<String>>,
    ignore: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct DonorReference {
    path: String,
}

#[derive(Debug, Deserialize)]
struct DonorFile {
    name: String,
    kind: String,
    path: String,
    priority: u32,
    roles: Vec<String>,
    exclude: Option<Vec<String>>,
    notes: Option<Vec<String>>,
}

pub fn load_workspace_config(root: &Path) -> Result<WorkspaceConfig> {
    let config_path = root.join("config").join("nom.toml");
    let raw = fs::read_to_string(&config_path).with_context(|| {
        format!(
            "failed to read workspace config at {}",
            config_path.display()
        )
    })?;
    let parsed: WorkspaceConfigFile = toml::from_str(&raw).with_context(|| {
        format!(
            "failed to parse workspace config at {}",
            config_path.display()
        )
    })?;

    let mut donors = Vec::new();
    for donor_ref in parsed.donors {
        let donor_path = root.join(donor_ref.path);
        let donor = load_donor_file(root, &donor_path)?;
        donors.push(donor);
    }

    if donors.is_empty() {
        bail!("workspace config does not define any donors");
    }

    let default_inventory_path = parsed
        .project
        .inventory_output
        .as_deref()
        .map(|path| root.join(path))
        .unwrap_or_else(|| root.join(DEFAULT_INVENTORY_PATH));

    let scan = parsed.scan.unwrap_or(ScanSection {
        default_languages: None,
        ignore: None,
    });

    Ok(WorkspaceConfig {
        name: parsed.project.name,
        root: root.to_path_buf(),
        default_inventory_path,
        default_languages: scan.default_languages.unwrap_or_default(),
        ignore_patterns: scan.ignore.unwrap_or_default(),
        donors,
    })
}

fn load_donor_file(root: &Path, donor_file_path: &Path) -> Result<DonorConfig> {
    let raw = fs::read_to_string(donor_file_path).with_context(|| {
        format!(
            "failed to read donor config at {}",
            donor_file_path.display()
        )
    })?;
    let parsed: DonorFile = toml::from_str(&raw).with_context(|| {
        format!(
            "failed to parse donor config at {}",
            donor_file_path.display()
        )
    })?;

    let donor_path = PathBuf::from(&parsed.path);
    let donor_path = if donor_path.is_absolute() {
        donor_path
    } else {
        root.join(donor_path)
    };

    Ok(DonorConfig {
        name: parsed.name,
        kind: parsed.kind,
        path: donor_path,
        priority: parsed.priority,
        roles: parsed.roles,
        exclude: parsed.exclude.unwrap_or_default(),
        notes: parsed.notes.unwrap_or_default(),
    })
}
