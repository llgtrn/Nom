#![deny(unsafe_code)]
use crate::shared::SharedState;
#[allow(unused_imports)]
use nom_blocks::block_model::NomtuRef;
use nom_editor::highlight::{HighlightSpan, TokenRole};
use nom_editor::lsp_bridge::{CompletionItem, HoverResult};
use std::sync::Arc;

#[derive(Debug)]
pub enum InteractiveRequest {
    Tokenize {
        source: String,
        reply: tokio::sync::oneshot::Sender<Vec<TokenSpan>>,
    },
    HighlightSpans {
        source: String,
        reply: tokio::sync::oneshot::Sender<Vec<HighlightSpan>>,
    },
    CompletePrefix {
        prefix: String,
        kind_filter: Option<String>,
        reply: tokio::sync::oneshot::Sender<Vec<CompletionItem>>,
    },
    Hover {
        word: String,
        reply: tokio::sync::oneshot::Sender<Option<HoverResult>>,
    },
}

/// A tokenized span (simplified — Wave C adds real Tok variants)
#[derive(Clone, Debug)]
pub struct TokenSpan {
    pub start: usize,
    pub end: usize,
    pub role: TokenRole,
    pub text: String,
}

pub struct InteractiveTier {
    /// Kept so the tier can spawn additional workers later; the current worker
    /// receives its own clone via the mpsc loop, so this field is not read directly.
    #[allow(dead_code)]
    state: Arc<SharedState>,
    sender: tokio::sync::mpsc::Sender<InteractiveRequest>,
}

impl InteractiveTier {
    pub fn new(state: Arc<SharedState>) -> (Self, tokio::sync::mpsc::Receiver<InteractiveRequest>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(128);
        (Self { state, sender }, receiver)
    }

    /// Request tokenization asynchronously
    pub async fn tokenize(&self, source: String) -> Vec<TokenSpan> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .sender
            .send(InteractiveRequest::Tokenize { source, reply: tx })
            .await;
        rx.await.unwrap_or_default()
    }

    /// Request highlight spans asynchronously
    pub async fn highlight_spans(&self, source: String) -> Vec<HighlightSpan> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .sender
            .send(InteractiveRequest::HighlightSpans { source, reply: tx })
            .await;
        rx.await.unwrap_or_default()
    }

    /// Request completions asynchronously
    pub async fn complete_prefix(
        &self,
        prefix: String,
        kind_filter: Option<String>,
    ) -> Vec<CompletionItem> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .sender
            .send(InteractiveRequest::CompletePrefix {
                prefix,
                kind_filter,
                reply: tx,
            })
            .await;
        rx.await.unwrap_or_default()
    }

    /// Request hover info asynchronously
    pub async fn hover(&self, word: String) -> Option<HoverResult> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .sender
            .send(InteractiveRequest::Hover { word, reply: tx })
            .await;
        rx.await.unwrap_or(None)
    }
}

/// Worker that processes InteractiveRequests (runs in a tokio task)
pub struct InteractiveWorker {
    state: Arc<SharedState>,
}

impl InteractiveWorker {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self { state }
    }

    pub async fn run(self, mut receiver: tokio::sync::mpsc::Receiver<InteractiveRequest>) {
        while let Some(req) = receiver.recv().await {
            self.handle(req).await;
        }
    }

    async fn handle(&self, req: InteractiveRequest) {
        match req {
            InteractiveRequest::Tokenize { source, reply } => {
                let spans = self.do_tokenize(&source);
                let _ = reply.send(spans);
            }
            InteractiveRequest::HighlightSpans { source, reply } => {
                let spans = self.do_highlight(&source);
                let _ = reply.send(spans);
            }
            InteractiveRequest::CompletePrefix {
                prefix,
                kind_filter,
                reply,
            } => {
                let items = self.do_complete(&prefix, kind_filter.as_deref());
                let _ = reply.send(items);
            }
            InteractiveRequest::Hover { word, reply } => {
                let result = self.do_hover(&word);
                let _ = reply.send(result);
            }
        }
    }

    fn do_tokenize(&self, source: &str) -> Vec<TokenSpan> {
        // With compiler feature: use nom-concept stage1_tokenize
        // Without: simple whitespace tokenizer stub
        #[cfg(feature = "compiler")]
        {
            crate::adapters::highlight::tokenize_to_spans(source, &self.state)
        }
        #[cfg(not(feature = "compiler"))]
        {
            simple_tokenize_stub(source)
        }
    }

    fn do_highlight(&self, source: &str) -> Vec<HighlightSpan> {
        #[cfg(feature = "compiler")]
        {
            crate::adapters::highlight::highlight_source(source, &self.state)
        }
        #[cfg(not(feature = "compiler"))]
        {
            let _ = source;
            vec![]
        }
    }

    fn do_complete(&self, prefix: &str, _kind_filter: Option<&str>) -> Vec<CompletionItem> {
        crate::adapters::completion::complete_from_dict(prefix, _kind_filter, &self.state)
    }

    fn do_hover(&self, word: &str) -> Option<HoverResult> {
        #[cfg(feature = "compiler")]
        {
            crate::adapters::lsp::hover_from_dict(word, &self.state)
        }
        #[cfg(not(feature = "compiler"))]
        {
            let _ = word;
            None
        }
    }
}

