#![deny(unsafe_code)]

/// Which compose backend to route to — DB-driven at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendKind {
    Video, Audio, Image, Document, Data, App, Workflow, Scenario, RagQuery,
    Transform, EmbedGen, Render, Export, Pipeline, CodeExec, WebScreen,
}

impl BackendKind {
    pub fn from_kind_name(name: &str) -> Option<Self> {
        match name {
            "video" => Some(Self::Video),
            "audio" => Some(Self::Audio),
            "image" => Some(Self::Image),
            "document" => Some(Self::Document),
            "data" => Some(Self::Data),
            "app" => Some(Self::App),
            "workflow" => Some(Self::Workflow),
            "scenario" => Some(Self::Scenario),
            "rag_query" => Some(Self::RagQuery),
            "transform" => Some(Self::Transform),
            "embed_gen" => Some(Self::EmbedGen),
            "render" => Some(Self::Render),
            "export" => Some(Self::Export),
            "pipeline" => Some(Self::Pipeline),
            "code_exec" => Some(Self::CodeExec),
            "web_screen" => Some(Self::WebScreen),
            _ => None,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Video => "video", Self::Audio => "audio", Self::Image => "image",
            Self::Document => "document", Self::Data => "data", Self::App => "app",
            Self::Workflow => "workflow", Self::Scenario => "scenario",
            Self::RagQuery => "rag_query", Self::Transform => "transform",
            Self::EmbedGen => "embed_gen", Self::Render => "render",
            Self::Export => "export", Self::Pipeline => "pipeline",
            Self::CodeExec => "code_exec", Self::WebScreen => "web_screen",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dispatch_kind_from_name_roundtrip() {
        assert_eq!(BackendKind::from_kind_name("video"), Some(BackendKind::Video));
        assert_eq!(BackendKind::from_kind_name("unknown"), None);
    }
    #[test]
    fn dispatch_kind_name_matches_from_name() {
        let kind = BackendKind::Document;
        assert_eq!(BackendKind::from_kind_name(kind.name()), Some(kind));
    }
    #[test]
    fn all_16_backends_have_kind_names() {
        let names = ["video","audio","image","document","data","app","workflow","scenario",
                     "rag_query","transform","embed_gen","render","export","pipeline","code_exec","web_screen"];
        for name in names {
            assert!(BackendKind::from_kind_name(name).is_some(), "missing: {name}");
        }
    }
}
