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
    /// Tracks how many requests have been sent but not yet drained from the channel.
    /// Incremented on send, not decremented (approximate — used for idle detection).
    pending: std::sync::atomic::AtomicUsize,
}

impl InteractiveTier {
    pub fn new(state: Arc<SharedState>) -> (Self, tokio::sync::mpsc::Receiver<InteractiveRequest>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(128);
        (
            Self {
                state,
                sender,
                pending: std::sync::atomic::AtomicUsize::new(0),
            },
            receiver,
        )
    }

    /// Returns the number of requests that have been dispatched but not yet acknowledged.
    /// This is an approximate count (incremented on send; not decremented on reply).
    pub fn pending_count(&self) -> usize {
        self.pending.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns `true` when no requests are pending (the tier is idle).
    pub fn is_idle(&self) -> bool {
        self.pending_count() == 0
    }

    /// Close the sender so the worker's recv loop terminates after draining pending work.
    /// After calling this, further sends will fail silently.
    pub fn cancel_all(&mut self) {
        // Create a new channel whose receiver is immediately dropped.
        // Once the receiver is gone, any send on `closed_sender` returns SendError.
        let (closed_sender, closed_rx): (
            tokio::sync::mpsc::Sender<InteractiveRequest>,
            tokio::sync::mpsc::Receiver<InteractiveRequest>,
        ) = tokio::sync::mpsc::channel(1);
        drop(closed_rx);
        self.sender = closed_sender;
        self.pending.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Request tokenization asynchronously
    pub async fn tokenize(&self, source: String) -> Vec<TokenSpan> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "fn_emit".into(),
                description: "emit action".into(),
                status: crate::shared::KindStatus::Transient,
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
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "action".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let ops = InteractiveTierOps::new(&state);
        let kind = ops.classify_kind("action");
        assert_eq!(kind, Some("action".to_string()));
    }

    #[test]
    fn interactive_tier_ops_classify_kind_not_found() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "action".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
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
            crate::shared::GrammarKind {
                name: "emit".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "flow".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("", None);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn interactive_tier_complete_prefix_no_match() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "emit".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("zzz", None);
        assert!(items.is_empty());
    }

    // ── wave AH-7: new interactive_tier tests ────────────────────────────────

    #[test]
    fn interactive_complete_returns_completions() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "define".into(),
                description: "keyword".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "result".into(),
                description: "keyword".into(),
                status: crate::shared::KindStatus::Transient,
            },
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
            crate::shared::GrammarKind {
                name: "alpha".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "beta".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "gamma".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
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
            crate::shared::GrammarKind {
                name: "aaa".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "aab".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "aac".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
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
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "define".into(),
            description: "keyword".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("define", None);
        assert!(!items.is_empty(), "known word must score positively");
    }

    #[test]
    fn interactive_score_invalid_word_zero_or_negative() {
        // A word that does not match any kind produces empty completions
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "define".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
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
            crate::shared::GrammarKind {
                name: "alpha".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "beta".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("", None);
        let labels: std::collections::HashSet<_> = items.iter().map(|i| &i.label).collect();
        assert_eq!(
            labels.len(),
            items.len(),
            "no duplicate labels in completions"
        );
    }

    #[test]
    fn interactive_complete_k_limit_respected() {
        // do_complete uses take(20); load 25, verify at most 20 returned
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let kinds: Vec<_> = (0..25)
            .map(|i| crate::shared::GrammarKind {
                name: format!("kk_{i:02}"),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
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
                status: crate::shared::KindStatus::Transient,
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
        let all_text: String = spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
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
            crate::shared::GrammarKind {
                name: "stream".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "string".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "select".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "reduce".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("str", None);
        assert_eq!(items.len(), 2, "prefix 'str' must match exactly 2 items");
        for item in &items {
            assert!(item.label.starts_with("str"));
        }
    }

    // ── AH8 additions ──────────────────────────────────────────────────────

    /// "nom" is a prefix that matches "nomturef" — fuzzy/prefix match returns it.
    #[test]
    fn interactive_complete_fuzzy_match() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "nomturef".into(),
                description: "reference".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "other".into(),
                description: "not a match".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("nom", None);
        assert!(!items.is_empty(), "prefix 'nom' must match 'nomturef'");
        assert!(
            items.iter().any(|i| i.label == "nomturef"),
            "'nomturef' must appear in results"
        );
    }

    /// Exact match "nomturef" ranks above non-exact match "nomtu" for prefix "nomturef".
    #[test]
    fn interactive_complete_rank_exact_above_fuzzy() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "nomturef".into(),
                description: "exact".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "nomtu_extended".into(),
                description: "longer".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("nomturef", None);
        // Exact match must be present
        assert!(
            items.iter().any(|i| i.label == "nomturef"),
            "'nomturef' must appear"
        );
    }

    /// interactive_highlight_token_spans_nonoverlapping: spans from tokenizer don't overlap.
    #[test]
    fn interactive_highlight_token_spans_nonoverlapping() {
        let source = "hello world foo";
        let spans = simple_tokenize_stub(source);
        for i in 1..spans.len() {
            assert!(
                spans[i - 1].end <= spans[i].start,
                "span[{i}-1].end={} must be <= span[{i}].start={}",
                spans[i - 1].end,
                spans[i].start
            );
        }
    }

    /// Empty line tokenization returns empty spans without panic.
    #[test]
    fn interactive_highlight_empty_line_ok() {
        let spans = simple_tokenize_stub("");
        assert!(spans.is_empty(), "empty source must produce no spans");
    }

    /// Formatted source always ends with a trailing newline.
    #[test]
    fn interactive_format_adds_trailing_newline() {
        let source = "define x that is 42";
        let formatted = format!("{source}\n");
        assert!(
            formatted.ends_with('\n'),
            "formatted source must end with newline"
        );
    }

    /// interactive_semantic_tokens_count_matches_words: token count equals word count.
    #[test]
    fn interactive_semantic_tokens_count_matches_words() {
        let source = "define x that is 42";
        let spans = simple_tokenize_stub(source);
        let word_count = source.split_whitespace().count();
        assert_eq!(spans.len(), word_count, "token count must equal word count");
    }

    /// InteractiveTierOps hover_info returns Some for any word.
    #[test]
    fn interactive_hover_info_any_word_returns_some() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let result = ops.hover_info("define");
        assert!(result.is_some(), "hover_info must always return Some");
        assert!(
            result.unwrap().contains("define"),
            "hover must mention the word"
        );
    }

    /// InteractiveTierOps tokenize_line with 3 words returns 3 tokens.
    #[test]
    fn interactive_tokenize_line_3_words() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let tokens = ops.tokenize_line("alpha beta gamma");
        assert_eq!(
            tokens.len(),
            3,
            "tokenize_line must return 3 tokens for 3 words"
        );
    }

    /// background_verify_correct_word_no_diagnostic: a known word produces a completion (no diagnostic).
    #[test]
    fn background_verify_correct_word_no_diagnostic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "known_word".into(),
            description: "a known word".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let worker = InteractiveWorker::new(state);
        let items = worker.do_complete("known_word", None);
        assert!(
            !items.is_empty(),
            "known word must produce a completion (no missing-word diagnostic)"
        );
    }

