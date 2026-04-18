/// ComponentKind — the category of a UI component in a web application.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentKind {
    Layout,
    Form,
    Navigation,
    DataTable,
    Chart,
    Modal,
}

impl ComponentKind {
    pub fn kind_name(&self) -> &str {
        match self {
            ComponentKind::Layout => "layout",
            ComponentKind::Form => "form",
            ComponentKind::Navigation => "navigation",
            ComponentKind::DataTable => "data_table",
            ComponentKind::Chart => "chart",
            ComponentKind::Modal => "modal",
        }
    }
}

/// WebComponent — a single UI component in the web app.
#[derive(Debug, Clone)]
pub struct WebComponent {
    pub id: u64,
    pub kind: ComponentKind,
    pub label: String,
    pub children: Vec<u64>,
}

impl WebComponent {
    pub fn new(id: u64, kind: ComponentKind, label: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            label: label.into(),
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child_id: u64) {
        self.children.push(child_id);
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Returns true if this component can contain other components
    /// (Layout and Navigation are containers).
    pub fn is_container(&self) -> bool {
        matches!(self.kind, ComponentKind::Layout | ComponentKind::Navigation)
    }
}

/// WebAppSpec — specification of a complete web application structure.
#[derive(Debug, Clone)]
pub struct WebAppSpec {
    pub name: String,
    pub components: Vec<WebComponent>,
}

impl WebAppSpec {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            components: Vec::new(),
        }
    }

    pub fn add_component(&mut self, c: WebComponent) {
        self.components.push(c);
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn find_by_kind(&self, kind: &ComponentKind) -> Vec<&WebComponent> {
        self.components.iter().filter(|c| &c.kind == kind).collect()
    }

    pub fn containers(&self) -> Vec<&WebComponent> {
        self.components.iter().filter(|c| c.is_container()).collect()
    }
}

/// WebComposer — assembles a WebAppSpec from natural language intent.
#[derive(Debug, Default)]
pub struct WebComposer;

impl WebComposer {
    pub fn new() -> Self {
        Self
    }

    /// Compose a WebAppSpec from an app name and a natural language intent string.
    ///
    /// Always creates: 1 Layout + 1 Navigation + 1 Form component.
    /// If intent contains "table", adds a DataTable component.
    /// If intent contains "chart", adds a Chart component.
    pub fn compose_from_intent(&self, app_name: &str, intent: &str) -> WebAppSpec {
        let mut spec = WebAppSpec::new(app_name);
        let mut next_id: u64 = 1;

        spec.add_component(WebComponent::new(next_id, ComponentKind::Layout, "root-layout"));
        next_id += 1;

        spec.add_component(WebComponent::new(next_id, ComponentKind::Navigation, "main-nav"));
        next_id += 1;

        spec.add_component(WebComponent::new(next_id, ComponentKind::Form, "main-form"));
        next_id += 1;

        let lower = intent.to_lowercase();

        if lower.contains("table") {
            spec.add_component(WebComponent::new(next_id, ComponentKind::DataTable, "data-table"));
            next_id += 1;
        }

        if lower.contains("chart") {
            spec.add_component(WebComponent::new(next_id, ComponentKind::Chart, "main-chart"));
            // next_id += 1; — suppress unused warning, future components may be added
        }

        spec
    }

    /// Estimate the number of components that would be produced for a given intent.
    /// 3 base components + 1 per "table" occurrence + 1 per "chart" occurrence.
    pub fn component_count_estimate(&self, intent: &str) -> usize {
        let lower = intent.to_lowercase();
        let table_count = lower.matches("table").count();
        let chart_count = lower.matches("chart").count();
        3 + table_count + chart_count
    }
}

#[cfg(test)]
mod web_compose_tests {
    use super::*;

