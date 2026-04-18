/// All widget types available in the palette.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    // Basic (8)
    Button,
    TextInput,
    NumberInput,
    Checkbox,
    Toggle,
    Select,
    MultiSelect,
    RadioGroup,
    // Display (7)
    Text,
    Heading,
    Image,
    Badge,
    Icon,
    Divider,
    Spacer,
    // Layout (5)
    Container,
    Grid,
    Tabs,
    Modal,
    Drawer,
    // Data (5)
    Table,
    Chart,
    Gauge,
    Stat,
    Tree,
    // Form (5)
    Form,
    DatePicker,
    TimePicker,
    ColorPicker,
    FileUpload,
    // Custom / Nom-native (5)
    NomBlock,
    NomCanvas,
    NomCompose,
    NomGraph,
    NomIntent,
}

impl WidgetKind {
    /// Human-readable display name for the widget.
    pub fn display_name(&self) -> &str {
        match self {
            WidgetKind::Button => "Button",
            WidgetKind::TextInput => "Text Input",
            WidgetKind::NumberInput => "Number Input",
            WidgetKind::Checkbox => "Checkbox",
            WidgetKind::Toggle => "Toggle",
            WidgetKind::Select => "Select",
            WidgetKind::MultiSelect => "Multi Select",
            WidgetKind::RadioGroup => "Radio Group",
            WidgetKind::Text => "Text",
            WidgetKind::Heading => "Heading",
            WidgetKind::Image => "Image",
            WidgetKind::Badge => "Badge",
            WidgetKind::Icon => "Icon",
            WidgetKind::Divider => "Divider",
            WidgetKind::Spacer => "Spacer",
            WidgetKind::Container => "Container",
            WidgetKind::Grid => "Grid",
            WidgetKind::Tabs => "Tabs",
            WidgetKind::Modal => "Modal",
            WidgetKind::Drawer => "Drawer",
            WidgetKind::Table => "Table",
            WidgetKind::Chart => "Chart",
            WidgetKind::Gauge => "Gauge",
            WidgetKind::Stat => "Stat",
            WidgetKind::Tree => "Tree",
            WidgetKind::Form => "Form",
            WidgetKind::DatePicker => "Date Picker",
            WidgetKind::TimePicker => "Time Picker",
            WidgetKind::ColorPicker => "Color Picker",
            WidgetKind::FileUpload => "File Upload",
            WidgetKind::NomBlock => "Nom Block",
            WidgetKind::NomCanvas => "Nom Canvas",
            WidgetKind::NomCompose => "Nom Compose",
            WidgetKind::NomGraph => "Nom Graph",
            WidgetKind::NomIntent => "Nom Intent",
        }
    }

    /// Category this widget belongs to.
    pub fn category(&self) -> WidgetCategory {
        match self {
            WidgetKind::Button
            | WidgetKind::TextInput
            | WidgetKind::NumberInput
            | WidgetKind::Checkbox
            | WidgetKind::Toggle
            | WidgetKind::Select
            | WidgetKind::MultiSelect
            | WidgetKind::RadioGroup => WidgetCategory::Basic,

            WidgetKind::Text
            | WidgetKind::Heading
            | WidgetKind::Image
            | WidgetKind::Badge
            | WidgetKind::Icon
            | WidgetKind::Divider
            | WidgetKind::Spacer => WidgetCategory::Display,

            WidgetKind::Container
            | WidgetKind::Grid
            | WidgetKind::Tabs
            | WidgetKind::Modal
            | WidgetKind::Drawer => WidgetCategory::Layout,

            WidgetKind::Table
            | WidgetKind::Chart
            | WidgetKind::Gauge
            | WidgetKind::Stat
            | WidgetKind::Tree => WidgetCategory::Data,

            WidgetKind::Form
            | WidgetKind::DatePicker
            | WidgetKind::TimePicker
            | WidgetKind::ColorPicker
            | WidgetKind::FileUpload => WidgetCategory::Form,

            WidgetKind::NomBlock
            | WidgetKind::NomCanvas
            | WidgetKind::NomCompose
            | WidgetKind::NomGraph
            | WidgetKind::NomIntent => WidgetCategory::Custom,
        }
    }

    /// Returns `true` for Nom-native widget variants.
    pub fn is_nom_native(&self) -> bool {
        matches!(
            self,
            WidgetKind::NomBlock
                | WidgetKind::NomCanvas
                | WidgetKind::NomCompose
                | WidgetKind::NomGraph
                | WidgetKind::NomIntent
        )
    }

