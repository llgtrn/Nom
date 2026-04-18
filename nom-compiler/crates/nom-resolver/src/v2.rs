//! Phase 4 B2 — v2 reference resolver.
//!
//! This module is the hash-identity counterpart to the legacy nomdict
//! resolver in [`crate`]. It takes a parsed [`SourceFile`] together with
//! a live [`NomDict`] and returns a `name -> hash` table that the
//! downstream rewrite pass (Task B3) uses to produce a fully hash-pinned
//! body.
//!
//! The resolver is intentionally minimal:
//!
//! * `use <name>` — look the word up in the dict. Zero matches →
//!   [`ResolveError::NotFound`]; more than one → [`ResolveError::Ambiguous`]
//!   with the list of candidate `(hash, source_path)` pairs.
//! * `use #<hex>@<name>` — verify the (possibly short) hex prefix maps to
//!   exactly one entry. Missing → [`ResolveError::UnknownHash`]. The
//!   `name` on the LHS of `@` becomes the local binding name.
//!
//! The module is additive: it neither modifies the legacy SQL resolver
//! nor the v1 `NomtuEntry` path. Task C wires it into the CLI.

use std::collections::HashMap;

use nom_ast::{SourceFile, Span, Statement, UseStmt};
use nom_dict::{Dict, dict::get_meta};
use thiserror::Error;

/// Errors surfaced by [`resolve_use_statements`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResolveError {
    /// Hash-pinned `use` referenced an id that is not present in the dict
    /// (no entry matches the full id, and no entry has the given prefix).
    #[error("unknown hash `{hash}` at span {span:?}")]
    UnknownHash { hash: String, span: Span },

    /// Bare `use <name>` did not match any entry.
    #[error("no dictionary entry named `{name}` at span {span:?}")]
    NotFound { name: String, span: Span },

    /// Bare `use <name>` matched multiple entries and needs a hash pin.
    /// `candidates` is `(hash, source_path_or_empty)` one per match, in
    /// deterministic (id-sorted) order.
    #[error("reference `{name}` is ambiguous: {} candidates", candidates.len())]
    Ambiguous {
        name: String,
        candidates: Vec<(String, String)>,
        span: Span,
    },

    /// Short hash-pin prefix matched more than one entry. The caller must
    /// supply a longer prefix or the full 64-char id.
    #[error("hash prefix `{hash}` is ambiguous: {} matches", candidates.len())]
    AmbiguousHash {
        hash: String,
        candidates: Vec<String>,
        span: Span,
    },
}

/// The result of resolving a single source file's `use` statements. Maps
/// local binding name → full 64-char hash id.
pub type ResolutionTable = HashMap<String, String>;

/// Resolve every `UseStmt` in `src` against `dict`. Returns a
/// `ResolutionTable` on success; the first resolution error on failure.
///
/// Only `UseImport::Single` is considered — the `{a, b}` and `*` forms
/// are module-level syntactic sugar that Task B does not yet address and
/// they are treated as a no-op (skipped).
pub fn resolve_use_statements(
    src: &SourceFile,
    dict: &Dict,
) -> Result<ResolutionTable, ResolveError> {
    let mut table: ResolutionTable = HashMap::new();
    for decl in &src.declarations {
        for stmt in &decl.statements {
            if let Statement::Use(use_stmt) = stmt {
                resolve_one_use(use_stmt, dict, &mut table)?;
            }
        }
    }
    Ok(table)
}

/// Handle a single `UseStmt`. Dispatches by `hash` presence and extracts
/// the local binding name from the `Single` import kind.
fn resolve_one_use(
    u: &UseStmt,
    dict: &Dict,
    table: &mut ResolutionTable,
) -> Result<(), ResolveError> {
    let local_name = match &u.imports {
        nom_ast::UseImport::Single(ident) => ident.name.clone(),
        // Multi / glob imports are not resolved here — Task B scope
        // covers single-name bindings only. Skip silently.
        _ => return Ok(()),
    };

    match &u.hash {
        Some(hex) => {
            let full = resolve_hash_pin(hex, dict, u.span)?;
            table.insert(local_name, full);
        }
        None => {
            let full = resolve_bare(&local_name, dict, u.span)?;
            table.insert(local_name, full);
        }
    }
    Ok(())
}

