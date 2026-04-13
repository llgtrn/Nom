//! Handlers for `nom build` subcommands.
//!
//! `nom build status <repo>` loads the concept closure from the DB,
//! attempts to resolve unresolved refs via the stub resolver, and reports
//! build-readiness per concept.  It is read-only — no compilation or
//! codegen is performed.
//!
//! `nom build status <repo> --write-locks` additionally rewrites every
//! `.nom` source file that still has prose-matching refs (no `@hash`) to
//! insert the resolved `@<hash>` after the word name.  This is idempotent:
//! if the `@hash` is already present the file is not modified.
//!
//! `nom build manifest <repo>` emits a JSON build manifest derived from
//! the closure walker + stub resolver + MECE pipeline.  All manifest logic
//! lives in `manifest.rs`; this function is the thin CLI adapter.

use std::collections::HashMap;
use std::path::Path;

use nom_dict::NomDict;

use crate::store::{ResolvedRef, materialize_concept_graph_from_db, resolve_closure};

/// CLI entry point: `nom build status <repo> [--dict <path>] [--concept <name>] [--write-locks]`.
///
/// Exit codes:
///   0 — all concepts in scope resolved cleanly (zero still-unresolved refs).
///   1 — at least one concept has unresolved refs, a DB error occurred, or
///       `--concept <name>` was given but no such concept was found in the repo.
pub fn cmd_build_status(
    repo: &Path,
    dict: &Path,
    concept_filter: Option<&str>,
    write_locks: bool,
) -> i32 {
    // ── open dict ────────────────────────────────────────────────────────────
    let dict_db = match open_dict_in_place(dict) {
        Some(d) => d,
        None => return 1,
    };

    // ── derive repo_id (basename of repo path) ───────────────────────────────
    let repo_id = repo
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // ── materialise ConceptGraph from DB ─────────────────────────────────────
    let graph = match materialize_concept_graph_from_db(&dict_db, repo_id) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("nom build status: cannot materialise graph: {e}");
            return 1;
        }
    };

    if graph.concepts.is_empty() {
        println!("nom build status: no concepts found for repo `{repo_id}`.");
        println!("  Run `nom store sync <repo> --dict <path>` first.");
        // Treat as clean: nothing to fail.
        return 0;
    }

    // ── apply --concept filter ────────────────────────────────────────────────
    let concepts_in_scope: Vec<&nom_concept::ConceptDecl> = if let Some(name) = concept_filter {
        let filtered: Vec<&nom_concept::ConceptDecl> = graph
            .concepts
            .iter()
            .filter(|c| c.name == name)
            .collect();
        if filtered.is_empty() {
            eprintln!(
                "nom build status: concept `{name}` not found in repo `{repo_id}`."
            );
            eprintln!("  Available concepts:");
            for c in &graph.concepts {
                eprintln!("    {}", c.name);
            }
            return 1;
        }
        filtered
    } else {
        graph.concepts.iter().collect()
    };

    // ── walk closure + resolve for each concept ───────────────────────────────
    let mut any_unresolved = false;

    // Track resolved refs per source file for write-lock pass.
    // Map: source_file_path → Vec<ResolvedRef>
    let mut file_resolved: HashMap<String, Vec<ResolvedRef>> = HashMap::new();

    for concept in concepts_in_scope {
        // Walk the closure.  Cycle errors are printed but don't fail other concepts.
        let closure = match graph.closure(&concept.name) {
            Ok(c) => c,
            Err(nom_concept::ClosureError::Cycle { path }) => {
                eprintln!(
                    "nom build status: [{}] cycle detected: {path}",
                    concept.name
                );
                any_unresolved = true;
                continue;
            }
            Err(e) => {
                eprintln!("nom build status: [{}] closure error: {e}", concept.name);
                any_unresolved = true;
                continue;
            }
        };

        // Run the stub resolver against words_v2.
        let (resolved, still_unresolved, stats) = resolve_closure(&closure, &dict_db);

        let total_words = closure.word_hashes.len() + stats.resolved + stats.still_unresolved;

        println!("concept: {}", concept.name);
        println!(
            "  words resolved: {}/{total_words}",
            stats.resolved
        );
        println!("  word hashes in closure: {}", closure.word_hashes.len());

        if stats.ambiguous > 0 {
            println!("  ambiguous (picked smallest hash): {}", stats.ambiguous);
            for r in resolved.iter().filter(|r| !r.alternatives.is_empty()) {
                println!(
                    "    `{}` → {} (alternatives: {})",
                    r.word,
                    &r.hash[..16.min(r.hash.len())],
                    r.alternatives
                        .iter()
                        .map(|h| &h[..16.min(h.len())])
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        if still_unresolved.is_empty() {
            println!("  status: all clear");
        } else {
            any_unresolved = true;
            println!("  status: {} unresolved ref(s)", still_unresolved.len());
            for uref in &still_unresolved {
                let kind_str = uref
                    .kind
                    .as_deref()
                    .map(|k| format!("{k} "))
                    .unwrap_or_default();
                let matching_str = uref
                    .matching
                    .as_deref()
                    .map(|m| format!(" matching \"{m}\""))
                    .unwrap_or_default();
                println!(
                    "    unresolved: {kind_str}`{}`{matching_str} (from `{}`)",
                    uref.word, uref.referenced_from
                );
            }
        }

        // ── MECE objectives check ─────────────────────────────────────────────
        // Collect child concepts: any Uses clause whose EntityRef has kind "concept".
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

        if !child_concept_names.is_empty() || !concept.objectives.is_empty() {
            let child_decls: Vec<&nom_concept::ConceptDecl> = child_concept_names
                .iter()
                .filter_map(|name| graph.concepts.iter().find(|c| &c.name == name))
                .collect();

            let mece = nom_concept::check_mece(concept, &child_decls);

            // Print the objectives union.
            let union_str: Vec<String> = mece
                .union
                .iter()
                .map(|b| format!("{}:{}", b.axis, b.source_concept))
                .collect();
            println!("  objectives union: [{}]", union_str.join(", "));

            // Print ME violations and mark build failed.
            for collision in &mece.me_collisions {
                let offenders: Vec<&str> = collision
                    .bindings
                    .iter()
                    .map(|b| b.source_concept.as_str())
                    .collect();
                println!(
                    "  MECE-ME violation: axis '{}' set by [{}]",
                    collision.axis,
                    offenders.join(", ")
                );
                any_unresolved = true;
            }

            // Print stub note once per concept report.
            for note in &mece.stub_notes {
                println!("  note: {note}");
            }
        }

        println!();

        // Collect resolved refs for write-lock pass, keyed by `referenced_from` source file.
        if write_locks {
            for rref in resolved {
                // `referenced_from` is the source file path (relative or absolute).
                // We need the unresolved ref's `referenced_from` to know which file to patch.
                // The resolved ref doesn't carry `referenced_from`; look it up via the closure
                // unresolved list which was consumed by resolve_closure.  Re-derive it by
                // re-scanning the original `closure.unresolved` that produced this ref.
                // Since resolve_closure consumed the vector, we reconstruct the mapping:
                // this word in `rref.word` → the unresolved ref in closure.unresolved.
                // We already walked the closure above, so we can match by word name.
                // The source file that declared this ref is in `uref.referenced_from`.
                let source_file = closure
                    .unresolved
                    .iter()
                    .find(|u| u.word == rref.word)
                    .map(|u| u.referenced_from.clone())
                    .unwrap_or_default();

                if !source_file.is_empty() {
                    file_resolved
                        .entry(source_file)
                        .or_default()
                        .push(rref);
                }
            }
        }
    }

    // ── write-lock pass ───────────────────────────────────────────────────────
    if write_locks {
        let mut total_locks_written = 0usize;
        let mut files_patched = 0usize;

        for (rel_source, refs) in &file_resolved {
            // Build absolute path: if rel_source is already absolute, use as-is;
            // otherwise join with repo.
            let source_path = {
                let p = Path::new(rel_source);
                if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    repo.join(p)
                }
            };

            let original = match std::fs::read_to_string(&source_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "nom build status: cannot read {} for lock writeback: {e}",
                        source_path.display()
                    );
                    continue;
                }
            };

            let (patched, n) = apply_hash_locks(&original, refs);
            if n > 0 {
                if let Err(e) = std::fs::write(&source_path, &patched) {
                    eprintln!(
                        "nom build status: cannot write lock to {}: {e}",
                        source_path.display()
                    );
                } else {
                    total_locks_written += n;
                    files_patched += 1;
                }
            }
        }

        println!("Wrote {total_locks_written} hash lock(s) to {files_patched} file(s).");
    }

    if any_unresolved { 1 } else { 0 }
}

