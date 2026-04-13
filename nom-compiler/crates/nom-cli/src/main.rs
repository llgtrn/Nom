//! nom-cli: The Nom language compiler command-line interface.
//!
//! Commands:
//!   nom run <file>      — compile and run a .nom file
//!   nom build <file>    — compile to .nomiz plan
//!   nom check <file>    — typecheck and verify contracts
//!   nom test <file>     — run test declarations in <file>
//!   nom report <file>   — security report for <file>
//!   nom dict <query>    — search the local nomdict database
//!   nom precompile      — pre-compile .nomtu bodies to LLVM bitcode (.bc)
//!   nom extract <dir>   — extract .nomtu from source files in a directory
//!   nom score           — score all .nomtu in the dictionary
//!   nom stats           — show dictionary statistics
//!   nom coverage <dir>  — show extraction coverage for a directory
//!   nom translate       — translate .nomtu bodies from other languages to Rust
//!   nom audit           — deep security audit of all .nomtu bodies in the dictionary
//!   nom fmt <path>      — format .nom source files with canonical style

mod author;
mod build;
mod concept;
mod corpus;
mod fmt;
mod locale;
mod manifest;
mod mcp;
mod media;
mod report;
mod store;

use clap::{Parser, Subcommand};
use nom_codegen::{CodegenOptions, collect_dependencies, generate};
use nom_dict::NomDict;
use nom_extract;
use nom_graph::NomtuGraph;
use nom_score;
use nom_search::BM25Index;
use std::collections::HashMap;

