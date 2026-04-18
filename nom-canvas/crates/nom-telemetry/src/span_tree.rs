// ---------------------------------------------------------------------------
// span_tree — span kinds, identifiers, tree structure, and analysis
// ---------------------------------------------------------------------------

/// The category of work a span represents.
#[derive(Debug, Clone, PartialEq)]
pub enum SpanKind {
    Http,
    Db,
    Rpc,
    Internal,
    Queue,
}

impl SpanKind {
    /// Returns `true` for spans that cross a process or network boundary.
    pub fn is_external(&self) -> bool {
        matches!(self, SpanKind::Http | SpanKind::Db | SpanKind::Rpc | SpanKind::Queue)
    }

    /// A short lowercase tag string suitable for labelling or filtering.
    pub fn kind_tag(&self) -> &'static str {
        match self {
            SpanKind::Http => "http",
            SpanKind::Db => "db",
            SpanKind::Rpc => "rpc",
            SpanKind::Internal => "internal",
            SpanKind::Queue => "queue",
        }
    }
}

// ---------------------------------------------------------------------------
// SpanId
// ---------------------------------------------------------------------------

/// A 64-bit span identifier.  The zero value is treated as invalid/absent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpanId(pub u64);

impl SpanId {
    /// Returns `true` when the id is non-zero (i.e. actually assigned).
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    /// Formats the id as a zero-padded 16-character lowercase hex string.
    pub fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Span
// ---------------------------------------------------------------------------

/// A single unit of traced work.
#[derive(Debug, Clone)]
pub struct Span {
    pub id: SpanId,
    pub parent_id: Option<SpanId>,
    pub name: String,
    pub kind: SpanKind,
    /// Elapsed wall-time for this span in microseconds.
    pub duration_us: u64,
}

impl Span {
    /// Returns `true` when this span has no parent (i.e. is a trace root).
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Returns `true` when the span took longer than 100 ms (100 000 µs).
    pub fn is_slow(&self) -> bool {
        self.duration_us > 100_000
    }

    /// A human-readable one-line summary of the span.
    pub fn summary(&self) -> String {
        format!("[{}] {} {}us", self.kind.kind_tag(), self.name, self.duration_us)
    }
}

// ---------------------------------------------------------------------------
// SpanTree
// ---------------------------------------------------------------------------

/// An in-memory collection of spans belonging to a single trace.
#[derive(Debug, Default)]
pub struct SpanTree {
    pub spans: Vec<Span>,
}

impl SpanTree {
    pub fn new() -> Self {
        SpanTree { spans: Vec::new() }
    }

    /// Appends a span to the tree.
    pub fn add(&mut self, span: Span) {
        self.spans.push(span);
    }

    /// Returns all root spans (spans without a parent).
    pub fn roots(&self) -> Vec<&Span> {
        self.spans.iter().filter(|s| s.is_root()).collect()
    }

    /// Returns all direct children of the span identified by `id`.
    pub fn children_of(&self, id: &SpanId) -> Vec<&Span> {
        self.spans
            .iter()
            .filter(|s| s.parent_id.as_ref() == Some(id))
            .collect()
    }

