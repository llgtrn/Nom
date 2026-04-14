//! Build manifest generation for `nom build manifest <repo>`.
//!
//! Derives a JSON-serialisable manifest from the closure walker + stub
//! resolver + MECE pipeline. The manifest is the Phase-5 planner input:
//! it contains every concept's build order (post-order, leaves first),
//! resolved hashes, MECE violations, and unresolved refs.
//!
//! Keep all logic in this file; do not put manifest logic in build.rs or
//! store.rs.

use std::collections::HashMap;
use std::path::Path;

use nom_dict::Dict;
use nom_dict::dict::{find_entities_by_word, find_entity, list_required_axes};
use nom_concept::stages::{PipelineOutput, run_pipeline};
use serde::{Deserialize, Serialize};

use crate::store::{materialize_concept_graph_from_db, resolve_closure};

// ── Public data types ────────────────────────────────────────────────────────

/// One repo's build manifest. Top-level container for one or more concepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoManifest {
    pub repo_path: String,
    /// Schema version; always 1 for now.
    pub manifest_version: u32,
    /// Seconds since Unix epoch at manifest generation time.
    /// Using epoch nanos / 1e9 avoids the chrono dep (not in workspace).
    pub generated_at_secs: u64,
    pub concepts: Vec<ConceptManifest>,
    /// Accumulated stub notes from MECE checks across all concepts.
    pub stub_notes: Vec<String>,
}

/// Manifest for a single concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptManifest {
    pub name: String,
    pub intent: String,
    /// Objectives as declared in the concept (ranked).
    pub objectives: Vec<String>,
    /// Acceptance criteria prose.
    pub acceptance: Vec<String>,
    /// Exposes list.
    pub exposes: Vec<String>,
    /// Build order: leaves first, root last (post-order from closure walker).
    pub build_order: Vec<BuildItem>,
    /// ME violations detected by the MECE check.
    pub mece_violations: Vec<MeceViolationRecord>,
    /// CE (Collectively-Exhaustive) unmet axes from the required-axes registry (M7a).
    /// Empty when the registry has no entries for this scope.
    #[serde(default)]
    pub ce_unmet: Vec<String>,
    /// Refs that the stub resolver could not pin to a hash.
    pub unresolved: Vec<UnresolvedRecord>,
}

/// One entry in the concept's build order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildItem {
    /// "function" | "module" | "concept" | etc.
    pub kind: String,
    pub word: String,
    /// None if unresolved (no hash pinned yet).
    pub hash: Option<String>,
    /// "bc" | "avif" | etc. from entities; None if unknown.
    pub body_kind: Option<String>,
    pub body_size: Option<i64>,
    /// For kind=module/composition: the constituent entity hashes.
    pub composed_of: Vec<String>,

    /// True when source used `the @Kind matching "..."` (typed-slot per doc 07 §3).
    /// When true, `word` is empty and `hash` may be None until Phase-9 resolver lands.
    #[serde(default)]
    pub typed_slot: bool,

    /// Effect declarations on this entity (motivation 02 §9 / motivation 10 §E #4).
    /// Empty for atomic Tier-0 entities ingested without effect annotations.
    /// NOTE: Phase-5 follow-up — store effects in a JSON column in entities during
    /// `nom store sync` (option a) so manifest generation does not need a source re-parse.
    #[serde(default)]
    pub effects: Vec<EffectRecord>,

    /// Per-slot inline confidence threshold (doc 07 §6.3).
    /// Phase-9 corpus-embedding-resolver enforces this. Stub resolver ignores it.
    /// `None` ≡ "use default per-kind threshold" (also ignored by stub).
    /// Only set when source used `the @Kind ... with at-least N confidence`.
    #[serde(default)]
    pub confidence_threshold: Option<f64>,
}

/// One effect group serialised for the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectRecord {
    /// "benefit" | "hazard"
    pub valence: String,
    /// Effect names (e.g. "cache_hit", "timeout").
    pub names: Vec<String>,
}

/// One ME (Mutually-Exclusive) violation serialised for the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeceViolationRecord {
    pub axis: String,
    pub bindings: Vec<ObjectiveBindingRecord>,
}

/// One binding inside a MECE violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveBindingRecord {
    pub source_concept: String,
    pub name: String,
    pub axis: String,
}

