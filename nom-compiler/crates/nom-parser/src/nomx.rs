//! `.nomx` parser prototype (proposal 05).
//!
//! First real parser code for the natural-language grammar track.
//! Consumes `NomxToken`s from [`nom_lexer::nomx`] and produces a
//! tiny AST of `NomxDecl`s. Today recognizes only the declaration
//! head:
//!
//!   define <name> that takes <param> and returns <ret>:
//!
//! Body parsing + control flow + contracts + record/choice arrive
//! incrementally as the grammar stabilizes. Not yet wired into
//! `parse_source` — callers invoke `parse_nomx` explicitly.

use nom_lexer::nomx::{NomxSpan, NomxToken, SpannedNomxToken, tokenize_nomx_with_spans};

/// A parsed `.nomx` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NomxDecl {
    /// `define <name> that takes <param> and returns <ret>:`
    /// Body parsing deferred; head only for now.
    Define {
        name: String,
        param: Option<String>,
        returns: Option<String>,
        span: NomxSpan,
    },
}

/// Parse error for the `.nomx` parser. Carries the span of the
/// offending token for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NomxParseError {
    pub message: String,
    pub span: NomxSpan,
}

pub type NomxParseResult<T> = Result<T, NomxParseError>;

/// Parse `.nomx` source into a Vec of top-level declarations.
/// Skips anything that isn't a recognized declaration head today;
/// real recovery lands when the grammar covers more than Define.
pub fn parse_nomx(source: &str) -> NomxParseResult<Vec<NomxDecl>> {
    let tokens = tokenize_nomx_with_spans(source);
    let mut parser = NomxParser::new(&tokens);
    parser.parse_file()
}

struct NomxParser<'a> {
    tokens: &'a [SpannedNomxToken],
    pos: usize,
}

impl<'a> NomxParser<'a> {
    fn new(tokens: &'a [SpannedNomxToken]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &NomxToken {
        self.tokens
            .get(self.pos)
            .map(|s| &s.token)
            .unwrap_or(&NomxToken::Eof)
    }

    fn peek_span(&self) -> NomxSpan {
        self.tokens
            .get(self.pos)
            .map(|s| s.span)
            .unwrap_or(NomxSpan::new(0, 0))
    }

    fn advance(&mut self) -> &SpannedNomxToken {
        let tok = &self.tokens[self.pos];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &NomxToken, label: &str) -> NomxParseResult<()> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(NomxParseError {
                message: format!("expected {label}, found {:?}", self.peek()),
                span: self.peek_span(),
            })
        }
    }

    fn consume_identifier(&mut self, role: &str) -> NomxParseResult<String> {
        match self.peek().clone() {
            NomxToken::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(NomxParseError {
                message: format!("expected {role} (identifier), found {other:?}"),
                span: self.peek_span(),
            }),
        }
    }

    fn parse_file(&mut self) -> NomxParseResult<Vec<NomxDecl>> {
        let mut decls = Vec::new();
        while self.peek() != &NomxToken::Eof {
            match self.peek() {
                NomxToken::Define => decls.push(self.parse_define()?),
                _ => {
                    // Skip unrecognized tokens. Real recovery comes later.
                    self.advance();
                }
            }
        }
        Ok(decls)
    }

    /// `define <name> that takes <param> and returns <ret>:`
    fn parse_define(&mut self) -> NomxParseResult<NomxDecl> {
        let start = self.peek_span().start;
        self.expect(&NomxToken::Define, "`define`")?;
        let name = self.consume_identifier("name after `define`")?;

        // Optional `that takes <param> and returns <ret>` tail.
        let mut param = None;
        let mut returns = None;

        if self.peek() == &NomxToken::That {
            self.advance();
            if self.peek() == &NomxToken::Takes {
                self.advance();
                param = Some(self.consume_identifier("parameter after `takes`")?);
                if self.peek() == &NomxToken::And {
                    self.advance();
                }
            }
            if self.peek() == &NomxToken::Returns {
                self.advance();
                returns = Some(self.consume_identifier("return name after `returns`")?);
            }
        }

        self.expect(&NomxToken::Colon, "`:` ending the declaration head")?;

        let end = self.peek_span().start.max(start);
        Ok(NomxDecl::Define {
            name,
            param,
            returns,
            span: NomxSpan::new(start, end),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_define_sentence() {
        let src = "define greet that takes a name and returns a greeting:";
        let decls = parse_nomx(src).unwrap();
        assert_eq!(decls.len(), 1);
        let NomxDecl::Define {
            name,
            param,
            returns,
            ..
        } = &decls[0];
        assert_eq!(name, "greet");
        assert_eq!(param.as_deref(), Some("name"));
        assert_eq!(returns.as_deref(), Some("greeting"));
    }

    #[test]
    fn parses_bare_define() {
        // `define x:` — no takes/returns clause.
        let decls = parse_nomx("define noop:").unwrap();
        assert_eq!(decls.len(), 1);
        let NomxDecl::Define {
            name,
            param,
            returns,
            ..
        } = &decls[0];
        assert_eq!(name, "noop");
        assert_eq!(*param, None);
        assert_eq!(*returns, None);
    }

    #[test]
    fn missing_name_errors() {
        let err = parse_nomx("define :").unwrap_err();
        assert!(err.message.contains("name after `define`"));
    }

    #[test]
    fn missing_colon_errors_with_span() {
        let src = "define foo";
        let err = parse_nomx(src).unwrap_err();
        assert!(
            err.message.contains("expected `:`"),
            "expected colon-diag, got: {}",
            err.message
        );
    }

    #[test]
    fn parses_multiple_definitions() {
        // Names must not collide with the article-stripping rule
        // (a/an/the/which/who/whose are consumed at lex time per
        // proposal 05 §4.8). Using `foo` and `bar` keeps them as
        // identifiers.
        let src = "define foo: define bar:";
        let decls = parse_nomx(src).unwrap();
        assert_eq!(decls.len(), 2);
    }

    #[test]
    fn parses_hello_nomx_sample() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/hello.nomx");
        let src = std::fs::read_to_string(&path).unwrap();
        let decls = parse_nomx(&src).unwrap();
        assert_eq!(decls.len(), 1);
        let NomxDecl::Define { name, param, returns, .. } = &decls[0];
        assert_eq!(name, "greet");
        assert_eq!(param.as_deref(), Some("name"));
        assert_eq!(returns.as_deref(), Some("greeting"));
    }
}