/// Rewrite `source` text by inserting `@<hash>` after each resolved word name
/// that does not already have `@` pinned.
///
/// For each `ResolvedRef { word, hash, .. }`, scans the source line-by-line
/// for lines that contain `the <kind> <word>` (no `@` immediately after
/// `<word>`) and splices `@<hash>` in.
///
/// Returns `(patched_source, count_of_insertions)`.
///
/// This is idempotent: if the line already contains `<word>@` it is skipped.
pub fn apply_hash_locks(source: &str, refs: &[ResolvedRef]) -> (String, usize) {
    if refs.is_empty() {
        return (source.to_owned(), 0);
    }

    let mut result = String::with_capacity(source.len() + refs.len() * 70);
    let mut count = 0usize;

    for line in source.lines() {
        let mut patched_line = line.to_owned();

        for rref in refs {
            let word = &rref.word;
            let hash = &rref.hash;

            // Skip if the word already has @<hash> pinned anywhere on this line.
            if patched_line.contains(&format!("{word}@")) {
                continue;
            }

            // Look for `<article> <kind> <word>` pattern.  The word must be
            // followed by whitespace, punctuation, end-of-line, or a matching
            // keyword.  We scan for the token boundary manually to avoid a
            // regex dep on the single caller site (regex is available via
            // workspace but we keep the rewrite pure for testability).
            //
            // Articles: `the` (English) and `cai` (Vietnamese locale-pack,
            //   motivation 02 alias for the classifier article).
            // Kind words: English names + Vietnamese ASCII aliases (same
            //   motivation 02 locale-pack):
            //   function / ham, module / mo_dun, concept / khai_niem,
            //   screen / man_hinh, data, event, media.
            let needles: &[(&str, &str)] = &[
                ("the", "function"),
                ("the", "module"),
                ("the", "concept"),
                ("the", "screen"),
                ("the", "data"),
                ("the", "event"),
                ("the", "media"),
                // Vietnamese ASCII aliases (motivation 02 locale-pack)
                ("cai", "ham"),
                ("cai", "mo_dun"),
                ("cai", "khai_niem"),
                ("cai", "man_hinh"),
                ("cai", "function"),
                ("cai", "module"),
                ("cai", "concept"),
                // Vietnamese diacritic aliases (motivation 02 locale-pack).
                // Multi-word kind nouns use underscore-joined diacritic form
                // (e.g. `mô_đun`, NOT `mô đun`) — deliberate design choice to
                // keep token boundaries unambiguous without whitespace lookahead.
                ("cái", "hàm"),
                ("cái", "mô_đun"),
                ("cái", "khái_niệm"),
                ("cái", "màn_hình"),
                ("cái", "dữ_liệu"),
                ("cái", "sự_kiện"),
                ("cái", "phương_tiện"),
            ];
            'needle_loop: for (article, kind) in needles {
                let needle = format!("{article} {kind} {word}");
                if let Some(pos) = patched_line.find(&needle) {
                    let after_pos = pos + needle.len();
                    // Confirm what follows is not `@` (already pinned) or an
                    // alnum/underscore char that would mean a longer word.
                    let next_char = patched_line[after_pos..].chars().next();
                    let already_pinned = next_char == Some('@');
                    let word_continues = next_char
                        .map(|c| c.is_alphanumeric() || c == '_')
                        .unwrap_or(false);

                    if !already_pinned && !word_continues {
                        patched_line.insert_str(after_pos, &format!("@{hash}"));
                        count += 1;
                        break 'needle_loop; // one insertion per ref per line is enough
                    }
                }
            }
        }

        result.push_str(&patched_line);
        result.push('\n');
    }

    // Preserve whether the original ended with a newline or not.
    if !source.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    (result, count)
}

