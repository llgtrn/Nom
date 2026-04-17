//! Data extraction backend (PDF/HTML/text → JSON/CSV).
//!
//! Describes the XY-Cut++ layout reconstruction spec.  Actual parsing
//! lives in runtime crates; this is pure data + a tiny XY-Cut demo on
//! pre-extracted blocks (for testing).
#![deny(unsafe_code)]

use crate::backend_trait::{CompositionBackend, ComposeSpec, ComposeOutput, ComposeError, InterruptFlag, ProgressSink};
use crate::kind::NomKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat { Json, Csv, Tsv, Ndjson }

#[derive(Clone, Debug, PartialEq)]
pub struct SchemaField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ExtractionSchema {
    pub fields: Vec<SchemaField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtractSpec {
    pub source_path: String,
    pub format: OutputFormat,
    pub schema: Option<ExtractionSchema>,
    /// When true, a multi-column layout is reconstructed via XY-Cut before schema-extraction.
    pub reconstruct_layout: bool,
    pub page_range: Option<(u32, u32)>,
}

impl ExtractSpec {
    pub fn new(source_path: impl Into<String>) -> Self {
        Self {
            source_path: source_path.into(),
            format: OutputFormat::Json,
            schema: None,
            reconstruct_layout: true,
            page_range: None,
        }
    }
    pub fn with_format(mut self, format: OutputFormat) -> Self { self.format = format; self }
    pub fn with_schema(mut self, schema: ExtractionSchema) -> Self { self.schema = Some(schema); self }
    pub fn with_page_range(mut self, start: u32, end: u32) -> Self { self.page_range = Some((start, end)); self }
    pub fn mime_type(&self) -> &'static str {
        match self.format {
            OutputFormat::Json   => "application/json",
            OutputFormat::Csv    => "text/csv",
            OutputFormat::Tsv    => "text/tab-separated-values",
            OutputFormat::Ndjson => "application/x-ndjson",
        }
    }
}

/// Text block on a page with its bounding box.  Consumed by XY-Cut.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutBlock {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Recursive XY-Cut: find the largest horizontal or vertical gap between
/// blocks and split into two groups; recurse until groups are small.
/// Returns blocks sorted in natural reading order (top-to-bottom,
/// left-to-right within each horizontal band).
pub fn xy_cut(mut blocks: Vec<LayoutBlock>) -> Vec<LayoutBlock> {
    if blocks.len() <= 1 { return blocks; }
    // Find the largest horizontal gap (sorted by y).
    blocks.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));
    let mut best_idx = 0usize;
    let mut best_gap = 0f32;
    for i in 1..blocks.len() {
        let gap = blocks[i].y - (blocks[i - 1].y + blocks[i - 1].height);
        if gap > best_gap { best_gap = gap; best_idx = i; }
    }
    if best_gap > 0.0 {
        let tail = blocks.split_off(best_idx);
        let mut sorted = xy_cut_horizontal(blocks);
        sorted.extend(xy_cut(tail));
        sorted
    } else {
        xy_cut_horizontal(blocks)
    }
}

/// Within a horizontal band, sort left-to-right (by x).
fn xy_cut_horizontal(mut blocks: Vec<LayoutBlock>) -> Vec<LayoutBlock> {
    blocks.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    blocks
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("source_path must not be empty")]
    EmptyPath,
    #[error("page_range end {1} < start {0}")]
    BadPageRange(u32, u32),
}

pub fn validate(spec: &ExtractSpec) -> Result<(), ExtractError> {
    if spec.source_path.trim().is_empty() { return Err(ExtractError::EmptyPath); }
    if let Some((s, e)) = spec.page_range { if e < s { return Err(ExtractError::BadPageRange(s, e)); } }
    Ok(())
}

pub struct StubDataExtractBackend;

