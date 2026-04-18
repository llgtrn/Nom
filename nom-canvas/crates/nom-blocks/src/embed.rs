#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EmbedType {
    Web,
    Youtube,
    Figma,
    Tweet,
    Github,
    Generic,
}

impl EmbedType {
    pub fn from_url(url: &str) -> Self {
        if url.contains("youtube.com") || url.contains("youtu.be") {
            EmbedType::Youtube
        } else if url.contains("figma.com") {
            EmbedType::Figma
        } else if url.contains("twitter.com") || url.contains("x.com") {
            EmbedType::Tweet
        } else if url.contains("github.com") {
            EmbedType::Github
        } else {
            EmbedType::Web
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbedBlock {
    pub entity: NomtuRef,
    pub url: String,
    pub embed_type: EmbedType,
    pub aspect_ratio: f32,
    pub title: Option<String>,
}

impl EmbedBlock {
    pub fn new(entity: NomtuRef, url: impl Into<String>) -> Self {
        let url = url.into();
        let embed_type = EmbedType::from_url(&url);
        Self {
            entity,
            url,
            embed_type,
            aspect_ratio: 16.0 / 9.0,
            title: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embed_type_from_url() {
        assert_eq!(
            EmbedType::from_url("https://youtube.com/watch?v=abc"),
            EmbedType::Youtube
        );
        assert_eq!(
            EmbedType::from_url("https://figma.com/file/xyz"),
            EmbedType::Figma
        );
        assert_eq!(EmbedType::from_url("https://example.com"), EmbedType::Web);
    }

    #[test]
    fn embed_type_twitter_and_x() {
        assert_eq!(
            EmbedType::from_url("https://twitter.com/user/status/123"),
            EmbedType::Tweet
        );
        assert_eq!(
            EmbedType::from_url("https://x.com/user/status/456"),
            EmbedType::Tweet
        );
    }

    #[test]
    fn embed_type_github() {
        assert_eq!(
            EmbedType::from_url("https://github.com/org/repo"),
            EmbedType::Github
        );
    }

    #[test]
    fn embed_type_youtu_be_short() {
        assert_eq!(
            EmbedType::from_url("https://youtu.be/abc123"),
            EmbedType::Youtube
        );
    }

    #[test]
    fn embed_block_new_sets_aspect_ratio() {
        let entity = crate::block_model::NomtuRef::new("em-01", "embed", "concept");
        let block = EmbedBlock::new(entity, "https://example.com");
        // default aspect ratio is 16/9
        let expected = 16.0_f32 / 9.0_f32;
        assert!((block.aspect_ratio - expected).abs() < 0.001);
        assert!(block.title.is_none());
    }

    #[test]
    fn embed_block_url_preserved() {
        let entity = crate::block_model::NomtuRef::new("em-02", "embed", "concept");
        let url = "https://youtube.com/watch?v=xyz";
        let block = EmbedBlock::new(entity, url);
        assert_eq!(block.url, url);
        assert_eq!(block.embed_type, EmbedType::Youtube);
    }

    #[test]
    fn embed_block_entity_is_present() {
        let entity = crate::block_model::NomtuRef::new("em-03", "display", "verb");
        let block = EmbedBlock::new(entity, "https://figma.com/proto/abc");
        assert_eq!(block.entity.id, "em-03");
        assert_eq!(block.embed_type, EmbedType::Figma);
    }

    #[test]
    fn embed_type_generic_for_unknown_url() {
        assert_eq!(
            EmbedType::from_url("https://notion.so/page"),
            EmbedType::Web
        );
    }

    #[test]
    fn embed_type_from_empty_url_returns_web() {
        assert_eq!(EmbedType::from_url(""), EmbedType::Web);
    }

    #[test]
    fn embed_block_default_aspect_ratio_is_16_9() {
        let entity = crate::block_model::NomtuRef::new("em-04", "embed", "concept");
        let block = EmbedBlock::new(entity, "https://github.com/org/repo");
        assert!((block.aspect_ratio - (16.0 / 9.0)).abs() < 0.001);
    }

    #[test]
    fn embed_block_title_starts_as_none() {
        let entity = crate::block_model::NomtuRef::new("em-05", "embed", "concept");
        let block = EmbedBlock::new(entity, "https://example.com");
        assert!(block.title.is_none());
    }

    #[test]
    fn embed_block_github_type_detected() {
        let entity = crate::block_model::NomtuRef::new("em-06", "code", "concept");
        let block = EmbedBlock::new(entity, "https://github.com/rust-lang/rust");
        assert_eq!(block.embed_type, EmbedType::Github);
    }

    #[test]
    fn embed_type_all_variants_reachable() {
        assert_eq!(
            EmbedType::from_url("https://youtube.com/"),
            EmbedType::Youtube
        );
        assert_eq!(EmbedType::from_url("https://figma.com/"), EmbedType::Figma);
        assert_eq!(
            EmbedType::from_url("https://twitter.com/"),
            EmbedType::Tweet
        );
        assert_eq!(
            EmbedType::from_url("https://github.com/"),
            EmbedType::Github
        );
        assert_eq!(EmbedType::from_url("https://other.com/"), EmbedType::Web);
    }

    #[test]
    fn embed_block_entity_word_preserved() {
        let entity = crate::block_model::NomtuRef::new("em-07", "visualize", "verb");
        let block = EmbedBlock::new(entity, "https://example.com");
        assert_eq!(block.entity.word, "visualize");
    }

    #[test]
    fn embed_size_zero_aspect_ratio_allowed() {
        let entity = crate::block_model::NomtuRef::new("em-08", "embed", "concept");
        let mut block = EmbedBlock::new(entity, "https://example.com");
        block.aspect_ratio = 0.0;
        assert_eq!(block.aspect_ratio, 0.0);
    }

    #[test]
    fn embed_type_equality() {
        assert_eq!(EmbedType::Youtube, EmbedType::Youtube);
        assert_ne!(EmbedType::Youtube, EmbedType::Figma);
        assert_eq!(EmbedType::Generic, EmbedType::Generic);
    }

    #[test]
    fn embed_block_clone_preserves_url() {
        let entity = crate::block_model::NomtuRef::new("em-09", "embed", "concept");
        let block = EmbedBlock::new(entity, "https://youtu.be/test");
        let cloned = block.clone();
        assert_eq!(cloned.url, block.url);
        assert_eq!(cloned.embed_type, EmbedType::Youtube);
    }

    #[test]
    fn embed_block_with_title_set() {
        let entity = crate::block_model::NomtuRef::new("em-10", "embed", "concept");
        let mut block = EmbedBlock::new(entity, "https://example.com");
        block.title = Some("My Embed".to_string());
        assert_eq!(block.title.as_deref(), Some("My Embed"));
    }

    // ── wave AG-8: additional embed tests ────────────────────────────────────

    #[test]
    fn embed_url_detection_https() {
        // HTTPS URL must be detected as Web type (generic)
        let t = EmbedType::from_url("https://example.com/page");
        assert_eq!(t, EmbedType::Web);
    }

    #[test]
    fn embed_url_detection_http() {
        // HTTP URL also maps to Web
        let t = EmbedType::from_url("http://example.com/page");
        assert_eq!(t, EmbedType::Web);
    }

    #[test]
    fn embed_non_url_returns_web() {
        // A bare string with no recognized domain falls back to Web
        let t = EmbedType::from_url("just some text");
        assert_eq!(t, EmbedType::Web);
    }

    #[test]
    fn embed_type_youtube_detected() {
        let t = EmbedType::from_url("https://youtube.com/watch?v=abc");
        assert_eq!(t, EmbedType::Youtube);
    }

    #[test]
    fn embed_type_github_detected() {
        let t = EmbedType::from_url("https://github.com/owner/repo");
        assert_eq!(t, EmbedType::Github);
    }

    #[test]
    fn embed_type_generic_url_fallback() {
        // Unknown domain falls back to Web (the generic fallback)
        let t = EmbedType::from_url("https://unknownsite.io/path");
        assert_eq!(t, EmbedType::Web);
    }

    #[test]
    fn embed_metadata_title_nonempty_when_set() {
        let entity = crate::block_model::NomtuRef::new("em-11", "embed", "concept");
        let mut block = EmbedBlock::new(entity, "https://github.com/org/repo");
        block.title = Some("GitHub Repo".to_string());
        let title = block.title.as_deref().unwrap_or("");
        assert!(!title.is_empty());
    }

    #[test]
    fn embed_metadata_description_may_be_empty() {
        // title field starts as None — effectively empty description
        let entity = crate::block_model::NomtuRef::new("em-12", "embed", "concept");
        let block = EmbedBlock::new(entity, "https://example.com");
        assert!(block.title.is_none());
    }

    #[test]
    fn embed_equality_by_url() {
        // Two embed blocks with the same URL have the same embed_type
        let e1 = crate::block_model::NomtuRef::new("em-13a", "embed", "concept");
        let e2 = crate::block_model::NomtuRef::new("em-13b", "embed", "concept");
        let url = "https://youtube.com/watch?v=test";
        let b1 = EmbedBlock::new(e1, url);
        let b2 = EmbedBlock::new(e2, url);
        assert_eq!(b1.url, b2.url);
        assert_eq!(b1.embed_type, b2.embed_type);
    }

    #[test]
    fn embed_type_figma_detected() {
        let t = EmbedType::from_url("https://figma.com/design/abc");
        assert_eq!(t, EmbedType::Figma);
    }
}
