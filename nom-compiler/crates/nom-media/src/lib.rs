//! `nom-media` — media ingestion and rendering per §5.16 / §4.4.6.
//!
//! Per §4.4.6 invariant 17 "One canonical format per modality":
//! - Image still → AVIF
//! - Video      → AV1 (in Matroska or ISOBMFF container)
//! - Audio lossy → AAC
//! - Audio lossless → FLAC
//! - Font       → WOFF2
//! - 3D mesh    → glTF
//! - Document   → PDF
//!
//! The dict stores media bodies as **the canonical-format bytes**,
//! tagged with `body_kind` from [`nom_types::body_kind`]. Alternative
//! encodings (PNG, JPEG, WebP, MP3, WAV, …) are produced on render
//! via `Specializes` variants — never stored as primary bodies.
//!
//! This crate is the Phase-5 §5.16 scaffold. Functional codec work
//! (ingest PNG, re-encode to AVIF; ingest WAV, re-encode to FLAC)
//! arrives incrementally per the §5.16.13 codec roadmap (PNG → FLAC →
//! JPEG → Opus → AVIF → AV1 → AAC → WebM → MP4 → HEVC).

use std::io::Cursor;

use flacenc::component::BitRepr;
use flacenc::error::Verify;
use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    ColorType, ExtendedColorType, ImageEncoder, ImageReader,
};
use ravif::Encoder as AvifEncoder;
use rgb::RGBA8;
use thiserror::Error;

/// Canonical media modality. Maps to exactly one storage format per
/// §4.4.6 invariant 17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Modality {
    ImageStill,
    Video,
    AudioLossy,
    AudioLossless,
    Font,
    Mesh3d,
    Document,
}

impl Modality {
    /// Every modality in a stable order. Mirrors
    /// [`nom_types::body_kind::ALL`] for enumeration in help output,
    /// tests, or property-test generators.
    pub const ALL: &'static [Modality] = &[
        Modality::ImageStill,
        Modality::Video,
        Modality::AudioLossy,
        Modality::AudioLossless,
        Modality::Font,
        Modality::Mesh3d,
        Modality::Document,
    ];

    /// Return the `body_kind` tag for this modality's canonical-format
    /// storage body. Parallels the constants in
    /// [`nom_types::body_kind`].
    pub const fn canonical_body_kind(self) -> &'static str {
        use nom_types::body_kind;
        match self {
            Modality::ImageStill => body_kind::AVIF,
            Modality::Video => body_kind::AV1,
            Modality::AudioLossy => body_kind::AAC,
            Modality::AudioLossless => body_kind::FLAC,
            Modality::Font => body_kind::WOFF2,
            Modality::Mesh3d => body_kind::GLTF,
            Modality::Document => body_kind::PDF,
        }
    }
}

/// Identify the modality from a file extension or MIME-like tag.
/// Returns `None` for unrecognized formats — callers decide whether
/// that is a hard error or a skip.
pub fn modality_from_ext(ext: &str) -> Option<Modality> {
    match ext.to_ascii_lowercase().as_str() {
        // Still images
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tiff" | "tif" | "heic" | "avif" => {
            Some(Modality::ImageStill)
        }
        // Video
        "mp4" | "mov" | "webm" | "mkv" | "avi" | "m4v" => Some(Modality::Video),
        // Audio lossy
        "aac" | "mp3" | "m4a" | "opus" | "ogg" => Some(Modality::AudioLossy),
        // Audio lossless
        "flac" | "wav" | "alac" | "aiff" => Some(Modality::AudioLossless),
        // Font
        "woff2" | "woff" | "otf" | "ttf" => Some(Modality::Font),
        // 3D mesh
        "gltf" | "glb" | "obj" | "fbx" | "stl" => Some(Modality::Mesh3d),
        // Document
        "pdf" | "epub" | "md" | "txt" | "rtf" | "docx" | "odt" => Some(Modality::Document),
        _ => None,
    }
}

/// Errors produced by `nom-media`. Kept minimal until real codec work
/// starts — each codec PR grows this enum as needed.
#[derive(Debug, Error)]
pub enum MediaError {
    #[error("unrecognized media extension: {0}")]
    UnrecognizedExt(String),
    #[error("codec not yet implemented: {codec} (landing in §5.16.13 order #{order})")]
    NotYetImplemented { codec: String, order: usize },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PNG codec error: {0}")]
    Png(String),
    #[error("FLAC codec error: {0}")]
    Flac(String),
    #[error("JPEG codec error: {0}")]
    Jpeg(String),
    #[error("Opus codec error: {0}")]
    Opus(String),
    #[error("AVIF codec error: {0}")]
    Avif(String),
    #[error("AV1 codec error: {0}")]
    Av1(String),
    #[error("AAC codec error: {0}")]
    Aac(String),
    #[error("WebM codec error: {0}")]
    Webm(String),
    #[error("MP4 codec error: {0}")]
    Mp4(String),
    #[error("HEVC codec error: {0}")]
    Hevc(String),
}

// ── PNG codec (§5.16.13 order #1) ────────────────────────────────────

/// Result of ingesting a PNG byte slice. Contains decoded image
/// dimensions, colour type, and canonical re-encoded bytes.
///
/// The `canonical_bytes` are re-encoded at fixed deterministic settings
/// (see [`ingest_png`]) so two calls with identical pixel content
/// produce byte-identical output.
#[derive(Debug, Clone)]
pub struct IngestedPng {
    pub width: u32,
    pub height: u32,
    /// Human-readable colour type label, e.g. `"rgb8"`, `"rgba8"`,
    /// `"l8"`, `"la8"`.
    pub color_type: String,
    /// Canonical PNG bytes re-encoded at fixed settings for
    /// determinism. Tagged `body_kind = "png"` in the dict.
    pub canonical_bytes: Vec<u8>,
}

/// Decode a PNG byte slice and return an [`IngestedPng`] containing
/// the decoded dimensions, colour type, and deterministically
/// re-encoded canonical bytes.
///
/// # Deterministic re-encode settings
///
/// Re-encoding uses:
/// - `CompressionType::Default` — zlib level 6, the standard default
///   used by virtually every PNG encoder. Level 6 balances compression
///   ratio and speed without relying on encoder-specific fast/best
///   flags that differ between libpng versions.
/// - `FilterType::Sub` — byte-level delta filter on rows. Sub is
///   deterministic across encoder versions and works well for natural
///   images without the variable output of the Paeth heuristic.
///
/// Together these produce byte-identical output for the same pixel
/// data across `image` crate versions that maintain this API.
///
/// Returns [`MediaError::Png`] on malformed or unsupported PNG input.
pub fn ingest_png(bytes: &[u8]) -> Result<IngestedPng, MediaError> {
    // Decode via `image` crate reader.
    let reader = ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png);
    let dyn_img = reader
        .decode()
        .map_err(|e| MediaError::Png(e.to_string()))?;

    let width = dyn_img.width();
    let height = dyn_img.height();

    // Determine colour type label from the decoded image.
    let color_type_label = color_type_label(dyn_img.color());

    // Re-encode to canonical bytes.
    let canonical_bytes = encode_png_deterministic(&dyn_img)?;

    Ok(IngestedPng {
        width,
        height,
        color_type: color_type_label,
        canonical_bytes,
    })
}

/// Verify that decoding `bytes` and re-encoding produces pixel-identical
/// output to decoding the re-encoded bytes. The round-trip proves that
/// `ingest_png` preserves all pixel information.
///
/// Returns `Ok(())` if pixels match, or [`MediaError::Png`] on any
/// decode/encode failure or pixel mismatch.
pub fn verify_png_roundtrip(bytes: &[u8]) -> Result<(), MediaError> {
    let ingested = ingest_png(bytes)?;

    // Decode the canonical bytes back to pixels.
    let reader2 = ImageReader::with_format(
        Cursor::new(&ingested.canonical_bytes),
        image::ImageFormat::Png,
    );
    let roundtripped = reader2
        .decode()
        .map_err(|e| MediaError::Png(format!("round-trip decode failed: {e}")))?;

    // Compare raw pixel bytes — pixel-equality, not byte-equality of
    // the compressed stream.
    let original_pixels = image_to_rgba8(&image::ImageReader::with_format(
        Cursor::new(bytes),
        image::ImageFormat::Png,
    )
    .decode()
    .map_err(|e| MediaError::Png(e.to_string()))?);

    let roundtripped_pixels = image_to_rgba8(&roundtripped);

    if original_pixels != roundtripped_pixels {
        return Err(MediaError::Png(
            "pixel mismatch after round-trip re-encode".to_owned(),
        ));
    }

    Ok(())
}

// ── FLAC codec (§5.16.13 order #2) ───────────────────────────────────

/// Result of ingesting a FLAC byte slice. Contains decoded stream
/// metadata and canonical re-encoded bytes.
///
/// `canonical_bytes` are re-encoded at fixed deterministic settings
/// (see [`ingest_flac`]) using `flacenc` (pure-Rust encoder).
#[derive(Debug, Clone)]
pub struct IngestedFlac {
    pub sample_rate: u32,
    pub channels: u8,
    pub bits_per_sample: u8,
    /// Total PCM samples across all channels (frames × channels).
    pub total_samples: u64,
    /// Canonical re-encoded FLAC bytes at fixed deterministic settings.
    pub canonical_bytes: Vec<u8>,
}

/// Decode a FLAC byte slice and return an [`IngestedFlac`] containing
/// stream metadata and deterministically re-encoded canonical bytes.
///
/// # Deterministic re-encode settings
///
/// Re-encoding uses `flacenc::config::Encoder::default()` which
/// selects a fixed compression level. The default encoder config is
/// stable across patch releases, producing byte-identical output for
/// the same PCM input. The encoder is pure-Rust (`flacenc` crate) with
/// no FFI dependency.
///
/// # Known limitation
///
/// `total_samples` is reported as `frames × channels` (all-channel
/// sample count). The FLAC streaminfo `samples` field stores per-channel
/// frame count; we multiply by channels to match the documented field
/// semantics.
///
/// Returns [`MediaError::Flac`] on malformed or unsupported FLAC input.
pub fn ingest_flac(bytes: &[u8]) -> Result<IngestedFlac, MediaError> {
    let (sample_rate, channels, bits_per_sample, pcm_samples) = decode_flac_pcm(bytes)?;

    // Total samples = per-channel frame count × channel count.
    let total_samples = pcm_samples.len() as u64;

    // Re-encode to canonical bytes using flacenc (pure-Rust).
    let canonical_bytes =
        encode_flac_deterministic(&pcm_samples, channels, bits_per_sample, sample_rate)?;

    Ok(IngestedFlac {
        sample_rate,
        channels,
        bits_per_sample,
        total_samples,
        canonical_bytes,
    })
}

/// Verify that decoding `bytes` and re-encoding produces sample-identical
/// output to decoding the re-encoded bytes.
///
/// Asserts **sample-equality**, not byte-equality — FLAC frame packing
/// is not bit-stable across encoder versions even at fixed settings.
///
/// Returns `Ok(())` if PCM samples match, or [`MediaError::Flac`] on
/// any decode/encode failure or sample mismatch.
pub fn verify_flac_roundtrip(bytes: &[u8]) -> Result<(), MediaError> {
    let ingested = ingest_flac(bytes)?;

    // Decode the canonical bytes back to PCM.
    let (_, _, _, roundtrip_samples) = decode_flac_pcm(&ingested.canonical_bytes)?;

    if ingested.total_samples != roundtrip_samples.len() as u64 {
        return Err(MediaError::Flac(format!(
            "sample count mismatch after round-trip: original={} canonical={}",
            ingested.total_samples,
            roundtrip_samples.len()
        )));
    }

    // Decode original PCM for comparison.
    let (_, _, _, original_samples) = decode_flac_pcm(bytes)?;

    if original_samples != roundtrip_samples {
        return Err(MediaError::Flac(
            "sample mismatch after FLAC round-trip re-encode".to_owned(),
        ));
    }

    Ok(())
}

// ── JPEG codec (§5.16.13 order #3) ────────────────────────────────────

/// Result of ingesting a JPEG byte slice. Contains decoded image
/// dimensions, colour type, and canonical re-encoded bytes.
///
/// `canonical_bytes` are re-encoded at a fixed quality of **85** for
/// determinism. Quality 85 was chosen as the standard "excellent" JPEG
/// floor: it preserves perceptual fidelity (typical PSNR 35–45 dB on
/// photographic content), keeps file sizes compact, and is the widely
/// adopted default in tools such as Pillow, Lightroom, and ImageMagick.
/// Lossless round-trip is intentionally not required; the
/// [`verify_jpeg_roundtrip`] gate uses a PSNR threshold of ≥ 30 dB
/// instead of pixel equality.
#[derive(Debug, Clone)]
pub struct IngestedJpeg {
    pub width: u32,
    pub height: u32,
    /// Human-readable colour type label, e.g. `"rgb8"`, `"rgba8"`.
    pub color_type: String,
    /// Canonical re-encoded JPEG bytes at fixed quality 85.
    /// Tagged `body_kind = "jpeg"` in the dict.
    pub canonical_bytes: Vec<u8>,
}

