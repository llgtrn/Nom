//! `nom store sync` subcommand — walk a repo and upsert `.nom`/`.nomtu` files.

use std::path::Path;

use nom_concept::{
    NomtuItem, parse_nom as concept_parse_nom, parse_nomtu,
};
use nom_dict::{ConceptRow, NomDict, WordV2Row};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use super::open_dict;

/// Statistics returned by `sync_repo`.
#[derive(Debug, Default)]
pub struct SyncStats {
    pub concepts: usize,
    pub words: usize,
    pub entities: usize,
    pub compositions: usize,
    pub files: usize,
}

/// Directories to skip during the repo walk (platform-independent names).
const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", "dist", "build"];

/// Core sync logic: walk `repo`, parse `.nom` and `.nomtu` files, upsert
/// rows into DB1 (`concept_defs`) and DB2-v2 (`words_v2`) of `dict`.
///
/// Parse errors are surfaced per-file (collected into `errors`) but do not
/// abort the walk; the caller decides whether to treat them as fatal.
///
/// # Hashing
///
/// Entity and composition rows use `sha256(serde_json::to_vec(&decl)?)` as
/// the content-addressed `hash`.  This is a scaffold only — a proper
/// canonical-bytes serialiser would live in `nom-concept` or `nom-types`
/// and be called here instead.
/// TODO: replace `serde_json` hash with a proper deterministic canonicaliser
/// once `nom-concept::canonical_bytes()` is implemented (doc 08 §5.1).
pub fn sync_repo(
    repo: &Path,
    dict: &NomDict,
) -> (SyncStats, Vec<String>) {
    let mut stats = SyncStats::default();
    let mut errors: Vec<String> = Vec::new();

    let repo_id = repo
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_owned();

    for entry in WalkDir::new(repo)
        .into_iter()
        .filter_entry(|e| {
            // Skip well-known non-source directories.
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !SKIP_DIRS.contains(&name);
                }
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Relative path for storage — fall back to full path if we can't
        // strip the prefix (e.g. when repo is relative and path is absolute).
        let rel = path
            .strip_prefix(repo)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));

        stats.files += 1;

        match ext {
            "nomtu" => {
                let src = match std::fs::read_to_string(path) {
                    Ok(s) => s,
                    Err(e) => {
                        errors.push(format!("{rel}: io error: {e}"));
                        continue;
                    }
                };

                let nomtu = match parse_nomtu(&src) {
                    Ok(f) => f,
                    Err(e) => {
                        errors.push(format!("{rel}: parse error: {e}"));
                        continue;
                    }
                };

                for item in &nomtu.items {
                    match item {
                        NomtuItem::Entity(entity) => {
                            // TODO: replace with nom-concept canonical_bytes once
                            // implemented (doc 08 §5.1); serde_json is deterministic
                            // for simple structs but not guaranteed across versions.
                            let canon = match serde_json::to_vec(entity) {
                                Ok(b) => b,
                                Err(e) => {
                                    errors.push(format!("{rel}: serialise error: {e}"));
                                    continue;
                                }
                            };
                            let hash = format!("{:x}", Sha256::digest(&canon));

                            let contracts_json = match serde_json::to_string(&entity.contracts) {
                                Ok(j) => Some(j),
                                Err(_) => None,
                            };

                            let row = WordV2Row {
                                hash,
                                word: entity.word.clone(),
                                kind: entity.kind.clone(),
                                signature: Some(entity.signature.clone()),
                                contracts: contracts_json,
                                body_kind: None,
                                body_size: None,
                                origin_ref: None,
                                bench_ids: None,
                                authored_in: Some(rel.clone()),
                                composed_of: None,
                            };

                            if let Err(e) = dict.upsert_word_v2(&row) {
                                errors.push(format!(
                                    "{rel}: upsert entity `{}`: {e}",
                                    entity.word
                                ));
                                continue;
                            }
                            stats.entities += 1;
                            stats.words += 1;
                        }

                        NomtuItem::Composition(comp) => {
                            let canon = match serde_json::to_vec(comp) {
                                Ok(b) => b,
                                Err(e) => {
                                    errors.push(format!("{rel}: serialise error: {e}"));
                                    continue;
                                }
                            };
                            let hash = format!("{:x}", Sha256::digest(&canon));

                            // Collect resolved hashes from EntityRefs where available.
                            let composes_hashes: Vec<String> = comp
                                .composes
                                .iter()
                                .filter_map(|r| r.hash.clone())
                                .collect();
                            let composed_of = if composes_hashes.is_empty() {
                                // Store word refs instead so the row isn't empty.
                                let words: Vec<String> =
                                    comp.composes.iter().map(|r| r.word.clone()).collect();
                                serde_json::to_string(&words).ok()
                            } else {
                                serde_json::to_string(&composes_hashes).ok()
                            };

                            let row = WordV2Row {
                                hash,
                                word: comp.word.clone(),
                                kind: "module".to_string(),
                                signature: None,
                                contracts: None,
                                body_kind: None,
                                body_size: None,
                                origin_ref: None,
                                bench_ids: None,
                                authored_in: Some(rel.clone()),
                                composed_of,
                            };

                            if let Err(e) = dict.upsert_word_v2(&row) {
                                errors.push(format!(
                                    "{rel}: upsert composition `{}`: {e}",
                                    comp.word
                                ));
                                continue;
                            }
                            stats.compositions += 1;
                            stats.words += 1;
                        }
                    }
                }
            }

            "nom" => {
                let src = match std::fs::read_to_string(path) {
                    Ok(s) => s,
                    Err(e) => {
                        errors.push(format!("{rel}: io error: {e}"));
                        continue;
                    }
                };

                let nom_file = match concept_parse_nom(&src) {
                    Ok(f) => f,
                    Err(e) => {
                        errors.push(format!("{rel}: parse error: {e}"));
                        continue;
                    }
                };

                let src_hash = format!("{:x}", Sha256::digest(src.as_bytes()));

                for concept in &nom_file.concepts {
                    let index_json = serde_json::to_string(&concept.index)
                        .unwrap_or_else(|_| "[]".to_string());
                    let exposes_json = serde_json::to_string(&concept.exposes)
                        .unwrap_or_else(|_| "[]".to_string());
                    let acceptance_json = serde_json::to_string(&concept.acceptance)
                        .unwrap_or_else(|_| "[]".to_string());
                    let objectives_json = serde_json::to_string(&concept.objectives)
                        .unwrap_or_else(|_| "[]".to_string());

                    let row = ConceptRow {
                        name: concept.name.clone(),
                        repo_id: repo_id.clone(),
                        intent: concept.intent.clone(),
                        index_into_db2: index_json,
                        exposes: exposes_json,
                        acceptance: acceptance_json,
                        objectives: objectives_json,
                        src_path: rel.clone(),
                        src_hash: src_hash.clone(),
                        body_hash: None,
                    };

                    if let Err(e) = dict.upsert_concept_def(&row) {
                        errors.push(format!(
                            "{rel}: upsert concept `{}`: {e}",
                            concept.name
                        ));
                        continue;
                    }
                    stats.concepts += 1;
                }
            }

            _ => {
                // Not a .nom or .nomtu file; don't count as a processed file.
                stats.files -= 1;
            }
        }
    }

    (stats, errors)
}

/// CLI entry point: `nom store sync <repo>`.
pub fn cmd_store_sync(repo: &Path, dict: &Path) -> i32 {
    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };

    let (stats, errors) = sync_repo(repo, &dict_db);

    // Report per-file errors but continue; return 1 if any errors occurred.
    for msg in &errors {
        eprintln!("nom: sync error: {msg}");
    }

    println!(
        "Synced {} concept(s) and {} word(s) ({} entities, {} compositions) from {} files.",
        stats.concepts, stats.words, stats.entities, stats.compositions, stats.files
    );

    if errors.is_empty() { 0 } else { 1 }
}