use nom_parser::parse_source;
use nom_planner::Planner;
use nom_resolver::{NomtuEntry, Resolver};
use nom_security::{SecurityChecker, SecurityConfig, Severity, scan_body, security_score};
use nom_verifier::Verifier;
use rusqlite::OpenFlags;
use sha2::{Digest, Sha256};
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
        /// Compilation target: rust (default), llvm
        #[arg(long, default_value = "rust")]
        target: String,
        /// Skip loading the standard prelude (Result, Option types)
        #[arg(long)]
        no_prelude: bool,
    },

    /// Build subcommands: compile a .nom file or query build status.
    Build {
        #[command(subcommand)]
        action: BuildCmd,
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
        /// Execute test flows (compile and run, not just verify)
        #[arg(long)]
        execute: bool,
        /// Generate property-based tests from contract pre/post conditions
        #[arg(long)]
        property: bool,
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
        /// Search query (searches describe column, or contract signature with --contract)
        query: String,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: usize,
        /// Search by contract shape instead of keyword (e.g., "bytes -> hash")
        #[arg(long)]
        contract: bool,
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

    /// Pre-compile .nomtu bodies to LLVM bitcode (.bc)
    Precompile {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Output directory for .bc files
        #[arg(long, default_value = ".nom-bc")]
        output_dir: PathBuf,
        /// Only precompile entries matching this word
        #[arg(long)]
        word: Option<String>,
        /// Only precompile entries of this language
        #[arg(long)]
        language: Option<String>,
        /// Maximum entries to precompile (0 = all)
        #[arg(long, default_value = "0")]
        limit: usize,
        /// Dry run: show what would be compiled
        #[arg(long)]
        dry_run: bool,
    },

    /// Extract .nomtu from source files in a directory
    Extract {
        /// Directory to scan for parseable files
        dir: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Maximum files to process (0 = unlimited)
        #[arg(long, default_value = "0")]
        limit: usize,
    },

    /// Score all .nomtu in the dictionary
    Score {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },

    /// Show dictionary statistics
    Stats {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },

    /// Show extraction coverage for a directory
    Coverage {
        /// Directory to check coverage for
        dir: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },

    /// Translate .nomtu bodies from other languages to Rust
    Translate {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Only translate entries of this language
        #[arg(long)]
        language: Option<String>,
        /// Maximum entries to translate (0 = all)
        #[arg(long, default_value = "0")]
        limit: usize,
        /// Minimum confidence to accept (0.0-1.0)
        #[arg(long, default_value = "0.3")]
        min_confidence: f64,
        /// Dry run: show translations without writing
        #[arg(long)]
        dry_run: bool,
    },

    /// Build .nomtu knowledge graph and detect communities
    Graph {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Maximum entries to analyze (0 = all)
        #[arg(long, default_value = "0")]
        limit: usize,
    },

    /// Search the dictionary with hybrid BM25 + semantic matching
    Search {
        /// Search query
        query: String,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Audit all .nomtu in the dictionary for security issues
    Audit {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Minimum severity to report (info, low, medium, high, critical)
        #[arg(long, default_value = "medium")]
        min_severity: String,
        /// Maximum entries to scan (0 = all)
        #[arg(long, default_value = "0")]
        limit: usize,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Comprehensive quality assessment of a .nom source file
    Quality {
        /// Path to the .nom source file
        file: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },
    /// Format .nom source files with canonical style
    Fmt {
        /// Path to the .nom file (or directory to format all .nom files)
        path: PathBuf,
        /// Check mode: don't write, just report if file would change
        #[arg(long)]
        check: bool,
    },

    /// v2 content-addressed store operations (add / get / closure / verify / gc)
    Store {
        #[command(subcommand)]
        action: StoreCmd,
    },

    /// Ingest media files via the §5.16 codec landings.
    Media {
        #[command(subcommand)]
        action: MediaCmd,
    },

    /// Corpus ingestion per §5.17.
    Corpus {
        #[command(subcommand)]
        action: CorpusCmd,
    },

    /// Start the Model Context Protocol server on stdio. LLMs connect
    /// to query the dict and discover nomtu words for use in .nom source.
    /// Speaks line-delimited JSON-RPC 2.0 (MCP 2024-11-05).
    Mcp {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },

    /// Manage concepts — named domain groupings of nomtu entries. Each
    /// concept name is a first-class Nom syntax token, addressable via
    /// `use <concept>@<hash>` in .nom source.
    Concept {
        #[command(subcommand)]
        action: ConceptCmd,
    },

    /// App-composition commands. Compiling an app fans out into one
    /// file per aspect (core / security / ux / env / bizlogic / bench /
    /// response / flow / optimize / criteria) — never a god file.
    App {
        #[command(subcommand)]
        action: AppCmd,
    },

    /// Author a Nom program by starting from a brainstorm `.md` file
    /// and gradually replacing prose with Nom syntax. Per user
    /// directive: "nom is kinda naturally coding language".
    Author {
        #[command(subcommand)]
        action: AuthorCmd,
    },

    /// Locale pack management (M3 scaffold).
    Locale {
        #[command(subcommand)]
        action: LocaleCmd,
    },
}

#[derive(Subcommand)]
enum BuildCmd {
    /// Compile a .nom file to a native binary.
    Compile {
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
        /// Compilation target: rust (default), llvm, native
        #[arg(long, default_value = "rust")]
        target: String,
        /// Skip loading the standard prelude (Result, Option types)
        #[arg(long)]
        no_prelude: bool,
    },

    /// Load a concept's closure from the DB and report its build-readiness.
    /// Reads DB rows written by `nom store sync`; no compilation is performed.
    Status {
        /// Repo path (its basename is used as repo_id, matching what sync stored)
        repo: PathBuf,
        /// Path to the nomdict database (default: nomdict.db)
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Only report on this concept name (error if not found)
        #[arg(long)]
        concept: Option<String>,
        /// Rewrite source .nom files to pin resolved prose-matching refs to
        /// their content-addressed hash (idempotent; per doc 08 §8.2).
        #[arg(long = "write-locks")]
        write_locks: bool,
    },

    /// Emit a JSON build manifest for the given repo, derived from the
    /// closure walker + stub resolver + MECE pipeline.  The manifest is
    /// the Phase-5 planner input: build_order is post-order (leaves
    /// first) so a downstream compiler can build bottom-up.
    Manifest {
        /// Path to the repo (its basename is used as repo_id).
        repo: PathBuf,
        /// Path to the nomdict database (default: nomdict.db).
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Restrict to one concept (default: all concepts in repo).
        #[arg(long)]
        concept: Option<String>,
        /// Write JSON to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Pretty-print JSON (default: compact).
        #[arg(long)]
        pretty: bool,
    },

    /// Emit a full auditable report combining per-slot resolution trace,
    /// rejection reasons, alternatives, MECE outcome, effect list, and
    /// provenance trail.  Both JSON and human-readable forms are supported.
    /// Per deferred 06 §Phase 7 / doc 09 M1.
    Report {
        /// Path to the repo (its basename is used as repo_id).
        repo: PathBuf,
        /// Path to the nomdict database (default: nomdict.db).
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Restrict to one concept (default: all concepts in repo).
        #[arg(long)]
        concept: Option<String>,
        /// Write output to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Output format: human (default) or json.
        #[arg(long, default_value = "human")]
        format: String,
    },

    /// Compare acceptance predicates from a prior `nom build report --format json`
    /// output against the current build.  Exits 0 if all prior predicates are
    /// preserved; exits 1 if any were dropped (structural violation).
    ///
    /// Rewordings and additions are informational only.
    /// Runtime semantic check is deferred to Phase-8.
    /// Per doc 09 M2.
    VerifyAcceptance {
        /// Path to the repo (its basename is used as repo_id).
        repo: PathBuf,
        /// Path to the nomdict database (default: nomdict.db).
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Path to a prior `nom build report --format json` output (baseline).
        #[arg(long)]
        prior: PathBuf,
        /// Restrict to one concept (default: all concepts in repo).
        #[arg(long)]
        concept: Option<String>,
    },
}

#[derive(Subcommand)]
enum AuthorCmd {
    /// Create a scratch `<name>.md` file with a brainstorm template.
    Start {
        /// Program name (ascii alnum + underscore).
        name: String,
        /// Output directory (default: current dir).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Report how much of a `.md` brainstorm is already Nom syntax
    /// (classifies each line as comment / nom-ish / prose). For a
    /// `.nom` file, verifies it contains no residual prose.
    Check {
        file: PathBuf,
        /// Emit JSON summary.
        #[arg(long)]
        json: bool,
    },
    /// Translate any natural input (draft / essay / sentence) into a
    /// production-ready artifact (app / video / image). Scaffold form
    /// inspects input + emits the next LLM step; full LLM loop via
    /// MCP arrives as downstream consumers wire in. Per the
    /// 2026-04-13 prose→artifact directive.
    Translate {
        /// Input file (treated as prose — markdown / plain text).
        input: PathBuf,
        /// Target artifact form.
        #[arg(long, default_value = "app")]
        target: String,
        /// Emit JSON plan instead of a human summary.
        #[arg(long)]
        json: bool,
        /// If set, materialize every extracted proposal into the dict
        /// (concept + Partial entry + membership link) idempotently.
        /// Runs the prose→artifact lockstep: concept layer and nomtu
        /// layer land together, one row each per proposal.
        #[arg(long)]
        write: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum AppCmd {
    /// Dreaming mode: compile the manifest, enumerate criteria
    /// proposals, print them as a to-do list for the LLM, and exit.
    /// The outer agent author the suggested nomtu then re-invokes
    /// `dream` until `is_epic == true`. Per §5.12 + user directive.
    Dream {
        /// App manifest hash (root of the closure).
        manifest_hash: String,
        /// Human-readable app name.
        #[arg(long, default_value = "app")]
        name: String,
        /// Default target (web, desktop, mobile).
        #[arg(long, default_value = "web")]
        target: String,
        /// Root page entry hash.
        #[arg(long, default_value = "")]
        root: String,
        /// Extra closure roots.
        #[arg(long = "include")]
        includes: Vec<String>,
        /// Path to the nomdict database.
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Dream tier: app (default, recursive), concept (single concept), module (single .nomtu).
        #[arg(long, default_value = "app")]
        tier: String,
        /// For tier=concept|module, the concept word or module hash to dream.
        #[arg(long)]
        target_id: Option<String>,
        /// Repository ID to materialize the concept graph from the dict.
        /// When provided, enables recursive child-concept dreaming via
        /// `layered_dream_app_recursive` / `layered_dream_concept_recursive`.
        #[arg(long)]
        repo_id: Option<String>,
        /// Print the Pareto front of child-concept candidates after the
        /// dream summary. No-op when --json is set (JSON already includes it).
        #[arg(long)]
        pareto_front: bool,
    },

    /// Emit one artifact per OutputAspect at the given output directory.
    /// Security populates real findings from the dict closure; other
    /// aspects are empty scaffolds today.
    Build {
        /// App manifest hash (root of the closure).
        manifest_hash: String,
        /// Human-readable name for the manifest; identity stays the hash.
        #[arg(long, default_value = "app")]
        name: String,
        /// Default target platform (web, desktop, mobile).
        #[arg(long, default_value = "web")]
        target: String,
        /// Root page entry hash (treated as closure root).
        #[arg(long, default_value = "")]
        root: String,
        /// Extra root hashes (data sources, actions, media assets).
        /// Pass multiple times: `--include H1 --include H2`.
        #[arg(long = "include")]
        includes: Vec<String>,
        /// Path to the nomdict database.
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Output directory (created if missing).
        #[arg(long, default_value = ".nom-out")]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum CorpusCmd {
    /// Walk a directory and report per-language file/byte counts.
    /// Read-only; no dict writes.
    Scan {
        /// Path to the directory (e.g. a cloned upstream repo).
        path: PathBuf,
        /// Emit JSON instead of a table.
        #[arg(long)]
        json: bool,
    },

    /// Walk a directory, hash each source file, and upsert one v2 Entry
    /// per file into the nomdict (§5.17 source ingestion).
    Ingest {
        /// Path to the directory to ingest.
        path: PathBuf,
        /// Path to the nomdict database (default: nomdict.db).
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit a JSON summary instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },

    /// Walk every immediate child directory of the given parent, ingest
    /// each one into the nomdict, and aggregate the results. Reuses a
    /// single dict connection across all repos for performance.
    /// Designed for pre-staged corpus directories (e.g. 231 upstream repos).
    ///
    /// A checkpoint file is maintained next to the dict DB so a crash
    /// mid-run can be resumed without re-processing already-committed repos.
    /// Use `--reset-checkpoint` to discard the checkpoint and start fresh.
    IngestParent {
        /// Path to the parent directory whose immediate children are repos.
        path: PathBuf,
        /// Path to the nomdict database (default: nomdict.db).
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Delete the checkpoint file before ingesting, forcing a full
        /// re-scan of all repos even if some were already committed.
        #[arg(long)]
        reset_checkpoint: bool,
        /// Emit a JSON summary instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },

    /// Shallow-clone a git URL, ingest into the dict, then delete the
    /// clone. Stream-and-discard disk discipline (§5.17): peak disk =
    /// max(clone size, current dict).
    CloneIngest {
        /// HTTPS or SSH git URL.
        url: String,
        /// Path to the nomdict database.
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit JSON summary.
        #[arg(long)]
        json: bool,
    },

    /// Read a newline-separated list of git URLs (# comments allowed)
    /// and clone-and-ingest each in turn. Failures are recorded and the
    /// loop continues.
    CloneBatch {
        /// Path to a text file, one URL per line.
        list: PathBuf,
        /// Path to the nomdict database.
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit JSON summary.
        #[arg(long)]
        json: bool,
    },

    /// Clone-and-ingest the first N entries of the curated PyPI top
    /// list. Uses the same stream-and-discard discipline as
    /// `clone-batch`. The list is baked into nom-corpus so the command
    /// works offline and is deterministic across runs; refresh the list
    /// by editing `PYPI_TOP_URLS` in nom-corpus.
    IngestPypi {
        /// How many top-list entries to ingest (clamped to list length).
        #[arg(long, default_value = "10")]
        top: usize,
        /// Path to the nomdict database.
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit JSON summary.
        #[arg(long)]
        json: bool,
    },

    /// Register a required quality axis for MECE CE-check (M7a).
    ///
    /// Registration is idempotent: re-registering the same
    /// (repo_id, scope, axis) tuple updates the cardinality in place.
    /// Axis labels are normalised to trimmed lowercase before storage.
    RegisterAxis {
        /// The quality axis label (e.g. "security", "safety", "performance").
        axis: String,
        /// Composition scope: "app" | "concept" | "module".
        #[arg(long)]
        scope: String,
        /// Required cardinality: "at_least_one" | "exactly_one".
        #[arg(long)]
        cardinality: String,
        /// Repository identifier (default: "default").
        #[arg(long, default_value = "default")]
        repo_id: String,
        /// Path to the nomdict database.
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },

    /// List required quality axes registered for MECE CE-check (M7a).
    ListAxes {
        /// Composition scope to query: "app" | "concept" | "module".
        #[arg(long)]
        scope: String,
        /// Repository identifier (default: "default").
        #[arg(long, default_value = "default")]
        repo_id: String,
        /// Path to the nomdict database.
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },
}

#[derive(Subcommand)]
enum StoreCmd {
    /// Parse, canonicalize, and upsert a .nom file into the v2 store.
    Add {
        /// Path to the .nom source file
        path: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit a JSON record instead of the bare id
        #[arg(long)]
        json: bool,
    },
    /// Fetch a stored entry by id (or hash prefix ≥ 8 hex chars).
    Get {
        /// Full 64-char id or ≥ 8-char unique prefix
        hash: String,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
    },
    /// Walk the transitive closure of `entry_refs` from the given root.
    Closure {
        /// Root hash (full id or ≥ 8-char unique prefix)
        hash: String,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit a JSON array of ids
        #[arg(long)]
        json: bool,
    },
    /// Verify reachability and report partial/opaque/broken entries.
    Verify {
        /// Root hash (full id or ≥ 8-char unique prefix)
        hash: String,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Treat partial/opaque leaves as failures (exit code 2)
        #[arg(long)]
        strict: bool,
    },
    /// Garbage-collect entries not reachable from any root in ~/.nom/roots.txt.
    Gc {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Print candidates without deleting
        #[arg(long)]
        dry_run: bool,
    },
    /// Summarize dict state: total entries, body_kind histogram.
    Stats {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit JSON instead of a table
        #[arg(long)]
        json: bool,
    },
    /// List stored entries with optional filters.
    List {
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Filter: canonical §4.4.6 body_kind tag.
        #[arg(long)]
        body_kind: Option<String>,
        /// Filter: source language (rust, typescript, python, …).
        #[arg(long)]
        language: Option<String>,
        /// Filter: entry status (complete, partial, opaque).
        #[arg(long)]
        status: Option<String>,
        /// Filter: entry kind (function, module, media_unit, …).
        #[arg(long)]
        kind: Option<String>,
        /// Max entries to return.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Emit one JSON record per line.
        #[arg(long)]
        json: bool,
    },
    /// Walk a repo directory for `.nom` and `.nomtu` files and upsert
    /// parsed rows into `concept_defs` (DB1) and `words_v2` (DB2-v2).
    /// Idempotent: re-running produces the same DB state.
    Sync {
        /// Path to the repo directory to walk
        repo: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },
    /// Ingest a media file (PNG/JPEG/AVIF/FLAC/Opus/AAC/AV1/WebM/MP4/HEVC)
    /// and persist its canonical bytes to the DIDS store with the
    /// matching body_kind tag per §4.4.6 invariant 17.
    ///
    /// Still images (PNG/JPEG/BMP/TIFF/WebP/…) are encoded to canonical AVIF
    /// by default (modality-canonical track). Use `--preserve-format` to
    /// store PNG→PNG, JPEG→JPEG, etc. instead.
    AddMedia {
        /// Path to the media file
        path: PathBuf,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
        /// Emit JSON instead of human-readable output
        #[arg(long)]
        json: bool,
        /// Store in per-format encoding instead of modality-canonical AVIF.
        /// When set, PNG is stored as PNG, JPEG as JPEG, etc. By default
        /// all still images are re-encoded to canonical AVIF (§4.4.6 inv 17).
        #[arg(long)]
        preserve_format: bool,
    },
}

#[derive(Subcommand)]
enum MediaCmd {
    /// Ingest a single file. Detects format from extension; dispatches
    /// to the matching nom-media codec ingester. Prints metadata + the
    /// canonical-bytes size. Does NOT yet persist to the dict.
    Import {
        /// Path to the media file
        path: PathBuf,
        /// Emit JSON instead of human-readable output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ConceptCmd {
    /// Create a new empty concept and register it as an Entry (kind=concept)
    /// so it is addressable via `use <name>@<hash>` in .nom source.
    New {
        /// Human-readable concept name (e.g. "cryptography")
        name: String,
        /// Optional description for this concept
        #[arg(long)]
        describe: Option<String>,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },
    /// Add one entry (by full id or ≥8-char hex prefix) to a concept.
    Add {
        /// Concept name
        concept: String,
        /// Entry id (full 64-hex or ≥8-char prefix)
        entry: String,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },
    /// Bulk-add entries matching filter flags to a concept.
    /// Note: --describe-like is exclusive with structural filters.
    AddBy {
        /// Concept name
        concept: String,
        /// Filter by source language (rust, typescript, …)
        #[arg(long)]
        language: Option<String>,
        /// Filter by entry kind (function, module, …)
        #[arg(long)]
        kind: Option<String>,
        /// Filter by body_kind tag (bc, avif, …)
        #[arg(long)]
        body_kind: Option<String>,
        /// Filter by status (complete, partial, opaque)
        #[arg(long)]
        status: Option<String>,
        /// Substring match on the describe field (exclusive with structural filters)
        #[arg(long)]
        describe_like: Option<String>,
        /// Maximum entries to consider
        #[arg(long, default_value_t = 10000)]
        limit: usize,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },
    /// List all concepts with member counts. Pass `--empty` to show
    /// only orphan concepts (member_count = 0) — useful as a cleanup
    /// check after `translate --write` sessions.
    List {
        /// Emit JSON instead of a table
        #[arg(long)]
        json: bool,
        /// Filter to only show concepts with zero members.
        #[arg(long)]
        empty: bool,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },
    /// Show members of a concept.
    Show {
        /// Concept name
        name: String,
        /// Maximum members to display
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Emit JSON
        #[arg(long)]
        json: bool,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },
    /// Remove a concept entirely (entries are preserved; only the grouping is dropped).
    Delete {
        /// Concept name
        name: String,
        /// Path to the nomdict database
        #[arg(long, default_value = "nomdict.db")]
        dict: PathBuf,
    },
}

#[derive(Subcommand)]
enum LocaleCmd {
    /// List all registered locale packs.
    List,
    /// Parse and validate a BCP 47 locale tag.
    Validate {
        /// BCP 47 locale tag to validate (e.g. "vi-VN", "zh-Hant-TW").
        tag: String,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Commands::Run { file, dict, target, no_prelude } => cmd_run(&file, &dict, &target, no_prelude),
        Commands::Build { action } => match action {
            BuildCmd::Compile {
                file,
                output,
                dict,
                emit_rust,
                compile,
                release,
                target,
                no_prelude,
            } => cmd_build(&file, output.as_deref(), &dict, emit_rust, compile, release, &target, no_prelude),
            BuildCmd::Status { repo, dict, concept, write_locks } => {
                build::cmd_build_status(&repo, &dict, concept.as_deref(), write_locks)
            }
            BuildCmd::Manifest { repo, dict, concept, out, pretty } => {
                build::cmd_build_manifest(
                    &repo,
                    &dict,
                    concept.as_deref(),
                    out.as_deref(),
                    pretty,
                )
            }
            BuildCmd::Report { repo, dict, concept, out, format } => {
                report::cmd_build_report(
                    &repo,
                    &dict,
                    concept.as_deref(),
                    out.as_deref(),
                    &format,
                )
            }
            BuildCmd::VerifyAcceptance { repo, dict, prior, concept } => {
                build::cmd_build_verify_acceptance(
                    &repo,
                    &dict,
                    &prior,
                    concept.as_deref(),
                )
            }
        },
        Commands::Check { file, dict } => cmd_check(&file, &dict),
        Commands::Test { file, dict, filter, execute, property } => {
            if property {
                cmd_property_test(&file)
            } else {
                cmd_test(&file, &dict, filter.as_deref(), execute)
            }
        }
        Commands::Report {
            file,
            dict,
            min_security,
            format,
        } => cmd_report(&file, &dict, min_security, &format),
        Commands::Dict {
            query,
            dict,
            limit,
            contract,
        } => cmd_dict(&query, &dict, limit, contract),
        Commands::Import {
            source,
            dict,
            language,
            concept,
            min_body_len,
            limit,
            dry_run,
        } => cmd_import(
            &source,
            &dict,
            language.as_deref(),
            concept.as_deref(),
            min_body_len,
            limit,
            dry_run,
        ),
        Commands::Precompile {
            dict,
            output_dir,
            word,
            language,
            limit,
            dry_run,
        } => cmd_precompile(
            &dict,
            &output_dir,
            word.as_deref(),
            language.as_deref(),
            limit,
            dry_run,
        ),
        Commands::Extract { dir, dict, limit } => cmd_extract(&dir, &dict, limit),
        Commands::Score { dict } => cmd_score(&dict),
        Commands::Stats { dict } => cmd_stats(&dict),
        Commands::Coverage { dir, dict } => cmd_coverage(&dir, &dict),
        Commands::Translate {
            dict,
            language,
            limit,
            min_confidence,
            dry_run,
        } => cmd_translate(&dict, language.as_deref(), limit, min_confidence, dry_run),
        Commands::Graph { dict, limit } => cmd_graph(&dict, limit),
        Commands::Search { query, dict, limit } => cmd_search(&query, &dict, limit),
        Commands::Audit {
            dict,
            min_severity,
            limit,
            format,
        } => cmd_audit(&dict, &min_severity, limit, &format),
        Commands::Quality { file, dict } => cmd_quality(&file, &dict),
        Commands::Fmt { path, check } => cmd_fmt(&path, check),
        Commands::Store { action } => match action {
            StoreCmd::Add { path, dict, json } => store::cmd_store_add(&path, &dict, json),
            StoreCmd::Get { hash, dict, json } => store::cmd_store_get(&hash, &dict, json),
            StoreCmd::Closure { hash, dict, json } => store::cmd_store_closure(&hash, &dict, json),
            StoreCmd::Verify { hash, dict, strict } => store::cmd_store_verify(&hash, &dict, strict),
            StoreCmd::Gc { dict, dry_run } => store::cmd_store_gc(&dict, dry_run),
            StoreCmd::Stats { dict, json } => store::cmd_store_stats(&dict, json),
            StoreCmd::List { dict, body_kind, language, status, kind, limit, json } => {
                store::cmd_store_list(
                    &dict,
                    body_kind.as_deref(),
                    language.as_deref(),
                    status.as_deref(),
                    kind.as_deref(),
                    limit,
                    json,
                )
            }
            StoreCmd::Sync { repo, dict } => store::cmd_store_sync(&repo, &dict),
            StoreCmd::AddMedia { path, dict, json, preserve_format } => store::cmd_store_add_media(&path, &dict, json, preserve_format),
        },
        Commands::Media { action } => match action {
            MediaCmd::Import { path, json } => media::cmd_media_import(&path, json),
        },
        Commands::Corpus { action } => match action {
            CorpusCmd::Scan { path, json } => corpus::cmd_corpus_scan(&path, json),
            CorpusCmd::Ingest { path, dict, json } => {
                corpus::cmd_corpus_ingest(&path, &dict, json)
            }
            CorpusCmd::IngestParent { path, dict, reset_checkpoint, json } => {
                corpus::cmd_corpus_ingest_parent(&path, &dict, reset_checkpoint, json)
            }
            CorpusCmd::CloneIngest { url, dict, json } => {
                corpus::cmd_corpus_clone_ingest(&url, &dict, json)
            }
            CorpusCmd::CloneBatch { list, dict, json } => {
                corpus::cmd_corpus_clone_batch(&list, &dict, json)
            }
            CorpusCmd::IngestPypi { top, dict, json } => {
                corpus::cmd_corpus_ingest_pypi(top, &dict, json)
            }
            CorpusCmd::RegisterAxis { axis, scope, cardinality, repo_id, dict } => {
                corpus::cmd_corpus_register_axis(&axis, &scope, &cardinality, &repo_id, &dict)
            }
            CorpusCmd::ListAxes { scope, repo_id, dict } => {
                corpus::cmd_corpus_list_axes(&scope, &repo_id, &dict)
            }
        },
        Commands::Mcp { dict } => mcp::cmd_mcp_serve(&dict),
        Commands::Concept { action } => match action {
            ConceptCmd::New { name, describe, dict } => {
                concept::cmd_concept_new(&name, describe.as_deref(), &dict)
            }
            ConceptCmd::Add { concept, entry, dict } => {
                concept::cmd_concept_add(&concept, &entry, &dict)
            }
            ConceptCmd::AddBy {
                concept,
                language,
                kind,
                body_kind,
                status,
                describe_like,
                limit,
                dict,
            } => concept::cmd_concept_add_by(
                &concept,
                language.as_deref(),
                kind.as_deref(),
                body_kind.as_deref(),
                status.as_deref(),
                describe_like.as_deref(),
                limit,
                &dict,
            ),
            ConceptCmd::List { json, empty, dict } => {
                concept::cmd_concept_list_filtered(json, &dict, empty)
            }
            ConceptCmd::Show { name, limit, json, dict } => {
                concept::cmd_concept_show(&name, limit, json, &dict)
            }
            ConceptCmd::Delete { name, dict } => concept::cmd_concept_delete(&name, &dict),
        },
        Commands::Author { action } => match action {
            AuthorCmd::Start { name, out } => author::cmd_author_start(&name, out.as_deref()),
            AuthorCmd::Check { file, json } => author::cmd_author_check(&file, json),
            AuthorCmd::Translate { input, target, json, write } => {
                match author::TranslateTarget::from_str(&target) {
                    Some(t) => author::cmd_author_translate(&input, t, json, write.as_deref()),
                    None => {
                        eprintln!(
                            "nom author translate: unknown target `{target}` (expected app|video|image)"
                        );
                        1
                    }
                }
            }
        },
        Commands::App { action } => match action {
            AppCmd::Build { manifest_hash, name, target, root, includes, dict, out } => {
                cmd_app_build(&manifest_hash, &name, &target, &root, &includes, &dict, &out)
            }
            AppCmd::Dream { manifest_hash, name, target, root, includes, dict, json, tier, target_id, repo_id, pareto_front } => {
                cmd_app_dream(&manifest_hash, &name, &target, &root, &includes, &dict, json, &tier, target_id.as_deref(), repo_id.as_deref(), pareto_front)
            }
        },
        Commands::Locale { action } => match action {
            LocaleCmd::List => locale::cmd_locale_list(),
            LocaleCmd::Validate { tag } => locale::cmd_locale_validate(&tag),
        },
    };
    process::exit(exit_code);
}

fn cmd_app_dream(
    manifest_hash: &str,
    name: &str,
    target: &str,
    root: &str,
    includes: &[String],
    dict_path: &Path,
    json: bool,
    tier: &str,
    target_id: Option<&str>,
    repo_id: Option<&str>,
    show_pareto_front: bool,
) -> i32 {
    // Validate tier string early.
    let dream_tier = match nom_app::DreamTier::from_str(tier) {
        Some(t) => t,
        None => {
            eprintln!("nom: unknown dream tier '{tier}' (expected app|concept|module)");
            return 1;
        }
    };

    // tier=concept|module require --target-id.
    if matches!(dream_tier, nom_app::DreamTier::Concept | nom_app::DreamTier::Module)
        && target_id.is_none()
    {
        eprintln!("nom: --target-id required for tier={tier}");
        return 1;
    }

    // module tier is deferred to M5c.
    if matches!(dream_tier, nom_app::DreamTier::Module) {
        eprintln!("nom dream: not yet implemented (module-tier coming in M5b)");
        return 2;
    }

    let dict = if dict_path.exists() {
        match NomDict::open_in_place(dict_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("open dict {}: {e}", dict_path.display());
                return 1;
            }
        }
    } else {
        NomDict::open_in_memory().unwrap()
    };

    // Optionally materialize the concept graph when --repo-id is provided.
    // If materialization fails, exit 1 with a diagnostic message.
    let maybe_graph: Option<nom_concept::ConceptGraph> = if let Some(rid) = repo_id {
        match store::materialize_concept_graph_from_db(&dict, rid) {
            Ok(g) => Some(g),
            Err(e) => {
                eprintln!("nom: cannot materialize concept graph: {e}");
                return 1;
            }
        }
    } else {
        None
    };

    match dream_tier {
        nom_app::DreamTier::Concept => {
            let word = target_id.unwrap_or("");
            let layered = match &maybe_graph {
                Some(graph) => {
                    let mut seen = std::collections::HashSet::new();
                    nom_app::layered_dream_concept_recursive(word, &dict, graph, &mut seen)
                }
                None => nom_app::layered_dream_concept(word, &dict),
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&layered).unwrap_or_default()
                );
            } else if layered.leaf.is_epic {
                println!(
                    "✨ concept dream: {word} is epic — score {} ≥ {}.",
                    layered.leaf.app_score, layered.leaf.score_threshold
                );
            } else {
                println!(
                    "concept dream: {word} score {}/{} — {} proposal(s)",
                    layered.leaf.app_score,
                    layered.leaf.score_threshold,
                    layered.leaf.proposals.len()
                );
                if !layered.child_reports.is_empty() {
                    let epic_count = layered.child_reports.iter().filter(|r| r.leaf.is_epic).count();
                    let below = layered.child_reports.len() - epic_count;
                    println!(
                        "└─ {} child concept(s) dreamed ({} epic, {} below threshold)",
                        layered.child_reports.len(),
                        epic_count,
                        below
                    );
                }
            }
            if !json && show_pareto_front {
                if layered.pareto_front.is_empty() {
                    println!("Pareto front: empty (no children to compare).");
                } else {
                    println!("Pareto front ({} candidate(s)):", layered.pareto_front.len());
                    for (i, entry) in layered.pareto_front.iter().enumerate() {
                        println!("  {}. {}", i + 1, entry);
                    }
                }
            }
            if layered.leaf.is_epic { 0 } else { 2 }
        }
        nom_app::DreamTier::App => {
            let manifest = nom_app::AppManifest {
                manifest_hash: manifest_hash.to_string(),
                name: name.to_string(),
                default_target: target.to_string(),
                root_page_hash: root.to_string(),
                data_sources: includes.to_vec(),
                actions: vec![],
                media_assets: vec![],
                settings: serde_json::Value::Null,
            };
            let layered = match &maybe_graph {
                Some(graph) => nom_app::layered_dream_app_recursive(&manifest, &dict, graph),
                None => nom_app::layered_dream_app(&manifest, &dict),
            };
            let report = &layered.leaf;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&layered).unwrap_or_default()
                );
            } else if report.is_epic {
                println!(
                    "✨ dream: {name} is epic — score {} ≥ {} threshold.",
                    report.app_score, report.score_threshold
                );
            } else {
                println!(
                    "dream: {name} score {}/{} — {} proposal(s):",
                    report.app_score,
                    report.score_threshold,
                    report.proposals.len()
                );
                for (i, p) in report.proposals.iter().enumerate() {
                    println!("  {}. [{}] {}", i + 1, p.kind, p.rationale);
                    if let Some(sw) = &p.suggested_word {
                        let kind = p.suggested_entry_kind.as_deref().unwrap_or("?");
                        let concept = p.suggested_concept.as_deref().unwrap_or("-");
                        println!("     → author nomtu `{sw}` (kind={kind}, concept={concept})");
                    }
                }
                if !layered.child_reports.is_empty() {
                    let epic_count = layered.child_reports.iter().filter(|r| r.leaf.is_epic).count();
                    let below = layered.child_reports.len() - epic_count;
                    println!(
                        "└─ {} child concept(s) dreamed ({} epic, {} below threshold)",
                        layered.child_reports.len(),
                        epic_count,
                        below
                    );
                }
                println!();
                println!("{}", report.next_instruction);
            }
            if !json && show_pareto_front {
                if layered.pareto_front.is_empty() {
                    println!("Pareto front: empty (no children to compare).");
                } else {
                    println!("Pareto front ({} candidate(s)):", layered.pareto_front.len());
                    for (i, entry) in layered.pareto_front.iter().enumerate() {
                        println!("  {}. {}", i + 1, entry);
                    }
                }
            }
            if report.is_epic { 0 } else { 2 }
        }
        nom_app::DreamTier::Module => unreachable!("handled above"),
    }
}

fn cmd_app_build(
    manifest_hash: &str,
    name: &str,
    target: &str,
    root: &str,
    includes: &[String],
    dict_path: &Path,
    out: &Path,
) -> i32 {
    let manifest = nom_app::AppManifest {
        manifest_hash: manifest_hash.to_string(),
        name: name.to_string(),
        default_target: target.to_string(),
        root_page_hash: root.to_string(),
        data_sources: includes.to_vec(),
        actions: vec![],
        media_assets: vec![],
        settings: serde_json::Value::Null,
    };
    let artifacts_result = if dict_path.exists() {
        match NomDict::open_in_place(dict_path) {
            Ok(d) => nom_app::compile_app_to_artifacts_with_dict(&manifest, &d),
            Err(e) => {
                eprintln!("open dict {} failed: {e}", dict_path.display());
                return 1;
            }
        }
    } else {
        nom_app::compile_app_to_artifacts(&manifest)
    };
    let artifacts = match artifacts_result {
        Ok(a) => a,
        Err(e) => {
            eprintln!("app build failed: {e}");
            return 1;
        }
    };
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("cannot create {}: {e}", out.display());
        return 1;
    }
    for art in &artifacts {
        let path = out.join(&art.path);
        if let Err(e) = std::fs::write(&path, &art.bytes) {
            eprintln!("write {} failed: {e}", path.display());
            return 1;
        }
        println!("{:?} → {}", art.aspect, path.display());
    }
    println!("emitted {} aspect file(s) under {}", artifacts.len(), out.display());
    0
}

// ── Command implementations ───────────────────────────────────────────────────

fn cmd_run(file: &PathBuf, dict: &PathBuf, target: &str, no_prelude: bool) -> i32 {
    if target == "llvm" {
        return cmd_run_llvm(file, dict);
    }

    // Build first (compile = true, release = false), output next to the .nom file
    let rc = cmd_build(file, None, dict, false, true, false, "rust", no_prelude);
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

/// Run a .nom file using the LLVM backend via `lli` (LLVM bitcode interpreter).
fn cmd_run_llvm(file: &PathBuf, dict: &PathBuf) -> i32 {
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
    let mut plan = match planner.plan(&parsed) {
        Ok(p) => p,
        Err(nom_planner::PlanError::VerificationFailed(findings)) => {
            eprintln!("nom: verification failed ({} findings):", findings.len());
            for (i, finding) in findings.iter().enumerate() {
                eprintln!("  {}. {}", i + 1, finding);
            }
            return 1;
        }
        Err(e) => {
            eprintln!("nom: plan error: {e}");
            return 1;
        }
    };

    planner.enrich_with_implementations(&mut plan);

    // Compile to LLVM IR
    let llvm_out = match nom_llvm::compile(&plan) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("nom: LLVM compilation error: {e}");
            return 1;
        }
    };

    // Write bitcode to temp directory
    let temp_dir = std::env::temp_dir().join("nom-run");
    if let Err(e) = std::fs::create_dir_all(&temp_dir) {
        eprintln!("nom: could not create temp dir: {e}");
        return 1;
    }
    let bc_path = temp_dir.join("program.bc");
    if let Err(e) = std::fs::write(&bc_path, &llvm_out.bitcode) {
        eprintln!("nom: write error: {e}");
        return 1;
    }

    // Also write .ll for debugging
    let ll_path = temp_dir.join("program.ll");
    let _ = std::fs::write(&ll_path, &llvm_out.ir_text);

    println!("nom: compiled to LLVM IR ({} bytes bitcode)", llvm_out.bitcode.len());

    // Strategy 1: try lli (LLVM bitcode interpreter) - fastest path
    if let Ok(status) = process::Command::new("lli").arg(&bc_path).status() {
        return status.code().unwrap_or(1);
    }

    // Strategy 2: use clang to compile .ll to a temp binary and run it
    let exe_path = if cfg!(windows) {
        temp_dir.join("program.exe")
    } else {
        temp_dir.join("program")
    };

    let ll_str = ll_path.to_string_lossy().into_owned();
    let exe_str = exe_path.to_string_lossy().into_owned();
    match process::Command::new("clang")
        .args([ll_str.as_str(), "-o", exe_str.as_str(), "-O0"])
        .status()
    {
        Ok(s) if s.success() => {
            println!("nom: compiled native binary via clang");
            match process::Command::new(&exe_path).status() {
                Ok(status) => status.code().unwrap_or(1),
                Err(e) => {
                    eprintln!("nom: failed to run {}: {e}", exe_path.display());
                    1
                }
            }
        }
        Ok(s) => {
            eprintln!("nom: clang exited with {}", s);
            1
        }
        Err(_) => {
            eprintln!("nom: could not find lli or clang to execute LLVM bitcode");
            eprintln!("  hint: ensure LLVM is installed and in PATH");
            eprintln!("  try: set PATH to include C:\\Program Files\\LLVM\\bin");
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

/// Try to load and parse the standard prelude (Result, Option types).
/// Returns the prelude's declarations, or an empty vec if not found.
fn load_prelude(file: &Path) -> Vec<nom_ast::Declaration> {
    // Look for stdlib/prelude.nom relative to the nom-compiler directory,
    // or relative to the source file's parent, or via the executable path.
    let candidates = [
        // Relative to the source file being compiled
        file.parent().map(|p| p.join("../stdlib/prelude.nom")),
        file.parent().map(|p| p.join("stdlib/prelude.nom")),
        // Relative to the current working directory
        Some(PathBuf::from("stdlib/prelude.nom")),
        // Relative to the executable
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("stdlib/prelude.nom"))),
    ];

    for candidate in candidates.iter().flatten() {
        if let Ok(source) = std::fs::read_to_string(candidate) {
            match parse_source(&source) {
                Ok(sf) => {
                    eprintln!("nom: loaded prelude from {}", candidate.display());
                    return sf.declarations;
                }
                Err(e) => {
                    eprintln!("nom: warning: failed to parse prelude {}: {e}", candidate.display());
                    return Vec::new();
                }
            }
        }
    }

    // Prelude not found is not an error — just means no stdlib available
    Vec::new()
}

fn cmd_build(
    file: &PathBuf,
    output: Option<&Path>,
    dict: &PathBuf,
    emit_rust: bool,
    compile: bool,
    release: bool,
    target: &str,
    no_prelude: bool,
) -> i32 {
    // Hash-prefix shortcut: if the positional arg uniquely resolves to
    // a stored entry, materialize the closure bodies to a tempfile and
    // build as usual. If the resolution fails we fall through to the
    // regular path so a genuine file named e.g. "deadbeef" still works.
    if !file.exists() {
        if let Some(arg) = file.to_str() {
            if let Some(closure) = store::try_build_by_hash(arg, dict) {
                if let Some(body) = store::materialize_closure_body(dict, &closure) {
                    let tmp_dir = std::env::temp_dir().join("nom-build-hash");
                    if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
                        eprintln!("nom: temp dir error: {e}");
                        return 1;
                    }
                    // Prefix with "nom_" so the Rust package name derived
                    // from the file stem is a valid identifier (Cargo
                    // rejects names that start with a digit).
                    let tmp_file = tmp_dir.join(format!("nom_{}.nom", &arg[..8]));
                    if let Err(e) = std::fs::write(&tmp_file, &body) {
                        eprintln!("nom: write temp file error: {e}");
                        return 1;
                    }
                    println!("nom: materialized {} closure entries to {}", closure.len(), tmp_file.display());
                    return cmd_build(
                        &tmp_file,
                        output,
                        dict,
                        emit_rust,
                        compile,
                        release,
                        target,
                        no_prelude,
                    );
                }
            }
        }
    }

    let source = match read_source(file) {
        Some(s) => s,
        None => return 1,
    };

    let mut parsed = match parse_source(&source) {
        Ok(sf) => sf,
        Err(e) => {
            eprintln!("nom: parse error: {e}");
            return 1;
        }
    };

    // Load the standard prelude unless --no-prelude is specified
    if !no_prelude {
        let prelude_decls = load_prelude(file);
        if !prelude_decls.is_empty() {
            // Prepend prelude declarations before the user's declarations
            let mut all_decls = prelude_decls;
            all_decls.append(&mut parsed.declarations);
            parsed.declarations = all_decls;
        }
    }

    let resolver = match open_resolver(dict) {
        Some(r) => r,
        None => return 1,
    };

    let planner = Planner::new(&resolver);
    let mut plan = match planner.plan(&parsed) {
        Ok(p) => p,
        Err(nom_planner::PlanError::VerificationFailed(findings)) => {
            eprintln!("nom: verification failed ({} findings):", findings.len());
            for (i, finding) in findings.iter().enumerate() {
                eprintln!("  {}. {}", i + 1, finding);
            }
            return 1;
        }
        Err(e) => {
            eprintln!("nom: plan error: {e}");
            return 1;
        }
    };

    // Enrich plan nodes with real implementation bodies from the dictionary
    planner.enrich_with_implementations(&mut plan);

    // Always write .nomiz
    let nomiz_path = file.with_extension("nomiz");
    match std::fs::write(&nomiz_path, &plan.nomiz) {
        Ok(_) => println!("nom: wrote {}", nomiz_path.display()),
        Err(e) => {
            eprintln!("nom: write error: {e}");
            return 1;
        }
    }

    // ── LLVM / native target path ─────────────────────────────────────────
    if target == "llvm" || target == "native" {
        let llvm_out = match nom_llvm::compile(&plan) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("nom: LLVM compilation error: {e}");
                return 1;
            }
        };

        // Write .bc (bitcode)
        let bc_path = file.with_extension("bc");
        if let Err(e) = std::fs::write(&bc_path, &llvm_out.bitcode) {
            eprintln!("nom: write error: {e}");
            return 1;
        }
        println!("nom: wrote {}", bc_path.display());

        // Write .ll (human-readable IR)
        let ll_path = file.with_extension("ll");
        if let Err(e) = std::fs::write(&ll_path, &llvm_out.ir_text) {
            eprintln!("nom: write error: {e}");
            return 1;
        }
        println!("nom: wrote {}", ll_path.display());

        // If native target, try to invoke llc to compile to object file
        if target == "native" {
            let obj_path = file.with_extension("o");
            let status = std::process::Command::new("llc")
                .args([
                    "-filetype=obj",
                    &bc_path.to_string_lossy(),
                    "-o",
                    &obj_path.to_string_lossy(),
                ])
                .status();
            match status {
                Ok(s) if s.success() => {
                    println!("nom: wrote {}", obj_path.display());
                }
                Ok(s) => {
                    eprintln!("nom: llc exited with {}", s);
                    return 1;
                }
                Err(e) => {
                    eprintln!("nom: could not invoke llc: {e}");
                    eprintln!("  hint: ensure LLVM is installed and llc is in PATH");
                    return 1;
                }
            }
        }

        return 0;
    }

    // ── Rust target path (default) ──────────────────────────────────────────

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
        "[workspace]\n\n[package]\nname = \"{file_stem}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n"
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

    // Print warnings first
    for finding in &result.findings {
        if !finding.error.is_error() {
            eprintln!("  warning [{}] {}", finding.declaration, finding.error);
        }
    }

    if result.ok() {
        let wc = result.warning_count();
        if wc > 0 {
            println!("nom: check passed — 0 errors, {wc} warning(s)");
        } else {
            println!("nom: check passed — 0 findings");
        }
        0
    } else {
        for finding in &result.findings {
            if finding.error.is_error() {
                eprintln!("  [{}] {}", finding.declaration, finding.error);
            }
        }
        eprintln!(
            "nom: check failed — {} error(s), {} warning(s)",
            result.error_count(),
            result.warning_count()
        );
        1
    }
}

