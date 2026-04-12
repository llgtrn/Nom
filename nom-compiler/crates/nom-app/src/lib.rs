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
/// Scaffold form — returns one empty `Artifact` per aspect. Use
/// [`compile_app_to_artifacts_with_dict`] when a [`nom_dict::NomDict`]
/// is available; that path populates aspects from real dictionary
/// state (closure walk, security findings, etc.).
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

/// The manifest's top-level closure roots: everything directly named
/// by the manifest that should be included in the app's hash closure.
/// Matches §5.12: root page + data sources + actions + media assets.
fn manifest_roots(manifest: &AppManifest) -> Vec<String> {
    let mut roots = Vec::new();
    if !manifest.root_page_hash.is_empty() {
        roots.push(manifest.root_page_hash.clone());
    }
    roots.extend(manifest.data_sources.iter().cloned());
    roots.extend(manifest.actions.iter().cloned());
    roots.extend(manifest.media_assets.iter().cloned());
    roots
}

/// Compile with real dictionary access — populates aspects that have
/// a query implementation today. Aspects without a populator yet get
/// the scaffold (empty bytes at their default path).
///
/// Populated today:
/// - **Security**: walks the manifest closure, reads
///   `entry_security_findings` for each closure member, emits one JSON
///   object `{app, findings: [{entry_id, severity, category, rule_id,
///   message, line}]}` ordered by (severity DESC, entry_id ASC).
pub fn compile_app_to_artifacts_with_dict(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<Artifact>, AppError> {
    let target = manifest.default_target_platform();
    let mut out = Vec::with_capacity(OutputAspect::ALL.len());
    for &aspect in OutputAspect::ALL {
        let bytes = match aspect {
            OutputAspect::Security => build_security_aspect(manifest, dict)?,
            OutputAspect::Ux => build_ux_aspect(manifest, dict)?,
            OutputAspect::Env => build_env_aspect(manifest, dict)?,
            OutputAspect::BizLogic => build_bizlogic_aspect(manifest, dict)?,
            OutputAspect::Bench => build_bench_aspect(manifest, dict)?,
            OutputAspect::Response => build_response_aspect(manifest, dict)?,
            OutputAspect::Flow => build_flow_aspect(manifest, dict)?,
            OutputAspect::Criteria => build_criteria_aspect(manifest, dict)?,
            OutputAspect::Optimize => build_optimize_aspect(manifest, dict)?,
            OutputAspect::Core => Vec::new(),
        };
        out.push(Artifact {
            aspect,
            path: aspect.default_path(target),
            bytes,
        });
    }
    Ok(out)
}

fn build_security_aspect(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<u8>, AppError> {
    use std::collections::BTreeSet;

    let mut closure: BTreeSet<String> = BTreeSet::new();
    for root in manifest_roots(manifest) {
        match dict.closure(&root) {
            Ok(ids) => closure.extend(ids),
            Err(_) => {
                // Treat missing-root as not-in-dict; the app build is
                // still useful for its other aspects.
                closure.insert(root);
            }
        }
    }

    #[derive(serde::Serialize)]
    struct AspectFinding<'a> {
        entry_id: &'a str,
        severity: String,
        category: &'a str,
        rule_id: Option<&'a str>,
        message: Option<&'a str>,
        line: Option<i64>,
    }

    let mut all: Vec<nom_types::SecurityFinding> = Vec::new();
    for id in &closure {
        match dict.get_findings(id) {
            Ok(mut fs) => all.append(&mut fs),
            Err(_) => {}
        }
    }
    all.sort_by(|a, b| {
        let sa = severity_rank(&a.severity);
        let sb = severity_rank(&b.severity);
        sb.cmp(&sa).then_with(|| a.id.cmp(&b.id))
    });

    let findings: Vec<AspectFinding> = all
        .iter()
        .map(|f| AspectFinding {
            entry_id: &f.id,
            severity: format!("{:?}", f.severity).to_lowercase(),
            category: &f.category,
            rule_id: f.rule_id.as_deref(),
            message: f.message.as_deref(),
            line: f.line,
        })
        .collect();

    let doc = serde_json::json!({
        "app": manifest.name,
        "manifest_hash": manifest.manifest_hash,
        "closure_size": closure.len(),
        "findings": findings,
    });
    Ok(serde_json::to_vec_pretty(&doc)?)
}

fn build_ux_aspect(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<u8>, AppError> {
    use std::collections::BTreeSet;

    let mut closure: BTreeSet<String> = BTreeSet::new();
    for root in manifest_roots(manifest) {
        if let Ok(ids) = dict.closure(&root) {
            closure.extend(ids);
        }
    }

    #[derive(serde::Serialize)]
    struct UxEntry<'a> {
        entry_id: &'a str,
        kind: &'a str,
        word: &'a str,
        describe: Option<&'a str>,
    }

    let mut screens: Vec<UxEntry> = Vec::new();
    let mut pages: Vec<UxEntry> = Vec::new();
    let mut flows: Vec<UxEntry> = Vec::new();
    let mut patterns: Vec<UxEntry> = Vec::new();

    let mut entries: Vec<nom_types::Entry> = Vec::new();
    for id in &closure {
        if let Ok(Some(e)) = dict.get_entry(id) {
            entries.push(e);
        }
    }

    for e in &entries {
        let row = UxEntry {
            entry_id: &e.id,
            kind: e.kind.as_str(),
            word: &e.word,
            describe: e.describe.as_deref(),
        };
        match e.kind {
            nom_types::EntryKind::Screen => screens.push(row),
            nom_types::EntryKind::Page => pages.push(row),
            nom_types::EntryKind::UserFlow => flows.push(row),
            nom_types::EntryKind::UxPattern => patterns.push(row),
            _ => {}
        }
    }

    let doc = serde_json::json!({
        "app": manifest.name,
        "manifest_hash": manifest.manifest_hash,
        "target": manifest.default_target,
        "root_page": manifest.root_page_hash,
        "screens": screens,
        "pages": pages,
        "user_flows": flows,
        "ux_patterns": patterns,
    });
    Ok(serde_json::to_vec_pretty(&doc)?)
}

/// Collect the full closure + fetch every entry. Shared helper for
/// aspect populators that need more than just ids.
fn closure_entries(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Vec<nom_types::Entry> {
    use std::collections::BTreeSet;
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for root in manifest_roots(manifest) {
        if let Ok(c) = dict.closure(&root) {
            ids.extend(c);
        }
    }
    let mut out = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Ok(Some(e)) = dict.get_entry(id) {
            out.push(e);
        }
    }
    out
}

fn build_env_aspect(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<u8>, AppError> {
    let entries = closure_entries(manifest, dict);
    let data_sources: Vec<&nom_types::Entry> = entries
        .iter()
        .filter(|e| e.kind == nom_types::EntryKind::DataSource)
        .collect();
    let app_vars: Vec<&nom_types::Entry> = entries
        .iter()
        .filter(|e| e.kind == nom_types::EntryKind::AppVariable)
        .collect();
    #[derive(serde::Serialize)]
    struct Row<'a> {
        entry_id: &'a str,
        word: &'a str,
        describe: Option<&'a str>,
    }
    let ds: Vec<Row> = data_sources
        .iter()
        .map(|e| Row {
            entry_id: &e.id,
            word: &e.word,
            describe: e.describe.as_deref(),
        })
        .collect();
    let vars: Vec<Row> = app_vars
        .iter()
        .map(|e| Row {
            entry_id: &e.id,
            word: &e.word,
            describe: e.describe.as_deref(),
        })
        .collect();
    let doc = serde_json::json!({
        "app": manifest.name,
        "manifest_hash": manifest.manifest_hash,
        "target_platform": manifest.default_target,
        "data_sources": ds,
        "app_variables": vars,
        "settings": manifest.settings,
    });
    Ok(serde_json::to_vec_pretty(&doc)?)
}

fn build_bizlogic_aspect(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<u8>, AppError> {
    let entries = closure_entries(manifest, dict);
    #[derive(serde::Serialize)]
    struct Rule<'a> {
        entry_id: &'a str,
        word: &'a str,
        kind: &'a str,
        input_type: Option<&'a str>,
        output_type: Option<&'a str>,
        pre: Option<&'a str>,
        post: Option<&'a str>,
    }
    let rules: Vec<Rule> = entries
        .iter()
        .filter(|e| {
            e.contract.pre.is_some()
                || e.contract.post.is_some()
                || e.contract.input_type.is_some()
                || e.contract.output_type.is_some()
        })
        .map(|e| Rule {
            entry_id: &e.id,
            word: &e.word,
            kind: e.kind.as_str(),
            input_type: e.contract.input_type.as_deref(),
            output_type: e.contract.output_type.as_deref(),
            pre: e.contract.pre.as_deref(),
            post: e.contract.post.as_deref(),
        })
        .collect();
    let doc = serde_json::json!({
        "app": manifest.name,
        "manifest_hash": manifest.manifest_hash,
        "rules": rules,
    });
    Ok(serde_json::to_vec_pretty(&doc)?)
}

fn build_bench_aspect(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<u8>, AppError> {
    let entries = closure_entries(manifest, dict);
    #[derive(serde::Serialize)]
    struct Row<'a> {
        entry_id: &'a str,
        word: &'a str,
        describe: Option<&'a str>,
    }
    let runs: Vec<Row> = entries
        .iter()
        .filter(|e| e.kind == nom_types::EntryKind::BenchmarkRun)
        .map(|e| Row {
            entry_id: &e.id,
            word: &e.word,
            describe: e.describe.as_deref(),
        })
        .collect();
    let doc = serde_json::json!({
        "app": manifest.name,
        "manifest_hash": manifest.manifest_hash,
        "benchmark_runs": runs,
    });
    Ok(serde_json::to_vec_pretty(&doc)?)
}

fn build_response_aspect(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<u8>, AppError> {
    let entries = closure_entries(manifest, dict);
    #[derive(serde::Serialize)]
    struct Row<'a> {
        entry_id: &'a str,
        word: &'a str,
        kind: &'a str,
        input_type: Option<&'a str>,
        output_type: Option<&'a str>,
    }
    let endpoints: Vec<Row> = entries
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                nom_types::EntryKind::ApiEndpoint | nom_types::EntryKind::Schema
            )
        })
        .map(|e| Row {
            entry_id: &e.id,
            word: &e.word,
            kind: e.kind.as_str(),
            input_type: e.contract.input_type.as_deref(),
            output_type: e.contract.output_type.as_deref(),
        })
        .collect();
    let doc = serde_json::json!({
        "app": manifest.name,
        "manifest_hash": manifest.manifest_hash,
        "endpoints": endpoints,
    });
    Ok(serde_json::to_vec_pretty(&doc)?)
}