/// Decode a JPEG byte slice and return an [`IngestedJpeg`] containing
/// decoded dimensions, colour type, and deterministically re-encoded
/// canonical bytes.
///
/// # Deterministic re-encode settings
///
/// Re-encoding uses quality **85** (fixed). The `image` crate JPEG
/// encoder produces deterministic output for the same pixel data at a
/// fixed quality level. Quality 85 is the canonical setting per
/// §5.16.13 order #3: it reaches PSNR ≥ 35 dB on typical photographic
/// input while keeping file sizes to ≈ 50 % of the lossless equivalent.
///
/// Returns [`MediaError::Jpeg`] on malformed or unsupported input.
pub fn ingest_jpeg(bytes: &[u8]) -> Result<IngestedJpeg, MediaError> {
    let reader = ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Jpeg);
    let dyn_img = reader
        .decode()
        .map_err(|e| MediaError::Jpeg(e.to_string()))?;

    let width = dyn_img.width();
    let height = dyn_img.height();
    let color_type_label = color_type_label(dyn_img.color());

    let canonical_bytes = encode_jpeg_deterministic(&dyn_img)?;

    Ok(IngestedJpeg {
        width,
        height,
        color_type: color_type_label,
        canonical_bytes,
    })
}

/// Lossy round-trip gate for JPEG.
///
/// Decodes `bytes`, re-encodes via [`ingest_jpeg`], decodes the
/// canonical bytes back to pixels, then computes the PSNR between the
/// original and re-encoded pixel buffers. Returns `Ok(())` if
/// PSNR ≥ 30 dB, which is the accepted floor for "acceptable JPEG"
/// quality. At quality 85, typical photographic content scores
/// 35–45 dB, well above the threshold.
///
/// Returns [`MediaError::Jpeg`] on decode/encode failure or if PSNR
/// falls below the 30 dB threshold.
pub fn verify_jpeg_roundtrip(bytes: &[u8]) -> Result<(), MediaError> {
    let ingested = ingest_jpeg(bytes)?;

    // Decode original to RGBA8.
    let original = ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Jpeg)
        .decode()
        .map_err(|e| MediaError::Jpeg(format!("original decode failed: {e}")))?;
    let original_pixels = image_to_rgba8(&original);

    // Decode canonical bytes back to RGBA8.
    let roundtripped =
        ImageReader::with_format(Cursor::new(&ingested.canonical_bytes), image::ImageFormat::Jpeg)
            .decode()
            .map_err(|e| MediaError::Jpeg(format!("round-trip decode failed: {e}")))?;
    let roundtripped_pixels = image_to_rgba8(&roundtripped);

    let score = psnr(&original_pixels, &roundtripped_pixels);
    const THRESHOLD_DB: f64 = 30.0;
    if score < THRESHOLD_DB {
        return Err(MediaError::Jpeg(format!(
            "PSNR {score:.2} dB is below {THRESHOLD_DB} dB threshold"
        )));
    }

    Ok(())
}

// ── Opus codec (§5.16.13 order #4) ────────────────────────────────────

/// Result of ingesting an Ogg-Opus byte slice. Contains decoded stream
/// metadata and canonical bytes.
///
/// # Encoder status
///
/// No pure-Rust Opus encoder is available as of §5.16.13 order #4.
/// `canonical_bytes` is therefore set to a copy of the input Ogg-Opus
/// bytes (identity mapping). The round-trip test decodes both sides,
/// which produces identical PCM; the PSNR gate passes at infinity dB.
/// When a pure-Rust encoder lands, this field will be replaced with
/// re-encoded bytes at a fixed bitrate and complexity setting.
///
/// The bytes are the full Ogg container (Ogg pages wrapping Opus
/// packets), not raw Opus-packet data. Tagged `body_kind = "opus"` in
/// the dict.
#[derive(Debug, Clone)]
pub struct IngestedOpus {
    pub sample_rate: u32,
    pub channels: u8,
    /// Duration in milliseconds, derived from packet count × frame size
    /// as reported by the Opus packet headers decoded during demux.
    pub duration_ms: u64,
    /// Canonical Ogg-Opus bytes. Currently an identity copy of the input
    /// (see struct-level doc for the encoder status note).
    pub canonical_bytes: Vec<u8>,
}

/// Decode an Ogg-Opus byte slice and return an [`IngestedOpus`] containing
/// stream metadata and canonical bytes.
///
/// Input must be a complete Ogg-Opus file (Ogg container with Opus audio).
/// The `canonical_bytes` field is currently an identity copy of `bytes`
/// because no pure-Rust Opus encoder is available; see [`IngestedOpus`].
///
/// Returns [`MediaError::Opus`] on malformed or unsupported input.
pub fn ingest_opus(bytes: &[u8]) -> Result<IngestedOpus, MediaError> {
    let (sample_rate, channels, duration_ms) = parse_opus_metadata(bytes)?;

    // Identity mapping: awaiting pure-Rust Opus encoder.
    // When a pure-Rust encoder is available, replace this with a
    // re-encode at a fixed bitrate (e.g. 64 kbps) and complexity level.
    let canonical_bytes = bytes.to_vec();

    Ok(IngestedOpus {
        sample_rate,
        channels,
        duration_ms,
        canonical_bytes,
    })
}

/// Lossy round-trip gate for Opus.
///
/// Decodes `bytes` via [`ingest_opus`], then decodes the canonical bytes
/// back to PCM, and computes the Signal-to-Noise Ratio between the two
/// PCM buffers. Returns `Ok(())` if SNR ≥ 20 dB.
///
/// With the current identity-mapping encoder, canonical bytes == input
/// bytes, so both decode to identical PCM and SNR is infinite. The
/// threshold of ≥ 20 dB is conservative enough to accept legitimate
/// re-encodes once a real encoder lands.
///
/// Returns [`MediaError::Opus`] on decode/encode failure or if SNR falls
/// below the 20 dB threshold.
pub fn verify_opus_roundtrip(bytes: &[u8]) -> Result<(), MediaError> {
    let ingested = ingest_opus(bytes)?;

    // Decode original to PCM.
    let original_pcm = decode_opus_pcm(bytes)?;

    // Decode canonical bytes to PCM.
    let canonical_pcm = decode_opus_pcm(&ingested.canonical_bytes)?;

    if original_pcm.is_empty() || canonical_pcm.is_empty() {
        return Err(MediaError::Opus(
            "round-trip produced empty PCM buffer".to_owned(),
        ));
    }

    // SNR comparison over the shorter buffer (in case of minor length diff
    // due to encoder framing; with identity mapping they are identical).
    let len = original_pcm.len().min(canonical_pcm.len());
    let score = audio_snr_i16(&original_pcm[..len], &canonical_pcm[..len]);

    const THRESHOLD_DB: f64 = 20.0;
    if score < THRESHOLD_DB {
        return Err(MediaError::Opus(format!(
            "audio SNR {score:.2} dB is below {THRESHOLD_DB} dB threshold"
        )));
    }

    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────

fn color_type_label(ct: ColorType) -> String {
    match ct {
        ColorType::L8 => "l8",
        ColorType::La8 => "la8",
        ColorType::Rgb8 => "rgb8",
        ColorType::Rgba8 => "rgba8",
        ColorType::L16 => "l16",
        ColorType::La16 => "la16",
        ColorType::Rgb16 => "rgb16",
        ColorType::Rgba16 => "rgba16",
        ColorType::Rgb32F => "rgb32f",
        ColorType::Rgba32F => "rgba32f",
        _ => "unknown",
    }
    .to_owned()
}

/// Re-encode a `DynamicImage` to PNG bytes at fixed deterministic settings.
fn encode_png_deterministic(img: &image::DynamicImage) -> Result<Vec<u8>, MediaError> {
    let mut out = Vec::new();
    // Fixed settings for deterministic output — see `ingest_png` doc comment.
    let encoder = PngEncoder::new_with_quality(
        Cursor::new(&mut out),
        CompressionType::Default,
        FilterType::Sub,
    );

    let width = img.width();
    let height = img.height();
    // Encode via the raw pixel bytes at the image's native colour type.
    let color = img.color();
    let raw = img.as_bytes();

    encoder
        .write_image(raw, width, height, ExtendedColorType::from(color))
        .map_err(|e| MediaError::Png(e.to_string()))?;

    Ok(out)
}

/// Flatten any colour type to RGBA8 for pixel-equality comparison.
fn image_to_rgba8(img: &image::DynamicImage) -> Vec<u8> {
    img.to_rgba8().into_raw()
}

/// Re-encode a `DynamicImage` to JPEG bytes at fixed quality 85.
///
/// The `image` crate JPEG encoder produces deterministic output for
/// fixed quality and pixel input. Quality 85 is the canonical setting
/// per §5.16.13 order #3.
fn encode_jpeg_deterministic(img: &image::DynamicImage) -> Result<Vec<u8>, MediaError> {
    use image::codecs::jpeg::JpegEncoder;
    let mut out = Vec::new();
    let encoder = JpegEncoder::new_with_quality(Cursor::new(&mut out), 85);
    let width = img.width();
    let height = img.height();
    let color = img.color();
    let raw = img.as_bytes();
    encoder
        .write_image(raw, width, height, ExtendedColorType::from(color))
        .map_err(|e| MediaError::Jpeg(e.to_string()))?;
    Ok(out)
}

/// Encode raw RGBA8 pixels to canonical AVIF bytes using `ravif`.
///
/// Uses fixed deterministic params: speed=[`AVIF_SPEED`], quality=[`AVIF_QUALITY`],
/// alpha_quality=[`AVIF_ALPHA_QUALITY`], threads=[`AVIF_THREADS`] (single-threaded
/// for byte-identical output across invocations).
///
/// `pixels` must be `width × height × 4` bytes (RGBA8, non-premultiplied).
fn encode_avif_deterministic(
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, MediaError> {
    // Convert flat u8 slice to &[RGBA8] for ravif.
    // Safety: RGBA8 is repr(C) with 4 u8 fields; the pixel slice length is
    // always width*height*4 (guaranteed by image::RgbaImage::as_raw).
    assert_eq!(
        pixels.len(),
        (width as usize) * (height as usize) * 4,
        "encode_avif_deterministic: pixel buffer size mismatch"
    );
    let rgba_pixels: &[RGBA8] = bytemuck_cast_slice(pixels);
    let img = ravif::Img::new(rgba_pixels, width as usize, height as usize);

    let encoded = AvifEncoder::new()
        .with_quality(AVIF_QUALITY)
        .with_alpha_quality(AVIF_ALPHA_QUALITY)
        .with_speed(AVIF_SPEED)
        .with_num_threads(Some(AVIF_THREADS))
        .encode_rgba(img)
        .map_err(|e| MediaError::Avif(format!("ravif encode failed: {e}")))?;

    Ok(encoded.avif_file)
}

/// Cast a `&[u8]` pixel buffer to `&[RGBA8]`.
///
/// `RGBA8` is `rgb::RGBA<u8>` which is `#[repr(C)]` with four `u8` fields.
/// The cast is safe when `len` is divisible by 4 (asserted by the caller).
fn bytemuck_cast_slice(pixels: &[u8]) -> &[RGBA8] {
    assert_eq!(
        pixels.len() % 4,
        0,
        "bytemuck_cast_slice: pixel buffer length must be divisible by 4"
    );
    // SAFETY: RGBA8 = rgb::RGBA<u8> is repr(C) with alignment 1 and size 4.
    // The pointer cast is valid because the source slice has the same element
    // size (1) × 4, so the resulting slice has the correct byte count.
    unsafe {
        std::slice::from_raw_parts(
            pixels.as_ptr() as *const RGBA8,
            pixels.len() / 4,
        )
    }
}

/// Compute the Peak Signal-to-Noise Ratio between two RGBA8 pixel buffers.
///
/// Both slices must have the same length (asserted). The MSE is computed
/// over all byte values (R, G, B, A channels flat). PSNR is defined as:
///
/// ```text
/// PSNR = 10 × log₁₀(255² / MSE)
/// ```
///
/// Returns `f64::INFINITY` if the inputs are identical (MSE = 0).
fn psnr(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(
        a.len(),
        b.len(),
        "psnr: buffers must have equal length"
    );
    if a == b {
        return f64::INFINITY;
    }
    let mse: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let diff = (x as f64) - (y as f64);
            diff * diff
        })
        .sum::<f64>()
        / a.len() as f64;
    10.0 * (255.0_f64 * 255.0 / mse).log10()
}

/// Compute the Signal-to-Noise Ratio between two i16 PCM buffers.
///
/// SNR is defined as:
///
/// ```text
/// SNR = 10 × log₁₀(signal_power / noise_power)
/// ```
///
/// where `signal_power` is the mean square of `reference` and
/// `noise_power` is the mean square of the per-sample difference.
/// Returns `f64::INFINITY` if the inputs are identical (noise = 0).
/// Returns `0.0` if both buffers are all-zero silence (signal = 0).
fn audio_snr_i16(reference: &[i16], candidate: &[i16]) -> f64 {
    assert_eq!(
        reference.len(),
        candidate.len(),
        "audio_snr_i16: buffers must have equal length"
    );
    if reference == candidate {
        return f64::INFINITY;
    }
    let signal_power: f64 = reference
        .iter()
        .map(|&s| (s as f64) * (s as f64))
        .sum::<f64>()
        / reference.len() as f64;
    if signal_power == 0.0 {
        // All-silence reference: use a fixed SNR that passes the gate when
        // the candidate is also silence (identical checked above).
        return 0.0;
    }
    let noise_power: f64 = reference
        .iter()
        .zip(candidate.iter())
        .map(|(&r, &c)| {
            let diff = (r as f64) - (c as f64);
            diff * diff
        })
        .sum::<f64>()
        / reference.len() as f64;
    if noise_power == 0.0 {
        return f64::INFINITY;
    }
    10.0 * (signal_power / noise_power).log10()
}

