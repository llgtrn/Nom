//! Outline view types for the editor panel.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutlineItemKind {
    Heading,
    Function,
    Class,
    Variable,
    Import,
}

impl OutlineItemKind {
    pub fn icon(&self) -> &'static str {
        match self {
            OutlineItemKind::Heading => "H",
            OutlineItemKind::Function => "fn",
            OutlineItemKind::Class => "cls",
            OutlineItemKind::Variable => "var",
            OutlineItemKind::Import => "imp",
        }
    }

    pub fn is_structural(&self) -> bool {
        matches!(
            self,
            OutlineItemKind::Heading | OutlineItemKind::Function | OutlineItemKind::Class
        )
    }
}

#[derive(Debug, Clone)]
pub struct OutlineItem {
    pub kind: OutlineItemKind,
    pub label: String,
    pub line: u32,
    pub depth: u32,
}

impl OutlineItem {
    pub fn indent_prefix(&self) -> String {
        "  ".repeat(self.depth as usize)
    }

    pub fn display(&self) -> String {
        format!("{}[{}] {}", self.indent_prefix(), self.kind.icon(), self.label)
    }
}

#[derive(Debug, Clone)]
pub struct OutlineSection {
    pub title: String,
    pub items: Vec<OutlineItem>,
}

impl OutlineSection {
    pub fn add(&mut self, item: OutlineItem) {
        self.items.push(item);
    }

    pub fn structural_items(&self) -> Vec<&OutlineItem> {
        self.items.iter().filter(|i| i.kind.is_structural()).collect()
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }
}

#[derive(Debug, Clone)]
pub struct OutlineTree {
    pub sections: Vec<OutlineSection>,
}

impl OutlineTree {
    pub fn add_section(&mut self, s: OutlineSection) {
        self.sections.push(s);
    }

    pub fn all_items(&self) -> Vec<&OutlineItem> {
        self.sections.iter().flat_map(|s| s.items.iter()).collect()
    }

    pub fn items_at_depth(&self, depth: u32) -> Vec<&OutlineItem> {
        self.all_items().into_iter().filter(|i| i.depth == depth).collect()
    }
}

pub struct OutlineRenderer {
    pub max_items: usize,
}

impl OutlineRenderer {
    pub fn render(&self, tree: &OutlineTree) -> Vec<String> {
        tree.all_items()
            .into_iter()
            .take(self.max_items)
            .map(|i| i.display())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_icon() {
        assert_eq!(OutlineItemKind::Heading.icon(), "H");
        assert_eq!(OutlineItemKind::Function.icon(), "fn");
        assert_eq!(OutlineItemKind::Class.icon(), "cls");
        assert_eq!(OutlineItemKind::Variable.icon(), "var");
        assert_eq!(OutlineItemKind::Import.icon(), "imp");
    }

    #[test]
    fn kind_is_structural() {
        assert!(OutlineItemKind::Heading.is_structural());
        assert!(OutlineItemKind::Function.is_structural());
        assert!(OutlineItemKind::Class.is_structural());
        assert!(!OutlineItemKind::Variable.is_structural());
        assert!(!OutlineItemKind::Import.is_structural());
    }

    #[test]
    fn item_indent_prefix_depth_2() {
        let item = OutlineItem {
            kind: OutlineItemKind::Function,
            label: "foo".into(),
            line: 1,
            depth: 2,
        };
        assert_eq!(item.indent_prefix(), "    ");
    }

    #[test]
    fn item_display_format() {
        let item = OutlineItem {
            kind: OutlineItemKind::Function,
            label: "bar".into(),
            line: 5,
            depth: 1,
        };
        assert_eq!(item.display(), "  [fn] bar");
    }

    #[test]
    fn section_structural_items() {
        let mut section = OutlineSection { title: "Top".into(), items: vec![] };
        section.add(OutlineItem { kind: OutlineItemKind::Function, label: "f".into(), line: 1, depth: 0 });
        section.add(OutlineItem { kind: OutlineItemKind::Variable, label: "v".into(), line: 2, depth: 0 });
        section.add(OutlineItem { kind: OutlineItemKind::Class, label: "C".into(), line: 3, depth: 0 });
        let structural = section.structural_items();
        assert_eq!(structural.len(), 2);
        assert_eq!(structural[0].label, "f");
        assert_eq!(structural[1].label, "C");
    }

    #[test]
    fn section_count() {
        let mut section = OutlineSection { title: "S".into(), items: vec![] };
        section.add(OutlineItem { kind: OutlineItemKind::Import, label: "x".into(), line: 1, depth: 0 });
        section.add(OutlineItem { kind: OutlineItemKind::Import, label: "y".into(), line: 2, depth: 0 });
        assert_eq!(section.count(), 2);
    }

    #[test]
    fn tree_all_items_flatten() {
        let mut tree = OutlineTree { sections: vec![] };
        let mut s1 = OutlineSection { title: "A".into(), items: vec![] };
        s1.add(OutlineItem { kind: OutlineItemKind::Function, label: "f1".into(), line: 1, depth: 0 });
        let mut s2 = OutlineSection { title: "B".into(), items: vec![] };
        s2.add(OutlineItem { kind: OutlineItemKind::Function, label: "f2".into(), line: 10, depth: 0 });
        s2.add(OutlineItem { kind: OutlineItemKind::Variable, label: "v1".into(), line: 11, depth: 0 });
        tree.add_section(s1);
        tree.add_section(s2);
        let all = tree.all_items();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].label, "f1");
        assert_eq!(all[1].label, "f2");
        assert_eq!(all[2].label, "v1");
    }

    #[test]
    fn tree_items_at_depth() {
        let mut tree = OutlineTree { sections: vec![] };
        let mut s = OutlineSection { title: "S".into(), items: vec![] };
        s.add(OutlineItem { kind: OutlineItemKind::Heading, label: "h".into(), line: 1, depth: 0 });
        s.add(OutlineItem { kind: OutlineItemKind::Function, label: "f".into(), line: 2, depth: 1 });
        s.add(OutlineItem { kind: OutlineItemKind::Class, label: "c".into(), line: 3, depth: 1 });
        tree.add_section(s);
        let depth1 = tree.items_at_depth(1);
        assert_eq!(depth1.len(), 2);
        let depth0 = tree.items_at_depth(0);
        assert_eq!(depth0.len(), 1);
    }

    #[test]
    fn renderer_render_truncates_to_max_items() {
        let mut tree = OutlineTree { sections: vec![] };
        let mut s = OutlineSection { title: "S".into(), items: vec![] };
        for i in 0..5 {
            s.add(OutlineItem {
                kind: OutlineItemKind::Function,
                label: format!("f{}", i),
                line: i as u32,
                depth: 0,
            });
        }
        tree.add_section(s);
        let renderer = OutlineRenderer { max_items: 3 };
        let out = renderer.render(&tree);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], "[fn] f0");
        assert_eq!(out[2], "[fn] f2");
    }
}
