/// Integration tests for the HEVC Annex-B codec (§5.16.13 order #10 — decode-only).
///
/// Verifies:
///  1. `ingest_hevc` parses the fixture and returns correct metadata.
///  2. `verify_hevc_roundtrip` confirms the round-trip gate passes.
///  3. Two back-to-back `ingest_hevc` calls produce byte-identical `canonical_bytes`.
///  4. `ingest_hevc` rejects garbage input.
///  5. `body_kind::HEVC` constant is `"hevc"` and is a known kind.
///
/// # Decoder status
///
/// `canonical_bytes` is always an identity copy of the input. This is the
/// correct and final design for this codec (decode-only, §5.16.13 order #10).
/// No re-encode is planned.
///
/// # Fixture: `tests/fixtures/tiny.hevc`
///
/// Hand-crafted 70-byte HEVC Annex-B bitstream containing 5 NAL units:
///   NAL type 32 (VPS) — minimal 18-byte body
///   NAL type 33 (SPS) — encodes profile_idc=1 (Main), 16×16
///   NAL type 34 (PPS) — minimal 3-byte body
///   NAL type 1  (TRAIL_R) — dummy slice ×2
///
/// No ffmpeg or PyAV was available; the fixture was hand-computed from the
/// H.265 §7.3.2.2.1 SPS RBSP layout and verified by decoding the packed
/// bits back to profile_idc=1, width=16, height=16.
use nom_media::{ingest_hevc, verify_hevc_roundtrip};
use nom_types::body_kind;

static TINY_HEVC: &[u8] = include_bytes!("fixtures/tiny.hevc");

#[test]
fn ingest_hevc_returns_correct_nal_count() {
    let result =
        ingest_hevc(TINY_HEVC).expect("ingest_hevc should succeed on a valid Annex-B fixture");
    assert_eq!(
        result.nal_unit_count, 5,
        "expected 5 NAL units (VPS+SPS+PPS+2 slices)"
    );
}

#[test]
fn ingest_hevc_returns_correct_dimensions_and_profile() {
    let result = ingest_hevc(TINY_HEVC).expect("ingest_hevc should succeed");
    assert_eq!(result.width, 16, "expected width=16");
    assert_eq!(result.height, 16, "expected height=16");
    assert_eq!(
        result.profile_idc, 1,
        "expected profile_idc=1 (Main Profile)"
    );
}

#[test]
fn ingest_hevc_canonical_bytes_are_identity_copy() {
    let result = ingest_hevc(TINY_HEVC).expect("ingest_hevc should succeed");
    assert_eq!(
        result.canonical_bytes, TINY_HEVC,
        "canonical_bytes must be an identity copy of the input (decode-only design)"
    );
}

#[test]
fn verify_hevc_roundtrip_passes_on_valid_fixture() {
    verify_hevc_roundtrip(TINY_HEVC).expect("round-trip gate should pass on a valid HEVC fixture");
}

#[test]
fn ingest_hevc_is_deterministic() {
    let first = ingest_hevc(TINY_HEVC).expect("first ingest_hevc should succeed");
    let second = ingest_hevc(TINY_HEVC).expect("second ingest_hevc should succeed");
    assert_eq!(
        first.canonical_bytes, second.canonical_bytes,
        "two back-to-back ingest_hevc calls must produce byte-identical canonical_bytes"
    );
}

#[test]
fn ingest_hevc_rejects_invalid_bytes() {
    let bad = b"this is not an HEVC Annex-B stream";
    let err = ingest_hevc(bad).expect_err("ingest_hevc should fail on garbage input");
    let msg = err.to_string();
    assert!(
        msg.contains("HEVC codec error"),
        "error should mention HEVC: {msg}"
    );
}

#[test]
fn hevc_body_kind_constant_is_known() {
    assert_eq!(body_kind::HEVC, "hevc");
    assert!(
        body_kind::is_known(body_kind::HEVC),
        "body_kind::HEVC must be recognized by is_known"
    );
}