fn cmd_fmt(path: &Path, check: bool) -> i32 {
    if path.is_dir() {
        fmt_directory(path, check)
    } else {
        fmt_single_file(path, check)
    }
}

fn fmt_single_file(file: &Path, check: bool) -> i32 {
    let source = match read_source(&file.to_path_buf()) {
        Some(s) => s,
        None => return 1,
    };

    let formatted = match fmt::format_source(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("nom: parse error in {}: {e}", file.display());
            return 1;
        }
    };

    if check {
        if source != formatted {
            println!("would reformat: {}", file.display());
            1
        } else {
            println!("already formatted: {}", file.display());
            0
        }
    } else {
        if source == formatted {
            println!("unchanged: {}", file.display());
            return 0;
        }
        match std::fs::write(file, &formatted) {
            Ok(_) => {
                println!("formatted: {}", file.display());
                0
            }
            Err(e) => {
                eprintln!("nom: write error: {e}");
                1
            }
        }
    }
}

fn fmt_directory(dir: &Path, check: bool) -> i32 {
    let mut exit_code = 0;
    let mut count = 0;
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "nom") {
                    let rc = fmt_single_file(&path, check);
                    if rc != 0 {
                        exit_code = 1;
                    }
                    count += 1;
                } else if path.is_dir() {
                    let rc = fmt_directory(&path, check);
                    if rc != 0 {
                        exit_code = 1;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("nom: cannot read directory {}: {e}", dir.display());
            return 1;
        }
    }
    if count == 0 && exit_code == 0 {
        println!("nom: no .nom files found in {}", dir.display());
    }
    exit_code
}

