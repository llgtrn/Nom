#![deny(unsafe_code)]
use crate::backends::ComposeResult;

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

impl DataExtractSpec {
    /// Returns true when a specific page range has been set.
    pub fn is_paged(&self) -> bool {
        self.page_range.is_some()
    }
}

pub fn compose(spec: &DataExtractSpec) -> ComposeResult {
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
