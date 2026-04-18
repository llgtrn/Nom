/// A UI component detected from an image/screenshot.
#[derive(Debug, Clone)]
pub struct UiComponent {
    pub component_type: UiComponentType,
    pub confidence: f32,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiComponentType {
    Button,
    Card,
    NavigationBar,
    Form,
    Input,
    Image,
    Text,
    Icon,
    List,
    Modal,
    Tab,
    Toggle,
    Unknown,
}

impl UiComponentType {
    pub fn from_label(label: &str) -> Self {
        match label.to_lowercase().as_str() {
            s if s.contains("button") || s.contains("btn") => Self::Button,
            s if s.contains("card") => Self::Card,
            s if s.contains("nav") || s.contains("menu") || s.contains("header") => Self::NavigationBar,
            s if s.contains("form") => Self::Form,
            s if s.contains("input") || s.contains("text field") => Self::Input,
            s if s.contains("image") || s.contains("img") || s.contains("photo") => Self::Image,
            s if s.contains("icon") => Self::Icon,
            s if s.contains("list") || s.contains("item") => Self::List,
            s if s.contains("modal") || s.contains("dialog") => Self::Modal,
            s if s.contains("tab") => Self::Tab,
            s if s.contains("toggle") || s.contains("switch") => Self::Toggle,
            s if s.contains("text") || s.contains("label") || s.contains("heading") => Self::Text,
            _ => Self::Unknown,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Button => "Button",
            Self::Card => "Card",
            Self::NavigationBar => "NavigationBar",
            Self::Form => "Form",
            Self::Input => "Input",
            Self::Image => "Image",
            Self::Text => "Text",
            Self::Icon => "Icon",
            Self::List => "List",
            Self::Modal => "Modal",
            Self::Tab => "Tab",
            Self::Toggle => "Toggle",
            Self::Unknown => "Unknown",
        }
    }
}

/// Image encoding for vision API calls.
#[derive(Debug, Clone)]
pub struct EncodedImage {
    pub base64_data: String,
    pub mime_type: String, // "image/png", "image/jpeg"
    pub width_hint: Option<u32>,
    pub height_hint: Option<u32>,
}

impl EncodedImage {
    pub fn from_bytes(bytes: &[u8], mime_type: &str) -> Self {
        use std::fmt::Write;
        let mut b64 = String::with_capacity(bytes.len() * 4 / 3 + 4);
        // Simple base64 stub (encodes to hex for test purposes)
        for byte in bytes {
            write!(b64, "{:02x}", byte).unwrap();
        }
        Self {
            base64_data: b64,
            mime_type: mime_type.into(),
            width_hint: None,
            height_hint: None,
        }
    }

    pub fn data_url(&self) -> String {
        format!("data:{};base64,{}", self.mime_type, self.base64_data)
    }
}

/// Result from vision analysis.
#[derive(Debug, Clone)]
pub struct VisionAnalysisResult {
    pub components: Vec<UiComponent>,
    pub generated_html: Option<String>,
    pub layout_description: String,
    pub confidence_avg: f32,
}

impl VisionAnalysisResult {
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn components_by_type(&self, t: &UiComponentType) -> Vec<&UiComponent> {
        self.components.iter().filter(|c| &c.component_type == t).collect()
    }
}

/// Trait for vision analysis providers (SAM, YOLO, LLM, etc.)
pub trait VisionProvider {
    fn analyze(&self, image: &EncodedImage) -> VisionAnalysisResult;
    fn provider_name(&self) -> &'static str;
}

/// Stub provider that parses component labels from a description string.
pub struct StubVisionProvider {
    pub component_labels: Vec<String>,
}

impl StubVisionProvider {
    pub fn new(labels: Vec<&str>) -> Self {
        Self { component_labels: labels.into_iter().map(String::from).collect() }
    }
}

impl VisionProvider for StubVisionProvider {
    fn analyze(&self, _image: &EncodedImage) -> VisionAnalysisResult {
        let components: Vec<UiComponent> = self.component_labels.iter().enumerate().map(|(i, label)| {
            UiComponent {
                component_type: UiComponentType::from_label(label),
                confidence: 0.9,
                label: label.clone(),
                x: i as f32 * 100.0,
                y: 0.0,
                width: 80.0,
                height: 40.0,
            }
        }).collect();
        let confidence_avg = if components.is_empty() { 0.0 } else {
            components.iter().map(|c| c.confidence).sum::<f32>() / components.len() as f32
        };
        VisionAnalysisResult {
            components,
            generated_html: None,
            layout_description: format!("{} components detected", self.component_labels.len()),
            confidence_avg,
        }
    }

