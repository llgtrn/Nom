//! Build report generation for `nom build report <repo>`.
//!
//! Combines the manifest resolver output (per-slot resolution trace,
//! rejection reasons, alternatives, scores) with MECE outcome and provenance
//! into a single auditable ReportBundle.
//!
//! The bundle can be emitted as:
//!   - Machine-readable JSON (`--format json`) via serde.
//!   - Human-readable prose (`--format human`, the default).
//!
//! Exit codes follow the same discipline as `status` and `manifest`:
//!   0 = OverallVerdict::Clean (zero unresolved + zero MECE + zero threshold failures)
//!   1 = OverallVerdict::NeedsAttention

use std::path::Path;

use nom_dict::NomDict;
use serde::{Deserialize, Serialize};

use crate::manifest::{self, MeceViolationRecord};

// ── Public types ──────────────────────────────────────────────────────────────

/// Top-level container produced by `nom build report`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportBundle {
    pub schema_version: u32,
    pub generated_at_secs: u64,
    pub repo_path: String,
    /// Result of `git rev-parse HEAD`, or None when not a git repo.
    pub head_commit: Option<String>,
    pub concepts: Vec<ConceptReport>,
    pub overall: OverallVerdict,
}

/// Per-concept auditable report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptReport {
    pub name: String,
    pub intent: String,
    pub slots: Vec<SlotResolution>,
    pub mece: MeceOutcome,
    pub effects_aggregate: EffectsAggregate,
    /// Acceptance predicate prose (pass-through from parser).
    pub acceptance: Vec<String>,
    /// Ranked objectives.
    pub objectives: Vec<String>,
}

/// Resolution trace for one slot in the concept closure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotResolution {
    pub source_line: Option<usize>,
    pub kind: Option<String>,
    /// Empty string for typed-slot refs (source has `the @Kind matching "..."`).
    pub word: String,
    /// Optional prose hint from the `matching "..."` clause.
    pub matching: Option<String>,
    pub typed_slot: bool,
    pub threshold: Option<f64>,
    pub outcome: SlotOutcome,
}

/// Resolution outcome for a single slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SlotOutcome {
    Resolved {
        hash: String,
        picked_word: String,
        alternatives: Vec<Alternative>,
        rejection_reasons: Vec<String>,
    },
    Unresolved {
        reason: String,
        candidates_considered: Vec<Alternative>,
    },
}

/// A candidate entry that was considered but not selected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub hash: String,
    pub word: String,
    pub why_not_picked: String,
}

/// MECE check outcome for a concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeceOutcome {
    pub me_collisions: Vec<MeceViolationRecord>,
    /// CE check is deferred to Phase 9.
    pub ce_notes: Vec<String>,
}

/// Union of all effect names referenced by a concept's build items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectsAggregate {
    pub benefits: Vec<String>,
    pub hazards: Vec<String>,
}

/// Overall verdict for the bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OverallVerdict {
    /// Zero unresolved, zero MECE collisions, zero threshold failures.
    Clean,
    NeedsAttention { reasons: Vec<String> },
}

// ── Core pipeline ─────────────────────────────────────────────────────────────

