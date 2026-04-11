//! nom-cli: The Nom language compiler command-line interface.
//!
//! Commands:
//!   nom run <file>      — compile and run a .nom file
//!   nom build <file>    — compile to .nomiz plan
//!   nom check <file>    — typecheck and verify contracts
//!   nom test <file>     — run test declarations in <file>
//!   nom report <file>   — security report for <file>
//!   nom dict <query>    — search the local nomdict database

use clap::{Parser, Subcommand};
use nom_codegen::{collect_dependencies, generate, CodegenOptions};

use nom_parser::parse_source;
use nom_planner::Planner;
use nom_resolver::{NomtuEntry, Resolver};
use nom_security::{SecurityChecker, SecurityConfig};
use nom_verifier::Verifier;
use rusqlite::OpenFlags;
use std::path::{Path, PathBuf};
use std::process;

// ── CLI structure ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "nom",
    version,
    about = "The Nom language compiler",
    long_about = "Nom is a composition language for assembling verified software from a dictionary of trusted words."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile and immediately run a .nom file (interpret mode)
    Run {
        /// Path to the .nom source file
        file: PathBuf,
        /// Path to the nomdict database (default: ./nomdict.db)
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },

    /// Compile a .nom file to a native binary
    Build {
        /// Path to the .nom source file
        file: PathBuf,
        /// Output path for the binary (default: <file> without extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Also emit Rust source code next to the .nom file
        #[arg(long)]
        emit_rust: bool,
        /// Compile generated Rust to a native binary (default: true)
        #[arg(long, default_value = "true")]
        compile: bool,
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },

    /// Type-check and verify contracts without producing output
    Check {
        /// Path to the .nom source file
        file: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },

    /// Run test declarations in a .nom file
    Test {
        /// Path to the .nom source file
        file: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Only run tests matching this pattern
        #[arg(long)]
        filter: Option<String>,
    },

    /// Generate a security report for a .nom file
    Report {
        /// Path to the .nom source file
        file: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Minimum security score threshold (0.0–1.0)
        #[arg(long, default_value = "0.7")]
        min_security: f64,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Search the local nomdict database
    Dict {
        /// Search query (searches describe column)
        query: String,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Import atoms from a Novelos database into the nomdict
    Import {
        /// Path to the Novelos SQLite database (e.g., data/novelos.db)
        source: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Only import atoms of this language (e.g., "rust")
        #[arg(long)]
        language: Option<String>,
        /// Only import atoms with this concept
        #[arg(long)]
        concept: Option<String>,
        /// Minimum body length to import (skip tiny atoms)
        #[arg(long, default_value = "10")]
        min_body_len: usize,
        /// Maximum atoms to import (0 = unlimited)
        #[arg(long, default_value = "0")]
        limit: usize,
        /// Dry run: count matches without importing
        #[arg(long)]
        dry_run: bool,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Commands::Run { file, dict } => cmd_run(&file, &dict),
        Commands::Build { file, output, dict, emit_rust, compile, release } => {
            cmd_build(&file, output.as_deref(), &dict, emit_rust, compile, release)
        }
        Commands::Check { file, dict } => cmd_check(&file, &dict),
        Commands::Test { file, dict, filter } => cmd_test(&file, &dict, filter.as_deref()),
        Commands::Report { file, dict, min_security, format } => {
            cmd_report(&file, &dict, min_security, &format)
        }
        Commands::Dict { query, dict, limit } => cmd_dict(&query, &dict, limit),
        Commands::Import { source, dict, language, concept, min_body_len, limit, dry_run } => {
            cmd_import(&source, &dict, language.as_deref(), concept.as_deref(), min_body_len, limit, dry_run)
        }
    };
    process::exit(exit_code);
}

// ── Command implementations ───────────────────────────────────────────────────

fn cmd_run(file: &PathBuf, dict: &PathBuf) -> i32 {
    // Build first (compile = true, release = false), output next to the .nom file
    let rc = cmd_build(file, None, dict, false, true, false);
    if rc != 0 {
        return rc;
    }

    // Determine the binary path (same logic as cmd_build)
    let binary_path = binary_output_path(file, None);
    if !binary_path.exists() {
        eprintln!("nom: binary not found at {}", binary_path.display());
        return 1;
    }

    // Execute the binary
    match process::Command::new(&binary_path).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("nom: failed to run {}: {e}", binary_path.display());
            1
        }
    }
}

