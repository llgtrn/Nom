use crate::span::Span;

pub struct AstNode {
    pub kind: String,
    pub span: Span,
    pub children: Vec<AstNode>,
}

pub trait RuleVisitor {
    fn pre_visit(&mut self, node: &AstNode);
    fn post_visit(&mut self, node: &AstNode);
}

/// Depth-first walk: pre_visit → recurse children → post_visit.
pub fn walk(node: &AstNode, visitor: &mut dyn RuleVisitor) {
    visitor.pre_visit(node);
    for child in &node.children {
        walk(child, visitor);
    }
    visitor.post_visit(node);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OrderRecorder {
        events: Vec<String>,
    }

    impl RuleVisitor for OrderRecorder {
        fn pre_visit(&mut self, node: &AstNode) {
            self.events.push(format!("pre:{}", node.kind));
        }
        fn post_visit(&mut self, node: &AstNode) {
            self.events.push(format!("post:{}", node.kind));
        }
    }

    fn leaf(kind: &str) -> AstNode {
        AstNode { kind: kind.to_string(), span: Span::new(0, 1), children: vec![] }
    }

    fn node(kind: &str, children: Vec<AstNode>) -> AstNode {
        AstNode { kind: kind.to_string(), span: Span::new(0, 10), children }
    }

    #[test]
    fn visit_order_correct() {
        let tree = node("root", vec![leaf("child_a"), leaf("child_b")]);
        let mut recorder = OrderRecorder { events: vec![] };
        walk(&tree, &mut recorder);
        assert_eq!(
            recorder.events,
            vec!["pre:root", "pre:child_a", "post:child_a", "pre:child_b", "post:child_b", "post:root"]
        );
    }

    #[test]
    fn leaf_no_children() {
        let tree = leaf("lone");
        let mut recorder = OrderRecorder { events: vec![] };
        walk(&tree, &mut recorder);
        assert_eq!(recorder.events, vec!["pre:lone", "post:lone"]);
    }

    #[test]
    fn mutation_via_visitor_state() {
        struct Counter { count: usize }
        impl RuleVisitor for Counter {
            fn pre_visit(&mut self, _node: &AstNode) { self.count += 1; }
            fn post_visit(&mut self, _node: &AstNode) {}
        }

        let tree = node("root", vec![leaf("a"), node("b", vec![leaf("c")])]);
        let mut counter = Counter { count: 0 };
        walk(&tree, &mut counter);
        // root + a + b + c = 4 pre_visit calls
        assert_eq!(counter.count, 4);
    }

    #[test]
    fn deeply_nested_walk() {
        let tree = node("l1", vec![node("l2", vec![node("l3", vec![leaf("l4")])])]);
        let mut recorder = OrderRecorder { events: vec![] };
        walk(&tree, &mut recorder);
        assert_eq!(
            recorder.events,
            vec!["pre:l1", "pre:l2", "pre:l3", "pre:l4", "post:l4", "post:l3", "post:l2", "post:l1"]
        );
    }
}