fn cmd_quality(file: &PathBuf, dict: &PathBuf) -> i32 {
    use nom_ast::{Classifier, Statement};

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

    println!("nom quality report for {}", file.display());
    println!("========================================\n");

    // 1. Structure analysis
    let decl_count = parsed.declarations.len();
    let mut flow_count = 0usize;
    let mut need_count = 0usize;
    let mut fn_count = 0usize;
    let mut struct_count = 0usize;
    let mut enum_count = 0usize;
    let mut test_count = 0usize;
    let mut has_effects = false;
    let mut has_describe = false;
    let mut has_contract = false;

    for decl in &parsed.declarations {
        if decl.classifier == Classifier::Test {
            test_count += 1;
        }
        for stmt in &decl.statements {
            match stmt {
                Statement::Flow(_) => flow_count += 1,
                Statement::Need(_) => need_count += 1,
                Statement::FnDef(_) => fn_count += 1,
                Statement::StructDef(_) => struct_count += 1,
                Statement::EnumDef(_) => enum_count += 1,
                Statement::Effects(_) => has_effects = true,
                Statement::Describe(_) => has_describe = true,
                Statement::Contract(_) => has_contract = true,
                _ => {}
            }
        }
    }

    println!("  Structure:");
    println!("    declarations: {decl_count}");
    println!("    flows:        {flow_count}");
    println!("    needs:        {need_count}");
    println!("    functions:    {fn_count}");
    println!("    structs:      {struct_count}");
    println!("    enums:        {enum_count}");
    println!("    tests:        {test_count}");
    println!();

    // 2. Quality scoring
    let mut score = 100i32;
    let mut issues: Vec<String> = Vec::new();
    let mut strengths: Vec<String> = Vec::new();

    // Effects declared?
    if has_effects {
        strengths.push("effects declared (explicit side-effect tracking)".to_owned());
    } else if flow_count > 0 {
        score -= 10;
        issues.push("no effects declared — add 'effects [...]' to track side effects".to_owned());
    }

    // Describe present?
    if has_describe {
        strengths.push("describe present (human-readable documentation)".to_owned());
    } else {
        score -= 5;
        issues.push("no describe — add 'describe \"...\"' for documentation".to_owned());
    }

    // Contract present?
    if has_contract {
        strengths.push("contract defined (typed interface with pre/post conditions)".to_owned());
    } else if fn_count > 0 {
        score -= 5;
        issues.push("functions without contracts — add 'contract' for formal interface".to_owned());
    }

    // Tests present?
    if test_count > 0 {
        strengths.push(format!("{test_count} test(s) defined"));
    } else {
        score -= 15;
        issues.push("no tests — add 'test <name>' declarations".to_owned());
    }

    // Require constraints?
    let has_require = parsed.declarations.iter().any(|d| {
        d.statements.iter().any(|s| matches!(s, Statement::Require(_)))
    });
    if has_require {
        strengths.push("require constraints set (quality thresholds enforced)".to_owned());
    } else if need_count > 0 {
        score -= 10;
        issues.push("no require constraints — add 'require latency<50ms' etc.".to_owned());
    }

    // Where constraints on needs?
    let needs_with_where = parsed.declarations.iter().flat_map(|d| &d.statements).filter(|s| {
        matches!(s, Statement::Need(n) if n.constraint.is_some())
    }).count();
    if needs_with_where > 0 {
        strengths.push(format!("{needs_with_where}/{need_count} needs have 'where' quality constraints"));
    } else if need_count > 0 {
        score -= 10;
        issues.push("needs without 'where' constraints — add 'where security>0.9' etc.".to_owned());
    }

    // 3. Verification results
    let resolver = open_resolver(dict);
    if let Some(ref r) = resolver {
        let verifier = Verifier::new(r);
        let vresult = verifier.verify(&parsed);
        let errors = vresult.error_count();
        let warnings = vresult.warning_count();

        // Separate resolver errors (missing dictionary) from semantic errors
        let resolver_errors = vresult.findings.iter().filter(|f| {
            matches!(&f.error, nom_verifier::VerifyError::Resolver(_))
        }).count();
        let semantic_errors = errors.saturating_sub(resolver_errors);

        if semantic_errors == 0 && resolver_errors == 0 {
            strengths.push("verification passed (0 errors)".to_owned());
        } else if semantic_errors == 0 && resolver_errors > 0 {
            strengths.push("verification passed (0 semantic errors)".to_owned());
            issues.push(format!("{resolver_errors} unresolved word(s) — populate nomdict or run 'nom import'"));
            // Don't penalize heavily for missing dictionary
            score -= (resolver_errors as i32).min(5);
        } else {
            score -= semantic_errors as i32 * 10;
            issues.push(format!("{semantic_errors} verification error(s) — run 'nom check' for details"));
        }
        if warnings > 0 {
            issues.push(format!("{warnings} verification warning(s)"));
        }
    }

    // 4. Security analysis
    for decl in &parsed.declarations {
        for stmt in &decl.statements {
            if let Statement::Implement(impl_stmt) = stmt {
                let findings = scan_body(&impl_stmt.code, &impl_stmt.language);
                if findings.is_empty() {
                    strengths.push(format!("implement block in '{}' passes security scan", decl.name.name));
                } else {
                    let max_sev = findings.iter().map(|f| &f.severity).max();
                    score -= findings.len() as i32 * 5;
                    issues.push(format!(
                        "implement block in '{}': {} security finding(s), max severity: {:?}",
                        decl.name.name,
                        findings.len(),
                        max_sev.unwrap()
                    ));
                }
            }
        }
    }

    // Print results
    score = score.max(0).min(100);

    println!("  Strengths:");
    for s in &strengths {
        println!("    + {s}");
    }
    println!();

    if !issues.is_empty() {
        println!("  Issues:");
        for i in &issues {
            println!("    - {i}");
        }
        println!();
    }

    let grade = match score {
        90..=100 => "A",
        80..=89 => "B",
        70..=79 => "C",
        60..=69 => "D",
        _ => "F",
    };

    println!("  Quality Score: {score}/100 (grade {grade})");
    println!();

    if score >= 80 {
        println!("nom: quality check passed");
        0
    } else {
        println!("nom: quality below threshold (80) — address issues above");
        1
    }
}

fn cmd_property_test(file: &PathBuf) -> i32 {
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

    use nom_ast::Statement;
    use nom_verifier::generate_contract_tests;

    let mut total = 0usize;

    for decl in &parsed.declarations {
        for stmt in &decl.statements {
            if let Statement::Contract(contract) = stmt {
                let tests = generate_contract_tests(contract);
                if tests.is_empty() {
                    continue;
                }
                println!(
                    "\n  contract in '{}' — {} property tests generated:",
                    decl.name.name,
                    tests.len()
                );
                for test in &tests {
                    println!("    {} {}", "●", test.name);
                    println!("      {}", test.description);
                    if !test.input_constraints.is_empty() {
                        println!(
                            "      inputs:  {}",
                            test.input_constraints.join("; ")
                        );
                    }
                    if !test.expected_postconditions.is_empty() {
                        println!(
                            "      expect:  {}",
                            test.expected_postconditions.join("; ")
                        );
                    }
                }
                total += tests.len();
            }
        }
    }

    if total == 0 {
        println!("nom: no contracts found in {}", file.display());
    } else {
        println!(
            "\nnom: {} property test(s) generated from contracts in {}",
            total,
            file.display()
        );
    }
    0
}

