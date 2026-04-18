#![deny(unsafe_code)]

/// Maximum number of characters allowed in a title bar title.
const TITLE_MAX_LEN: usize = 60;

/// A title bar panel rendered at the very top of a canvas window.
#[derive(Debug, Clone)]
pub struct TitleBarPanel {
    pub title: String,
    pub show_close: bool,
    pub show_minimize: bool,
    pub show_maximize: bool,
}

impl TitleBarPanel {
    /// Create a new title bar with the given title.
    ///
    /// Titles longer than [`TITLE_MAX_LEN`] characters are truncated.
    pub fn new(title: &str) -> Self {
        Self {
            title: Self::truncate(title),
            show_close: false,
            show_minimize: false,
            show_maximize: false,
        }
    }

    /// Enable the close, minimize and maximize traffic-light buttons.
    pub fn with_traffic_lights(mut self) -> Self {
        self.show_close = true;
        self.show_minimize = true;
        self.show_maximize = true;
        self
    }

    fn truncate(title: &str) -> String {
        if title.chars().count() > TITLE_MAX_LEN {
            title.chars().take(TITLE_MAX_LEN).collect()
        } else {
            title.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_title_and_hides_buttons() {
        let tb = TitleBarPanel::new("My Window");
        assert_eq!(tb.title, "My Window");
        assert!(!tb.show_close);
        assert!(!tb.show_minimize);
        assert!(!tb.show_maximize);
    }

    #[test]
    fn with_traffic_lights_enables_all_buttons() {
        let tb = TitleBarPanel::new("My Window").with_traffic_lights();
        assert!(tb.show_close);
        assert!(tb.show_minimize);
        assert!(tb.show_maximize);
    }

    #[test]
    fn title_truncation_at_60_chars() {
        let long_title = "a".repeat(80);
        let tb = TitleBarPanel::new(&long_title);
        assert_eq!(tb.title.chars().count(), 60);
    }
}
