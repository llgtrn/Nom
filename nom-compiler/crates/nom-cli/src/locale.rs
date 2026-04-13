//! CLI handlers for `nom locale` subcommands (M3a + M3c).

use nom_locale::{ApplyDirection, ApplyReport, LocaleTag, apply_locale, builtin_packs};
use std::path::PathBuf;

/// `nom locale list` — print one line per registered pack.
///
/// Format: `<canonical-tag>\t<display_name>\t<source>\t<license>`
pub fn cmd_locale_list() -> i32 {
    for pack in builtin_packs() {
        println!(
            "{}\t{}\t{}\t{}",
            pack.id.canonical(),
            pack.register_metadata.display_name,
            pack.register_metadata.source,
            pack.register_metadata.license,
        );
    }
    0
}

/// `nom locale validate <tag>` — parse + validate a BCP 47 tag.
///
/// Exit 0 + `"valid: <canonical>"` on success (with optional note if no pack registered).
/// Exit 1 + `"invalid: <err>"` on parse failure.
pub fn cmd_locale_validate(tag: &str) -> i32 {
    match LocaleTag::parse(tag) {
        Ok(parsed) => {
            let canonical = parsed.canonical();
            let registered = builtin_packs()
                .iter()
                .any(|p| p.id.canonical() == canonical);
            if registered {
                println!("valid: {canonical}");
            } else {
                println!("valid: {canonical} (no pack registered)");
            }
            0
        }
        Err(e) => {
            eprintln!("invalid: {e}");
            1
        }
    }
}

/// `nom locale apply <tag> <file>` — apply a locale pack to a source file.
///
/// By default prints the transformed source to stdout.
/// `--write` overwrites the file in place and prints a summary.
/// `--json` emits a JSON report instead of the transformed source.
/// `--from-canonical` inverts the direction (English → localized).
#[allow(clippy::too_many_arguments)]
pub fn cmd_locale_apply(
    tag: &str,
    file: &PathBuf,
    from_canonical: bool,
    write: bool,
    json: bool,
) -> i32 {
    // Resolve the pack.
    let packs = builtin_packs();
    let pack = match packs.iter().find(|p| p.id.canonical() == tag) {
        Some(p) => p,
        None => {
            eprintln!("nom: no builtin pack for {tag}");
            return 1;
        }
    };

    // Read source.
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: cannot read {}: {e}", file.display());
            return 1;
        }
    };

    let direction = if from_canonical {
        ApplyDirection::FromCanonical
    } else {
        ApplyDirection::ToCanonical
    };

    let report: ApplyReport = apply_locale(&source, pack, direction);

    if json {
        // Emit a JSON report.
        let replacements_json: Vec<String> = report
            .replacements
            .iter()
            .map(|r| {
                format!(
                    r#"{{"line":{},"column":{},"from":{},"to":{}}}"#,
                    r.line,
                    r.column,
                    json_string(&r.from),
                    json_string(&r.to),
                )
            })
            .collect();
        println!(
            r#"{{"replacements":[{}],"skipped_in_literals":{},"output_len":{}}}"#,
            replacements_json.join(","),
            report.skipped_in_literals,
            report.output.len(),
        );
        return 0;
    }

    if write {
        if let Err(e) = std::fs::write(file, report.output.as_bytes()) {
            eprintln!("nom: cannot write {}: {e}", file.display());
            return 1;
        }
        println!(
            "applied: {} replacement(s) to {}",
            report.replacements.len(),
            file.display()
        );
        return 0;
    }

    // Default: print transformed source to stdout.
    print!("{}", report.output);
    0
}

/// Minimal JSON string escaping (no external dep).
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