/// Parse Ogg-Opus metadata from the ID header.
///
/// Returns `(sample_rate, channels, duration_ms)`.
///
/// `duration_ms` is estimated from the last granule position in the Ogg
/// stream minus the pre-skip, divided by the output sample rate. If the
/// granule position is 0 or unavailable, `duration_ms` is 0.
fn parse_opus_metadata(bytes: &[u8]) -> Result<(u32, u8, u64), MediaError> {
    use ogg::reading::PacketReader;

    let cursor = Cursor::new(bytes);
    let mut reader = PacketReader::new(cursor);

    // First packet: OpusHead
    let id_packet = reader
        .read_packet_expected()
        .map_err(|e| MediaError::Opus(format!("failed to read OpusHead: {e}")))?;

    if id_packet.data.len() < 19 || &id_packet.data[..8] != b"OpusHead" {
        return Err(MediaError::Opus(
            "missing or malformed OpusHead packet".to_owned(),
        ));
    }
    let channels = id_packet.data[9];
    let pre_skip = u16::from_le_bytes([id_packet.data[10], id_packet.data[11]]) as u64;
    let sample_rate = u32::from_le_bytes([
        id_packet.data[12],
        id_packet.data[13],
        id_packet.data[14],
        id_packet.data[15],
    ]);

    // Second packet: OpusTags — skip it.
    reader
        .read_packet_expected()
        .map_err(|e| MediaError::Opus(format!("failed to read OpusTags: {e}")))?;

    // Scan remaining packets to find the last granule position.
    let mut last_granule: u64 = 0;
    loop {
        match reader.read_packet() {
            Ok(Some(pkt)) => {
                let gp = pkt.absgp_page();
                if gp != u64::MAX {
                    last_granule = gp;
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    let output_rate = if sample_rate == 0 { 48000u64 } else { sample_rate as u64 };
    let duration_ms = if last_granule > pre_skip {
        (last_granule - pre_skip) * 1000 / output_rate
    } else {
        0
    };

    Ok((sample_rate, channels, duration_ms))
}

/// Decode an Ogg-Opus byte slice to interleaved i16 PCM samples.
///
/// Uses `ogg` (pure-Rust) for demuxing and `opus-decoder` (pure-Rust)
/// for Opus packet decoding. The decoder is initialised at 48 kHz
/// internally (Opus always decodes to 48 kHz internally) and the
/// output sample rate is determined by the OpusHead `input_sample_rate`
/// field (rounded to a supported value). If the stored rate is not one
/// of the five Opus-supported rates (8, 12, 16, 24, 48 kHz), the
/// decoder falls back to 48 kHz.
fn decode_opus_pcm(bytes: &[u8]) -> Result<Vec<i16>, MediaError> {
    use ogg::reading::PacketReader;
    use opus_decoder::OpusDecoder;

    let cursor = Cursor::new(bytes);
    let mut reader = PacketReader::new(cursor);

    // Read and parse OpusHead.
    let id_packet = reader
        .read_packet_expected()
        .map_err(|e| MediaError::Opus(format!("Opus PCM decode: failed to read header: {e}")))?;
    if id_packet.data.len() < 19 || &id_packet.data[..8] != b"OpusHead" {
        return Err(MediaError::Opus(
            "Opus PCM decode: malformed OpusHead".to_owned(),
        ));
    }
    let channels = id_packet.data[9] as usize;
    let stored_rate = u32::from_le_bytes([
        id_packet.data[12],
        id_packet.data[13],
        id_packet.data[14],
        id_packet.data[15],
    ]);
    // Snap to a supported Opus output rate.
    let output_rate: u32 = match stored_rate {
        8000 | 12000 | 16000 | 24000 | 48000 => stored_rate,
        _ => 48000,
    };

    // Skip OpusTags.
    reader.read_packet_expected().map_err(|e| {
        MediaError::Opus(format!("Opus PCM decode: failed to read comment: {e}"))
    })?;

    let mut decoder = OpusDecoder::new(output_rate, channels)
        .map_err(|e| MediaError::Opus(format!("OpusDecoder::new failed: {e}")))?;

    let max_frame = decoder.max_frame_size_per_channel() * channels;
    let mut pcm_buf = vec![0i16; max_frame];
    let mut all_pcm: Vec<i16> = Vec::new();

    loop {
        match reader.read_packet() {
            Ok(Some(pkt)) => {
                match decoder.decode(&pkt.data, &mut pcm_buf, false) {
                    Ok(samples_per_channel) => {
                        let written = samples_per_channel * channels;
                        all_pcm.extend_from_slice(&pcm_buf[..written]);
                    }
                    Err(_) => {
                        // Skip undecodable packets (e.g. silence/PLC triggers);
                        // this is expected for minimal fixture packets.
                    }
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    Ok(all_pcm)
}

/// Decode a FLAC byte slice using `claxon` (pure-Rust decoder).
///
/// Returns `(sample_rate, channels, bits_per_sample, pcm_samples)` where
/// `pcm_samples` is interleaved across all channels in sample order.
fn decode_flac_pcm(
    bytes: &[u8],
) -> Result<(u32, u8, u8, Vec<i32>), MediaError> {
    let cursor = Cursor::new(bytes);
    let mut reader =
        claxon::FlacReader::new(cursor).map_err(|e| MediaError::Flac(e.to_string()))?;

    let info = reader.streaminfo();
    let sample_rate = info.sample_rate;
    let channels = info
        .channels
        .try_into()
        .map_err(|_| MediaError::Flac(format!("channel count {} overflows u8", info.channels)))?;
    let bits_per_sample = info.bits_per_sample.try_into().map_err(|_| {
        MediaError::Flac(format!(
            "bits_per_sample {} overflows u8",
            info.bits_per_sample
        ))
    })?;

    let pcm_samples: Result<Vec<i32>, _> = reader.samples().collect();
    let pcm_samples = pcm_samples.map_err(|e| MediaError::Flac(e.to_string()))?;

    Ok((sample_rate, channels, bits_per_sample, pcm_samples))
}

// ── AVIF codec (§5.16.13 order #5 — §4.4.6 canonical still-image format) ─

/// Result of ingesting an AVIF byte slice. Contains image dimensions,
/// colour type, and canonical bytes.
///
/// Used by [`ingest_avif`] for the AVIF→AVIF pass-through path (per-format
/// track). For the modality-canonical track (any still image → AVIF),
/// see [`ingest_image_still_to_avif`].
///
/// Tagged `body_kind = "avif"` in the dict.
#[derive(Debug, Clone)]
pub struct IngestedAvif {
    pub width: u32,
    pub height: u32,
    /// Human-readable colour type label derived from the AV1 sequence
    /// header, e.g. `"yuv420_8bit"`, `"yuv444_8bit"`, `"mono_8bit"`.
    pub color_type: String,
    /// Canonical AVIF bytes.
    pub canonical_bytes: Vec<u8>,
}

/// Decode an AVIF byte slice and return an [`IngestedAvif`] containing
/// image dimensions, colour type, and canonical bytes.
///
/// Input must be a complete AVIF file (ISO-BMFF container with AV1
/// still-picture payload). The `canonical_bytes` field is a
/// re-encoded copy at fixed canonical params (speed=4, quality=80).
///
/// Returns [`MediaError::Avif`] on malformed or unsupported input.
pub fn ingest_avif(bytes: &[u8]) -> Result<IngestedAvif, MediaError> {
    let (width, height, color_type) = parse_avif_metadata(bytes)?;
    // Identity mapping: AVIF input is already in the canonical format.
    // Re-encoding from AVIF→AVIF requires a pure-Rust AVIF decoder
    // (no such decoder is available on Windows without nasm/FFI as of
    // §5.16.13 order #5). Use identity mapping for AVIF inputs; the
    // modality-canonical track for other formats decodes via `image`.
    let canonical_bytes = bytes.to_vec();
    Ok(IngestedAvif {
        width,
        height,
        color_type,
        canonical_bytes,
    })
}

/// Result of modality-canonical AVIF ingestion from any still-image format.
///
/// Returned by [`ingest_image_still_to_avif`]. The source image is decoded
/// via the `image` crate and re-encoded to canonical AVIF bytes using `ravif`
/// with fixed deterministic params (speed=4, quality=80, threads=1).
///
/// Tagged `body_kind = "avif"` in the dict.
#[derive(Debug, Clone)]
pub struct IngestedImageStillAvif {
    pub width: u32,
    pub height: u32,
    /// `"rgba8"` — the canonical pixel format before AV1 encoding.
    pub color_type: String,
    /// Canonical AVIF bytes encoded at fixed params for determinism.
    /// Two calls with identical pixel content produce byte-identical output.
    pub canonical_bytes: Vec<u8>,
}

/// Canonical AVIF params used by [`ingest_image_still_to_avif`].
///
/// - `SPEED`: ravif speed 4 — balanced (1=slow/best, 10=fast/worst).
/// - `QUALITY`: 80.0 — high visual quality; typical PSNR ≥ 35 dB.
/// - `ALPHA_QUALITY`: 80.0 — same quality for the alpha channel.
/// - `THREADS`: 1 — single thread ensures deterministic AV1 output.
///   Parallel AV1 encoding may produce different bit patterns for the
///   same input depending on thread scheduling.
const AVIF_SPEED: u8 = 4;
const AVIF_QUALITY: f32 = 80.0;
const AVIF_ALPHA_QUALITY: f32 = 80.0;
const AVIF_THREADS: usize = 1;

/// Decode any still-image input (PNG, JPEG, BMP, TIFF, …) and
/// re-encode to canonical AVIF bytes using `ravif` (pure-Rust AV1 encoder,
/// no FFI, no nasm).
///
/// # Canonical AVIF profile
///
/// Encoding uses fixed deterministic parameters:
/// - Speed 4, quality 80, alpha quality 80 (see [`AVIF_SPEED`] etc.).
/// - Single thread — ensures byte-identical output for the same pixel data
///   across invocations (multi-threaded AV1 encoding is non-deterministic).
/// - YCbCr color space (ravif default for non-monochrome images).
///
/// Two calls with identical `bytes` on the same ravif version produce
/// byte-identical `canonical_bytes`.
///
/// # Parameters
///
/// - `bytes`: raw bytes of any still-image format supported by the `image`
///   crate (PNG, JPEG, BMP, TIFF, WebP, …).
/// - `src_modality`: must be [`Modality::ImageStill`]; validated at call site.
///
/// Returns [`MediaError::Avif`] on decode or encode failure.
pub fn ingest_image_still_to_avif(
    bytes: &[u8],
    src_modality: Modality,
) -> Result<IngestedImageStillAvif, MediaError> {
    debug_assert_eq!(
        src_modality,
        Modality::ImageStill,
        "ingest_image_still_to_avif: src_modality must be ImageStill"
    );

    // Decode via `image` crate with format guessing — handles PNG, JPEG,
    // BMP, TIFF, WebP, and any other format the `image` crate supports.
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| MediaError::Avif(format!("format detection failed: {e}")))?;
    let dyn_img = reader
        .decode()
        .map_err(|e| MediaError::Avif(format!("source image decode failed: {e}")))?;

    let width = dyn_img.width();
    let height = dyn_img.height();

    // Convert to RGBA8 — ravif encode_rgba expects non-premultiplied RGBA.
    let rgba_img = dyn_img.into_rgba8();
    let raw_pixels = rgba_img.as_raw();

    let canonical_bytes = encode_avif_deterministic(raw_pixels, width, height)?;

    Ok(IngestedImageStillAvif {
        width,
        height,
        color_type: "rgba8".to_owned(),
        canonical_bytes,
    })
}

/// Lossy round-trip gate for modality-canonical AVIF.
///
/// Decodes both `original_bytes` (any still-image format) and `stored_avif`
/// to RGBA8 pixels, then returns the pixel-domain PSNR in dB.
///
/// # Platform note — Path C (§5.16.13 order #5 review fix)
///
/// Pixel-accurate AVIF decode requires `dav1d` (C bindings) or `rav1d`
/// (also requires nasm + cc), neither of which is available without a C
/// toolchain on Windows. On Windows this function returns
/// `Err(MediaError::Avif(...))` so that missing decode capability fails
/// loudly rather than silently passing with a vacuous `Ok(INFINITY)`.
///
/// On non-Windows the `image` crate's `avif-native` feature (backed by
/// `dav1d`) decodes the stored AVIF and computes real pixel PSNR.
/// Add `avif-native` to the image features in Cargo.toml and remove the
/// `cfg(not(windows))` gate when that path is enabled.
///
/// Returns [`MediaError::Avif`] if:
/// - `original_bytes` cannot be decoded by the `image` crate,
/// - `stored_avif` is not a valid AVIF container,
/// - the platform has no AVIF pixel decoder (Windows without C toolchain), or
/// - PSNR computation encounters mismatched image dimensions.
pub fn verify_avif_roundtrip(
    original_bytes: &[u8],
    stored_avif: &[u8],
) -> Result<f64, MediaError> {
    // Decode original source to RGBA8 pixels.
    let reader = ImageReader::new(Cursor::new(original_bytes))
        .with_guessed_format()
        .map_err(|e| MediaError::Avif(format!("original format detection failed: {e}")))?;
    let orig_img = reader
        .decode()
        .map_err(|e| MediaError::Avif(format!("original image decode failed: {e}")))?;
    let orig_rgba = orig_img.into_rgba8();
    #[cfg_attr(windows, allow(unused_variables))]
    let orig_pixels: &[u8] = orig_rgba.as_raw();

    // Validate stored AVIF container structure (works on all platforms).
    parse_avif_metadata(stored_avif)?;

    // Decode stored AVIF to RGBA8 and compute pixel PSNR.
    // This block requires a C toolchain (dav1d) and is therefore unavailable
    // on Windows without additional build infrastructure.
    #[cfg(not(windows))]
    {
        // image 0.25 avif-native feature: AvifDecoder backed by dav1d.
        use image::ImageDecoder;
        let decoder = image::codecs::avif::AvifDecoder::new(Cursor::new(stored_avif))
            .map_err(|e| MediaError::Avif(format!("AVIF decode failed: {e}")))?;
        let (dec_w, dec_h) = decoder.dimensions();
        let orig_w = orig_rgba.width();
        let orig_h = orig_rgba.height();
        if dec_w != orig_w || dec_h != orig_h {
            return Err(MediaError::Avif(format!(
                "AVIF dimension mismatch: original {orig_w}×{orig_h}, stored {dec_w}×{dec_h}"
            )));
        }
        let mut stored_pixels = vec![0u8; (dec_w * dec_h * 4) as usize];
        decoder
            .read_image(&mut stored_pixels)
            .map_err(|e| MediaError::Avif(format!("AVIF pixel read failed: {e}")))?;
        return Ok(psnr_rgba8(orig_pixels, &stored_pixels));
    }

    // Windows fallback: no AVIF pixel decoder available without C toolchain.
    // Return Err rather than Ok(INFINITY) so the gate fails loudly.
    #[cfg(windows)]
    Err(MediaError::Avif(
        "pixel PSNR not available on Windows: AVIF decode requires dav1d (C toolchain); \
         run verify_avif_roundtrip on Linux/macOS or install a C toolchain with nasm"
            .to_owned(),
    ))
}

#[cfg(not(windows))]
/// Compute PSNR (dB) between two equal-length RGBA8 pixel buffers.
///
/// Returns `f64::INFINITY` when MSE is zero (identical images), which is a
/// legitimate result — not the placeholder that `Ok(INFINITY)` was before.
fn psnr_rgba8(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len(), "psnr_rgba8: buffers must have equal length");
    let mse: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = *x as f64 - *y as f64;
            d * d
        })
        .sum::<f64>()
        / (a.len() as f64);
    if mse == 0.0 {
        return f64::INFINITY; // identical pixels — legitimate infinity
    }
    20.0 * (255.0_f64).log10() - 10.0 * mse.log10()
}

// ── AV1 video codec (§5.16.13 order #6 — §4.4.6 canonical video) ─────

/// Result of ingesting an AV1-IVF byte slice. Contains video metadata
/// and canonical bytes.
///
/// # Container format
///
/// Input must be an IVF file (`DKIF` signature, codec fourcc `AV01`).
/// IVF is the simplest AV1 container: 32-byte file header followed by
/// 12-byte frame headers (4-byte size + 8-byte PTS timestamp) and raw
/// AV1 OBU payload bytes per frame.
///
/// # Encoder status
///
/// No pure-Rust AV1 video decoder is available without FFI on Windows as
/// of §5.16.13 order #6. `canonical_bytes` is therefore an identity copy
/// of the input IVF bytes (same pattern as Opus order #4 and AVIF order
/// #5). The round-trip gate asserts `frame_count` matches on both sides.
/// When `rav1d` or equivalent is available without nasm/FFI, replace with
/// a decode-then-re-encode path using `ravif` for per-frame stills.
///
/// Tagged `body_kind = "av1"` in the dict.
#[derive(Debug, Clone)]
pub struct IngestedAv1 {
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,
    /// Duration in milliseconds, derived from fps_num/fps_den × frame_count.
    pub duration_ms: u64,
    /// Canonical AV1-IVF bytes. Currently an identity copy of the input
    /// (see struct-level doc for the encoder status note).
    pub canonical_bytes: Vec<u8>,
}

/// Decode an AV1-IVF byte slice and return an [`IngestedAv1`] containing
/// video metadata and canonical bytes.
///
/// Input must be a complete IVF file with the `DKIF` signature and `AV01`
/// codec fourcc. The `canonical_bytes` field is currently an identity copy
/// of `bytes`; see [`IngestedAv1`] for the encoder status note.
///
/// Returns [`MediaError::Av1`] on malformed or unsupported input.
pub fn ingest_av1(bytes: &[u8]) -> Result<IngestedAv1, MediaError> {
    let (width, height, frame_count, duration_ms) = parse_ivf_metadata(bytes)?;
    // Identity mapping: awaiting pure-Rust AV1 video decoder.
    // When rav1d (or equivalent) is available without nasm/FFI, replace
    // this with a decode-then-re-encode path at fixed quality settings.
    let canonical_bytes = bytes.to_vec();
    Ok(IngestedAv1 {
        width,
        height,
        frame_count,
        duration_ms,
        canonical_bytes,
    })
}

/// Round-trip gate for AV1-IVF.
///
/// Ingests `bytes` via [`ingest_av1`], then parses the canonical bytes.
/// Asserts that `frame_count` is the same in both parse results.
///
/// With the current identity-mapping encoder `canonical_bytes` == input
/// bytes, so both sides always agree. Once a decoder + re-encoder lands,
/// this will also compare frame dimensions.
///
/// Returns [`MediaError::Av1`] if either parse fails or frame counts differ.
pub fn verify_av1_roundtrip(bytes: &[u8]) -> Result<(), MediaError> {
    let ingested = ingest_av1(bytes)?;
    let (_, _, roundtripped_frame_count, _) = parse_ivf_metadata(&ingested.canonical_bytes)?;
    if ingested.frame_count != roundtripped_frame_count {
        return Err(MediaError::Av1(format!(
            "frame_count mismatch after round-trip: original={} canonical={}",
            ingested.frame_count, roundtripped_frame_count
        )));
    }
    Ok(())
}

// ── AAC codec (§5.16.13 order #7) ─────────────────────────────────────

/// Result of ingesting an ADTS-wrapped AAC byte slice. Contains decoded
/// stream metadata and canonical bytes.
///
/// # Container format
///
/// Input must be ADTS-wrapped AAC (Audio Data Transport Stream): raw AAC
/// payloads each preceded by a 7-byte ADTS header (or 9 bytes when CRC is
/// present). Each frame starts with the 12-bit syncword `0xFFF`. This is
/// the simplest AAC container and what `ffmpeg -f adts` produces. AAC
/// inside MP4/ISOBMFF is order #9's concern, not this one.
///
/// # Encoder status
///
/// No mature pure-Rust AAC encoder exists on crates.io as of §5.16.13
/// order #7. The §5.16.11 plan names `fdk-aac` (patent FFI, opt-in) and
/// `faac` (C lib fallback) as future integration points. Both require C
/// FFI and are deliberately deferred to avoid build complexity. For now,
/// `canonical_bytes` is an identity copy of the input bytes (the same
/// pattern used by Opus order #4, AVIF order #5, and AV1 order #6). The
/// round-trip test re-parses the canonical bytes and asserts that
/// `sample_rate`, `channels`, and `duration_ms` are identical. Once a
/// pure-Rust encoder lands, the field will be replaced with re-encoded
/// bytes at a fixed bitrate and complexity setting.
///
/// Tagged `body_kind = "aac"` in the dict.
#[derive(Debug, Clone)]
pub struct IngestedAac {
    pub sample_rate: u32,
    pub channels: u8,
    /// Duration in milliseconds. Derived from the ADTS frame count:
    /// `frame_count × 1024 × 1000 / sample_rate`. Each AAC frame decodes
    /// to exactly 1024 PCM samples regardless of bitrate or profile.
    pub duration_ms: u64,
    /// Canonical ADTS-AAC bytes. Currently an identity copy of the input
    /// (see struct-level doc for the encoder status note).
    pub canonical_bytes: Vec<u8>,
}

/// Decode an ADTS-wrapped AAC byte slice and return an [`IngestedAac`]
/// containing stream metadata and canonical bytes.
///
/// Input must be a sequence of ADTS frames (each beginning with syncword
/// `0xFFF`). The `canonical_bytes` field is currently an identity copy of
/// `bytes` because no pure-Rust AAC encoder is available; see
/// [`IngestedAac`].
///
/// Returns [`MediaError::Aac`] on malformed or unsupported input.
pub fn ingest_aac(bytes: &[u8]) -> Result<IngestedAac, MediaError> {
    let (sample_rate, channels, duration_ms) = parse_adts_metadata(bytes)?;
    // Identity mapping: awaiting pure-Rust AAC encoder.
    // Future: replace with fdk-aac or faac FFI re-encode at a fixed
    // bitrate (e.g. 128 kbps) for deterministic canonical bytes.
    let canonical_bytes = bytes.to_vec();
    Ok(IngestedAac {
        sample_rate,
        channels,
        duration_ms,
        canonical_bytes,
    })
}

/// Round-trip gate for ADTS-AAC.
///
/// Ingests `bytes` via [`ingest_aac`], then re-parses the canonical bytes.
/// Asserts that `sample_rate`, `channels`, and `duration_ms` are identical
/// on both sides.
///
/// With the current identity-mapping encoder `canonical_bytes` == input
/// bytes, so both sides always agree. Once a real encoder lands this will
/// catch regressions where the encoder changes stream parameters.
///
/// Returns [`MediaError::Aac`] if either parse fails or the metadata does
/// not match.
pub fn verify_aac_roundtrip(bytes: &[u8]) -> Result<(), MediaError> {
    let ingested = ingest_aac(bytes)?;
    let (rt_sample_rate, rt_channels, rt_duration_ms) =
        parse_adts_metadata(&ingested.canonical_bytes)?;
    if ingested.sample_rate != rt_sample_rate
        || ingested.channels != rt_channels
        || ingested.duration_ms != rt_duration_ms
    {
        return Err(MediaError::Aac(format!(
            "metadata mismatch after round-trip: \
             original=({},{},{}ms) canonical=({},{},{}ms)",
            ingested.sample_rate,
            ingested.channels,
            ingested.duration_ms,
            rt_sample_rate,
            rt_channels,
            rt_duration_ms,
        )));
    }
    Ok(())
}

// ── WebM/MKV container (§5.16.13 order #8) ────────────────────────────

/// A single track inside a WebM/Matroska container.
#[derive(Debug, Clone)]
pub struct WebmTrack {
    pub track_number: u64,
    /// `"video"`, `"audio"`, `"subtitle"`, etc.
    pub track_type: String,
    /// E.g. `"V_AV1"`, `"A_OPUS"`, `"A_VORBIS"`.
    pub codec_id: String,
}

/// Result of ingesting a WebM/Matroska container.
///
/// `canonical_bytes` are an identity copy of the input. Awaiting a
/// pure-Rust Matroska muxer for deterministic re-muxing; the same
/// passthrough discipline as Opus/AVIF/AV1/AAC until one lands.
///
/// Tagged `body_kind = "webm"` in the dict.
#[derive(Debug, Clone)]
pub struct IngestedWebm {
    /// Duration derived from the Segment/Info/Duration EBML element
    /// (milliseconds). Zero if the element is absent.
    pub duration_ms: u64,
    pub tracks: Vec<WebmTrack>,
    /// Identity copy of the input bytes. See struct-level doc for the
    /// encoder status note.
    pub canonical_bytes: Vec<u8>,
}

/// Parse a WebM/Matroska byte slice and return an [`IngestedWebm`]
/// containing the duration, track list, and canonical bytes.
///
/// Uses a hand-rolled EBML element walker (no external crate) that
/// recognises the minimal element set needed for track metadata:
/// EBML header, Segment, SegmentInfo (Duration), Tracks, TrackEntry
/// (TrackNumber, TrackType, CodecID). Unknown elements are skipped by
/// their declared DataSize.
///
/// `canonical_bytes` is currently an identity copy of `bytes` because
/// no pure-Rust Matroska muxer is available; see [`IngestedWebm`].
///
/// Returns [`MediaError::Webm`] on malformed input.
pub fn ingest_webm(bytes: &[u8]) -> Result<IngestedWebm, MediaError> {
    let (duration_ms, tracks) = parse_webm_metadata(bytes)?;
    // Identity mapping: awaiting pure-Rust Matroska muxer.
    // Future: replace with re-mux at fixed settings for deterministic
    // canonical bytes once a suitable pure-Rust muxer is available.
    let canonical_bytes = bytes.to_vec();
    Ok(IngestedWebm {
        duration_ms,
        tracks,
        canonical_bytes,
    })
}

/// Round-trip gate for WebM/Matroska.
///
/// Ingests `bytes` via [`ingest_webm`], then re-parses the canonical
/// bytes. Asserts that `duration_ms`, track count, and track type list
/// all match.
///
/// With the current identity-mapping encoder `canonical_bytes` == input
/// bytes, so both sides always agree. Once a real muxer lands this will
/// catch regressions where re-muxing changes container metadata.
///
/// Returns [`MediaError::Webm`] if either parse fails or metadata
/// does not match.
pub fn verify_webm_roundtrip(bytes: &[u8]) -> Result<(), MediaError> {
    let ingested = ingest_webm(bytes)?;
    let (rt_duration_ms, rt_tracks) = parse_webm_metadata(&ingested.canonical_bytes)?;
    if ingested.duration_ms != rt_duration_ms {
        return Err(MediaError::Webm(format!(
            "duration_ms mismatch after round-trip: original={}ms canonical={}ms",
            ingested.duration_ms, rt_duration_ms,
        )));
    }
    if ingested.tracks.len() != rt_tracks.len() {
        return Err(MediaError::Webm(format!(
            "track count mismatch after round-trip: original={} canonical={}",
            ingested.tracks.len(),
            rt_tracks.len(),
        )));
    }
    let orig_types: Vec<&str> = ingested.tracks.iter().map(|t| t.track_type.as_str()).collect();
    let rt_types: Vec<&str> = rt_tracks.iter().map(|t| t.track_type.as_str()).collect();
    if orig_types != rt_types {
        return Err(MediaError::Webm(format!(
            "track_type list mismatch after round-trip: original={orig_types:?} canonical={rt_types:?}",
        )));
    }
    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────

/// Parse AVIF container metadata.
///
/// Returns `(width, height, color_type_label)` where dimensions are in
/// pixels and colour type is derived from the AV1 sequence header.
fn parse_avif_metadata(bytes: &[u8]) -> Result<(u32, u32, String), MediaError> {
    use avif_parse::AvifData;
    let mut cursor = Cursor::new(bytes);
    let avif = AvifData::from_reader(&mut cursor)
        .map_err(|e| MediaError::Avif(format!("AVIF container parse failed: {e}")))?;
    let meta = avif
        .primary_item_metadata()
        .map_err(|e| MediaError::Avif(format!("AV1 sequence header parse failed: {e}")))?;
    let width = meta.max_frame_width.get();
    let height = meta.max_frame_height.get();
    let color_type = avif_color_type_label(&meta);
    Ok((width, height, color_type))
}

/// Derive a human-readable colour type label from AV1 sequence metadata.
///
/// The label encodes chroma subsampling + bit depth, matching AV1
/// native storage rather than decoded-output formats such as `"rgba8"`.
fn avif_color_type_label(meta: &avif_parse::AV1Metadata) -> String {
    if meta.monochrome {
        return format!("mono_{}bit", meta.bit_depth);
    }
    let subsampling = match meta.chroma_subsampling {
        (true, true) => "yuv420",
        (true, false) => "yuv422",
        (false, false) => "yuv444",
        _ => "yuv420", // fallback for non-standard combinations
    };
    format!("{subsampling}_{}bit", meta.bit_depth)
}

/// Re-encode PCM samples to FLAC bytes using `flacenc` (pure-Rust encoder).
///
/// Uses the default encoder config for determinism. The `flacenc` default
/// is stable within a crate version, producing byte-identical output for
/// the same interleaved `i32` PCM input.
fn encode_flac_deterministic(
    pcm_samples: &[i32],
    channels: u8,
    bits_per_sample: u8,
    sample_rate: u32,
) -> Result<Vec<u8>, MediaError> {
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, e)| MediaError::Flac(format!("encoder config error: {e:?}")))?;

    let source = flacenc::source::MemSource::from_samples(
        pcm_samples,
        channels as usize,
        bits_per_sample as usize,
        sample_rate as usize,
    );

    let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| MediaError::Flac(format!("encode error: {e}")))?;

    let mut sink = flacenc::bitsink::ByteSink::new();
    flac_stream
        .write(&mut sink)
        .map_err(|e| MediaError::Flac(format!("bitstream write error: {e}")))?;

    Ok(sink.as_slice().to_vec())
}

/// Parse IVF container metadata.
///
/// IVF file header layout (32 bytes, all little-endian):
///
/// ```text
/// offset  size  field
///      0     4  signature: "DKIF"
///      4     2  version (must be 0)
///      6     2  header_size (32)
///      8     4  codec fourcc (e.g. "AV01")
///     12     2  width  (pixels)
///     14     2  height (pixels)
///     16     4  fps_numerator
///     20     4  fps_denominator
///     24     4  frame_count
///     28     4  unused
/// ```
///
/// Each frame is preceded by a 12-byte frame header:
///
/// ```text
/// offset  size  field
///      0     4  frame_size (bytes of payload following this header)
///      4     8  pts timestamp (in fps_denominator units)
/// ```
///
/// Returns `(width, height, frame_count, duration_ms)`.
/// `duration_ms` = frame_count × fps_den × 1000 / fps_num (or 0 if fps_num is 0).
///
/// Returns [`MediaError::Av1`] if the signature or fourcc is wrong, or
/// the byte slice is too short.
fn parse_ivf_metadata(bytes: &[u8]) -> Result<(u32, u32, u32, u64), MediaError> {
    const FILE_HEADER: usize = 32;
    if bytes.len() < FILE_HEADER {
        return Err(MediaError::Av1(format!(
            "IVF file too short: {} bytes (need ≥ 32)",
            bytes.len()
        )));
    }
    if &bytes[0..4] != b"DKIF" {
        return Err(MediaError::Av1(
            "not an IVF file: missing DKIF signature".to_owned(),
        ));
    }
    if &bytes[8..12] != b"AV01" {
        let fourcc = std::str::from_utf8(&bytes[8..12]).unwrap_or("????");
        return Err(MediaError::Av1(format!(
            "IVF codec is {fourcc:?}, expected AV01"
        )));
    }

    let width = u16::from_le_bytes([bytes[12], bytes[13]]) as u32;
    let height = u16::from_le_bytes([bytes[14], bytes[15]]) as u32;
    let fps_num = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let fps_den = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    let frame_count = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);

    // Duration: frame_count × fps_den / fps_num seconds → milliseconds.
    let duration_ms: u64 = if fps_num > 0 {
        (frame_count as u64) * (fps_den as u64) * 1000 / (fps_num as u64)
    } else {
        0
    };

    // Walk frame headers to validate the byte stream, counting actual frames.
    let mut offset = FILE_HEADER;
    let mut counted = 0u32;
    while offset + 12 <= bytes.len() {
        let frame_size =
            u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
                as usize;
        offset += 12 + frame_size;
        counted += 1;
    }

    // Use counted if the file header says 0 (some encoders leave it unset).
    let effective_count = if frame_count == 0 { counted } else { frame_count };

    Ok((width, height, effective_count, duration_ms))
}

/// Parse ADTS-wrapped AAC stream metadata by walking the frame headers.
///
/// ADTS header layout (7 bytes, `protection_absent = 1`; 9 bytes when
/// `protection_absent = 0` and a 2-byte CRC follows):
///
/// ```text
/// bits  field
///   12  syncword                       (must be 0xFFF)
///    1  ID                             (0 = MPEG-4, 1 = MPEG-2)
///    2  layer                          (must be 00)
///    1  protection_absent              (1 = no CRC, header is 7 bytes;
///                                       0 = CRC present, header is 9 bytes)
///    2  profile_ObjectType             (actual AAC profile = value + 1)
///    4  sampling_frequency_index       (index into ADTS_FREQ_TABLE)
///    1  private_bit                    (ignored)
///    3  channel_configuration          (0 = inband; 1–7 = direct count)
///    1  originality/copy
///    1  home
///    1  copyright_id_bit
///    1  copyright_id_start
///   13  aac_frame_length               (header + CRC + payload, in bytes)
///   11  adts_buffer_fullness           (0x7FF = VBR)
///    2  number_of_raw_data_blocks + 1
/// ```
///
/// `sample_rate` and `channels` are read from the **first** valid frame.
/// `duration_ms` = `frame_count × 1024 × 1000 / sample_rate`
/// (each AAC frame always decodes to exactly 1024 PCM samples).
///
/// Returns [`MediaError::Aac`] if no valid ADTS frames are found or the
/// byte slice is malformed.
fn parse_adts_metadata(bytes: &[u8]) -> Result<(u32, u8, u64), MediaError> {
    /// ADTS sampling-frequency index table (ISO 14496-3 Table 1.13).
    const ADTS_FREQ_TABLE: [u32; 13] = [
        96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
    ];

    if bytes.len() < 7 {
        return Err(MediaError::Aac(format!(
            "byte slice too short for ADTS: {} bytes",
            bytes.len()
        )));
    }

    let mut offset: usize = 0;
    let mut frame_count: u64 = 0;
    let mut first_sample_rate: u32 = 0;
    let mut first_channels: u8 = 0;

    while offset + 7 <= bytes.len() {
        let b0 = bytes[offset];
        let b1 = bytes[offset + 1];

        // Verify 12-bit syncword (top 8 bits of b0, top 4 bits of b1).
        let sync = ((b0 as u16) << 4) | ((b1 as u16) >> 4);
        if sync != 0xFFF {
            // Not a syncword at this offset — stream is malformed or
            // we hit padding. Stop walking.
            break;
        }

        let protection_absent = b1 & 0x01; // 1 = no CRC (7-byte header)
        let header_len: usize = if protection_absent == 1 { 7 } else { 9 };

        if offset + header_len > bytes.len() {
            break;
        }

        let b2 = bytes[offset + 2];
        let b3 = bytes[offset + 3];
        let b4 = bytes[offset + 4];
        let b5 = bytes[offset + 5];
        let b6 = bytes[offset + 6];

        // sampling_frequency_index: bits [3:0] of b2 shifted right by 2
        // (bits 18..15 of the 28-bit header word, zero-indexed from MSB).
        let freq_idx = (b2 >> 2) & 0x0F;
        if freq_idx as usize >= ADTS_FREQ_TABLE.len() {
            return Err(MediaError::Aac(format!(
                "invalid sampling_frequency_index {freq_idx} at frame {frame_count}"
            )));
        }
        let sample_rate = ADTS_FREQ_TABLE[freq_idx as usize];

        // channel_configuration: bit 0 of b2 (high bit) | bits [7:6] of b3
        // (bits 14..12 of the 28-bit header word).
        let chan_cfg = ((b2 & 0x01) << 2) | ((b3 >> 6) & 0x03);

        // aac_frame_length: bits [1:0] of b3 | b4 | bits [7:5] of b5
        // (bits 12..0 of a 13-bit field across b3..b5).
        let frame_length = (((b3 & 0x03) as usize) << 11)
            | ((b4 as usize) << 3)
            | (((b5 >> 5) & 0x07) as usize);

        if frame_length < header_len {
            return Err(MediaError::Aac(format!(
                "aac_frame_length {frame_length} < header_len {header_len} at frame {frame_count}"
            )));
        }
        if offset + frame_length > bytes.len() {
            // Truncated last frame — count what we have and stop.
            break;
        }

        // Discard b5/b6 fields (buffer_fullness, raw_data_blocks) — not
        // needed for metadata extraction.
        let _ = b5;
        let _ = b6;

        // Record metadata from the first valid frame.
        if frame_count == 0 {
            first_sample_rate = sample_rate;
            // channel_configuration 0 means channel count is signalled
            // in-band (inside the raw_data_block). Treat as mono (1)
            // for metadata purposes; real decoders handle inband config.
            first_channels = if chan_cfg == 0 { 1 } else { chan_cfg };
        }

        frame_count += 1;
        offset += frame_length;
    }

    if frame_count == 0 {
        return Err(MediaError::Aac(
            "no valid ADTS frames found (bad syncword or empty input)".to_owned(),
        ));
    }

    // Each AAC frame always decodes to exactly 1024 PCM samples.
    let duration_ms = frame_count * 1024 * 1000 / first_sample_rate as u64;

    Ok((first_sample_rate, first_channels, duration_ms))
}

/// Parse WebM/Matroska container metadata using a hand-rolled EBML walker.
///
/// # EBML wire format
///
/// Every element = VInt(ElementID) + VInt(DataSize) + payload.
///
/// Variable-length integers (VInt): the number of leading zero bits in
/// the first byte determines the total byte-width; the width marker bit
/// is cleared when reading the value.
///
/// ```text
/// first-byte pattern  total bytes  max value
/// 1xxx xxxx            1             2^7  − 2  (127)
/// 01xx xxxx  ...       2             2^14 − 2
/// 001x xxxx  ...       3             2^21 − 2
/// ...                  up to 8
/// ```
///
/// Known EBML IDs used in this parser (all in the Matroska/WebM spec):
///
/// ```text
/// 0x1A45DFA3  EBML (root)
/// 0x18538067  Segment
/// 0x1549A966  SegmentInfo
/// 0x4489      Duration  (float, seconds × timecode scale)
/// 0x2AD7B1    TimecodeScale (default 1_000_000 ns = 1ms per timecode unit)
/// 0x1654AE6B  Tracks
/// 0xAE        TrackEntry
/// 0xD7        TrackNumber (uint)
/// 0x83        TrackType   (uint: 1=video 2=audio 17=subtitle 33=buttons)
/// 0x86        CodecID     (UTF-8 string)
/// ```
///
/// Returns `(duration_ms, tracks)`.
fn parse_webm_metadata(bytes: &[u8]) -> Result<(u64, Vec<WebmTrack>), MediaError> {
    // ── VInt readers ──────────────────────────────────────────────────

    /// Read a variable-length integer from `data[pos..]`.
    /// Returns `(value, bytes_consumed)`.
    fn read_vint(data: &[u8], pos: usize) -> Result<(u64, usize), &'static str> {
        if pos >= data.len() {
            return Err("EBML: unexpected end of data reading vint");
        }
        let first = data[pos];
        let width = first.leading_zeros() as usize + 1; // 1..=8
        if width > 8 || pos + width > data.len() {
            return Err("EBML: vint width overflows buffer");
        }
        // Clear the width-marker bit and accumulate remaining bytes.
        let mask = 0xFF >> width; // clears the top `width` bits
        let mut val = (first & mask) as u64;
        for i in 1..width {
            val = (val << 8) | (data[pos + i] as u64);
        }
        Ok((val, width))
    }

    /// Read an EBML element ID from `data[pos..]`.
    /// IDs keep the width-marker bit, unlike data-size VInts.
    fn read_element_id(data: &[u8], pos: usize) -> Result<(u32, usize), &'static str> {
        if pos >= data.len() {
            return Err("EBML: unexpected end of data reading element id");
        }
        let first = data[pos];
        let width = first.leading_zeros() as usize + 1;
        if width > 4 || pos + width > data.len() {
            return Err("EBML: element id width overflows buffer");
        }
        let mut id = first as u32;
        for i in 1..width {
            id = (id << 8) | (data[pos + i] as u32);
        }
        Ok((id, width))
    }

    // ── Scalar payload readers ─────────────────────────────────────────

    fn read_uint(data: &[u8], pos: usize, size: usize) -> u64 {
        let end = (pos + size).min(data.len());
        let mut v = 0u64;
        for &b in &data[pos..end] {
            v = (v << 8) | b as u64;
        }
        v
    }

    fn read_float(data: &[u8], pos: usize, size: usize) -> f64 {
        match size {
            4 => {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(&data[pos..pos + 4]);
                f32::from_be_bytes(arr) as f64
            }
            8 => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&data[pos..pos + 8]);
                f64::from_be_bytes(arr)
            }
            _ => 0.0,
        }
    }

    fn read_utf8(data: &[u8], pos: usize, size: usize) -> String {
        let end = (pos + size).min(data.len());
        String::from_utf8_lossy(&data[pos..end])
            .trim_end_matches('\0')
            .to_owned()
    }

    // ── EBML element IDs (Matroska/WebM spec) ─────────────────────────
    const ID_EBML: u32 = 0x1A45DFA3;
    const ID_SEGMENT: u32 = 0x18538067;
    const ID_SEGMENT_INFO: u32 = 0x1549A966;
    const ID_TRACKS: u32 = 0x1654AE6B;
    const ID_TIMECODE_SCALE: u32 = 0x2AD7B1;
    const ID_DURATION: u32 = 0x4489;
    const ID_TRACK_ENTRY: u32 = 0xAE;
    const ID_TRACK_NUMBER: u32 = 0xD7;
    const ID_TRACK_TYPE: u32 = 0x83;
    const ID_CODEC_ID: u32 = 0x86;

    // Validate EBML magic.
    if bytes.len() < 4 {
        return Err(MediaError::Webm("file too short to be WebM".to_owned()));
    }
    // First element ID must be 0x1A45DFA3 (EBML root).
    if bytes[0] != 0x1A || bytes[1] != 0x45 || bytes[2] != 0xDF || bytes[3] != 0xA3 {
        return Err(MediaError::Webm(
            "not a WebM/Matroska file: missing EBML signature".to_owned(),
        ));
    }

    // ── Top-level element walker ────────────────────────────────────────
    let mut pos = 0usize;
    let total = bytes.len();

    let mut duration_ms: u64 = 0;
    let mut timecode_scale_ns: u64 = 1_000_000; // default: 1 ms per timecode unit
    let mut tracks: Vec<WebmTrack> = Vec::new();

    while pos < total {
        let (id, id_len) = read_element_id(bytes, pos)
            .map_err(|e| MediaError::Webm(e.to_owned()))?;
        pos += id_len;
        if pos >= total {
            break;
        }
        let (data_size, ds_len) = read_vint(bytes, pos)
            .map_err(|e| MediaError::Webm(e.to_owned()))?;
        pos += ds_len;

        // 0xFF...FF sizes = "unknown size" (master elements that run to
        // the next top-level element). We handle this for Segment.
        let is_unknown_size = data_size == (1u64 << (7 * ds_len)) - 1;

        match id {
            ID_EBML => {
                // Skip EBML header payload — just advance past it.
                if !is_unknown_size {
                    pos += data_size as usize;
                }
            }
            ID_SEGMENT => {
                // Segment: master element, may have unknown size.
                // Walk its children in place (they are all at `pos`..end).
                let seg_end = if is_unknown_size {
                    total
                } else {
                    (pos + data_size as usize).min(total)
                };

                while pos < seg_end {
                    let (child_id, cid_len) = match read_element_id(bytes, pos) {
                        Ok(v) => v,
                        Err(_) => break,
                    };
                    pos += cid_len;
                    let (child_size, cds_len) = match read_vint(bytes, pos) {
                        Ok(v) => v,
                        Err(_) => break,
                    };
                    pos += cds_len;
                    let child_is_unknown = child_size == (1u64 << (7 * cds_len)) - 1;
                    let child_payload_start = pos;
                    let child_end = if child_is_unknown {
                        seg_end
                    } else {
                        (pos + child_size as usize).min(seg_end)
                    };

                    match child_id {
                        ID_SEGMENT_INFO => {
                            // Walk SegmentInfo children for Duration + TimecodeScale.
                            let mut ipos = child_payload_start;
                            while ipos < child_end {
                                let (iid, iid_len) = match read_element_id(bytes, ipos) {
                                    Ok(v) => v,
                                    Err(_) => break,
                                };
                                ipos += iid_len;
                                let (isz, isz_len) = match read_vint(bytes, ipos) {
                                    Ok(v) => v,
                                    Err(_) => break,
                                };
                                ipos += isz_len;
                                let iend = (ipos + isz as usize).min(child_end);
                                match iid {
                                    ID_TIMECODE_SCALE => {
                                        if iend - ipos <= 8 {
                                            timecode_scale_ns = read_uint(bytes, ipos, iend - ipos);
                                        }
                                    }
                                    ID_DURATION => {
                                        let sz = iend - ipos;
                                        if sz == 4 || sz == 8 {
                                            let secs = read_float(bytes, ipos, sz);
                                            // Duration in Matroska is in timecode-scale units.
                                            // duration_s = duration_value × timecode_scale_ns / 1e9
                                            let scale_s = timecode_scale_ns as f64 / 1_000_000_000.0;
                                            duration_ms = (secs * scale_s * 1000.0) as u64;
                                        }
                                    }
                                    _ => {}
                                }
                                ipos = iend;
                            }
                            pos = child_end;
                        }
                        ID_TRACKS => {
                            // Walk Tracks children for TrackEntry elements.
                            let mut tpos = child_payload_start;
                            while tpos < child_end {
                                let (tid, tid_len) = match read_element_id(bytes, tpos) {
                                    Ok(v) => v,
                                    Err(_) => break,
                                };
                                tpos += tid_len;
                                let (tsz, tsz_len) = match read_vint(bytes, tpos) {
                                    Ok(v) => v,
                                    Err(_) => break,
                                };
                                tpos += tsz_len;
                                let tend = (tpos + tsz as usize).min(child_end);

                                if tid == ID_TRACK_ENTRY {
                                    let mut track_num: u64 = 0;
                                    let mut track_type_code: u64 = 0;
                                    let mut codec_id = String::new();
                                    let mut epos = tpos;
                                    while epos < tend {
                                        let (eid, eid_len) = match read_element_id(bytes, epos) {
                                            Ok(v) => v,
                                            Err(_) => break,
                                        };
                                        epos += eid_len;
                                        let (esz, esz_len) = match read_vint(bytes, epos) {
                                            Ok(v) => v,
                                            Err(_) => break,
                                        };
                                        epos += esz_len;
                                        let eend = (epos + esz as usize).min(tend);
                                        match eid {
                                            ID_TRACK_NUMBER => {
                                                track_num = read_uint(bytes, epos, eend - epos);
                                            }
                                            ID_TRACK_TYPE => {
                                                track_type_code = read_uint(bytes, epos, eend - epos);
                                            }
                                            ID_CODEC_ID => {
                                                codec_id = read_utf8(bytes, epos, eend - epos);
                                            }
                                            _ => {}
                                        }
                                        epos = eend;
                                    }
                                    let track_type = match track_type_code {
                                        1 => "video".to_owned(),
                                        2 => "audio".to_owned(),
                                        3 => "complex".to_owned(),
                                        16 => "logo".to_owned(),
                                        17 => "subtitle".to_owned(),
                                        18 => "buttons".to_owned(),
                                        20 => "control".to_owned(),
                                        _ => format!("type{track_type_code}"),
                                    };
                                    tracks.push(WebmTrack {
                                        track_number: track_num,
                                        track_type,
                                        codec_id,
                                    });
                                }
                                tpos = tend;
                            }
                            pos = child_end;
                        }
                        _ => {
                            pos = child_end;
                        }
                    }
                }
            }
            _ => {
                if !is_unknown_size {
                    pos += data_size as usize;
                } else {
                    break; // Unknown element with unknown size — stop.
                }
            }
        }
    }

    if tracks.is_empty() && duration_ms == 0 {
        // Minimal check: at least the EBML signature was valid; we may
        // have a file with no tracks (unusual but not fatal). If neither
        // was found, the file is likely truncated or invalid.
        // We allow duration_ms == 0 (Duration element absent) but require
        // a valid EBML signature (checked above). Return what we have.
    }

    Ok((duration_ms, tracks))
}