fn build_flow_aspect(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<u8>, AppError> {
    let entries = closure_entries(manifest, dict);
    #[derive(serde::Serialize)]
    struct Row<'a> {
        entry_id: &'a str,
        word: &'a str,
        describe: Option<&'a str>,
    }
    let flows: Vec<Row> = entries
        .iter()
        .filter(|e| e.kind == nom_types::EntryKind::FlowArtifact)
        .map(|e| Row {
            entry_id: &e.id,
            word: &e.word,
            describe: e.describe.as_deref(),
        })
        .collect();
    let doc = serde_json::json!({
        "app": manifest.name,
        "manifest_hash": manifest.manifest_hash,
        "flow_artifacts": flows,
    });
    Ok(serde_json::to_vec_pretty(&doc)?)
}

fn build_criteria_aspect(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<u8>, AppError> {
    let entries = closure_entries(manifest, dict);
    let total = entries.len();
    let complete = entries
        .iter()
        .filter(|e| e.status == nom_types::EntryStatus::Complete)
        .count();
    let partial = entries
        .iter()
        .filter(|e| e.status == nom_types::EntryStatus::Partial)
        .count();
    #[derive(serde::Serialize)]
    struct TestRow<'a> {
        entry_id: &'a str,
        word: &'a str,
    }
    let tests: Vec<TestRow> = entries
        .iter()
        .filter(|e| e.kind == nom_types::EntryKind::TestCase)
        .map(|e| TestRow {
            entry_id: &e.id,
            word: &e.word,
        })
        .collect();
    let report = dream_report(manifest, dict);
    let doc = serde_json::json!({
        "app": manifest.name,
        "manifest_hash": manifest.manifest_hash,
        "closure_size": total,
        "complete": complete,
        "partial": partial,
        "test_cases": tests,
        "app_score": report.app_score,
        "score_threshold": report.score_threshold,
        "is_epic": report.is_epic,
        "proposals": report.proposals,
        "next_instruction": report.next_instruction,
    });
    Ok(serde_json::to_vec_pretty(&doc)?)
}

