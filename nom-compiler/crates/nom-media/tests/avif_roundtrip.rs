/// Integration test for the AVIF codec (§5.16.13 order #5 — §4.4.6 canonical image format).
///
/// Verifies:
///  1. `ingest_avif` parses the fixture and returns correct metadata.
///  2. `verify_avif_roundtrip` confirms the round-trip gate passes.
///  3. Two back-to-back `ingest_avif` calls on the same bytes produce
///     byte-identical `canonical_bytes` (determinism of the identity-map encoder).
///  4. `ingest_avif` rejects garbage input.
///
/// # Encoder status note
///
/// `canonical_bytes` is currently an identity copy of the input (no pure-Rust
/// AVIF decoder is available on Windows without nasm/FFI as of §5.16.13 order
/// #5). The PSNR is implicitly infinite (same bytes = same image). These tests
/// will be updated when a decoder + ravif re-encode path lands.

use nom_media::{ingest_avif, verify_avif_roundtrip};

/// 4×4 RGBA AVIF fixture, generated from `tiny.png` via Pillow 12.1.1:
/// `img.save("tiny.avif")` — still-picture, 8-bit, ~315 bytes.
static TINY_AVIF: &[u8] = include_bytes!("fixtures/tiny.avif");

#[test]
fn ingest_avif_returns_correct_dimensions() {
    let result = ingest_avif(TINY_AVIF).expect("ingest_avif should succeed on a valid AVIF");
    assert_eq!(result.width, 4, "expected width 4");
    assert_eq!(result.height, 4, "expected height 4");
    assert!(
        !result.color_type.is_empty(),
        "color_type must not be empty"
    );
    assert!(
        !result.canonical_bytes.is_empty(),
        "canonical_bytes must not be empty"
    );
}

#[test]
fn ingest_avif_canonical_bytes_start_with_ftyp_box() {
    let result = ingest_avif(TINY_AVIF).expect("ingest_avif should succeed");
    // AVIF files are ISO-BMFF containers; the first box is `ftyp` with
    // the 'avif' brand. Box layout: 4-byte size + 4-byte type 'ftyp'.
    assert!(
        result.canonical_bytes.len() >= 8,
        "canonical_bytes too short to contain ftyp box"
    );
    assert_eq!(
        &result.canonical_bytes[4..8],
        b"ftyp",
        "canonical_bytes must start with ftyp box"
    );
}

#[test]
fn verify_avif_roundtrip_passes_on_valid_fixture() {
    verify_avif_roundtrip(TINY_AVIF)
        .expect("round-trip gate should pass on a valid AVIF fixture");
}

#[test]
fn ingest_avif_is_deterministic() {
    let first = ingest_avif(TINY_AVIF).expect("first ingest_avif should succeed");
    let second = ingest_avif(TINY_AVIF).expect("second ingest_avif should succeed");
    assert_eq!(
        first.canonical_bytes, second.canonical_bytes,
        "two back-to-back ingest_avif calls must produce byte-identical canonical_bytes"
    );
}

#[test]
fn ingest_avif_rejects_invalid_bytes() {
    let bad = b"this is not an AVIF file";
    let err = ingest_avif(bad).expect_err("ingest_avif should fail on garbage input");
    let msg = err.to_string();
    assert!(
        msg.contains("AVIF codec error"),
        "error should mention AVIF: {msg}"
    );
}
