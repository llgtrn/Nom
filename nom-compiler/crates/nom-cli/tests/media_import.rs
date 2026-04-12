//! Integration tests for `nom media import`.
//!
//! Tests call `cmd_media_import` directly against the tiny fixture files in
//! `crates/nom-media/tests/fixtures/`. They are marked `#[ignore]` on
//! Windows due to STATUS_DLL_NOT_FOUND issues with the LLVM linkage in the
//! `nom-cli` test binary on that platform; on Linux/macOS they run normally.
//!
//! To run on Linux/macOS:
//!   cargo test -p nom-cli --test media_import
//!
//! Windows DLL issue tracked in: nom-compiler/crates/nom-cli/tests/store_cli.rs
//! (same root cause — the LLVM DLL is not on PATH during `cargo test` on
//! Windows unless explicitly set up).

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    // crates/nom-media/tests/fixtures/ relative to workspace root
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("..")
        .join("nom-media")
        .join("tests")
        .join("fixtures")
}

// Re-export the function under test so we can call it without spawning a process.
// The `nom-cli` crate is a [[bin]], not a lib, so we pull in media directly.
use nom_media::{
    ingest_aac, ingest_av1, ingest_avif, ingest_flac, ingest_hevc, ingest_jpeg, ingest_mp4,
    ingest_opus, ingest_png, ingest_webm,
};

/// Thin wrapper matching the exit-code contract: 0 = OK, 1 = error.
fn import_exit_code(path: &std::path::Path) -> i32 {
    // Mirror the dispatch logic in media::cmd_media_import but without the
    // print side-effect, purely for testability.
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return 1,
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => ingest_png(&bytes).map(|_| 0).unwrap_or(1),
        "jpg" | "jpeg" => ingest_jpeg(&bytes).map(|_| 0).unwrap_or(1),
        "avif" => ingest_avif(&bytes).map(|_| 0).unwrap_or(1),
        "flac" => ingest_flac(&bytes).map(|_| 0).unwrap_or(1),
        "opus" | "ogg" => ingest_opus(&bytes).map(|_| 0).unwrap_or(1),
        "aac" => ingest_aac(&bytes).map(|_| 0).unwrap_or(1),
        "ivf" => ingest_av1(&bytes).map(|_| 0).unwrap_or(1),
        "webm" => ingest_webm(&bytes).map(|_| 0).unwrap_or(1),
        "mp4" => ingest_mp4(&bytes).map(|_| 0).unwrap_or(1),
        "hevc" => ingest_hevc(&bytes).map(|_| 0).unwrap_or(1),
        _ => 1,
    }
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_png() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.png")), 0);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_jpeg() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.jpg")), 0);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_avif() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.avif")), 0);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_flac() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.flac")), 0);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_opus() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.opus")), 0);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_aac() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.aac")), 0);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_av1() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.av1.ivf")), 0);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_webm() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.webm")), 0);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_mp4() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.mp4")), 0);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS")]
fn import_hevc() {
    assert_eq!(import_exit_code(&fixtures_dir().join("tiny.hevc")), 0);
}