/// A machine-authorable gap the LLM should fill. Each proposal is
/// deterministic from the current dict state — rerunning with the
/// same inputs yields the same proposals in the same order. Consumed
/// by `nom app dream` and the MCP `criteria_proposals` tool.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Proposal {
    /// Stable tag for grouping (`partial_entry`, `missing_root`,
    /// `unbalanced_contract`, `empty_closure`, `no_tests`).
    pub kind: String,
    /// One-line why-it-matters; gets shown to the LLM.
    pub rationale: String,
    /// Entry id or manifest-reference the proposal relates to, if any.
    pub target: Option<String>,
    /// Suggested EntryKind for any new nomtu the LLM should author.
    pub suggested_entry_kind: Option<String>,
    /// Suggested `word` (syntax token) for the new nomtu.
    pub suggested_word: Option<String>,
    /// Suggested concept membership for the new nomtu.
    pub suggested_concept: Option<String>,
    /// Pre-computed dict hints the LLM can use without further
    /// queries: nomtu whose `word` or `describe` matches the
    /// suggested_word. Empty when no hint was derivable. Populated
    /// only via `dream_report_with_hints`; `criteria_proposals`
    /// leaves it empty to keep the hot path cheap.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dict_hints: Vec<DictHint>,
}

/// One pre-computed hint: an existing dict entry the LLM should
/// consider either using directly (`use <word>@<id>`) or cloning as
/// a starting point for the new nomtu. Scored by match quality
/// (word prefix > word substring > describe substring).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DictHint {
    pub entry_id: String,
    pub word: String,
    pub kind: String,
    pub describe: Option<String>,
    pub match_score: u8,
}

/// Collect proposals in a deterministic order. Sort is:
/// (kind ASC, target ASC, suggested_word ASC).
fn collect_proposals(
    manifest: &AppManifest,
    entries: &[nom_types::Entry],
    dict: &nom_dict::NomDict,
) -> Vec<Proposal> {
    let mut out: Vec<Proposal> = Vec::new();

    // Empty closure — app is a shell.
    if entries.is_empty() {
        out.push(Proposal {
            kind: "empty_closure".into(),
            rationale: "manifest closure is empty — no code will be linked".into(),
            target: Some(manifest.manifest_hash.clone()),
            suggested_entry_kind: Some("page".into()),
            suggested_word: Some(format!("{}_home", manifest.name)),
            suggested_concept: Some(manifest.name.clone()),
            dict_hints: Vec::new(),
        });
    }

    // Manifest roots that don't resolve in the dict.
    for root in manifest_roots(manifest) {
        if dict.get_entry(&root).ok().flatten().is_none() {
            out.push(Proposal {
                kind: "missing_root".into(),
                rationale: format!(
                    "manifest names entry `{root}` but no such entry exists in the dict"
                ),
                target: Some(root.clone()),
                suggested_entry_kind: Some("function".into()),
                suggested_word: None,
                suggested_concept: Some(manifest.name.clone()),
                dict_hints: Vec::new(),
            });
        }
    }

    // Partial entries — body may be incomplete or unverified.
    for e in entries {
        if e.status == nom_types::EntryStatus::Partial {
            out.push(Proposal {
                kind: "partial_entry".into(),
                rationale: format!(
                    "`{}` ({}) is Partial — contract or body incomplete; lift to Complete",
                    e.word,
                    e.kind.as_str()
                ),
                target: Some(e.id.clone()),
                suggested_entry_kind: Some(e.kind.as_str().to_string()),
                suggested_word: Some(e.word.clone()),
                suggested_concept: e.concept.clone(),
                dict_hints: Vec::new(),
            });
        }
    }

    // Contracts with pre but no post (or vice-versa) — suspicious shape.
    for e in entries {
        let has_pre = e.contract.pre.is_some();
        let has_post = e.contract.post.is_some();
        if has_pre ^ has_post {
            out.push(Proposal {
                kind: "unbalanced_contract".into(),
                rationale: format!(
                    "`{}` has {} without {}; add the missing clause or both",
                    e.word,
                    if has_pre { "pre" } else { "post" },
                    if has_pre { "post" } else { "pre" },
                ),
                target: Some(e.id.clone()),
                suggested_entry_kind: Some(e.kind.as_str().to_string()),
                suggested_word: Some(e.word.clone()),
                suggested_concept: e.concept.clone(),
                dict_hints: Vec::new(),
            });
        }
    }

    // No test-cases in closure — criteria can't verify.
    let has_tests = entries
        .iter()
        .any(|e| e.kind == nom_types::EntryKind::TestCase);
    if !has_tests && !entries.is_empty() {
        out.push(Proposal {
            kind: "no_tests".into(),
            rationale: "closure has no TestCase entries — criteria cannot verify".into(),
            target: Some(manifest.manifest_hash.clone()),
            suggested_entry_kind: Some("test_case".into()),
            suggested_word: Some(format!("test_{}_smoke", manifest.name)),
            suggested_concept: Some(manifest.name.clone()),
            dict_hints: Vec::new(),
        });
    }

    out.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.target.cmp(&b.target))
            .then_with(|| a.suggested_word.cmp(&b.suggested_word))
    });
    out
}