impl CompositionBackend for StubDataExtractBackend {
    fn kind(&self) -> NomKind { NomKind::DataExtract }
    fn name(&self) -> &str { "stub-data-extract" }
    fn compose(&self, _spec: &ComposeSpec, _progress: &dyn ProgressSink, _interrupt: &InterruptFlag) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput { bytes: b"{}".to_vec(), mime_type: "application/json".to_string(), cost_cents: 0 })
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn block(text: &str, x: f32, y: f32) -> LayoutBlock {
        LayoutBlock { text: text.into(), x, y, width: 10.0, height: 5.0 }
    }

    #[test]
    fn extract_spec_new_defaults() {
        let s = ExtractSpec::new("file.pdf");
        assert_eq!(s.source_path, "file.pdf");
        assert_eq!(s.format, OutputFormat::Json);
        assert!(s.reconstruct_layout);
        assert!(s.schema.is_none());
        assert!(s.page_range.is_none());
    }

    #[test]
    fn with_format_chain() {
        let s = ExtractSpec::new("a.csv").with_format(OutputFormat::Csv);
        assert_eq!(s.format, OutputFormat::Csv);
    }

    #[test]
    fn with_schema_chain() {
        let schema = ExtractionSchema {
            fields: vec![SchemaField { name: "title".into(), field_type: "string".into(), required: true }],
        };
        let s = ExtractSpec::new("a.pdf").with_schema(schema.clone());
        assert_eq!(s.schema.unwrap(), schema);
    }

    #[test]
    fn with_page_range_chain() {
        let s = ExtractSpec::new("doc.pdf").with_page_range(2, 5);
        assert_eq!(s.page_range, Some((2, 5)));
    }

    #[test]
    fn mime_type_json() {
        assert_eq!(ExtractSpec::new("x").mime_type(), "application/json");
    }

    #[test]
    fn mime_type_csv() {
        assert_eq!(ExtractSpec::new("x").with_format(OutputFormat::Csv).mime_type(), "text/csv");
    }

    #[test]
    fn mime_type_tsv() {
        assert_eq!(ExtractSpec::new("x").with_format(OutputFormat::Tsv).mime_type(), "text/tab-separated-values");
    }

    #[test]
    fn mime_type_ndjson() {
        assert_eq!(ExtractSpec::new("x").with_format(OutputFormat::Ndjson).mime_type(), "application/x-ndjson");
    }

    #[test]
    fn xy_cut_empty_returns_empty() {
        let result = xy_cut(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn xy_cut_single_block_unchanged() {
        let b = block("hello", 0.0, 0.0);
        let result = xy_cut(vec![b.clone()]);
        assert_eq!(result, vec![b]);
    }

    #[test]
    fn xy_cut_two_vertical_blocks_top_first() {
        let top = block("top", 0.0, 0.0);
        let bottom = block("bottom", 0.0, 100.0);
        let result = xy_cut(vec![bottom.clone(), top.clone()]);
        assert_eq!(result[0], top);
        assert_eq!(result[1], bottom);
    }

    #[test]
    fn xy_cut_horizontal_band_left_to_right() {
        // Four blocks at same y, scrambled x order
        let b1 = block("a", 10.0, 5.0);
        let b2 = block("b", 30.0, 5.0);
        let b3 = block("c", 50.0, 5.0);
        let b4 = block("d", 70.0, 5.0);
        let result = xy_cut(vec![b3.clone(), b1.clone(), b4.clone(), b2.clone()]);
        assert_eq!(result[0], b1);
        assert_eq!(result[1], b2);
        assert_eq!(result[2], b3);
        assert_eq!(result[3], b4);
    }

    #[test]
    fn validate_ok() {
        let s = ExtractSpec::new("report.pdf").with_page_range(1, 5);
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn validate_empty_path_err() {
        let s = ExtractSpec::new("   ");
        let err = validate(&s).unwrap_err();
        assert!(matches!(err, ExtractError::EmptyPath));
    }

    #[test]
    fn validate_bad_page_range_err() {
        let s = ExtractSpec::new("doc.pdf").with_page_range(10, 5);
        let err = validate(&s).unwrap_err();
        assert!(matches!(err, ExtractError::BadPageRange(10, 5)));
    }

    #[test]
    fn stub_backend_kind_and_name() {
        let b = StubDataExtractBackend;
        assert_eq!(b.kind(), NomKind::DataExtract);
        assert_eq!(b.name(), "stub-data-extract");
    }
}