/// Resolve a bare (unpinned) `use <name>` by consulting `dict.find_by_word`.
#[allow(deprecated)]
fn resolve_bare(name: &str, dict: &Dict, span: Span) -> Result<String, ResolveError> {
    let entries =
        nom_dict::find_by_word(dict, name).map_err(|e| internal_lookup_error(name, span, &e))?;
    match entries.len() {
        0 => Err(ResolveError::NotFound {
            name: name.to_string(),
            span,
        }),
        1 => Ok(entries.into_iter().next().unwrap().id),
        _ => {
            let mut candidates: Vec<(String, String)> = entries
                .iter()
                .map(|e| {
                    // meta(key=source_path) may not exist; fall back to empty string.
                    let source = get_meta(dict, &e.id)
                        .ok()
                        .and_then(|rows| {
                            rows.into_iter()
                                .find(|(k, _)| k == "source_path")
                                .map(|(_, v)| v)
                        })
                        .unwrap_or_default();
                    (e.id.clone(), source)
                })
                .collect();
            candidates.sort();
            Err(ResolveError::Ambiguous {
                name: name.to_string(),
                candidates,
                span,
            })
        }
    }
}

/// Resolve a `#<hex>` pin. Accepts a full 64-char id or any prefix
/// ≥ 1 char — the DB lookup decides uniqueness.
#[allow(deprecated)]
fn resolve_hash_pin(hex: &str, dict: &Dict, span: Span) -> Result<String, ResolveError> {
    // Fast path: full-length id lookup via get_entry.
    if hex.len() == 64 {
        return match nom_dict::get_entry(dict, hex)
            .map_err(|e| internal_lookup_error(hex, span, &e))?
        {
            Some(entry) => Ok(entry.id),
            None => Err(ResolveError::UnknownHash {
                hash: hex.to_string(),
                span,
            }),
        };
    }
    // Prefix path: scan ids with a LIKE query.
    let pattern = format!("{hex}%");
    let mut stmt = dict
        .entities
        .prepare_cached("SELECT id FROM entries WHERE id LIKE ?1 ORDER BY id")
        .map_err(|e| internal_lookup_error(hex, span, &e))?;
    let ids: Vec<String> = stmt
        .query_map([pattern], |row| row.get::<_, String>(0))
        .map_err(|e| internal_lookup_error(hex, span, &e))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| internal_lookup_error(hex, span, &e))?;
    match ids.len() {
        0 => Err(ResolveError::UnknownHash {
            hash: hex.to_string(),
            span,
        }),
        1 => Ok(ids.into_iter().next().unwrap()),
        _ => Err(ResolveError::AmbiguousHash {
            hash: hex.to_string(),
            candidates: ids,
            span,
        }),
    }
}