    #[test]
    fn component_kind_kind_name() {
        assert_eq!(ComponentKind::Layout.kind_name(), "layout");
        assert_eq!(ComponentKind::Form.kind_name(), "form");
        assert_eq!(ComponentKind::Navigation.kind_name(), "navigation");
        assert_eq!(ComponentKind::DataTable.kind_name(), "data_table");
        assert_eq!(ComponentKind::Chart.kind_name(), "chart");
        assert_eq!(ComponentKind::Modal.kind_name(), "modal");
    }

    #[test]
    fn web_component_is_container_layout() {
        let c = WebComponent::new(1, ComponentKind::Layout, "root");
        assert!(c.is_container(), "Layout must be a container");
    }

    #[test]
    fn web_component_is_container_form_false() {
        let c = WebComponent::new(2, ComponentKind::Form, "sign-up");
        assert!(!c.is_container(), "Form must NOT be a container");
    }

    #[test]
    fn web_component_add_child() {
        let mut c = WebComponent::new(1, ComponentKind::Layout, "root");
        assert_eq!(c.child_count(), 0);
        c.add_child(2);
        c.add_child(3);
        assert_eq!(c.child_count(), 2);
        assert_eq!(c.children, vec![2u64, 3u64]);
    }

    #[test]
    fn web_app_spec_component_count() {
        let mut spec = WebAppSpec::new("my-app");
        assert_eq!(spec.component_count(), 0);
        spec.add_component(WebComponent::new(1, ComponentKind::Layout, "l"));
        spec.add_component(WebComponent::new(2, ComponentKind::Form, "f"));
        assert_eq!(spec.component_count(), 2);
    }

    #[test]
    fn web_app_spec_find_by_kind() {
        let mut spec = WebAppSpec::new("demo");
        spec.add_component(WebComponent::new(1, ComponentKind::Layout, "l1"));
        spec.add_component(WebComponent::new(2, ComponentKind::Form, "f1"));
        spec.add_component(WebComponent::new(3, ComponentKind::Layout, "l2"));

        let layouts = spec.find_by_kind(&ComponentKind::Layout);
        assert_eq!(layouts.len(), 2, "must find 2 Layout components");

        let forms = spec.find_by_kind(&ComponentKind::Form);
        assert_eq!(forms.len(), 1, "must find 1 Form component");

        let charts = spec.find_by_kind(&ComponentKind::Chart);
        assert_eq!(charts.len(), 0, "must find 0 Chart components");
    }

    #[test]
    fn web_composer_compose_base_components() {
        let composer = WebComposer::new();
        let spec = composer.compose_from_intent("hello-app", "show a dashboard");
        // No "table" or "chart" in intent → exactly 3 base components
        assert_eq!(spec.component_count(), 3);
        assert_eq!(spec.find_by_kind(&ComponentKind::Layout).len(), 1);
        assert_eq!(spec.find_by_kind(&ComponentKind::Navigation).len(), 1);
        assert_eq!(spec.find_by_kind(&ComponentKind::Form).len(), 1);
    }

    #[test]
    fn web_composer_compose_with_table() {
        let composer = WebComposer::new();
        let spec = composer.compose_from_intent("data-app", "show a table of users and a chart");
        // "table" + "chart" in intent → 3 base + 1 + 1 = 5
        assert_eq!(spec.component_count(), 5);
        assert_eq!(spec.find_by_kind(&ComponentKind::DataTable).len(), 1);
        assert_eq!(spec.find_by_kind(&ComponentKind::Chart).len(), 1);
    }

    #[test]
    fn web_composer_component_count_estimate() {
        let composer = WebComposer::new();
        // No extras
        assert_eq!(composer.component_count_estimate("show a login page"), 3);
        // One "table"
        assert_eq!(composer.component_count_estimate("show a table of orders"), 4);
        // One "chart"
        assert_eq!(composer.component_count_estimate("display a chart"), 4);
        // Both
        assert_eq!(composer.component_count_estimate("table and chart view"), 5);
    }
}
