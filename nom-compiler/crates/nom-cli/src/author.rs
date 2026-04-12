//! `nom author` — brainstorm-in-markdown → .nom code authoring flow.
//!
//! Per user directive: Nom source is close enough to natural language
//! that authoring begins in a `.md` scratch file. Prose is replaced
//! fragment-by-fragment with Nom syntax until every line is either a
//! comment or a Nom token. At that point the file is renamed to `.nom`.
//!
//! Commands:
//!   `nom author start <name>`  — seed `<name>.md` with a scratch template.
//!   `nom author check <file>`  — for `.md`: report %-Nom progression;
//!                                for `.nom`: run `nom check`.

use std::path::{Path, PathBuf};

const SCRATCH_TEMPLATE: &str = "\
# {name}

<!-- Brainstorm scratch file for a Nom program.
     Start by writing the intent in plain English below, then gradually
     replace prose with Nom syntax. Rename to `{name}.nom` when every
     non-comment line is Nom.

     Tips:
       - `nom mcp serve` gives your LLM `search_nomtu` / `list_concepts`
         so it can find existing nomtu that match what you've written.
       - `nom author check {name}.md` reports how much remains prose.
       - Dreaming mode (`nom app dream`) surfaces missing nomtu. -->

## Intent

What should this program do? Write it in plain English.

## Sketch

Rough step-by-step in prose. Each bullet will become a Nom line.

- step one
- step two
- step three

## Nom substitutions

As you replace prose with Nom, paste the substituted fragments here
for quick diff. Once every bullet above is a Nom line (or a comment),
rename this file `{name}.nom` and run `nom check`.
";

/// Target artifact form for `nom author translate`. Each maps to a
/// distinct compile-and-verify pipeline downstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslateTarget {
    App,
    Video,
    Image,
}

impl TranslateTarget {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "app" => Some(Self::App),
            "video" => Some(Self::Video),
            "image" => Some(Self::Image),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Video => "video",
            Self::Image => "image",
        }
    }
}

/// `nom author translate <input> --target <app|video|image>` — the
/// prose→artifact entry point per the 2026-04-13 directive.
///
/// Scaffold form today: accepts any input file (treats it as prose),
/// inspects with the existing line-classifier, and emits a translation
/// plan describing what the LLM loop must do next. Real end-to-end
/// translation (LLM-driven nomtu + concept brainstorming + compile-
/// verify loop) lands as downstream consumers (`nom mcp serve` +
/// `nom app dream`) hook into this command.
pub fn cmd_author_translate(
    input: &Path,
    target: TranslateTarget,
    json: bool,
) -> i32 {
    let text = match std::fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom author translate: read {}: {e}", input.display());
            return 1;
        }
    };
    let stats = classify_lines(&text);
    let ready = stats.prose == 0 && stats.nom_ish > 0;
    let proposals = extract_prose_proposals(&text);

    if json {
        let doc = serde_json::json!({
            "input": input.display().to_string(),
            "target": target.as_str(),
            "lines": {
                "comment": stats.comment,
                "nom_ish": stats.nom_ish,
                "prose": stats.prose,
            },
            "progression_pct": stats.progression_pct(),
            "ready_to_compile": ready,
            "next_step": translate_next_step(&stats, target),
            "proposals": proposals,
        });
        println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
    } else {
        println!("author translate: {} → {}", input.display(), target.as_str());
        println!("  prose lines:    {}", stats.prose);
        println!("  nom-ish lines:  {}", stats.nom_ish);
        println!("  progression:    {}%", stats.progression_pct());
        println!();
        if !proposals.is_empty() {
            println!("  proposals ({} nomtu candidate(s)):", proposals.len());
            for p in &proposals {
                println!("    - word={} kind={} concept={}", p.word, p.kind, p.concept);
                println!("      from: {}", p.source_phrase);
            }
            println!();
        }
        println!("  next: {}", translate_next_step(&stats, target));
    }
    if ready { 0 } else { 2 }
}

/// One nomtu candidate extracted from a prose phrase. The translate
/// LLM loop uses these to drive `nom store add` + `nom concept add`
/// calls in lockstep — every nomtu gets a concept, every concept gets
/// nomtu, per the 2026-04-13 prose→artifact directive.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TranslateProposal {
    /// Suggested nomtu word, sanitized to [a-z0-9_].
    pub word: String,
    /// Suggested EntryKind tag (function / page / user_flow / …).
    pub kind: String,
    /// Suggested concept membership; inferred from the markdown
    /// section the phrase appeared in (Intent → "intent", Sketch →
    /// "sketch", other → "misc").
    pub concept: String,
    /// The original prose phrase, trimmed.
    pub source_phrase: String,
}

