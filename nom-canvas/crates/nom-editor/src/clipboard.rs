//! In-memory clipboard (text + rich variants).
//!
//! Runtime crates wrap this with OS-clipboard integration; the editor
//! only deals with this in-memory model so commands remain testable.
#![deny(unsafe_code)]

#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardPayload {
    /// Plain-text fallback (always present).
    pub plain_text: String,
    /// Optional rich variants keyed by MIME type (e.g. "text/html", "application/x-nom-blocks").
    pub variants: Vec<(String, Vec<u8>)>,
}

impl ClipboardPayload {
    pub fn plain(text: impl Into<String>) -> Self {
        Self { plain_text: text.into(), variants: Vec::new() }
    }
    pub fn with_variant(mut self, mime: impl Into<String>, data: Vec<u8>) -> Self {
        self.variants.push((mime.into(), data));
        self
    }
    pub fn variant(&self, mime: &str) -> Option<&[u8]> {
        self.variants.iter().find(|(m, _)| m == mime).map(|(_, d)| d.as_slice())
    }
    pub fn has_variant(&self, mime: &str) -> bool { self.variant(mime).is_some() }
    pub fn text_len(&self) -> usize { self.plain_text.len() }
    pub fn is_empty(&self) -> bool { self.plain_text.is_empty() && self.variants.is_empty() }
}

#[derive(Default)]
pub struct Clipboard {
    stack: Vec<ClipboardPayload>,
    max_history: usize,
}

impl Clipboard {
    pub fn new(max_history: usize) -> Self {
        Self { stack: Vec::new(), max_history: max_history.max(1) }
    }

    /// Push a new payload onto the clipboard history.  Oldest entries are
    /// evicted when we exceed `max_history`.
    pub fn push(&mut self, payload: ClipboardPayload) {
        self.stack.push(payload);
        while self.stack.len() > self.max_history {
            self.stack.remove(0);
        }
    }

    /// Latest payload.
    pub fn current(&self) -> Option<&ClipboardPayload> { self.stack.last() }

    /// Historical payload at offset N from most-recent (0 = current, 1 = previous, ...).
    pub fn at_offset(&self, offset: usize) -> Option<&ClipboardPayload> {
        if offset >= self.stack.len() { return None; }
        Some(&self.stack[self.stack.len() - 1 - offset])
    }

    pub fn len(&self) -> usize { self.stack.len() }
    pub fn is_empty(&self) -> bool { self.stack.is_empty() }
    pub fn clear(&mut self) { self.stack.clear(); }
}

/// Snapshot + helpers for executing copy/cut/paste over a source string.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardSnapshot {
    pub before_cursor: usize,
    pub before_source: String,
    pub payload: ClipboardPayload,
}

impl Default for ClipboardSnapshot {
    fn default() -> Self {
        Self { before_cursor: 0, before_source: String::new(), payload: ClipboardPayload::plain("") }
    }
}

/// Execute a copy: no mutation; returns the payload for pushing.
pub fn copy_range(source: &str, range: std::ops::Range<usize>) -> ClipboardPayload {
    ClipboardPayload::plain(source.get(range).unwrap_or("").to_string())
}

/// Execute a cut: returns the payload + a mutated source with the range removed.
pub fn cut_range(source: &str, range: std::ops::Range<usize>) -> (ClipboardPayload, String) {
    let payload = copy_range(source, range.clone());
    let mut out = String::with_capacity(source.len());
    out.push_str(&source[..range.start.min(source.len())]);
    out.push_str(&source[range.end.min(source.len())..]);
    (payload, out)
}

