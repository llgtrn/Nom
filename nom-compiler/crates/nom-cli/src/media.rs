//! `nom media` CLI dispatch per plan §5.16.7.
//!
//! Today: `import` reads a file, dispatches to the matching
//! nom-media codec ingester, prints a summary. Future: `render`,
//! `transcode`, `diff`, `similar` per the same §5.16.7 list.
//!
//! The shared `ingest_by_extension` helper is also called by
//! `store::cmd_store_add_media` so both commands use the same codec
//! dispatch table.

use std::path::Path;

use nom_media::{
    ingest_aac, ingest_av1, ingest_avif, ingest_flac, ingest_hevc, ingest_jpeg, ingest_mp4,
    ingest_opus, ingest_png, ingest_webm, modality_from_ext,
};
use nom_types::body_kind;

// ── Shared ingest result ─────────────────────────────────────────────

/// Codec-agnostic summary returned by [`ingest_by_extension`].
///
/// Contains the pieces needed by both `nom media import` (print) and
/// `nom store add-media` (persist + print): the §4.4.6 body_kind tag,
/// the canonical bytes, and a short human-readable label for the
/// `describe` field.
pub struct IngestSummary {
    /// §4.4.6 body_kind tag, e.g. `"png"`, `"flac"`.
    pub body_kind_tag: &'static str,
    /// Canonical, deterministically re-encoded bytes.
    pub canonical_bytes: Vec<u8>,
    /// Short auto-generated label, e.g. `"png image, 4x4, rgba8"`.
    pub describe: String,
}