/// Walk the input as markdown-ish text and extract one proposal per
/// prose bullet or intent-line. Current heuristics:
///   - `## Intent` section lines with text → concept "intent"
///   - `## Sketch` section `- bullets` → concept "sketch"
///   - Fallback: any line outside a comment that has ≥3 words
///     → concept "misc"
/// Word is the first 3 words sanitized + joined by underscore.
fn extract_prose_proposals(text: &str) -> Vec<TranslateProposal> {
    let mut out: Vec<TranslateProposal> = Vec::new();
    let mut section = "misc".to_string();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            section = rest.to_ascii_lowercase();
            continue;
        }
        if line.starts_with('#') || line.starts_with("<!--") || line.starts_with("-->") {
            continue;
        }
        if looks_nom_ish(line) {
            continue;
        }
        let phrase = line.trim_start_matches("- ").trim_start_matches("* ").trim();
        if phrase.split_whitespace().count() < 2 {
            continue;
        }
        let word = prose_to_word(phrase);
        if word.is_empty() {
            continue;
        }
        out.push(TranslateProposal {
            word,
            kind: "function".to_string(),
            concept: section.clone(),
            source_phrase: phrase.to_string(),
        });
    }
    out
}

fn prose_to_word(phrase: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    for w in phrase.split_whitespace().take(3) {
        let sanitized: String = w
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        if !sanitized.is_empty() {
            words.push(sanitized);
        }
    }
    words.join("_")
}

/// Describe the next step in the prose→artifact loop given current
/// state. The LLM consumes this verbatim via MCP; keep it specific.
fn translate_next_step(stats: &LineStats, target: TranslateTarget) -> String {
    if stats.nom_ish + stats.prose == 0 {
        return "file is empty or all-comment — add at least one line of intent".to_string();
    }
    if stats.prose > 0 {
        return format!(
            "query dict (list_nomtu, search_nomtu, list_concepts) for nomtu \
             matching the {} prose lines; author missing nomtu + concepts in \
             lockstep; re-run `nom author translate` until progression is 100%",
            stats.prose
        );
    }
    match target {
        TranslateTarget::App => {
            "rename to .nom; run `nom check`; then `nom app dream <manifest> \
             --target web` until app_score ≥ 95 (EPIC threshold)"
                .to_string()
        }
        TranslateTarget::Video => {
            "rename to .nom; compile to AV1 body bytes; use `nom store add-media` \
             to canonicalize into body_kind=av1"
                .to_string()
        }
        TranslateTarget::Image => {
            "rename to .nom; compile to AVIF body bytes; use `nom store add-media` \
             to canonicalize into body_kind=avif"
                .to_string()
        }
    }
}

pub fn cmd_author_start(name: &str, out_dir: Option<&Path>) -> i32 {
    if !is_valid_name(name) {
        eprintln!(
            "nom author start: invalid name `{name}` (ascii alnum + underscore only)"
        );
        return 1;
    }
    let dir = out_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("nom author start: cannot create {}: {e}", dir.display());
        return 1;
    }
    let file = dir.join(format!("{name}.md"));
    if file.exists() {
        eprintln!("nom author start: {} already exists (refusing to overwrite)", file.display());
        return 1;
    }
    let body = SCRATCH_TEMPLATE.replace("{name}", name);
    if let Err(e) = std::fs::write(&file, body) {
        eprintln!("nom author start: write {}: {e}", file.display());
        return 1;
    }
    println!("seeded {}", file.display());
    println!("  edit intent + sketch, then `nom author check {}`", file.display());
    0
}

/// Report on the progression of a brainstorm file toward Nom syntax.
/// Line classification:
///   - comment:   starts with `//`, `/*`, `*`, `<!--`, or `#` or blank
///   - nom-ish:   contains `use `, `fn `, `let `, `return ` keywords or
///                the `.` / `|>` operators
///   - prose:     everything else
/// The progression metric is `nom_ish / (nom_ish + prose)` — once it
/// reaches 100% the file is ready to rename to `.nom`.
pub fn cmd_author_check(file: &Path, json: bool) -> i32 {
    let text = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom author check: read {}: {e}", file.display());
            return 1;
        }
    };
    let stats = classify_lines(&text);
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "nom" {
        if stats.prose == 0 {
            println!("{}: pure Nom ({} non-comment line(s))", file.display(), stats.nom_ish);
            0
        } else {
            eprintln!(
                "{}: {} prose line(s) still present — finish substitutions",
                file.display(),
                stats.prose
            );
            2
        }
    } else if json {
        let doc = serde_json::json!({
            "file": file.display().to_string(),
            "extension": ext,
            "comment": stats.comment,
            "nom_ish": stats.nom_ish,
            "prose": stats.prose,
            "progression_pct": stats.progression_pct(),
            "ready_for_rename": stats.prose == 0 && stats.nom_ish > 0,
        });
        println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
        0
    } else {
        println!("author check: {}", file.display());
        println!("  comment lines:  {}", stats.comment);
        println!("  nom-ish lines:  {}", stats.nom_ish);
        println!("  prose lines:    {}", stats.prose);
        println!("  progression:    {}%", stats.progression_pct());
        if stats.prose == 0 && stats.nom_ish > 0 {
            let target = file.with_extension("nom");
            println!();
            println!("  ready to rename → {}", target.display());
        }
        0
    }
}

struct LineStats {
    comment: usize,
    nom_ish: usize,
    prose: usize,
}