/// Public entry point for the MCP `criteria_proposals` tool.
pub fn criteria_proposals(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Vec<Proposal> {
    let entries = closure_entries(manifest, dict);
    let mut proposals = collect_proposals(manifest, &entries, dict);
    attach_dict_hints(&mut proposals, dict);
    proposals
}

/// For each proposal with a `suggested_word`, run two quick dict
/// queries — prefix match on `word`, substring on `describe` — and
/// attach the top 3 matches as hints. Scored:
///   - 3: word == suggested_word exactly
///   - 2: word starts with suggested_word
///   - 1: describe contains suggested_word
/// Sorted desc by score, ties broken by entry_id asc. Silently keeps
/// hints empty on query failure; dreaming mode must tolerate a
/// partially-populated dict.
fn attach_dict_hints(proposals: &mut [Proposal], dict: &nom_dict::NomDict) {
    use std::collections::HashMap;

    for p in proposals.iter_mut() {
        let Some(word) = p.suggested_word.clone() else { continue };
        if word.is_empty() {
            continue;
        }

        let mut scored: HashMap<String, (u8, DictHint)> = HashMap::new();
        if let Ok(rows) = dict.find_by_word(&word) {
            for e in rows {
                let score = if e.word == word { 3 } else { 2 };
                scored
                    .entry(e.id.clone())
                    .and_modify(|cur| {
                        if score > cur.0 {
                            *cur = (
                                score,
                                DictHint {
                                    entry_id: e.id.clone(),
                                    word: e.word.clone(),
                                    kind: e.kind.as_str().to_string(),
                                    describe: e.describe.clone(),
                                    match_score: score,
                                },
                            );
                        }
                    })
                    .or_insert((
                        score,
                        DictHint {
                            entry_id: e.id.clone(),
                            word: e.word.clone(),
                            kind: e.kind.as_str().to_string(),
                            describe: e.describe.clone(),
                            match_score: score,
                        },
                    ));
            }
        }
        if let Ok(rows) = dict.search_describe(&word, 10) {
            for e in rows {
                scored.entry(e.id.clone()).or_insert((
                    1,
                    DictHint {
                        entry_id: e.id.clone(),
                        word: e.word.clone(),
                        kind: e.kind.as_str().to_string(),
                        describe: e.describe.clone(),
                        match_score: 1,
                    },
                ));
            }
        }
        let mut hints: Vec<DictHint> = scored.into_values().map(|(_, h)| h).collect();
        hints.sort_by(|a, b| {
            b.match_score
                .cmp(&a.match_score)
                .then_with(|| a.entry_id.cmp(&b.entry_id))
        });
        hints.truncate(3);
        p.dict_hints = hints;
    }
}

/// Epic threshold in 0..=100 points. Dreaming mode stops when the
/// computed app_score exceeds this value, per user directive:
/// "recompose the .nom in loop until scoring match higher 95".
pub const EPIC_SCORE_THRESHOLD: u32 = 95;

/// Dreaming-mode report surfaced both by `nom app dream` and the
/// Criteria aspect. The LLM consumes this across iterations to decide
/// whether to keep authoring, ask the user to skip, or stop (epic).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DreamReport {
    /// 0..=100 blended fitness score. `is_epic` iff score ≥ threshold.
    pub app_score: u32,
    pub score_threshold: u32,
    pub is_epic: bool,
    pub closure_size: usize,
    pub complete: usize,
    pub partial: usize,
    pub test_cases: usize,
    pub proposals: Vec<Proposal>,
    /// Directive surfaced to the LLM each iteration: "query more,
    /// then recompose". Empty when epic.
    pub next_instruction: String,
}

