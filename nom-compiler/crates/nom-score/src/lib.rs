//! Type Scoring and Compatibility Contracts.
//!
//! Each atom gets scored 0.0--1.0 on 8 quality dimensions.
//! Compatibility contracts determine whether two atoms can be wired together.

use nom_types::{Atom, AtomKind};
use serde::{Deserialize, Serialize};

// ── Quality Scores ────────────────────────────────────────────────────

/// 8-dimensional quality scores for a single atom.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomScores {
    pub security: f32,
    pub reliability: f32,
    pub performance: f32,
    pub readability: f32,
    pub testability: f32,
    pub portability: f32,
    pub composability: f32,
    pub maturity: f32,
}

impl AtomScores {
    /// Weighted average of all 8 dimensions.
    pub fn overall(&self) -> f32 {
        let weights = [
            (self.security, 0.15),
            (self.reliability, 0.20),
            (self.performance, 0.10),
            (self.readability, 0.10),
            (self.testability, 0.10),
            (self.portability, 0.10),
            (self.composability, 0.20),
            (self.maturity, 0.05),
        ];
        weights.iter().map(|(score, w)| score * w).sum()
    }
}

// ── Per-dimension scoring heuristics ─────────────────────────────────

pub fn score_security(atom: &Atom) -> f32 {
    let lower = atom.name.to_lowercase();
    let path_lower = atom.source_path.to_lowercase();

    let mut score = 0.8_f32;

    let unsafe_signals = ["unsafe", "raw_ptr", "mem::transmute", "deref_raw"];
    for sig in &unsafe_signals {
        if lower.contains(sig) {
            score -= 0.3;
            break;
        }
    }

    let unvalidated = ["unwrap", "expect", "unchecked", "as_ptr"];
    for sig in &unvalidated {
        if lower.contains(sig) {
            score -= 0.15;
            break;
        }
    }

    let secure_signals = ["validate", "sanitize", "verify", "check", "guard", "safe"];
    for sig in &secure_signals {
        if lower.contains(sig) {
            score = (score + 0.15).min(1.0);
            break;
        }
    }

    if atom.labels.iter().any(|l| l == "security")
        || path_lower.contains("security")
        || path_lower.contains("crypto")
        || path_lower.contains("auth")
    {
        score = (score + 0.1).min(1.0);
    }

    score.clamp(0.0, 1.0)
}

pub fn score_reliability(atom: &Atom) -> f32 {
    let lower = atom.name.to_lowercase();
    let mut score = 0.6_f32;

    if let Some(ref sig) = atom.signature {
        if let Some(ref ret) = sig.returns {
            let ret_lower = ret.to_lowercase();
            if ret_lower.contains("result") {
                score += 0.25;
            } else if ret_lower.contains("option") {
                score += 0.10;
            }
        }

        if sig.returns.is_none() && !sig.params.is_empty() {
            score -= 0.05;
        }
    }

    let reliable_signals = [
        "handle_error", "retry", "fallback", "recover", "result", "safe",
    ];
    for sig in &reliable_signals {
        if lower.contains(sig) {
            score = (score + 0.1).min(1.0);
            break;
        }
    }

    let unreliable_signals = ["panic", "abort", "unwrap", "expect"];
    for sig in &unreliable_signals {
        if lower.contains(sig) {
            score -= 0.2;
            break;
        }
    }

    if atom.kind == AtomKind::TestCase {
        score = score.min(0.7);
    }

    score.clamp(0.0, 1.0)
}

pub fn score_performance(atom: &Atom) -> f32 {
    let lower = atom.name.to_lowercase();
    let mut score = 0.7_f32;

    let fast_signals = [
        "cache", "pool", "batch", "bulk", "stream", "zero_copy", "simd", "mmap", "prealloc",
    ];
    for sig in &fast_signals {
        if lower.contains(sig) {
            score = (score + 0.15).min(1.0);
            break;
        }
    }

    let slow_signals = ["clone_all", "collect_all", "realloc", "copy_bytes"];
    for sig in &slow_signals {
        if lower.contains(sig) {
            score -= 0.1;
            break;
        }
    }

    match atom.kind {
        AtomKind::DockerPattern | AtomKind::K8sManifest | AtomKind::CiWorkflow => {
            score -= 0.1;
        }
        AtomKind::Pipeline | AtomKind::EtlPipeline | AtomKind::RagPipeline => {
            score -= 0.05;
        }
        _ => {}
    }

    score.clamp(0.0, 1.0)
}

pub fn score_readability(atom: &Atom) -> f32 {
    let name = &atom.name;
    let mut score = 0.7_f32;

    if name.len() < 3 {
        score -= 0.2;
    }

    if name.len() > 50 {
        score -= 0.15;
    }

    if atom.labels.iter().any(|l| l == "documented" || l == "doc") {
        score = (score + 0.2).min(1.0);
    }

    if name.contains('_') && name.len() > 5 {
        score = (score + 0.05).min(1.0);
    }

    if name.chars().all(|c| c.is_uppercase() || c == '_') && name.len() > 2 {
        score -= 0.05;
    }

    score.clamp(0.0, 1.0)
}

