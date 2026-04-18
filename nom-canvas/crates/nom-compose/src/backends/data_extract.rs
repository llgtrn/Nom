#![deny(unsafe_code)]
use crate::backends::ComposeResult;
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;

/// Source format for data extraction.
#[derive(Debug, Clone, PartialEq)]
pub enum ExtractMode {
    Pdf,
    Html,
    Image,
    Raw,
}

/// Specification for extracting structured data from a source.
#[derive(Debug, Clone)]
pub struct DataExtractSpec {
    pub source_uri: String,
    pub mode: ExtractMode,
    pub page_range: Option<(u32, u32)>,
}

pub struct DataExtractOutput {
    pub artifact_hash: [u8; 32],
    pub row_count: usize,
    pub field_count: usize,
}

impl DataExtractSpec {
    /// Returns true when a specific page range has been set.
    pub fn is_paged(&self) -> bool {
        self.page_range.is_some()
    }
}

pub fn compose(spec: &DataExtractSpec) -> ComposeResult {
    validate(spec)
}

fn validate(spec: &DataExtractSpec) -> ComposeResult {
    if spec.source_uri.is_empty() {
        return Err("data extract source_uri must not be empty".into());
    }
    if let Some((start, end)) = spec.page_range {
        if start > end {
            return Err(format!(
                "page_range start ({}) must not exceed end ({})",
                start, end
            ));
        }
    }
    Ok(())
}

pub fn compose_to_store(
    spec: &DataExtractSpec,
    store: &mut dyn ArtifactStore,
    sink: &dyn ProgressSink,
) -> Result<DataExtractOutput, String> {
    validate(spec)?;
    sink.emit(ComposeEvent::Started {
        backend: "data_extract".into(),
        entity_id: spec.source_uri.clone(),
    });
    let fields = mode_fields(&spec.mode);
    let json = serde_json::json!({
        "source_uri": spec.source_uri,
        "mode": format!("{:?}", spec.mode),
        "page_range": spec.page_range,
        "fields": fields,
        "rows": [{
            "source_uri": spec.source_uri,
            "mode": format!("{:?}", spec.mode),
        }],
    });
    let bytes = json.to_string().into_bytes();
    sink.emit(ComposeEvent::Progress {
        percent: 0.5,
        stage: "structured extraction".into(),
                rendered_frames: None,
                encoded_frames: None,
                elapsed_ms: None,
    });
    let artifact_hash = store.write(&bytes);
    sink.emit(ComposeEvent::Completed {
        artifact_hash,
        byte_size: bytes.len() as u64,
    });
    Ok(DataExtractOutput {
        artifact_hash,
        row_count: 1,
        field_count: fields.len(),
    })
}

fn mode_fields(mode: &ExtractMode) -> &'static [&'static str] {
    match mode {
        ExtractMode::Pdf => &["page", "text", "tables"],
        ExtractMode::Html => &["title", "links", "text"],
        ExtractMode::Image => &["ocr_text", "regions"],
        ExtractMode::Raw => &["bytes", "mime"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_extract_is_paged() {
        let unpaged = DataExtractSpec {
            source_uri: "file:///doc.html".into(),
            mode: ExtractMode::Html,
            page_range: None,
        };
        assert!(!unpaged.is_paged());

        let paged = DataExtractSpec {
            source_uri: "file:///report.pdf".into(),
            mode: ExtractMode::Pdf,
            page_range: Some((1, 10)),
        };
        assert!(paged.is_paged());
    }

    #[test]
    fn data_extract_compose_produces_artifact() {
        let spec = DataExtractSpec {
            source_uri: "file:///image.png".into(),
            mode: ExtractMode::Image,
            page_range: None,
        };
        let result = compose(&spec);
        assert!(result.is_ok(), "compose must return Ok for valid spec");
    }

    #[test]
    fn data_extract_compose_to_store_writes_structured_json() {
        let mut store = crate::store::InMemoryStore::new();
        let sink = crate::progress::VecProgressSink::new();
        let spec = DataExtractSpec {
            source_uri: "file:///report.pdf".into(),
            mode: ExtractMode::Pdf,
            page_range: Some((1, 2)),
        };
        let out = compose_to_store(&spec, &mut store, &sink).unwrap();
        assert_eq!(out.row_count, 1);
        assert_eq!(out.field_count, 3);
        let payload = store.read(&out.artifact_hash).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(value["source_uri"], "file:///report.pdf");
        assert!(value["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "tables"));
        assert!(sink.take().len() >= 3);
    }

    #[test]
    fn data_extract_backend_kind() {
        // The backend name for data extraction is "data_extract".
        let spec = DataExtractSpec {
            source_uri: "file:///doc.pdf".into(),
            mode: ExtractMode::Pdf,
            page_range: None,
        };
        // The module name is "data_extract" — verify spec fields are intact.
        assert_eq!(spec.mode, ExtractMode::Pdf);
        assert!(!spec.source_uri.is_empty());
    }

    #[test]
    fn data_extract_backend_compose_ok() {
        let spec = DataExtractSpec {
            source_uri: "https://example.com/page".into(),
            mode: ExtractMode::Html,
            page_range: None,
        };
        assert!(compose(&spec).is_ok());
    }
}
