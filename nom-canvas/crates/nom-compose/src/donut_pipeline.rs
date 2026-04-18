/// Special token in Donut-style document markup.
#[derive(Debug, Clone, PartialEq)]
pub enum DonutToken {
    Open(String),   // <s_key>
    Close(String),  // </s_key>
    Value(String),  // text content between tags
}

impl DonutToken {
    pub fn parse_sequence(text: &str) -> Vec<Self> {
        let mut tokens = Vec::new();
        let mut pos = 0;
        let chars: Vec<char> = text.chars().collect();
        while pos < chars.len() {
            if chars[pos] == '<' {
                // Find closing >
                if let Some(end) = chars[pos..].iter().position(|&c| c == '>') {
                    let tag: String = chars[pos+1..pos+end].iter().collect();
                    if tag.starts_with('/') {
                        tokens.push(DonutToken::Close(tag[1..].to_string()));
                    } else {
                        tokens.push(DonutToken::Open(tag.to_string()));
                    }
                    pos += end + 1;
                } else {
                    pos += 1;
                }
            } else {
                // Collect text until next <
                let start = pos;
                while pos < chars.len() && chars[pos] != '<' { pos += 1; }
                let text: String = chars[start..pos].iter().collect();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    tokens.push(DonutToken::Value(trimmed.to_string()));
                }
            }
        }
        tokens
    }
}

/// A structured field extracted from Donut output.
#[derive(Debug, Clone)]
pub struct DocField {
    pub key: String,
    pub value: String,
    pub children: Vec<DocField>,
}

impl DocField {
    pub fn leaf(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self { key: key.into(), value: value.into(), children: vec![] }
    }

    pub fn is_leaf(&self) -> bool { self.children.is_empty() }
}

/// Parsed document structure from Donut output.
#[derive(Debug, Clone)]
pub struct DocStructure {
    pub fields: Vec<DocField>,
    pub raw_markup: String,
}

impl DocStructure {
    pub fn field_count(&self) -> usize { self.fields.len() }

    pub fn get_field(&self, key: &str) -> Option<&DocField> {
        self.fields.iter().find(|f| f.key == key)
    }

    pub fn all_values(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.value.clone()).collect()
    }
}

/// Donut-style markup parser: converts token sequence to nested DocStructure.
pub struct DonutParser;

impl DonutParser {
    pub fn parse(markup: &str) -> DocStructure {
        let tokens = DonutToken::parse_sequence(markup);
        let mut fields = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                DonutToken::Open(key) => {
                    let key = key.clone();
                    i += 1;
                    let mut value = String::new();
                    // Collect value until matching close
                    while i < tokens.len() {
                        match &tokens[i] {
                            DonutToken::Value(v) => { value = v.clone(); i += 1; }
                            DonutToken::Close(_) => { i += 1; break; }
                            DonutToken::Open(_) => break,
                        }
                    }
                    fields.push(DocField::leaf(key, value));
                }
                _ => { i += 1; }
            }
        }
        DocStructure { fields, raw_markup: markup.to_string() }
    }
}

/// Document task types for Donut.
#[derive(Debug, Clone, PartialEq)]
pub enum DocTask {
    DocumentParsing,   // extract structured info from document
    DocVqa,            // visual question answering
    DocumentClassify,  // classify document type
}

impl DocTask {
    pub fn prompt_template(&self) -> &'static str {
        match self {
            Self::DocumentParsing => "<s_document>",
            Self::DocVqa => "<s_docvqa><s_question>",
            Self::DocumentClassify => "<s_classify>",
        }
    }
}

/// Donut pipeline stub — processes document image → structured output.
pub struct DonutPipeline {
    pub task: DocTask,
    pub max_length: usize,
}

impl DonutPipeline {
    pub fn new(task: DocTask) -> Self {
        Self { task, max_length: 512 }
    }

    /// Process a document image (stub — returns synthetic structured output).
    pub fn process(&self, image_width: u32, image_height: u32, question: Option<&str>) -> DocStructure {
        let markup = match &self.task {
            DocTask::DocumentParsing => format!(
                "<s_document><s_date>2026-04-19</s_date><s_total>${:.2}</s_total><s_type>invoice</s_type></s_document>",
                (image_width as f32 * 0.01).round()
            ),
            DocTask::DocVqa => format!(
                "<s_docvqa><s_question>{}</s_question><s_answer>42</s_answer></s_docvqa>",
                question.unwrap_or("what is the total?")
            ),
            DocTask::DocumentClassify => "<s_classify><s_class>invoice</s_class></s_classify>".to_string(),
        };
        let _ = image_height;
        DonutParser::parse(&markup)
    }
}

#[cfg(test)]
mod donut_tests {
    use super::*;

    #[test]
    fn test_token_parse_basic() {
        let tokens = DonutToken::parse_sequence("<s_name>Alice</s_name>");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], DonutToken::Open("s_name".into()));
        assert_eq!(tokens[1], DonutToken::Value("Alice".into()));
        assert_eq!(tokens[2], DonutToken::Close("s_name".into()));
    }

    #[test]
    fn test_donut_parser_leaf_fields() {
        let doc = DonutParser::parse("<s_date>2026</s_date><s_total>100</s_total>");
        assert_eq!(doc.field_count(), 2);
        assert_eq!(doc.get_field("s_date").unwrap().value, "2026");
        assert_eq!(doc.get_field("s_total").unwrap().value, "100");
    }

    #[test]
    fn test_donut_parser_empty() {
        let doc = DonutParser::parse("");
        assert_eq!(doc.field_count(), 0);
    }

    #[test]
    fn test_doc_task_prompt_template() {
        assert_eq!(DocTask::DocumentParsing.prompt_template(), "<s_document>");
        assert_eq!(DocTask::DocVqa.prompt_template(), "<s_docvqa><s_question>");
    }

    #[test]
    fn test_pipeline_document_parsing() {
        let p = DonutPipeline::new(DocTask::DocumentParsing);
        let doc = p.process(800, 1100, None);
        assert!(doc.field_count() > 0);
        assert!(doc.get_field("s_date").is_some());
    }

    #[test]
    fn test_pipeline_docvqa() {
        let p = DonutPipeline::new(DocTask::DocVqa);
        let doc = p.process(800, 1100, Some("what is the date?"));
        assert!(doc.get_field("s_answer").is_some());
    }

    #[test]
    fn test_pipeline_classify() {
        let p = DonutPipeline::new(DocTask::DocumentClassify);
        let doc = p.process(800, 1100, None);
        assert!(doc.get_field("s_class").is_some());
    }

    #[test]
    fn test_doc_field_is_leaf() {
        let f = DocField::leaf("key", "value");
        assert!(f.is_leaf());
    }

    #[test]
    fn test_all_values() {
        let doc = DonutParser::parse("<s_a>x</s_a><s_b>y</s_b>");
        let vals = doc.all_values();
        assert!(vals.contains(&"x".to_string()));
        assert!(vals.contains(&"y".to_string()));
    }
}