/// One reference that could not be resolved to a hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedRecord {
    pub kind: Option<String>,
    pub word: String,
    pub matching: Option<String>,
    pub referenced_from: String,
    /// True when source used the `.nomx v2` typed-slot form `the @Kind matching "..."`.
    #[serde(default)]
    pub typed_slot: bool,
    /// Per-slot inline confidence threshold (doc 07 §6.3), if declared.
    #[serde(default)]
    pub confidence_threshold: Option<f64>,
}

// ── Effect collector ─────────────────────────────────────────────────────────

/// Walk all `*.nomtu` files under `repo_path` (skipping `.git` and `target`)
/// and collect the effect clauses declared on each entity or composition,
/// keyed by word name.
///
/// Last-write-wins when the same word appears in multiple files (words are
/// supposed to be unique per doc 08 §6.5).
///
/// # Phase-5 follow-up (option a)
/// Store effects as a JSON column in `entities` during `nom store sync` so
/// manifest generation does not need a source re-parse.  The column would be
/// populated in the entity upsert loop and consumed here via `dict.find_entity`.
fn collect_effects_from_repo(repo: &Path) -> HashMap<String, Vec<EffectRecord>> {
    let mut map: HashMap<String, Vec<EffectRecord>> = HashMap::new();

    collect_effects_recurse(repo, &mut map);
    map
}

fn collect_effects_recurse(dir: &Path, map: &mut HashMap<String, Vec<EffectRecord>>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if ft.is_dir() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == ".git" || name_str == "target" {
                continue;
            }
            collect_effects_recurse(&path, map);
        } else if ft.is_file() {
            if path.extension().and_then(|e| e.to_str()) != Some("nomtu") {
                continue;
            }
            let src = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let pipeline = match run_pipeline(&src) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let nomtu_file = match pipeline {
                PipelineOutput::Nomtu(f) => f,
                PipelineOutput::Nom(_) => continue,
            };
            for item in &nomtu_file.items {
                let (word, effect_clauses) = match item {
                    nom_concept::NomtuItem::Entity(e) => (&e.word, &e.effects),
                    nom_concept::NomtuItem::Composition(c) => (&c.word, &c.effects),
                };
                if effect_clauses.is_empty() {
                    continue;
                }
                let records: Vec<EffectRecord> = effect_clauses
                    .iter()
                    .map(|ec| EffectRecord {
                        valence: match ec.valence {
                            nom_concept::EffectValence::Benefit => "benefit".to_string(),
                            nom_concept::EffectValence::Hazard => "hazard".to_string(),
                        },
                        names: ec.effects.clone(),
                    })
                    .collect();
                map.insert(word.clone(), records);
            }
        }
    }
}

// ── Core pipeline ────────────────────────────────────────────────────────────

