//! `nom store add-media` subcommand — ingest a media file into the dict.

use std::path::Path;

use nom_types::{Contract, Entry, EntryKind, EntryStatus};
use sha2::{Digest, Sha256};

use super::{chrono_like_now, open_dict};

/// `nom store add-media <file> [--dict <path>] [--json] [--preserve-format]`
///
/// Ingest a media file, persist its canonical bytes to a `nomtu` row tagged
/// with the matching §4.4.6 `body_kind`, and print the resulting id.
///
/// Default (modality-canonical track): still images are re-encoded to AVIF
/// regardless of source format. Pass `preserve_format = true` (CLI flag
/// `--preserve-format`) to store PNG→PNG, JPEG→JPEG, etc. instead.
pub fn cmd_store_add_media(path: &Path, dict: &Path, json: bool, preserve_format: bool) -> i32 {
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

    // Dispatch to the shared ingest helper (same table as `nom media import`).
    // preserve_format=false (default) routes ImageStill formats to modality-
    // canonical AVIF; preserve_format=true uses the per-format track.
    let summary = match crate::media::ingest_by_extension(&bytes, &ext, preserve_format) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: {e}");
            return 1;
        }
    };

    // SHA-256 the canonical bytes → hex id (same shape as Phase-4 hashes).
    let id = {
        let mut hasher = Sha256::new();
        hasher.update(&summary.canonical_bytes);
        format!("{:x}", hasher.finalize())
    };

    // Derive word = file stem, stripped to [a-z0-9].
    let word = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("media")
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>();
    let word = if word.is_empty() { "media".to_string() } else { word };

    // variant = the extension (lets multiple encodings of same word coexist).
    let variant = Some(ext.clone());

    let canonical_bytes_len = summary.canonical_bytes.len();

    let entry = Entry {
        id: id.clone(),
        word: word.clone(),
        variant,
        kind: EntryKind::MediaUnit,
        language: "media".to_string(),
        describe: Some(summary.describe.clone()),
        concept: None,
        body: None,
        body_nom: None,
        body_bytes: Some(summary.canonical_bytes),
        body_kind: Some(summary.body_kind_tag.to_owned()),
        contract: Contract::default(),
        status: EntryStatus::Complete,
        translation_score: None,
        is_canonical: true,
        deprecated_by: None,
        created_at: chrono_like_now(),
        updated_at: None,
    };

    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };

    if let Err(e) = dict_db.upsert_entry(&entry) {
        eprintln!("nom: upsert error for {id}: {e}");
        return 1;
    }

    if json {
        println!(
            "{{\"id\":\"{id}\",\"body_kind\":\"{}\",\"canonical_bytes\":{canonical_bytes_len},\"word\":\"{word}\",\"variant\":\"{ext}\"}}",
            summary.body_kind_tag
        );
    } else {
        println!("id:              {id}");
        println!("body_kind:       {}", summary.body_kind_tag);
        println!("canonical_bytes: {canonical_bytes_len}");
        println!("word:            {word}");
        println!("variant:         {ext}");
        println!("describe:        {}", summary.describe);
    }
    0
}