/// Compute a DreamReport for the manifest — the single source of
/// truth for both Criteria-aspect scoring and the dream CLI.
pub fn dream_report(manifest: &AppManifest, dict: &nom_dict::NomDict) -> DreamReport {
    let entries = closure_entries(manifest, dict);
    let mut proposals = collect_proposals(manifest, &entries, dict);
    attach_dict_hints(&mut proposals, dict);
    let closure_size = entries.len();
    let complete = entries
        .iter()
        .filter(|e| e.status == nom_types::EntryStatus::Complete)
        .count();
    let partial = entries
        .iter()
        .filter(|e| e.status == nom_types::EntryStatus::Partial)
        .count();
    let test_cases = entries
        .iter()
        .filter(|e| e.kind == nom_types::EntryKind::TestCase)
        .count();

    let app_score = compute_app_score(closure_size, complete, test_cases, proposals.len());
    let is_epic = app_score >= EPIC_SCORE_THRESHOLD;
    let next_instruction = if is_epic {
        String::new()
    } else {
        format!(
            "Query the dict (list_nomtu / search_nomtu / get_concept) for the \
             suggested_word + suggested_concept of each proposal, then author \
             matching nomtu (`nom store add`) or lift Partial entries to \
             Complete. Re-run `nom app dream` after each batch. Target score: \
             ≥{EPIC_SCORE_THRESHOLD}. If you have exhausted the dict without \
             reaching epic, ask the user whether to skip this manifest."
        )
    };
    DreamReport {
        app_score,
        score_threshold: EPIC_SCORE_THRESHOLD,
        is_epic,
        closure_size,
        complete,
        partial,
        test_cases,
        proposals,
        next_instruction,
    }
}

/// Blended 0..=100 fitness score. Four 25-point axes:
///   - closure breadth: min(closure_size / 8, 1) × 25
///   - completeness:    (complete / closure_size) × 25
///   - test coverage:   min(test_cases / 2, 1) × 25
///   - gap cleanliness: max(0, 1 − proposal_count / 8) × 25
/// Empty closures score 0 (no breadth, no completeness).
fn compute_app_score(
    closure_size: usize,
    complete: usize,
    test_cases: usize,
    proposal_count: usize,
) -> u32 {
    if closure_size == 0 {
        return 0;
    }
    let breadth = (closure_size as f64 / 8.0).min(1.0) * 25.0;
    let completeness = (complete as f64 / closure_size as f64) * 25.0;
    let tests = (test_cases as f64 / 2.0).min(1.0) * 25.0;
    let cleanliness = (1.0 - (proposal_count as f64 / 8.0)).max(0.0) * 25.0;
    (breadth + completeness + tests + cleanliness).round() as u32
}

fn build_optimize_aspect(
    manifest: &AppManifest,
    dict: &nom_dict::NomDict,
) -> Result<Vec<u8>, AppError> {
    let entries = closure_entries(manifest, dict);

    // Per-entry payload size (bc bytes if present, else source body length).
    #[derive(serde::Serialize)]
    struct SizedEntry<'a> {
        entry_id: &'a str,
        word: &'a str,
        kind: &'a str,
        language: &'a str,
        body_kind: Option<&'a str>,
        body_bytes: usize,
        translation_score: Option<f32>,
    }

    let sized: Vec<SizedEntry> = entries
        .iter()
        .map(|e| SizedEntry {
            entry_id: &e.id,
            word: &e.word,
            kind: e.kind.as_str(),
            language: &e.language,
            body_kind: e.body_kind.as_deref(),
            body_bytes: e.body_bytes.as_ref().map(|b| b.len()).unwrap_or(0),
            translation_score: e.translation_score,
        })
        .collect();

    // Per-language byte totals for platform specialization planning.
    let mut per_language: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut per_body_kind: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut total_bytes: u64 = 0;
    for s in &sized {
        *per_language.entry(s.language.to_string()).or_insert(0) += s.body_bytes as u64;
        if let Some(bk) = s.body_kind {
            *per_body_kind.entry(bk.to_string()).or_insert(0) += s.body_bytes as u64;
        }
        total_bytes += s.body_bytes as u64;
    }

    // Top-10 largest — specialization candidates.
    let mut top: Vec<&SizedEntry> = sized.iter().collect();
    top.sort_by(|a, b| b.body_bytes.cmp(&a.body_bytes).then(a.entry_id.cmp(b.entry_id)));
    top.truncate(10);

    // Score summary — which entries are gold-standard vs need translator work.
    let scored: Vec<&SizedEntry> = sized
        .iter()
        .filter(|s| s.translation_score.is_some())
        .collect();
    let avg_score = if scored.is_empty() {
        None
    } else {
        let sum: f32 = scored
            .iter()
            .map(|s| s.translation_score.unwrap_or(0.0))
            .sum();
        Some(sum / scored.len() as f32)
    };

    let doc = serde_json::json!({
        "app": manifest.name,
        "manifest_hash": manifest.manifest_hash,
        "closure_size": sized.len(),
        "total_bytes": total_bytes,
        "per_language_bytes": per_language,
        "per_body_kind_bytes": per_body_kind,
        "avg_translation_score": avg_score,
        "scored_count": scored.len(),
        "top_specialization_candidates": top,
    });
    Ok(serde_json::to_vec_pretty(&doc)?)
}