fn cmd_test(file: &PathBuf, dict: &PathBuf, filter: Option<&str>, execute: bool) -> i32 {
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
    use nom_ast::{Classifier, Statement};

    let tests: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|d| d.classifier == Classifier::Test)
        .filter(|d| filter.map(|f| d.name.name.contains(f)).unwrap_or(true))
        .collect();

    if tests.is_empty() {
        println!("nom: no tests found");
        return 0;
    }

    // Open resolver for verification-level testing
    let resolver = open_resolver(dict);
    let verifier = resolver.as_ref().map(|r| Verifier::new(r));

    let mut passed = 0usize;
    let mut failed = 0usize;

    for test in &tests {
        let name = &test.name.name;
        let mut test_errors: Vec<String> = Vec::new();

        // Level 1: Verify the test declaration (type compatibility, constraints, effects)
        if let Some(ref v) = verifier {
            let vresult = v.verify(&nom_ast::SourceFile {
                path: parsed.path.clone(),
                locale: parsed.locale.clone(),
                declarations: vec![(*test).clone()],
            });
            for finding in &vresult.findings {
                test_errors.push(format!("verify: {}", finding.error));
            }
        }

        // Level 2: Evaluate then/and assertions
        // Assertions can reference resolved dictionary scores, e.g. `then security > 0.9`
        for stmt in &test.statements {
            match stmt {
                Statement::Then(then_stmt) => {
                    if let Err(reason) = eval_assertion(&then_stmt.assertion, resolver.as_ref(), test) {
                        test_errors.push(format!("then: {reason}"));
                    }
                }
                Statement::And(and_stmt) => {
                    if let Err(reason) = eval_assertion(&and_stmt.assertion, resolver.as_ref(), test) {
                        test_errors.push(format!("and: {reason}"));
                    }
                }
                _ => {}
            }
        }

        if test_errors.is_empty() {
            println!("  test {name} ... ok");
            passed += 1;
        } else {
            println!("  test {name} ... FAILED");
            for err in &test_errors {
                eprintln!("    {err}");
            }
            failed += 1;
        }
    }

    // Level 3: Execute test flows (compile and run) when --execute is passed
    if execute && failed == 0 {
        println!("\nnom: executing test flows...");

        // Find all non-test declarations (the system/flow definitions that tests reference)
        let non_tests: Vec<_> = parsed
            .declarations
            .iter()
            .filter(|d| d.classifier != Classifier::Test)
            .cloned()
            .collect();

        if non_tests.is_empty() {
            println!("  (no executable declarations found, skipping execution)");
        } else if let Some(ref r) = resolver {
            // Build a source file with only the non-test declarations
            let exec_source = nom_ast::SourceFile {
                path: parsed.path.clone(),
                locale: parsed.locale.clone(),
                declarations: non_tests,
            };

            let planner = Planner::new(r);
            match planner.plan(&exec_source) {
                Ok(mut plan) => {
                    planner.enrich_with_implementations(&mut plan);
                    let opts = CodegenOptions::default();
                    match generate(&plan, &opts) {
                        Ok(codegen_out) => {
                            // Write a test harness that imports and calls run_all
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

                            let build_dir = parent_dir.join(".nom-out").join(format!("{file_stem}_test"));
                            let src_dir = build_dir.join("src");

                            if let Err(e) = std::fs::create_dir_all(&src_dir) {
                                eprintln!("  exec: cannot create build dir: {e}");
                            } else {
                                // Generate test main that wraps run_all in a test
                                let rust_src = &codegen_out.rust_source;
                                let test_main = if rust_src.contains("fn main()") {
                                    rust_src.clone()
                                } else {
                                    let mut src = rust_src.clone();
                                    src.push_str("\nfn main() {\n    run_all();\n    println!(\"nom: execution ok\");\n}\n");
                                    src
                                };

                                if let Err(e) = std::fs::write(src_dir.join("main.rs"), &test_main) {
                                    eprintln!("  exec: write error: {e}");
                                } else {
                                    // Generate Cargo.toml
                                    let deps = collect_dependencies(&plan);
                                    let mut cargo_toml = format!(
                                        "[workspace]\n\n[package]\nname = \"{file_stem}_test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n"
                                    );
                                    for dep in &deps {
                                        cargo_toml.push_str(&format!("{} = {}\n", dep.name, dep.spec));
                                    }

                                    if let Err(e) = std::fs::write(build_dir.join("Cargo.toml"), &cargo_toml) {
                                        eprintln!("  exec: write Cargo.toml error: {e}");
                                    } else {
                                        // Build and run
                                        match process::Command::new("cargo")
                                            .arg("run")
                                            .current_dir(&build_dir)
                                            .status()
                                        {
                                            Ok(status) if status.success() => {
                                                println!("  exec: all flows executed successfully");
                                            }
                                            Ok(status) => {
                                                eprintln!("  exec: flow execution failed (exit {})", status.code().unwrap_or(-1));
                                                failed += 1;
                                            }
                                            Err(e) => {
                                                eprintln!("  exec: cargo failed: {e}");
                                                failed += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("  exec: codegen error: {e}");
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  exec: plan error: {e}");
                    failed += 1;
                }
            }
        }
    }

    println!("\nnom: {passed} passed, {failed} failed");
    if failed > 0 { 1 } else { 0 }
}

/// Evaluate a test assertion expression.
///
/// Supports:
/// - Comparison expressions: `security > 0.9`, `performance >= 0.5`
/// - Identifier checks: resolves the `given` subject and checks its existence
/// - Literal true/false
fn eval_assertion(
    expr: &nom_ast::Expr,
    resolver: Option<&Resolver>,
    test: &nom_ast::Declaration,
) -> Result<(), String> {
    use nom_ast::{BinOp, Expr, Literal, Statement};

    match expr {
        Expr::Literal(Literal::Bool(true)) => Ok(()),
        Expr::Literal(Literal::Bool(false)) => Err("assertion is literal false".into()),

        // Binary comparison: e.g. `security > 0.9`
        Expr::BinaryOp(left, op, right) => {
            // Find the test subject from `given` statements
            let subject = test.statements.iter().find_map(|s| {
                if let Statement::Given(g) = s { Some(&g.subject) } else { None }
            });

            // Try to resolve the subject to get its scores
            let entry = subject.and_then(|subj| {
                resolver.and_then(|r| {
                    let nom_ref = nom_ast::NomRef {
                        word: subj.clone(),
                        variant: None,
                        span: subj.span,
                    };
                    r.resolve(&nom_ref).ok()
                })
            });

            let lval = eval_numeric(left, entry.as_ref());
            let rval = eval_numeric(right, entry.as_ref());

            match (lval, rval) {
                (Some(l), Some(r)) => {
                    let pass = match op {
                        BinOp::Gt => l > r,
                        BinOp::Lt => l < r,
                        BinOp::Gte => l >= r,
                        BinOp::Lte => l <= r,
                        BinOp::Eq => (l - r).abs() < 1e-9,
                        BinOp::Neq => (l - r).abs() >= 1e-9,
                        _ => return Err(format!("unsupported operator in assertion: {op:?}")),
                    };
                    if pass { Ok(()) } else {
                        Err(format!("{l:.4} {op:?} {r:.4} is false"))
                    }
                }
                _ => {
                    // Can't evaluate numerically — treat as structural check (pass if well-formed)
                    Ok(())
                }
            }
        }

        // Identifier: check that the word resolves
        Expr::Ident(id) => {
            if let Some(r) = resolver {
                let nom_ref = nom_ast::NomRef {
                    word: id.clone(),
                    variant: None,
                    span: id.span,
                };
                match r.resolve(&nom_ref) {
                    Ok(_) => Ok(()),
                    Err(_) => Err(format!("'{}' could not be resolved", id.name)),
                }
            } else {
                Ok(()) // No resolver, skip resolution checks
            }
        }

        _ => Ok(()), // Other expression types: pass (not yet evaluable)
    }
}

/// Resolve a numeric value from an expression, optionally using a resolved word entry's scores.
fn eval_numeric(
    expr: &nom_ast::Expr,
    entry: Option<&nom_resolver::WordEntry>,
) -> Option<f64> {
    use nom_ast::{Expr, Literal};
    match expr {
        Expr::Literal(Literal::Number(n)) => Some(*n),
        Expr::Literal(Literal::Integer(i)) => Some(*i as f64),
        Expr::Ident(id) => {
            // Look up score fields from the resolved entry
            entry.and_then(|e| match id.name.as_str() {
                "security" => Some(e.security),
                "performance" => Some(e.performance),
                "reliability" => Some(e.reliability),
                "readability" => Some(e.readability),
                "testability" => Some(e.testability),
                "portability" => Some(e.portability),
                "composability" => Some(e.composability),
                "maturity" => Some(e.maturity),
                "overall_score" => Some(e.overall_score),
                _ => None,
            })
        }
        _ => None,
    }
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
                    let word_label = f.word.as_deref().unwrap_or("?");
                    let variant_label = f
                        .variant
                        .as_deref()
                        .map(|v| format!("::{v}"))
                        .unwrap_or_default();
                    println!(
                        "[{}] [{}] {}{}: {}",
                        f.severity, f.rule_id, word_label, variant_label, f.message
                    );
                    if let Some(rem) = &f.remediation {
                        println!("         -> {rem}");
                    }
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

fn cmd_dict(query: &str, dict: &PathBuf, limit: usize, contract: bool) -> i32 {
    let resolver = match open_resolver(dict) {
        Some(r) => r,
        None => return 1,
    };

    let result = if contract {
        resolver.search_by_contract(query, limit)
    } else {
        resolver.search_by_describe(query, limit)
    };

    match result {
        Ok(entries) => {
            if entries.is_empty() {
                if contract {
                    println!("No entries matching contract '{query}'");
                } else {
                    println!("No results for '{query}'");
                }
            } else {
                if contract {
                    println!(
                        "{:<20} {:<12} {:<20} {:<20} {}",
                        "WORD", "VARIANT", "INPUT", "OUTPUT", "DESCRIPTION"
                    );
                    println!("{}", "-".repeat(85));
                    for e in &entries {
                        println!(
                            "{:<20} {:<12} {:<20} {:<20} {}",
                            e.word,
                            e.variant.as_deref().unwrap_or("-"),
                            e.input_type.as_deref().unwrap_or("-"),
                            e.output_type.as_deref().unwrap_or("-"),
                            e.describe.as_deref().unwrap_or(""),
                        );
                    }
                } else {
                    println!(
                        "{:<20} {:<12} {:<8} {:<8} {}",
                        "WORD", "VARIANT", "SEC", "PERF", "DESCRIPTION"
                    );
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
    let novelos_conn =
        match rusqlite::Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY) {
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

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec
        .iter()
        .map(|b| b.as_ref() as &dyn rusqlite::types::ToSql)
        .collect();

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
        let (hash, name, kind, lang, conc, signature, body, _source_path) = match row_result {
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

        // Derive word and variant using smart name mapping.
        // Goal: .nom files write `need hash::argon2` so we need
        // word=hash, variant=argon2 — not word=raw_func_name, variant=concept.
        //
        // Strategy:
        // 1. Check if the function name contains a well-known domain word
        //    (hash, store, cache, http, auth, encrypt, parse, sort, etc.)
        //    → word = domain, variant = name (the specific implementation)
        // 2. If concept exists, use concept as word, name as variant
        // 3. Fallback: name as word, kind as variant
        let name_lower = name.to_lowercase();
        let (word, variant, describe) = if let Some(domain) = detect_domain(&name_lower) {
            let var = name.clone();
            let desc = format!("{domain} implementation: {name}");
            (domain.to_owned(), Some(var), desc)
        } else if let Some(ref c) = conc {
            let desc = format!("{c} implementation: {name}");
            (c.clone(), Some(name.clone()), desc)
        } else {
            let desc = format!("{kind_str}: {name}");
            (name.clone(), Some(kind_str.clone()), desc)
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
            overall_score: quality,
            hash,
            language: lang_str,
            body: Some(body),
            body_kind: Some("nom_source".to_owned()),
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

    let input_type = v.get("inputs").and_then(|v| {
        if v.is_array() {
            Some(v.to_string())
        } else {
            v.as_str().map(|s| s.to_owned())
        }
    });

    let output_type = v.get("outputs").and_then(|v| {
        if v.is_array() {
            Some(v.to_string())
        } else {
            v.as_str().map(|s| s.to_owned())
        }
    });

    let effects = v
        .get("effects")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    (input_type, output_type, effects)
}

/// Detect which semantic domain a function name belongs to.
/// Returns the dictionary word (e.g., "hash", "store", "http") if the name
/// contains a known pattern. This maps raw function names to semantic words
/// that .nom files use with `need hash::argon2`.
fn detect_domain(name_lower: &str) -> Option<&'static str> {
    // Crypto / hashing
    if name_lower.contains("argon")
        || name_lower.contains("bcrypt")
        || name_lower.contains("sha256")
        || name_lower.contains("sha512")
        || name_lower.contains("sha1")
        || name_lower.contains("blake")
        || name_lower.contains("pbkdf")
        || name_lower.contains("scrypt")
        || (name_lower.contains("hash") && !name_lower.contains("hashmap"))
    {
        return Some("hash");
    }
    if name_lower.contains("encrypt")
        || name_lower.contains("decrypt")
        || name_lower.contains("aes")
        || name_lower.contains("chacha")
        || name_lower.contains("cipher")
    {
        return Some("encrypt");
    }
    if name_lower.contains("sign")
        && (name_lower.contains("ed25519")
            || name_lower.contains("rsa")
            || name_lower.contains("ecdsa")
            || name_lower.contains("verify_sig")
            || name_lower.contains("digital"))
    {
        return Some("sign");
    }
    if name_lower.contains("tls")
        || name_lower.contains("ssl")
        || name_lower.contains("certificate")
        || name_lower.contains("handshake")
    {
        return Some("tls");
    }

    // Data / storage
    if name_lower.contains("redis") {
        return Some("store");
    }
    if name_lower.contains("postgres")
        || name_lower.contains("sqlite")
        || name_lower.contains("mysql")
        || name_lower.contains("database")
        || name_lower.contains("db_")
        || name_lower.contains("_db")
    {
        return Some("database");
    }
    if name_lower.contains("cache") || name_lower.contains("lru") || name_lower.contains("memoize")
    {
        return Some("cache");
    }
    if name_lower.contains("queue")
        || name_lower.contains("kafka")
        || name_lower.contains("rabbitmq")
        || name_lower.contains("pubsub")
    {
        return Some("queue");
    }

    // Network
    if name_lower.contains("http")
        || name_lower.contains("server")
        || name_lower.contains("router")
        || name_lower.contains("endpoint")
        || name_lower.contains("handler")
        || name_lower.contains("middleware")
    {
        return Some("http");
    }
    if name_lower.contains("websocket") || name_lower.contains("ws_") {
        return Some("websocket");
    }
    if name_lower.contains("grpc") || name_lower.contains("protobuf") {
        return Some("grpc");
    }
    if name_lower.contains("dns") || name_lower.contains("resolve_host") {
        return Some("dns");
    }
    if name_lower.contains("rate_limit")
        || name_lower.contains("throttle")
        || name_lower.contains("limiter")
        || name_lower.contains("token_bucket")
    {
        return Some("limiter");
    }

    // Auth
    if name_lower.contains("jwt")
        || name_lower.contains("token")
        || name_lower.contains("oauth")
        || name_lower.contains("auth")
        || name_lower.contains("login")
        || name_lower.contains("session")
    {
        return Some("auth");
    }

    // IO
    if name_lower.contains("print")
        || name_lower.contains("stdout")
        || name_lower.contains("display")
        || name_lower.contains("write_output")
    {
        return Some("print");
    }
    if name_lower.contains("log") || name_lower.contains("logger") || name_lower.contains("tracing")
    {
        return Some("log");
    }
    if name_lower.contains("read_file")
        || name_lower.contains("write_file")
        || name_lower.contains("open_file")
        || name_lower.contains("fs_")
    {
        return Some("file");
    }

    // Compute
    if name_lower.contains("serialize")
        || name_lower.contains("json")
        || name_lower.contains("serde")
        || name_lower.contains("marshal")
    {
        return Some("serialize");
    }
    if name_lower.contains("compress")
        || name_lower.contains("gzip")
        || name_lower.contains("zstd")
        || name_lower.contains("deflate")
    {
        return Some("compress");
    }
    if name_lower.contains("parse")
        || name_lower.contains("parser")
        || name_lower.contains("tokenize")
        || name_lower.contains("lexer")
    {
        return Some("parse");
    }
    if name_lower.contains("sort") && !name_lower.contains("resort") {
        return Some("sort");
    }
    if name_lower.contains("filter") && !name_lower.contains("_filtered") {
        return Some("filter");
    }

    // OS
    if name_lower.contains("process") || name_lower.contains("spawn") || name_lower.contains("exec")
    {
        return Some("process");
    }
    if name_lower.contains("thread")
        || name_lower.contains("mutex")
        || name_lower.contains("rwlock")
    {
        return Some("thread");
    }
    if name_lower.contains("socket")
        || name_lower.contains("tcp_")
        || name_lower.contains("udp_")
        || name_lower.contains("bind_addr")
    {
        return Some("socket");
    }

    // AI / ML
    if name_lower.contains("embed")
        || name_lower.contains("vector")
        || name_lower.contains("cosine")
        || name_lower.contains("similarity")
    {
        return Some("embed");
    }
    if name_lower.contains("llm")
        || name_lower.contains("generate_text")
        || name_lower.contains("completion")
        || name_lower.contains("inference")
    {
        return Some("generate");
    }
    if name_lower.contains("classify")
        || name_lower.contains("predict")
        || name_lower.contains("detect")
    {
        return Some("classify");
    }

    // Graph
    if name_lower.contains("traverse")
        || name_lower.contains("bfs")
        || name_lower.contains("dfs")
        || name_lower.contains("shortest_path")
        || name_lower.contains("dijkstra")
        || name_lower.contains("topological")
    {
        return Some("traverse");
    }

    // Agent
    if name_lower.contains("agent")
        || name_lower.contains("supervisor")
        || name_lower.contains("worker")
    {
        return Some("agent");
    }
    if name_lower.contains("schedule")
        || name_lower.contains("cron")
        || name_lower.contains("timer")
    {
        return Some("schedule");
    }

    None
}

// ── Precompile ───────────────────────────────────────────────────────────────

fn cmd_precompile(
    dict: &PathBuf,
    output_dir: &PathBuf,
    word: Option<&str>,
    language: Option<&str>,
    limit: usize,
    dry_run: bool,
) -> i32 {
    // Open nomdict read-only to query entries with bodies
    let conn = match rusqlite::Connection::open_with_flags(dict, OpenFlags::SQLITE_OPEN_READ_WRITE)
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nom: cannot open dict {}: {e}", dict.display());
            return 1;
        }
    };

    // Ensure artifact_path column exists
    let _ = conn.execute_batch("ALTER TABLE nomtu ADD COLUMN artifact_path TEXT;");
    // Ensure body_bytes column exists (idempotent — ignored if already present)
    let _ = conn.execute_batch("ALTER TABLE nomtu ADD COLUMN body_bytes BLOB;");

    // Backfill body_bytes for rows that were precompiled before this migration.
    // Idempotent: rows already populated (body_bytes IS NOT NULL) are skipped.
    {
        let backfill_ids: Vec<(i64, String)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT id, artifact_path FROM nomtu \
                     WHERE body_kind = 'bc' \
                       AND (body_bytes IS NULL OR length(body_bytes) = 0) \
                       AND artifact_path IS NOT NULL",
                )
                .unwrap_or_else(|e| panic!("backfill query: {e}"));
            stmt.query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        };
        let mut backfilled = 0usize;
        for (row_id, artifact_path) in &backfill_ids {
            match std::fs::read(artifact_path) {
                Ok(bytes) => {
                    let hash_hex = format!("{:x}", Sha256::digest(&bytes));
                    let bc_size = bytes.len() as i64;
                    match conn.execute(
                        "UPDATE nomtu SET body_bytes = ?1, bc_hash = ?2, bc_size = ?3 \
                         WHERE id = ?4",
                        rusqlite::params![bytes, hash_hex, bc_size, row_id],
                    ) {
                        Ok(_) => backfilled += 1,
                        Err(e) => eprintln!(
                            "  backfill warn: UPDATE failed for id={row_id}: {e}"
                        ),
                    }
                }
                Err(e) => {
                    eprintln!(
                        "  backfill warn: skipping id={row_id} ({}): {e}",
                        artifact_path
                    );
                    let _ = conn.execute(
                        "UPDATE nomtu SET artifact_path = NULL WHERE id = ?1",
                        rusqlite::params![row_id],
                    );
                }
            }
        }
        if backfilled > 0 {
            println!("nom: backfilled body_bytes for {backfilled} existing bc rows");
        }
    }

    // Build query
    let mut sql = String::from(
        "SELECT id, word, variant, language, body, signature \
         FROM nomtu WHERE body IS NOT NULL AND length(body) > 0",
    );
    let mut param_index = 1u32;

    let word_idx = if word.is_some() {
        let idx = param_index;
        sql.push_str(&format!(" AND word = ?{idx}"));
        param_index += 1;
        Some(idx)
    } else {
        None
    };

    let lang_idx = if language.is_some() {
        let idx = param_index;
        sql.push_str(&format!(" AND language = ?{idx}"));
        param_index += 1;
        Some(idx)
    } else {
        None
    };

    if limit > 0 {
        sql.push_str(&format!(" LIMIT ?{param_index}"));
    }

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: query error: {e}");
            return 1;
        }
    };

    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let (Some(_), Some(w)) = (word_idx, word) {
        params_vec.push(Box::new(w.to_owned()));
    }
    if let (Some(_), Some(l)) = (lang_idx, language) {
        params_vec.push(Box::new(l.to_owned()));
    }
    if limit > 0 {
        params_vec.push(Box::new(limit as i64));
    }

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec
        .iter()
        .map(|b| b.as_ref() as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = match stmt.query_map(params_refs.as_slice(), |row| {
        Ok((
            row.get::<_, i64>(0)?,            // id
            row.get::<_, String>(1)?,         // word
            row.get::<_, Option<String>>(2)?, // variant
            row.get::<_, String>(3)?,         // language
            row.get::<_, String>(4)?,         // body
            row.get::<_, Option<String>>(5)?, // signature
        ))
    }) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nom: query execution error: {e}");
            return 1;
        }
    };

    // Find LLVM tools
    let llvm_bin = find_llvm_bin();
    if llvm_bin.is_none() && !dry_run {
        eprintln!("nom: warning: LLVM tools not found via rustc --print sysroot");
        eprintln!("nom: stub .bc generation for non-Rust entries will be skipped");
    }

    // Create output directory
    if !dry_run {
        if let Err(e) = std::fs::create_dir_all(output_dir) {
            eprintln!(
                "nom: cannot create output dir {}: {e}",
                output_dir.display()
            );
            return 1;
        }
    }

    // Counters per language
    let mut total = 0usize;
    let mut compiled = 0usize;
    let mut stubbed = 0usize;
    let mut failed = 0usize;
    let mut lang_counts: std::collections::HashMap<String, (usize, usize, usize)> =
        std::collections::HashMap::new();

    // Collect rows (can't hold stmt borrow and also use conn for UPDATE)
    let entries: Vec<_> = rows.filter_map(|r| r.ok()).collect();

    println!(
        "nom: precompiling {} nomtu entries to .bc...",
        entries.len()
    );

    for (id, entry_word, variant, lang, body, _signature) in &entries {
        total += 1;

        let safe_name = llvm_safe_name(entry_word, variant.as_deref());
        let entry_dir = output_dir.join(entry_word);
        let bc_filename = if let Some(v) = variant {
            format!("{v}.bc")
        } else {
            format!("{entry_word}.bc")
        };

        if dry_run {
            let status = match lang.as_str() {
                "rust" => "compile",
                "c" | "cpp" => "compile (via llvm-as)",
                _ => "stub",
            };
            if total <= 20 || total % 100 == 0 {
                println!(
                    "  [{total}] {entry_word}::{} ({lang}) -> {status}",
                    variant.as_deref().unwrap_or("-"),
                );
            }
            let counts = lang_counts.entry(lang.clone()).or_insert((0, 0, 0));
            match lang.as_str() {
                "rust" | "c" | "cpp" => counts.0 += 1,
                _ => counts.1 += 1,
            }
            continue;
        }

        if let Err(e) = std::fs::create_dir_all(&entry_dir) {
            eprintln!("nom: cannot create dir {}: {e}", entry_dir.display());
            failed += 1;
            continue;
        }

        let bc_path = entry_dir.join(&bc_filename);
        let result = match lang.as_str() {
            "rust" => {
                // Try native Rust compilation first, fall back to stub on failure
                match precompile_rust(&safe_name, body, &bc_path) {
                    ok @ Ok(_) => ok,
                    Err(_) => {
                        precompile_stub(&safe_name, body, lang, &bc_path, llvm_bin.as_deref())
                    }
                }
            }
            "c" | "cpp" => precompile_c(&safe_name, body, lang, &bc_path, llvm_bin.as_deref()),
            _ => precompile_stub(&safe_name, body, lang, &bc_path, llvm_bin.as_deref()),
        };

        let counts = lang_counts.entry(lang.clone()).or_insert((0, 0, 0));
        match result {
            Ok(PrecompileResult::Compiled) => {
                compiled += 1;
                counts.0 += 1;
                // §4.4.6: a successful precompile means this entry has a
                // canonical `.bc` artifact. Inline the bytes into body_bytes
                // so the row is self-contained; bc_path remains as a cache
                // key only.
                match std::fs::read(&bc_path) {
                    Ok(bytes) => {
                        let hash_hex = format!("{:x}", Sha256::digest(&bytes));
                        let bc_size = bytes.len() as i64;
                        let _ = conn.execute(
                            "UPDATE nomtu SET artifact_path = ?1, body_kind = ?2, \
                             body_bytes = ?3, bc_hash = ?4, bc_size = ?5 WHERE id = ?6",
                            rusqlite::params![
                                bc_path.to_string_lossy().as_ref(),
                                nom_types::body_kind::BC,
                                bytes,
                                hash_hex,
                                bc_size,
                                id,
                            ],
                        );
                    }
                    Err(e) => {
                        // File write succeeded but read-back failed.
                        // Do NOT promote body_kind to 'bc' — that would
                        // violate invariant 15 (body_kind='bc' ⇒ body_bytes
                        // IS NOT NULL). Leave the row unchanged; the next
                        // precompile run will re-attempt.
                        eprintln!(
                            "  warn: could not read {}: {e}",
                            bc_path.display()
                        );
                    }
                }
            }
            Ok(PrecompileResult::Stubbed) => {
                stubbed += 1;
                counts.1 += 1;
                // Stubbed entries have an empty-shell .bc — not a real
                // compiled artifact. artifact_path is still set so
                // downstream tooling can find the placeholder, but
                // body_kind stays NULL to mark this as incomplete.
                let _ = conn.execute(
                    "UPDATE nomtu SET artifact_path = ?1 WHERE id = ?2",
                    rusqlite::params![bc_path.to_string_lossy().as_ref(), id],
                );
            }
            Err(e) => {
                failed += 1;
                counts.2 += 1;
                if failed <= 10 {
                    eprintln!(
                        "  error: {entry_word}::{} ({lang}): {e}",
                        variant.as_deref().unwrap_or("-"),
                    );
                }
            }
        }

        if total % 100 == 0 {
            println!(
                "  processed {total} entries ({compiled} compiled, {stubbed} stubbed, {failed} failed)..."
            );
        }
    }

    // Summary
    println!();
    println!("nom: precompile complete");
    println!("  total:    {total}");
    if dry_run {
        println!("  (dry run — no files written)");
    } else {
        println!("  compiled: {compiled}");
        println!("  stubbed:  {stubbed}");
        println!("  failed:   {failed}");
        println!("  output:   {}", output_dir.display());
    }

    // Per-language breakdown
    let mut langs: Vec<_> = lang_counts.iter().collect();
    langs.sort_by_key(|(k, _)| (*k).clone());
    for (lang, (comp, stub, fail)) in &langs {
        if dry_run {
            println!("    {lang}: {comp} compile, {stub} stub");
        } else {
            println!("    {lang}: {comp} compiled, {stub} stubbed, {fail} failed");
        }
    }

    0
}

enum PrecompileResult {
    Compiled,
    Stubbed,
}

/// Compile a Rust body to LLVM bitcode via rustc.
fn precompile_rust(
    safe_name: &str,
    body: &str,
    bc_path: &Path,
) -> Result<PrecompileResult, String> {
    // Determine if the body is already a complete item (fn, struct, impl, etc.)
    // or just a function body that needs wrapping.
    let trimmed = body.trim();
    let is_complete_item = trimmed.starts_with("pub fn ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("pub struct ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("pub enum ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("pub trait ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("pub async fn ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with("pub const ")
        || trimmed.starts_with("const ")
        || trimmed.starts_with("pub type ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("pub static ");

    let wrapper = if is_complete_item {
        // Body is already a complete Rust item — include as-is
        // Add allow attributes to suppress warnings from extracted code
        format!(
            "#![allow(dead_code, unused_variables, unused_imports, non_snake_case, unused_mut)]\n\n\
             {body}\n"
        )
    } else {
        // Body is a function body or expression — wrap it
        format!(
            "#![allow(dead_code, unused_variables, unused_imports, non_snake_case, unused_mut)]\n\n\
             #[no_mangle]\npub extern \"C\" fn {safe_name}() {{\n{body}\n}}\n"
        )
    };

    let tmp_dir = std::env::temp_dir().join("nom-precompile");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("mkdir: {e}"))?;

    let rs_path = tmp_dir.join(format!("{safe_name}.rs"));
    std::fs::write(&rs_path, &wrapper).map_err(|e| format!("write .rs: {e}"))?;

    // Try compiling — if it fails, fall back to stub
    let output = process::Command::new("rustc")
        .args([
            "--emit=llvm-bc",
            "--crate-type=cdylib",
            "--edition=2021",
            "-o",
        ])
        .arg(bc_path)
        .arg(&rs_path)
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::piped())
        .output()
        .map_err(|e| format!("rustc: {e}"))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&rs_path);

    if output.status.success() {
        Ok(PrecompileResult::Compiled)
    } else {
        // Compilation failed — body has external deps
        // Fall back to LLVM IR stub (same as other languages)
        Err(format!("rustc failed (has external deps)"))
    }
}

/// Compile a C/C++ body to LLVM bitcode via llvm-as (generate LLVM IR text).
fn precompile_c(
    safe_name: &str,
    body: &str,
    lang: &str,
    bc_path: &Path,
    llvm_bin: Option<&Path>,
) -> Result<PrecompileResult, String> {
    let llvm_bin = llvm_bin.ok_or("llvm-as not found")?;
    let llvm_as = llvm_bin.join("llvm-as");
    if !llvm_as.exists() && !llvm_as.with_extension("exe").exists() {
        return Err("llvm-as not found".to_owned());
    }

    // Generate minimal LLVM IR wrapping the C function as an opaque stub
    // with a comment containing the original body for future translation.
    let escaped_body: String = body
        .lines()
        .map(|line| format!("; C: {line}"))
        .collect::<Vec<_>>()
        .join("\n");

    let ir = format!(
        concat!(
            "; ModuleID = '{name}'\n",
            "source_filename = \"{name}.{lang}\"\n",
            "target datalayout = \"e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n",
            "target triple = \"x86_64-pc-windows-msvc\"\n",
            "\n",
            "{body_comment}\n",
            "\n",
            "define void @{name}() {{\n",
            "entry:\n",
            "  ret void\n",
            "}}\n",
        ),
        name = safe_name,
        lang = lang,
        body_comment = escaped_body,
    );

    let tmp_dir = std::env::temp_dir().join("nom-precompile");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("mkdir: {e}"))?;

    let ll_path = tmp_dir.join(format!("{safe_name}.ll"));
    std::fs::write(&ll_path, &ir).map_err(|e| format!("write .ll: {e}"))?;

    let status = process::Command::new(&llvm_as)
        .arg("-o")
        .arg(bc_path)
        .arg(&ll_path)
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::piped())
        .status()
        .map_err(|e| format!("llvm-as: {e}"))?;

    let _ = std::fs::remove_file(&ll_path);

    if status.success() {
        Ok(PrecompileResult::Compiled)
    } else {
        Err(format!("llvm-as exit code {}", status.code().unwrap_or(-1)))
    }
}

/// Generate a stub .bc for languages that cannot compile directly (Python, JS, TS, Go, etc.).
fn precompile_stub(
    safe_name: &str,
    body: &str,
    lang: &str,
    bc_path: &Path,
    llvm_bin: Option<&Path>,
) -> Result<PrecompileResult, String> {
    let llvm_bin = llvm_bin.ok_or("llvm-as not found — cannot generate stub .bc")?;
    let llvm_as = llvm_bin.join("llvm-as");
    if !llvm_as.exists() && !llvm_as.with_extension("exe").exists() {
        return Err("llvm-as not found".to_owned());
    }

    // Embed a comment with the first 20 lines of the original body
    let body_preview: String = body
        .lines()
        .take(20)
        .map(|line| format!("; {lang}: {line}"))
        .collect::<Vec<_>>()
        .join("\n");

    let ir = format!(
        concat!(
            "; ModuleID = '{name}'\n",
            "source_filename = \"{name}.{lang}\"\n",
            "target datalayout = \"e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n",
            "target triple = \"x86_64-pc-windows-msvc\"\n",
            "\n",
            "; STUB: needs translation from {lang} to native code\n",
            "; Original body available in nomdict\n",
            "{body_preview}\n",
            "\n",
            "define void @{name}() {{\n",
            "entry:\n",
            "  ret void\n",
            "}}\n",
        ),
        name = safe_name,
        lang = lang,
        body_preview = body_preview,
    );

    let tmp_dir = std::env::temp_dir().join("nom-precompile");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("mkdir: {e}"))?;

    let ll_path = tmp_dir.join(format!("{safe_name}.ll"));
    std::fs::write(&ll_path, &ir).map_err(|e| format!("write .ll: {e}"))?;

    let status = process::Command::new(&llvm_as)
        .arg("-o")
        .arg(bc_path)
        .arg(&ll_path)
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::piped())
        .status()
        .map_err(|e| format!("llvm-as: {e}"))?;

    let _ = std::fs::remove_file(&ll_path);

    if status.success() {
        Ok(PrecompileResult::Stubbed)
    } else {
        Err(format!("llvm-as exit code {}", status.code().unwrap_or(-1)))
    }
}

/// Locate the LLVM tools directory bundled with rustc.
fn find_llvm_bin() -> Option<PathBuf> {
    let output = process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .ok()?;
    let sysroot = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    let bin = PathBuf::from(&sysroot).join("lib/rustlib/x86_64-pc-windows-msvc/bin");
    if bin.exists() { Some(bin) } else { None }
}

/// Sanitize a word+variant into an LLVM-safe symbol name.
fn llvm_safe_name(word: &str, variant: Option<&str>) -> String {
    let sanitize = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };
    let base = sanitize(word);
    if let Some(v) = variant {
        let v_safe = sanitize(v);
        format!("nom_{base}_{v_safe}")
    } else {
        format!("nom_{base}")
    }
}

// ── Extract / Score / Stats / Coverage ────────────────────────────────────────

fn cmd_extract(dir: &PathBuf, dict: &PathBuf, limit: usize) -> i32 {
    if !dir.is_dir() {
        eprintln!("nom: {} is not a directory", dir.display());
        return 1;
    }

    // Open NomDict — use the dict path's parent as root (NomDict adds data/nomdict.db)
    // If dict is "nomdict.db" we use the current dir's parent as root.
    let nomdict = match open_nomdict(dict) {
        Some(d) => d,
        None => return 1,
    };

    let paths = nom_extract::scan::scan_directory(dir);
    let total_files = if limit > 0 {
        paths.len().min(limit)
    } else {
        paths.len()
    };

    println!("nom: scanning {} for source files...", dir.display());
    println!("nom: found {} parseable files", paths.len());
    if limit > 0 && paths.len() > limit {
        println!("nom: limiting to {limit} files");
    }

    let mut files_parsed = 0usize;
    let mut total_atoms = 0usize;
    let mut total_new = 0usize;

    let paths_to_process: Vec<_> = if limit > 0 {
        paths.into_iter().take(limit).collect()
    } else {
        paths
    };

    for path in &paths_to_process {
        let path_str = path.display().to_string();
        let language = match nom_extract::detect_language(&path_str) {
            Some(lang) if nom_extract::parseable_languages().contains(&lang) => lang,
            _ => continue,
        };

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Skip very large files
        if source.len() > 2 * 1024 * 1024 {
            continue;
        }

        // Run parser in a thread with bounded stack to catch stack overflows.
        // Use catch_unwind inside the thread so panics don't kill the process.
        let src_clone = source.clone();
        let path_clone = path_str.clone();
        let lang_clone = language.to_string();
        let parse_result = std::thread::Builder::new()
            .name("parser".to_string())
            .stack_size(32 * 1024 * 1024) // 32MB stack limit
            .spawn(move || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    nom_extract::parse_file(&src_clone, &path_clone, &lang_clone)
                }))
            })
            .and_then(|h| {
                h.join()
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "thread panic"))
            });
        let atoms = match parse_result {
            Ok(Ok(Ok(a))) => a,
            _ => {
                eprintln!("nom: skipping {} (parser crash)", path.display());
                continue;
            }
        };

        if atoms.is_empty() {
            files_parsed += 1;
            continue;
        }

        match nomdict.store_atoms(&atoms) {
            Ok(result) => {
                total_atoms += atoms.len();
                total_new += result.stored;
            }
            Err(e) => {
                eprintln!("nom: store error for {}: {e}", path.display());
            }
        }

        files_parsed += 1;

        if files_parsed % 100 == 0 {
            println!("nom: processed {files_parsed}/{total_files} files ({total_atoms} atoms)...");
        }
    }

    let dict_total = nomdict.count().unwrap_or(0);

    println!();
    println!("nom: extraction complete");
    println!("  files parsed:   {files_parsed}");
    println!("  atoms extracted: {total_atoms}");
    println!("  new in dict:    {total_new}");
    println!("  total in dict:  {dict_total}");

    0
}