/// Determine the final binary output path.
fn binary_output_path(file: &Path, output: Option<&Path>) -> PathBuf {
    if let Some(out) = output {
        out.to_path_buf()
    } else {
        let stem = file.file_stem().unwrap_or_default();
        let parent = file.parent().unwrap_or_else(|| Path::new("."));
        if cfg!(windows) {
            parent.join(format!("{}.exe", stem.to_string_lossy()))
        } else {
            parent.join(stem)
        }
    }
}

fn cmd_build(
    file: &PathBuf,
    output: Option<&Path>,
    dict: &PathBuf,
    emit_rust: bool,
    compile: bool,
    release: bool,
) -> i32 {
    let source = match read_source(file) {
        Some(s) => s,
        None => return 1,
    };

    let parsed = match parse_source(&source) {
        Ok(sf) => sf,
        Err(e) => {
            eprintln!("nom: parse error: {e}");
            return 1;
        }
    };

    let resolver = match open_resolver(dict) {
        Some(r) => r,
        None => return 1,
    };

    let planner = Planner::new(&resolver);
    let plan = match planner.plan_unchecked(&parsed) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("nom: plan error: {e}");
            return 1;
        }
    };

    // Always write .nomiz
    let nomiz_path = file.with_extension("nomiz");
    match std::fs::write(&nomiz_path, &plan.nomiz) {
        Ok(_) => println!("nom: wrote {}", nomiz_path.display()),
        Err(e) => {
            eprintln!("nom: write error: {e}");
            return 1;
        }
    }

    // Generate Rust source
    let opts = CodegenOptions::default();
    let codegen_out = match generate(&plan, &opts) {
        Ok(out) => out,
        Err(e) => {
            eprintln!("nom: codegen error: {e}");
            return 1;
        }
    };

    // Optionally emit .rs next to the source
    if emit_rust {
        let rs_path = file.with_extension("rs");
        match std::fs::write(&rs_path, &codegen_out.rust_source) {
            Ok(_) => println!("nom: wrote {}", rs_path.display()),
            Err(e) => {
                eprintln!("nom: write error: {e}");
                return 1;
            }
        }
    }

    if !compile {
        return 0;
    }

    // --- Compile to native binary via Cargo ---

    let file_stem = file
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let parent_dir = file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."));

    // Create build directory: .nom-out/<stem>/
    let build_dir = parent_dir.join(".nom-out").join(&file_stem);
    let src_dir = build_dir.join("src");

    if let Err(e) = std::fs::create_dir_all(&src_dir) {
        eprintln!("nom: cannot create build dir: {e}");
        return 1;
    }

    // Wrap generated code in a main function if it doesn't have one
    let rust_source = &codegen_out.rust_source;
    let main_rs = if rust_source.contains("fn main()") {
        rust_source.clone()
    } else {
        // The codegen emits a `run_all` dispatcher. Wrap it with main.
        let mut src = rust_source.clone();
        src.push_str("\nfn main() {\n    run_all();\n}\n");
        src
    };

    if let Err(e) = std::fs::write(src_dir.join("main.rs"), &main_rs) {
        eprintln!("nom: write main.rs error: {e}");
        return 1;
    }

    // Generate Cargo.toml
    let deps = collect_dependencies(&plan);
    let mut cargo_toml = format!(
        "[workspace]\n\n[package]\nname = \"{file_stem}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
    );
    for dep in &deps {
        cargo_toml.push_str(&format!("{} = {}\n", dep.name, dep.spec));
    }

    if let Err(e) = std::fs::write(build_dir.join("Cargo.toml"), &cargo_toml) {
        eprintln!("nom: write Cargo.toml error: {e}");
        return 1;
    }

    // Invoke cargo build
    let mut cmd = process::Command::new("cargo");
    cmd.arg("build").current_dir(&build_dir);
    if release {
        cmd.arg("--release");
    }

    println!("nom: compiling {file_stem}...");
    match cmd.status() {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!(
                "nom: cargo build failed (exit {})",
                status.code().unwrap_or(-1)
            );
            return 1;
        }
        Err(e) => {
            eprintln!("nom: failed to run cargo: {e}");
            return 1;
        }
    }

    // Copy the binary to the output path
    let profile = if release { "release" } else { "debug" };
    let bin_name = if cfg!(windows) {
        format!("{file_stem}.exe")
    } else {
        file_stem.clone()
    };
    let built_binary = build_dir.join("target").join(profile).join(&bin_name);
    let final_path = binary_output_path(file, output);

    match std::fs::copy(&built_binary, &final_path) {
        Ok(_) => {
            println!("nom: built {}", final_path.display());
            0
        }
        Err(e) => {
            eprintln!(
                "nom: cannot copy binary from {} to {}: {e}",
                built_binary.display(),
                final_path.display()
            );
            1
        }
    }
}