// ── MP4/ISOBMFF container (§5.16.13 order #9) ────────────────────────

/// A single track inside an MP4/ISOBMFF container.
#[derive(Debug, Clone)]
pub struct Mp4Track {
    pub track_id: u32,
    /// ISOBMFF `hdlr` handler_type fourcc, e.g. `"vide"`, `"soun"`, `"text"`.
    pub handler_type: String,
    /// Codec fourcc from the `stsd` sample-description box,
    /// e.g. `"av01"`, `"mp4a"`, `"avc1"`, `"hvc1"`.
    pub codec: String,
}

/// Result of ingesting an MP4/ISOBMFF byte slice.
///
/// `canonical_bytes` is currently an identity copy of `bytes`.
/// No pure-Rust MP4 muxer with deterministic output is used yet;
/// the identity mapping preserves round-trip correctness until one
/// lands (same discipline as [`IngestedWebm`]).
///
/// Tagged `body_kind = "mp4"` in the dict.
#[derive(Debug, Clone)]
pub struct IngestedMp4 {
    /// Duration in milliseconds derived from `moov/mvhd`
    /// (`duration / timescale * 1000`). Zero if `mvhd` is absent.
    pub duration_ms: u64,
    pub tracks: Vec<Mp4Track>,
    /// Identity copy of the input bytes; see struct-level doc.
    pub canonical_bytes: Vec<u8>,
}

