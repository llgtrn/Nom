#![deny(unsafe_code)]
use crate::ui_tier::CompileStatus;
use crate::shared::SharedState;

/// Score a word+kind and return the compile status badge
pub fn score_to_status(word: &str, kind: &str, _state: &SharedState) -> CompileStatus {
    #[cfg(feature = "compiler")]
    {
        use nom_types::{Atom, AtomKind};
        let atom = Atom {
            id: word.to_string(),
            kind: AtomKind::Function,
            name: word.to_string(),
            source_path: String::new(),
            language: "nom".to_string(),
            labels: vec![],
            concept: Some(kind.to_string()),
            signature: None,
            body: None,
        };
        let scores = nom_score::score_atom(&atom);
        CompileStatus::from_score(scores.overall())
    }
    #[cfg(not(feature = "compiler"))]
    {
        let _ = (word, kind);
        CompileStatus::NotChecked
    }
}

/// Score label for status bar display
pub fn status_label(status: &CompileStatus) -> &'static str {
    status.label()
}

/// Color hint for status (as [h,s,l,a] for nom-gpui Hsla)
pub fn status_color(status: &CompileStatus) -> [f32; 4] {
    match status {
        CompileStatus::Valid => [0.397, 0.63, 0.49, 1.0],          // green: accent-green
        CompileStatus::LowConfidence => [0.105, 0.921, 0.502, 1.0], // amber
        CompileStatus::Unknown => [0.0, 0.842, 0.602, 1.0],         // red
        CompileStatus::NotChecked => [0.0, 0.0, 0.45, 1.0],         // gray
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_label_strings() {
        assert_eq!(status_label(&CompileStatus::Valid), "Valid");
        assert_eq!(status_label(&CompileStatus::LowConfidence), "Low confidence");
        assert_eq!(status_label(&CompileStatus::Unknown), "Unknown");
    }

    #[test]
    fn score_to_status_not_checked_without_feature() {
        let state = SharedState::new("a.db", "b.db");
        let status = score_to_status("summarize", "verb", &state);
        // Without compiler feature, always NotChecked
        assert_eq!(status, CompileStatus::NotChecked);
    }
}