/// Build the manifest for all (or one) concept in `repo` using `dict`.
///
/// # Errors
/// Returns `Err(String)` only on hard failures (DB open, graph
/// materialisation). Per-concept resolver failures are surfaced as
/// `unresolved` entries, not errors.
pub fn build_manifest(
    repo: &Path,
    dict: &Dict,
    concept_filter: Option<&str>,
) -> Result<RepoManifest, String> {
    let repo_path = repo.to_string_lossy().into_owned();

    let repo_id = repo
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let generated_at_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // ── collect effects from source (option b — Phase-5 will migrate to entities column) ──
    let effects_map = collect_effects_from_repo(repo);

    // ── materialise graph from DB ─────────────────────────────────────────────
    let graph = materialize_concept_graph_from_db(dict, repo_id)?;

    // ── apply concept filter ──────────────────────────────────────────────────
    let concepts_in_scope: Vec<&nom_concept::ConceptDecl> = if let Some(name) = concept_filter {
        let filtered: Vec<&nom_concept::ConceptDecl> =
            graph.concepts.iter().filter(|c| c.name == name).collect();
        if filtered.is_empty() {
            return Err(format!("concept `{name}` not found in repo `{repo_id}`"));
        }
        filtered
    } else {
        graph.concepts.iter().collect()
    };

    let mut concept_manifests: Vec<ConceptManifest> = Vec::new();
    let mut all_stub_notes: Vec<String> = Vec::new();

    for concept in concepts_in_scope {
        // ── walk closure ─────────────────────────────────────────────────────
        let closure = match graph.closure(&concept.name) {
            Ok(c) => c,
            Err(nom_concept::ClosureError::Cycle { path }) => {
                // Emit a synthetic unresolved record and continue so the
                // manifest is still useful for the concepts that do resolve.
                concept_manifests.push(ConceptManifest {
                    name: concept.name.clone(),
                    intent: concept.intent.clone(),
                    objectives: concept.objectives.clone(),
                    acceptance: concept.acceptance.clone(),
                    exposes: concept.exposes.clone(),
                    build_order: vec![],
                    mece_violations: vec![],
                    ce_unmet: vec![],
                    unresolved: vec![UnresolvedRecord {
                        kind: Some("cycle".to_string()),
                        word: path.clone(),
                        matching: None,
                        referenced_from: concept.name.clone(),
                        typed_slot: false,
                        confidence_threshold: None,
                    }],
                });
                continue;
            }
            Err(e) => {
                concept_manifests.push(ConceptManifest {
                    name: concept.name.clone(),
                    intent: concept.intent.clone(),
                    objectives: concept.objectives.clone(),
                    acceptance: concept.acceptance.clone(),
                    exposes: concept.exposes.clone(),
                    build_order: vec![],
                    mece_violations: vec![],
                    ce_unmet: vec![],
                    unresolved: vec![UnresolvedRecord {
                        kind: Some("error".to_string()),
                        word: e.to_string(),
                        matching: None,
                        referenced_from: concept.name.clone(),
                        typed_slot: false,
                        confidence_threshold: None,
                    }],
                });
                continue;
            }
        };

        // ── run stub resolver ─────────────────────────────────────────────────
        let (resolved_refs, still_unresolved, _stats) = resolve_closure(&closure, dict);

        // Build a word→hash lookup from resolved refs (v1 word-based only).
        // Typed-slot refs (word="") are handled separately in build_order below.
        let resolved_map: std::collections::HashMap<String, String> = resolved_refs
            .iter()
            .filter(|r| !r.word.is_empty())
            .map(|r| (r.word.clone(), r.hash.clone()))
            .collect();

        // ── assemble build_order: word_hashes (leaves) then concepts ─────────
        //
        // `closure.word_hashes` is already in post-order (leaves first) per
        // the closure walker contract (§4.3 doc 08).  `closure.concepts` follows
        // the same topological order with the root at the end.
        //
        // For each word hash in the closure, look up the entities row for
        // body_kind / body_size / composed_of.  Words that are referenced only
        // by prose (no hash yet) come from `closure.unresolved` — we map them
        // by word name.

        let mut build_order: Vec<BuildItem> = Vec::new();

        // First: resolved word hashes (entities + modules).
        for hash in &closure.word_hashes {
            let row = find_entity(dict, hash).ok().flatten();
            let composed_of: Vec<String> = row
                .as_ref()
                .and_then(|r| r.composed_of.as_deref())
                .and_then(|j| serde_json::from_str(j).ok())
                .unwrap_or_default();

            let word = row
                .as_ref()
                .map(|r| r.word.clone())
                .unwrap_or_else(|| hash[..16.min(hash.len())].to_string());

            let effects = effects_map.get(&word).cloned().unwrap_or_default();

            build_order.push(BuildItem {
                kind: row
                    .as_ref()
                    .map(|r| r.kind.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                word,
                hash: Some(hash.clone()),
                body_kind: row.as_ref().and_then(|r| r.body_kind.clone()),
                body_size: row.as_ref().and_then(|r| r.body_size),
                composed_of,
                typed_slot: false,
                effects,
                confidence_threshold: None,
            });
        }

        // Then: concepts referenced in post-order (leaves first, root last).
        for concept_name in &closure.concepts {
            // Look up the concept hash from resolved_map or from entities by word.
            let hash = resolved_map.get(concept_name).cloned().or_else(|| {
                find_entities_by_word(dict, concept_name)
                    .ok()
                    .and_then(|rows| rows.into_iter().next().map(|r| r.hash))
            });

            build_order.push(BuildItem {
                kind: "concept".to_string(),
                word: concept_name.clone(),
                hash,
                body_kind: None,
                body_size: None,
                composed_of: vec![],
                typed_slot: false,
                effects: vec![],
                confidence_threshold: None,
            });
        }

        // Then: resolved typed-slot refs (`@Kind matching "..."`).
        // These were resolved by find_entities_by_kind (kind-only lookup).
        // word stays empty — the source line has no word token for @hash splicing
        // (doc 07 §3.5: typed-slot hash lives in manifest/DB only, not rewritten).
        for rref in resolved_refs.iter().filter(|r| r.word.is_empty()) {
            build_order.push(BuildItem {
                kind: rref.kind.clone().unwrap_or_else(|| "unknown".to_string()),
                word: String::new(),
                hash: Some(rref.hash.clone()),
                body_kind: None,
                body_size: None,
                composed_of: vec![],
                typed_slot: true,
                effects: vec![],
                confidence_threshold: rref.confidence_threshold,
            });
        }

        // Then: still-unresolved refs (resolver found no candidate).
        // Typed-slot refs that couldn't be resolved appear here with hash=None.
        for uref in &still_unresolved {
            build_order.push(BuildItem {
                kind: uref.kind.clone().unwrap_or_else(|| "unknown".to_string()),
                word: uref.word.clone(),
                hash: None,
                body_kind: None,
                body_size: None,
                composed_of: vec![],
                typed_slot: uref.typed_slot,
                effects: vec![],
                confidence_threshold: uref.confidence_threshold,
            });
        }

        // ── MECE check ────────────────────────────────────────────────────────
        let child_concept_names: Vec<String> = concept
            .index
            .iter()
            .flat_map(|clause| match clause {
                nom_concept::IndexClause::Uses(refs) => refs.as_slice(),
                nom_concept::IndexClause::Extends { .. } => &[],
            })
            .filter(|eref| eref.kind.as_deref() == Some("concept"))
            .map(|eref| eref.word.clone())
            .collect();

        let mece_violations: Vec<MeceViolationRecord>;
        let ce_unmet_for_concept: Vec<String>;
        if !child_concept_names.is_empty() || !concept.objectives.is_empty() {
            let child_decls: Vec<&nom_concept::ConceptDecl> = child_concept_names
                .iter()
                .filter_map(|name| graph.concepts.iter().find(|c| &c.name == name))
                .collect();

            // Use registry-aware CE check.
            let required_axes: Vec<(String, String)> = list_required_axes(dict, repo_id, "concept")
                .unwrap_or_default()
                .into_iter()
                .map(|ax| (ax.axis, ax.cardinality))
                .collect();
            let mece =
                nom_concept::check_mece_with_required_axes(concept, &child_decls, &required_axes);

            // Collect stub notes (deduplicate across concepts).
            for note in &mece.stub_notes {
                if !all_stub_notes.contains(note) {
                    all_stub_notes.push(note.clone());
                }
            }

            mece_violations = mece
                .me_collisions
                .iter()
                .map(|col| MeceViolationRecord {
                    axis: col.axis.clone(),
                    bindings: col
                        .bindings
                        .iter()
                        .map(|b| ObjectiveBindingRecord {
                            source_concept: b.source_concept.clone(),
                            name: b.name.clone(),
                            axis: b.axis.clone(),
                        })
                        .collect(),
                })
                .collect();

            ce_unmet_for_concept = mece.ce_unmet;
        } else {
            mece_violations = vec![];
            ce_unmet_for_concept = vec![];
        }

        // ── map still_unresolved to UnresolvedRecord ──────────────────────────
        let unresolved: Vec<UnresolvedRecord> = still_unresolved
            .iter()
            .map(|u| UnresolvedRecord {
                kind: u.kind.clone(),
                word: u.word.clone(),
                matching: u.matching.clone(),
                referenced_from: u.referenced_from.clone(),
                typed_slot: u.typed_slot,
                confidence_threshold: u.confidence_threshold,
            })
            .collect();

        concept_manifests.push(ConceptManifest {
            name: concept.name.clone(),
            intent: concept.intent.clone(),
            objectives: concept.objectives.clone(),
            acceptance: concept.acceptance.clone(),
            exposes: concept.exposes.clone(),
            build_order,
            mece_violations,
            ce_unmet: ce_unmet_for_concept,
            unresolved,
        });
    }

    Ok(RepoManifest {
        repo_path,
        manifest_version: 1,
        generated_at_secs,
        concepts: concept_manifests,
        stub_notes: all_stub_notes,
    })
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nom_dict::Dict;
    use nom_dict::dict::upsert_concept_def;
    use std::path::Path;

    /// Minimal in-memory dict + one synthetic concept → build_manifest returns
    /// a well-formed RepoManifest.
    #[test]
    fn build_manifest_empty_graph_returns_version_1() {
        let tmp = std::env::temp_dir().join(format!(
            "nom-manifest-unit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp).expect("create tmp");

        let dict = Dict::open_in_memory().expect("open dict");

        // No concepts in DB → empty graph → empty concepts list.
        let manifest =
            build_manifest(Path::new("/nonexistent/myrepo"), &dict, None).expect("build_manifest");

        assert_eq!(manifest.manifest_version, 1);
        assert!(manifest.concepts.is_empty());
    }

    #[test]
    fn build_manifest_single_concept_no_words() {
        let tmp = std::env::temp_dir().join(format!(
            "nom-manifest-unit2-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp).expect("create tmp");

        let dict = Dict::open_in_memory().expect("open dict");
        use nom_dict::ConceptRow;

        // Insert one concept_def for repo_id "myrepo".
        let row = ConceptRow {
            name: "test_concept".to_string(),
            repo_id: "myrepo".to_string(),
            intent: "test the manifest builder".to_string(),
            index_into_db2: "[]".to_string(),
            exposes: "[]".to_string(),
            acceptance: "[\"it works\"]".to_string(),
            objectives: "[\"correctness\"]".to_string(),
            src_path: "test.nom".to_string(),
            src_hash: "abc".to_string(),
            body_hash: None,
        };
        upsert_concept_def(&dict, &row).expect("upsert concept");

        let manifest =
            build_manifest(Path::new("/some/path/myrepo"), &dict, None).expect("build_manifest");

        assert_eq!(manifest.manifest_version, 1);
        assert_eq!(manifest.concepts.len(), 1);

        let cm = &manifest.concepts[0];
        assert_eq!(cm.name, "test_concept");
        assert_eq!(cm.intent, "test the manifest builder");
        assert_eq!(cm.acceptance, vec!["it works"]);
        assert_eq!(cm.objectives, vec!["correctness"]);
        assert!(cm.mece_violations.is_empty());
        assert!(cm.unresolved.is_empty());
    }

    #[test]
    fn build_manifest_concept_filter_unknown_returns_err() {
        let tmp = std::env::temp_dir().join(format!(
            "nom-manifest-unit3-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp).expect("create tmp");
        let dict = Dict::open_in_memory().expect("open dict");

        let result = build_manifest(
            Path::new("/some/path/myrepo"),
            &dict,
            Some("does_not_exist"),
        );
        assert!(result.is_err(), "should error on unknown concept filter");
    }

    /// `collect_effects_from_repo` returns effects from files that have them
    /// and ignores files without effects.
    #[test]
    fn collect_effects_from_repo_handles_mixed_files() {
        let tmp = std::env::temp_dir().join(format!(
            "nom-manifest-effects-unit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp).expect("create tmp");

        // File with effects on one entity.
        std::fs::write(
            tmp.join("with_effects.nomtu"),
            "the function fetch_url is\n  \
             given a url of text, returns text.\n  \
             benefit cache_hit, fast_path.\n  \
             hazard timeout.\n",
        )
        .expect("write nomtu with effects");

        // File without effects.
        std::fs::write(
            tmp.join("no_effects.nomtu"),
            "the function list_dir is\n  given a path of text, returns text.\n",
        )
        .expect("write nomtu without effects");

        let map = collect_effects_from_repo(&tmp);

        // fetch_url has 2 effect groups (benefit + hazard).
        let fetch_effects = map.get("fetch_url").expect("fetch_url must be in map");
        assert_eq!(
            fetch_effects.len(),
            2,
            "expected 2 effect groups for fetch_url"
        );

        let benefit = fetch_effects
            .iter()
            .find(|e| e.valence == "benefit")
            .expect("benefit group must exist");
        assert!(benefit.names.contains(&"cache_hit".to_string()));
        assert!(benefit.names.contains(&"fast_path".to_string()));

        let hazard = fetch_effects
            .iter()
            .find(|e| e.valence == "hazard")
            .expect("hazard group must exist");
        assert!(hazard.names.contains(&"timeout".to_string()));

        // list_dir has no effects — should not be in the map.
        assert!(
            !map.contains_key("list_dir"),
            "list_dir must not be in effects map (no effects declared)"
        );
    }
}
