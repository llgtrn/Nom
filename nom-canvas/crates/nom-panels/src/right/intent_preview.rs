/// Intent preview card — shows classified kind + confidence before composition runs.
#[derive(Debug, Clone)]
pub struct IntentPreviewCard {
    pub query: String,
    pub classified_kind: String,
    pub confidence: f32,
    pub top_alternatives: Vec<(String, f32)>, // (kind, score) pairs
    pub purpose_clause: Option<String>,
}

impl IntentPreviewCard {
    pub fn new(query: impl Into<String>, kind: impl Into<String>, confidence: f32) -> Self {
        Self {
            query: query.into(),
            classified_kind: kind.into(),
            confidence,
            top_alternatives: vec![],
            purpose_clause: None,
        }
    }

    pub fn with_alternatives(mut self, alts: Vec<(String, f32)>) -> Self {
        self.top_alternatives = alts;
        self
    }

    pub fn with_purpose(mut self, purpose: impl Into<String>) -> Self {
        self.purpose_clause = Some(purpose.into());
        self
    }

    pub fn confidence_label(&self) -> &str {
        match self.confidence {
            c if c >= 0.8 => "high",
            c if c >= 0.5 => "medium",
            _ => "low",
        }
    }
}

/// AI Review card — shows generated .nomx glue for user accept/reject.
#[derive(Debug, Clone)]
pub struct AiReviewCard {
    pub glue_hash: String,
    pub kind: String,
    pub nomx_preview: String, // first 200 chars of generated .nomx
    pub tier: String,         // "ai_leading" | "provider" | "db_driven"
    pub accepted: bool,
}

impl AiReviewCard {
    pub fn new(
        hash: impl Into<String>,
        kind: impl Into<String>,
        preview: impl Into<String>,
    ) -> Self {
        Self {
            glue_hash: hash.into(),
            kind: kind.into(),
            nomx_preview: preview.into(),
            tier: "ai_leading".into(),
            accepted: false,
        }
    }

    pub fn accept(&mut self) {
        self.accepted = true;
    }

    pub fn reject(&mut self) {
        self.accepted = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_preview_confidence_label_high() {
        let card = IntentPreviewCard::new("make a video", "video", 0.92);
        assert_eq!(card.confidence_label(), "high");
    }

    #[test]
    fn test_intent_preview_confidence_label_low() {
        let card = IntentPreviewCard::new("something vague", "data", 0.3);
        assert_eq!(card.confidence_label(), "low");
    }

    #[test]
    fn test_ai_review_card_accept() {
        let mut card = AiReviewCard::new("abc123", "video", "define clip that ...");
        assert!(!card.accepted, "card must start unaccepted");
        card.accept();
        assert!(card.accepted, "card must be accepted after accept()");
        card.reject();
        assert!(!card.accepted, "card must be rejected after reject()");
    }

    #[test]
    fn test_ai_review_card_with_alternatives() {
        let card = IntentPreviewCard::new("render scene", "image", 0.75)
            .with_alternatives(vec![("video".into(), 0.6), ("document".into(), 0.3)]);
        assert_eq!(card.top_alternatives.len(), 2);
        assert_eq!(card.top_alternatives[0].0, "video");
        assert!((card.top_alternatives[0].1 - 0.6).abs() < 1e-5);
        assert_eq!(card.confidence_label(), "medium");
    }
}