/// Dispatch `bytes` through the matching codec ingester keyed on `ext`
/// (lowercase file extension without the leading dot).
///
/// Returns `Ok(IngestSummary)` on success or `Err(message)` on any
/// codec error or unrecognized extension. The caller owns the error
/// string and may pass it to `eprintln!` unchanged.
pub fn ingest_by_extension(bytes: &[u8], ext: &str) -> Result<IngestSummary, String> {
    match ext {
        "png" => {
            let r = ingest_png(bytes).map_err(|e| format!("PNG ingest failed: {e}"))?;
            Ok(IngestSummary {
                body_kind_tag: body_kind::PNG,
                describe: format!(
                    "png image, {}x{}, {}",
                    r.width, r.height, r.color_type
                ),
                canonical_bytes: r.canonical_bytes,
            })
        }
        "jpg" | "jpeg" => {
            let r = ingest_jpeg(bytes).map_err(|e| format!("JPEG ingest failed: {e}"))?;
            Ok(IngestSummary {
                body_kind_tag: body_kind::JPEG,
                describe: format!(
                    "jpeg image, {}x{}, {}",
                    r.width, r.height, r.color_type
                ),
                canonical_bytes: r.canonical_bytes,
            })
        }
        "avif" => {
            let r = ingest_avif(bytes).map_err(|e| format!("AVIF ingest failed: {e}"))?;
            Ok(IngestSummary {
                body_kind_tag: body_kind::AVIF,
                describe: format!(
                    "avif image, {}x{}, {}",
                    r.width, r.height, r.color_type
                ),
                canonical_bytes: r.canonical_bytes,
            })
        }
        "flac" => {
            let r = ingest_flac(bytes).map_err(|e| format!("FLAC ingest failed: {e}"))?;
            // duration_ms = (total_samples / channels) / sample_rate * 1000
            let frames = if r.channels > 0 {
                r.total_samples / r.channels as u64
            } else {
                0
            };
            let duration_ms = if r.sample_rate > 0 {
                frames * 1000 / r.sample_rate as u64
            } else {
                0
            };
            Ok(IngestSummary {
                body_kind_tag: body_kind::FLAC,
                describe: format!(
                    "flac audio, {}Hz {}ch, {}ms",
                    r.sample_rate, r.channels, duration_ms
                ),
                canonical_bytes: r.canonical_bytes,
            })
        }
        "opus" | "ogg" => {
            let r = ingest_opus(bytes).map_err(|e| format!("Opus ingest failed: {e}"))?;
            Ok(IngestSummary {
                body_kind_tag: body_kind::OPUS,
                describe: format!(
                    "opus audio, {}Hz {}ch, {}ms",
                    r.sample_rate, r.channels, r.duration_ms
                ),
                canonical_bytes: r.canonical_bytes,
            })
        }
        "aac" => {
            let r = ingest_aac(bytes).map_err(|e| format!("AAC ingest failed: {e}"))?;
            Ok(IngestSummary {
                body_kind_tag: body_kind::AAC,
                describe: format!(
                    "aac audio, {}Hz {}ch, {}ms",
                    r.sample_rate, r.channels, r.duration_ms
                ),
                canonical_bytes: r.canonical_bytes,
            })
        }
        "ivf" | "av1" => {
            let r = ingest_av1(bytes).map_err(|e| format!("AV1 ingest failed: {e}"))?;
            Ok(IngestSummary {
                body_kind_tag: body_kind::AV1,
                describe: format!(
                    "av1 video, {}x{}, {} frames, {}ms",
                    r.width, r.height, r.frame_count, r.duration_ms
                ),
                canonical_bytes: r.canonical_bytes,
            })
        }
        "webm" => {
            let r = ingest_webm(bytes).map_err(|e| format!("WebM ingest failed: {e}"))?;
            let track_count = r.tracks.len();
            Ok(IngestSummary {
                body_kind_tag: body_kind::WEBM,
                describe: format!("webm container, {} tracks, {}ms", track_count, r.duration_ms),
                canonical_bytes: r.canonical_bytes,
            })
        }
        "mp4" | "m4a" | "mov" => {
            let r = ingest_mp4(bytes).map_err(|e| format!("MP4 ingest failed: {e}"))?;
            let track_count = r.tracks.len();
            Ok(IngestSummary {
                body_kind_tag: body_kind::MP4,
                describe: format!("mp4 container, {} tracks, {}ms", track_count, r.duration_ms),
                canonical_bytes: r.canonical_bytes,
            })
        }
        "hevc" | "h265" | "265" => {
            let r = ingest_hevc(bytes).map_err(|e| format!("HEVC ingest failed: {e}"))?;
            Ok(IngestSummary {
                body_kind_tag: body_kind::HEVC,
                describe: format!(
                    "hevc bitstream, {}x{}, {} NAL units",
                    r.width, r.height, r.nal_unit_count
                ),
                canonical_bytes: r.canonical_bytes,
            })
        }
        _ => Err(format!("no codec for extension: {ext}")),
    }
}

// ── Print helpers ────────────────────────────────────────────────────

/// Print key=value lines for human-readable mode or a flat JSON object.
fn print_kv(pairs: &[(&str, String)], json: bool) {
    if json {
        let mut parts = Vec::with_capacity(pairs.len());
        for (k, v) in pairs {
            // Determine whether the value is a bare number or needs quoting.
            let is_number = v.parse::<u64>().is_ok() || v.parse::<f64>().is_ok();
            if is_number {
                parts.push(format!("\"{}\":{}", k, v));
            } else {
                parts.push(format!("\"{}\":\"{}\"", k, v));
            }
        }
        println!("{{{}}}", parts.join(","));
    } else {
        for (k, v) in pairs {
            println!("{}: {}", k, v);
        }
    }
}

// ── cmd_media_import ─────────────────────────────────────────────────

pub fn cmd_media_import(path: &Path, json: bool) -> i32 {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("nom: cannot read {}: {e}", path.display());
            return 1;
        }
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match modality_from_ext(&ext) {
        None => {
            eprintln!("nom: no codec for extension: {ext}");
            return 1;
        }
        Some(_) => {} // modality known; dispatch by extension below
    }

    match ingest_by_extension(&bytes, &ext) {
        Ok(summary) => {
            // Re-derive the per-format metadata for the detailed print_kv output.
            // Since IngestSummary carries describe + body_kind_tag, we reconstruct
            // the rich key-value pairs by re-dispatching per format.
            print_kv_for_summary(&bytes, &ext, &summary, json);
            0
        }
        Err(e) => {
            eprintln!("nom: {e}");
            1
        }
    }
}