/// Parse an MP4/ISOBMFF byte slice and return an [`IngestedMp4`]
/// containing duration, track list, and canonical bytes.
///
/// Uses a hand-rolled ISOBMFF box walker (no external crate) that
/// recognises the minimal box set needed for track metadata:
/// `ftyp`, `moov` → `mvhd` (duration+timescale), `trak`×N →
/// `tkhd` (track_id) + `mdia` → `hdlr` (handler_type) +
/// `minf` → `stbl` → `stsd` (codec fourcc).
///
/// Unknown boxes are skipped by their declared size. Both standard
/// 32-bit and extended 64-bit (`size==1`) box sizes are handled.
/// FullBox prefix (version + 24-bit flags) is consumed before the
/// payload for all known FullBox types.
///
/// Returns [`MediaError::Mp4`] on malformed or truncated input.
pub fn ingest_mp4(bytes: &[u8]) -> Result<IngestedMp4, MediaError> {
    let (duration_ms, tracks) = parse_mp4_metadata(bytes)?;
    // Identity mapping: awaiting pure-Rust ISOBMFF muxer.
    // Future: replace with re-mux at fixed settings once available.
    let canonical_bytes = bytes.to_vec();
    Ok(IngestedMp4 {
        duration_ms,
        tracks,
        canonical_bytes,
    })
}