pub fn score_testability(atom: &Atom) -> f32 {
    let lower = atom.name.to_lowercase();
    let mut score = 0.65_f32;

    if atom.kind == AtomKind::TestCase {
        return 0.95;
    }

    if let Some(ref sig) = atom.signature {
        if !sig.is_method {
            score += 0.1;
        }
        if !sig.params.is_empty() {
            score += 0.05;
        }
        if sig.returns.is_some() {
            score += 0.1;
        }
    }

    let testable_signals = ["parse", "convert", "compute", "calculate", "format", "map"];
    for sig in &testable_signals {
        if lower.contains(sig) {
            score = (score + 0.05).min(1.0);
            break;
        }
    }

    let hard_signals = ["global", "singleton", "main", "init_once", "static_init"];
    for sig in &hard_signals {
        if lower.contains(sig) {
            score -= 0.15;
            break;
        }
    }

    score.clamp(0.0, 1.0)
}

pub fn score_portability(atom: &Atom) -> f32 {
    let lower = atom.name.to_lowercase();
    let path_lower = atom.source_path.to_lowercase();
    let mut score = 0.8_f32;

    let platform_signals = [
        "windows", "linux", "macos", "darwin", "win32", "posix", "unix", "winapi", "ntdll",
        "syscall", "ioctl",
    ];
    for sig in &platform_signals {
        if lower.contains(sig) {
            score -= 0.25;
            break;
        }
    }

    let path_signals = [
        "windows", "linux", "macos", "darwin", "win32", "posix", "platform",
    ];
    for sig in &path_signals {
        if path_lower.contains(sig) {
            score -= 0.15;
            break;
        }
    }

    match atom.kind {
        AtomKind::NixModule => {
            score -= 0.2;
        }
        AtomKind::DockerPattern | AtomKind::K8sManifest => {
            score -= 0.05;
        }
        _ => {}
    }

    score.clamp(0.0, 1.0)
}

pub fn score_composability(atom: &Atom) -> f32 {
    let mut score = 0.5_f32;

    if let Some(ref sig) = atom.signature {
        if sig.visibility == "pub" {
            score += 0.2;
        } else if sig.visibility == "pub(crate)" {
            score += 0.1;
        }

        let typed_params = sig.params.iter().filter(|(_, t)| !t.is_empty()).count();
        let total_params = sig.params.len();
        if total_params > 0 {
            let ratio = typed_params as f32 / total_params as f32;
            score += ratio * 0.15;
        }

        if sig.returns.is_some() {
            score += 0.1;
        }

        if sig.is_async {
            score += 0.05;
        }
    } else {
        score -= 0.1;
    }

    if atom.kind == AtomKind::Schema {
        score = (score + 0.1).min(1.0);
    }

    match atom.kind {
        AtomKind::Pipeline | AtomKind::Workflow | AtomKind::EtlPipeline | AtomKind::RagPipeline => {
            score = (score + 0.1).min(1.0);
        }
        _ => {}
    }

    score.clamp(0.0, 1.0)
}

pub fn score_maturity(atom: &Atom) -> f32 {
    let path_lower = atom.source_path.to_lowercase().replace('\\', "/");
    let mut score = 0.6_f32;

    let established_donors = [
        "tokio", "serde", "anyhow", "clap", "axum", "hyper", "rustls", "ring", "openssl",
        "reqwest", "sqlx", "diesel", "actix", "warp", "tonic",
    ];
    for donor in &established_donors {
        if path_lower.contains(donor) {
            score = (score + 0.3).min(1.0);
            break;
        }
    }

    if atom.kind == AtomKind::TestCase
        || atom.labels.iter().any(|l| l == "test")
        || path_lower.contains("/test")
        || path_lower.contains("_test.")
        || path_lower.contains("tests/")
    {
        score = (score + 0.1).min(1.0);
    }

    if path_lower.contains("upstreams/") || path_lower.contains("corpus/") {
        score = (score + 0.1).min(1.0);
    }

    let wip_signals = ["wip", "hack", "todo", "fixme", "temp", "draft"];
    for sig in &wip_signals {
        if path_lower.contains(sig) || atom.name.to_lowercase().contains(sig) {
            score -= 0.2;
            break;
        }
    }

    score.clamp(0.0, 1.0)
}

// ── Top-level scorer ──────────────────────────────────────────────────

/// Compute all 8 quality scores for an atom.
pub fn score_atom(atom: &Atom) -> AtomScores {
    AtomScores {
        security: score_security(atom),
        reliability: score_reliability(atom),
        performance: score_performance(atom),
        readability: score_readability(atom),
        testability: score_testability(atom),
        portability: score_portability(atom),
        composability: score_composability(atom),
        maturity: score_maturity(atom),
    }
}