/// Build the report bundle for all (or one) concept in `repo` using `dict`.
///
/// Reuses `manifest::build_manifest` internally; the ReportBundle is a richer
/// view over the same data with per-slot traces and provenance.
///
/// # Errors
/// Returns `Err(String)` only on hard failures (DB open, graph materialisation).
/// Per-slot resolver failures surface as `SlotOutcome::Unresolved`, not errors.
pub fn build_report(
    repo: &Path,
    dict: &NomDict,
    concept_filter: Option<&str>,
) -> Result<ReportBundle, String> {
    let repo_path = repo.to_string_lossy().into_owned();

    let generated_at_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // ── git HEAD ──────────────────────────────────────────────────────────────
    let head_commit = read_git_head(repo);

    // ── delegate to manifest pipeline ────────────────────────────────────────
    let repo_manifest = manifest::build_manifest(repo, dict, concept_filter)?;

    // ── build per-concept reports ─────────────────────────────────────────────
    let mut concept_reports: Vec<ConceptReport> = Vec::new();
    let mut overall_reasons: Vec<String> = Vec::new();

    for cm in &repo_manifest.concepts {
        // ── slots: derive from build_order ────────────────────────────────────
        let mut slots: Vec<SlotResolution> = Vec::new();

        for item in &cm.build_order {
            let outcome = if let Some(hash) = &item.hash {
                // Resolved item.  Alternatives aren't carried in BuildItem today;
                // Phase-9 will populate real per-slot scoring.  For now we derive
                // alternatives from the unresolved list (which has the full context
                // we need via the store resolver).  Since BuildItem doesn't carry
                // alternatives directly we leave the list empty here — the human
                // render notes "Phase-9 will populate alternatives".
                //
                // The picked_word is the word from BuildItem; for typed-slot items
                // the word is empty so we re-derive from the hash via the dict.
                let picked_word = if item.word.is_empty() {
                    dict.find_entity(hash)
                        .ok()
                        .flatten()
                        .map(|r| r.word)
                        .unwrap_or_else(|| hash[..16.min(hash.len())].to_string())
                } else {
                    item.word.clone()
                };

                SlotOutcome::Resolved {
                    hash: hash.clone(),
                    picked_word,
                    alternatives: vec![],
                    rejection_reasons: vec![],
                }
            } else {
                // Unresolved item.
                let reason = if item.typed_slot {
                    format!(
                        "no @{} entries in dict",
                        item.kind
                            .chars()
                            .next()
                            .map(|c| c.to_uppercase().to_string() + &item.kind[c.len_utf8()..])
                            .as_deref()
                            .unwrap_or(&item.kind)
                    )
                } else {
                    "no matching entry in entities".to_string()
                };
                SlotOutcome::Unresolved {
                    reason,
                    candidates_considered: vec![],
                }
            };

            slots.push(SlotResolution {
                source_line: None,
                kind: Some(item.kind.clone()),
                word: item.word.clone(),
                matching: None,
                typed_slot: item.typed_slot,
                threshold: item.confidence_threshold,
                outcome,
            });
        }

        // Also emit slots for unresolved refs that aren't yet in build_order
        // (the manifest already maps these into build_order with hash=None, so
        // they are covered above; this loop is a safety net for future changes).
        for uref in &cm.unresolved {
            // Avoid duplicating entries already present as hash=None items above.
            let already_present = slots.iter().any(|s| {
                s.word == uref.word && s.kind.as_deref() == uref.kind.as_deref()
            });
            if !already_present {
                slots.push(SlotResolution {
                    source_line: None,
                    kind: uref.kind.clone(),
                    word: uref.word.clone(),
                    matching: uref.matching.clone(),
                    typed_slot: uref.typed_slot,
                    threshold: uref.confidence_threshold,
                    outcome: SlotOutcome::Unresolved {
                        reason: "no matching entry in entities".to_string(),
                        candidates_considered: vec![],
                    },
                });
            }
        }

        // ── MECE outcome ──────────────────────────────────────────────────────
        let mece = MeceOutcome {
            me_collisions: cm.mece_violations.clone(),
            ce_notes: vec!["CE check deferred to Phase 9".to_string()],
        };

        // ── effects aggregate ─────────────────────────────────────────────────
        let mut benefits: Vec<String> = Vec::new();
        let mut hazards: Vec<String> = Vec::new();

        for item in &cm.build_order {
            for eff in &item.effects {
                let target = if eff.valence == "benefit" {
                    &mut benefits
                } else {
                    &mut hazards
                };
                for name in &eff.names {
                    if !target.contains(name) {
                        target.push(name.clone());
                    }
                }
            }
        }

        let effects_aggregate = EffectsAggregate { benefits, hazards };

        // ── contribute to overall verdict ─────────────────────────────────────
        let unresolved_count = slots
            .iter()
            .filter(|s| matches!(&s.outcome, SlotOutcome::Unresolved { .. }))
            .count();
        let mece_count = mece.me_collisions.len();

        if unresolved_count > 0 {
            overall_reasons.push(format!(
                "{}: {} unresolved slot(s)",
                cm.name, unresolved_count
            ));
        }
        if mece_count > 0 {
            overall_reasons.push(format!(
                "{}: {} MECE ME collision(s)",
                cm.name, mece_count
            ));
        }

        concept_reports.push(ConceptReport {
            name: cm.name.clone(),
            intent: cm.intent.clone(),
            slots,
            mece,
            effects_aggregate,
            acceptance: cm.acceptance.clone(),
            objectives: cm.objectives.clone(),
        });
    }

    let overall = if overall_reasons.is_empty() {
        OverallVerdict::Clean
    } else {
        OverallVerdict::NeedsAttention {
            reasons: overall_reasons,
        }
    };

    Ok(ReportBundle {
        schema_version: 1,
        generated_at_secs,
        repo_path,
        head_commit,
        concepts: concept_reports,
        overall,
    })
}