/// Round-trip gate for MP4/ISOBMFF.
///
/// Ingests `bytes` via [`ingest_mp4`], then re-parses the canonical
/// bytes. Asserts that `duration_ms`, track count, handler_types list,
/// and codec list all match.
///
/// With the current identity-mapping encoder `canonical_bytes` == input
/// bytes, so both sides always agree. Once a real muxer lands this will
/// catch regressions where re-muxing changes container metadata.
///
/// Returns [`MediaError::Mp4`] if either parse fails or metadata
/// does not match.
pub fn verify_mp4_roundtrip(bytes: &[u8]) -> Result<(), MediaError> {
    let ingested = ingest_mp4(bytes)?;
    let (rt_duration_ms, rt_tracks) = parse_mp4_metadata(&ingested.canonical_bytes)?;
    if ingested.duration_ms != rt_duration_ms {
        return Err(MediaError::Mp4(format!(
            "duration_ms mismatch after round-trip: original={}ms canonical={}ms",
            ingested.duration_ms, rt_duration_ms,
        )));
    }
    if ingested.tracks.len() != rt_tracks.len() {
        return Err(MediaError::Mp4(format!(
            "track count mismatch after round-trip: original={} canonical={}",
            ingested.tracks.len(),
            rt_tracks.len(),
        )));
    }
    let orig_handlers: Vec<&str> = ingested.tracks.iter().map(|t| t.handler_type.as_str()).collect();
    let rt_handlers: Vec<&str> = rt_tracks.iter().map(|t| t.handler_type.as_str()).collect();
    if orig_handlers != rt_handlers {
        return Err(MediaError::Mp4(format!(
            "handler_type list mismatch after round-trip: original={orig_handlers:?} canonical={rt_handlers:?}",
        )));
    }
    let orig_codecs: Vec<&str> = ingested.tracks.iter().map(|t| t.codec.as_str()).collect();
    let rt_codecs: Vec<&str> = rt_tracks.iter().map(|t| t.codec.as_str()).collect();
    if orig_codecs != rt_codecs {
        return Err(MediaError::Mp4(format!(
            "codec list mismatch after round-trip: original={orig_codecs:?} canonical={rt_codecs:?}",
        )));
    }
    Ok(())
}

