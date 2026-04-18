#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeLensKind {
    References,
    Tests,
    Implementations,
    Performance,
    AIInsight,
}

impl CodeLensKind {
    pub fn is_actionable(&self) -> bool {
        matches!(self, CodeLensKind::References | CodeLensKind::Tests | CodeLensKind::Implementations)
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            CodeLensKind::References => "ref",
            CodeLensKind::Tests => "test",
            CodeLensKind::Implementations => "impl",
            CodeLensKind::Performance => "perf",
            CodeLensKind::AIInsight => "ai",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodeLens {
    pub kind: CodeLensKind,
    pub line: u32,
    pub title: String,
    pub command: String,
}

impl CodeLens {
    pub fn display_title(&self) -> String {
        format!("[{}] {}", self.kind.icon_name(), self.title)
    }

    pub fn is_on_line(&self, l: u32) -> bool {
        self.line == l
    }
}

#[derive(Debug, Default)]
pub struct CodeLensProvider {
    pub lenses: Vec<CodeLens>,
}

impl CodeLensProvider {
    pub fn add(&mut self, lens: CodeLens) {
        self.lenses.push(lens);
    }

    pub fn for_line(&self, line: u32) -> Vec<&CodeLens> {
        self.lenses.iter().filter(|l| l.is_on_line(line)).collect()
    }

    pub fn actionable(&self) -> Vec<&CodeLens> {
        self.lenses.iter().filter(|l| l.kind.is_actionable()).collect()
    }

    pub fn clear(&mut self) {
        self.lenses.clear();
    }
}

#[derive(Debug, Default)]
pub struct CodeLensOverlay {
    pub provider: CodeLensProvider,
    pub visible: bool,
}

impl CodeLensOverlay {
    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn visible_lenses(&self) -> Vec<&CodeLens> {
        if !self.visible {
            return vec![];
        }
        self.provider.lenses.iter().collect()
    }

    pub fn total_count(&self) -> usize {
        self.provider.lenses.len()
    }
}

#[derive(Debug, Default)]
pub struct LensResolver {
    pub overlays: Vec<CodeLensOverlay>,
}

impl LensResolver {
    pub fn add_overlay(&mut self, o: CodeLensOverlay) {
        self.overlays.push(o);
    }

    pub fn total_visible(&self) -> usize {
        self.overlays.iter().map(|o| o.visible_lenses().len()).sum()
    }
}

#[cfg(test)]
mod code_lens_tests {
    use super::*;

    fn make_lens(kind: CodeLensKind, line: u32, title: &str) -> CodeLens {
        CodeLens {
            kind,
            line,
            title: title.to_string(),
            command: "cmd".to_string(),
        }
    }

    #[test]
    fn kind_is_actionable_perf_false() {
        assert!(!CodeLensKind::Performance.is_actionable());
        assert!(!CodeLensKind::AIInsight.is_actionable());
        assert!(CodeLensKind::References.is_actionable());
    }

    #[test]
    fn kind_icon_name() {
        assert_eq!(CodeLensKind::References.icon_name(), "ref");
        assert_eq!(CodeLensKind::Tests.icon_name(), "test");
        assert_eq!(CodeLensKind::Implementations.icon_name(), "impl");
        assert_eq!(CodeLensKind::Performance.icon_name(), "perf");
        assert_eq!(CodeLensKind::AIInsight.icon_name(), "ai");
    }

    #[test]
    fn lens_display_title_format() {
        let lens = make_lens(CodeLensKind::Tests, 5, "Run tests");
        assert_eq!(lens.display_title(), "[test] Run tests");
    }

    #[test]
    fn lens_is_on_line() {
        let lens = make_lens(CodeLensKind::References, 10, "refs");
        assert!(lens.is_on_line(10));
        assert!(!lens.is_on_line(11));
    }

    #[test]
    fn provider_for_line_count() {
        let mut provider = CodeLensProvider::default();
        provider.add(make_lens(CodeLensKind::References, 5, "a"));
        provider.add(make_lens(CodeLensKind::Tests, 5, "b"));
        provider.add(make_lens(CodeLensKind::Implementations, 10, "c"));
        assert_eq!(provider.for_line(5).len(), 2);
        assert_eq!(provider.for_line(10).len(), 1);
        assert_eq!(provider.for_line(99).len(), 0);
    }

    #[test]
    fn provider_actionable_count() {
        let mut provider = CodeLensProvider::default();
        provider.add(make_lens(CodeLensKind::References, 1, "a"));
        provider.add(make_lens(CodeLensKind::Performance, 2, "b"));
        provider.add(make_lens(CodeLensKind::AIInsight, 3, "c"));
        provider.add(make_lens(CodeLensKind::Tests, 4, "d"));
        assert_eq!(provider.actionable().len(), 2);
    }

    #[test]
    fn overlay_show_visible_lenses_not_empty() {
        let mut overlay = CodeLensOverlay::default();
        overlay.provider.add(make_lens(CodeLensKind::Tests, 1, "t"));
        overlay.show();
        assert!(!overlay.visible_lenses().is_empty());
    }

    #[test]
    fn overlay_hide_visible_lenses_empty() {
        let mut overlay = CodeLensOverlay::default();
        overlay.provider.add(make_lens(CodeLensKind::Tests, 1, "t"));
        overlay.show();
        overlay.hide();
        assert!(overlay.visible_lenses().is_empty());
    }

    #[test]
    fn resolver_total_visible_sum() {
        let mut resolver = LensResolver::default();

        let mut o1 = CodeLensOverlay::default();
        o1.provider.add(make_lens(CodeLensKind::References, 1, "a"));
        o1.provider.add(make_lens(CodeLensKind::Tests, 2, "b"));
        o1.show();

        let mut o2 = CodeLensOverlay::default();
        o2.provider.add(make_lens(CodeLensKind::Performance, 3, "c"));
        o2.show();

        let mut o3 = CodeLensOverlay::default();
        o3.provider.add(make_lens(CodeLensKind::AIInsight, 4, "d"));
        // o3 hidden — should not count

        resolver.add_overlay(o1);
        resolver.add_overlay(o2);
        resolver.add_overlay(o3);

        assert_eq!(resolver.total_visible(), 3);
    }
}
