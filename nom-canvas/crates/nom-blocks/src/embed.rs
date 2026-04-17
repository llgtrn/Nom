//! External-content embed block (iframe, bookmark, linked/synced doc, youtube, figma).
#![deny(unsafe_code)]

use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{EMBED, NOTE, SURFACE};

pub type BlobId = String;

#[derive(Clone, Debug, PartialEq)]
pub enum EmbedKind {
    Iframe,
    Bookmark,
    LinkedDoc { target_doc_id: String },
    SyncedDoc { target_doc_id: String },
    Youtube { video_id: String },
    Figma { file_key: String, node_id: Option<String> },
    Generic,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmbedProps {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<BlobId>,
    pub kind: EmbedKind,
}

impl EmbedProps {
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        let kind = detect_kind(&url);
        EmbedProps {
            url,
            title: None,
            description: None,
            thumbnail: None,
            kind,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_thumbnail(mut self, thumb: impl Into<BlobId>) -> Self {
        self.thumbnail = Some(thumb.into());
        self
    }
}

/// Decode only `%3A` → `:` in a query-parameter value (Figma node-id specific).
fn decode_figma_node_id(s: &str) -> String {
    s.replace("%3A", ":").replace("%3a", ":")
}

/// Extract the value of a query parameter from a URL query string.
/// `query` is the part after `?` (and before `#` if any).
fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    for part in query.split('&') {
        if let Some(rest) = part.strip_prefix(key) {
            if let Some(val) = rest.strip_prefix('=') {
                return Some(val);
            }
        }
    }
    None
}

/// Detect the embed kind from a URL string.
///
/// Rules (checked in order; first match wins):
///   - `youtube.com/watch?v=XXX` → Youtube { video_id }
///   - `youtu.be/XXX` → Youtube { video_id }
///   - `figma.com/file/KEY/...` → Figma { file_key, node_id from ?node-id= }
///   - URL starts with `nom://doc/` → LinkedDoc
///   - URL starts with `nom://synced/` → SyncedDoc
///   - Otherwise → Bookmark
pub fn detect_kind(url: &str) -> EmbedKind {
    if url.is_empty() {
        return EmbedKind::Bookmark;
    }

    // Split off the scheme+host portion from path+query
    // Strip common scheme prefixes to get host+path
    let without_scheme = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else if url.starts_with("nom://doc/") {
        let target = &url["nom://doc/".len()..];
        return EmbedKind::LinkedDoc { target_doc_id: target.to_string() };
    } else if url.starts_with("nom://synced/") {
        let target = &url["nom://synced/".len()..];
        return EmbedKind::SyncedDoc { target_doc_id: target.to_string() };
    } else {
        return EmbedKind::Bookmark;
    };

    // Split host from path?query
    let (host, path_and_query) = without_scheme
        .split_once('/')
        .unwrap_or((without_scheme, ""));

    // Split path from query
    let (path, query) = path_and_query
        .split_once('?')
        .unwrap_or((path_and_query, ""));

    // Normalize host: strip www. prefix
    let host = host.strip_prefix("www.").unwrap_or(host);
    // Strip port if present
    let host = host.split(':').next().unwrap_or(host);

    // --- YouTube ---
    if host == "youtube.com" || host == "m.youtube.com" {
        if path == "watch" {
            if let Some(v) = query_param(query, "v") {
                return EmbedKind::Youtube { video_id: v.to_string() };
            }
        }
    }

    if host == "youtu.be" {
        // path_and_query starts with the video id
        let video_id = path.trim_matches('/');
        if !video_id.is_empty() {
            return EmbedKind::Youtube { video_id: video_id.to_string() };
        }
    }

    // --- Figma ---
    if host == "figma.com" {
        // path looks like: file/KEY/optional-title
        let mut segments = path.splitn(3, '/');
        if segments.next() == Some("file") {
            if let Some(file_key) = segments.next() {
                if !file_key.is_empty() {
                    let node_id = query_param(query, "node-id")
                        .map(|v| decode_figma_node_id(v));
                    return EmbedKind::Figma {
                        file_key: file_key.to_string(),
                        node_id,
                    };
                }
            }
        }
    }

    EmbedKind::Bookmark
}

pub fn embed_schema() -> BlockSchema {
    BlockSchema {
        flavour: EMBED,
        version: 1,
        role: Role::Content,
        parents: &[NOTE, SURFACE],
        children: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_watch_url() {
        assert_eq!(
            detect_kind("https://www.youtube.com/watch?v=abc123"),
            EmbedKind::Youtube { video_id: "abc123".to_string() }
        );
    }

    #[test]
    fn youtube_watch_url_no_www() {
        assert_eq!(
            detect_kind("https://youtube.com/watch?v=dQw4w9WgXcQ"),
            EmbedKind::Youtube { video_id: "dQw4w9WgXcQ".to_string() }
        );
    }

    #[test]
    fn youtu_be_short_url() {
        assert_eq!(
            detect_kind("https://youtu.be/xyz"),
            EmbedKind::Youtube { video_id: "xyz".to_string() }
        );
    }

    #[test]
    fn figma_with_node_id_percent_encoded() {
        assert_eq!(
            detect_kind("https://www.figma.com/file/ABCDEF/Design?node-id=1%3A2"),
            EmbedKind::Figma {
                file_key: "ABCDEF".to_string(),
                node_id: Some("1:2".to_string()),
            }
        );
    }

    #[test]
    fn figma_without_node_id() {
        assert_eq!(
            detect_kind("https://www.figma.com/file/GHI"),
            EmbedKind::Figma {
                file_key: "GHI".to_string(),
                node_id: None,
            }
        );
    }

    #[test]
    fn nom_linked_doc() {
        assert_eq!(
            detect_kind("nom://doc/foo"),
            EmbedKind::LinkedDoc { target_doc_id: "foo".to_string() }
        );
    }

    #[test]
    fn nom_synced_doc() {
        assert_eq!(
            detect_kind("nom://synced/bar"),
            EmbedKind::SyncedDoc { target_doc_id: "bar".to_string() }
        );
    }

    #[test]
    fn generic_https_url_is_bookmark() {
        assert_eq!(
            detect_kind("https://example.com"),
            EmbedKind::Bookmark
        );
    }

    #[test]
    fn empty_string_is_bookmark() {
        assert_eq!(detect_kind(""), EmbedKind::Bookmark);
    }

    #[test]
    fn embed_props_new_calls_detect_kind() {
        let props = EmbedProps::new("https://youtu.be/test99");
        assert_eq!(props.kind, EmbedKind::Youtube { video_id: "test99".to_string() });
        assert_eq!(props.url, "https://youtu.be/test99");
        assert!(props.title.is_none());
        assert!(props.description.is_none());
        assert!(props.thumbnail.is_none());
    }

    #[test]
    fn embed_props_builders_chain() {
        let props = EmbedProps::new("https://example.com")
            .with_title("My Title")
            .with_description("A description")
            .with_thumbnail("blob-id-42");
        assert_eq!(props.title, Some("My Title".to_string()));
        assert_eq!(props.description, Some("A description".to_string()));
        assert_eq!(props.thumbnail, Some("blob-id-42".to_string()));
    }

    #[test]
    fn embed_schema_role_is_content() {
        let schema = embed_schema();
        assert_eq!(schema.role, Role::Content);
    }

    #[test]
    fn embed_schema_parents_contain_note_and_surface() {
        let schema = embed_schema();
        assert!(schema.parents.contains(&NOTE));
        assert!(schema.parents.contains(&SURFACE));
    }

    #[test]
    fn figma_node_id_lowercase_percent() {
        assert_eq!(
            detect_kind("https://figma.com/file/XYZ/page?node-id=2%3a5"),
            EmbedKind::Figma {
                file_key: "XYZ".to_string(),
                node_id: Some("2:5".to_string()),
            }
        );
    }

    #[test]
    fn http_url_is_bookmark() {
        assert_eq!(detect_kind("http://example.org/page"), EmbedKind::Bookmark);
    }
}