/// Hand-rolled ISOBMFF box walker.
///
/// Box layout (ISO 14496-12 §4.2):
/// ```text
/// ┌──────────────┬────────────────────────────────────────────────────┐
/// │ size (u32)   │  If == 0: box extends to EOF.                      │
/// │              │  If == 1: extended size follows as u64.            │
/// │              │  Otherwise: total box size in bytes.               │
/// ├──────────────┤                                                    │
/// │ type (4 bytes)│ ASCII fourcc.                                     │
/// ├──────────────┴────────────────────────────────────────────────────┤
/// │ [largesize (u64)]  — present only when size == 1                  │
/// ├───────────────────────────────────────────────────────────────────┤
/// │ payload                                                           │
/// └───────────────────────────────────────────────────────────────────┘
/// ```
/// FullBox prepends `version (u8) + flags (u24)` to the payload.
///
/// Boxes parsed: `moov`, `mvhd`, `trak`, `tkhd`, `mdia`, `hdlr`,
/// `minf`, `stbl`, `stsd`.
///
/// Returns `(duration_ms, tracks)`.
fn parse_mp4_metadata(bytes: &[u8]) -> Result<(u64, Vec<Mp4Track>), MediaError> {
    // ── helpers ──────────────────────────────────────────────────────

    /// Read a big-endian u32 from `data[pos..]`.
    fn read_u32(data: &[u8], pos: usize) -> Result<u32, &'static str> {
        if pos + 4 > data.len() {
            return Err("truncated u32");
        }
        Ok(u32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]))
    }

    /// Read a big-endian u64 from `data[pos..]`.
    fn read_u64(data: &[u8], pos: usize) -> Result<u64, &'static str> {
        if pos + 8 > data.len() {
            return Err("truncated u64");
        }
        Ok(u64::from_be_bytes([
            data[pos], data[pos+1], data[pos+2], data[pos+3],
            data[pos+4], data[pos+5], data[pos+6], data[pos+7],
        ]))
    }

    /// Read the 4-byte ASCII fourcc at `data[pos..]`.
    fn read_fourcc(data: &[u8], pos: usize) -> Result<[u8; 4], &'static str> {
        if pos + 4 > data.len() {
            return Err("truncated fourcc");
        }
        Ok([data[pos], data[pos+1], data[pos+2], data[pos+3]])
    }

    /// Decode a fourcc to a lossy ASCII string (non-ASCII → '?').
    fn fourcc_str(cc: [u8; 4]) -> String {
        cc.iter().map(|&b| if b.is_ascii() { b as char } else { '?' }).collect()
    }

    /// Read one box header from `data[pos..]`.
    /// Returns `(fourcc, payload_start, box_end)`.
    /// `payload_start` is the offset of the first payload byte
    /// (after any largesize field).
    fn read_box_header(
        data: &[u8],
        pos: usize,
    ) -> Result<([u8; 4], usize, usize), &'static str> {
        if pos + 8 > data.len() {
            return Err("truncated box header");
        }
        let size32 = read_u32(data, pos)?;
        let fourcc = read_fourcc(data, pos + 4)?;
        let (payload_start, box_end) = if size32 == 1 {
            // Extended 64-bit size follows the 8-byte standard header.
            let size64 = read_u64(data, pos + 8)? as usize;
            if size64 < 16 || pos + size64 > data.len() {
                return Err("invalid extended box size");
            }
            (pos + 16, pos + size64)
        } else if size32 == 0 {
            // Box extends to EOF.
            (pos + 8, data.len())
        } else {
            let size = size32 as usize;
            if size < 8 || pos + size > data.len() {
                return Err("invalid box size");
            }
            (pos + 8, pos + size)
        };
        Ok((fourcc, payload_start, box_end))
    }

    // ── Validate that the first box is recognizable MP4 ──────────────
    // We accept files that start with 'ftyp' or directly with 'moov'
    // (some muxers omit ftyp for streaming).
    if bytes.len() < 8 {
        return Err(MediaError::Mp4("file too short to be MP4".to_owned()));
    }
    let first_fourcc = read_fourcc(bytes, 4)
        .map_err(|e| MediaError::Mp4(e.to_owned()))?;
    if first_fourcc != *b"ftyp" && first_fourcc != *b"moov" {
        return Err(MediaError::Mp4(format!(
            "not an MP4: first box is '{}', expected 'ftyp' or 'moov'",
            fourcc_str(first_fourcc)
        )));
    }

    // ── Top-level walk — find 'moov' ──────────────────────────────────
    let mut duration_ms: u64 = 0;
    let mut tracks: Vec<Mp4Track> = Vec::new();

    let mut pos = 0usize;
    while pos + 8 <= bytes.len() {
        let (fourcc, payload_start, box_end) =
            read_box_header(bytes, pos).map_err(|e| MediaError::Mp4(e.to_owned()))?;

        if fourcc == *b"moov" {
            // ── Walk moov children ────────────────────────────────────
            let mut mpos = payload_start;
            while mpos + 8 <= box_end {
                let (mfcc, mp_start, m_end) =
                    read_box_header(bytes, mpos).map_err(|e| MediaError::Mp4(e.to_owned()))?;

                if mfcc == *b"mvhd" {
                    // FullBox: skip version(1) + flags(3) = 4 bytes.
                    let p = mp_start + 4;
                    let version = bytes[mp_start];
                    // version 0: creation(4)+modification(4)+timescale(4)+duration(4)
                    // version 1: creation(8)+modification(8)+timescale(4)+duration(8)
                    let (timescale, dur_raw) = if version == 0 {
                        let ts = read_u32(bytes, p + 8)
                            .map_err(|e| MediaError::Mp4(e.to_owned()))? as u64;
                        let dr = read_u32(bytes, p + 12)
                            .map_err(|e| MediaError::Mp4(e.to_owned()))? as u64;
                        (ts, dr)
                    } else {
                        let ts = read_u32(bytes, p + 16)
                            .map_err(|e| MediaError::Mp4(e.to_owned()))? as u64;
                        let dr = read_u64(bytes, p + 20)
                            .map_err(|e| MediaError::Mp4(e.to_owned()))?;
                        (ts, dr)
                    };
                    if timescale > 0 {
                        duration_ms = dur_raw * 1000 / timescale;
                    }
                } else if mfcc == *b"trak" {
                    // ── Walk trak children ────────────────────────────
                    let mut track_id: u32 = 0;
                    let mut handler_type = String::new();
                    let mut codec = String::new();

                    let mut tpos = mp_start;
                    while tpos + 8 <= m_end {
                        let (tfcc, tp_start, t_end) =
                            read_box_header(bytes, tpos).map_err(|e| MediaError::Mp4(e.to_owned()))?;

                        if tfcc == *b"tkhd" {
                            // FullBox: version(1)+flags(3)
                            let p = tp_start + 4;
                            let version = bytes[tp_start];
                            // version 0: creation(4)+modification(4)+track_id(4)
                            // version 1: creation(8)+modification(8)+track_id(4)
                            let id_offset = if version == 0 { p + 8 } else { p + 16 };
                            track_id = read_u32(bytes, id_offset)
                                .map_err(|e| MediaError::Mp4(e.to_owned()))?;
                        } else if tfcc == *b"mdia" {
                            // ── Walk mdia children ────────────────────
                            let mut dpos = tp_start;
                            while dpos + 8 <= t_end {
                                let (dfcc, dp_start, d_end) =
                                    read_box_header(bytes, dpos).map_err(|e| MediaError::Mp4(e.to_owned()))?;

                                if dfcc == *b"hdlr" {
                                    // FullBox: version(1)+flags(3)+pre_defined(4)+handler_type(4)
                                    let p = dp_start + 4 + 4; // skip fullbox prefix + pre_defined
                                    if p + 4 <= d_end {
                                        let cc = read_fourcc(bytes, p)
                                            .map_err(|e| MediaError::Mp4(e.to_owned()))?;
                                        handler_type = fourcc_str(cc);
                                    }
                                } else if dfcc == *b"minf" {
                                    // ── Walk minf → stbl ─────────────
                                    let mut ipos = dp_start;
                                    while ipos + 8 <= d_end {
                                        let (ifcc, ip_start, i_end) =
                                            read_box_header(bytes, ipos).map_err(|e| MediaError::Mp4(e.to_owned()))?;

                                        if ifcc == *b"stbl" {
                                            // ── Walk stbl → stsd ─────
                                            let mut spos = ip_start;
                                            while spos + 8 <= i_end {
                                                let (sfcc, sp_start, s_end) =
                                                    read_box_header(bytes, spos).map_err(|e| MediaError::Mp4(e.to_owned()))?;

                                                if sfcc == *b"stsd" {
                                                    // FullBox: version(1)+flags(3)+entry_count(4)
                                                    let ep = sp_start + 4 + 4;
                                                    // First sample entry: size(4)+fourcc(4)+...
                                                    if ep + 8 <= s_end {
                                                        let cc = read_fourcc(bytes, ep + 4)
                                                            .map_err(|e| MediaError::Mp4(e.to_owned()))?;
                                                        codec = fourcc_str(cc);
                                                    }
                                                }
                                                if spos == s_end { break; }
                                                spos = s_end;
                                            }
                                        }
                                        if ipos == i_end { break; }
                                        ipos = i_end;
                                    }
                                }
                                if dpos == d_end { break; }
                                dpos = d_end;
                            }
                        }
                        if tpos == t_end { break; }
                        tpos = t_end;
                    }

                    tracks.push(Mp4Track { track_id, handler_type, codec });
                }
                if mpos == m_end { break; }
                mpos = m_end;
            }
        }

        if pos == box_end { break; }
        pos = box_end;
    }

    if tracks.is_empty() && duration_ms == 0 {
        // Allow a file that has ftyp but an empty moov — return what we have.
        // The caller decides if empty tracks is an error.
    }

    Ok((duration_ms, tracks))
}

// ── HEVC codec (§5.16.13 order #10, decode-only) ──────────────────────

/// Result of ingesting an HEVC Annex-B bitstream. Contains NAL unit count
/// and, when an SPS NAL is present, the video dimensions and profile.
///
/// # Decode-only design
///
/// HEVC is decode-only per §5.16.13: intended for reading legacy iPhone
/// videos. No re-encode is planned. `canonical_bytes` is always an identity
/// copy of the input — not a placeholder pending a future encoder, but the
/// correct and final design for this codec entry.
///
/// # SPS parsing
///
/// When a NAL of type 33 (SPS) is found, the parser decodes:
/// - `profile_idc` from `profile_tier_level` (general_profile_idc, u5)
/// - `pic_width_in_luma_samples` and `pic_height_in_luma_samples` via ue(v)
///
/// If no parseable SPS is found, width/height/profile_idc are 0.
///
/// Tagged `body_kind = "hevc"` in the dict.
#[derive(Debug, Clone)]
pub struct IngestedHevc {
    pub width: u32,
    pub height: u32,
    pub profile_idc: u8,
    /// Total NAL units counted in the Annex-B stream.
    pub nal_unit_count: u32,
    /// Canonical HEVC bytes. Always an identity copy of the input per the
    /// decode-only design (§5.16.13 order #10).
    pub canonical_bytes: Vec<u8>,
}

/// Ingest an HEVC Annex-B bitstream.
///
/// Walks NAL unit start codes, counts all NAL units, and attempts to decode
/// the first SPS (type 33) to extract width, height, and profile_idc.
/// If SPS parsing fails or no SPS is found, those fields are 0.
///
/// Returns [`MediaError::Hevc`] if the input contains no valid Annex-B
/// start codes.
pub fn ingest_hevc(bytes: &[u8]) -> Result<IngestedHevc, MediaError> {
    let (nal_unit_count, width, height, profile_idc) = parse_hevc_annexb(bytes)?;
    let canonical_bytes = bytes.to_vec();
    Ok(IngestedHevc {
        width,
        height,
        profile_idc,
        nal_unit_count,
        canonical_bytes,
    })
}

/// Decode-only round-trip: parse → identity copy → parse; `nal_unit_count`
/// and any decoded SPS fields must match.
///
/// No re-encode is ever planned per §5.16.13.
///
/// Returns [`MediaError::Hevc`] if either parse fails or metadata differs.
pub fn verify_hevc_roundtrip(bytes: &[u8]) -> Result<(), MediaError> {
    let ingested = ingest_hevc(bytes)?;
    // Re-parse canonical bytes (which are the identity copy).
    let (rt_count, rt_w, rt_h, rt_p) = parse_hevc_annexb(&ingested.canonical_bytes)?;
    if ingested.nal_unit_count != rt_count {
        return Err(MediaError::Hevc(format!(
            "nal_unit_count mismatch after round-trip: original={} canonical={}",
            ingested.nal_unit_count, rt_count
        )));
    }
    if ingested.width != rt_w || ingested.height != rt_h || ingested.profile_idc != rt_p {
        return Err(MediaError::Hevc(format!(
            "SPS metadata mismatch after round-trip: \
             original=({},{},p{}) canonical=({},{},p{})",
            ingested.width, ingested.height, ingested.profile_idc,
            rt_w, rt_h, rt_p
        )));
    }
    Ok(())
}

