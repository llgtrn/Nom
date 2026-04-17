//! NomKind — the set of composition targets this crate understands.

/// Every composition request carries exactly one kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NomKind {
    MediaVideo,
    MediaImage,
    MediaAudio,
    Media3D,
    MediaStoryboard,
    MediaNovelVideo,
    ScreenWeb,
    ScreenNative,
    DataExtract,
    DataQuery,
    DataTransform,
    ConceptDocument,
    ScenarioWorkflow,
}

impl NomKind {
    /// Canonical string form used in routing tables and logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            NomKind::MediaVideo => "media/video",
            NomKind::MediaImage => "media/image",
            NomKind::MediaAudio => "media/audio",
            NomKind::Media3D => "media/3d",
            NomKind::MediaStoryboard => "media/storyboard",
            NomKind::MediaNovelVideo => "media/novel-video",
            NomKind::ScreenWeb => "screen/web",
            NomKind::ScreenNative => "screen/native",
            NomKind::DataExtract => "data/extract",
            NomKind::DataQuery => "data/query",
            NomKind::DataTransform => "data/transform",
            NomKind::ConceptDocument => "concept/document",
            NomKind::ScenarioWorkflow => "scenario/workflow",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_video_str() {
        assert_eq!(NomKind::MediaVideo.as_str(), "media/video");
    }

    #[test]
    fn screen_native_str() {
        assert_eq!(NomKind::ScreenNative.as_str(), "screen/native");
    }

    #[test]
    fn all_kinds_have_slash() {
        let kinds = [
            NomKind::MediaVideo,
            NomKind::MediaImage,
            NomKind::MediaAudio,
            NomKind::Media3D,
            NomKind::MediaStoryboard,
            NomKind::MediaNovelVideo,
            NomKind::ScreenWeb,
            NomKind::ScreenNative,
            NomKind::DataExtract,
            NomKind::DataQuery,
            NomKind::DataTransform,
            NomKind::ConceptDocument,
            NomKind::ScenarioWorkflow,
        ];
        for k in &kinds {
            assert!(k.as_str().contains('/'), "{:?} missing slash", k);
        }
    }
}
