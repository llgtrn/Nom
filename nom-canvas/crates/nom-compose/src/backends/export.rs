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
            assert!(out.byte_size > 0, "format {fmt} must produce non-empty output");
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
        assert_eq!(out.byte_size, 0, "missing hash must produce zero-size output");
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
                out.byte_size,
                expected_len,
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
}