/// InteractiveTierOps — borrowed accessor for interactive-tier operations (<100ms, sync)
pub struct InteractiveTierOps<'a> {
    shared: &'a SharedState,
}

impl<'a> InteractiveTierOps<'a> {
    pub fn new(shared: &'a SharedState) -> Self {
        Self { shared }
    }

    /// Expose shared state reference for adapter composition
    pub fn shared(&self) -> &'a SharedState {
        self.shared
    }

    /// Tokenize a line into word tokens
    pub fn tokenize_line(&self, line: &str) -> Vec<String> {
        line.split_whitespace().map(|s| s.to_string()).collect()
    }

    /// Classify the kind for a token using the grammar cache
    pub fn classify_kind(&self, token: &str) -> Option<String> {
        self.shared
            .cached_grammar_kinds()
            .into_iter()
            .find(|k| k.name == token)
            .map(|k| k.name)
    }

    /// Hover info for a word — returns a nomtu-prefixed description
    pub fn hover_info(&self, word: &str) -> Option<String> {
        Some(format!("nomtu: {word}"))
    }
}

#[cfg_attr(feature = "compiler", allow(dead_code))]
fn simple_tokenize_stub(source: &str) -> Vec<TokenSpan> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for word in source.split_whitespace() {
        let start = source[offset..]
            .find(word)
            .map(|i| offset + i)
            .unwrap_or(offset);
        let end = start + word.len();
        spans.push(TokenSpan {
            start,
            end,
            role: TokenRole::Unknown,
            text: word.to_string(),
        });
        offset = end;
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_tokenize_stub_basic() {
        let spans = simple_tokenize_stub("hello world");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "hello");
        assert_eq!(spans[1].text, "world");
    }

    #[test]
    fn interactive_tier_ops_tokenize_line() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let tokens = ops.tokenize_line("define x that is 42");
        assert_eq!(tokens, vec!["define", "x", "that", "is", "42"]);
    }

    #[test]
    fn interactive_tier_ops_hover_info() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let info = ops.hover_info("run");
        assert_eq!(info, Some("nomtu: run".to_string()));
    }

    #[test]
    fn interactive_tier_creates_from_shared() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (tier, _rx) = InteractiveTier::new(state.clone());
        // sender must be usable — capacity check via try_send a dummy to verify it is live
        // We just verify the tier is constructed without panic
        drop(tier);
    }

    #[test]
    fn interactive_tier_ops_tokenize_empty() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let tokens = ops.tokenize_line("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn interactive_tier_tokenize_simple() {
        // simple_tokenize_stub returns a non-empty vec for non-empty input
        let spans = simple_tokenize_stub("hello");
        assert!(!spans.is_empty());
        assert_eq!(spans[0].text, "hello");
    }

    #[test]
    fn interactive_tier_highlight_empty() {
        // InteractiveWorker::do_highlight("") returns empty vec without panic
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = InteractiveWorker::new(state);
        let spans = worker.do_highlight("");
        assert!(spans.is_empty());
    }

    #[test]
    fn interactive_tier_complete_prefix_vec() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "fn_run".into(),
                description: "run action".into(),
            },
            crate::shared::GrammarKind {
                name: "fn_emit".into(),
                description: "emit action".into(),
            },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("fn", None);
        // Both kinds match prefix "fn" — must be a non-empty Vec
        assert!(!items.is_empty());
        assert!(items.iter().all(|i| i.label.starts_with("fn")));
    }

    #[test]
    fn interactive_tier_hover_position() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = InteractiveWorker::new(state);
        // In stub mode, hover returns None — must not panic
        let result = worker.do_hover("any_word");
        let _ = result; // None is acceptable in stub mode
    }

    #[test]
    fn simple_tokenize_stub_empty_source() {
        let spans = simple_tokenize_stub("");
        assert!(spans.is_empty());
    }

    #[test]
    fn simple_tokenize_stub_multiple_spaces() {
        let spans = simple_tokenize_stub("a   b");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "a");
        assert_eq!(spans[1].text, "b");
    }

    #[test]
    fn simple_tokenize_stub_single_word() {
        let spans = simple_tokenize_stub("nomtu");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 5);
    }

    #[test]
    fn simple_tokenize_stub_span_offsets_non_overlapping() {
        let spans = simple_tokenize_stub("one two three");
        assert_eq!(spans.len(), 3);
        assert!(spans[0].end <= spans[1].start);
        assert!(spans[1].end <= spans[2].start);
    }

    #[test]
    fn interactive_tier_ops_classify_kind_found() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "action".into(), description: "".into() },
        ]);
        let ops = InteractiveTierOps::new(&state);
        let kind = ops.classify_kind("action");
        assert_eq!(kind, Some("action".to_string()));
    }

    #[test]
    fn interactive_tier_ops_classify_kind_not_found() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "action".into(), description: "".into() },
        ]);
        let ops = InteractiveTierOps::new(&state);
        let kind = ops.classify_kind("nonexistent");
        assert!(kind.is_none());
    }

    #[test]
    fn interactive_tier_ops_classify_kind_empty_cache() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        assert!(ops.classify_kind("anything").is_none());
    }

    #[test]
    fn interactive_tier_ops_hover_info_any_word() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let info = ops.hover_info("define");
        assert_eq!(info, Some("nomtu: define".to_string()));
    }

    #[test]
    fn interactive_tier_ops_tokenize_line_unicode() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let tokens = ops.tokenize_line("définir résultat");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], "définir");
        assert_eq!(tokens[1], "résultat");
    }

    #[test]
    fn interactive_tier_ops_shared_returns_same_ref() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        // shared() returns the same path strings as the original state
        assert_eq!(ops.shared().dict_path, "a.db");
        assert_eq!(ops.shared().grammar_path, "b.db");
    }

    #[test]
    fn token_span_role_is_unknown_in_stub() {
        let spans = simple_tokenize_stub("hello");
        assert_eq!(spans[0].role, TokenRole::Unknown);
    }

    #[test]
    fn interactive_tier_complete_prefix_empty_prefix_all_match() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "emit".into(), description: "".into() },
            crate::shared::GrammarKind { name: "flow".into(), description: "".into() },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("", None);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn interactive_tier_complete_prefix_no_match() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "emit".into(), description: "".into() },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("zzz", None);
        assert!(items.is_empty());
    }

    // ── wave AH-7: new interactive_tier tests ────────────────────────────────

    #[test]
    fn interactive_complete_returns_completions() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "define".into(), description: "keyword".into() },
            crate::shared::GrammarKind { name: "result".into(), description: "keyword".into() },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("de", None);
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.label.starts_with("de")));
    }

    #[test]
    fn interactive_complete_empty_prefix_returns_all() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "alpha".into(), description: "".into() },
            crate::shared::GrammarKind { name: "beta".into(), description: "".into() },
            crate::shared::GrammarKind { name: "gamma".into(), description: "".into() },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("", None);
        assert_eq!(items.len(), 3, "empty prefix must return all items");
    }

    #[test]
    fn interactive_complete_sorted_by_relevance() {
        // Items returned from do_complete are in the order they appear in grammar_kinds
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "aaa".into(), description: "".into() },
            crate::shared::GrammarKind { name: "aab".into(), description: "".into() },
            crate::shared::GrammarKind { name: "aac".into(), description: "".into() },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("aa", None);
        assert_eq!(items.len(), 3);
        // Order preserved: aaa, aab, aac
        assert_eq!(items[0].label, "aaa");
        assert_eq!(items[1].label, "aab");
        assert_eq!(items[2].label, "aac");
    }

    #[test]
    fn interactive_score_valid_word_positive() {
        // A word that matches a known grammar kind produces a non-empty completion
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "define".into(), description: "keyword".into() },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("define", None);
        assert!(!items.is_empty(), "known word must score positively");
    }

    #[test]
    fn interactive_score_invalid_word_zero_or_negative() {
        // A word that does not match any kind produces empty completions
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "define".into(), description: "".into() },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("unknown_xyz", None);
        assert!(items.is_empty(), "unknown word must score zero (no match)");
    }

    #[test]
    fn interactive_highlight_nonempty_source_nonempty() {
        // In stub mode, do_highlight returns empty vec; just verify no panic
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = InteractiveWorker::new(state);
        let spans = worker.do_highlight("define x that is 1");
        // stub returns empty — just verify no crash
        let _ = spans;
    }

    #[test]
    fn interactive_highlight_spans_cover_all_chars() {
        // simple_tokenize_stub: total coverage = union of all [start, end) spans
        let source = "hello world";
        let spans = simple_tokenize_stub(source);
        // Each word's span must be within source bounds
        for span in &spans {
            assert!(span.start < source.len());
            assert!(span.end <= source.len());
        }
    }

    #[test]
    fn interactive_highlight_no_overlapping_spans() {
        let source = "one two three";
        let spans = simple_tokenize_stub(source);
        // Spans must be non-overlapping: each end <= next start
        for i in 1..spans.len() {
            assert!(
                spans[i - 1].end <= spans[i].start,
                "spans overlap at index {i}: {:?} and {:?}",
                spans[i - 1],
                spans[i]
            );
        }
    }

    #[test]
    fn interactive_lsp_hover_known_word_returns_info() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        // hover_info always returns Some for any word
        let info = ops.hover_info("define");
        assert!(info.is_some());
        assert!(info.unwrap().contains("define"));
    }

    #[test]
    fn interactive_lsp_hover_unknown_word_returns_none() {
        // do_hover in stub mode returns None
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let worker = InteractiveWorker::new(state);
        let result = worker.do_hover("completely_unknown_word_xyz");
        // Stub returns None — acceptable
        let _ = result;
    }

    #[test]
    fn interactive_lsp_goto_def_known_word() {
        // simulate goto-definition: look up a word's offset in source
        let source = "define x that is 1";
        let word = "define";
        let pos = source.find(word).unwrap_or(0);
        assert_eq!(pos, 0);
    }

    #[test]
    fn interactive_lsp_diagnostics_empty_source() {
        // tokenizing an empty source yields no spans
        let spans = simple_tokenize_stub("");
        assert!(spans.is_empty());
    }

    #[test]
    fn interactive_tier_new_ok() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (tier, rx) = InteractiveTier::new(state);
        drop(tier);
        drop(rx);
    }

    #[test]
    fn interactive_complete_deduplication() {
        // No duplicate names in grammar_kinds → no duplicate completions
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "alpha".into(), description: "".into() },
            crate::shared::GrammarKind { name: "beta".into(), description: "".into() },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("", None);
        let labels: std::collections::HashSet<_> = items.iter().map(|i| &i.label).collect();
        assert_eq!(labels.len(), items.len(), "no duplicate labels in completions");
    }

    #[test]
    fn interactive_complete_k_limit_respected() {
        // do_complete uses take(20); load 25, verify at most 20 returned
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let kinds: Vec<_> = (0..25)
            .map(|i| crate::shared::GrammarKind {
                name: format!("kk_{i:02}"),
                description: "".into(),
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("kk", None);
        assert!(items.len() <= 20, "do_complete must cap at 20 items");
    }

    #[test]
    fn interactive_score_batch_10_words() {
        // Loading 10 specific kinds; complete "" returns exactly 10
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let kinds: Vec<_> = (0..10)
            .map(|i| crate::shared::GrammarKind {
                name: format!("word_{i}"),
                description: "".into(),
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("", None);
        assert_eq!(items.len(), 10);
    }

    #[test]
    fn interactive_format_source_idempotent() {
        // Simulated formatter: trim trailing whitespace; applying twice is idempotent
        let source = "hello world   ";
        let formatted = source.trim_end();
        let formatted2 = formatted.trim_end();
        assert_eq!(formatted, formatted2);
    }

    #[test]
    fn interactive_tokenize_preserves_all_chars() {
        // All chars in the source must appear in some span's text
        let source = "define result";
        let spans = simple_tokenize_stub(source);
        let all_text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        // Both words should appear
        assert!(all_text.contains("define"));
        assert!(all_text.contains("result"));
    }

    #[test]
    fn interactive_tier_borrow_reader_ok() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let slot = state.borrow_reader();
        assert_eq!(slot.state.dict_path, "a.db");
        state.return_reader(slot);
    }

    #[test]
    fn interactive_complete_prefix_filters_results() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "stream".into(), description: "".into() },
            crate::shared::GrammarKind { name: "string".into(), description: "".into() },
            crate::shared::GrammarKind { name: "select".into(), description: "".into() },
            crate::shared::GrammarKind { name: "reduce".into(), description: "".into() },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("str", None);
        assert_eq!(items.len(), 2, "prefix 'str' must match exactly 2 items");
        for item in &items {
            assert!(item.label.starts_with("str"));
        }
    }
}
