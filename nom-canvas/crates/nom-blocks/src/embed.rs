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
}
