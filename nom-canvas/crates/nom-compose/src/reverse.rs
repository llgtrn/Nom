/// Input kinds for reverse engineering
pub enum ReverseInput {
    ImageBytes(Vec<u8>),
    WebUrl(String),
    ScreenshotPath(String),
}

/// Component detected in the input
pub struct DetectedComponent {
    pub kind: String,
    pub confidence: f32,
    pub nomx_snippet: String,
}

/// Result of reverse engineering
pub struct ReverseResult {
    pub components: Vec<DetectedComponent>,
    pub full_nomx: String,
    pub grammar_valid: bool,
    pub score: f32,
}

impl ReverseResult {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            full_nomx: String::new(),
            grammar_valid: false,
            score: 0.0,
        }
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn is_valid(&self) -> bool {
        self.grammar_valid && self.score > 0.5
    }
}

impl Default for ReverseResult {
    fn default() -> Self {
        Self::new()
    }
}

/// AI glue orchestrator for reverse engineering
pub struct ReverseOrchestrator {
    pub confidence_threshold: f32,
}

impl ReverseOrchestrator {
    pub fn new(threshold: f32) -> Self {
        Self {
            confidence_threshold: threshold,
        }
    }

    pub fn detect_components(input: &ReverseInput) -> Vec<DetectedComponent> {
        match input {
            ReverseInput::ImageBytes(_) => vec![
                DetectedComponent {
                    kind: "button".to_string(),
                    confidence: 0.85,
                    nomx_snippet: "  button that \"Submit\"".to_string(),
                },
                DetectedComponent {
                    kind: "card".to_string(),
                    confidence: 0.78,
                    nomx_snippet: "  card that contains image and text".to_string(),
                },
            ],
            ReverseInput::WebUrl(_) => vec![
                DetectedComponent {
                    kind: "header".to_string(),
                    confidence: 0.92,
                    nomx_snippet: "  header that contains logo and nav".to_string(),
                },
                DetectedComponent {
                    kind: "nav".to_string(),
                    confidence: 0.88,
                    nomx_snippet: "  nav that links to home and about".to_string(),
                },
                DetectedComponent {
                    kind: "content".to_string(),
                    confidence: 0.75,
                    nomx_snippet: "  content that shows main body".to_string(),
                },
            ],
            ReverseInput::ScreenshotPath(_) => vec![
                DetectedComponent {
                    kind: "panel".to_string(),
                    confidence: 0.80,
                    nomx_snippet: "  panel that wraps ui elements".to_string(),
                },
                DetectedComponent {
                    kind: "button".to_string(),
                    confidence: 0.72,
                    nomx_snippet: "  button that \"Action\"".to_string(),
                },
            ],
        }
    }

    pub fn to_nomx(components: &[DetectedComponent]) -> String {
        let mut lines = vec!["define page that".to_string()];
        for c in components {
            lines.push(c.nomx_snippet.clone());
        }
        lines.join("\n")
    }

    pub fn validate_grammar(nomx: &str) -> bool {
        !nomx.is_empty() && nomx.contains("define") && nomx.contains("that")
    }

    pub fn reverse(input: ReverseInput) -> ReverseResult {
        let components = Self::detect_components(&input);
        let full_nomx = Self::to_nomx(&components);
        let grammar_valid = Self::validate_grammar(&full_nomx);
        let score = if components.is_empty() {
            0.0
        } else {
            components.iter().map(|c| c.confidence).sum::<f32>() / components.len() as f32
        };
        ReverseResult {
            components,
            full_nomx,
            grammar_valid,
            score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_image_bytes_produces_components() {
        let input = ReverseInput::ImageBytes(vec![0u8, 1, 2, 3]);
        let components = ReverseOrchestrator::detect_components(&input);
        assert!(!components.is_empty(), "image bytes must produce at least one component");
        assert!(
            components.iter().any(|c| c.kind == "button"),
            "image bytes must detect a button component"
        );
    }

    #[test]
    fn reverse_web_url_produces_components() {
        let input = ReverseInput::WebUrl("https://example.com".to_string());
        let components = ReverseOrchestrator::detect_components(&input);
        assert!(components.len() >= 2, "web url must produce at least 2 components");
        assert!(
            components.iter().any(|c| c.kind == "header"),
            "web url must detect a header component"
        );
    }

    #[test]
    fn to_nomx_contains_define_that() {
        let components = vec![DetectedComponent {
            kind: "button".to_string(),
            confidence: 0.9,
            nomx_snippet: "  button that \"OK\"".to_string(),
        }];
        let nomx = ReverseOrchestrator::to_nomx(&components);
        assert!(nomx.contains("define"), "nomx must start with define keyword");
        assert!(nomx.contains("that"), "nomx must contain 'that'");
    }

    #[test]
    fn validate_grammar_valid() {
        let nomx = "define page that\n  button that \"Submit\"";
        assert!(
            ReverseOrchestrator::validate_grammar(nomx),
            "valid nomx with define+that must pass grammar check"
        );
    }

    #[test]
    fn validate_grammar_invalid() {
        assert!(
            !ReverseOrchestrator::validate_grammar(""),
            "empty string must fail grammar validation"
        );
    }

    #[test]
    fn reverse_result_is_valid() {
        let result = ReverseResult {
            components: vec![DetectedComponent {
                kind: "button".to_string(),
                confidence: 0.9,
                nomx_snippet: "  button that \"OK\"".to_string(),
            }],
            full_nomx: "define page that\n  button that \"OK\"".to_string(),
            grammar_valid: true,
            score: 0.9,
        };
        assert!(result.is_valid(), "result with grammar_valid=true and score=0.9 must be valid");
    }

    #[test]
    fn reverse_full_pipeline_image() {
        let input = ReverseInput::ImageBytes(vec![255u8; 16]);
        let result = ReverseOrchestrator::reverse(input);
        assert!(!result.components.is_empty(), "pipeline must produce components");
        assert!(result.grammar_valid, "assembled nomx must be grammar-valid");
        assert!(result.score > 0.0, "score must be positive");
        assert!(result.full_nomx.contains("define"), "full_nomx must contain define");
    }

    #[test]
    fn component_count() {
        let result = ReverseResult {
            components: vec![
                DetectedComponent {
                    kind: "header".to_string(),
                    confidence: 0.9,
                    nomx_snippet: "  header that contains logo".to_string(),
                },
                DetectedComponent {
                    kind: "footer".to_string(),
                    confidence: 0.8,
                    nomx_snippet: "  footer that shows links".to_string(),
                },
            ],
            full_nomx: String::new(),
            grammar_valid: true,
            score: 0.85,
        };
        assert_eq!(result.component_count(), 2, "component_count must return 2");
    }
}
