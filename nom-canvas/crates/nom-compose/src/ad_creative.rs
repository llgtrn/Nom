#![deny(unsafe_code)]

/// The format of an ad creative.
#[derive(Debug, Clone, PartialEq)]
pub enum AdFormat {
    StaticImage,
    VideoAd,
    Interactive,
    Carousel,
}

impl AdFormat {
    /// Returns a human-readable name for the format.
    pub fn format_name(&self) -> &str {
        match self {
            AdFormat::StaticImage => "static-image",
            AdFormat::VideoAd => "video-ad",
            AdFormat::Interactive => "interactive",
            AdFormat::Carousel => "carousel",
        }
    }

    /// Returns true if this format requires motion (animation or interactivity).
    pub fn requires_motion(&self) -> bool {
        matches!(self, AdFormat::VideoAd | AdFormat::Interactive)
    }
}

/// Pixel dimensions for an ad creative.
#[derive(Debug, Clone, PartialEq)]
pub struct AdDimension {
    pub width: u32,
    pub height: u32,
}

impl AdDimension {
    /// Creates a new `AdDimension`.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns width / height as a float. Returns 0.0 if height is zero.
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            return 0.0;
        }
        self.width as f32 / self.height as f32
    }

    /// Returns true when width equals height.
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }

    /// Returns a `"WxH"` label string.
    pub fn label(&self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

/// Full specification for an ad creative.
#[derive(Debug, Clone)]
pub struct AdCreativeSpec {
    pub title: String,
    pub format: AdFormat,
    pub dimension: AdDimension,
    pub cta: String,
}

impl AdCreativeSpec {
    /// Constructs a new `AdCreativeSpec`.
    pub fn new(title: String, format: AdFormat, dimension: AdDimension, cta: String) -> Self {
        Self {
            title,
            format,
            dimension,
            cta,
        }
    }

    /// Returns true when the underlying format requires motion.
    pub fn requires_motion(&self) -> bool {
        self.format.requires_motion()
    }

    /// Returns a one-line summary: `"<title> [<format_name>] <dimension_label>"`.
    pub fn summary(&self) -> String {
        format!(
            "{} [{}] {}",
            self.title,
            self.format.format_name(),
            self.dimension.label()
        )
    }
}

/// Composes ad creatives from intent.
pub struct AdComposer;

impl AdComposer {
    /// Creates a new `AdComposer`.
    pub fn new() -> Self {
        Self
    }

    /// Produces a 1200×628 static-image creative.
    pub fn compose_static(&self, title: &str, cta: &str) -> AdCreativeSpec {
        AdCreativeSpec::new(
            title.to_string(),
            AdFormat::StaticImage,
            AdDimension::new(1200, 628),
            cta.to_string(),
        )
    }

    /// Produces a 1920×1080 video-ad creative.
    pub fn compose_video(&self, title: &str, cta: &str) -> AdCreativeSpec {
        AdCreativeSpec::new(
            title.to_string(),
            AdFormat::VideoAd,
            AdDimension::new(1920, 1080),
            cta.to_string(),
        )
    }

    /// Produces a 1080×1080 static-image square creative.
    pub fn compose_square(&self, title: &str, cta: &str) -> AdCreativeSpec {
        AdCreativeSpec::new(
            title.to_string(),
            AdFormat::StaticImage,
            AdDimension::new(1080, 1080),
            cta.to_string(),
        )
    }

    /// Returns `[static, video, square]` creatives for the given intent.
    pub fn compose_all(&self, title: &str, cta: &str) -> Vec<AdCreativeSpec> {
        vec![
            self.compose_static(title, cta),
            self.compose_video(title, cta),
            self.compose_square(title, cta),
        ]
    }
}

impl Default for AdComposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod ad_creative_tests {
    use super::*;

    #[test]
    fn ad_format_format_name() {
        assert_eq!(AdFormat::StaticImage.format_name(), "static-image");
        assert_eq!(AdFormat::VideoAd.format_name(), "video-ad");
        assert_eq!(AdFormat::Interactive.format_name(), "interactive");
        assert_eq!(AdFormat::Carousel.format_name(), "carousel");
    }

    #[test]
    fn ad_format_requires_motion() {
        assert!(!AdFormat::StaticImage.requires_motion());
        assert!(AdFormat::VideoAd.requires_motion());
        assert!(AdFormat::Interactive.requires_motion());
        assert!(!AdFormat::Carousel.requires_motion());
    }

    #[test]
    fn ad_dimension_aspect_ratio() {
        let dim = AdDimension::new(1920, 1080);
        let ratio = dim.aspect_ratio();
        assert!(
            (ratio - (16.0 / 9.0)).abs() < 1e-4,
            "expected 16:9 ratio, got {}",
            ratio
        );
        let zero = AdDimension::new(100, 0);
        assert_eq!(zero.aspect_ratio(), 0.0);
    }

    #[test]
    fn ad_dimension_is_square() {
        assert!(AdDimension::new(1080, 1080).is_square());
        assert!(!AdDimension::new(1200, 628).is_square());
    }

    #[test]
    fn ad_dimension_label() {
        assert_eq!(AdDimension::new(1200, 628).label(), "1200x628");
        assert_eq!(AdDimension::new(1080, 1080).label(), "1080x1080");
    }

    #[test]
    fn ad_creative_spec_requires_motion() {
        let spec_static = AdCreativeSpec::new(
            "Promo".to_string(),
            AdFormat::StaticImage,
            AdDimension::new(1200, 628),
            "Buy now".to_string(),
        );
        let spec_video = AdCreativeSpec::new(
            "Promo".to_string(),
            AdFormat::VideoAd,
            AdDimension::new(1920, 1080),
            "Watch".to_string(),
        );
        assert!(!spec_static.requires_motion());
        assert!(spec_video.requires_motion());
    }

    #[test]
    fn ad_composer_compose_static_dimension() {
        let composer = AdComposer::new();
        let spec = composer.compose_static("Summer Sale", "Shop Now");
        assert_eq!(spec.dimension.width, 1200);
        assert_eq!(spec.dimension.height, 628);
        assert_eq!(spec.format, AdFormat::StaticImage);
        assert_eq!(spec.title, "Summer Sale");
        assert_eq!(spec.cta, "Shop Now");
    }

    #[test]
    fn ad_composer_compose_all_count() {
        let composer = AdComposer::new();
        let all = composer.compose_all("Launch", "Try free");
        assert_eq!(all.len(), 3, "compose_all must return exactly 3 creatives");
        assert_eq!(all[0].format, AdFormat::StaticImage);
        assert_eq!(all[1].format, AdFormat::VideoAd);
        assert_eq!(all[2].format, AdFormat::StaticImage);
        assert!(all[2].dimension.is_square());
    }

    #[test]
    fn ad_creative_spec_summary() {
        let composer = AdComposer::new();
        let spec = composer.compose_video("Brand Story", "Learn More");
        let summary = spec.summary();
        assert!(
            summary.contains("Brand Story"),
            "summary must contain title, got: {summary}"
        );
        assert!(
            summary.contains("video-ad"),
            "summary must contain format name, got: {summary}"
        );
        assert!(
            summary.contains("1920x1080"),
            "summary must contain dimension label, got: {summary}"
        );
    }
}
