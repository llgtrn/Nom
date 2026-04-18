use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ExtractTarget
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractTarget {
    Pdf,
    Docx,
    Html,
    Csv,
    Json,
}

impl ExtractTarget {
    pub fn mime_type(&self) -> &'static str {
        match self {
            ExtractTarget::Pdf => "application/pdf",
            ExtractTarget::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            ExtractTarget::Html => "text/html",
            ExtractTarget::Csv => "text/csv",
            ExtractTarget::Json => "application/json",
        }
    }

    pub fn is_text_based(&self) -> bool {
        matches!(self, ExtractTarget::Html | ExtractTarget::Csv | ExtractTarget::Json)
    }
}

// ---------------------------------------------------------------------------
// ExtractField
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ExtractField {
    pub name: String,
    pub required: bool,
}

impl ExtractField {
    pub fn is_optional(&self) -> bool {
        !self.required
    }

    pub fn field_key(&self) -> String {
        format!("field:{}", self.name.to_lowercase())
    }
}

// ---------------------------------------------------------------------------
// ExtractSchema
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ExtractSchema {
    pub fields: Vec<ExtractField>,
    pub target: ExtractTarget,
}

impl ExtractSchema {
    pub fn new(target: ExtractTarget) -> Self {
        Self {
            fields: Vec::new(),
            target,
        }
    }

    pub fn add_field(&mut self, field: ExtractField) {
        self.fields.push(field);
    }

    pub fn required_count(&self) -> usize {
        self.fields.iter().filter(|f| f.required).count()
    }

    pub fn optional_count(&self) -> usize {
        self.fields.iter().filter(|f| !f.required).count()
    }
}

// ---------------------------------------------------------------------------
// ExtractResult
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ExtractResult {
    pub extracted: HashMap<String, String>,
    pub missing_required: Vec<String>,
}

impl ExtractResult {
    pub fn new() -> Self {
        Self {
            extracted: HashMap::new(),
            missing_required: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, val: impl Into<String>) {
        self.extracted.insert(key.into(), val.into());
    }

    pub fn is_complete(&self) -> bool {
        self.missing_required.is_empty()
    }

    pub fn field_count(&self) -> usize {
        self.extracted.len()
    }
}

impl Default for ExtractResult {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ExtractPipeline
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ExtractPipeline {
    pub schema: ExtractSchema,
}

impl ExtractPipeline {
    pub fn new(schema: ExtractSchema) -> Self {
        Self { schema }
    }

    /// Returns a list of field names that are required but absent from `result.extracted`.
    pub fn validate(&self, result: &ExtractResult) -> Vec<String> {
        let mut errors = Vec::new();
        for field in &self.schema.fields {
            if field.required {
                let key = field.field_key();
                if !result.extracted.contains_key(&key) {
                    errors.push(field.name.clone());
                }
            }
        }
        errors
    }

    /// Returns a human-readable summary of the pipeline schema.
    pub fn summary(&self) -> String {
        format!(
            "{} target: {} required, {} optional fields",
            self.schema.target.mime_type(),
            self.schema.required_count(),
            self.schema.optional_count(),
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // 1. ExtractTarget::mime_type for Pdf
    #[test]
    fn extract_target_pdf_mime_type() {
        assert_eq!(ExtractTarget::Pdf.mime_type(), "application/pdf");
    }

    // 2. ExtractTarget::is_text_based for Html
    #[test]
    fn extract_target_html_is_text_based() {
        assert!(ExtractTarget::Html.is_text_based());
        assert!(!ExtractTarget::Pdf.is_text_based());
        assert!(!ExtractTarget::Docx.is_text_based());
    }

    // 3. ExtractField::is_optional returns true when required == false
    #[test]
    fn extract_field_is_optional() {
        let optional = ExtractField { name: "notes".to_string(), required: false };
        let required = ExtractField { name: "title".to_string(), required: true };
        assert!(optional.is_optional());
        assert!(!required.is_optional());
    }

    // 4. ExtractField::field_key lowercases the name and prefixes "field:"
    #[test]
    fn extract_field_key_lowercase() {
        let field = ExtractField { name: "AuthorName".to_string(), required: true };
        assert_eq!(field.field_key(), "field:authorname");
    }

    // 5. ExtractSchema::required_count and optional_count
    #[test]
    fn extract_schema_required_and_optional_count() {
        let mut schema = ExtractSchema::new(ExtractTarget::Json);
        schema.add_field(ExtractField { name: "id".to_string(), required: true });
        schema.add_field(ExtractField { name: "title".to_string(), required: true });
        schema.add_field(ExtractField { name: "description".to_string(), required: false });
        assert_eq!(schema.required_count(), 2);
        assert_eq!(schema.optional_count(), 1);
    }

    // 6. ExtractResult::insert increases field_count
    #[test]
    fn extract_result_insert_and_field_count() {
        let mut result = ExtractResult::new();
        assert_eq!(result.field_count(), 0);
        result.insert("field:id", "42");
        result.insert("field:title", "Hello");
        assert_eq!(result.field_count(), 2);
    }

    // 7. ExtractResult::is_complete returns true when missing_required is empty
    #[test]
    fn extract_result_is_complete_true() {
        let result = ExtractResult::new();
        assert!(result.is_complete());
    }

    // 8. ExtractResult::is_complete returns false when missing_required is non-empty
    #[test]
    fn extract_result_is_complete_false() {
        let mut result = ExtractResult::new();
        result.missing_required.push("id".to_string());
        assert!(!result.is_complete());
    }

    // 9. ExtractPipeline::validate finds missing required fields;
    //    summary produces the expected format string.
    #[test]
    fn extract_pipeline_validate_and_summary() {
        let mut schema = ExtractSchema::new(ExtractTarget::Csv);
        schema.add_field(ExtractField { name: "row_id".to_string(), required: true });
        schema.add_field(ExtractField { name: "value".to_string(), required: true });
        schema.add_field(ExtractField { name: "comment".to_string(), required: false });

        let pipeline = ExtractPipeline::new(schema);

        // Provide only the "value" field — "row_id" must be flagged as missing.
        let mut result = ExtractResult::new();
        result.insert("field:value", "99");

        let errors = pipeline.validate(&result);
        assert_eq!(errors.len(), 1, "one required field is missing");
        assert_eq!(errors[0], "row_id");

        let summary = pipeline.summary();
        assert!(
            summary.contains("text/csv"),
            "summary must contain mime type, got: {summary}"
        );
        assert!(
            summary.contains("2 required"),
            "summary must state 2 required, got: {summary}"
        );
        assert!(
            summary.contains("1 optional"),
            "summary must state 1 optional, got: {summary}"
        );
    }
}