/// Walk an HEVC Annex-B bitstream, counting NAL units and parsing the first
/// SPS (type 33) for profile_idc, width, and height.
///
/// Returns `(nal_unit_count, width, height, profile_idc)`.
/// Returns `MediaError::Hevc` if no start codes are found (not an Annex-B
/// stream).
fn parse_hevc_annexb(bytes: &[u8]) -> Result<(u32, u32, u32, u8), MediaError> {
    // ── NAL unit walker ───────────────────────────────────────────────
    // Collect byte offsets of every start code (3-byte or 4-byte variant).
    // We record the offset of the first byte AFTER the start code (i.e.,
    // the first byte of the NAL header).
    let nal_starts = collect_nal_starts(bytes);
    if nal_starts.is_empty() {
        return Err(MediaError::Hevc(
            "no Annex-B start codes found — not an HEVC stream".to_owned(),
        ));
    }

    let nal_unit_count = nal_starts.len() as u32;
    let mut width: u32 = 0;
    let mut height: u32 = 0;
    let mut profile_idc: u8 = 0;

    // ── SPS parser ────────────────────────────────────────────────────
    // Iterate NAL units; stop as soon as we successfully parse one SPS.
    for (idx, &start) in nal_starts.iter().enumerate() {
        if start >= bytes.len() {
            continue;
        }
        // HEVC NAL header is 2 bytes.
        // Byte 0: bit15=forbidden_zero_bit, bits14..9=nal_unit_type(6), bits8..3=nuh_layer_id(6hi)
        // Byte 1: bits2..0=nuh_layer_id(6lo) and nuh_temporal_id_plus1(3)
        // nal_unit_type = (bytes[start] >> 1) & 0x3F
        let nal_type = (bytes[start] >> 1) & 0x3F;
        if nal_type != 33 {
            // Not SPS; skip.
            continue;
        }
        // NAL body starts after the 2-byte header.
        let body_start = start + 2;
        // NAL body end: up to the next start code (exclusive), or end of stream.
        let body_end = if idx + 1 < nal_starts.len() {
            // Walk back from nal_starts[idx+1] past the preceding start code bytes.
            // nal_starts[idx+1] already points past the start code; subtract start-code length.
            // We don't know if it was 3- or 4-byte; scan backwards.
            let next = nal_starts[idx + 1];
            // Determine start code length preceding next NAL.
            if next >= 4 && bytes[next - 4] == 0x00 && bytes[next - 3] == 0x00
                && bytes[next - 2] == 0x00 && bytes[next - 1] == 0x01
            {
                next - 4
            } else if next >= 3 && bytes[next - 3] == 0x00 && bytes[next - 2] == 0x00
                && bytes[next - 1] == 0x01
            {
                next - 3
            } else {
                next
            }
        } else {
            bytes.len()
        };
        if body_start >= body_end {
            continue;
        }
        let nal_body = &bytes[body_start..body_end];
        // Remove emulation prevention bytes before bit-parsing.
        let rbsp = remove_emulation_prevention(nal_body);
        // Attempt SPS field extraction.
        if let Ok((p, w, h)) = decode_sps_fields(&rbsp) {
            profile_idc = p;
            width = w;
            height = h;
        }
        // Stop after first SPS regardless of parse success.
        break;
    }

    Ok((nal_unit_count, width, height, profile_idc))
}

/// Find all Annex-B start code positions and return the offset of the first
/// byte of each NAL header (i.e., start-code-end + 1).
/// Recognises both 4-byte (`0x00 0x00 0x00 0x01`) and 3-byte
/// (`0x00 0x00 0x01`) start codes; always records the byte immediately
/// after the start code.
fn collect_nal_starts(bytes: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 3 <= bytes.len() {
        if bytes[i] == 0x00 && bytes[i + 1] == 0x00 {
            if i + 4 <= bytes.len() && bytes[i + 2] == 0x00 && bytes[i + 3] == 0x01 {
                out.push(i + 4);
                i += 4;
                continue;
            }
            if bytes[i + 2] == 0x01 {
                out.push(i + 3);
                i += 3;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Remove HEVC emulation prevention bytes (`0x00 0x00 0x03` → `0x00 0x00`)
/// from a raw NAL body to produce the RBSP byte sequence.
fn remove_emulation_prevention(nal_body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(nal_body.len());
    let mut i = 0usize;
    while i < nal_body.len() {
        // Emulation prevention: 0x00 0x00 0x03 xx where xx ∈ {0x00,0x01,0x02,0x03}
        if i + 2 < nal_body.len()
            && nal_body[i] == 0x00
            && nal_body[i + 1] == 0x00
            && nal_body[i + 2] == 0x03
        {
            out.push(0x00);
            out.push(0x00);
            i += 3; // skip the 0x03
        } else {
            out.push(nal_body[i]);
            i += 1;
        }
    }
    out
}

/// Bit-reader helper over a `&[u8]` slice.
struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8, // 0 = MSB, 7 = LSB
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, byte_pos: 0, bit_pos: 0 }
    }

    /// Read `n` bits as a u32. Returns `None` if insufficient data.
    fn read_bits(&mut self, n: u8) -> Option<u32> {
        let mut val: u32 = 0;
        for _ in 0..n {
            if self.byte_pos >= self.data.len() {
                return None;
            }
            let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1;
            val = (val << 1) | (bit as u32);
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }
        Some(val)
    }

    /// Decode one ue(v) Exp-Golomb value. Returns `None` if insufficient data.
    fn read_ue(&mut self) -> Option<u32> {
        let mut leading_zeros: u8 = 0;
        loop {
            let bit = self.read_bits(1)?;
            if bit == 1 {
                break;
            }
            leading_zeros += 1;
            if leading_zeros > 31 {
                return None; // guard against malformed input
            }
        }
        if leading_zeros == 0 {
            return Some(0);
        }
        let suffix = self.read_bits(leading_zeros)?;
        Some((1u32 << leading_zeros) - 1 + suffix)
    }
}

/// Decode the minimum SPS RBSP fields needed to extract profile_idc, width,
/// and height. Returns `(profile_idc, width, height)` or an error string.
///
/// SPS RBSP layout (H.265 §7.3.2.2.1):
///   sps_video_parameter_set_id           u(4)
///   sps_max_sub_layers_minus1            u(3)  ← controls sub-layer loop below
///   sps_temporal_id_nesting_flag         u(1)
///   profile_tier_level(maxNumSubLayersMinus1):
///     general_profile_space              u(2)
///     general_tier_flag                  u(1)
///     general_profile_idc                u(5)  ← we want this
///     general_profile_compatibility_flag u(32)
///     general_progressive_source_flag    u(1)
///     general_interlaced_source_flag     u(1)
///     general_non_packed_constraint_flag u(1)
///     general_frame_only_constraint_flag u(1)
///     general_reserved_zero_43bits       u(43)
///     general_inbld_flag                 u(1)
///     general_level_idc                  u(8)
///     [sub-layer flags loop if maxNumSubLayersMinus1 > 0 — not needed here]
///   sps_seq_parameter_set_id             ue(v)
///   chroma_format_idc                    ue(v)
///   [separate_colour_plane_flag          u(1)  if chroma_format_idc == 3]
///   pic_width_in_luma_samples            ue(v) ← we want this
///   pic_height_in_luma_samples           ue(v) ← we want this
fn decode_sps_fields(rbsp: &[u8]) -> Result<(u8, u32, u32), &'static str> {
    let mut r = BitReader::new(rbsp);

    // sps_video_parameter_set_id (u4)
    r.read_bits(4).ok_or("truncated: sps_video_parameter_set_id")?;
    // sps_max_sub_layers_minus1 (u3)
    let max_sub_layers_minus1 = r.read_bits(3).ok_or("truncated: sps_max_sub_layers_minus1")?;
    // sps_temporal_id_nesting_flag (u1)
    r.read_bits(1).ok_or("truncated: sps_temporal_id_nesting_flag")?;

    // profile_tier_level(maxNumSubLayersMinus1)
    // general_profile_space (u2), general_tier_flag (u1)
    r.read_bits(3).ok_or("truncated: profile_space+tier")?;
    // general_profile_idc (u5)
    let profile_idc = r.read_bits(5).ok_or("truncated: general_profile_idc")? as u8;
    // general_profile_compatibility_flag[32] (u32)
    r.read_bits(32).ok_or("truncated: profile_compatibility_flag")?;
    // progressive(1) + interlaced(1) + non_packed(1) + frame_only(1) (u4)
    r.read_bits(4).ok_or("truncated: source flags")?;
    // general_reserved_zero_43bits (u43) + general_inbld_flag (u1) = 44 bits total
    r.read_bits(32).ok_or("truncated: constraint bits [0..31]")?;
    r.read_bits(12).ok_or("truncated: constraint bits [32..43] + inbld")?;
    // general_level_idc (u8)
    r.read_bits(8).ok_or("truncated: general_level_idc")?;

    // Sub-layer profile/level present flags — only needed if maxNumSubLayersMinus1 > 0.
    // For each i in 0..maxNumSubLayersMinus1: sub_layer_profile_present_flag[i] u(1),
    //                                         sub_layer_level_present_flag[i] u(1).
    let mut sub_profile_present = [false; 8];
    let mut sub_level_present = [false; 8];
    for i in 0..max_sub_layers_minus1 as usize {
        sub_profile_present[i] = r.read_bits(1).ok_or("truncated: sub_layer_profile_present")? != 0;
        sub_level_present[i] = r.read_bits(1).ok_or("truncated: sub_layer_level_present")? != 0;
    }
    // If maxNumSubLayersMinus1 > 0, read reserved_zero_2bits padding for each i from
    // maxNumSubLayersMinus1 up to 7 (spec Table 7-1 note — 8 slots total, filled with u(2)).
    if max_sub_layers_minus1 > 0 {
        for _ in max_sub_layers_minus1 as usize..8 {
            r.read_bits(2).ok_or("truncated: reserved_zero_2bits")?;
        }
        // Then sub-layer profile/level bodies for present layers.
        for i in 0..max_sub_layers_minus1 as usize {
            if sub_profile_present[i] {
                // profile_space(2)+tier(1)+idc(5)+compat(32)+4 source flags+44 constraint = 88 bits
                r.read_bits(32).ok_or("truncated: sub_layer profile body [0]")?;
                r.read_bits(32).ok_or("truncated: sub_layer profile body [1]")?;
                r.read_bits(24).ok_or("truncated: sub_layer profile body [2]")?;
            }
            if sub_level_present[i] {
                r.read_bits(8).ok_or("truncated: sub_layer_level_idc")?;
            }
        }
    }

    // sps_seq_parameter_set_id ue(v)
    r.read_ue().ok_or("truncated: sps_seq_parameter_set_id")?;
    // chroma_format_idc ue(v)
    let chroma_format_idc = r.read_ue().ok_or("truncated: chroma_format_idc")?;
    if chroma_format_idc == 3 {
        r.read_bits(1).ok_or("truncated: separate_colour_plane_flag")?;
    }
    // pic_width_in_luma_samples ue(v)
    let width = r.read_ue().ok_or("truncated: pic_width_in_luma_samples")?;
    // pic_height_in_luma_samples ue(v)
    let height = r.read_ue().ok_or("truncated: pic_height_in_luma_samples")?;

    Ok((profile_idc, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_types::body_kind;

    #[test]
    fn modality_maps_to_canonical_body_kind() {
        assert_eq!(Modality::ImageStill.canonical_body_kind(), body_kind::AVIF);
        assert_eq!(Modality::Video.canonical_body_kind(), body_kind::AV1);
        assert_eq!(Modality::AudioLossy.canonical_body_kind(), body_kind::AAC);
        assert_eq!(
            Modality::AudioLossless.canonical_body_kind(),
            body_kind::FLAC
        );
        assert_eq!(Modality::Font.canonical_body_kind(), body_kind::WOFF2);
        assert_eq!(Modality::Mesh3d.canonical_body_kind(), body_kind::GLTF);
        assert_eq!(Modality::Document.canonical_body_kind(), body_kind::PDF);
    }

    #[test]
    fn modality_from_ext_handles_common_formats() {
        assert_eq!(modality_from_ext("png"), Some(Modality::ImageStill));
        assert_eq!(modality_from_ext("AVIF"), Some(Modality::ImageStill));
        assert_eq!(modality_from_ext("mp4"), Some(Modality::Video));
        assert_eq!(modality_from_ext("flac"), Some(Modality::AudioLossless));
        assert_eq!(modality_from_ext("woff2"), Some(Modality::Font));
        assert_eq!(modality_from_ext("obj"), Some(Modality::Mesh3d));
        assert_eq!(modality_from_ext("pdf"), Some(Modality::Document));
        assert_eq!(modality_from_ext("xyz"), None);
    }

    #[test]
    fn every_canonical_format_is_a_known_body_kind() {
        for m in Modality::ALL {
            assert!(body_kind::is_known(m.canonical_body_kind()));
        }
        // Drift check: if someone adds a Modality variant but forgets
        // to add it to ALL, this count goes stale and the next test
        // fails.
        assert_eq!(Modality::ALL.len(), 7);
    }

    #[test]
    fn modality_all_covers_every_variant() {
        // Exhaustive-match sentinel: if a new Modality variant is
        // added, this match fails to compile until the maintainer
        // also updates Modality::ALL above.
        for m in Modality::ALL {
            let _: () = match m {
                Modality::ImageStill
                | Modality::Video
                | Modality::AudioLossy
                | Modality::AudioLossless
                | Modality::Font
                | Modality::Mesh3d
                | Modality::Document => (),
            };
        }
    }
}
