#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TooltipKind {
    Type,
    Documentation,
    Signature,
    Reference,
}

impl TooltipKind {
    pub fn is_code(&self) -> bool {
        matches!(self, TooltipKind::Type | TooltipKind::Signature)
    }
}

#[derive(Debug, Clone)]
pub struct TooltipContent {
    pub kind: TooltipKind,
    pub text: String,
    pub detail: Option<String>,
}

impl TooltipContent {
    pub fn has_detail(&self) -> bool {
        self.detail.is_some()
    }

    pub fn full_text(&self) -> String {
        match &self.detail {
            Some(d) => format!("{} — {}", self.text, d),
            None => self.text.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TooltipAnchor {
    pub byte_offset: usize,
    pub line: u32,
    pub column: u32,
}

impl TooltipAnchor {
    pub fn position_string(&self) -> String {
        format!("line:{} col:{}", self.line, self.column)
    }
}

#[derive(Debug, Clone)]
pub struct HoverTooltip {
    pub content: TooltipContent,
    pub anchor: TooltipAnchor,
    pub visible: bool,
}

impl HoverTooltip {
    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

#[derive(Debug, Default)]
pub struct TooltipRenderer {
    pub tooltips: Vec<HoverTooltip>,
}

impl TooltipRenderer {
    pub fn push(&mut self, t: HoverTooltip) {
        self.tooltips.push(t);
    }

    pub fn visible_count(&self) -> usize {
        self.tooltips.iter().filter(|t| t.visible).count()
    }

    pub fn hide_all(&mut self) {
        for t in &mut self.tooltips {
            t.visible = false;
        }
    }
}

#[cfg(test)]
mod hover_tooltip_tests {
    use super::*;

    fn make_anchor() -> TooltipAnchor {
        TooltipAnchor { byte_offset: 10, line: 3, column: 7 }
    }

    fn make_content(kind: TooltipKind, text: &str, detail: Option<&str>) -> TooltipContent {
        TooltipContent {
            kind,
            text: text.to_string(),
            detail: detail.map(|s| s.to_string()),
        }
    }

    fn make_tooltip(visible: bool) -> HoverTooltip {
        HoverTooltip {
            content: make_content(TooltipKind::Type, "i32", None),
            anchor: make_anchor(),
            visible,
        }
    }

    #[test]
    fn tooltip_kind_is_code() {
        assert!(TooltipKind::Type.is_code());
        assert!(TooltipKind::Signature.is_code());
        assert!(!TooltipKind::Documentation.is_code());
        assert!(!TooltipKind::Reference.is_code());
    }

    #[test]
    fn full_text_with_detail() {
        let c = make_content(TooltipKind::Type, "hello", Some("world"));
        assert_eq!(c.full_text(), "hello — world");
    }

    #[test]
    fn full_text_without_detail() {
        let c = make_content(TooltipKind::Documentation, "just text", None);
        assert_eq!(c.full_text(), "just text");
    }

    #[test]
    fn has_detail_true() {
        let c = make_content(TooltipKind::Signature, "fn foo()", Some("returns u32"));
        assert!(c.has_detail());
    }

    #[test]
    fn has_detail_false() {
        let c = make_content(TooltipKind::Reference, "ref", None);
        assert!(!c.has_detail());
    }

    #[test]
    fn anchor_position_string() {
        let a = TooltipAnchor { byte_offset: 0, line: 5, column: 12 };
        assert_eq!(a.position_string(), "line:5 col:12");
    }

    #[test]
    fn tooltip_show_hide_toggle() {
        let mut t = make_tooltip(false);
        t.show();
        assert!(t.visible);
        t.hide();
        assert!(!t.visible);
        t.toggle();
        assert!(t.visible);
        t.toggle();
        assert!(!t.visible);
    }

    #[test]
    fn renderer_visible_count() {
        let mut r = TooltipRenderer::default();
        r.push(make_tooltip(true));
        r.push(make_tooltip(false));
        r.push(make_tooltip(true));
        assert_eq!(r.visible_count(), 2);
    }

    #[test]
    fn renderer_hide_all() {
        let mut r = TooltipRenderer::default();
        r.push(make_tooltip(true));
        r.push(make_tooltip(true));
        r.hide_all();
        assert_eq!(r.visible_count(), 0);
    }
}
