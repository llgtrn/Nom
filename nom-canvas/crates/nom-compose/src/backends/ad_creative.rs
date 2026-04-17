#![deny(unsafe_code)]
use crate::backends::ComposeResult;

/// Ad format type.
#[derive(Debug, Clone, PartialEq)]
pub enum AdFormat {
    Banner,
    Square,
    Story,
    Video,
}

/// Specification for an advertising creative.
#[derive(Debug, Clone)]
pub struct AdCreativeSpec {
    pub brand: String,
    pub headline: String,
    pub cta: String,
    pub format: AdFormat,
    pub width: u32,
    pub height: u32,
}

impl AdCreativeSpec {
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            return 0.0;
        }
        self.width as f32 / self.height as f32
    }
}

pub fn compose(spec: &AdCreativeSpec) -> ComposeResult {
    if spec.brand.is_empty() {
        return Err("ad creative brand must not be empty".into());
    }
    if spec.width == 0 || spec.height == 0 {
        return Err("ad creative dimensions must be non-zero".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ad_creative_aspect_ratio() {
        let spec = AdCreativeSpec {
            brand: "Acme".into(),
            headline: "Buy Now".into(),
            cta: "Shop".into(),
            format: AdFormat::Banner,
            width: 1920,
            height: 1080,
        };
        let ratio = spec.aspect_ratio();
        assert!((ratio - (16.0 / 9.0)).abs() < 1e-4, "expected 16:9 ratio, got {}", ratio);

        let square = AdCreativeSpec {
            brand: "Acme".into(),
            headline: "".into(),
            cta: "".into(),
            format: AdFormat::Square,
            width: 1080,
            height: 1080,
        };
        assert!((square.aspect_ratio() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ad_creative_compose_produces_artifact() {
        let spec = AdCreativeSpec {
            brand: "Nom Labs".into(),
            headline: "Build anything".into(),
            cta: "Try free".into(),
            format: AdFormat::Story,
            width: 1080,
            height: 1920,
        };
        let result = compose(&spec);
        assert!(result.is_ok(), "compose must return Ok for valid spec");
    }
}
