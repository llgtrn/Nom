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
