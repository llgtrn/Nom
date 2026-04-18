#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreadcrumbKind {
    File,
    Module,
    Function,
    Type,
    Scope,
}

impl BreadcrumbKind {
    pub fn is_navigable(&self) -> bool {
        matches!(self, BreadcrumbKind::File | BreadcrumbKind::Module | BreadcrumbKind::Function | BreadcrumbKind::Type)
    }

    pub fn separator(&self) -> &'static str {
        match self {
            BreadcrumbKind::File => "/",
            BreadcrumbKind::Module => "::",
            BreadcrumbKind::Function => "→",
            BreadcrumbKind::Type => "<>",
            BreadcrumbKind::Scope => "{}",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BreadcrumbSegment {
    pub kind: BreadcrumbKind,
    pub label: String,
    pub byte_offset: Option<usize>,
}

impl BreadcrumbSegment {
    pub fn display(&self) -> String {
        format!("{}{}", self.kind.separator(), self.label)
    }

    pub fn is_located(&self) -> bool {
        self.byte_offset.is_some()
    }
}

#[derive(Debug, Clone, Default)]
pub struct BreadcrumbPath {
    pub segments: Vec<BreadcrumbSegment>,
}

impl BreadcrumbPath {
    pub fn push(&mut self, s: BreadcrumbSegment) {
        self.segments.push(s);
    }

    pub fn pop(&mut self) -> Option<BreadcrumbSegment> {
        self.segments.pop()
    }

    pub fn full_path(&self) -> String {
        self.segments.iter().map(|s| s.display()).collect::<Vec<_>>().join("")
    }

    pub fn depth(&self) -> usize {
        self.segments.len()
    }
}

#[derive(Debug, Default)]
pub struct BreadcrumbNav {
    pub history: Vec<BreadcrumbPath>,
    pub cursor: usize,
}

impl BreadcrumbNav {
    pub fn navigate_to(&mut self, p: BreadcrumbPath) {
        self.history.push(p);
        self.cursor = self.history.len() - 1;
    }

    pub fn can_go_back(&self) -> bool {
        self.cursor > 0
    }

    pub fn go_back(&mut self) -> Option<&BreadcrumbPath> {
        if self.can_go_back() {
            self.cursor -= 1;
            Some(&self.history[self.cursor])
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct BreadcrumbRenderer {
    pub max_depth: usize,
}

impl BreadcrumbRenderer {
    pub fn render(&self, path: &BreadcrumbPath) -> String {
        let segs = &path.segments;
        let start = segs.len().saturating_sub(self.max_depth);
        segs[start..]
            .iter()
            .map(|s| s.display())
            .collect::<Vec<_>>()
            .join(" > ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_is_navigable() {
        assert!(BreadcrumbKind::File.is_navigable());
        assert!(BreadcrumbKind::Module.is_navigable());
        assert!(BreadcrumbKind::Function.is_navigable());
        assert!(BreadcrumbKind::Type.is_navigable());
        assert!(!BreadcrumbKind::Scope.is_navigable());
    }

    #[test]
    fn test_kind_separator() {
        assert_eq!(BreadcrumbKind::File.separator(), "/");
        assert_eq!(BreadcrumbKind::Module.separator(), "::");
        assert_eq!(BreadcrumbKind::Function.separator(), "→");
        assert_eq!(BreadcrumbKind::Type.separator(), "<>");
        assert_eq!(BreadcrumbKind::Scope.separator(), "{}");
    }

    #[test]
    fn test_segment_display() {
        let seg = BreadcrumbSegment {
            kind: BreadcrumbKind::Module,
            label: "foo".to_string(),
            byte_offset: None,
        };
        assert_eq!(seg.display(), "::foo");

        let seg2 = BreadcrumbSegment {
            kind: BreadcrumbKind::Function,
            label: "bar".to_string(),
            byte_offset: Some(42),
        };
        assert_eq!(seg2.display(), "→bar");
    }

    #[test]
    fn test_segment_is_located() {
        let located = BreadcrumbSegment {
            kind: BreadcrumbKind::File,
            label: "main.nom".to_string(),
            byte_offset: Some(0),
        };
        let unlocated = BreadcrumbSegment {
            kind: BreadcrumbKind::Scope,
            label: "block".to_string(),
            byte_offset: None,
        };
        assert!(located.is_located());
        assert!(!unlocated.is_located());
    }

    #[test]
    fn test_path_full_path_join() {
        let mut path = BreadcrumbPath::default();
        path.push(BreadcrumbSegment { kind: BreadcrumbKind::File, label: "lib".to_string(), byte_offset: None });
        path.push(BreadcrumbSegment { kind: BreadcrumbKind::Module, label: "core".to_string(), byte_offset: None });
        path.push(BreadcrumbSegment { kind: BreadcrumbKind::Function, label: "run".to_string(), byte_offset: None });
        assert_eq!(path.full_path(), "/lib::core→run");
    }

    #[test]
    fn test_path_depth() {
        let mut path = BreadcrumbPath::default();
        assert_eq!(path.depth(), 0);
        path.push(BreadcrumbSegment { kind: BreadcrumbKind::File, label: "x".to_string(), byte_offset: None });
        assert_eq!(path.depth(), 1);
        path.push(BreadcrumbSegment { kind: BreadcrumbKind::Module, label: "y".to_string(), byte_offset: None });
        assert_eq!(path.depth(), 2);
        path.pop();
        assert_eq!(path.depth(), 1);
    }

    #[test]
    fn test_nav_navigate_to_and_cursor() {
        let mut nav = BreadcrumbNav::default();
        let mut p1 = BreadcrumbPath::default();
        p1.push(BreadcrumbSegment { kind: BreadcrumbKind::File, label: "a".to_string(), byte_offset: None });
        let mut p2 = BreadcrumbPath::default();
        p2.push(BreadcrumbSegment { kind: BreadcrumbKind::File, label: "b".to_string(), byte_offset: None });

        nav.navigate_to(p1);
        assert_eq!(nav.cursor, 0);
        nav.navigate_to(p2);
        assert_eq!(nav.cursor, 1);
        assert_eq!(nav.history.len(), 2);
    }

    #[test]
    fn test_nav_can_go_back_and_go_back() {
        let mut nav = BreadcrumbNav::default();
        assert!(!nav.can_go_back());
        assert!(nav.go_back().is_none());

        let mut p1 = BreadcrumbPath::default();
        p1.push(BreadcrumbSegment { kind: BreadcrumbKind::File, label: "first".to_string(), byte_offset: None });
        let mut p2 = BreadcrumbPath::default();
        p2.push(BreadcrumbSegment { kind: BreadcrumbKind::File, label: "second".to_string(), byte_offset: None });

        nav.navigate_to(p1);
        nav.navigate_to(p2);
        assert!(nav.can_go_back());

        let back = nav.go_back().unwrap();
        assert_eq!(back.segments[0].label, "first");
        assert_eq!(nav.cursor, 0);
        assert!(!nav.can_go_back());
    }

    #[test]
    fn test_renderer_truncates_to_max_depth() {
        let renderer = BreadcrumbRenderer { max_depth: 2 };
        let mut path = BreadcrumbPath::default();
        path.push(BreadcrumbSegment { kind: BreadcrumbKind::File, label: "root".to_string(), byte_offset: None });
        path.push(BreadcrumbSegment { kind: BreadcrumbKind::Module, label: "mod1".to_string(), byte_offset: None });
        path.push(BreadcrumbSegment { kind: BreadcrumbKind::Function, label: "fn1".to_string(), byte_offset: None });

        let rendered = renderer.render(&path);
        // Only last 2 segments: Module::mod1 and Function→fn1
        assert_eq!(rendered, "::mod1 > →fn1");

        let renderer_full = BreadcrumbRenderer { max_depth: 10 };
        let rendered_full = renderer_full.render(&path);
        assert_eq!(rendered_full, "/root > ::mod1 > →fn1");
    }
}
