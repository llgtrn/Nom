use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{NOMX, NOTE};

/// Which Nom dialect the source is written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NomxLang {
    /// `.nom` source files (concept-level).
    Nom,
    /// `.nomx` source files (expression-level / inline).
    Nomx,
}

/// Props for a code block that holds Nom/Nomx source.
#[derive(Debug, Clone)]
pub struct NomxProps {
    pub source: String,
    pub lang: NomxLang,
    pub wrap: bool,
    pub caption: Option<String>,
    pub line_numbers: bool,
}

impl Default for NomxProps {
    fn default() -> Self {
        Self {
            source: String::new(),
            lang: NomxLang::Nomx,
            wrap: false,
            caption: None,
            line_numbers: true,
        }
    }
}

/// Static schema for Nom/Nomx source blocks.
pub fn nomx_schema() -> BlockSchema {
    BlockSchema {
        flavour: NOMX,
        version: 1,
        role: Role::Content,
        parents: &[NOTE],
        children: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::{validate_child, validate_parent};
    use crate::flavour::NOTE;

    #[test]
    fn schema_defaults_sensibly() {
        let s = nomx_schema();
        assert_eq!(s.flavour, NOMX);
        assert_eq!(s.version, 1);
        assert_eq!(s.role, Role::Content);
        assert!(validate_parent(&s, NOTE).is_ok());
        assert!(validate_child(&s, NOTE).is_err());
    }

    #[test]
    fn source_persists() {
        let mut props = NomxProps::default();
        props.source = "define add that x + y".to_string();
        props.lang = NomxLang::Nom;
        assert_eq!(props.source, "define add that x + y");
        assert_eq!(props.lang, NomxLang::Nom);
        assert!(props.caption.is_none());
    }
}