    fn provider_name(&self) -> &'static str { "stub" }
}

/// Screenshot analyzer — orchestrates vision → component list → nomx.
pub struct ScreenshotAnalyzer {
    pub provider: Box<dyn VisionProvider>,
}

impl ScreenshotAnalyzer {
    pub fn new(provider: Box<dyn VisionProvider>) -> Self { Self { provider } }

    pub fn analyze(&self, image: &EncodedImage) -> VisionAnalysisResult {
        self.provider.analyze(image)
    }

    /// Convert vision result to nomx source stub.
    pub fn to_nomx(&self, result: &VisionAnalysisResult) -> String {
        let mut lines = vec!["@nomx natural".to_string()];
        for c in &result.components {
            lines.push(format!("define {} that ui_component(\"{}\")",
                c.label.replace(' ', "_").to_lowercase(),
                c.component_type.display_name()));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod vision_tests {
    use super::*;

    #[test]
    fn test_component_type_from_label_button() {
        assert_eq!(UiComponentType::from_label("submit button"), UiComponentType::Button);
    }

    #[test]
    fn test_component_type_from_label_nav() {
        assert_eq!(UiComponentType::from_label("navbar"), UiComponentType::NavigationBar);
    }

    #[test]
    fn test_component_type_from_label_unknown() {
        assert_eq!(UiComponentType::from_label("xyzzy"), UiComponentType::Unknown);
    }

    #[test]
    fn test_encoded_image_data_url() {
        let img = EncodedImage::from_bytes(&[0xFF, 0xD8], "image/jpeg");
        let url = img.data_url();
        assert!(url.starts_with("data:image/jpeg;base64,"));
    }

    #[test]
    fn test_stub_provider_basic() {
        let provider = StubVisionProvider::new(vec!["login button", "user card", "nav header"]);
        let img = EncodedImage { base64_data: "x".into(), mime_type: "image/png".into(), width_hint: None, height_hint: None };
        let result = provider.analyze(&img);
        assert_eq!(result.component_count(), 3);
    }

    #[test]
    fn test_components_by_type() {
        let provider = StubVisionProvider::new(vec!["button1", "submit button", "card element"]);
        let img = EncodedImage { base64_data: "x".into(), mime_type: "image/png".into(), width_hint: None, height_hint: None };
        let result = provider.analyze(&img);
        let buttons = result.components_by_type(&UiComponentType::Button);
        assert_eq!(buttons.len(), 2);
    }

    #[test]
    fn test_to_nomx() {
        let provider = Box::new(StubVisionProvider::new(vec!["login button"]));
        let analyzer = ScreenshotAnalyzer::new(provider);
        let img = EncodedImage { base64_data: "x".into(), mime_type: "image/png".into(), width_hint: None, height_hint: None };
        let result = analyzer.analyze(&img);
        let nomx = analyzer.to_nomx(&result);
        assert!(nomx.starts_with("@nomx natural"));
        assert!(nomx.contains("define "));
        assert!(nomx.contains("Button"));
    }

    #[test]
    fn test_confidence_avg() {
        let provider = StubVisionProvider::new(vec!["a", "b", "c"]);
        let img = EncodedImage { base64_data: "x".into(), mime_type: "image/png".into(), width_hint: None, height_hint: None };
        let result = provider.analyze(&img);
        assert!((result.confidence_avg - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_provider_name() {
        let p = StubVisionProvider::new(vec![]);
        assert_eq!(p.provider_name(), "stub");
    }

    #[test]
    fn test_display_names() {
        assert_eq!(UiComponentType::Button.display_name(), "Button");
        assert_eq!(UiComponentType::NavigationBar.display_name(), "NavigationBar");
    }
}
