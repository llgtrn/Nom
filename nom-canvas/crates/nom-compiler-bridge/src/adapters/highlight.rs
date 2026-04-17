#![deny(unsafe_code)]
use nom_editor::highlight::HighlightSpan;
#[cfg(feature = "compiler")]
use nom_editor::highlight::TokenRole;
use crate::shared::SharedState;
use crate::interactive_tier::TokenSpan;

// With compiler feature: real tokenizer from nom-concept
#[cfg(feature = "compiler")]
pub fn tokenize_to_spans(source: &str, state: &SharedState) -> Vec<TokenSpan> {
    use nom_concept::stage1_tokenize;
    let Ok(stream) = stage1_tokenize(source) else { return vec![]; };
    stream.toks.iter().map(|spanned| {
        let role = tok_to_role(&spanned.tok, state);
        TokenSpan {
            start: spanned.pos,
            end: spanned.pos,
            role,
            text: String::new(),
        }
    }).collect()
}

/// First wire: stage1_tokenize → TokenRole → HighlightSpan
/// This is the keystone that proves nom-canvas understands Nom syntax in real-time.
#[cfg(feature = "compiler")]
pub fn highlight_source(source: &str, state: &SharedState) -> Vec<HighlightSpan> {
    use nom_concept::stage1_tokenize;
    let Ok(stream) = stage1_tokenize(source) else { return vec![]; };
    stream.toks.iter().map(|spanned| {
        let role = tok_to_role(&spanned.tok, state);
        HighlightSpan::new(spanned.pos..spanned.pos, role)
    }).collect()
}

#[cfg(feature = "compiler")]
fn tok_to_role(tok: &nom_concept::Tok, state: &SharedState) -> TokenRole {
    use nom_concept::Tok;
    match tok {
        Tok::The | Tok::Is | Tok::Composes | Tok::Then | Tok::With |
        Tok::Requires | Tok::Ensures | Tok::Matching | Tok::Benefit |
        Tok::Hazard | Tok::At | Tok::Dot | Tok::Comma |
        Tok::Intended | Tok::To | Tok::Uses | Tok::Extends |
        Tok::Adding | Tok::Removing | Tok::Exposes | Tok::This |
        Tok::Works | Tok::When | Tok::Favor | Tok::AtLeast | Tok::AtMost |
        Tok::Retry | Tok::Format | Tok::Accesses | Tok::Shaped | Tok::Like |
        Tok::Field | Tok::Tagged | Tok::Watermark | Tok::Lag | Tok::Seconds |
        Tok::Window | Tok::Clock | Tok::Domain | Tok::Mhz | Tok::Quality
            => TokenRole::Keyword,
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
}
