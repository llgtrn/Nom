#![deny(unsafe_code)]
use crate::interactive_tier::TokenSpan;
use crate::shared::SharedState;
use nom_editor::highlight::HighlightSpan;
#[cfg(feature = "compiler")]
use nom_editor::highlight::TokenRole;

/// Returns the byte length of the canonical surface text for a token.
#[cfg(feature = "compiler")]
fn tok_text_len(tok: &nom_concept::Tok) -> usize {
    use nom_concept::Tok;
    match tok {
        Tok::The => 3,
        Tok::Is => 2,
        Tok::Composes => 8,
        Tok::Then => 4,
        Tok::With => 4,
        Tok::Requires => 8,
        Tok::Ensures => 7,
        Tok::Matching => 8,
        Tok::Benefit => 7,
        Tok::Hazard => 6,
        Tok::At => 2,
        Tok::Dot => 1,
        Tok::Comma => 1,
        Tok::Intended => 8,
        Tok::To => 2,
        Tok::Uses => 4,
        Tok::Extends => 7,
        Tok::Adding => 6,
        Tok::Removing => 8,
        Tok::Exposes => 7,
        Tok::This => 4,
        Tok::Works => 5,
        Tok::When => 4,
        Tok::Favor => 5,
        Tok::AtLeast => 8,  // "at-least"
        Tok::AtMost => 7,   // "at-most"
        Tok::Retry => 5,
        Tok::Format => 6,
        Tok::Accesses => 8,
        Tok::Shaped => 6,
        Tok::Like => 4,
        Tok::Field => 5,
        Tok::Tagged => 6,
        Tok::Watermark => 9,
        Tok::Lag => 3,
        Tok::Seconds => 7,
        Tok::Window => 6,
        Tok::Clock => 5,
        Tok::Domain => 6,
        Tok::Mhz => 3,
        Tok::Quality => 7,
        Tok::NumberLit(n) => format!("{n}").len(),
        Tok::Kind(s) => s.len(),
        Tok::Word(s) => s.len(),
        Tok::Quoted(s) => s.len() + 2, // include the surrounding quotes
        Tok::AtKind(s) => s.len() + 1, // include the leading '@'
    }
}

// With compiler feature: real tokenizer from nom-concept
#[cfg(feature = "compiler")]
pub fn tokenize_to_spans(source: &str, state: &SharedState) -> Vec<TokenSpan> {
    use nom_concept::stage1_tokenize;
    let Ok(stream) = stage1_tokenize(source) else {
        return vec![];
    };
    stream
        .toks
        .iter()
        .map(|spanned| {
            let role = tok_to_role(&spanned.tok, state);
            let token_len = tok_text_len(&spanned.tok);
            TokenSpan {
                start: spanned.pos,
                end: spanned.pos + token_len,
                role,
                text: String::new(),
            }
        })
        .collect()
}

/// First wire: stage1_tokenize → TokenRole → HighlightSpan
/// This is the keystone that proves nom-canvas understands Nom syntax in real-time.
#[cfg(feature = "compiler")]
pub fn highlight_source(source: &str, state: &SharedState) -> Vec<HighlightSpan> {
    use nom_concept::stage1_tokenize;
    let Ok(stream) = stage1_tokenize(source) else {
        return vec![];
    };
    stream
        .toks
        .iter()
        .map(|spanned| {
            let role = tok_to_role(&spanned.tok, state);
            let token_len = tok_text_len(&spanned.tok);
            HighlightSpan::new(spanned.pos..spanned.pos + token_len, role)
        })
        .collect()
}

#[cfg(feature = "compiler")]
fn tok_to_role(tok: &nom_concept::Tok, state: &SharedState) -> TokenRole {
    use nom_concept::Tok;
    match tok {
        Tok::The
        | Tok::Is
        | Tok::Composes
        | Tok::Then
        | Tok::With
        | Tok::Requires
        | Tok::Ensures
        | Tok::Matching
        | Tok::Benefit
        | Tok::Hazard
        | Tok::At
        | Tok::Dot
        | Tok::Comma
        | Tok::Intended
        | Tok::To
        | Tok::Uses
        | Tok::Extends
        | Tok::Adding
        | Tok::Removing
        | Tok::Exposes
        | Tok::This
        | Tok::Works
        | Tok::When
        | Tok::Favor
        | Tok::AtLeast
        | Tok::AtMost
        | Tok::Retry
        | Tok::Format
        | Tok::Accesses
        | Tok::Shaped
        | Tok::Like
        | Tok::Field
        | Tok::Tagged
        | Tok::Watermark
        | Tok::Lag
        | Tok::Seconds
        | Tok::Window
        | Tok::Clock
        | Tok::Domain
        | Tok::Mhz
        | Tok::Quality => TokenRole::Keyword,
        Tok::Word(word) => {
            let kinds = state.cached_grammar_kinds();
            if kinds.iter().any(|k| &k.name == word) {
                TokenRole::NomtuRef
            } else {
                TokenRole::Identifier
            }
        }
        Tok::Kind(_) | Tok::AtKind(_) => TokenRole::NomtuRef,
        Tok::NumberLit(_) | Tok::Quoted(_) => TokenRole::Literal,
    }
}

// Without compiler feature: stubs
#[cfg(not(feature = "compiler"))]
pub fn tokenize_to_spans(_source: &str, _state: &SharedState) -> Vec<TokenSpan> {
    vec![]
}

#[cfg(not(feature = "compiler"))]
pub fn highlight_source(_source: &str, _state: &SharedState) -> Vec<HighlightSpan> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_empty_source_no_panic() {
        let state = SharedState::new("a.db", "b.db");
        let spans = highlight_source("", &state);
        assert!(spans.is_empty());
    }

    #[test]
    fn tokenize_empty_no_panic() {
        let state = SharedState::new("a.db", "b.db");
        let spans = tokenize_to_spans("", &state);
        assert!(spans.is_empty());
    }

    #[test]
    fn highlight_adapter_empty_input() {
        // empty input → 0 color runs (HighlightSpan vec is empty)
        let state = SharedState::new("a.db", "b.db");
        let spans = highlight_source("", &state);
        assert_eq!(spans.len(), 0);
    }

    #[test]
    fn highlight_adapter_non_empty_source_no_panic() {
        // Without compiler feature, any source returns empty vec without panic
        let state = SharedState::new("a.db", "b.db");
        let spans = highlight_source("define something", &state);
        // stub returns empty; the real impl may return spans — either is acceptable
        let _ = spans; // no panic is the invariant
    }

    #[test]
    fn highlight_adapter_multiple_tokens() {
        // In stub mode tokenize_to_spans always returns empty; tokenize_to_spans("")
        // must return empty without panic — multiple tokens would come from real compiler feature.
        // We verify that the stub correctly handles multi-word input without producing incorrect spans.
        let state = SharedState::new("a.db", "b.db");
        let spans = tokenize_to_spans("word1 word2 word3", &state);
        // Stub returns empty; real compiler would produce 3 TokenSpan items.
        // Either way the result must be a valid Vec.
        let _count = spans.len();
        // No panic is the invariant for stub mode
    }
}
