//! Nom CLI — user-facing entry point.
//!
//! Exposes `nom` binary with subcommands `compose`, `plan`, `lint`,
//! `version`.  The crate is structured so that `run(args) -> i32` is
//! fully testable without spawning a subprocess.
#![deny(unsafe_code)]

pub mod commands;

use commands::{cmd_compose, cmd_lint, cmd_plan, cmd_version};

/// Parse + dispatch.  Returns process exit code.
///
/// `args[0]` is the binary name.  `args[1]` is the subcommand.  Remaining
/// args are passed to the subcommand handler.
pub fn run(args: &[String]) -> i32 {
    let Some(subcommand) = args.get(1) else { return print_usage(); };
    let rest: Vec<String> = args.iter().skip(2).cloned().collect();
    match subcommand.as_str() {
        "compose" => cmd_compose(&rest),
        "plan" => cmd_plan(&rest),
        "lint" => cmd_lint(&rest),
        "version" | "--version" | "-V" => cmd_version(),
        "help" | "--help" | "-h" => print_usage(),
        unknown => {
            eprintln!("nom: unknown subcommand '{}'", unknown);
            print_usage_to_stderr();
            2
        }
    }
}

fn print_usage() -> i32 {
    println!("{}", usage_text());
    0
}

fn print_usage_to_stderr() {
    eprintln!("{}", usage_text());
}

fn usage_text() -> &'static str {
    "nom — Nom project CLI\n\nUSAGE:\n    nom <subcommand> [options]\n\nSUBCOMMANDS:\n    compose   Dispatch a composition spec through a backend\n    plan      Inspect or execute a composition plan\n    lint      Run lint rules over a source string\n    version   Print version\n    help      Print this help\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::extract_flag;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_args_returns_zero() {
        assert_eq!(run(&args(&["nom"])), 0);
    }

    #[test]
    fn help_returns_zero() {
        assert_eq!(run(&args(&["nom", "help"])), 0);
    }

    #[test]
    fn version_flag_returns_zero() {
        assert_eq!(run(&args(&["nom", "--version"])), 0);
    }

    #[test]
    fn unknown_subcommand_returns_two() {
        assert_eq!(run(&args(&["nom", "frobulate"])), 2);
    }

    #[test]
    fn compose_with_kind_returns_zero() {
        assert_eq!(run(&args(&["nom", "compose", "--kind", "media-image"])), 0);
    }

    #[test]
    fn plan_with_steps_returns_zero() {
        assert_eq!(run(&args(&["nom", "plan", "--steps", "3"])), 0);
    }

    #[test]
    fn lint_trailing_whitespace_returns_one() {
        // "foo   \nbar" — first line has trailing spaces, second does not
        assert_eq!(run(&args(&["nom", "lint", "--source", "foo   \nbar"])), 1);
    }

    #[test]
    fn extract_flag_finds_present_flag() {
        let a = args(&["--kind", "media-image"]);
        assert_eq!(extract_flag(&a, "--kind"), Some("media-image".to_string()));
    }

    #[test]
    fn extract_flag_returns_none_for_absent_flag() {
        let a = args(&["--kind", "media-image"]);
        assert_eq!(extract_flag(&a, "--steps"), None);
    }
}
