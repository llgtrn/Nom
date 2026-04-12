//! `nom media` CLI dispatch per plan §5.16.7.
//!
//! Today: `import` reads a file, dispatches to the matching
//! nom-media codec ingester, prints a summary. Future: `render`,
//! `transcode`, `diff`, `similar` per the same §5.16.7 list.

use std::path::Path;

use nom_media::{
    ingest_aac, ingest_av1, ingest_avif, ingest_flac, ingest_hevc, ingest_jpeg, ingest_mp4,
    ingest_opus, ingest_png, ingest_webm, modality_from_ext,
};
use nom_types::body_kind;

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

    // Extension-to-ingester dispatch (§5.16.7 table).
    match ext.as_str() {
        "png" => match ingest_png(&bytes) {
            Ok(r) => print_kv(
                &[
                    ("format", body_kind::PNG.to_string()),
                    ("width", r.width.to_string()),
                    ("height", r.height.to_string()),
                    ("color_type", r.color_type.clone()),
                    ("canonical_bytes", r.canonical_bytes.len().to_string()),
                ],
                json,
            ),
            Err(e) => {
                eprintln!("nom: PNG ingest failed: {e}");
                return 1;
            }
        },
        "jpg" | "jpeg" => match ingest_jpeg(&bytes) {
            Ok(r) => print_kv(
                &[
                    ("format", body_kind::JPEG.to_string()),
                    ("width", r.width.to_string()),
                    ("height", r.height.to_string()),
                    ("color_type", r.color_type.clone()),
                    ("canonical_bytes", r.canonical_bytes.len().to_string()),
                ],
                json,
            ),
            Err(e) => {
                eprintln!("nom: JPEG ingest failed: {e}");
                return 1;
            }
        },
        "avif" => match ingest_avif(&bytes) {
            Ok(r) => print_kv(
                &[
                    ("format", body_kind::AVIF.to_string()),
                    ("width", r.width.to_string()),
                    ("height", r.height.to_string()),
                    ("color_type", r.color_type.clone()),
                    ("canonical_bytes", r.canonical_bytes.len().to_string()),
                ],
                json,
            ),
            Err(e) => {
                eprintln!("nom: AVIF ingest failed: {e}");
                return 1;
            }
        },
        "flac" => match ingest_flac(&bytes) {
            Ok(r) => {
                // duration_ms = total_samples / channels / sample_rate * 1000
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
                        ("format", body_kind::FLAC.to_string()),
                        ("sample_rate", r.sample_rate.to_string()),
                        ("channels", r.channels.to_string()),
                        ("duration_ms", duration_ms.to_string()),
                        ("canonical_bytes", r.canonical_bytes.len().to_string()),
                    ],
                    json,
                )
            }
            Err(e) => {
                eprintln!("nom: FLAC ingest failed: {e}");
                return 1;
            }
        },
        "opus" | "ogg" => match ingest_opus(&bytes) {
            Ok(r) => print_kv(
                &[
                    ("format", body_kind::OPUS.to_string()),
                    ("sample_rate", r.sample_rate.to_string()),
                    ("channels", r.channels.to_string()),
                    ("duration_ms", r.duration_ms.to_string()),
                    ("canonical_bytes", r.canonical_bytes.len().to_string()),
                ],
                json,
            ),
            Err(e) => {
                eprintln!("nom: Opus ingest failed: {e}");
                return 1;
            }
        },
        "aac" => match ingest_aac(&bytes) {
            Ok(r) => print_kv(
                &[
                    ("format", body_kind::AAC.to_string()),
                    ("sample_rate", r.sample_rate.to_string()),
                    ("channels", r.channels.to_string()),
                    ("duration_ms", r.duration_ms.to_string()),
                    ("canonical_bytes", r.canonical_bytes.len().to_string()),
                ],
                json,
            ),
            Err(e) => {
                eprintln!("nom: AAC ingest failed: {e}");
                return 1;
            }
        },
        "ivf" | "av1" => match ingest_av1(&bytes) {
            Ok(r) => print_kv(
                &[
                    ("format", body_kind::AV1.to_string()),
                    ("width", r.width.to_string()),
                    ("height", r.height.to_string()),
                    ("frame_count", r.frame_count.to_string()),
                    ("duration_ms", r.duration_ms.to_string()),
                    ("canonical_bytes", r.canonical_bytes.len().to_string()),
                ],
                json,
            ),
            Err(e) => {
                eprintln!("nom: AV1 ingest failed: {e}");
                return 1;
            }
        },
        "webm" => match ingest_webm(&bytes) {
            Ok(r) => {
                let track_count = r.tracks.len();
                let track_summary = r
                    .tracks
                    .iter()
                    .map(|t| format!("{}:{}", t.track_type, t.codec_id))
                    .collect::<Vec<_>>()
                    .join(",");
                print_kv(
                    &[
                        ("format", body_kind::WEBM.to_string()),
                        ("duration_ms", r.duration_ms.to_string()),
                        ("track_count", track_count.to_string()),
                        ("tracks", track_summary),
                        ("canonical_bytes", r.canonical_bytes.len().to_string()),
                    ],
                    json,
                )
            }
            Err(e) => {
                eprintln!("nom: WebM ingest failed: {e}");
                return 1;
            }
        },
        "mp4" | "m4a" | "mov" => match ingest_mp4(&bytes) {
            Ok(r) => {
                let track_count = r.tracks.len();
                let track_summary = r
                    .tracks
                    .iter()
                    .map(|t| format!("{}:{}", t.handler_type, t.codec))
                    .collect::<Vec<_>>()
                    .join(",");
                print_kv(
                    &[
                        ("format", body_kind::MP4.to_string()),
                        ("duration_ms", r.duration_ms.to_string()),
                        ("track_count", track_count.to_string()),
                        ("tracks", track_summary),
                        ("canonical_bytes", r.canonical_bytes.len().to_string()),
                    ],
                    json,
                )
            }
            Err(e) => {
                eprintln!("nom: MP4 ingest failed: {e}");
                return 1;
            }
        },
        "hevc" | "h265" | "265" => match ingest_hevc(&bytes) {
            Ok(r) => print_kv(
                &[
                    ("format", body_kind::HEVC.to_string()),
                    ("width", r.width.to_string()),
                    ("height", r.height.to_string()),
                    ("profile_idc", r.profile_idc.to_string()),
                    ("nal_unit_count", r.nal_unit_count.to_string()),
                    ("canonical_bytes", r.canonical_bytes.len().to_string()),
                ],
                json,
            ),
            Err(e) => {
                eprintln!("nom: HEVC ingest failed: {e}");
                return 1;
            }
        },
        _ => {
            eprintln!("nom: no codec for extension: {ext}");
            return 1;
        }
    }

    0
}