impl LineStats {
    fn progression_pct(&self) -> u32 {
        let denom = self.nom_ish + self.prose;
        if denom == 0 {
            100
        } else {
            ((self.nom_ish as f64 / denom as f64) * 100.0).round() as u32
        }
    }
}

fn classify_lines(text: &str) -> LineStats {
    let mut s = LineStats {
        comment: 0,
        nom_ish: 0,
        prose: 0,
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            s.comment += 1;
            continue;
        }
        if line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with('*')
            || line.starts_with("<!--")
            || line.starts_with("-->")
            || line.starts_with('#')
        {
            s.comment += 1;
            continue;
        }
        if looks_nom_ish(line) {
            s.nom_ish += 1;
        } else {
            s.prose += 1;
        }
    }
    s
}

fn looks_nom_ish(line: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "use ", "fn ", "let ", "return ", "if ", "else", "match ", "for ",
        "while ", "struct ", "enum ", "trait ", "impl ", "type ",
    ];
    for kw in KEYWORDS {
        if line.starts_with(kw) || line.contains(&format!(" {kw}")) {
            return true;
        }
    }
    // Operator signals: pipe, arrow, @-hash reference.
    line.contains("|>") || line.contains("->") || line.contains('@')
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_name_rules() {
        assert!(is_valid_name("hello"));
        assert!(is_valid_name("snake_case_123"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("has space"));
        assert!(!is_valid_name("dash-bad"));
    }

    #[test]
    fn classify_recognizes_nom_ish() {
        let text = "\
# heading
Some prose.
use math@abc123.
let x = 1.
fn greet() { }
random words without syntax
";
        let s = classify_lines(text);
        assert_eq!(s.comment, 1);
        assert_eq!(s.nom_ish, 3);
        assert_eq!(s.prose, 2);
        assert_eq!(s.progression_pct(), 60);
    }

    #[test]
    fn progression_all_nom_is_100() {
        let text = "use a@h.\nfn f() { }\n";
        let s = classify_lines(text);
        assert_eq!(s.prose, 0);
        assert_eq!(s.progression_pct(), 100);
    }

    #[test]
    fn prose_to_word_sanitizes_and_joins() {
        assert_eq!(prose_to_word("Hello World Foo Bar"), "hello_world_foo");
        assert_eq!(prose_to_word("Render the user's dashboard"), "render_the_users");
        assert_eq!(prose_to_word("..."), "");
    }

    #[test]
    fn extract_prose_proposals_tracks_sections() {
        let text = "\
# Title

## Intent

Render a dashboard for logged-in users.

## Sketch

- fetch user profile
- render greeting
- show recent activity
- hi
";
        let ps = extract_prose_proposals(text);
        assert_eq!(ps.len(), 4, "got: {ps:?}");
        assert_eq!(ps[0].concept, "intent");
        assert_eq!(ps[0].word, "render_a_dashboard");
        assert_eq!(ps[1].concept, "sketch");
        assert_eq!(ps[1].word, "fetch_user_profile");
        // "hi" is < 2 words → skipped.
    }

    #[test]
    fn extract_prose_proposals_skips_nom_ish_lines() {
        let text = "\
## Sketch

- use greeting@abc123
- fetch user data
fn already_nom() -> integer { return 0 }
";
        let ps = extract_prose_proposals(text);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].word, "fetch_user_data");
    }

    #[test]
    fn translate_target_from_str_and_back() {
        assert_eq!(TranslateTarget::from_str("app"), Some(TranslateTarget::App));
        assert_eq!(TranslateTarget::from_str("video"), Some(TranslateTarget::Video));
        assert_eq!(TranslateTarget::from_str("image"), Some(TranslateTarget::Image));
        assert_eq!(TranslateTarget::from_str("garbage"), None);
        assert_eq!(TranslateTarget::App.as_str(), "app");
    }

    #[test]
    fn translate_next_step_pivots_on_prose_vs_target() {
        let all_prose = LineStats { comment: 0, nom_ish: 0, prose: 3 };
        let s = translate_next_step(&all_prose, TranslateTarget::App);
        assert!(s.contains("query dict"), "expected dict-query prompt: {s}");

        let all_nom = LineStats { comment: 0, nom_ish: 5, prose: 0 };
        let app = translate_next_step(&all_nom, TranslateTarget::App);
        assert!(app.contains("nom app dream"), "expected dream prompt: {app}");
        let vid = translate_next_step(&all_nom, TranslateTarget::Video);
        assert!(vid.contains("av1"), "expected av1 prompt: {vid}");
        let img = translate_next_step(&all_nom, TranslateTarget::Image);
        assert!(img.contains("avif"), "expected avif prompt: {img}");
    }

    #[test]
    fn author_start_refuses_overwrite() {
        let dir = std::env::temp_dir().join(format!("nom-author-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let rc = cmd_author_start("sample", Some(&dir));
        assert_eq!(rc, 0);
        let rc2 = cmd_author_start("sample", Some(&dir));
        assert_eq!(rc2, 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