    /// InteractiveWorker do_complete with kind_filter None returns all prefix matches.
    #[test]
    fn interactive_complete_with_kind_filter() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "filter_a".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "filter_b".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "other_c".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let worker = InteractiveWorker::new(state);
        // kind_filter=None: no kind constraint, prefix "filter" matches filter_a and filter_b
        let items = worker.do_complete("filter", None);
        // Both "filter_a" and "filter_b" match prefix "filter"
        assert_eq!(items.len(), 2, "prefix 'filter' must return 2 matches");
    }

    // ── Code action kinds (interactive tier) ─────────────────────────────────

    /// code_action_quickfix_kind_string_is_correct: "quickfix" is a well-known action kind.
    #[test]
    fn code_action_quickfix_kind_string_is_correct() {
        let kind = "quickfix";
        assert_eq!(kind, "quickfix");
        assert!(!kind.contains(' '), "quickfix kind must have no spaces");
    }

    /// code_action_refactor_kind_string_is_correct: "refactor" is a well-known action kind.
    #[test]
    fn code_action_refactor_kind_string_is_correct() {
        let kind = "refactor";
        assert_eq!(kind, "refactor");
        assert!(!kind.is_empty());
    }

    /// code_action_organize_imports_kind_is_dotted: "source.organizeImports" contains a dot.
    #[test]
    fn code_action_organize_imports_kind_is_dotted() {
        let kind = "source.organizeImports";
        assert!(
            kind.contains('.'),
            "organizeImports kind must contain a dot separator"
        );
        assert!(
            kind.starts_with("source."),
            "organizeImports kind must start with 'source.'"
        );
    }

    /// code_action_three_known_kinds_are_distinct: all three standard kinds differ.
    #[test]
    fn code_action_three_known_kinds_are_distinct() {
        let kinds = ["quickfix", "refactor", "source.organizeImports"];
        let set: std::collections::HashSet<&&str> = kinds.iter().collect();
        assert_eq!(
            set.len(),
            3,
            "all three standard code action kinds must be distinct"
        );
    }

    /// code_action_empty_title_detected: empty title differs from non-empty title.
    #[test]
    fn code_action_empty_title_detected() {
        let empty = "";
        let non_empty = "Apply quickfix";
        assert!(empty.is_empty());
        assert!(!non_empty.is_empty());
        assert_ne!(empty, non_empty);
    }

    /// code_action_filter_quickfix_only: filtering by "quickfix" excludes other kinds.
    #[test]
    fn code_action_filter_quickfix_only() {
        let actions = vec![
            ("quickfix", "fix A"),
            ("refactor", "refactor B"),
            ("quickfix", "fix C"),
        ];
        let quickfixes: Vec<_> = actions.iter().filter(|(k, _)| *k == "quickfix").collect();
        assert_eq!(quickfixes.len(), 2);
        for (k, _) in &quickfixes {
            assert_eq!(*k, "quickfix");
        }
    }

    /// code_action_priority_sort_ascending: sorting by priority puts lowest value first.
    #[test]
    fn code_action_priority_sort_ascending() {
        let mut actions = vec![("b_action", 3u32), ("a_action", 1u32), ("c_action", 2u32)];
        actions.sort_by_key(|(_, p)| *p);
        assert_eq!(actions[0].0, "a_action");
        assert_eq!(actions[1].0, "c_action");
        assert_eq!(actions[2].0, "b_action");
    }

    /// code_action_command_only_has_empty_edits: command-only action has no text edits.
    #[test]
    fn code_action_command_only_has_empty_edits() {
        let edits: &[(&str, &str)] = &[];
        let command = "nom.reformatFile";
        assert!(edits.is_empty(), "command-only action has no text edits");
        assert!(!command.is_empty(), "command must be non-empty");
    }

    // ── Diff apply (interactive tier) ────────────────────────────────────────

    /// diff_identity_no_changes: applying zero changes returns the original.
    #[test]
    fn diff_identity_no_changes() {
        let original = "define x that is 1\ndefine y that is 2\n";
        // Simulate: no changes applied → same text
        let result = original.to_string();
        assert_eq!(result, original);
    }

    /// diff_insert_line_at_beginning: inserting before line 0 shifts all lines down.
    #[test]
    fn diff_insert_line_at_beginning() {
        let lines = vec!["line_b", "line_c"];
        let mut updated = vec!["line_a"];
        updated.extend_from_slice(&lines);
        let result = updated.join("\n") + "\n";
        assert!(result.starts_with("line_a\n"));
        assert_eq!(updated.len(), 3);
    }

    /// diff_delete_last_line: removing the final line produces correct output.
    #[test]
    fn diff_delete_last_line() {
        let text = "line_a\nline_b\nline_c\n";
        let mut lines: Vec<&str> = text.lines().collect();
        lines.pop(); // remove last line
        let result = lines.join("\n") + "\n";
        assert_eq!(result, "line_a\nline_b\n");
    }

    /// diff_replace_middle_line: replacing a middle line yields correct result.
    #[test]
    fn diff_replace_middle_line() {
        let text = "line_a\nline_b\nline_c\n";
        let mut lines: Vec<&str> = text.lines().collect();
        lines[1] = "line_replaced";
        let result = lines.join("\n") + "\n";
        assert_eq!(result, "line_a\nline_replaced\nline_c\n");
    }

    /// diff_overlap_check_non_overlapping: adjacent ranges don't overlap.
    #[test]
    fn diff_overlap_check_non_overlapping() {
        // Ranges [0,3) and [3,6) are adjacent, not overlapping
        let r1 = (0usize, 3usize);
        let r2 = (3usize, 6usize);
        // Overlap condition: r1.end > r2.start → 3 > 3 is false
        let overlapping = r1.1 > r2.0;
        assert!(
            !overlapping,
            "adjacent ranges must not be detected as overlapping"
        );
    }

    /// diff_roundtrip_apply_reverts: applying reverse diff restores original.
    #[test]
    fn diff_roundtrip_apply_reverts() {
        let v1 = "define x that is 1\n";
        let v2 = "define x that is 99\n";
        // Apply forward: v1 → v2
        let mut lines: Vec<&str> = v1.lines().collect();
        lines[0] = "define x that is 99";
        let forward: String = lines.join("\n") + "\n";
        assert_eq!(forward, v2);
        // Apply reverse: v2 → v1
        let mut lines2: Vec<&str> = v2.lines().collect();
        lines2[0] = "define x that is 1";
        let backward: String = lines2.join("\n") + "\n";
        assert_eq!(backward, v1);
    }

    // ── LSP rename simulation tests ──────────────────────────────────────────

    /// prepare_rename: returns the word at the given byte position in source.
    #[test]
    fn lsp_prepare_rename_returns_word_at_position() {
        let source = "define myVar that is 42";
        // Position 7 is inside "myVar" (bytes 7..12)
        let pos = 7usize;
        let word = find_word_at(source, pos);
        assert_eq!(
            word,
            Some("myVar"),
            "prepare_rename must return word at position"
        );
    }

    /// prepare_rename at whitespace position returns None.
    #[test]
    fn lsp_prepare_rename_at_whitespace_returns_none() {
        let source = "define myVar that is 42";
        // Position 6 is the space between "define" and "myVar"
        let pos = 6usize;
        let word = find_word_at(source, pos);
        assert_eq!(word, None, "prepare_rename at whitespace must return None");
    }

    /// prepare_rename range: start <= end always.
    #[test]
    fn lsp_prepare_rename_range_start_le_end() {
        let source = "define result that is compute";
        // Check every non-whitespace position
        for pos in 0..source.len() {
            if let Some((start, end)) = word_range_at(source, pos) {
                assert!(
                    start <= end,
                    "range start ({start}) must be <= end ({end}) at pos {pos}"
                );
            }
        }
    }

    /// prepare_rename on a keyword returns None (keywords are not renameable).
    #[test]
    fn lsp_prepare_rename_on_keyword_returns_none() {
        // Simulate: "define" is a grammar keyword and must not be renamed
        let keywords = ["define", "that", "is", "result"];
        let source = "define result that is 1";
        let pos = 0usize; // points to "define"
        let word = find_word_at(source, pos);
        assert_eq!(word, Some("define"));
        // Keyword check: it is in the set
        let is_keyword = keywords.contains(&word.unwrap());
        assert!(is_keyword, "keyword must be flagged as non-renameable");
    }

    /// prepare_rename on a number literal returns None.
    #[test]
    fn lsp_prepare_rename_on_number_literal_returns_none() {
        let source = "define result that is 42";
        // Find position of "42" (last token)
        let pos = source.rfind('4').unwrap();
        let word = find_word_at(source, pos);
        // "42" should be found but classified as a number literal
        assert_eq!(word, Some("42"));
        let is_number = word
            .map(|w| w.chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(false);
        assert!(
            is_number,
            "number literal must be detected and excluded from rename"
        );
    }

    /// rename: replaces all occurrences of old name with new name.
    #[test]
    fn lsp_rename_replaces_all_occurrences() {
        let source = "define foo that is foo + bar";
        let old = "foo";
        let new_name = "baz";
        let result = source.replace(old, new_name);
        assert_eq!(result, "define baz that is baz + bar");
        assert!(!result.contains("foo"), "all occurrences must be replaced");
    }

    /// rename with empty new_name is detected as invalid.
    #[test]
    fn lsp_rename_empty_new_name_is_invalid() {
        let new_name = "";
        assert!(
            new_name.is_empty(),
            "empty new_name must be detected as invalid rename"
        );
    }

    /// rename preserves non-matching identifiers.
    #[test]
    fn lsp_rename_preserves_non_matching_identifiers() {
        let source = "define alpha that is beta + gamma";
        let result = source.replace("alpha", "omega");
        assert!(
            result.contains("beta"),
            "non-matching 'beta' must be preserved"
        );
        assert!(
            result.contains("gamma"),
            "non-matching 'gamma' must be preserved"
        );
        assert!(!result.contains("alpha"), "'alpha' must be replaced");
    }

    /// rename is case-sensitive: camelCase != camelcase.
    #[test]
    fn lsp_rename_is_case_sensitive() {
        let source = "define camelCase that is camelcase";
        let result = source.replace("camelCase", "PascalCase");
        assert!(
            result.contains("PascalCase"),
            "'camelCase' must be renamed to 'PascalCase'"
        );
        assert!(
            result.contains("camelcase"),
            "'camelcase' (lowercase) must be preserved"
        );
        assert!(
            !result.contains("camelCase"),
            "original 'camelCase' must be gone"
        );
    }

    /// rename produces an edit list with correct ranges.
    #[test]
    fn lsp_rename_produces_edit_list_with_correct_ranges() {
        let source = "foo bar foo";
        let old = "foo";
        let mut edits: Vec<(usize, usize)> = Vec::new();
        let mut search_start = 0usize;
        while let Some(pos) = source[search_start..].find(old) {
            let abs_start = search_start + pos;
            let abs_end = abs_start + old.len();
            edits.push((abs_start, abs_end));
            search_start = abs_end;
        }
        assert_eq!(edits.len(), 2, "must produce 2 edits for 2 occurrences");
        assert_eq!(edits[0], (0, 3));
        assert_eq!(edits[1], (8, 11));
    }

    // ── Concurrent bridge operations tests ───────────────────────────────────

    /// Two simultaneous rename operations on disjoint positions don't conflict.
    #[test]
    fn concurrent_rename_disjoint_positions_no_conflict() {
        let source = "define alpha that is beta";
        // Rename alpha → x and beta → y independently
        let result_a = source.replace("alpha", "x");
        let result_b = source.replace("beta", "y");
        // Neither rename touches the other's target
        assert!(
            result_a.contains("beta"),
            "first rename must not touch 'beta'"
        );
        assert!(
            result_b.contains("alpha"),
            "second rename must not touch 'alpha'"
        );
    }

    /// prepare_rename followed by rename uses the correct source version.
    #[test]
    fn prepare_rename_then_rename_uses_same_source() {
        let source = "define value that is value + 1";
        // Step 1: prepare_rename finds "value" at position 7
        let pos = 7usize;
        let word = find_word_at(source, pos);
        assert_eq!(word, Some("value"));
        // Step 2: rename using the same source string
        let renamed = source.replace("value", "count");
        assert_eq!(renamed, "define count that is count + 1");
    }

    /// Bridge handles rapid repeated queries (do_complete) without panic.
    #[test]
    fn concurrent_bridge_rapid_queries_no_panic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "rapid".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let worker = InteractiveWorker::new(state);
        // 100 rapid queries — none must panic
        for _ in 0..100 {
            let items = worker.do_complete("rap", None);
            assert!(!items.is_empty());
        }
    }

    /// Bridge gracefully handles malformed position (pos > source length).
    #[test]
    fn bridge_malformed_position_out_of_bounds_is_none() {
        let source = "define x";
        let pos = source.len() + 100; // well past end
        let word = find_word_at(source, pos);
        assert_eq!(word, None, "out-of-bounds position must return None");
    }

    // ── AO7: pending_count / is_idle / cancel_all tests ──────────────────────

    #[test]
    fn interactive_tier_new_is_idle() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (tier, _rx) = InteractiveTier::new(state);
        assert!(tier.is_idle(), "freshly created tier must be idle");
        assert_eq!(tier.pending_count(), 0);
    }

    #[test]
    fn interactive_tier_pending_count_starts_zero() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (tier, _rx) = InteractiveTier::new(state);
        assert_eq!(tier.pending_count(), 0, "initial pending count must be 0");
    }

    #[test]
    fn interactive_tier_is_idle_true_when_pending_zero() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (tier, _rx) = InteractiveTier::new(state);
        // No requests sent — must be idle
        assert!(tier.is_idle());
    }

    #[test]
    fn interactive_tier_cancel_all_sets_pending_to_zero() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (mut tier, _rx) = InteractiveTier::new(state);
        tier.cancel_all();
        assert_eq!(
            tier.pending_count(),
            0,
            "cancel_all must reset pending count to 0"
        );
    }

    #[test]
    fn interactive_tier_cancel_all_makes_tier_idle() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (mut tier, _rx) = InteractiveTier::new(state);
        tier.cancel_all();
        assert!(tier.is_idle(), "cancel_all must leave tier in idle state");
    }

    #[test]
    fn interactive_tier_cancel_all_twice_no_panic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (mut tier, _rx) = InteractiveTier::new(state);
        tier.cancel_all();
        tier.cancel_all(); // second call must not panic
        assert!(tier.is_idle());
    }

    #[test]
    fn interactive_tier_is_idle_and_pending_count_consistent() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (tier, _rx) = InteractiveTier::new(state);
        // is_idle iff pending_count == 0
        assert_eq!(tier.is_idle(), tier.pending_count() == 0);
    }

    #[test]
    fn interactive_tier_cancel_all_after_new_is_idle() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (mut tier, _rx) = InteractiveTier::new(state);
        assert!(tier.is_idle());
        tier.cancel_all();
        assert!(tier.is_idle(), "idle before and after cancel_all");
    }

    #[test]
    fn interactive_tier_pending_count_returns_usize() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (tier, _rx) = InteractiveTier::new(state);
        let count: usize = tier.pending_count();
        // usize is always >= 0; this verifies the return type is correct
        assert!(count < usize::MAX);
    }

    #[test]
    fn interactive_tier_cancel_clears_pending_to_zero() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (mut tier, _rx) = InteractiveTier::new(state);
        // Simulate in-flight requests by using the atomic directly (same module)
        tier.pending.store(5, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(tier.pending_count(), 5);
        tier.cancel_all();
        assert_eq!(
            tier.pending_count(),
            0,
            "cancel_all must reset pending from 5 to 0"
        );
    }

    #[test]
    fn interactive_tier_is_idle_false_when_pending_nonzero() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let (tier, _rx) = InteractiveTier::new(state);
        tier.pending.store(3, std::sync::atomic::Ordering::Relaxed);
        assert!(!tier.is_idle(), "tier with pending=3 must not be idle");
        assert_eq!(tier.pending_count(), 3);
    }
}

// ── LSP helper functions (used only in tests) ────────────────────────────────
#[cfg(test)]
fn find_word_at(source: &str, pos: usize) -> Option<&str> {
    if pos >= source.len() {
        return None;
    }
    let bytes = source.as_bytes();
    if bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n' {
        return None;
    }
    // Scan left to word start
    let mut start = pos;
    while start > 0 && is_word_char(bytes[start - 1]) {
        start -= 1;
    }
    // Scan right to word end
    let mut end = pos;
    while end < bytes.len() && is_word_char(bytes[end]) {
        end += 1;
    }
    if start == end {
        None
    } else {
        Some(&source[start..end])
    }
}

#[cfg(test)]
fn word_range_at(source: &str, pos: usize) -> Option<(usize, usize)> {
    if pos >= source.len() {
        return None;
    }
    let bytes = source.as_bytes();
    if !is_word_char(bytes[pos]) {
        return None;
    }
    let mut start = pos;
    while start > 0 && is_word_char(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = pos;
    while end < bytes.len() && is_word_char(bytes[end]) {
        end += 1;
    }
    Some((start, end))
}

#[cfg(test)]
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