/// Adapter: a DB error during lookup is surfaced as `UnknownHash` so the
/// caller sees a single error shape. We log nothing yet — the CLI wiring
/// in Task C will attach proper diagnostics.
fn internal_lookup_error(hex_or_name: &str, span: Span, _e: &impl std::fmt::Debug) -> ResolveError {
    ResolveError::UnknownHash {
        hash: hex_or_name.to_string(),
        span,
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nom_ast::{
        Classifier, Declaration, Identifier, SourceFile, Span, Statement, UseImport, UseStmt,
    };
    use nom_dict::dict::{add_ref, closure, upsert_entry};
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    fn make_entry(id: &str, word: &str) -> Entry {
        Entry {
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
            created_at: "2026-04-12T00:00:00Z".into(),
            updated_at: None,
        }
    }

    fn seeded_dict(entries: &[Entry]) -> Dict {
        let d = Dict::open_in_memory().unwrap();
        for e in entries {
            upsert_entry(&d, e).unwrap();
        }
        d
    }

    fn ident(name: &str) -> Identifier {
        Identifier::new(name, Span::default())
    }

    fn parse_src(src: &str) -> SourceFile {
        let use_line = src
            .lines()
            .find_map(|line| line.trim().strip_prefix("use "))
            .expect("expected a use line in test source")
            .trim();

        let (hash, name) = if let Some(pinned) = use_line.strip_prefix('#') {
            let (hash, name) = pinned
                .split_once('@')
                .expect("expected #hash@name test syntax");
            (Some(hash.to_string()), name.to_string())
        } else {
            (None, use_line.to_string())
        };

        SourceFile {
            path: None,
            locale: None,
            declarations: vec![Declaration {
                classifier: Classifier::System,
                name: ident("main"),
                statements: vec![Statement::Use(UseStmt {
                    path: Vec::new(),
                    imports: UseImport::Single(ident(&name)),
                    hash,
                    span: Span::default(),
                })],
                span: Span::default(),
            }],
        }
    }

    #[test]
    fn bare_use_resolves_unique_entry() {
        let dict = seeded_dict(&[make_entry(&"a".repeat(64), "greet")]);
        let sf = parse_src("system main\n  use greet\n");
        let table = resolve_use_statements(&sf, &dict).unwrap();
        assert_eq!(table.get("greet").unwrap(), &"a".repeat(64));
    }

    #[test]
    fn hash_pinned_use_resolves_full_hash() {
        let id = "b".repeat(64);
        let dict = seeded_dict(&[make_entry(&id, "greet")]);
        let sf = parse_src(&format!("system main\n  use #{id}@greet\n"));
        let table = resolve_use_statements(&sf, &dict).unwrap();
        assert_eq!(table.get("greet").unwrap(), &id);
    }

    #[test]
    fn hash_pinned_use_resolves_prefix() {
        // An 8-char prefix is enough when it's unique.
        let id = format!("{}{}", "c", "c".repeat(63));
        let dict = seeded_dict(&[make_entry(&id, "greet")]);
        let prefix = &id[..8];
        let sf = parse_src(&format!("system main\n  use #{prefix}@greet\n"));
        let table = resolve_use_statements(&sf, &dict).unwrap();
        assert_eq!(table.get("greet").unwrap(), &id);
    }

    #[test]
    fn ambiguous_bare_use_returns_candidates() {
        let dict = seeded_dict(&[
            make_entry(&format!("{}{}", "1", "a".repeat(63)), "greet"),
            make_entry(&format!("{}{}", "2", "a".repeat(63)), "greet"),
        ]);
        let sf = parse_src("system main\n  use greet\n");
        let err = resolve_use_statements(&sf, &dict).unwrap_err();
        match err {
            ResolveError::Ambiguous {
                name, candidates, ..
            } => {
                assert_eq!(name, "greet");
                assert_eq!(candidates.len(), 2);
                // ordered by id
                assert!(candidates[0].0.starts_with('1'));
                assert!(candidates[1].0.starts_with('2'));
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn missing_bare_use_returns_not_found() {
        let dict = seeded_dict(&[]);
        let sf = parse_src("system main\n  use greet\n");
        let err = resolve_use_statements(&sf, &dict).unwrap_err();
        assert!(matches!(err, ResolveError::NotFound { ref name, .. } if name == "greet"));
    }

    #[test]
    fn missing_hash_pin_returns_unknown_hash() {
        let dict = seeded_dict(&[]);
        let hex = "deadbeefcafef00d";
        let sf = parse_src(&format!("system main\n  use #{hex}@greet\n"));
        let err = resolve_use_statements(&sf, &dict).unwrap_err();
        assert!(matches!(err, ResolveError::UnknownHash { ref hash, .. } if hash == hex));
    }

    // ── B4: closure walker integration ─────────────────────────────────────

    #[test]
    fn closure_visits_three_entries_in_bfs_order() {
        // Seed a tiny dict: A -> B -> C. The closure of A must contain all
        // three in BFS-from-root order. This is an end-to-end smoke test
        // that `nom-dict::closure` works with the v2 schema as the resolver
        // consumes it.
        let a = format!("{}{}", "a", "0".repeat(63));
        let b = format!("{}{}", "b", "0".repeat(63));
        let c = format!("{}{}", "c", "0".repeat(63));
        let dict = Dict::open_in_memory().unwrap();
        for (id, word) in [(&a, "a"), (&b, "b"), (&c, "c")] {
            upsert_entry(&dict, &make_entry(id, word)).unwrap();
        }
        add_ref(&dict, &a, &b).unwrap();
        add_ref(&dict, &b, &c).unwrap();

        let closure = closure(&dict, &a).unwrap();
        // Set semantics: every node reachable from A, once.
        let set: std::collections::HashSet<_> = closure.iter().cloned().collect();
        assert_eq!(set.len(), 3);
        assert!(set.contains(&a));
        assert!(set.contains(&b));
        assert!(set.contains(&c));

        // BFS order: root first, then direct children, then grandchildren.
        assert_eq!(closure[0], a, "root must be first");
        assert_eq!(closure[1], b, "direct child next");
        assert_eq!(closure[2], c, "grandchild last");
    }
}
