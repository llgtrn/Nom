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
}