/// Execute a paste: inserts the payload's plain_text at `at`.  Returns new source + new cursor position.
pub fn paste_at(source: &str, at: usize, payload: &ClipboardPayload) -> (String, usize) {
    let at = at.min(source.len());
    let mut out = String::with_capacity(source.len() + payload.plain_text.len());
    out.push_str(&source[..at]);
    out.push_str(&payload.plain_text);
    out.push_str(&source[at..]);
    (out, at + payload.plain_text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_plain_empty_variants() {
        let p = ClipboardPayload::plain("hello");
        assert_eq!(p.plain_text, "hello");
        assert!(p.variants.is_empty());
    }

    #[test]
    fn with_variant_adds_and_retrieves() {
        let data = b"<b>bold</b>".to_vec();
        let p = ClipboardPayload::plain("bold").with_variant("text/html", data.clone());
        assert_eq!(p.variant("text/html"), Some(data.as_slice()));
        assert!(p.has_variant("text/html"));
    }

    #[test]
    fn variant_miss_returns_none() {
        let p = ClipboardPayload::plain("hi");
        assert_eq!(p.variant("text/html"), None);
        assert!(!p.has_variant("text/html"));
    }

    #[test]
    fn text_len_and_is_empty() {
        let empty = ClipboardPayload::plain("");
        assert_eq!(empty.text_len(), 0);
        assert!(empty.is_empty());

        let non_empty = ClipboardPayload::plain("abc");
        assert_eq!(non_empty.text_len(), 3);
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn is_empty_false_when_variant_present() {
        let p = ClipboardPayload::plain("").with_variant("application/x-nom-blocks", vec![1, 2, 3]);
        assert!(!p.is_empty());
    }

    #[test]
    fn clipboard_new_minimum_history_1() {
        let cb = Clipboard::new(0);
        assert_eq!(cb.max_history, 1);
    }

    #[test]
    fn push_and_current() {
        let mut cb = Clipboard::new(5);
        let p = ClipboardPayload::plain("foo");
        cb.push(p.clone());
        assert_eq!(cb.current(), Some(&p));
    }

    #[test]
    fn push_evicts_oldest_when_exceeds_max() {
        let mut cb = Clipboard::new(3);
        for i in 0..4u8 {
            cb.push(ClipboardPayload::plain(format!("item{i}")));
        }
        assert_eq!(cb.len(), 3);
        assert_eq!(cb.current().unwrap().plain_text, "item3");
        // oldest (item0) evicted; item1 is now index 0
        assert_eq!(cb.at_offset(2).unwrap().plain_text, "item1");
    }

    #[test]
    fn at_offset_0_is_current_1_is_previous() {
        let mut cb = Clipboard::new(5);
        cb.push(ClipboardPayload::plain("first"));
        cb.push(ClipboardPayload::plain("second"));
        assert_eq!(cb.at_offset(0).unwrap().plain_text, "second");
        assert_eq!(cb.at_offset(1).unwrap().plain_text, "first");
    }

    #[test]
    fn at_offset_past_len_returns_none() {
        let mut cb = Clipboard::new(5);
        cb.push(ClipboardPayload::plain("only"));
        assert_eq!(cb.at_offset(1), None);
        assert_eq!(cb.at_offset(100), None);
    }

    #[test]
    fn clear_empties_stack() {
        let mut cb = Clipboard::new(5);
        cb.push(ClipboardPayload::plain("a"));
        cb.push(ClipboardPayload::plain("b"));
        cb.clear();
        assert!(cb.is_empty());
        assert_eq!(cb.len(), 0);
    }

    #[test]
    fn copy_range_returns_correct_slice() {
        let s = "hello world";
        let p = copy_range(s, 6..11);
        assert_eq!(p.plain_text, "world");
    }

    #[test]
    fn copy_range_out_of_bounds_returns_empty() {
        let s = "hi";
        let p = copy_range(s, 10..20);
        assert_eq!(p.plain_text, "");
    }

    #[test]
    fn cut_range_removes_range() {
        let s = "hello world";
        let (payload, out) = cut_range(s, 5..11);
        assert_eq!(payload.plain_text, " world");
        assert_eq!(out, "hello");
    }

    #[test]
    fn cut_range_out_of_bounds_saturates() {
        let s = "abc";
        let (payload, out) = cut_range(s, 10..20);
        assert_eq!(payload.plain_text, "");
        assert_eq!(out, "abc");
    }

    #[test]
    fn paste_at_inserts_text() {
        let s = "hello world";
        let p = ClipboardPayload::plain("beautiful ");
        let (out, cursor) = paste_at(s, 6, &p);
        assert_eq!(out, "hello beautiful world");
        assert_eq!(cursor, 16);
    }

    #[test]
    fn paste_at_out_of_bounds_saturates_to_end() {
        let s = "abc";
        let p = ClipboardPayload::plain("XY");
        let (out, cursor) = paste_at(s, 100, &p);
        assert_eq!(out, "abcXY");
        assert_eq!(cursor, 5);
    }

    #[test]
    fn paste_at_empty_payload_unchanged() {
        let s = "abc";
        let p = ClipboardPayload::plain("");
        let (out, cursor) = paste_at(s, 1, &p);
        assert_eq!(out, "abc");
        assert_eq!(cursor, 1);
    }
}
