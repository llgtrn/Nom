//! Extraction pipeline: parse source files, extract UIR entities, produce atoms.
//!
//! Combines parsing (tree-sitter), entity extraction (UIR), and atom extraction
//! into a single crate. Also provides directory scanning for batch extraction.

pub mod extract;
pub mod scan;

use anyhow::{Context, Result, bail};
use tree_sitter::{Language, Parser, Tree};

use nom_types::{Atom, AtomKind, AtomSignature, UirEntity};

// ── Parsing ──────────────────────────────────────────────────────────

/// All languages the parser recognizes (detection works for all).
pub fn supported_languages() -> &'static [&'static str] {
    &[
        "rust",
        "typescript",
        "javascript",
        "python",
        "c",
        "cpp",
        "go",
        "nix",
        "yaml",
        "json",
        "toml",
        "shell",
        "powershell",
        "dockerfile",
    ]
}

/// Languages with tree-sitter grammars (parsing + extraction works for these).
pub fn parseable_languages() -> &'static [&'static str] {
    &[
        "rust",
        "typescript",
        "javascript",
        "python",
        "c",
        "cpp",
        "go",
    ]
}

/// Get the tree-sitter Language for a given language name.
pub fn language_for(name: &str) -> Result<Language> {
    match name {
        "rust" => Ok(tree_sitter_rust::LANGUAGE.into()),
        "typescript" => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "javascript" => Ok(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "python" => Ok(tree_sitter_python::LANGUAGE.into()),
        "c" => Ok(tree_sitter_c::LANGUAGE.into()),
        "cpp" => Ok(tree_sitter_cpp::LANGUAGE.into()),
        "go" => Ok(tree_sitter_go::LANGUAGE.into()),
        other => bail!("no tree-sitter grammar for language: {other}"),
    }
}

/// Parse source code into a tree-sitter AST.
pub fn parse_source(source: &str, language: &str) -> Result<Tree> {
    let lang = language_for(language)?;
    let mut parser = Parser::new();
    parser
        .set_language(&lang)
        .context("failed to set parser language")?;
    parser
        .parse(source, None)
        .context("tree-sitter parse returned None")
}

/// Summary statistics from a parsed tree.
#[derive(Debug, Clone)]
pub struct ParseStats {
    pub language: String,
    pub total_nodes: usize,
    pub named_nodes: usize,
    pub depth: usize,
    pub has_errors: bool,
}

/// Parse source and return summary stats.
pub fn parse_and_summarize(source: &str, language: &str) -> Result<ParseStats> {
    let tree = parse_source(source, language)?;
    let root = tree.root_node();

    let mut total = 0;
    let mut named = 0;
    let mut max_depth = 0;
    let mut has_errors = false;

    let mut cursor = root.walk();
    let mut depth: usize = 0;
    loop {
        let node = cursor.node();
        total += 1;
        if node.is_named() {
            named += 1;
        }
        if node.is_error() {
            has_errors = true;
        }
        if depth > max_depth {
            max_depth = depth;
        }

        if cursor.goto_first_child() {
            depth += 1;
        } else {
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return Ok(ParseStats {
                        language: language.to_string(),
                        total_nodes: total,
                        named_nodes: named,
                        depth: max_depth,
                        has_errors,
                    });
                }
                depth -= 1;
            }
        }
    }
}

