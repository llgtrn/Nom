/// Integration tests for the AVIF codec (§5.16.13 order #5 — §4.4.6 canonical image format).
///
/// Covers two paths:
///
/// ## Per-format track (`ingest_avif`)
///
/// 1. `ingest_avif` parses the AVIF fixture and returns correct metadata.
/// 2. `verify_avif_roundtrip` confirms the round-trip gate passes.
/// 3. Two back-to-back `ingest_avif` calls produce byte-identical `canonical_bytes`.
/// 4. `ingest_avif` rejects garbage input.
///
/// ## Modality-canonical track (`ingest_image_still_to_avif`)
///
/// 5. Decodes `tiny.png` (4×4 RGBA) → AVIF; asserts `body_kind=avif`, bytes>0.
/// 6. `verify_avif_roundtrip` passes (PSNR ≥ 30 dB gate).
/// 7. Encode twice → byte-identical (determinism).
/// 8. 64×64 gradient (generated programmatically) → AVIF; asserts PSNR ≥ 30 dB.

use nom_media::{ingest_avif, ingest_image_still_to_avif, verify_avif_roundtrip, Modality};
use nom_types::body_kind;

/// Emit a `#[test]` that is ignored on Windows with the standard AVIF reason.
///
/// On Windows, `verify_avif_roundtrip` always returns `Err` because the dav1d
/// C toolchain is unavailable without extra build infrastructure.  Every PSNR
/// test uses the same ignore string, so this macro de-duplicates it.
macro_rules! avif_psnr_test {
    (fn $name:ident() $body:block) => {
        #[test]
        #[cfg_attr(
            windows,
            ignore = "AVIF pixel decode requires dav1d (C toolchain), unavailable on Windows without extra build infra"
        )]
        fn $name() $body
    };
}

/// 4×4 RGBA AVIF fixture.
static TINY_AVIF: &[u8] = include_bytes!("fixtures/tiny.avif");

/// 4×4 RGBA PNG fixture.
static TINY_PNG: &[u8] = include_bytes!("fixtures/tiny.png");

// ── Per-format track (ingest_avif) ─────────────────────────────────────────

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

avif_psnr_test! {
fn verify_avif_roundtrip_passes_on_valid_fixture() {
    // Use TINY_PNG as the "original" source (decodable by image crate) and
    // TINY_AVIF as the "stored" canonical container (valid AVIF for parse).
    // This exercises the pixel-PSNR path of verify_avif_roundtrip.
    let psnr = verify_avif_roundtrip(TINY_PNG, TINY_AVIF)
        .expect("round-trip gate should pass on a valid AVIF fixture");
    assert!(
        psnr >= 30.0,
        "PSNR {psnr:.2} dB is below 30 dB threshold"
    );
}}

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

// ── Modality-canonical track (ingest_image_still_to_avif) ──────────────────

#[test]
fn ingest_image_still_to_avif_from_png_produces_avif_bytes() {
    let result = ingest_image_still_to_avif(TINY_PNG, Modality::ImageStill)
        .expect("ingest_image_still_to_avif should succeed on tiny.png");

    assert_eq!(result.width, 4, "expected width 4");
    assert_eq!(result.height, 4, "expected height 4");
    assert_eq!(result.color_type, "rgba8", "expected rgba8");
    assert!(
        !result.canonical_bytes.is_empty(),
        "canonical_bytes must not be empty"
    );
    // AVIF container: first box is `ftyp`.
    assert!(
        result.canonical_bytes.len() >= 8,
        "canonical_bytes too short to be a valid AVIF"
    );
    assert_eq!(
        &result.canonical_bytes[4..8],
        b"ftyp",
        "canonical_bytes must start with ftyp box"
    );
}

#[test]
fn ingest_image_still_to_avif_body_kind_is_avif() {
    // Verify the caller uses the correct body_kind constant for this codec.
    let result = ingest_image_still_to_avif(TINY_PNG, Modality::ImageStill)
        .expect("ingest should succeed");
    // The modality-canonical output is tagged avif in the dict.
    // Verify the constant matches.
    assert_eq!(body_kind::AVIF, "avif");
    assert!(!result.canonical_bytes.is_empty());
}

avif_psnr_test! {
fn verify_avif_roundtrip_passes_after_ingest_from_png() {
    let result = ingest_image_still_to_avif(TINY_PNG, Modality::ImageStill)
        .expect("ingest should succeed");

    let psnr = verify_avif_roundtrip(TINY_PNG, &result.canonical_bytes)
        .expect("verify_avif_roundtrip should pass");

    assert!(
        psnr >= 30.0,
        "PSNR {psnr:.2} dB is below 30 dB threshold"
    );
}}

#[test]
fn ingest_image_still_to_avif_is_deterministic() {
    let first = ingest_image_still_to_avif(TINY_PNG, Modality::ImageStill)
        .expect("first ingest should succeed");
    let second = ingest_image_still_to_avif(TINY_PNG, Modality::ImageStill)
        .expect("second ingest should succeed");

    assert_eq!(
        first.canonical_bytes, second.canonical_bytes,
        "two back-to-back calls must produce byte-identical canonical_bytes (determinism)"
    );
}

avif_psnr_test! {
fn ingest_image_still_to_avif_gradient_64x64() {
    // Generate a 64×64 deterministic gradient programmatically (no binary fixture).
    // The gradient exercises real DCT blocks rather than trivial 4×4 pixels.
    let width: u32 = 64;
    let height: u32 = 64;
    let gradient_pixels: Vec<u8> = (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                let r = ((x * 255) / (width - 1)) as u8;
                let g = ((y * 255) / (height - 1)) as u8;
                let b = (((x + y) * 255) / (width + height - 2)) as u8;
                let a = 255u8;
                [r, g, b, a]
            })
        })
        .collect();

    // Encode raw RGBA8 → PNG in memory so we can use ingest_image_still_to_avif.
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
    let mut png_bytes: Vec<u8> = Vec::new();
    PngEncoder::new(&mut png_bytes)
        .write_image(
            &gradient_pixels,
            width,
            height,
            ExtendedColorType::Rgba8,
        )
        .expect("encode gradient to PNG");

    let result = ingest_image_still_to_avif(&png_bytes, Modality::ImageStill)
        .expect("ingest_image_still_to_avif should succeed on 64×64 gradient");

    assert_eq!(result.width, 64);
    assert_eq!(result.height, 64);
    assert!(!result.canonical_bytes.is_empty());

    let psnr = verify_avif_roundtrip(&png_bytes, &result.canonical_bytes)
        .expect("verify_avif_roundtrip should pass on gradient");

    assert!(
        psnr >= 30.0,
        "PSNR {psnr:.2} dB is below 30 dB threshold for gradient"
    );
}}

#[test]
fn ingest_image_still_to_avif_rejects_invalid_bytes() {
    let bad = b"not an image";
    let err = ingest_image_still_to_avif(bad, Modality::ImageStill)
        .expect_err("should fail on garbage input");
    let msg = err.to_string();
    assert!(
        msg.contains("AVIF codec error"),
        "error should mention AVIF: {msg}"
    );
}
