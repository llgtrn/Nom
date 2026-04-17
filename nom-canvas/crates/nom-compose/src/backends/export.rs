#![deny(unsafe_code)]
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

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
    pub fn compose(input: ExportInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> ExportOutput {
        sink.emit(ComposeEvent::Started { backend: "export".into(), entity_id: input.entity.id.clone() });
        let source = store.read(&input.input_hash).unwrap_or_default();
        let converted = Self::convert(&input.output_format, &source);
        let artifact_hash = store.write(&converted);
        let byte_size = converted.len() as u64;
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });
        ExportOutput { artifact_hash, byte_size, format: input.output_format }
    }

    fn convert(format: &str, data: &[u8]) -> Vec<u8> {
        match format {
            "base64" => {
                // Minimal base64 encoding stub
                let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                let mut out = Vec::new();
                let mut i = 0;
                while i < data.len() {
                    let b0 = data[i];
                    let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
                    let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };
                    out.push(alphabet[(b0 >> 2) as usize]);
                    out.push(alphabet[((b0 & 3) << 4 | b1 >> 4) as usize]);
                    out.push(if i + 1 < data.len() { alphabet[((b1 & 0xf) << 2 | b2 >> 6) as usize] } else { b'=' });
                    out.push(if i + 2 < data.len() { alphabet[(b2 & 0x3f) as usize] } else { b'=' });
                    i += 3;
                }
                out
            },
            "hex" => data.iter().flat_map(|b| {
                let hi = b >> 4;
                let lo = b & 0xf;
                let hex_char = |n: u8| if n < 10 { b'0' + n } else { b'a' + n - 10 };
                [hex_char(hi), hex_char(lo)]
            }).collect(),
            _ => data.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

    #[test]
    fn export_hex_format() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"\xde\xad");
        let out = ExportBackend::compose(ExportInput {
            entity: NomtuRef { id: "ex1".into(), word: "export".into(), kind: "concept".into() },
            input_hash,
            output_format: "hex".into(),
        }, &mut store, &LogProgressSink);
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"dead");
    }

    #[test]
    fn export_passthrough_unknown_format() {
        let mut store = InMemoryStore::new();
        let input_hash = store.write(b"raw");
        let out = ExportBackend::compose(ExportInput {
            entity: NomtuRef { id: "ex2".into(), word: "export".into(), kind: "concept".into() },
            input_hash,
            output_format: "raw".into(),
        }, &mut store, &LogProgressSink);
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"raw");
    }
}
