#![deny(unsafe_code)]
use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{NOTE, SURFACE};
use crate::media::{BlobId, FractionalIndex};

#[derive(Clone, Debug, PartialEq)]
pub enum AppKind {
    Web { url: String },
    Native { binary_path: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppBlockProps {
    pub source_id: BlobId,
    pub kind: AppKind,
    pub title: String,
    pub description: Option<String>,
    pub index: FractionalIndex,
}

impl AppBlockProps {
    pub fn new(source_id: BlobId, kind: AppKind, title: impl Into<String>) -> Self {
        Self {
            source_id,
            kind,
            title: title.into(),
            description: None,
            index: "a0".to_owned(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn is_web(&self) -> bool {
        matches!(self.kind, AppKind::Web { .. })
    }

    pub fn is_native(&self) -> bool {
        matches!(self.kind, AppKind::Native { .. })
    }
}

pub fn app_block_schema() -> BlockSchema {
    BlockSchema {
        flavour: crate::compose::COMPOSE_APP,
        version: 1,
        role: Role::Content,
        parents: &[NOTE, SURFACE],
        children: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::Role;

    #[test]
    fn web_variant() {
        let app = AppBlockProps::new(
            "blob-app".to_owned(),
            AppKind::Web { url: "https://example.com".to_owned() },
            "My App",
        );
        assert!(app.is_web());
        assert!(!app.is_native());
        assert_eq!(app.title, "My App");
    }

    #[test]
    fn native_variant() {
        let app = AppBlockProps::new(
            "blob-bin".to_owned(),
            AppKind::Native { binary_path: "/usr/bin/myapp".to_owned() },
            "Native App",
        );
        assert!(app.is_native());
        assert!(!app.is_web());
    }

    #[test]
    fn kind_discriminator() {
        let web = AppKind::Web { url: "https://a.com".to_owned() };
        let native = AppKind::Native { binary_path: "/bin/app".to_owned() };
        assert_ne!(web, native);
    }

    #[test]
    fn with_description_sets_field() {
        let app = AppBlockProps::new(
            "b".to_owned(),
            AppKind::Web { url: "https://x.com".to_owned() },
            "X",
        )
        .with_description("A web app");
        assert_eq!(app.description.as_deref(), Some("A web app"));
    }

    #[test]
    fn schema_role_content() {
        assert_eq!(app_block_schema().role, Role::Content);
    }
}
