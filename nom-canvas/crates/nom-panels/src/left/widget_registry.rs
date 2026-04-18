/// All widget types available in the palette.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    // Basic (10)
    Button,
    TextInput,
    NumberInput,
    Checkbox,
    Toggle,
    Select,
    MultiSelect,
    RadioGroup,
    ButtonGroup,
    Divider,
    // Display (9)
    Text,
    Heading,
    Image,
    Badge,
    Icon,
    Spacer,
    Timeline,
    Carousel,
    Sparkline,
    // Layout (7)
    Container,
    Grid,
    Tabs,
    Modal,
    Drawer,
    DockPanel,
    Accordion,
    // Data (9)
    Table,
    Chart,
    Gauge,
    Stat,
    Tree,
    Heatmap,
    TreeMap,
    Funnel,
    GaugeChart,
    // Form (7)
    Form,
    DatePicker,
    TimePicker,
    ColorPicker,
    FileUpload,
    TagInput,
    RangeSlider,
    // Custom / Nom-native (9)
    NomBlock,
    NomCanvas,
    NomCompose,
    NomGraph,
    NomIntent,
    NomFlow,
    NomTimeline,
    NomDiff,
    NomSearch,
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
            WidgetKind::ButtonGroup => "Button Group",
            WidgetKind::Divider => "Divider",
            WidgetKind::Text => "Text",
            WidgetKind::Heading => "Heading",
            WidgetKind::Image => "Image",
            WidgetKind::Badge => "Badge",
            WidgetKind::Icon => "Icon",
            WidgetKind::Spacer => "Spacer",
            WidgetKind::Timeline => "Timeline",
            WidgetKind::Carousel => "Carousel",
            WidgetKind::Sparkline => "Sparkline",
            WidgetKind::Container => "Container",
            WidgetKind::Grid => "Grid",
            WidgetKind::Tabs => "Tabs",
            WidgetKind::Modal => "Modal",
            WidgetKind::Drawer => "Drawer",
            WidgetKind::DockPanel => "Dock Panel",
            WidgetKind::Accordion => "Accordion",
            WidgetKind::Table => "Table",
            WidgetKind::Chart => "Chart",
            WidgetKind::Gauge => "Gauge",
            WidgetKind::Stat => "Stat",
            WidgetKind::Tree => "Tree",
            WidgetKind::Heatmap => "Heatmap",
            WidgetKind::TreeMap => "Tree Map",
            WidgetKind::Funnel => "Funnel",
            WidgetKind::GaugeChart => "Gauge Chart",
            WidgetKind::Form => "Form",
            WidgetKind::DatePicker => "Date Picker",
            WidgetKind::TimePicker => "Time Picker",
            WidgetKind::ColorPicker => "Color Picker",
            WidgetKind::FileUpload => "File Upload",
            WidgetKind::TagInput => "Tag Input",
            WidgetKind::RangeSlider => "Range Slider",
            WidgetKind::NomBlock => "Nom Block",
            WidgetKind::NomCanvas => "Nom Canvas",
            WidgetKind::NomCompose => "Nom Compose",
            WidgetKind::NomGraph => "Nom Graph",
            WidgetKind::NomIntent => "Nom Intent",
            WidgetKind::NomFlow => "Nom Flow",
            WidgetKind::NomTimeline => "Nom Timeline",
            WidgetKind::NomDiff => "Nom Diff",
            WidgetKind::NomSearch => "Nom Search",
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
            | WidgetKind::RadioGroup
            | WidgetKind::ButtonGroup
            | WidgetKind::Divider => WidgetCategory::Basic,

            WidgetKind::Text
            | WidgetKind::Heading
            | WidgetKind::Image
            | WidgetKind::Badge
            | WidgetKind::Icon
            | WidgetKind::Spacer
            | WidgetKind::Timeline
            | WidgetKind::Carousel
            | WidgetKind::Sparkline => WidgetCategory::Display,

            WidgetKind::Container
            | WidgetKind::Grid
            | WidgetKind::Tabs
            | WidgetKind::Modal
            | WidgetKind::Drawer
            | WidgetKind::DockPanel
            | WidgetKind::Accordion => WidgetCategory::Layout,

            WidgetKind::Table
            | WidgetKind::Chart
            | WidgetKind::Gauge
            | WidgetKind::Stat
            | WidgetKind::Tree
            | WidgetKind::Heatmap
            | WidgetKind::TreeMap
            | WidgetKind::Funnel
            | WidgetKind::GaugeChart => WidgetCategory::Data,

            WidgetKind::Form
            | WidgetKind::DatePicker
            | WidgetKind::TimePicker
            | WidgetKind::ColorPicker
            | WidgetKind::FileUpload
            | WidgetKind::TagInput
            | WidgetKind::RangeSlider => WidgetCategory::Form,

            WidgetKind::NomBlock
            | WidgetKind::NomCanvas
            | WidgetKind::NomCompose
            | WidgetKind::NomGraph
            | WidgetKind::NomIntent
            | WidgetKind::NomFlow
            | WidgetKind::NomTimeline
            | WidgetKind::NomDiff
            | WidgetKind::NomSearch => WidgetCategory::Custom,
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
                | WidgetKind::NomFlow
                | WidgetKind::NomTimeline
                | WidgetKind::NomDiff
                | WidgetKind::NomSearch
        )
    }

    /// All 51 widget variants.
    pub fn all() -> Vec<WidgetKind> {
        vec![
            // Basic (10)
            WidgetKind::Button,
            WidgetKind::TextInput,
            WidgetKind::NumberInput,
            WidgetKind::Checkbox,
            WidgetKind::Toggle,
            WidgetKind::Select,
            WidgetKind::MultiSelect,
            WidgetKind::RadioGroup,
            WidgetKind::ButtonGroup,
            WidgetKind::Divider,
            // Display (9)
            WidgetKind::Text,
            WidgetKind::Heading,
            WidgetKind::Image,
            WidgetKind::Badge,
            WidgetKind::Icon,
            WidgetKind::Spacer,
            WidgetKind::Timeline,
            WidgetKind::Carousel,
            WidgetKind::Sparkline,
            // Layout (7)
            WidgetKind::Container,
            WidgetKind::Grid,
            WidgetKind::Tabs,
            WidgetKind::Modal,
            WidgetKind::Drawer,
            WidgetKind::DockPanel,
            WidgetKind::Accordion,
            // Data (9)
            WidgetKind::Table,
            WidgetKind::Chart,
            WidgetKind::Gauge,
            WidgetKind::Stat,
            WidgetKind::Tree,
            WidgetKind::Heatmap,
            WidgetKind::TreeMap,
            WidgetKind::Funnel,
            WidgetKind::GaugeChart,
            // Form (7)
            WidgetKind::Form,
            WidgetKind::DatePicker,
            WidgetKind::TimePicker,
            WidgetKind::ColorPicker,
            WidgetKind::FileUpload,
            WidgetKind::TagInput,
            WidgetKind::RangeSlider,
            // Custom / Nom-native (9)
            WidgetKind::NomBlock,
            WidgetKind::NomCanvas,
            WidgetKind::NomCompose,
            WidgetKind::NomGraph,
            WidgetKind::NomIntent,
            WidgetKind::NomFlow,
            WidgetKind::NomTimeline,
            WidgetKind::NomDiff,
            WidgetKind::NomSearch,
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

    /// Create a registry pre-populated with all widgets.
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
        // Name preserved for history; registry now has 51 variants.
        assert_eq!(WidgetKind::all().len(), 51);
    }

    #[test]
    fn widget_kind_nom_native() {
        let native: Vec<_> =
            WidgetKind::all().into_iter().filter(|w| w.is_nom_native()).collect();
        assert_eq!(native.len(), 9);
        assert!(WidgetKind::NomBlock.is_nom_native());
        assert!(WidgetKind::NomCanvas.is_nom_native());
        assert!(WidgetKind::NomCompose.is_nom_native());
        assert!(WidgetKind::NomGraph.is_nom_native());
        assert!(WidgetKind::NomIntent.is_nom_native());
        assert!(WidgetKind::NomFlow.is_nom_native());
        assert!(WidgetKind::NomTimeline.is_nom_native());
        assert!(WidgetKind::NomDiff.is_nom_native());
        assert!(WidgetKind::NomSearch.is_nom_native());
        assert!(!WidgetKind::Button.is_nom_native());
    }

    #[test]
    fn widget_category_counts() {
        let all = WidgetKind::all();
        let count = |cat: WidgetCategory| all.iter().filter(|w| w.category() == cat).count();
        assert_eq!(count(WidgetCategory::Basic), 10);
        assert_eq!(count(WidgetCategory::Display), 9);
        assert_eq!(count(WidgetCategory::Layout), 7);
        assert_eq!(count(WidgetCategory::Data), 9);
        assert_eq!(count(WidgetCategory::Form), 7);
        assert_eq!(count(WidgetCategory::Custom), 9);
    }

    #[test]
    fn registry_with_all_count() {
        let reg = WidgetRegistry::with_all();
        assert_eq!(reg.count(), 51);
    }

    #[test]
    fn registry_by_category_basic() {
        let reg = WidgetRegistry::with_all();
        let basic = reg.by_category(&WidgetCategory::Basic);
        assert_eq!(basic.len(), 10);
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
        assert_eq!(reg.search("").len(), 51);
    }

    #[test]
    fn widget_kind_display_name_not_empty() {
        for w in WidgetKind::all() {
            assert!(!w.display_name().is_empty(), "{w:?} has empty display_name");
        }
    }

    // --- New tests for the expanded registry ---

    #[test]
    fn test_55_variants_registered() {
        // 51 total variants: Basic=10, Display=9, Layout=7, Data=9, Form=7, Custom=9
        assert_eq!(WidgetKind::all().len(), 51);
    }

    #[test]
    fn test_nom_native_count_7() {
        // 9 nom-native: original 5 + NomFlow, NomTimeline, NomDiff, NomSearch
        let count = WidgetKind::all().into_iter().filter(|w| w.is_nom_native()).count();
        assert_eq!(count, 9);
        assert!(WidgetKind::NomFlow.is_nom_native());
        assert!(WidgetKind::NomTimeline.is_nom_native());
        assert!(WidgetKind::NomDiff.is_nom_native());
        assert!(WidgetKind::NomSearch.is_nom_native());
    }

    #[test]
    fn test_by_category_data_count() {
        let reg = WidgetRegistry::with_all();
        let data = reg.by_category(&WidgetCategory::Data);
        // Table, Chart, Gauge, Stat, Tree, Heatmap, TreeMap, Funnel, GaugeChart
        assert_eq!(data.len(), 9);
    }

    #[test]
    fn test_search_finds_gauge() {
        let reg = WidgetRegistry::with_all();
        let results = reg.search("gauge");
        // "Gauge" and "Gauge Chart" both contain "gauge"
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|w| **w == WidgetKind::Gauge));
        assert!(results.iter().any(|w| **w == WidgetKind::GaugeChart));
    }
}
