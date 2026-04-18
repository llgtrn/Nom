#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::NomtuRef;

pub struct ExportInput {
    pub entity: NomtuRef,
    pub input_hash: [u8; 32],
    pub output_format: String,
}

pub struct ExportOutput {
    pub artifact_hash: [u8; 32],
    pub byte_size: u64,
    pub format: String,
}

pub struct ExportBackend;

impl ExportBackend {
    pub fn compose(
        input: ExportInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> ExportOutput {
        sink.emit(ComposeEvent::Started {
            backend: "export".into(),
            entity_id: input.entity.id.clone(),
        });
        let source = store.read(&input.input_hash).unwrap_or_default();
        let converted = Self::convert(&input.output_format, &source);
        let artifact_hash = store.write(&converted);
        let byte_size = converted.len() as u64;
        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        });
        ExportOutput {
            artifact_hash,
            byte_size,
            format: input.output_format,
        }
    }

    fn convert(format: &str, data: &[u8]) -> Vec<u8> {
        match format {
            "base64" => encode_base64(data),
            "hex" => data
                .iter()
                .flat_map(|b| {
                    let hi = b >> 4;
                    let lo = b & 0xf;
                    let hex_char = |n: u8| if n < 10 { b'0' + n } else { b'a' + n - 10 };
                    [hex_char(hi), hex_char(lo)]
                })
                .collect(),
            _ => data.to_vec(),
        }
    }
}

