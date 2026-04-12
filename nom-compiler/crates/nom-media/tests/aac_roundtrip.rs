/// Integration test for the AAC codec (§5.16.13 order #7).
///
/// Verifies:
///  1. `ingest_aac` parses the ADTS fixture and returns correct metadata.
///  2. `verify_aac_roundtrip` confirms metadata round-trips cleanly.
///  3. Two back-to-back `ingest_aac` calls on the same bytes produce
///     byte-identical `canonical_bytes` (determinism — trivially satisfied
///     by identity mapping; will still hold after a real encoder lands if
///     the encoder is deterministic at fixed settings).
///  4. `ingest_aac` rejects garbage input.
///  5. `body_kind::AAC` constant is `"aac"` and is a known kind.
///
/// Fixture: `tests/fixtures/tiny.aac` — 3 hand-crafted ADTS frames,
/// 44100 Hz, stereo (channel_configuration = 2), AAC-LC, no CRC.
/// Generated without ffmpeg by computing the ADTS bit fields manually:
///   - syncword = 0xFFF (12 bits)
///   - ID = 0 (MPEG-4), layer = 00, protection_absent = 1
///   - profile_ObjectType = 01 (AAC-LC, +1 → profile 2)
///   - sampling_frequency_index = 4 (44100 Hz per ISO 14496-3 Table 1.13)
///   - channel_configuration = 2 (stereo)
///   - aac_frame_length = 8 (7-byte header + 1-byte payload)
///   - adts_buffer_fullness = 0x7FF (VBR)
/// Each frame bytes: FF F1 50 80 01 1F FC 00
/// Expected duration: 3 × 1024 × 1000 / 44100 = 69 ms.

use nom_media::{ingest_aac, verify_aac_roundtrip};
use nom_types::body_kind;

/// Minimal ADTS-wrapped AAC file: 44100 Hz stereo, 3 × 1024-sample frames.
/// Hand-crafted ADTS headers (see module doc above for bit-field derivation).
static TINY_AAC: &[u8] = include_bytes!("fixtures/tiny.aac");

#[test]
fn ingest_aac_returns_correct_metadata() {
    let result = ingest_aac(TINY_AAC).expect("ingest_aac should succeed on a valid ADTS file");
    assert_eq!(result.sample_rate, 44100, "expected sample_rate 44100 Hz");
    assert_eq!(result.channels, 2, "expected 2 channels (stereo)");
    // duration_ms = 3 frames × 1024 samples × 1000 ms/s / 44100 samples/s = 69 ms
    assert_eq!(result.duration_ms, 69, "expected duration_ms 69");
    assert!(
        !result.canonical_bytes.is_empty(),
        "canonical_bytes must not be empty"
    );
    // ADTS stream must start with syncword 0xFF, 0xF? (12-bit 0xFFF).
    assert_eq!(result.canonical_bytes[0], 0xFF, "canonical_bytes[0] must be 0xFF (ADTS syncword)");
    assert_eq!(
        result.canonical_bytes[1] & 0xF0,
        0xF0,
        "canonical_bytes[1] upper nibble must be 0xF (ADTS syncword)"
    );
}

#[test]
fn verify_aac_roundtrip_passes() {
    verify_aac_roundtrip(TINY_AAC)
        .expect("round-trip metadata should match (identity mapping)");
}

#[test]
fn ingest_aac_is_deterministic() {
    let first = ingest_aac(TINY_AAC).expect("first ingest_aac call should succeed");
    let second = ingest_aac(TINY_AAC).expect("second ingest_aac call should succeed");
    assert_eq!(
        first.canonical_bytes, second.canonical_bytes,
        "two back-to-back ingest_aac calls must produce byte-identical canonical_bytes"
    );
}

#[test]
fn ingest_aac_rejects_invalid_bytes() {
    let bad = b"this is not an ADTS-AAC file at all";
    let err = ingest_aac(bad).expect_err("ingest_aac should fail on garbage input");
    let msg = err.to_string();
    assert!(
        msg.contains("AAC codec error"),
        "error should mention AAC: {msg}"
    );
}

#[test]
fn aac_body_kind_constant_is_known() {
    assert_eq!(body_kind::AAC, "aac");
    assert!(
        body_kind::is_known(body_kind::AAC),
        "body_kind::AAC must be recognized by is_known"
    );
}