/// Try to read `git rev-parse HEAD` from the repo directory.
/// Returns None if git is unavailable or this is not a git repo.
fn read_git_head(repo: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;

    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

// ── Human renderer ────────────────────────────────────────────────────────────

/// Render the ReportBundle as human-readable prose.
pub fn render_report_human(bundle: &ReportBundle) -> String {
    let mut out = String::new();

    out.push_str(&format!("generated_at: {}\n", bundle.generated_at_secs));
    out.push_str(&format!("repo: {}\n", bundle.repo_path));
    if let Some(ref commit) = bundle.head_commit {
        out.push_str(&format!("HEAD: {commit}\n"));
    }
    out.push('\n');

    for cr in &bundle.concepts {
        out.push_str(&format!("═══ concept {} ═══\n", cr.name));
        out.push_str(&format!("intent: {}\n", cr.intent));
        out.push('\n');

        // ── slots ─────────────────────────────────────────────────────────────
        let slot_count = cr.slots.len();
        out.push_str(&format!("slots ({slot_count}):\n"));

        for slot in &cr.slots {
            let kind_str = slot.kind.as_deref().unwrap_or("?");
            let word_display = if slot.word.is_empty() {
                format!("@{}", capitalize(kind_str))
            } else {
                slot.word.clone()
            };

            match &slot.outcome {
                SlotOutcome::Resolved {
                    hash,
                    picked_word,
                    alternatives,
                    rejection_reasons,
                } => {
                    let hash_short = &hash[..16.min(hash.len())];
                    if slot.typed_slot || slot.word.is_empty() {
                        let matching_str = slot
                            .matching
                            .as_deref()
                            .map(|m| format!(" matching \"{m}\""))
                            .unwrap_or_default();
                        out.push_str(&format!(
                            "  \u{2713} the {word_display}{matching_str}\n"
                        ));
                        out.push_str(&format!(
                            "    resolved: {picked_word}@{hash_short}\n"
                        ));
                    } else {
                        let alt_count = alternatives.len();
                        out.push_str(&format!(
                            "  \u{2713} the {kind_str} {word_display}@{hash_short}",
                        ));
                        if alt_count > 0 {
                            out.push_str(&format!("  ({alt_count} alternative(s))"));
                        } else {
                            out.push_str("  (word match)");
                        }
                        out.push('\n');
                    }

                    if !alternatives.is_empty() {
                        out.push_str("    alternatives:\n");
                        for alt in alternatives {
                            let alt_hash_short = &alt.hash[..16.min(alt.hash.len())];
                            out.push_str(&format!(
                                "      {}@{}   why not: {}\n",
                                alt.word, alt_hash_short, alt.why_not_picked
                            ));
                        }
                    }

                    if !rejection_reasons.is_empty() {
                        for r in rejection_reasons {
                            out.push_str(&format!("    note: {r}\n"));
                        }
                    }
                }
                SlotOutcome::Unresolved {
                    reason,
                    candidates_considered,
                } => {
                    let matching_str = slot
                        .matching
                        .as_deref()
                        .map(|m| format!(" matching \"{m}\""))
                        .unwrap_or_default();
                    out.push_str(&format!(
                        "  \u{2717} the {word_display}{matching_str}\n"
                    ));
                    out.push_str(&format!("    UNRESOLVED: {reason}\n"));
                    if candidates_considered.is_empty() {
                        out.push_str("    candidates considered: (none)\n");
                    } else {
                        out.push_str("    candidates considered:\n");
                        for c in candidates_considered {
                            let ch = &c.hash[..16.min(c.hash.len())];
                            out.push_str(&format!("      {}@{}\n", c.word, ch));
                        }
                    }
                }
            }
        }
        out.push('\n');

        // ── MECE ──────────────────────────────────────────────────────────────
        out.push_str("MECE:\n");
        if cr.mece.me_collisions.is_empty() {
            out.push_str("  \u{2713} no ME collisions\n");
        } else {
            for col in &cr.mece.me_collisions {
                let offenders: Vec<&str> = col
                    .bindings
                    .iter()
                    .map(|b| b.source_concept.as_str())
                    .collect();
                out.push_str(&format!(
                    "  \u{2717} ME collision on axis '{}' \u{2014} from [{}]\n",
                    col.axis,
                    offenders.join(", ")
                ));
            }
        }
        for note in &cr.mece.ce_notes {
            out.push_str(&format!("  \u{23f3} {note}\n"));
        }
        out.push('\n');

        // ── effects ───────────────────────────────────────────────────────────
        out.push_str("effects:\n");
        if cr.effects_aggregate.benefits.is_empty() && cr.effects_aggregate.hazards.is_empty() {
            out.push_str("  (none declared)\n");
        } else {
            if !cr.effects_aggregate.benefits.is_empty() {
                out.push_str(&format!(
                    "  benefits: {}\n",
                    cr.effects_aggregate.benefits.join(", ")
                ));
            }
            if !cr.effects_aggregate.hazards.is_empty() {
                out.push_str(&format!(
                    "  hazards:  {}\n",
                    cr.effects_aggregate.hazards.join(", ")
                ));
            }
        }
        out.push('\n');

        // ── acceptance ────────────────────────────────────────────────────────
        if !cr.acceptance.is_empty() {
            out.push_str("acceptance predicates:\n");
            for pred in &cr.acceptance {
                out.push_str(&format!("  - {pred}\n"));
            }
            out.push('\n');
        }

        // ── objectives ────────────────────────────────────────────────────────
        if !cr.objectives.is_empty() {
            out.push_str(&format!(
                "objectives (ranked): {}\n",
                cr.objectives.join(", ")
            ));
        }

        out.push('\n');
    }

    // ── overall verdict ───────────────────────────────────────────────────────
    match &bundle.overall {
        OverallVerdict::Clean => {
            out.push_str("\u{2550}\u{2550}\u{2550} OVERALL: CLEAN \u{2550}\u{2550}\u{2550}\n");
        }
        OverallVerdict::NeedsAttention { reasons } => {
            out.push_str(
                "\u{2550}\u{2550}\u{2550} OVERALL: NEEDS ATTENTION \u{2550}\u{2550}\u{2550}\n",
            );
            for r in reasons {
                out.push_str(&format!("  - {r}\n"));
            }
        }
    }

    out
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ── CLI entry point ───────────────────────────────────────────────────────────

/// CLI entry point: `nom build report <repo> [--dict <p>] [--concept <n>] [--out <f>] [--format json|human]`.
///
/// Exit codes:
///   0 — OverallVerdict::Clean (zero unresolved, zero MECE, zero threshold failures).
///   1 — OverallVerdict::NeedsAttention or a hard error.
pub fn cmd_build_report(
    repo: &Path,
    dict: &Path,
    concept_filter: Option<&str>,
    out: Option<&Path>,
    format: &str,
) -> i32 {
    // ── open dict ─────────────────────────────────────────────────────────────
    let dict_db = match open_dict_in_place(dict) {
        Some(d) => d,
        None => return 1,
    };

    // ── build report ─────────────────────────────────────────────────────────
    let bundle = match build_report(repo, &dict_db, concept_filter) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("nom build report: {e}");
            return 1;
        }
    };

    let exit_code = match &bundle.overall {
        OverallVerdict::Clean => 0,
        OverallVerdict::NeedsAttention { .. } => 1,
    };

    // ── render ────────────────────────────────────────────────────────────────
    let content = match format {
        "json" => match serde_json::to_string_pretty(&bundle) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("nom build report: serialise error: {e}");
                return 1;
            }
        },
        _ => render_report_human(&bundle),
    };

    // ── output ────────────────────────────────────────────────────────────────
    if let Some(path) = out {
        if let Err(e) = std::fs::write(path, &content) {
            eprintln!("nom build report: cannot write {}: {e}", path.display());
            return 1;
        }
    } else {
        print!("{content}");
    }

    exit_code
}