    /// Sum of `duration_us` across all spans in the tree.
    pub fn total_duration_us(&self) -> u64 {
        self.spans.iter().map(|s| s.duration_us).sum()
    }
}

// ---------------------------------------------------------------------------
// SpanAnalyzer
// ---------------------------------------------------------------------------

/// Stateless analysis helpers that operate on a `SpanTree`.
pub struct SpanAnalyzer;

impl SpanAnalyzer {
    /// Returns every span whose `duration_us` exceeds 100 000 µs.
    pub fn slow_spans<'a>(tree: &'a SpanTree) -> Vec<&'a Span> {
        tree.spans.iter().filter(|s| s.is_slow()).collect()
    }

    /// Returns the maximum nesting depth reachable from any root span.
    /// Returns 0 when the tree contains no spans.
    pub fn max_depth(tree: &SpanTree) -> u32 {
        if tree.spans.is_empty() {
            return 0;
        }

        let mut max = 0u32;

        // BFS from each root, tracking current depth.
        for root in tree.roots() {
            let mut queue: std::collections::VecDeque<(&Span, u32)> = std::collections::VecDeque::new();
            queue.push_back((root, 1));

            while let Some((span, depth)) = queue.pop_front() {
                if depth > max {
                    max = depth;
                }
                for child in tree.children_of(&span.id) {
                    queue.push_back((child, depth + 1));
                }
            }
        }

        max
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_span(id: u64, parent_id: Option<u64>, name: &str, kind: SpanKind, duration_us: u64) -> Span {
        Span {
            id: SpanId(id),
            parent_id: parent_id.map(SpanId),
            name: name.to_string(),
            kind,
            duration_us,
        }
    }

    // 1. SpanKind::is_external — Http/Db/Rpc/Queue are external; Internal is not
    #[test]
    fn test_span_kind_is_external() {
        assert!(SpanKind::Http.is_external());
        assert!(SpanKind::Db.is_external());
        assert!(SpanKind::Rpc.is_external());
        assert!(SpanKind::Queue.is_external());
        assert!(!SpanKind::Internal.is_external());
    }

    // 2. SpanKind::kind_tag — correct lowercase strings
    #[test]
    fn test_span_kind_tag() {
        assert_eq!(SpanKind::Http.kind_tag(), "http");
        assert_eq!(SpanKind::Db.kind_tag(), "db");
        assert_eq!(SpanKind::Rpc.kind_tag(), "rpc");
        assert_eq!(SpanKind::Internal.kind_tag(), "internal");
        assert_eq!(SpanKind::Queue.kind_tag(), "queue");
    }

    // 3. SpanId::is_valid — zero is invalid, non-zero is valid
    #[test]
    fn test_span_id_is_valid() {
        assert!(!SpanId(0).is_valid());
        assert!(SpanId(1).is_valid());
        assert!(SpanId(u64::MAX).is_valid());
    }

    // 4. SpanId::hex — zero-padded 16-char lowercase hex
    #[test]
    fn test_span_id_hex_format() {
        assert_eq!(SpanId(0).hex(), "0000000000000000");
        assert_eq!(SpanId(1).hex(), "0000000000000001");
        assert_eq!(SpanId(0xdeadbeef).hex(), "00000000deadbeef");
        assert_eq!(SpanId(u64::MAX).hex(), "ffffffffffffffff");
        // Length is always 16
        assert_eq!(SpanId(42).hex().len(), 16);
    }

    // 5. Span::is_root — no parent means root
    #[test]
    fn test_span_is_root() {
        let root = make_span(1, None, "root", SpanKind::Internal, 500);
        let child = make_span(2, Some(1), "child", SpanKind::Http, 200);
        assert!(root.is_root());
        assert!(!child.is_root());
    }

    // 6. Span::is_slow — threshold is strictly greater than 100_000 µs
    #[test]
    fn test_span_is_slow_threshold() {
        let fast = make_span(1, None, "fast", SpanKind::Internal, 100_000);
        let slow = make_span(2, None, "slow", SpanKind::Db, 100_001);
        assert!(!fast.is_slow(), "exactly 100_000 µs should not be slow");
        assert!(slow.is_slow(), "100_001 µs should be slow");
    }

    // 7. Span::summary — correct format string
    #[test]
    fn test_span_summary() {
        let span = make_span(1, None, "fetch", SpanKind::Http, 5000);
        assert_eq!(span.summary(), "[http] fetch 5000us");

        let span2 = make_span(2, None, "query", SpanKind::Db, 200_000);
        assert_eq!(span2.summary(), "[db] query 200000us");
    }

    // 8. SpanTree::children_of — returns correct direct children only
    #[test]
    fn test_span_tree_children_of() {
        let mut tree = SpanTree::new();
        tree.add(make_span(1, None, "root", SpanKind::Internal, 1000));
        tree.add(make_span(2, Some(1), "child-a", SpanKind::Http, 400));
        tree.add(make_span(3, Some(1), "child-b", SpanKind::Db, 300));
        tree.add(make_span(4, Some(2), "grandchild", SpanKind::Rpc, 100));

        let children = tree.children_of(&SpanId(1));
        assert_eq!(children.len(), 2);
        let names: Vec<&str> = children.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"child-a"));
        assert!(names.contains(&"child-b"));

        // Grandchild is NOT a direct child of root
        assert!(!names.contains(&"grandchild"));
    }

    // 9. SpanAnalyzer::slow_spans — filters correctly by threshold
    #[test]
    fn test_span_analyzer_slow_spans_filter() {
        let mut tree = SpanTree::new();
        tree.add(make_span(1, None, "fast1", SpanKind::Internal, 50_000));
        tree.add(make_span(2, None, "fast2", SpanKind::Http, 100_000));
        tree.add(make_span(3, None, "slow1", SpanKind::Db, 100_001));
        tree.add(make_span(4, None, "slow2", SpanKind::Rpc, 500_000));

        let slow = SpanAnalyzer::slow_spans(&tree);
        assert_eq!(slow.len(), 2);
        let names: Vec<&str> = slow.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"slow1"));
        assert!(names.contains(&"slow2"));
        assert!(!names.contains(&"fast1"));
        assert!(!names.contains(&"fast2"));
    }
}
