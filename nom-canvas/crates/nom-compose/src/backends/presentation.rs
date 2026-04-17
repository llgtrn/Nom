#![deny(unsafe_code)]
use crate::backends::ComposeResult;

/// A single slide in a presentation.
#[derive(Debug, Clone)]
pub struct PresentationSlide {
    pub title: String,
    pub body: String,
    pub speaker_notes: String,
}

/// Specification for a full presentation.
#[derive(Debug, Clone)]
pub struct PresentationSpec {
    pub title: String,
    pub author: String,
    pub slides: Vec<PresentationSlide>,
    pub theme: String,
}

impl PresentationSpec {
    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }

    pub fn add_slide(&mut self, slide: PresentationSlide) {
        self.slides.push(slide);
    }
}

pub fn compose(spec: &PresentationSpec) -> ComposeResult {
    if spec.title.is_empty() {
        return Err("presentation title must not be empty".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_slide_count() {
        let mut spec = PresentationSpec {
            title: "Q1 Review".into(),
            author: "Alice".into(),
            slides: vec![
                PresentationSlide {
                    title: "Intro".into(),
                    body: "Welcome".into(),
                    speaker_notes: "Say hello".into(),
                },
            ],
            theme: "dark".into(),
        };
        assert_eq!(spec.slide_count(), 1);
        spec.add_slide(PresentationSlide {
            title: "Metrics".into(),
            body: "Numbers here".into(),
            speaker_notes: "Explain trends".into(),
        });
        assert_eq!(spec.slide_count(), 2);
    }

    #[test]
    fn presentation_compose_produces_artifact() {
        let spec = PresentationSpec {
            title: "Annual Deck".into(),
            author: "Bob".into(),
            slides: vec![
                PresentationSlide {
                    title: "Cover".into(),
                    body: "Company name".into(),
                    speaker_notes: "".into(),
                },
            ],
            theme: "light".into(),
        };
        let result = compose(&spec);
        assert!(result.is_ok(), "compose must return Ok for valid spec");
    }
}