fn cmd_check(file: &PathBuf, dict: &PathBuf) -> i32 {
    let source = match read_source(file) {
        Some(s) => s,
        None => return 1,
    };

    let parsed = match parse_source(&source) {
        Ok(sf) => sf,
        Err(e) => {
            eprintln!("nom: parse error: {e}");
            return 1;
        }
    };

    let resolver = match open_resolver(dict) {
        Some(r) => r,
        None => return 1,
    };

    let verifier = Verifier::new(&resolver);
    let result = verifier.verify(&parsed);

    if result.ok() {
        println!("nom: check passed — 0 findings");
        0
    } else {
        for finding in &result.findings {
            eprintln!("  [{}] {}", finding.declaration, finding.error);
        }
        eprintln!(
            "nom: check failed — {} finding(s)",
            result.findings.len()
        );
        1
    }
}

fn cmd_test(file: &PathBuf, _dict: &PathBuf, filter: Option<&str>) -> i32 {
    let source = match read_source(file) {
        Some(s) => s,
        None => return 1,
    };

    let parsed = match parse_source(&source) {
        Ok(sf) => sf,
        Err(e) => {
            eprintln!("nom: parse error: {e}");
            return 1;
        }
    };

    // Find test declarations
    use nom_ast::Classifier;
    let tests: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|d| d.classifier == Classifier::Test)
        .filter(|d| {
            filter
                .map(|f| d.name.name.contains(f))
                .unwrap_or(true)
        })
        .collect();

    if tests.is_empty() {
        println!("nom: no tests found");
        return 0;
    }

    let mut passed = 0usize;
    let failed = 0usize;

    for test in &tests {
        println!("  test {} ... ", test.name.name);
        // For now: a test passes if it parses successfully (full execution TBD)
        println!("    ok (parse-level)");
        passed += 1;
    }

    println!("nom: {} passed, {} failed", passed, failed);
    if failed > 0 { 1 } else { 0 }
}

fn cmd_report(file: &PathBuf, dict: &PathBuf, min_security: f64, format: &str) -> i32 {
    let source = match read_source(file) {
        Some(s) => s,
        None => return 1,
    };

    let parsed = match parse_source(&source) {
        Ok(sf) => sf,
        Err(e) => {
            eprintln!("nom: parse error: {e}");
            return 1;
        }
    };

    let resolver = match open_resolver(dict) {
        Some(r) => r,
        None => return 1,
    };

    let config = SecurityConfig {
        min_security_score: min_security,
        ..SecurityConfig::default()
    };
    let checker = SecurityChecker::new(&resolver, config);
    let report = match checker.check(&parsed) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nom: security check error: {e}");
            return 1;
        }
    };

    match format {
        "json" => match report.to_json() {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("nom: json error: {e}");
                return 1;
            }
        },
        _ => {
            println!("Security Report for {}", file.display());
            println!("{}", "=".repeat(50));
            if report.findings.is_empty() {
                println!("No findings. All checks passed.");
            } else {
                for f in &report.findings {
                    println!(
                        "[{}] {}{}: {}",
                        f.severity,
                        f.word,
                        f.variant.as_deref().map(|v| format!("::{v}")).unwrap_or_default(),
                        f.message
                    );
                }
            }
            println!("{}", "=".repeat(50));
            println!(
                "Result: {} | {} finding(s)",
                if report.passed { "PASS" } else { "FAIL" },
                report.findings.len()
            );
        }
    }

    if report.passed { 0 } else { 1 }
}