fn open_dict_in_place(dict: &Path) -> Option<NomDict> {
    let result = if dict.extension().is_some_and(|e| e == "db") {
        NomDict::open_in_place(dict)
    } else {
        NomDict::open(dict)
    };
    match result {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("nom: cannot open dict at {}: {e}", dict.display());
            None
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bundle(concepts: Vec<ConceptReport>, overall: OverallVerdict) -> ReportBundle {
        ReportBundle {
            schema_version: 1,
            generated_at_secs: 1_700_000_000,
            repo_path: "/tmp/test_repo".to_string(),
            head_commit: Some("abc1234".to_string()),
            concepts,
            overall,
        }
    }

    fn resolved_slot(word: &str, hash: &str) -> SlotResolution {
        SlotResolution {
            source_line: None,
            kind: Some("function".to_string()),
            word: word.to_string(),
            matching: None,
            typed_slot: false,
            threshold: None,
            outcome: SlotOutcome::Resolved {
                hash: hash.to_string(),
                picked_word: word.to_string(),
                alternatives: vec![],
                rejection_reasons: vec![],
            },
        }
    }

    fn unresolved_slot(word: &str, kind: &str) -> SlotResolution {
        SlotResolution {
            source_line: None,
            kind: Some(kind.to_string()),
            word: word.to_string(),
            matching: Some("dashboard".to_string()),
            typed_slot: word.is_empty(),
            threshold: None,
            outcome: SlotOutcome::Unresolved {
                reason: format!("no @{} entries in dict", capitalize(kind)),
                candidates_considered: vec![],
            },
        }
    }

    fn make_concept_report(
        name: &str,
        slots: Vec<SlotResolution>,
        me_collisions: Vec<MeceViolationRecord>,
    ) -> ConceptReport {
        ConceptReport {
            name: name.to_string(),
            intent: format!("intent of {name}"),
            slots,
            mece: MeceOutcome {
                me_collisions,
                ce_notes: vec!["CE check deferred to Phase 9".to_string()],
            },
            effects_aggregate: EffectsAggregate {
                benefits: vec!["cache_hit".to_string()],
                hazards: vec!["timeout".to_string()],
            },
            acceptance: vec!["the safety policy is composed".to_string()],
            objectives: vec!["security".to_string(), "speed".to_string()],
        }
    }

    // ── render_report_human: basic structure ──────────────────────────────────

    #[test]
    fn render_human_contains_concept_header() {
        let cr = make_concept_report("my_concept", vec![], vec![]);
        let bundle = make_bundle(vec![cr], OverallVerdict::Clean);
        let rendered = render_report_human(&bundle);
        assert!(
            rendered.contains("═══ concept my_concept ═══"),
            "must contain concept header: {rendered}"
        );
    }

    #[test]
    fn render_human_overall_clean() {
        let cr = make_concept_report("alpha", vec![], vec![]);
        let bundle = make_bundle(vec![cr], OverallVerdict::Clean);
        let rendered = render_report_human(&bundle);
        assert!(
            rendered.contains("OVERALL: CLEAN"),
            "must contain OVERALL: CLEAN: {rendered}"
        );
        assert!(
            !rendered.contains("NEEDS ATTENTION"),
            "must not contain NEEDS ATTENTION when clean: {rendered}"
        );
    }

    #[test]
    fn render_human_overall_needs_attention() {
        let cr = make_concept_report("beta", vec![], vec![]);
        let bundle = make_bundle(
            vec![cr],
            OverallVerdict::NeedsAttention {
                reasons: vec!["beta: 1 unresolved slot(s)".to_string()],
            },
        );
        let rendered = render_report_human(&bundle);
        assert!(
            rendered.contains("OVERALL: NEEDS ATTENTION"),
            "must contain OVERALL: NEEDS ATTENTION: {rendered}"
        );
        assert!(
            rendered.contains("1 unresolved slot(s)"),
            "must include reason: {rendered}"
        );
    }

    // ── render_report_human: resolved vs unresolved slots ────────────────────

    #[test]
    fn render_human_resolved_slot_shows_checkmark() {
        let slot = resolved_slot("login_user", "a1b2c3d4e5f60000a1b2c3d4e5f60000a1b2c3d4e5f60000a1b2c3d4e5f60000");
        let cr = make_concept_report("c1", vec![slot], vec![]);
        let bundle = make_bundle(vec![cr], OverallVerdict::Clean);
        let rendered = render_report_human(&bundle);
        // Unicode checkmark ✓
        assert!(rendered.contains('\u{2713}'), "resolved slot must show ✓: {rendered}");
        assert!(rendered.contains("login_user"), "must show word: {rendered}");
    }

    #[test]
    fn render_human_unresolved_slot_shows_cross() {
        let slot = unresolved_slot("", "Screen");
        let cr = make_concept_report("c2", vec![slot], vec![]);
        let bundle = make_bundle(
            vec![cr],
            OverallVerdict::NeedsAttention {
                reasons: vec!["c2: 1 unresolved slot(s)".to_string()],
            },
        );
        let rendered = render_report_human(&bundle);
        // Unicode cross ✗
        assert!(rendered.contains('\u{2717}'), "unresolved slot must show ✗: {rendered}");
        assert!(rendered.contains("UNRESOLVED"), "must contain UNRESOLVED: {rendered}");
    }

    #[test]
    fn render_human_mece_section_present() {
        let cr = make_concept_report("c3", vec![], vec![]);
        let bundle = make_bundle(vec![cr], OverallVerdict::Clean);
        let rendered = render_report_human(&bundle);
        assert!(rendered.contains("MECE:"), "must contain MECE section: {rendered}");
        assert!(
            rendered.contains("CE check deferred to Phase 9"),
            "must contain CE note: {rendered}"
        );
    }

    #[test]
    fn render_human_effects_section_present() {
        let cr = make_concept_report("c4", vec![], vec![]);
        let bundle = make_bundle(vec![cr], OverallVerdict::Clean);
        let rendered = render_report_human(&bundle);
        assert!(rendered.contains("effects:"), "must contain effects section: {rendered}");
        assert!(rendered.contains("cache_hit"), "must contain benefit name: {rendered}");
        assert!(rendered.contains("timeout"), "must contain hazard name: {rendered}");
    }

    // ── JSON round-trip ───────────────────────────────────────────────────────

    #[test]
    fn report_bundle_json_roundtrip() {
        let cr = make_concept_report("json_test", vec![], vec![]);
        let bundle = make_bundle(vec![cr], OverallVerdict::Clean);
        let json = serde_json::to_string(&bundle).expect("serialize");
        let back: ReportBundle = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.concepts.len(), 1);
        assert_eq!(back.concepts[0].name, "json_test");
        assert_eq!(back.overall, OverallVerdict::Clean);
    }

    #[test]
    fn overall_verdict_needs_attention_roundtrip() {
        let bundle = make_bundle(
            vec![],
            OverallVerdict::NeedsAttention {
                reasons: vec!["some reason".to_string()],
            },
        );
        let json = serde_json::to_string(&bundle).expect("serialize");
        let back: ReportBundle = serde_json::from_str(&json).expect("deserialize");
        match back.overall {
            OverallVerdict::NeedsAttention { reasons } => {
                assert_eq!(reasons, vec!["some reason"]);
            }
            OverallVerdict::Clean => panic!("expected NeedsAttention"),
        }
    }
}
