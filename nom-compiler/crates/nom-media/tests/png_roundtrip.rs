/// Integration test for the PNG codec (§5.16.13 order #1).
///
/// Verifies:
///  1. `ingest_png` decodes the fixture and returns correct metadata.
///  2. `verify_png_roundtrip` confirms pixel-identical decode → re-encode.
///  3. Two back-to-back `ingest_png` calls on the same bytes produce
///     byte-identical `canonical_bytes` (determinism of the encoder).

use nom_media::{ingest_png, verify_png_roundtrip};

static TINY_PNG: &[u8] = include_bytes!("fixtures/tiny.png");

/// The fixture is a 4×4 RGBA gradient created with Pillow.
#[test]
fn ingest_png_returns_correct_dimensions_and_color_type() {
    let result = ingest_png(TINY_PNG).expect("ingest_png should succeed on a valid PNG");
    assert_eq!(result.width, 4, "expected width 4");
    assert_eq!(result.height, 4, "expected height 4");
    assert_eq!(result.color_type, "rgba8", "expected rgba8 colour type");
    assert!(!result.canonical_bytes.is_empty(), "canonical_bytes must not be empty");
    // Canonical bytes must still be a valid PNG (magic header bytes).
    assert_eq!(&result.canonical_bytes[..8], b"\x89PNG\r\n\x1a\n", "canonical_bytes must start with PNG signature");
}

#[test]
fn verify_png_roundtrip_succeeds_on_valid_png() {
    verify_png_roundtrip(TINY_PNG).expect("round-trip should preserve pixel content");
}

#[test]
fn ingest_png_is_deterministic() {
    let first = ingest_png(TINY_PNG).expect("first ingest_png call should succeed");
    let second = ingest_png(TINY_PNG).expect("second ingest_png call should succeed");
    assert_eq!(
        first.canonical_bytes, second.canonical_bytes,
        "two back-to-back ingest_png calls must produce byte-identical canonical_bytes"
    );
}

#[test]
fn ingest_png_rejects_invalid_bytes() {
    let bad = b"this is not a PNG";
    let err = ingest_png(bad).expect_err("ingest_png should fail on garbage input");
    let msg = err.to_string();
    assert!(msg.contains("PNG codec error"), "error should mention PNG: {msg}");
}