fn cmd_score(dict: &PathBuf) -> i32 {
    let nomdict = match open_nomdict(dict) {
        Some(d) => d,
        None => return 1,
    };

    let atoms = match nomdict.load_all() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("nom: failed to load atoms: {e}");
            return 1;
        }
    };

    if atoms.is_empty() {
        println!("nom: dictionary is empty, nothing to score");
        return 0;
    }

    println!("nom: scoring {} atoms...", atoms.len());

    let mut total_security = 0.0_f64;
    let mut total_performance = 0.0_f64;
    let mut total_quality = 0.0_f64;
    let mut total_reliability = 0.0_f64;
    let mut scored = 0usize;

    // Open the raw sqlite connection to update scores
    let db_path = nomdict.db_path();
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nom: cannot open db for update: {e}");
            return 1;
        }
    };

    for atom in &atoms {
        let scores = nom_score::score_atom(atom);
        total_security += scores.security as f64;
        total_performance += scores.performance as f64;
        total_quality += scores.readability as f64;
        total_reliability += scores.reliability as f64;

        // Update scores in the database using the atom's hash/id
        let _ = conn.execute(
            "UPDATE nomtu SET security = ?1, performance = ?2, quality = ?3, reliability = ?4 \
             WHERE atom_id = ?5 OR word = ?6",
            rusqlite::params![
                scores.security as f64,
                scores.performance as f64,
                scores.readability as f64,
                scores.reliability as f64,
                atom.id,
                atom.name,
            ],
        );

        scored += 1;

        if scored % 1000 == 0 {
            println!("nom: scored {scored}/{}...", atoms.len());
        }
    }

    let n = scored as f64;
    println!();
    println!("nom: scoring complete");
    println!("  total scored:     {scored}");
    println!("  avg security:     {:.3}", total_security / n);
    println!("  avg performance:  {:.3}", total_performance / n);
    println!("  avg quality:      {:.3}", total_quality / n);
    println!("  avg reliability:  {:.3}", total_reliability / n);

    0
}

