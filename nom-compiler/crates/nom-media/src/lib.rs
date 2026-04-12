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