/// Detect language from file extension.
pub fn detect_language(path: &str) -> Option<&'static str> {
    let ext = path.rsplit('.').next()?;
    match ext {
        "rs" => Some("rust"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "py" | "pyi" => Some("python"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Some("cpp"),
        "go" => Some("go"),
        "nix" => Some("nix"),
        "yaml" | "yml" => Some("yaml"),
        "json" => Some("json"),
        "toml" => Some("toml"),
        "sh" | "bash" | "zsh" => Some("shell"),
        "ps1" | "psm1" => Some("powershell"),
        "dockerfile" => Some("dockerfile"),
        _ => None,
    }
}

// ── Atom Extraction ──────────────────────────────────────────────────

/// Convert UIR entities into atoms.
pub fn extract_atoms(entities: &[UirEntity]) -> Vec<Atom> {
    entities
        .iter()
        .filter_map(|entity| entity_to_atom(entity, None, None))
        .collect()
}

/// Convert UIR entities with pre-extracted signatures into atoms.
pub fn extract_atoms_with_signatures(pairs: &[(UirEntity, Option<AtomSignature>)]) -> Vec<Atom> {
    pairs
        .iter()
        .filter_map(|(entity, sig)| entity_to_atom(entity, sig.clone(), None))
        .collect()
}

/// Convert UIR entities with signatures and source bodies into atoms.
pub fn extract_atoms_with_bodies(
    triples: &[(UirEntity, Option<AtomSignature>, Option<String>)],
) -> Vec<Atom> {
    triples
        .iter()
        .filter_map(|(entity, sig, body)| entity_to_atom(entity, sig.clone(), body.clone()))
        .collect()
}

fn entity_to_atom(
    entity: &UirEntity,
    signature: Option<AtomSignature>,
    body: Option<String>,
) -> Option<Atom> {
    let kind = map_uir_to_atom(&entity.kind, &entity.labels)?;
    let name = entity
        .id
        .rsplit(':')
        .next()
        .unwrap_or("unknown")
        .to_string();

    Some(Atom {
        id: entity.id.clone(),
        kind,
        name: name.clone(),
        source_path: entity.source_path.clone(),
        language: entity.language.clone().unwrap_or_default(),
        labels: entity.labels.clone(),
        concept: infer_concept(&name, &entity.labels),
        signature,
        body,
    })
}

/// Map UIR kind string to AtomKind.
fn map_uir_to_atom(uir_kind: &str, labels: &[String]) -> Option<AtomKind> {
    if labels.contains(&"test".to_string()) {
        return Some(AtomKind::TestCase);
    }

    match uir_kind {
        "function" => Some(AtomKind::Function),
        "method" => Some(AtomKind::Method),
        "class" => Some(AtomKind::Schema),
        "struct" => Some(AtomKind::Schema),
        "trait" => Some(AtomKind::Schema),
        "interface" => Some(AtomKind::Schema),
        "module" => None,
        "api_endpoint" => Some(AtomKind::ApiEndpoint),
        "sql_query" => Some(AtomKind::SqlQuery),
        "state_machine" => Some(AtomKind::StateMachine),
        "event_handler" => Some(AtomKind::EventHandler),
        "ui_component" => Some(AtomKind::UiComponent),
        "cli_command" => Some(AtomKind::CliCommand),
        "test_case" => Some(AtomKind::TestCase),
        "schema" => Some(AtomKind::Schema),
        "config_pattern" => Some(AtomKind::ConfigPattern),
        "workflow" => Some(AtomKind::Workflow),
        "pipeline" => Some(AtomKind::Pipeline),
        _ => None,
    }
}

/// Infer a concept hint from the atom name and labels.
fn infer_concept(name: &str, labels: &[String]) -> Option<String> {
    if labels.contains(&"test".to_string()) {
        return Some("test".to_string());
    }

    let lower = name.to_lowercase();

    let rules: &[(&[&str], &str)] = &[
        (&["socket", "tcp", "udp", "listen", "bind", "connect", "accept"], "network"),
        (&["dns", "resolve", "lookup", "nameserver"], "dns"),
        (&["http", "request", "response", "header", "url", "uri"], "http"),
        (&["tls", "ssl", "handshake", "certificate", "cert"], "tls"),
        (&["vpn", "tunnel", "wireguard"], "vpn"),
        (&["encrypt", "decrypt", "cipher", "aes", "chacha", "hmac", "hash", "sha", "blake"], "crypto"),
        (&["sign", "verify", "signature", "ed25519", "rsa"], "signing"),
        (&["key", "keypair", "pubkey", "privkey", "secret_key"], "key-management"),
        (&["auth", "login", "logout", "jwt", "token", "session", "credential"], "auth"),
        (&["password", "passwd", "bcrypt", "argon"], "password"),
        (&["read", "write", "open", "close", "flush", "seek"], "io"),
        (&["send", "recv", "transmit", "receive"], "transport"),
        (&["buffer", "buf", "queue", "ring_buf", "channel"], "buffer"),
        (&["parse", "deserialize", "decode", "from_str", "from_bytes"], "parse"),
        (&["serialize", "encode", "to_string", "to_bytes", "format"], "serialize"),
        (&["convert", "transform", "map", "into", "from"], "convert"),
        (&["error", "err", "fail", "panic", "abort", "unwrap"], "error"),
        (&["retry", "backoff", "exponential"], "retry"),
        (&["timeout", "deadline", "expire"], "timeout"),
        (&["config", "setting", "option", "preference"], "config"),
        (&["init", "setup", "bootstrap", "start"], "init"),
        (&["shutdown", "stop", "terminate", "cleanup", "drop"], "cleanup"),
        (&["state", "status", "machine", "transition"], "state"),
        (&["cache", "memoize", "lru"], "cache"),
        (&["lock", "mutex", "rwlock", "semaphore", "guard"], "concurrency"),
        (&["async", "await", "future", "poll", "spawn", "task"], "async"),
        (&["valid", "check", "assert", "ensure", "require"], "validation"),
        (&["display", "render", "draw", "paint", "show", "print"], "display"),
        (&["log", "trace", "debug", "info", "warn"], "logging"),
        (&["iter", "collect", "filter", "fold", "reduce"], "iteration"),
        (&["sort", "search", "find", "index", "lookup"], "search"),
        (&["insert", "remove", "delete", "add", "push", "pop"], "collection"),
        (&["build", "builder", "create", "new", "make", "construct"], "builder"),
    ];

    for (keywords, concept) in rules {
        for kw in *keywords {
            if lower.contains(kw) {
                return Some(concept.to_string());
            }
        }
    }

    None
}

/// Parse a file, extract entities, and produce atoms in one call.
pub fn parse_file(source: &str, file_path: &str, language: &str) -> Result<Vec<Atom>> {
    let triples = extract::parse_and_extract_full(source, file_path, language)?;
    Ok(extract_atoms_with_bodies(&triples))
}

/// Scan a directory and extract all atoms from parseable files.
pub fn extract_from_dir(dir: &std::path::Path) -> Result<Vec<Atom>> {
    let paths = scan::scan_directory(dir);
    let mut all_atoms = Vec::new();

    for path in paths {
        let path_str = path.display().to_string();
        let language = match detect_language(&path_str) {
            Some(lang) if parseable_languages().contains(&lang) => lang,
            _ => continue,
        };

        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Skip very large files
        if source.len() > 2 * 1024 * 1024 {
            continue;
        }

        match parse_file(&source, &path_str, language) {
            Ok(atoms) => all_atoms.extend(atoms),
            Err(_) => continue,
        }
    }

    Ok(all_atoms)
}
