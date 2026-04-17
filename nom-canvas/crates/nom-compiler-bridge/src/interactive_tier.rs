#![deny(unsafe_code)]
use std::sync::Arc;
use crate::shared::SharedState;
#[allow(unused_imports)]
use nom_blocks::block_model::NomtuRef;
use nom_editor::lsp_bridge::{CompletionItem, CompletionKind, HoverResult};
use nom_editor::highlight::{HighlightSpan, TokenRole};

#[derive(Debug)]
pub enum InteractiveRequest {
    Tokenize { source: String, reply: tokio::sync::oneshot::Sender<Vec<TokenSpan>> },
    HighlightSpans { source: String, reply: tokio::sync::oneshot::Sender<Vec<HighlightSpan>> },
    CompletePrefix { prefix: String, kind_filter: Option<String>, reply: tokio::sync::oneshot::Sender<Vec<CompletionItem>> },
    Hover { word: String, reply: tokio::sync::oneshot::Sender<Option<HoverResult>> },
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
        let _ = self.sender.send(InteractiveRequest::Tokenize { source, reply: tx }).await;
        rx.await.unwrap_or_default()
    }

    /// Request highlight spans asynchronously
    pub async fn highlight_spans(&self, source: String) -> Vec<HighlightSpan> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.sender.send(InteractiveRequest::HighlightSpans { source, reply: tx }).await;
        rx.await.unwrap_or_default()
    }

    /// Request completions asynchronously
    pub async fn complete_prefix(&self, prefix: String, kind_filter: Option<String>) -> Vec<CompletionItem> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.sender.send(InteractiveRequest::CompletePrefix { prefix, kind_filter, reply: tx }).await;
        rx.await.unwrap_or_default()
    }

    /// Request hover info asynchronously
    pub async fn hover(&self, word: String) -> Option<HoverResult> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.sender.send(InteractiveRequest::Hover { word, reply: tx }).await;
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
            InteractiveRequest::CompletePrefix { prefix, kind_filter, reply } => {
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
        #[cfg(feature = "compiler")]
        {
            crate::adapters::completion::complete_from_dict(prefix, _kind_filter, &self.state)
        }
        #[cfg(not(feature = "compiler"))]
        {
            self.state.cached_grammar_kinds()
                .into_iter()
                .filter(|k| k.name.starts_with(prefix))
                .map(|k| CompletionItem {
                    label: k.name.clone(),
                    kind: CompletionKind::Keyword,
                    detail: Some(k.description),
                    insert_text: k.name,
                    sort_text: None,
                })
                .take(20)
                .collect()
        }
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
        self.shared.cached_grammar_kinds()
            .into_iter()
            .find(|k| k.name == token)
            .map(|k| k.name)
    }

    /// Hover info for a word — returns a nomtu-prefixed description
    pub fn hover_info(&self, word: &str) -> Option<String> {
        Some(format!("nomtu: {word}"))
    }
}

fn simple_tokenize_stub(source: &str) -> Vec<TokenSpan> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for word in source.split_whitespace() {
        let start = source[offset..].find(word).map(|i| offset + i).unwrap_or(offset);
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
}
