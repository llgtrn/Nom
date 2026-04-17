//! Breadcrumb trail for navigation (root doc → section → block).
#![deny(unsafe_code)]

#[derive(Clone, Debug, PartialEq)]
pub struct BreadcrumbSegment {
    pub label: String,
    pub target_id: String,       // document id or block id
    pub is_clickable: bool,
}

impl BreadcrumbSegment {
    pub fn new(label: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self { label: label.into(), target_id: target_id.into(), is_clickable: true }
    }
    pub fn non_clickable(label: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self { label: label.into(), target_id: target_id.into(), is_clickable: false }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Breadcrumb {
    pub segments: Vec<BreadcrumbSegment>,
    pub separator: String,
    pub max_visible_segments: usize,
}

impl Breadcrumb {
    pub fn new() -> Self {
        Self { segments: Vec::new(), separator: " / ".to_string(), max_visible_segments: 5 }
    }

    pub fn push(&mut self, segment: BreadcrumbSegment) { self.segments.push(segment); }
    pub fn pop(&mut self) -> Option<BreadcrumbSegment> { self.segments.pop() }
    pub fn clear(&mut self) { self.segments.clear(); }
    pub fn len(&self) -> usize { self.segments.len() }
    pub fn is_empty(&self) -> bool { self.segments.is_empty() }

    pub fn with_separator(mut self, sep: impl Into<String>) -> Self { self.separator = sep.into(); self }
    pub fn with_max_visible(mut self, n: usize) -> Self { self.max_visible_segments = n.max(1); self }

    /// Produce the rendered breadcrumb string.  Inserts a single ellipsis
    /// segment when `segments.len() > max_visible_segments`.
    pub fn rendered(&self) -> String {
        let visible = self.visible_segments();
        let labels: Vec<String> = visible.iter().map(|s| s.label.clone()).collect();
        labels.join(&self.separator)
    }

    /// Segments that would actually be rendered (after truncation).
    /// First segment + ellipsis + last N-1 segments when over limit.
    pub fn visible_segments(&self) -> Vec<BreadcrumbSegment> {
        if self.segments.len() <= self.max_visible_segments {
            return self.segments.clone();
        }
        let keep_tail = self.max_visible_segments.saturating_sub(2).max(1);
        let mut out = Vec::with_capacity(self.max_visible_segments);
        out.push(self.segments[0].clone());
        out.push(BreadcrumbSegment::non_clickable("…", "breadcrumb:ellipsis"));
        let start = self.segments.len() - keep_tail;
        out.extend(self.segments[start..].iter().cloned());
        out
    }

    pub fn current(&self) -> Option<&BreadcrumbSegment> { self.segments.last() }
    pub fn parent(&self) -> Option<&BreadcrumbSegment> {
        if self.segments.len() < 2 { return None; }
        self.segments.get(self.segments.len() - 2)
    }

    /// Truncate the breadcrumb back to the segment with target_id.
    /// Useful when the user clicks a mid-segment to navigate up.
    pub fn navigate_to(&mut self, target_id: &str) -> bool {
        let Some(pos) = self.segments.iter().position(|s| s.target_id == target_id) else { return false };
        self.segments.truncate(pos + 1);
        true
    }
}

impl Default for Breadcrumb { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_new_is_clickable() {
        let s = BreadcrumbSegment::new("Home", "doc:root");
        assert!(s.is_clickable);
        assert_eq!(s.label, "Home");
        assert_eq!(s.target_id, "doc:root");
    }

    #[test]
    fn segment_non_clickable_is_false() {
        let s = BreadcrumbSegment::non_clickable("…", "breadcrumb:ellipsis");
        assert!(!s.is_clickable);
    }

    #[test]
    fn breadcrumb_new_defaults() {
        let b = Breadcrumb::new();
        assert!(b.segments.is_empty());
        assert_eq!(b.separator, " / ");
        assert_eq!(b.max_visible_segments, 5);
    }

    #[test]
    fn with_separator_chain() {
        let b = Breadcrumb::new().with_separator(" > ");
        assert_eq!(b.separator, " > ");
    }

    #[test]
    fn with_max_visible_enforces_min_1_on_zero() {
        let b = Breadcrumb::new().with_max_visible(0);
        assert_eq!(b.max_visible_segments, 1);
    }

    #[test]
    fn push_pop_clear_len_is_empty() {
        let mut b = Breadcrumb::new();
        assert!(b.is_empty());
        b.push(BreadcrumbSegment::new("a", "id:a"));
        b.push(BreadcrumbSegment::new("b", "id:b"));
        assert_eq!(b.len(), 2);
        assert!(!b.is_empty());
        let popped = b.pop().unwrap();
        assert_eq!(popped.label, "b");
        assert_eq!(b.len(), 1);
        b.clear();
        assert!(b.is_empty());
    }

    #[test]
    fn rendered_empty_returns_empty_string() {
        let b = Breadcrumb::new();
        assert_eq!(b.rendered(), "");
    }

    #[test]
    fn rendered_three_segments() {
        let mut b = Breadcrumb::new();
        b.push(BreadcrumbSegment::new("a", "id:a"));
        b.push(BreadcrumbSegment::new("b", "id:b"));
        b.push(BreadcrumbSegment::new("c", "id:c"));
        assert_eq!(b.rendered(), "a / b / c");
    }

    #[test]
    fn visible_segments_under_limit_returns_all() {
        let mut b = Breadcrumb::new().with_max_visible(5);
        for i in 0..4 {
            b.push(BreadcrumbSegment::new(format!("s{i}"), format!("id:{i}")));
        }
        assert_eq!(b.visible_segments().len(), 4);
    }

    #[test]
    fn visible_segments_at_exactly_limit_no_ellipsis() {
        let mut b = Breadcrumb::new().with_max_visible(5);
        for i in 0..5 {
            b.push(BreadcrumbSegment::new(format!("s{i}"), format!("id:{i}")));
        }
        let vis = b.visible_segments();
        assert_eq!(vis.len(), 5);
        assert!(vis.iter().all(|s| s.label != "…"));
    }

    #[test]
    fn visible_segments_over_limit_has_ellipsis() {
        let mut b = Breadcrumb::new().with_max_visible(5);
        for i in 0..8 {
            b.push(BreadcrumbSegment::new(format!("s{i}"), format!("id:{i}")));
        }
        let vis = b.visible_segments();
        assert_eq!(vis[0].label, "s0");
        assert_eq!(vis[1].label, "…");
        assert!(!vis[1].is_clickable);
        // last segment should be the last original segment
        assert_eq!(vis.last().unwrap().label, "s7");
    }

    #[test]
    fn current_and_parent_accessors() {
        let mut b = Breadcrumb::new();
        b.push(BreadcrumbSegment::new("root", "id:root"));
        b.push(BreadcrumbSegment::new("child", "id:child"));
        assert_eq!(b.current().unwrap().label, "child");
        assert_eq!(b.parent().unwrap().label, "root");
    }

    #[test]
    fn current_returns_none_when_empty() {
        let b = Breadcrumb::new();
        assert!(b.current().is_none());
    }

    #[test]
    fn parent_returns_none_when_fewer_than_two() {
        let mut b = Breadcrumb::new();
        b.push(BreadcrumbSegment::new("only", "id:only"));
        assert!(b.parent().is_none());
    }

    #[test]
    fn navigate_to_existing_truncates() {
        let mut b = Breadcrumb::new();
        b.push(BreadcrumbSegment::new("root", "id:root"));
        b.push(BreadcrumbSegment::new("section", "id:section"));
        b.push(BreadcrumbSegment::new("block", "id:block"));
        assert!(b.navigate_to("id:section"));
        assert_eq!(b.len(), 2);
        assert_eq!(b.current().unwrap().target_id, "id:section");
    }

    #[test]
    fn navigate_to_missing_returns_false_and_unchanged() {
        let mut b = Breadcrumb::new();
        b.push(BreadcrumbSegment::new("root", "id:root"));
        b.push(BreadcrumbSegment::new("child", "id:child"));
        assert!(!b.navigate_to("id:nonexistent"));
        assert_eq!(b.len(), 2);
    }
}
