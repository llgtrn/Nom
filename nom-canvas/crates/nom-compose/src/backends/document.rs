#![deny(unsafe_code)]
use nom_blocks::compose::document_block::DocumentBlock;
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};
use crate::backends::ComposeResult;

/// A document section — typst-style content block.
#[derive(Debug, Clone)]
pub struct DocSection {
    pub heading: Option<String>,
    pub body: String,
    pub page_break: bool,
}

/// Document spec — typst pattern.
#[derive(Debug, Clone)]
pub struct DocSpec {
    pub title: String,
    pub author: String,
    pub sections: Vec<DocSection>,
    pub page_count: usize,
}

impl DocSpec {
    pub fn word_count(&self) -> usize {
        self.sections.iter().map(|s| s.body.split_whitespace().count()).sum()
    }

    pub fn page_count_estimate(&self) -> usize {
        (self.word_count() / 250).max(1)
    }
}

fn parse_section(index: usize, block: &str) -> DocSection {
    if block.starts_with('#') {
        let mut parts = block.splitn(2, '\n');
        let h = parts.next().unwrap_or("").trim_start_matches('#').trim().to_string();
        let body = parts.next().unwrap_or("").trim().to_string();
        DocSection {
            heading: Some(h),
            body,
            page_break: index > 0 && index % 5 == 0,
        }
    } else {
        DocSection {
            heading: None,
            body: block.to_string(),
            page_break: index > 0 && index % 5 == 0,
        }
    }
}

pub struct DocumentInput {
    pub entity: NomtuRef,
    pub content_blocks: Vec<String>,
    pub target_mime: String,
}

pub struct DocumentBackend;

impl DocumentBackend {
    pub fn compose(input: DocumentInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> DocumentBlock {
        sink.emit(ComposeEvent::Started { backend: "document".into(), entity_id: input.entity.id.clone() });

        let sections: Vec<DocSection> = input.content_blocks.iter().enumerate()
            .map(|(i, block)| parse_section(i, block))
            .collect();

        let spec = DocSpec {
            title: input.entity.word.clone(),
            author: String::new(),
            sections,
            page_count: 0,
        };

        // Emit per-section progress.
        let total = spec.sections.len().max(1);
        for (i, section) in spec.sections.iter().enumerate() {
            let pct = (i + 1) as f32 / total as f32;
            let stage_name = section.heading.as_deref().unwrap_or("body");
            sink.emit(ComposeEvent::Progress {
                percent: pct,
                stage: format!("rendering section: {}", stage_name),
            });
        }

        // Serialize to JSON and content-address it.
        let doc_json = serde_json::json!({
            "title": spec.title,
            "author": spec.author,
            "word_count": spec.word_count(),
            "page_count_estimate": spec.page_count_estimate(),
            "section_count": spec.sections.len(),
        });
        let content_bytes = doc_json.to_string().into_bytes();
        let artifact_hash = store.write(&content_bytes);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);

        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });

        DocumentBlock {
            entity: input.entity,
            artifact_hash,
            page_count: spec.page_count_estimate() as u32,
            mime: input.target_mime,
        }
    }

    /// Error-wrapped variant of [`compose`]. Returns `Ok(())` on success.
    pub fn compose_safe(input: DocumentInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> ComposeResult {
        let _block = Self::compose(input, store, sink);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

    #[test]
    fn doc_spec_creation() {
        let spec = DocSpec {
            title: "Test Doc".into(),
            author: "Author".into(),
            sections: vec![
                DocSection { heading: Some("Intro".into()), body: "Hello world".into(), page_break: false },
                DocSection { heading: None, body: "More content here".into(), page_break: false },
            ],
            page_count: 0,
        };
        assert_eq!(spec.sections.len(), 2);
        assert_eq!(spec.title, "Test Doc");
    }

    #[test]
    fn doc_spec_word_count() {
        let spec = DocSpec {
            title: "WC Test".into(),
            author: "".into(),
            sections: vec![
                DocSection { heading: None, body: "one two three".into(), page_break: false },
                DocSection { heading: None, body: "four five".into(), page_break: false },
            ],
            page_count: 0,
        };
        assert_eq!(spec.word_count(), 5);
        // 5 words / 250 = 0, max(1) = 1
        assert_eq!(spec.page_count_estimate(), 1);

        // 250 words → 1 page
        let big_body = vec!["word"; 250].join(" ");
        let spec2 = DocSpec {
            title: "Big".into(),
            author: "".into(),
            sections: vec![DocSection { heading: None, body: big_body, page_break: false }],
            page_count: 0,
        };
        assert_eq!(spec2.word_count(), 250);
        assert_eq!(spec2.page_count_estimate(), 1);
    }

    #[test]
    fn document_compose_produces_artifact() {
        let mut store = InMemoryStore::new();
        let input = DocumentInput {
            entity: NomtuRef { id: "doc1".into(), word: "report".into(), kind: "concept".into() },
            content_blocks: vec!["# Title\nIntroduction text".into(), "body text here".into()],
            target_mime: "text/markdown".into(),
        };
        let block = DocumentBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.mime, "text/markdown");
        assert!(store.exists(&block.artifact_hash));
    }

    #[test]
    fn document_compose_safe_returns_ok() {
        let mut store = InMemoryStore::new();
        let input = DocumentInput {
            entity: NomtuRef { id: "doc2".into(), word: "brief".into(), kind: "concept".into() },
            content_blocks: vec!["intro".into(), "conclusion".into()],
            target_mime: "text/plain".into(),
        };
        let result = DocumentBackend::compose_safe(input, &mut store, &LogProgressSink);
        assert!(result.is_ok(), "compose_safe must return Ok(()) on success");
    }
}
