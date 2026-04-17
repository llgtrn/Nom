//! Subcommand handlers.
//!
//! Each returns a process exit code.  Handlers print to stdout/stderr
//! directly; for testability, the handlers take `&[String]` so callers
//! can simulate arg lists.
#![deny(unsafe_code)]

pub fn cmd_compose(args: &[String]) -> i32 {
    // Minimal flag parsing: expect `--kind <name>`.
    let kind = extract_flag(args, "--kind").unwrap_or("data-extract".to_string());
    println!("compose kind={}", kind);
    0
}

pub fn cmd_plan(args: &[String]) -> i32 {
    // Minimal flag parsing: expect `--steps <count>`.
    let steps = extract_flag(args, "--steps").and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
    println!("plan steps={}", steps);
    0
}

pub fn cmd_lint(args: &[String]) -> i32 {
    // Minimal form: `--source <string>`; run placeholder lint suite and print diagnostic count.
    let source = extract_flag(args, "--source").unwrap_or_default();
    let mut total_diagnostics: usize = 0;
    for line in source.lines() {
        if line.trim_end().len() != line.len() {
            total_diagnostics += 1;
        }
    }
    println!("lint diagnostics={}", total_diagnostics);
    if total_diagnostics > 0 { 1 } else { 0 }
}

pub fn cmd_version() -> i32 {
    println!("nom-cli {}", env!("CARGO_PKG_VERSION"));
    0
}

pub fn extract_flag(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter.next().cloned();
        }
    }
    None
}