fn cmd_dict(query: &str, dict: &PathBuf, limit: usize) -> i32 {
    let resolver = match open_resolver(dict) {
        Some(r) => r,
        None => return 1,
    };

    match resolver.search_by_describe(query, limit) {
        Ok(entries) => {
            if entries.is_empty() {
                println!("No results for '{query}'");
            } else {
                println!("{:<20} {:<12} {:<8} {:<8} {}",
                    "WORD", "VARIANT", "SEC", "PERF", "DESCRIPTION");
                println!("{}", "-".repeat(70));
                for e in &entries {
                    println!(
                        "{:<20} {:<12} {:<8.2} {:<8.2} {}",
                        e.word,
                        e.variant.as_deref().unwrap_or("-"),
                        e.security,
                        e.performance,
                        e.describe.as_deref().unwrap_or(""),
                    );
                }
            }
            0
        }
        Err(e) => {
            eprintln!("nom: dict error: {e}");
            1
        }
    }
}

fn cmd_import(
    source: &PathBuf,
    dict: &PathBuf,
    language: Option<&str>,
    concept: Option<&str>,
    min_body_len: usize,
    limit: usize,
    dry_run: bool,
) -> i32 {
    // Open Novelos database read-only
    let novelos_conn = match rusqlite::Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nom: cannot open novelos db {}: {e}", source.display());
            return 1;
        }
    };

    // Open nomdict via resolver
    let resolver = match open_resolver(dict) {
        Some(r) => r,
        None => return 1,
    };

    // Build query with optional filters
    let mut sql = String::from(
        "SELECT hash, name, kind, language, concept, signature, body, source_path \
         FROM atoms WHERE body IS NOT NULL AND length(body) >= ?1",
    );
    let mut param_index = 2u32;

    let lang_idx = if language.is_some() {
        let idx = param_index;
        sql.push_str(&format!(" AND language = ?{idx}"));
        param_index += 1;
        Some(idx)
    } else {
        None
    };

    let concept_idx = if concept.is_some() {
        let idx = param_index;
        sql.push_str(&format!(" AND concept = ?{idx}"));
        param_index += 1;
        Some(idx)
    } else {
        None
    };

    if limit > 0 {
        sql.push_str(&format!(" LIMIT ?{param_index}"));
    }

    let mut stmt = match novelos_conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: query error: {e}");
            return 1;
        }
    };

    // Build dynamic params
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    params_vec.push(Box::new(min_body_len as i64));
    if let (Some(_), Some(lang)) = (lang_idx, language) {
        params_vec.push(Box::new(lang.to_owned()));
    }
    if let (Some(_), Some(conc)) = (concept_idx, concept) {
        params_vec.push(Box::new(conc.to_owned()));
    }
    if limit > 0 {
        params_vec.push(Box::new(limit as i64));
    }

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|b| b.as_ref() as &dyn rusqlite::types::ToSql).collect();

    let rows = match stmt.query_map(params_refs.as_slice(), |row| {
        Ok((
            row.get::<_, Option<String>>(0)?, // hash
            row.get::<_, Option<String>>(1)?, // name
            row.get::<_, Option<String>>(2)?, // kind
            row.get::<_, Option<String>>(3)?, // language
            row.get::<_, Option<String>>(4)?, // concept
            row.get::<_, Option<String>>(5)?, // signature
            row.get::<_, String>(6)?,         // body
            row.get::<_, Option<String>>(7)?, // source_path
        ))
    }) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nom: query execution error: {e}");
            return 1;
        }
    };

    let mut total = 0usize;
    let mut imported = 0usize;
    let mut skipped_small = 0usize;
    let mut skipped_dup = 0usize;

    for row_result in rows {
        let (hash, name, kind, lang, conc, signature, body, source_path) = match row_result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("nom: row error: {e}");
                continue;
            }
        };

        total += 1;

        let name = match name {
            Some(n) if !n.is_empty() => n,
            _ => {
                skipped_small += 1;
                continue;
            }
        };

        if body.len() < min_body_len {
            skipped_small += 1;
            continue;
        }

        let lang_str = lang.unwrap_or_else(|| "unknown".to_owned());
        let kind_str = kind.unwrap_or_else(|| "function".to_owned());

        // Derive word and variant
        let (word, variant) = if let Some(ref c) = conc {
            (name.clone(), Some(c.clone()))
        } else {
            (name.clone(), Some(kind_str.clone()))
        };

        // Build description
        let describe = if let Some(ref c) = conc {
            format!("implementation of {c}: {name}")
        } else {
            format!("{kind_str} {name}")
        };

        // Extract input_type, output_type, effects from signature JSON
        let (input_type, output_type, effects) = if let Some(ref sig) = signature {
            parse_signature(sig)
        } else {
            (None, None, Vec::new())
        };

        // Compute quality score heuristic
        let lang_quality = match lang_str.as_str() {
            "rust" => 0.9,
            "c" | "go" => 0.8,
            "python" => 0.7,
            "javascript" | "typescript" => 0.6,
            "cpp" => 0.8,
            _ => 0.5,
        };
        let body_bonus = (body.len() as f64 / 5000.0).min(0.1);
        let quality = lang_quality + body_bonus;

        if dry_run {
            imported += 1;
            if total % 10_000 == 0 {
                println!("nom: scanned {total} atoms ({imported} matched)...");
            }
            continue;
        }

        // Single unified upsert into nomtu
        let entry = NomtuEntry {
            word: word.clone(),
            variant,
            describe: Some(describe),
            input_type,
            output_type,
            effects,
            security: lang_quality,
            performance: lang_quality,
            reliability: lang_quality,
            quality,
            hash,
            source: source_path.clone(),
            source_path,
            language: lang_str,
            body: Some(body),
            signature,
            ..NomtuEntry::default()
        };

        if let Err(e) = resolver.upsert(&entry) {
            eprintln!("nom: upsert error for {word}: {e}");
            skipped_dup += 1;
            continue;
        }

        imported += 1;

        if total % 10_000 == 0 {
            println!("nom: processed {total} atoms ({imported} imported)...");
        }
    }

    println!();
    println!("nom: import complete");
    println!("  total scanned: {total}");
    if dry_run {
        println!("  matched (dry run): {imported}");
    } else {
        println!("  imported:          {imported}");
    }
    println!("  skipped (small):   {skipped_small}");
    println!("  skipped (dup):     {skipped_dup}");

    0
}

/// Parse a JSON signature string to extract input_type, output_type, and effects.
fn parse_signature(sig: &str) -> (Option<String>, Option<String>, Vec<String>) {
    let v: serde_json::Value = match serde_json::from_str(sig) {
        Ok(v) => v,
        Err(_) => return (None, None, Vec::new()),
    };

    let input_type = v.get("inputs")
        .and_then(|v| {
            if v.is_array() {
                Some(v.to_string())
            } else {
                v.as_str().map(|s| s.to_owned())
            }
        });

    let output_type = v.get("outputs")
        .and_then(|v| {
            if v.is_array() {
                Some(v.to_string())
            } else {
                v.as_str().map(|s| s.to_owned())
            }
        });

    let effects = v.get("effects")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    (input_type, output_type, effects)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_source(file: &PathBuf) -> Option<String> {
    match std::fs::read_to_string(file) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("nom: cannot read {}: {e}", file.display());
            None
        }
    }
}

fn open_resolver(dict: &PathBuf) -> Option<Resolver> {
    let path = dict.to_str().unwrap_or("nomdict.db");
    match Resolver::open(path) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("nom: cannot open dict {}: {e}", dict.display());
            None
        }
    }
}
