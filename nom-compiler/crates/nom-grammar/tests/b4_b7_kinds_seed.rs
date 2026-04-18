//! B4 + B7 seed verification tests.
//!
//! B4: all 29 extended kinds (UX/app/bench/flow/media) are present after
//!     importing baseline.sql into a fresh grammar.sqlite.
//! B7: all 9 self-documenting skill entries are present after the same import.
//! Guard: no external brand names appear in any kind name or description.

use std::path::PathBuf;

fn baseline_conn() -> (tempfile::TempDir, rusqlite::Connection) {
    let baseline_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("baseline.sql");
    let sql = std::fs::read_to_string(&baseline_path).expect("baseline.sql exists");
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init schema");
    conn.execute_batch(&sql).expect("import baseline.sql");
    (dir, conn)
}

fn kind_names(conn: &rusqlite::Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM kinds ORDER BY name")
        .expect("prepare");
    stmt.query_map([], |r| r.get::<_, String>(0))
        .expect("query")
        .filter_map(|r| r.ok())
        .collect()
}

// ── B4: extended kinds ────────────────────────────────────────────────

const B4_KINDS: &[&str] = &[
    "ux_pattern",
    "design_rule",
    "screen",
    "user_flow",
    "skill",
    "app_manifest",
    "data_source",
    "query",
    "app_action",
    "app_variable",
    "page",
    "benchmark",
    "benchmark_run",
    "flow_artifact",
    "flow_step",
    "flow_middleware",
    "media_unit",
    "pixel_grid",
    "audio_buffer",
    "video_stream",
    "vector_path",
    "glyph_outline",
    "mesh_geometry",
    "color",
    "palette",
    "codec",
    "container",
    "media_metadata",
    "render_pipeline",
];

#[test]
fn b4_all_29_extended_kind_names_present_after_import() {
    let (_dir, conn) = baseline_conn();
    let names = kind_names(&conn);
    let mut missing: Vec<&str> = Vec::new();
    for &k in B4_KINDS {
        if !names.contains(&k.to_string()) {
            missing.push(k);
        }
    }
    assert!(
        missing.is_empty(),
        "B4 extended kinds missing from grammar.sqlite after baseline import: {missing:?}"
    );
}

#[test]
fn b4_extended_kinds_count_is_29() {
    let (_dir, conn) = baseline_conn();
    let names = kind_names(&conn);
    let found: Vec<&&str> = B4_KINDS
        .iter()
        .filter(|k| names.contains(&k.to_string()))
        .collect();
    assert_eq!(
        found.len(),
        29,
        "expected all 29 B4 extended kinds, found {}",
        found.len()
    );
}

// ── B7: self-documenting skill entries ───────────────────────────────

const B7_SKILLS: &[&str] = &[
    "author_nom_app",
    "compose_from_dict",
    "debug_nom_closure",
    "extend_nom_compiler",
    "ingest_new_ecosystem",
    "use_ai_loop",
    "compose_brutalist_webpage",
    "compose_generative_art",
    "compose_lofi_audio_loop",
];

#[test]
fn b7_all_9_skill_entries_present_after_import() {
    let (_dir, conn) = baseline_conn();
    let names = kind_names(&conn);
    let mut missing: Vec<&str> = Vec::new();
    for &s in B7_SKILLS {
        if !names.contains(&s.to_string()) {
            missing.push(s);
        }
    }
    assert!(
        missing.is_empty(),
        "B7 skill entries missing from grammar.sqlite after baseline import: {missing:?}"
    );
}

#[test]
fn b7_skill_entries_count_is_9() {
    let (_dir, conn) = baseline_conn();
    let names = kind_names(&conn);
    let found: Vec<&&str> = B7_SKILLS
        .iter()
        .filter(|k| names.contains(&k.to_string()))
        .collect();
    assert_eq!(
        found.len(),
        9,
        "expected all 9 B7 skill entries, found {}",
        found.len()
    );
}

// ── AH-DB-KINDS: 14 composition-target kinds ─────────────────────────

const AH_DB_KINDS: &[&str] = &[
    "video_compose",
    "picture_compose",
    "audio_compose",
    "presentation_compose",
    "web_app_compose",
    "mobile_app_compose",
    "native_app_compose",
    "document_compose",
    "data_extract",
    "data_query",
    "workflow_compose",
    "ad_creative_compose",
    "mesh_3d_compose",
    "storyboard_compose",
];

const AH_DB_BANNED: &[&str] = &[
    "affine",
    "comfy",
    "dify",
    "n8n",
    "bolt",
    "langchain",
    "openai",
    "gemini",
];

#[test]
fn test_ah_db_kinds_count_is_14() {
    let (_dir, conn) = baseline_conn();
    let names = kind_names(&conn);
    let found: Vec<&&str> = AH_DB_KINDS
        .iter()
        .filter(|k| names.contains(&k.to_string()))
        .collect();
    assert_eq!(
        found.len(),
        14,
        "expected 14 AH-DB-KINDS composition targets, found {}",
        found.len()
    );
}

#[test]
fn test_ah_db_kinds_no_foreign_names() {
    for kind in AH_DB_KINDS {
        for banned in AH_DB_BANNED {
            assert!(
                !kind.contains(banned),
                "AH-DB kind '{kind}' contains banned foreign name '{banned}'"
            );
        }
    }
}

// ── Guard: no external brand names in kind names or descriptions ──────

/// Banned external brand / foreign-language names that must not appear
/// as whole words in any kind `name` or `description` column.
const BANNED_NAMES: &[&str] = &[
    "comfy",
    "higgsfield",
    "figma",
    "sketch",
    "react",
    "openai",
    "anthropic",
    "canva",
    "framer",
    "notion",
];

fn contains_whole_word(haystack: &str, needle: &str) -> bool {
    let hay = haystack.to_ascii_lowercase();
    let need = needle.to_ascii_lowercase();
    let mut start = 0usize;
    while let Some(idx) = hay[start..].find(&need) {
        let abs = start + idx;
        let before_ok = abs == 0
            || !hay[..abs]
                .chars()
                .last()
                .map(|c| c.is_ascii_alphanumeric() || c == '_')
                .unwrap_or(false);
        let after = abs + need.len();
        let after_ok = after == hay.len()
            || !hay[after..]
                .chars()
                .next()
                .map(|c| c.is_ascii_alphanumeric() || c == '_')
                .unwrap_or(false);
        if before_ok && after_ok {
            return true;
        }
        start = abs + need.len();
        if start >= hay.len() {
            break;
        }
    }
    false
}

#[test]
fn no_external_brand_names_in_kind_names_or_descriptions() {
    let (_dir, conn) = baseline_conn();
    let mut stmt = conn
        .prepare("SELECT name, description FROM kinds")
        .expect("prepare");
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .expect("query")
        .filter_map(|r| r.ok())
        .collect();

    let mut hits: Vec<(String, String, String)> = Vec::new();
    for (name, desc) in &rows {
        for &banned in BANNED_NAMES {
            if contains_whole_word(name, banned) {
                hits.push((name.clone(), "name".to_string(), banned.to_string()));
            }
            if contains_whole_word(desc, banned) {
                hits.push((name.clone(), "description".to_string(), banned.to_string()));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "external brand names found in kinds table: {hits:#?}"
    );
}