fn severity_rank(s: &nom_types::Severity) -> u8 {
    match s {
        nom_types::Severity::Critical => 4,
        nom_types::Severity::High => 3,
        nom_types::Severity::Medium => 2,
        nom_types::Severity::Low => 1,
        nom_types::Severity::Info => 0,
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
    fn security_aspect_serializes_findings_from_closure() {
        use nom_dict::NomDict;
        use nom_types::{
            Contract, Entry, EntryKind, EntryStatus, SecurityFinding, Severity,
        };

        let dict = NomDict::open_in_memory().unwrap();
        let make = |id: &str, word: &str| Entry {
            id: id.into(),
            word: word.into(),
            variant: None,
            kind: EntryKind::Function,
            language: "nom".into(),
            describe: None,
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract::default(),
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "2026-04-13T00:00:00Z".into(),
            updated_at: None,
        };
        dict.upsert_entry(&make("root", "root_page")).unwrap();
        dict.upsert_entry(&make("act1", "act")).unwrap();
        dict.add_finding(
            "root",
            &SecurityFinding {
                finding_id: 0,
                id: "root".into(),
                severity: Severity::Low,
                category: "info".into(),
                rule_id: Some("R1".into()),
                message: Some("low on root".into()),
                evidence: None,
                line: Some(1),
                remediation: None,
            },
        )
        .unwrap();
        dict.add_finding(
            "act1",
            &SecurityFinding {
                finding_id: 0,
                id: "act1".into(),
                severity: Severity::Critical,
                category: "injection".into(),
                rule_id: Some("R2".into()),
                message: Some("critical on action".into()),
                evidence: None,
                line: Some(42),
                remediation: None,
            },
        )
        .unwrap();

        let manifest = AppManifest {
            manifest_hash: "m1".into(),
            name: "demo".into(),
            default_target: "web".into(),
            root_page_hash: "root".into(),
            data_sources: vec![],
            actions: vec!["act1".into()],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        };
        let arts = compile_app_to_artifacts_with_dict(&manifest, &dict).unwrap();
        let sec = arts.iter().find(|a| a.aspect == OutputAspect::Security).unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&sec.bytes).unwrap();
        assert_eq!(doc["app"], "demo");
        assert_eq!(doc["closure_size"], 2);
        let findings = doc["findings"].as_array().unwrap();
        assert_eq!(findings.len(), 2);
        // Critical sorted before Low.
        assert_eq!(findings[0]["severity"], "critical");
        assert_eq!(findings[0]["entry_id"], "act1");
        assert_eq!(findings[1]["severity"], "low");
    }

    #[test]
    fn ux_aspect_serializes_screens_and_pages_from_closure() {
        use nom_dict::NomDict;
        use nom_types::{Contract, Entry, EntryKind, EntryStatus};

        let dict = NomDict::open_in_memory().unwrap();
        let make = |id: &str, word: &str, kind: EntryKind| Entry {
            id: id.into(),
            word: word.into(),
            variant: None,
            kind,
            language: "nom".into(),
            describe: Some(format!("desc for {word}")),
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract::default(),
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "2026-04-13T00:00:00Z".into(),
            updated_at: None,
        };
        dict.upsert_entry(&make("home", "home", EntryKind::Page)).unwrap();
        dict.upsert_entry(&make("login", "login", EntryKind::Screen)).unwrap();
        dict.upsert_entry(&make("signup", "signup_flow", EntryKind::UserFlow)).unwrap();
        dict.upsert_entry(&make("btn_pat", "primary_btn", EntryKind::UxPattern)).unwrap();

        let manifest = AppManifest {
            manifest_hash: "m".into(),
            name: "ux_demo".into(),
            default_target: "web".into(),
            root_page_hash: "home".into(),
            data_sources: vec!["login".into()],
            actions: vec!["signup".into()],
            media_assets: vec!["btn_pat".into()],
            settings: serde_json::Value::Null,
        };
        let arts = compile_app_to_artifacts_with_dict(&manifest, &dict).unwrap();
        let ux = arts.iter().find(|a| a.aspect == OutputAspect::Ux).unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&ux.bytes).unwrap();
        assert_eq!(doc["app"], "ux_demo");
        assert_eq!(doc["pages"].as_array().unwrap().len(), 1);
        assert_eq!(doc["screens"].as_array().unwrap().len(), 1);
        assert_eq!(doc["user_flows"].as_array().unwrap().len(), 1);
        assert_eq!(doc["ux_patterns"].as_array().unwrap().len(), 1);
        assert_eq!(doc["pages"][0]["entry_id"], "home");
        assert_eq!(doc["screens"][0]["entry_id"], "login");
    }

    #[test]
    fn all_populators_emit_valid_json_for_populated_aspects() {
        use nom_dict::NomDict;
        use nom_types::{Contract, Entry, EntryKind, EntryStatus};

        let dict = NomDict::open_in_memory().unwrap();
        let mk_kind = |id: &str, kind: EntryKind| Entry {
            id: id.into(),
            word: id.into(),
            variant: None,
            kind,
            language: "nom".into(),
            describe: Some("d".into()),
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract {
                input_type: Some("In".into()),
                output_type: Some("Out".into()),
                pre: Some("x>0".into()),
                post: Some("y>0".into()),
            },
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "2026-04-13T00:00:00Z".into(),
            updated_at: None,
        };
        dict.upsert_entry(&mk_kind("root", EntryKind::Page)).unwrap();
        dict.upsert_entry(&mk_kind("ds", EntryKind::DataSource)).unwrap();
        dict.upsert_entry(&mk_kind("av", EntryKind::AppVariable)).unwrap();
        dict.upsert_entry(&mk_kind("bench", EntryKind::BenchmarkRun)).unwrap();
        dict.upsert_entry(&mk_kind("api", EntryKind::ApiEndpoint)).unwrap();
        dict.upsert_entry(&mk_kind("flow", EntryKind::FlowArtifact)).unwrap();
        dict.upsert_entry(&mk_kind("test", EntryKind::TestCase)).unwrap();

        let manifest = AppManifest {
            manifest_hash: "m".into(),
            name: "big".into(),
            default_target: "desktop".into(),
            root_page_hash: "root".into(),
            data_sources: vec!["ds".into(), "av".into(), "api".into()],
            actions: vec!["bench".into(), "flow".into(), "test".into()],
            media_assets: vec![],
            settings: serde_json::json!({"theme": "dark"}),
        };
        let arts = compile_app_to_artifacts_with_dict(&manifest, &dict).unwrap();

        let parse = |aspect: OutputAspect| -> serde_json::Value {
            let a = arts.iter().find(|x| x.aspect == aspect).unwrap();
            serde_json::from_slice(&a.bytes).unwrap()
        };

        assert_eq!(parse(OutputAspect::Env)["target_platform"], "desktop");
        assert_eq!(parse(OutputAspect::Env)["data_sources"].as_array().unwrap().len(), 1);
        assert_eq!(parse(OutputAspect::Env)["app_variables"].as_array().unwrap().len(), 1);
        assert_eq!(parse(OutputAspect::BizLogic)["rules"].as_array().unwrap().len(), 7);
        assert_eq!(parse(OutputAspect::Bench)["benchmark_runs"].as_array().unwrap().len(), 1);
        assert_eq!(parse(OutputAspect::Response)["endpoints"].as_array().unwrap().len(), 1);
        assert_eq!(parse(OutputAspect::Flow)["flow_artifacts"].as_array().unwrap().len(), 1);
        assert_eq!(parse(OutputAspect::Criteria)["test_cases"].as_array().unwrap().len(), 1);
        assert_eq!(parse(OutputAspect::Criteria)["closure_size"], 7);
    }

    #[test]
    fn criteria_proposals_surface_weird_gaps() {
        use nom_dict::NomDict;
        use nom_types::{Contract, Entry, EntryKind, EntryStatus};

        let dict = NomDict::open_in_memory().unwrap();
        // Complete entry — no proposal.
        dict.upsert_entry(&Entry {
            id: "good".into(),
            word: "fine".into(),
            variant: None,
            kind: EntryKind::Function,
            language: "nom".into(),
            describe: None,
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract::default(),
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "t".into(),
            updated_at: None,
        })
        .unwrap();
        // Partial entry — should produce a partial_entry proposal.
        dict.upsert_entry(&Entry {
            id: "half".into(),
            word: "rough".into(),
            variant: None,
            kind: EntryKind::Function,
            language: "nom".into(),
            describe: None,
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract::default(),
            status: EntryStatus::Partial,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "t".into(),
            updated_at: None,
        })
        .unwrap();
        // Entry with pre but no post — unbalanced contract.
        dict.upsert_entry(&Entry {
            id: "lopsided".into(),
            word: "needs_post".into(),
            variant: None,
            kind: EntryKind::Function,
            language: "nom".into(),
            describe: None,
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract {
                input_type: None,
                output_type: None,
                pre: Some("x > 0".into()),
                post: None,
            },
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "t".into(),
            updated_at: None,
        })
        .unwrap();

        let manifest = AppManifest {
            manifest_hash: "m1".into(),
            name: "demo".into(),
            default_target: "web".into(),
            root_page_hash: "good".into(),
            data_sources: vec!["half".into(), "lopsided".into(), "ghost".into()],
            actions: vec![],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        };

        let proposals = criteria_proposals(&manifest, &dict);
        let kinds: Vec<&str> = proposals.iter().map(|p| p.kind.as_str()).collect();
        assert!(kinds.contains(&"missing_root"), "expected missing_root for ghost: {kinds:?}");
        assert!(kinds.contains(&"partial_entry"), "expected partial_entry for half: {kinds:?}");
        assert!(kinds.contains(&"unbalanced_contract"), "expected unbalanced: {kinds:?}");
        assert!(kinds.contains(&"no_tests"), "expected no_tests: {kinds:?}");

        // Also exercised via Criteria aspect.
        let arts = compile_app_to_artifacts_with_dict(&manifest, &dict).unwrap();
        let crit = arts.iter().find(|a| a.aspect == OutputAspect::Criteria).unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&crit.bytes).unwrap();
        assert_eq!(doc["is_epic"], false);
        assert!(doc["proposals"].as_array().unwrap().len() >= 4);

        // Deterministic: run again, same order.
        let again = criteria_proposals(&manifest, &dict);
        assert_eq!(proposals, again);
    }

    #[test]
    fn dream_report_scores_zero_for_empty_and_high_for_clean() {
        use nom_dict::NomDict;
        use nom_types::{Contract, Entry, EntryKind, EntryStatus};

        let dict = NomDict::open_in_memory().unwrap();
        let manifest_empty = AppManifest {
            manifest_hash: "m".into(),
            name: "e".into(),
            default_target: "web".into(),
            root_page_hash: String::new(),
            data_sources: vec![],
            actions: vec![],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        };
        let r0 = dream_report(&manifest_empty, &dict);
        assert_eq!(r0.app_score, 0);
        assert!(!r0.is_epic);
        assert!(!r0.next_instruction.is_empty());

        // Build a broad, complete, well-tested closure.
        let mk = |id: &str, kind: EntryKind, status: EntryStatus| Entry {
            id: id.into(),
            word: id.into(),
            variant: None,
            kind,
            language: "nom".into(),
            describe: None,
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract::default(),
            status,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "t".into(),
            updated_at: None,
        };
        for i in 0..8 {
            let id = format!("ok{i}");
            dict.upsert_entry(&mk(&id, EntryKind::Function, EntryStatus::Complete))
                .unwrap();
        }
        dict.upsert_entry(&mk("t1", EntryKind::TestCase, EntryStatus::Complete)).unwrap();
        dict.upsert_entry(&mk("t2", EntryKind::TestCase, EntryStatus::Complete)).unwrap();
        let manifest_epic = AppManifest {
            manifest_hash: "m2".into(),
            name: "epic".into(),
            default_target: "web".into(),
            root_page_hash: "ok0".into(),
            data_sources: (1..8).map(|i| format!("ok{i}")).collect(),
            actions: vec!["t1".into(), "t2".into()],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        };
        let r1 = dream_report(&manifest_epic, &dict);
        assert!(r1.app_score >= EPIC_SCORE_THRESHOLD, "got {}", r1.app_score);
        assert!(r1.is_epic);
        assert_eq!(r1.next_instruction, "");
    }

    #[test]
    fn optimize_aspect_reports_sizes_and_top_candidates() {
        use nom_dict::NomDict;
        use nom_types::{Contract, Entry, EntryKind, EntryStatus};

        let dict = NomDict::open_in_memory().unwrap();
        let mk = |id: &str, lang: &str, bc_size: usize, score: Option<f32>| Entry {
            id: id.into(),
            word: id.into(),
            variant: None,
            kind: EntryKind::Function,
            language: lang.into(),
            describe: None,
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: Some(vec![0u8; bc_size]),
            body_kind: Some("bc".into()),
            contract: Contract::default(),
            status: EntryStatus::Complete,
            translation_score: score,
            is_canonical: true,
            deprecated_by: None,
            created_at: "t".into(),
            updated_at: None,
        };
        dict.upsert_entry(&mk("a", "rust", 1000, Some(0.9))).unwrap();
        dict.upsert_entry(&mk("b", "rust", 500, Some(0.7))).unwrap();
        dict.upsert_entry(&mk("c", "typescript", 200, None)).unwrap();

        let manifest = AppManifest {
            manifest_hash: "m".into(),
            name: "opt".into(),
            default_target: "web".into(),
            root_page_hash: "a".into(),
            data_sources: vec!["b".into(), "c".into()],
            actions: vec![],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        };
        let arts = compile_app_to_artifacts_with_dict(&manifest, &dict).unwrap();
        let opt = arts.iter().find(|x| x.aspect == OutputAspect::Optimize).unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&opt.bytes).unwrap();
        assert_eq!(doc["closure_size"], 3);
        assert_eq!(doc["total_bytes"], 1700);
        assert_eq!(doc["per_language_bytes"]["rust"], 1500);
        assert_eq!(doc["per_language_bytes"]["typescript"], 200);
        assert_eq!(doc["per_body_kind_bytes"]["bc"], 1700);
        assert_eq!(doc["scored_count"], 2);
        // Top candidate is the largest.
        let top = doc["top_specialization_candidates"].as_array().unwrap();
        assert_eq!(top[0]["entry_id"], "a");
        assert_eq!(top[0]["body_bytes"], 1000);
    }

    #[test]
    fn dict_hints_surface_exact_word_match() {
        use nom_dict::NomDict;
        use nom_types::{Contract, Entry, EntryKind, EntryStatus};

        let dict = NomDict::open_in_memory().unwrap();
        // Seed dict with entries matching a word we'll suggest.
        let mk = |id: &str, word: &str, describe: Option<&str>| Entry {
            id: id.into(),
            word: word.into(),
            variant: None,
            kind: EntryKind::Function,
            language: "nom".into(),
            describe: describe.map(str::to_string),
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract::default(),
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "t".into(),
            updated_at: None,
        };
        dict.upsert_entry(&mk("e1", "needs_post", Some("exact word match"))).unwrap();
        dict.upsert_entry(&mk("e2", "something_else", Some("contains needs_post here"))).unwrap();

        // Trigger unbalanced_contract proposal via an entry with pre-only.
        dict.upsert_entry(&Entry {
            id: "lopsided".into(),
            word: "needs_post".into(),
            variant: None,
            kind: EntryKind::Function,
            language: "nom".into(),
            describe: None,
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract {
                input_type: None,
                output_type: None,
                pre: Some("x>0".into()),
                post: None,
            },
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "t".into(),
            updated_at: None,
        }).unwrap();

        let manifest = AppManifest {
            manifest_hash: "m".into(),
            name: "app".into(),
            default_target: "web".into(),
            root_page_hash: "lopsided".into(),
            data_sources: vec![],
            actions: vec![],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        };

        let proposals = criteria_proposals(&manifest, &dict);
        let hinted: Vec<_> = proposals
            .iter()
            .filter(|p| !p.dict_hints.is_empty())
            .collect();
        assert!(!hinted.is_empty(), "expected at least one proposal with hints");
        let first = &hinted[0];
        // exact word match should be top-scored (3)
        assert_eq!(first.dict_hints[0].match_score, 3);
        assert_eq!(first.dict_hints[0].word, "needs_post");
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