/// CLI entry point: `nom build manifest <repo> [--dict <p>] [--concept <n>] [--out <f>] [--pretty]`.
///
/// Exit codes:
///   0 — all concepts resolved cleanly and no MECE violations.
///   1 — at least one concept has unresolved refs, a MECE violation, or an error.
pub fn cmd_build_manifest(
    repo: &Path,
    dict: &Path,
    concept_filter: Option<&str>,
    out: Option<&Path>,
    pretty: bool,
) -> i32 {
    let dict_db = match open_dict_in_place(dict) {
        Some(d) => d,
        None => return 1,
    };

    let manifest = match crate::manifest::build_manifest(repo, &dict_db, concept_filter) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("nom build manifest: {e}");
            return 1;
        }
    };

    let json = if pretty {
        match serde_json::to_string_pretty(&manifest) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("nom build manifest: serialise error: {e}");
                return 1;
            }
        }
    } else {
        match serde_json::to_string(&manifest) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("nom build manifest: serialise error: {e}");
                return 1;
            }
        }
    };

    if let Some(path) = out {
        if let Err(e) = std::fs::write(path, &json) {
            eprintln!("nom build manifest: cannot write {}: {e}", path.display());
            return 1;
        }
    } else {
        println!("{json}");
    }

    // Exit 1 if any concept has unresolved refs or MECE violations (mirrors `status`).
    let any_issue = manifest.concepts.iter().any(|c| {
        !c.unresolved.is_empty() || !c.mece_violations.is_empty()
    });

    if any_issue { 1 } else { 0 }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn open_dict_in_place(dict: &Path) -> Option<NomDict> {
    // If dict points directly at a .db file, use open_in_place; otherwise
    // open the directory root (same logic as store::open_dict).
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

// ── unit tests for apply_hash_locks ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ref(word: &str, hash: &str) -> ResolvedRef {
        ResolvedRef {
            word: word.to_owned(),
            kind: Some("module".to_owned()),
            hash: hash.to_owned(),
            alternatives: vec![],
        }
    }

    #[test]
    fn apply_hash_locks_inserts_hash_after_word() {
        let source = r#"the concept authentication_demo is
  uses the module auth_session_compose_demo matching "validate then issue session".
"#;
        let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let refs = vec![make_ref("auth_session_compose_demo", hash)];
        let (patched, count) = apply_hash_locks(source, &refs);
        assert_eq!(count, 1);
        assert!(
            patched.contains(&format!("auth_session_compose_demo@{hash}")),
            "expected @hash in patched: {patched}"
        );
        // The rest of the line should be preserved.
        assert!(
            patched.contains("matching \"validate then issue session\""),
            "matching clause must be preserved: {patched}"
        );
    }

    #[test]
    fn apply_hash_locks_is_idempotent() {
        let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let source = format!(
            "  uses the module auth_session_compose_demo@{hash} matching \"x\".\n"
        );
        let refs = vec![make_ref("auth_session_compose_demo", hash)];
        let (patched, count) = apply_hash_locks(&source, &refs);
        assert_eq!(count, 0, "already-pinned ref must not be modified");
        assert_eq!(patched, source, "source must be unchanged");
    }

    #[test]
    fn apply_hash_locks_preserves_other_lines() {
        let source = "the concept foo is\n  intended to do bar.\n  uses the function baz.\n";
        let hash = "deadbeef00000000deadbeef00000000deadbeef00000000deadbeef00000000";
        let refs = vec![make_ref("baz", hash)];
        let (patched, count) = apply_hash_locks(source, &refs);
        assert_eq!(count, 1);
        assert!(patched.contains("the concept foo is"), "other lines preserved");
        assert!(patched.contains("intended to do bar"), "other lines preserved");
        assert!(patched.contains(&format!("the function baz@{hash}")));
    }

    // ── Vietnamese locale-pack regression tests ───────────────────────────────

    #[test]
    fn apply_hash_locks_vn_cai_ham_inserts_hash() {
        // Regression: `cai ham foo khop "..."` must have @hash spliced after `foo`.
        let source = "     cai ham foo khop \"read text from a workspace path\",\n";
        let hash = "cafebabe00000000cafebabe00000000cafebabe00000000cafebabe00000000";
        let refs = vec![make_ref("foo", hash)];
        let (patched, count) = apply_hash_locks(source, &refs);
        assert_eq!(count, 1, "VN cai ham line must receive hash insertion");
        assert!(
            patched.contains(&format!("foo@{hash}")),
            "expected foo@hash in patched VN line: {patched}"
        );
        // The matching clause must survive unchanged.
        assert!(
            patched.contains("khop \"read text from a workspace path\""),
            "khop clause must be preserved: {patched}"
        );
    }

    #[test]
    fn apply_hash_locks_vn_cai_ham_idempotent() {
        // If `cai ham foo@<hash>` is already pinned, no second insertion.
        let hash = "cafebabe00000000cafebabe00000000cafebabe00000000cafebabe00000000";
        let source = format!("     cai ham foo@{hash} khop \"x\",\n");
        let refs = vec![make_ref("foo", hash)];
        let (patched, count) = apply_hash_locks(&source, &refs);
        assert_eq!(count, 0, "already-pinned VN line must not be modified");
        assert_eq!(patched, source, "source must be unchanged");
    }

    #[test]
    fn apply_hash_locks_diacritic_cai_ham_inserts_hash() {
        // `cái hàm read_file khớp "..."` must have @hash spliced after `read_file`.
        let source = "     cái hàm read_file khớp \"read text from a workspace path\",\n";
        let hash = "abc123def456abc123def456abc123def456abc123def456abc123def456abc1";
        let refs = vec![make_ref("read_file", hash)];
        let (patched, count) = apply_hash_locks(source, &refs);
        assert_eq!(count, 1, "diacritic cái hàm line must receive hash insertion");
        assert!(
            patched.contains(&format!("read_file@{hash}")),
            "expected read_file@hash in patched diacritic VN line: {patched}"
        );
        // The khớp clause must survive unchanged.
        assert!(
            patched.contains("khớp \"read text from a workspace path\""),
            "khớp clause must be preserved: {patched}"
        );
    }

    #[test]
    fn apply_hash_locks_vn_concept_line_not_touched() {
        // A `cai khai_niem <name>` declaration line must NOT get a hash
        // spliced when we are looking for a function named differently.
        let source = "cai khai_niem agent_safety_policy muc_dich constrain what an agent may do.\n";
        let hash = "aaaa000000000000aaaa000000000000aaaa000000000000aaaa000000000000";
        // We are resolving `read_file`, not `agent_safety_policy`.
        let refs = vec![make_ref("read_file", hash)];
        let (patched, count) = apply_hash_locks(source, &refs);
        assert_eq!(count, 0, "concept declaration line must not be patched for a different word");
        assert_eq!(patched, source, "source must be unchanged");
    }
}
