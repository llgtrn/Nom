//! Web-screen composition backend (HTML+WASM bundle generation).
//!
//! Consumes a `Screen` kind (widgets + layout + data bindings) and emits
//! an HTML + WASM bundle that runs in a browser.
#![deny(unsafe_code)]

use crate::backend_trait::{CompositionBackend, ComposeSpec, ComposeOutput, ComposeError, InterruptFlag, ProgressSink};
use crate::kind::NomKind;

pub type WidgetId = String;

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetSpec {
    pub id: WidgetId,
    pub widget_type: String,          // e.g. "button", "text_input", "table"
    pub props: Vec<(String, String)>,
    pub bindings: Vec<DataBinding>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DataBinding {
    pub target_prop: String,
    pub source_path: String,          // e.g. "$.users[0].name"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutKind { Absolute, Flex, Grid }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSpec {
    pub kind: LayoutKind,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub gap_px: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScreenSpec {
    pub title: String,
    pub layout: LayoutSpec,
    pub widgets: Vec<WidgetSpec>,
    pub target_wasm: bool,
}

impl ScreenSpec {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            layout: LayoutSpec { kind: LayoutKind::Flex, viewport_width: 1280, viewport_height: 720, gap_px: 8 },
            widgets: Vec::new(),
            target_wasm: true,
        }
    }
    pub fn add_widget(&mut self, widget: WidgetSpec) { self.widgets.push(widget); }
    pub fn widget_count(&self) -> usize { self.widgets.len() }
    pub fn widget_by_id(&self, id: &str) -> Option<&WidgetSpec> { self.widgets.iter().find(|w| w.id == id) }
    pub fn total_bindings(&self) -> usize { self.widgets.iter().map(|w| w.bindings.len()).sum() }
}

#[derive(Debug, thiserror::Error)]
pub enum ScreenError {
    #[error("title must not be empty")]
    EmptyTitle,
    #[error("duplicate widget id '{0}'")]
    DuplicateWidgetId(String),
    #[error("viewport dimensions must be > 0")]
    InvalidViewport,
}

pub fn validate(spec: &ScreenSpec) -> Result<(), ScreenError> {
    if spec.title.trim().is_empty() { return Err(ScreenError::EmptyTitle); }
    if spec.layout.viewport_width == 0 || spec.layout.viewport_height == 0 { return Err(ScreenError::InvalidViewport); }
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for w in &spec.widgets {
        if !seen.insert(&w.id) { return Err(ScreenError::DuplicateWidgetId(w.id.clone())); }
    }
    Ok(())
}

pub struct StubWebScreenBackend;

impl CompositionBackend for StubWebScreenBackend {
    fn kind(&self) -> NomKind { NomKind::ScreenWeb }
    fn name(&self) -> &str { "stub-web-screen" }
    fn compose(&self, _spec: &ComposeSpec, _progress: &dyn ProgressSink, _interrupt: &InterruptFlag) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput { bytes: b"<!doctype html><html></html>".to_vec(), mime_type: "text/html".to_string(), cost_cents: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_trait::{ComposeSpec, InterruptFlag};

    struct NoopSink;
    impl ProgressSink for NoopSink {
        fn notify(&self, _percent: u32, _message: &str) {}
    }

    fn make_widget(id: &str) -> WidgetSpec {
        WidgetSpec { id: id.to_string(), widget_type: "button".to_string(), props: vec![], bindings: vec![] }
    }

    #[test]
    fn new_defaults_flex_layout() {
        let s = ScreenSpec::new("Dashboard");
        assert_eq!(s.layout.kind, LayoutKind::Flex);
    }

    #[test]
    fn new_defaults_target_wasm_true() {
        let s = ScreenSpec::new("Dashboard");
        assert!(s.target_wasm);
    }

    #[test]
    fn new_defaults_viewport_1280x720() {
        let s = ScreenSpec::new("Dashboard");
        assert_eq!(s.layout.viewport_width, 1280);
        assert_eq!(s.layout.viewport_height, 720);
    }

    #[test]
    fn add_widget_and_widget_count() {
        let mut s = ScreenSpec::new("App");
        assert_eq!(s.widget_count(), 0);
        s.add_widget(make_widget("btn1"));
        s.add_widget(make_widget("btn2"));
        assert_eq!(s.widget_count(), 2);
    }

    #[test]
    fn widget_by_id_found_and_missing() {
        let mut s = ScreenSpec::new("App");
        s.add_widget(make_widget("btn1"));
        assert!(s.widget_by_id("btn1").is_some());
        assert!(s.widget_by_id("missing").is_none());
    }

    #[test]
    fn total_bindings_sum() {
        let mut s = ScreenSpec::new("App");
        let w1 = WidgetSpec {
            id: "w1".to_string(), widget_type: "table".to_string(), props: vec![],
            bindings: vec![
                DataBinding { target_prop: "data".to_string(), source_path: "$.rows".to_string() },
                DataBinding { target_prop: "title".to_string(), source_path: "$.name".to_string() },
            ],
        };
        let w2 = WidgetSpec {
            id: "w2".to_string(), widget_type: "text".to_string(), props: vec![],
            bindings: vec![
                DataBinding { target_prop: "value".to_string(), source_path: "$.label".to_string() },
            ],
        };
        s.add_widget(w1);
        s.add_widget(w2);
        assert_eq!(s.total_bindings(), 3);
    }

    #[test]
    fn validate_ok() {
        let mut s = ScreenSpec::new("My Screen");
        s.add_widget(make_widget("w1"));
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn validate_empty_title() {
        let s = ScreenSpec::new("   ");
        assert!(matches!(validate(&s), Err(ScreenError::EmptyTitle)));
    }

    #[test]
    fn validate_invalid_viewport() {
        let mut s = ScreenSpec::new("App");
        s.layout.viewport_width = 0;
        assert!(matches!(validate(&s), Err(ScreenError::InvalidViewport)));
    }

    #[test]
    fn validate_duplicate_widget_id() {
        let mut s = ScreenSpec::new("App");
        s.add_widget(make_widget("dup"));
        s.add_widget(make_widget("dup"));
        assert!(matches!(validate(&s), Err(ScreenError::DuplicateWidgetId(_))));
    }

    #[test]
    fn stub_backend_kind_is_screen_web() {
        let b = StubWebScreenBackend;
        assert_eq!(b.kind(), NomKind::ScreenWeb);
    }

    #[test]
    fn stub_backend_name() {
        let b = StubWebScreenBackend;
        assert_eq!(b.name(), "stub-web-screen");
    }

    #[test]
    fn stub_backend_compose_returns_html() {
        let b = StubWebScreenBackend;
        let spec = ComposeSpec { kind: NomKind::ScreenWeb, params: vec![] };
        let flag = InterruptFlag::new();
        let out = b.compose(&spec, &NoopSink, &flag).unwrap();
        assert_eq!(out.mime_type, "text/html");
        assert!(out.bytes.starts_with(b"<!doctype html>"));
        assert_eq!(out.cost_cents, 0);
    }
}