/// Emit the rich per-format key-value pairs that `nom media import` prints.
/// Called after a successful `ingest_by_extension` to keep the output format
/// identical to before the refactor.
fn print_kv_for_summary(bytes: &[u8], ext: &str, summary: &IngestSummary, json: bool) {
    match ext {
        "png" => {
            // Re-ingest for fields not in IngestSummary (width/height/color_type).
            if let Ok(r) = ingest_png(bytes) {
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("width", r.width.to_string()),
                        ("height", r.height.to_string()),
                        ("color_type", r.color_type.clone()),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        "jpg" | "jpeg" => {
            if let Ok(r) = ingest_jpeg(bytes) {
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("width", r.width.to_string()),
                        ("height", r.height.to_string()),
                        ("color_type", r.color_type.clone()),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        "avif" => {
            if let Ok(r) = ingest_avif(bytes) {
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("width", r.width.to_string()),
                        ("height", r.height.to_string()),
                        ("color_type", r.color_type.clone()),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        "flac" => {
            if let Ok(r) = ingest_flac(bytes) {
                let frames = if r.channels > 0 {
                    r.total_samples / r.channels as u64
                } else {
                    0
                };
                let duration_ms = if r.sample_rate > 0 {
                    frames * 1000 / r.sample_rate as u64
                } else {
                    0
                };
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("sample_rate", r.sample_rate.to_string()),
                        ("channels", r.channels.to_string()),
                        ("duration_ms", duration_ms.to_string()),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        "opus" | "ogg" => {
            if let Ok(r) = ingest_opus(bytes) {
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("sample_rate", r.sample_rate.to_string()),
                        ("channels", r.channels.to_string()),
                        ("duration_ms", r.duration_ms.to_string()),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        "aac" => {
            if let Ok(r) = ingest_aac(bytes) {
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("sample_rate", r.sample_rate.to_string()),
                        ("channels", r.channels.to_string()),
                        ("duration_ms", r.duration_ms.to_string()),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        "ivf" | "av1" => {
            if let Ok(r) = ingest_av1(bytes) {
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("width", r.width.to_string()),
                        ("height", r.height.to_string()),
                        ("frame_count", r.frame_count.to_string()),
                        ("duration_ms", r.duration_ms.to_string()),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        "webm" => {
            if let Ok(r) = ingest_webm(bytes) {
                let track_count = r.tracks.len();
                let track_summary = r
                    .tracks
                    .iter()
                    .map(|t| format!("{}:{}", t.track_type, t.codec_id))
                    .collect::<Vec<_>>()
                    .join(",");
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("duration_ms", r.duration_ms.to_string()),
                        ("track_count", track_count.to_string()),
                        ("tracks", track_summary),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        "mp4" | "m4a" | "mov" => {
            if let Ok(r) = ingest_mp4(bytes) {
                let track_count = r.tracks.len();
                let track_summary = r
                    .tracks
                    .iter()
                    .map(|t| format!("{}:{}", t.handler_type, t.codec))
                    .collect::<Vec<_>>()
                    .join(",");
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("duration_ms", r.duration_ms.to_string()),
                        ("track_count", track_count.to_string()),
                        ("tracks", track_summary),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        "hevc" | "h265" | "265" => {
            if let Ok(r) = ingest_hevc(bytes) {
                print_kv(
                    &[
                        ("format", summary.body_kind_tag.to_string()),
                        ("width", r.width.to_string()),
                        ("height", r.height.to_string()),
                        ("profile_idc", r.profile_idc.to_string()),
                        ("nal_unit_count", r.nal_unit_count.to_string()),
                        ("canonical_bytes", summary.canonical_bytes.len().to_string()),
                    ],
                    json,
                );
            }
        }
        _ => {} // already guarded by modality_from_ext check above
    }
}