fn cmd_stats(dict: &PathBuf) -> i32 {
    let nomdict = match open_nomdict(dict) {
        Some(d) => d,
        None => return 1,
    };

    let total = match nomdict.count() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("nom: count error: {e}");
            return 1;
        }
    };

    println!("nom: dictionary statistics");
    println!("{}", "=".repeat(50));
    println!("  total entries:  {total}");

    // Count entries with bodies
    let atoms = match nomdict.load_all() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("nom: load error: {e}");
            return 1;
        }
    };
    let with_bodies = atoms.iter().filter(|a| a.body.is_some()).count();
    let with_concepts = atoms.iter().filter(|a| a.concept.is_some()).count();
    println!("  with bodies:    {with_bodies}");
    println!("  with concepts:  {with_concepts}");

    // By kind
    match nomdict.stats_by_kind() {
        Ok(stats) => {
            println!();
            println!("  by kind:");
            for (kind, count) in &stats {
                println!("    {kind:<20} {count}");
            }
        }
        Err(e) => eprintln!("nom: kind stats error: {e}"),
    }

    // By language
    match nomdict.stats_by_language() {
        Ok(stats) => {
            println!();
            println!("  by language:");
            for (lang, count) in &stats {
                println!("    {lang:<20} {count}");
            }
        }
        Err(e) => eprintln!("nom: language stats error: {e}"),
    }

    // Top concepts
    match nomdict.dictionary_summary() {
        Ok(summary) => {
            println!();
            println!("  top concepts:");
            for (concept, count) in summary.iter().take(15) {
                println!("    {concept:<20} {count}");
            }
        }
        Err(e) => eprintln!("nom: summary error: {e}"),
    }

    // Score distribution (if any atom has non-zero scores)
    let scored_atoms: Vec<_> = atoms.iter().collect();
    if !scored_atoms.is_empty() {
        // Get score stats from the database
        let db_path = nomdict.db_path();
        if let Ok(conn) =
            rusqlite::Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            if let Ok(mut stmt) = conn.prepare(
                "SELECT \
                   AVG(security), AVG(performance), AVG(quality), AVG(reliability), \
                   COUNT(CASE WHEN security > 0.0 THEN 1 END) \
                 FROM nomtu",
            ) {
                if let Ok(row) = stmt.query_row([], |row| {
                    Ok((
                        row.get::<_, f64>(0).unwrap_or(0.0),
                        row.get::<_, f64>(1).unwrap_or(0.0),
                        row.get::<_, f64>(2).unwrap_or(0.0),
                        row.get::<_, f64>(3).unwrap_or(0.0),
                        row.get::<_, i64>(4).unwrap_or(0),
                    ))
                }) {
                    if row.4 > 0 {
                        println!();
                        println!("  score averages ({} scored):", row.4);
                        println!("    security:     {:.3}", row.0);
                        println!("    performance:  {:.3}", row.1);
                        println!("    quality:      {:.3}", row.2);
                        println!("    reliability:  {:.3}", row.3);
                    }
                }
            }
        }
    }

    println!("{}", "=".repeat(50));

    0
}

fn cmd_coverage(dir: &PathBuf, dict: &PathBuf) -> i32 {
    if !dir.is_dir() {
        eprintln!("nom: {} is not a directory", dir.display());
        return 1;
    }

    let nomdict = match open_nomdict(dict) {
        Some(d) => d,
        None => return 1,
    };

    let paths = nom_extract::scan::scan_directory(dir);

    let mut total_files = 0usize;
    let mut total_functions = 0usize;
    let mut extracted = 0usize;
    let mut missing = 0usize;

    // Load all atoms from dict for comparison
    let dict_atoms = match nomdict.load_all() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("nom: failed to load dict: {e}");
            return 1;
        }
    };

    // Build a set of (name, source_path) for fast lookup
    let dict_keys: std::collections::HashSet<(String, String)> = dict_atoms
        .iter()
        .map(|a| (a.name.clone(), a.source_path.clone()))
        .collect();

    for path in &paths {
        let path_str = path.display().to_string();
        let language = match nom_extract::detect_language(&path_str) {
            Some(lang) if nom_extract::parseable_languages().contains(&lang) => lang,
            _ => continue,
        };

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if source.len() > 2 * 1024 * 1024 {
            continue;
        }

        let atoms = match nom_extract::parse_file(&source, &path_str, language) {
            Ok(a) => a,
            Err(_) => continue,
        };

        total_files += 1;

        for atom in &atoms {
            total_functions += 1;
            if dict_keys.contains(&(atom.name.clone(), atom.source_path.clone())) {
                extracted += 1;
            } else {
                missing += 1;
            }
        }
    }

    let coverage_pct = if total_functions > 0 {
        (extracted as f64 / total_functions as f64) * 100.0
    } else {
        0.0
    };

    println!("nom: coverage report for {}", dir.display());
    println!("{}", "=".repeat(50));
    println!("  total files:      {total_files}");
    println!("  total functions:  {total_functions}");
    println!("  in dictionary:    {extracted}");
    println!("  missing:          {missing}");
    println!("  coverage:         {coverage_pct:.1}%");
    println!("{}", "=".repeat(50));

    0
}

/// Open a NomDict from the --dict path.
/// The NomDict::open expects a root directory and creates data/nomdict.db inside it.
/// If the --dict points directly to a .db file, we use its parent as the root.
fn open_nomdict(dict: &PathBuf) -> Option<NomDict> {
    // If dict path ends with nomdict.db, use the grandparent dir as root
    // because NomDict stores at <root>/data/nomdict.db
    let root = if dict.extension().is_some_and(|e| e == "db") {
        // dict points to a .db file, e.g. "nomdict.db" or "data/nomdict.db"
        // NomDict::open creates data/nomdict.db so root should be parent of parent
        // But the existing convention is --dict nomdict.db at the project root,
        // so we just use current directory as root.
        PathBuf::from(".")
    } else {
        dict.clone()
    };

    match NomDict::open(&root) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("nom: cannot open nomdict: {e}");
            None
        }
    }
}

// ── Translate command ─────────────────────────────────────────────────────────