/// Convenience: score an atom and return the overall weighted average.
pub fn score_atom_overall(atom: &Atom) -> f32 {
    score_atom(atom).overall()
}

// ── Compatibility Contracts ───────────────────────────────────────────

/// Result of a wiring compatibility check between two atoms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireResult {
    Compatible { score: f32 },
    NeedsAdapter { reason: String },
    Incompatible { reason: String },
}

/// Compute a compatibility score (0.0--1.0) between two atoms.
pub fn compat_score(a: &Atom, b: &Atom) -> f32 {
    let mut score = 0.5_f32;

    if !a.language.is_empty() && a.language == b.language {
        score += 0.2;
    }

    if let (Some(ca), Some(cb)) = (&a.concept, &b.concept)
        && ca == cb
    {
        score += 0.2;
    }

    let shared_labels = a.labels.iter().filter(|l| b.labels.contains(l)).count();
    score += (shared_labels as f32 * 0.05).min(0.1);

    if let (Some(prod_sig), Some(cons_sig)) = (&a.signature, &b.signature)
        && let Some(ref ret_type) = prod_sig.returns
    {
        let match_found = cons_sig.params.iter().any(|(_, param_type)| {
            !param_type.is_empty()
                && (param_type == ret_type
                    || ret_type.contains(param_type.as_str())
                    || param_type.contains(ret_type.as_str()))
        });
        if match_found {
            score += 0.2;
        }
    }

    score.clamp(0.0, 1.0)
}

/// Detailed compatibility check for wiring `producer` -> `consumer`.
pub fn can_wire(producer: &Atom, consumer: &Atom) -> WireResult {
    let prod_vis = producer
        .signature
        .as_ref()
        .map(|s| s.visibility.as_str())
        .unwrap_or("private");
    let cons_vis = consumer
        .signature
        .as_ref()
        .map(|s| s.visibility.as_str())
        .unwrap_or("private");

    if prod_vis == "private" {
        return WireResult::Incompatible {
            reason: format!(
                "producer '{}' is private -- cannot be wired across module boundaries",
                producer.name
            ),
        };
    }
    if cons_vis == "private" {
        return WireResult::Incompatible {
            reason: format!(
                "consumer '{}' is private -- cannot receive external wiring",
                consumer.name
            ),
        };
    }

    if !producer.language.is_empty()
        && !consumer.language.is_empty()
        && producer.language != consumer.language
    {
        return WireResult::NeedsAdapter {
            reason: format!(
                "language boundary: '{}' ({}) -> '{}' ({}) requires FFI or serialisation adapter",
                producer.name, producer.language, consumer.name, consumer.language
            ),
        };
    }

    if let (Some(prod_sig), Some(cons_sig)) = (&producer.signature, &consumer.signature)
        && let Some(ref ret_type) = prod_sig.returns
    {
        let inner_type = strip_result(ret_type);

        let type_match = cons_sig.params.iter().any(|(_, param_type)| {
            !param_type.is_empty()
                && (param_type == ret_type
                    || param_type == inner_type
                    || ret_type.contains(param_type.as_str())
                    || param_type.contains(ret_type.as_str()))
        });

        if type_match {
            let score = compat_score(producer, consumer);
            return WireResult::Compatible { score };
        }

        if cons_sig.params.is_empty() {
            return WireResult::NeedsAdapter {
                reason: format!(
                    "consumer '{}' takes no parameters; producer output '{}' needs an adapter",
                    consumer.name, ret_type
                ),
            };
        }

        let param_types: Vec<&str> = cons_sig
            .params
            .iter()
            .map(|(_, t)| t.as_str())
            .filter(|t| !t.is_empty())
            .collect();

        return WireResult::NeedsAdapter {
            reason: format!(
                "type mismatch: producer '{}' returns '{}' but consumer '{}' expects [{}]",
                producer.name,
                ret_type,
                consumer.name,
                param_types.join(", ")
            ),
        };
    }

    let score = compat_score(producer, consumer);
    if score >= 0.7 {
        WireResult::Compatible { score }
    } else if score >= 0.4 {
        WireResult::NeedsAdapter {
            reason: format!(
                "no type signatures available; heuristic score {score:.2} -- adapter recommended"
            ),
        }
    } else {
        WireResult::Incompatible {
            reason: format!(
                "no type signatures and low heuristic score {score:.2} -- wiring inadvisable"
            ),
        }
    }
}

/// Strip `Result<T, E>` or `Option<T>` wrapper to get the inner type.
fn strip_result(ty: &str) -> &str {
    if let Some(inner) = ty.strip_prefix("Result<") {
        inner.split(',').next().map(str::trim).unwrap_or(ty)
    } else if let Some(inner) = ty.strip_prefix("Option<") {
        inner.trim_end_matches('>')
    } else {
        ty
    }
}
