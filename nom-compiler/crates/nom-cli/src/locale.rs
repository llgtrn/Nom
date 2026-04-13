//! CLI handlers for `nom locale` subcommands (M3a scaffold).

use nom_locale::{LocaleTag, builtin_packs};

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