    /// All 35 widget variants.
    pub fn all() -> Vec<WidgetKind> {
        vec![
            WidgetKind::Button,
            WidgetKind::TextInput,
            WidgetKind::NumberInput,
            WidgetKind::Checkbox,
            WidgetKind::Toggle,
            WidgetKind::Select,
            WidgetKind::MultiSelect,
            WidgetKind::RadioGroup,
            WidgetKind::Text,
            WidgetKind::Heading,
            WidgetKind::Image,
            WidgetKind::Badge,
            WidgetKind::Icon,
            WidgetKind::Divider,
            WidgetKind::Spacer,
            WidgetKind::Container,
            WidgetKind::Grid,
            WidgetKind::Tabs,
            WidgetKind::Modal,
            WidgetKind::Drawer,
            WidgetKind::Table,
            WidgetKind::Chart,
            WidgetKind::Gauge,
            WidgetKind::Stat,
            WidgetKind::Tree,
            WidgetKind::Form,
            WidgetKind::DatePicker,
            WidgetKind::TimePicker,
            WidgetKind::ColorPicker,
            WidgetKind::FileUpload,
            WidgetKind::NomBlock,
            WidgetKind::NomCanvas,
            WidgetKind::NomCompose,
            WidgetKind::NomGraph,
            WidgetKind::NomIntent,
        ]
    }
}

/// Grouping categories for widgets.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetCategory {
    Basic,
    Display,
    Layout,
    Data,
    Form,
    Custom,
}

impl WidgetCategory {
    /// Human-readable display name for the category.
    pub fn display_name(&self) -> &str {
        match self {
            WidgetCategory::Basic => "Basic",
            WidgetCategory::Display => "Display",
            WidgetCategory::Layout => "Layout",
            WidgetCategory::Data => "Data",
            WidgetCategory::Form => "Form",
            WidgetCategory::Custom => "Custom",
        }
    }
}

/// Registry of available widgets.
pub struct WidgetRegistry {
    pub widgets: Vec<WidgetKind>,
}

impl WidgetRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { widgets: Vec::new() }
    }

    /// Create a registry pre-populated with all 35 widgets.
    pub fn with_all() -> Self {
        Self { widgets: WidgetKind::all() }
    }

    /// Number of widgets in this registry.
    pub fn count(&self) -> usize {
        self.widgets.len()
    }

    /// All widgets belonging to the given category.
    pub fn by_category(&self, cat: &WidgetCategory) -> Vec<&WidgetKind> {
        self.widgets.iter().filter(|w| &w.category() == cat).collect()
    }

    /// Widgets whose display name contains `query` (case-insensitive).
    /// An empty query returns all widgets.
    pub fn search(&self, query: &str) -> Vec<&WidgetKind> {
        if query.is_empty() {
            return self.widgets.iter().collect();
        }
        let lc = query.to_lowercase();
        self.widgets
            .iter()
            .filter(|w| w.display_name().to_lowercase().contains(&lc))
            .collect()
    }
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_kind_all_count_is_35() {
        assert_eq!(WidgetKind::all().len(), 35);
    }

    #[test]
    fn widget_kind_nom_native() {
        let native: Vec<_> = WidgetKind::all().into_iter().filter(|w| w.is_nom_native()).collect();
        assert_eq!(native.len(), 5);
        assert!(WidgetKind::NomBlock.is_nom_native());
        assert!(WidgetKind::NomCanvas.is_nom_native());
        assert!(WidgetKind::NomCompose.is_nom_native());
        assert!(WidgetKind::NomGraph.is_nom_native());
        assert!(WidgetKind::NomIntent.is_nom_native());
        assert!(!WidgetKind::Button.is_nom_native());
    }

    #[test]
    fn widget_category_counts() {
        let all = WidgetKind::all();
        let count = |cat: WidgetCategory| all.iter().filter(|w| w.category() == cat).count();
        assert_eq!(count(WidgetCategory::Basic), 8);
        assert_eq!(count(WidgetCategory::Display), 7);
        assert_eq!(count(WidgetCategory::Layout), 5);
        assert_eq!(count(WidgetCategory::Data), 5);
        assert_eq!(count(WidgetCategory::Form), 5);
        assert_eq!(count(WidgetCategory::Custom), 5);
    }

    #[test]
    fn registry_with_all_count() {
        let reg = WidgetRegistry::with_all();
        assert_eq!(reg.count(), 35);
    }

    #[test]
    fn registry_by_category_basic() {
        let reg = WidgetRegistry::with_all();
        let basic = reg.by_category(&WidgetCategory::Basic);
        assert_eq!(basic.len(), 8);
    }

    #[test]
    fn registry_search_text_returns_results() {
        let reg = WidgetRegistry::with_all();
        let results = reg.search("text");
        // "Text Input" and "Text" both contain "text"
        assert!(!results.is_empty());
        for w in &results {
            assert!(w.display_name().to_lowercase().contains("text"));
        }
    }

    #[test]
    fn registry_search_empty_returns_all() {
        let reg = WidgetRegistry::with_all();
        assert_eq!(reg.search("").len(), 35);
    }

    #[test]
    fn widget_kind_display_name_not_empty() {
        for w in WidgetKind::all() {
            assert!(!w.display_name().is_empty(), "{w:?} has empty display_name");
        }
    }
}