fn encode_base64(data: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(ALPHABET[(b0 >> 2) as usize]);
        out.push(ALPHABET[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize]);
        out.push(if chunk.len() > 1 {
            ALPHABET[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(b2 & 0b0011_1111) as usize]
        } else {
            b'='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

    #[test]
    fn export_hex_format() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"\xde\xad");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef {
                    id: "ex1".into(),
                    word: "export".into(),
                    kind: "concept".into(),
                },
                input_hash,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"dead");
    }

    #[test]
    fn export_passthrough_unknown_format() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"raw");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef {
                    id: "ex2".into(),
                    word: "export".into(),
                    kind: "concept".into(),
                },
                input_hash,
                output_format: "raw".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"raw");
    }

    #[test]
    fn export_base64_format_with_padding() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"Nom");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("ex3", "export", "concept"),
                input_hash,
                output_format: "base64".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(store.read(&out.artifact_hash).unwrap(), b"Tm9t");

        let input_hash = store.write(b"No");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("ex4", "export", "concept"),
                input_hash,
                output_format: "base64".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(store.read(&out.artifact_hash).unwrap(), b"Tm8=");
    }

    #[test]
    fn export_multiple_formats_sequentially() {
        let mut store = InMemoryStore::new();
        let data = b"\xca\xfe";
        let formats = ["hex", "base64", "raw"];
        for (i, fmt) in formats.iter().enumerate() {
            let input_hash = store.write(data);
            let out = ExportBackend::compose(
                ExportInput {
                    entity: NomtuRef::new(format!("multi{i}"), "export", "concept"),
                    input_hash,
                    output_format: fmt.to_string(),
                },
                &mut store,
                &LogProgressSink,
            );
            assert_eq!(out.format, *fmt);
            assert!(
                out.byte_size > 0,
                "format {fmt} must produce non-empty output"
            );
        }
    }

    #[test]
    fn export_failure_returns_error_via_missing_hash() {
        // Reading a hash that was never written returns empty bytes (default).
        // ExportBackend uses unwrap_or_default, so result is zero-byte output.
        let mut store = InMemoryStore::new();
        let missing_hash = [0xffu8; 32]; // hash not written to store
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("missing", "export", "concept"),
                input_hash: missing_hash,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        // Empty source produces empty hex — byte_size is 0.
        assert_eq!(
            out.byte_size, 0,
            "missing hash must produce zero-size output"
        );
        let result = store.read(&out.artifact_hash).unwrap_or_default();
        assert!(result.is_empty(), "hex of empty slice must be empty");
    }

    #[test]
    fn export_base64_length_formula() {
        // For n input bytes, base64 output length = ceil(n / 3) * 4.
        let mut store = InMemoryStore::new();
        for n in [0usize, 1, 2, 3, 4, 5, 6, 9, 12, 15] {
            let input: Vec<u8> = (0..n as u8).collect();
            let input_hash = store.write(&input);
            let out = ExportBackend::compose(
                ExportInput {
                    entity: NomtuRef::new(format!("b64len{n}"), "export", "concept"),
                    input_hash,
                    output_format: "base64".into(),
                },
                &mut store,
                &LogProgressSink,
            );
            let expected_len = if n == 0 { 0 } else { n.div_ceil(3) * 4 } as u64;
            assert_eq!(
                out.byte_size, expected_len,
                "base64 length formula failed for input len {n}: got {}, expected {expected_len}",
                out.byte_size
            );
        }
    }

    #[test]
    fn export_hex_output_is_lowercase() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"\xAB\xCD\xEF");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("hexcase", "export", "concept"),
                input_hash,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"abcdef", "hex output must be lowercase");
    }

    #[test]
    fn export_empty_input_base64_produces_empty_output() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("empty-b64", "export", "concept"),
                input_hash,
                output_format: "base64".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.byte_size, 0);
        let result = store.read(&out.artifact_hash).unwrap_or_default();
        assert!(result.is_empty(), "empty input base64 must be empty");
    }

    #[test]
    fn export_hex_output_length_is_double_input() {
        let mut store = InMemoryStore::new();
        let input = b"\x01\x02\x03\x04";
        let input_hash = store.write(input);
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("hexlen", "export", "concept"),
                input_hash,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        // Each byte → 2 hex chars.
        assert_eq!(out.byte_size, (input.len() * 2) as u64);
    }

    #[test]
    fn export_compose_emits_started_and_completed_events() {
        use crate::progress::VecProgressSink;
        let mut store = InMemoryStore::new();
        let sink = VecProgressSink::new();
        let input_hash = store.write(b"hello");
        ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("events", "export", "concept"),
                input_hash,
                output_format: "raw".into(),
            },
            &mut store,
            &sink,
        );
        let events = sink.take();
        assert_eq!(events.len(), 2, "must emit Started and Completed events");
        assert!(matches!(events[0], ComposeEvent::Started { .. }));
        assert!(matches!(events[1], ComposeEvent::Completed { .. }));
    }

    // ── Wave AE new tests ────────────────────────────────────────────────────

    #[test]
    fn export_empty_source_returns_zero_byte_size() {
        // Empty artifact: hash points to empty bytes — byte_size must be 0.
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("empty-src", "export", "concept"),
                input_hash,
                output_format: "raw".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(
            out.byte_size, 0,
            "empty source must produce zero-size output"
        );
    }

    #[test]
    fn export_known_formats_produce_non_empty_output_for_non_empty_input() {
        // Known formats: base64 and hex must produce non-empty output for non-empty input.
        let mut store = InMemoryStore::new();
        for fmt in ["base64", "hex"] {
            let input_hash = store.write(b"\x01");
            let out = ExportBackend::compose(
                ExportInput {
                    entity: NomtuRef::new(format!("fmt-{fmt}"), "export", "concept"),
                    input_hash,
                    output_format: fmt.into(),
                },
                &mut store,
                &LogProgressSink,
            );
            assert!(
                out.byte_size > 0,
                "known format {fmt} must produce non-empty output"
            );
        }
    }

    #[test]
    fn export_unknown_format_acts_as_passthrough() {
        // Any unrecognised format falls through to the identity branch.
        let mut store = InMemoryStore::new();
        let data = b"passme";
        for fmt in ["bin", "zstd", "lz4", "unknown_xyz"] {
            let input_hash = store.write(data);
            let out = ExportBackend::compose(
                ExportInput {
                    entity: NomtuRef::new(format!("unk-{fmt}"), "export", "concept"),
                    input_hash,
                    output_format: fmt.into(),
                },
                &mut store,
                &LogProgressSink,
            );
            let result = store.read(&out.artifact_hash).unwrap();
            assert_eq!(
                result, data,
                "unknown format {fmt} must pass bytes through unchanged"
            );
        }
    }

    #[test]
    fn export_format_field_echoes_requested_format() {
        // ExportOutput.format must echo the requested format string exactly.
        let mut store = InMemoryStore::new();
        for fmt in ["base64", "hex", "raw", "custom_fmt"] {
            let input_hash = store.write(b"x");
            let out = ExportBackend::compose(
                ExportInput {
                    entity: NomtuRef::new(format!("echofmt-{fmt}"), "export", "concept"),
                    input_hash,
                    output_format: fmt.into(),
                },
                &mut store,
                &LogProgressSink,
            );
            assert_eq!(
                out.format, fmt,
                "output.format must echo input.output_format"
            );
        }
    }

    #[test]
    fn export_multi_format_same_source_produces_distinct_hashes() {
        // Exporting the same bytes in different formats must yield different artifact hashes
        // (because the encoded content differs).
        let mut store = InMemoryStore::new();
        let data = b"\xbe\xef";
        let input_hash_hex = store.write(data);
        let input_hash_b64 = store.write(data);

        let out_hex = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("multi-hex", "export", "concept"),
                input_hash: input_hash_hex,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let out_b64 = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("multi-b64", "export", "concept"),
                input_hash: input_hash_b64,
                output_format: "base64".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_ne!(
            out_hex.artifact_hash, out_b64.artifact_hash,
            "hex and base64 outputs for same source must have different hashes"
        );
    }

    #[test]
    fn export_completed_event_carries_correct_byte_size() {
        use crate::progress::VecProgressSink;
        let mut store = InMemoryStore::new();
        let sink = VecProgressSink::new();
        let data = b"\x01\x02\x03";
        let input_hash = store.write(data);
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("meta-size", "export", "concept"),
                input_hash,
                output_format: "hex".into(),
            },
            &mut store,
            &sink,
        );
        let events = sink.take();
        let completed = events
            .iter()
            .find(|e| matches!(e, ComposeEvent::Completed { .. }));
        assert!(completed.is_some(), "must emit Completed event");
        if let Some(ComposeEvent::Completed { byte_size, .. }) = completed {
            assert_eq!(
                *byte_size, out.byte_size,
                "Completed event byte_size must match output"
            );
        }
    }

    #[test]
    fn export_started_event_carries_entity_id() {
        use crate::progress::VecProgressSink;
        let mut store = InMemoryStore::new();
        let sink = VecProgressSink::new();
        let input_hash = store.write(b"id-check");
        ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("my-entity-id", "export", "concept"),
                input_hash,
                output_format: "raw".into(),
            },
            &mut store,
            &sink,
        );
        let events = sink.take();
        if let ComposeEvent::Started { entity_id, backend } = &events[0] {
            assert_eq!(entity_id, "my-entity-id");
            assert_eq!(backend, "export");
        } else {
            panic!("first event must be Started");
        }
    }

    #[test]
    fn export_raw_passthrough_byte_size_equals_input_len() {
        let mut store = InMemoryStore::new();
        let data = b"hello world";
        let input_hash = store.write(data);
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("rawlen", "export", "concept"),
                input_hash,
                output_format: "raw".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.byte_size, data.len() as u64);
    }

    #[test]
    fn export_hex_all_zero_bytes_produces_correct_output() {
        let mut store = InMemoryStore::new();
        let data = [0u8; 4];
        let input_hash = store.write(&data);
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("hexzero", "export", "concept"),
                input_hash,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"00000000");
    }

    #[test]
    fn export_base64_single_byte_has_padding() {
        // 1-byte input → base64 always has two '=' padding chars.
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"\xff");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("b64-pad1", "export", "concept"),
                input_hash,
                output_format: "base64".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result.len(), 4, "single-byte base64 must be 4 chars");
        assert_eq!(result[2], b'=', "must have first padding '='");
        assert_eq!(result[3], b'=', "must have second padding '='");
    }

    #[test]
    fn export_hex_single_ff_byte() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"\xff");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("hex-ff", "export", "concept"),
                input_hash,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"ff");
    }

    #[test]
    fn export_format_raw_byte_size_equals_source() {
        let mut store = InMemoryStore::new();
        let data = b"abcdefghij"; // 10 bytes
        let input_hash = store.write(data);
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("rawsize", "export", "concept"),
                input_hash,
                output_format: "raw".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.byte_size, 10);
    }

    #[test]
    fn export_two_bytes_base64_has_one_padding() {
        // 2-byte input → base64 output ends with exactly one '='.
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"\x01\x02");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("b64-2b", "export", "concept"),
                input_hash,
                output_format: "base64".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(result[3], b'=', "2-byte input must have one trailing '='");
        assert_ne!(result[2], b'=', "2-byte input must NOT have two '=' pads");
    }

    #[test]
    fn export_three_bytes_base64_has_no_padding() {
        // 3-byte input → base64 has no padding.
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"\x01\x02\x03");
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("b64-3b", "export", "concept"),
                input_hash,
                output_format: "base64".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result.len(), 4);
        assert_ne!(result[3], b'=', "3-byte input must NOT have padding");
        assert_ne!(result[2], b'=', "3-byte input must NOT have padding");
    }

    #[test]
    fn export_hex_output_only_contains_valid_hex_chars() {
        let mut store = InMemoryStore::new();
        let data: Vec<u8> = (0u8..=255).collect();
        let input_hash = store.write(&data);
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("hexvalid", "export", "concept"),
                input_hash,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let result = store.read(&out.artifact_hash).unwrap();
        for &b in &result {
            assert!(
                b.is_ascii_hexdigit(),
                "hex output must only contain [0-9a-f], got char {b}"
            );
        }
    }

    #[test]
    fn export_artifact_hash_changes_when_data_changes() {
        let mut store = InMemoryStore::new();
        let h1 = store.write(b"data-a");
        let h2 = store.write(b"data-b");
        let out1 = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("chg1", "export", "concept"),
                input_hash: h1,
                output_format: "raw".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let out2 = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("chg2", "export", "concept"),
                input_hash: h2,
                output_format: "raw".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_ne!(
            out1.artifact_hash, out2.artifact_hash,
            "different source data must produce different artifact hashes"
        );
    }

    #[test]
    fn export_same_data_same_format_produces_same_hash() {
        let mut store = InMemoryStore::new();
        let data = b"stable-content";
        let h1 = store.write(data);
        let h2 = store.write(data);
        let out1 = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("stable1", "export", "concept"),
                input_hash: h1,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        let out2 = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef::new("stable2", "export", "concept"),
                input_hash: h2,
                output_format: "hex".into(),
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(
            out1.artifact_hash, out2.artifact_hash,
            "same data + same format must produce identical artifact hashes"
        );
    }
}