fn cmd_translate(
    dict: &PathBuf,
    language: Option<&str>,
    limit: usize,
    min_confidence: f64,
    dry_run: bool,
) -> i32 {
    // Open the nomdict database directly via rusqlite
    let conn = match rusqlite::Connection::open_with_flags(
        dict,
        if dry_run {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        },
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nom: cannot open dict {}: {e}", dict.display());
            return 1;
        }
    };

    // Query non-Rust entries with bodies
    let mut sql = String::from(
        "SELECT id, word, variant, language, body, concept, kind \
         FROM nomtu WHERE language != 'rust' AND body IS NOT NULL",
    );
    let mut param_idx = 1u32;
    let lang_idx = if language.is_some() {
        param_idx += 1;
        sql.push_str(&format!(" AND language = ?{}", param_idx - 1));
        Some(param_idx - 1)
    } else {
        None
    };

    sql.push_str(" ORDER BY id");

    if limit > 0 {
        param_idx += 1;
        sql.push_str(&format!(" LIMIT ?{}", param_idx - 1));
    }

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: query error: {e}");
            return 1;
        }
    };

    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let (Some(_), Some(lang)) = (lang_idx, language) {
        params_vec.push(Box::new(lang.to_owned()));
    }
    if limit > 0 {
        params_vec.push(Box::new(limit as i64));
    }

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec
        .iter()
        .map(|b| b.as_ref() as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = match stmt.query_map(params_refs.as_slice(), |row| {
        Ok((
            row.get::<_, i64>(0)?,            // id
            row.get::<_, String>(1)?,         // word
            row.get::<_, Option<String>>(2)?, // variant
            row.get::<_, String>(3)?,         // language
            row.get::<_, String>(4)?,         // body
            row.get::<_, Option<String>>(5)?, // concept
            row.get::<_, Option<String>>(6)?, // kind
        ))
    }) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nom: query error: {e}");
            return 1;
        }
    };

    let entries: Vec<_> = rows.filter_map(|r| r.ok()).collect();
    let total = entries.len();
    println!("nom: found {total} non-Rust entries to translate");

    if total == 0 {
        return 0;
    }

    let mut translated_count = 0usize;
    let mut skipped_low_conf = 0usize;
    let mut conf_buckets = [0usize; 10]; // 0.0-0.1, 0.1-0.2, ..., 0.9-1.0

    // Prepare insert statement for translated entries
    let mut insert_stmt = if !dry_run {
        match conn.prepare(
            "INSERT OR IGNORE INTO nomtu \
             (word, variant, language, body, concept, kind, describe) \
             VALUES (?1, ?2, 'rust', ?3, ?4, ?5, ?6)",
        ) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("nom: prepare insert error: {e}");
                return 1;
            }
        }
    } else {
        None
    };

    for (i, (_id, word, variant, lang, body, concept, kind)) in
        entries.iter().enumerate()
    {
        let result = nom_translate::translate(body, lang);

        // Track confidence distribution
        let bucket = (result.confidence * 10.0).min(9.0) as usize;
        conf_buckets[bucket] += 1;

        if result.confidence < min_confidence {
            skipped_low_conf += 1;
            continue;
        }

        if dry_run {
            if i < 5 || (i < 50 && i % 10 == 0) {
                println!(
                    "  [{}/{}] {} ({lang}) → confidence {:.2}, {} warnings, {} untranslated lines",
                    i + 1,
                    total,
                    word,
                    result.confidence,
                    result.warnings.len(),
                    result.untranslated_lines,
                );
                if i < 3 {
                    // Show first few translations
                    for line in result.rust_body.lines().take(5) {
                        println!("    | {line}");
                    }
                    if result.rust_body.lines().count() > 5 {
                        println!("    | ...");
                    }
                }
            }
        } else if let Some(ref mut stmt) = insert_stmt {
            let describe = format!("translated from {lang}: {word}");
            match stmt.execute(rusqlite::params![
                word,
                variant,
                result.rust_body,
                concept,
                kind,
                describe,
            ]) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("  warn: failed to insert translation for {word}: {e}");
                }
            }
        }

        translated_count += 1;

        if (i + 1) % 10000 == 0 {
            eprintln!("  progress: {}/{total}...", i + 1);
        }
    }

    // Summary
    println!("\n── Translation Summary ──");
    println!("  Total entries:    {total}");
    println!("  Translated:       {translated_count}");
    println!("  Skipped (low):    {skipped_low_conf}");
    println!("  Min confidence:   {min_confidence}");
    if dry_run {
        println!("  Mode:             DRY RUN (no writes)");
    }

    println!("\n  Confidence distribution:");
    for (i, count) in conf_buckets.iter().enumerate() {
        let lo = i as f64 * 0.1;
        let hi = lo + 0.1;
        let bar_len = (*count as f64 / total as f64 * 40.0) as usize;
        let bar: String = "#".repeat(bar_len);
        println!("    {lo:.1}-{hi:.1}: {bar} ({count})");
    }

    0
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

// ── Graph command ────────────────────────────────────────────────────────────

fn cmd_graph(dict: &PathBuf, limit: usize) -> i32 {
    let resolver = match open_resolver(dict) {
        Some(r) => r,
        None => return 1,
    };

    // Load all entries via describe search with wildcard
    let entries = match resolver.search_by_describe("%", if limit == 0 { 100_000 } else { limit }) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("nom: failed to load entries: {e}");
            return 1;
        }
    };

    if entries.is_empty() {
        println!("No .nomtu entries found in dictionary.");
        return 0;
    }

    println!("Building knowledge graph from {} entries...", entries.len());

    let mut graph = NomtuGraph::from_entries(&entries);

    println!("Detecting call edges...");
    graph.build_call_edges();
    let call_count = graph
        .edges()
        .iter()
        .filter(|e| e.edge_type == nom_graph::EdgeType::Calls)
        .count();
    println!("  {} call edges", call_count);

    println!("Detecting import edges...");
    graph.build_import_edges();
    let import_count = graph
        .edges()
        .iter()
        .filter(|e| e.edge_type == nom_graph::EdgeType::Imports)
        .count();
    println!("  {} import edges", import_count);

    println!("\nDetecting communities...");
    let communities = graph.detect_communities();
    println!("  {} communities found", communities.len());
    for c in communities.iter().take(20) {
        println!(
            "  [{:>6}] {} ({} members, cohesion: {:.2})",
            c.id,
            c.label,
            c.members.len(),
            c.cohesion,
        );
        for m in c.members.iter().take(5) {
            println!("           - {m}");
        }
        if c.members.len() > 5 {
            println!("           ... and {} more", c.members.len() - 5);
        }
    }

    println!("\nEntry points:");
    let eps = graph.entry_points();
    for ep in eps.iter().take(10) {
        println!("  {} ({})", ep.word, ep.kind);
    }

    println!(
        "\nGraph: {} nodes, {} edges, {} communities",
        graph.nodes().len(),
        graph.edges().len(),
        communities.len(),
    );

    0
}

// ── Search command ───────────────────────────────────────────────────────────

fn cmd_search(query: &str, dict: &PathBuf, limit: usize) -> i32 {
    let resolver = match open_resolver(dict) {
        Some(r) => r,
        None => return 1,
    };

    // Load all entries for BM25 indexing
    let entries = match resolver.search_by_describe("%", 100_000) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("nom: failed to load entries: {e}");
            return 1;
        }
    };

    if entries.is_empty() {
        println!("No .nomtu entries found in dictionary.");
        return 0;
    }

    // Build BM25 index
    let mut bm25 = BM25Index::new();
    for entry in &entries {
        let doc_id = match &entry.variant {
            Some(v) => format!("{}::{}", entry.word, v),
            None => entry.word.clone(),
        };
        // Combine searchable fields
        let text = format!(
            "{} {} {} {}",
            entry.word,
            entry.variant.as_deref().unwrap_or(""),
            entry.describe.as_deref().unwrap_or(""),
            entry.kind.as_str(),
        );
        bm25.add_document(&doc_id, &text);
    }

    // BM25 search
    let bm25_results = bm25.search(query, limit);

    // Also get LIKE-based results from resolver for RRF fusion
    let like_results = resolver
        .search_by_describe(query, limit)
        .unwrap_or_default();
    let like_ranked: Vec<(String, f64)> = like_results
        .iter()
        .enumerate()
        .map(|(rank, e)| {
            let doc_id = match &e.variant {
                Some(v) => format!("{}::{}", e.word, v),
                None => e.word.clone(),
            };
            (doc_id, (limit - rank) as f64)
        })
        .collect();

    let bm25_ranked: Vec<(String, f64)> = bm25_results
        .iter()
        .map(|r| (r.doc_id.clone(), r.score))
        .collect();

    // Fuse with RRF
    let fused = nom_search::reciprocal_rank_fusion(&[bm25_ranked, like_ranked], 60.0, limit);

    if fused.is_empty() {
        println!("No results for '{query}'");
        return 0;
    }

    println!(
        "{:<30} {:<10} {:<10} {}",
        "WORD", "SCORE", "SOURCES", "DESCRIPTION"
    );
    println!("{}", "-".repeat(80));

    // Map doc_ids back to entries for display
    let entry_map: HashMap<String, &NomtuEntry> = entries
        .iter()
        .map(|e| {
            let doc_id = match &e.variant {
                Some(v) => format!("{}::{}", e.word, v),
                None => e.word.clone(),
            };
            (doc_id, e)
        })
        .collect();

    for result in &fused {
        let desc = entry_map
            .get(&result.doc_id)
            .and_then(|e| e.describe.as_deref())
            .unwrap_or("");
        let sources = result.sources.len();
        println!(
            "{:<30} {:<10.4} {:<10} {}",
            result.doc_id, result.score, sources, desc,
        );
    }

    0
}

// ── Audit command ───────────────────────────────────────────────────────────

fn cmd_audit(dict: &PathBuf, min_severity: &str, limit: usize, format: &str) -> i32 {
    let min_sev = match Severity::from_str_loose(min_severity) {
        Some(s) => s,
        None => {
            eprintln!(
                "nom: unknown severity level '{min_severity}'. Use: info, low, medium, high, critical"
            );
            return 1;
        }
    };

    // Open the database directly and query all entries with bodies
    let db_path = dict.to_str().unwrap_or("nomdict.db");
    let conn = match rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nom: cannot open dict {}: {e}", dict.display());
            return 1;
        }
    };

    let limit_clause = if limit > 0 {
        format!(" LIMIT {limit}")
    } else {
        String::new()
    };

    let sql = format!(
        "SELECT word, variant, language, body FROM nomtu \
         WHERE body IS NOT NULL AND length(body) > 0{limit_clause}"
    );

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: query error: {e}");
            return 1;
        }
    };

    struct AuditRow {
        word: String,
        variant: Option<String>,
        language: String,
        body: String,
    }

    let rows: Vec<AuditRow> = match stmt.query_map([], |row| {
        Ok(AuditRow {
            word: row.get(0)?,
            variant: row.get(1)?,
            language: row
                .get::<_, String>(2)
                .unwrap_or_else(|_| "unknown".to_owned()),
            body: row.get(3)?,
        })
    }) {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(e) => {
            eprintln!("nom: failed to query entries: {e}");
            return 1;
        }
    };

    if rows.is_empty() {
        println!("nom: no .nomtu entries with bodies found in dictionary");
        return 0;
    }

    println!("nom: auditing {} entries...", rows.len());

    let mut total_findings = 0usize;
    let mut entries_with_findings = 0usize;
    let mut all_findings: Vec<(String, nom_security::SecurityFinding)> = Vec::new();

    for row in &rows {
        let findings = scan_body(&row.body, &row.language);
        let filtered: Vec<_> = findings
            .into_iter()
            .filter(|f| f.severity >= min_sev)
            .collect();
        if !filtered.is_empty() {
            entries_with_findings += 1;
            let label = match &row.variant {
                Some(v) => format!("{}::{}", row.word, v),
                None => row.word.clone(),
            };
            for f in filtered {
                total_findings += 1;
                all_findings.push((label.clone(), f));
            }
        }
    }

    match format {
        "json" => {
            let json_output: Vec<serde_json::Value> = all_findings
                .iter()
                .map(|(label, f)| {
                    serde_json::json!({
                        "entry": label,
                        "severity": format!("{}", f.severity),
                        "category": f.category,
                        "rule_id": f.rule_id,
                        "message": f.message,
                        "evidence": f.evidence,
                        "line": f.line,
                        "remediation": f.remediation,
                    })
                })
                .collect();
            match serde_json::to_string_pretty(&json_output) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("nom: json error: {e}"),
            }
        }
        _ => {
            println!();
            println!("{}", "=".repeat(70));
            println!("Security Audit Report");
            println!("{}", "=".repeat(70));

            if all_findings.is_empty() {
                println!("No findings at severity >= {min_severity}.");
            } else {
                let mut current_entry = String::new();
                for (label, f) in &all_findings {
                    if *label != current_entry {
                        println!(
                            "\n  {} [{}]:",
                            label,
                            rows.iter()
                                .find(|r| {
                                    let l = match &r.variant {
                                        Some(v) => format!("{}::{}", r.word, v),
                                        None => r.word.clone(),
                                    };
                                    l == *label
                                })
                                .map(|r| r.language.as_str())
                                .unwrap_or("?")
                        );
                        current_entry = label.clone();
                    }
                    println!(
                        "    [{:>8}] {} ({}): {}",
                        format!("{}", f.severity),
                        f.rule_id,
                        f.category,
                        f.message
                    );
                    if let Some(evidence) = &f.evidence {
                        if evidence.len() <= 100 {
                            println!("             evidence: {evidence}");
                        }
                    }
                    if let Some(rem) = &f.remediation {
                        println!("             fix: {rem}");
                    }
                }
            }

            // Category breakdown
            if !all_findings.is_empty() {
                let mut category_counts: std::collections::BTreeMap<&str, usize> =
                    std::collections::BTreeMap::new();
                for (_, f) in &all_findings {
                    *category_counts.entry(f.category.as_str()).or_insert(0) += 1;
                }
                println!("\n  Categories:");
                for (cat, count) in &category_counts {
                    let label = match *cat {
                        "injection" => "Injection (SQLi/CMDi)",
                        "secrets" => "Secrets & Credentials (TruffleHog)",
                        "crypto" => "Weak Cryptography",
                        "payload" => "Attack Payloads (Metasploit)",
                        "xss" => "Cross-Site Scripting",
                        "deserialization" => "Insecure Deserialization",
                        "path_traversal" => "Path Traversal",
                        "config" => "Insecure Configuration",
                        "web" => "Web Vulnerabilities (Kali/OWASP)",
                        "credential" => "Credential Security (Kali)",
                        "execution" => "Code Execution (Kali)",
                        "network" => "Network Security (Kali/Suricata)",
                        "data_handling" => "Data Handling (Forensics)",
                        "protocol" => "Protocol Analysis (Suricata)",
                        "guardrail" => "Guardrail Violations (RedAmon)",
                        other => other,
                    };
                    println!("    {label}: {count}");
                }
            }

            println!("\n{}", "=".repeat(70));
            let score = if !all_findings.is_empty() {
                let finding_refs: Vec<_> = all_findings.iter().map(|(_, f)| f.clone()).collect();
                security_score(&finding_refs)
            } else {
                1.0
            };
            println!(
                "Scanned: {} entries | Findings: {} | Affected: {} | Score: {:.2}",
                rows.len(),
                total_findings,
                entries_with_findings,
                score,
            );
        }
    }

    if total_findings > 0 { 1 } else { 0 }
}
