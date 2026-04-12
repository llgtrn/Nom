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
    /// followed by zero or more body statements, terminated at the
    /// next `define` / `record` / `choice` keyword or EOF.
    Define {
        name: String,
        param: Option<String>,
        returns: Option<String>,
        body: Vec<NomxStatement>,
        span: NomxSpan,
    },
    /// `record <name> holds:` followed by one-field-per-statement
    /// body. Fields captured by raw tokens today; real field typing
    /// lands with the type system.
    Record {
        name: String,
        fields: Vec<NomxRecordField>,
        span: NomxSpan,
    },
    /// `choice <name> is one of:` followed by one-variant-per-statement
    /// body. Variant payload captured by raw tokens.
    Choice {
        name: String,
        variants: Vec<NomxChoiceVariant>,
        span: NomxSpan,
    },
}

/// One field in a `record` declaration. Raw tokens until the field
/// terminator (`.` or the next `record`/`choice`/`define`/EOF).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NomxRecordField {
    pub name: String,
    pub type_tokens: Vec<NomxToken>,
    pub span: NomxSpan,
}

/// One variant in a `choice` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NomxChoiceVariant {
    pub name: String,
    pub payload_tokens: Vec<NomxToken>,
    pub span: NomxSpan,
}

/// One statement inside a declaration body. Scaffold-level AST —
/// richer expression tree lands once the type system wires in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NomxStatement {
    /// `<subject> is <rhs...>` — a binding. `subject` is a single
    /// identifier; `rhs_tokens` captures the raw token sequence
    /// until `.` or the next declaration. Lets us lock the parse
    /// shape now; expression parsing arrives with the type system.
    Binding {
        subject: String,
        rhs_tokens: Vec<NomxToken>,
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
                NomxToken::Record => decls.push(self.parse_record()?),
                NomxToken::Choice => decls.push(self.parse_choice()?),
                _ => {
                    self.advance();
                }
            }
        }
        Ok(decls)
    }

    /// `record <name> holds: <fields>`
    /// Each field: `<name> is <type-tokens>` terminated by `.` or
    /// the next top-level keyword / EOF.
    fn parse_record(&mut self) -> NomxParseResult<NomxDecl> {
        let start = self.peek_span().start;
        self.expect(&NomxToken::Record, "`record`")?;
        let name = self.consume_identifier("name after `record`")?;
        if self.peek() == &NomxToken::Holds {
            self.advance();
        }
        self.expect(&NomxToken::Colon, "`:` ending the record head")?;

        let mut fields = Vec::new();
        while !self.peek_is_body_terminator() {
            let field_start = self.peek_span();
            let fname = self.consume_identifier("field name")?;
            self.expect(&NomxToken::Is, "`is` after field name")?;
            let mut type_tokens = Vec::new();
            while !self.peek_is_body_terminator() && self.peek() != &NomxToken::Period {
                type_tokens.push(self.advance().token.clone());
            }
            if self.peek() == &NomxToken::Period {
                self.advance();
            }
            fields.push(NomxRecordField {
                name: fname,
                type_tokens,
                span: NomxSpan::new(field_start.start, self.peek_span().start),
            });
        }

        let end = self.peek_span().start.max(start);
        Ok(NomxDecl::Record {
            name,
            fields,
            span: NomxSpan::new(start, end),
        })
    }

    /// `choice <name> is one of: <variants>`
    fn parse_choice(&mut self) -> NomxParseResult<NomxDecl> {
        let start = self.peek_span().start;
        self.expect(&NomxToken::Choice, "`choice`")?;
        let name = self.consume_identifier("name after `choice`")?;
        // Optional `is one of` prefix; grammar accepts shorter forms too.
        if self.peek() == &NomxToken::Is {
            self.advance();
            // skip `one of` identifiers if present
            while matches!(self.peek(), NomxToken::Identifier(_) | NomxToken::Of) {
                self.advance();
            }
        }
        self.expect(&NomxToken::Colon, "`:` ending the choice head")?;

        let mut variants = Vec::new();
        while !self.peek_is_body_terminator() {
            let var_start = self.peek_span();
            let vname = self.consume_identifier("variant name")?;
            let mut payload_tokens = Vec::new();
            while !self.peek_is_body_terminator() && self.peek() != &NomxToken::Period {
                payload_tokens.push(self.advance().token.clone());
            }
            if self.peek() == &NomxToken::Period {
                self.advance();
            }
            variants.push(NomxChoiceVariant {
                name: vname,
                payload_tokens,
                span: NomxSpan::new(var_start.start, self.peek_span().start),
            });
        }

        let end = self.peek_span().start.max(start);
        Ok(NomxDecl::Choice {
            name,
            variants,
            span: NomxSpan::new(start, end),
        })
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

        let body = self.parse_body()?;

        let end = self.peek_span().start.max(start);
        Ok(NomxDecl::Define {
            name,
            param,
            returns,
            body,
            span: NomxSpan::new(start, end),
        })
    }

    /// Body of a declaration: zero or more statements until the next
    /// top-level declaration keyword or EOF.
    fn parse_body(&mut self) -> NomxParseResult<Vec<NomxStatement>> {
        let mut out = Vec::new();
        while !self.peek_is_body_terminator() {
            let stmt = self.parse_statement()?;
            out.push(stmt);
        }
        Ok(out)
    }

    fn peek_is_body_terminator(&self) -> bool {
        matches!(
            self.peek(),
            NomxToken::Eof
                | NomxToken::Define
                | NomxToken::Record
                | NomxToken::Choice
        )
    }

    fn parse_statement(&mut self) -> NomxParseResult<NomxStatement> {
        let start_span = self.peek_span();
        // Scaffold: accept only `<subject> is <rhs...>`. Other forms
        // (when/otherwise branches, return expressions, etc.) arrive
        // with the type system. Unknown-shape = skip the token and
        // continue; lets the parse gate accept real-world bodies even
        // before every form is recognized.
        let subject = match self.peek().clone() {
            NomxToken::Identifier(n) => {
                self.advance();
                n
            }
            _ => {
                return Err(NomxParseError {
                    message: format!(
                        "expected binding subject (identifier), found {:?}",
                        self.peek()
                    ),
                    span: self.peek_span(),
                });
            }
        };
        self.expect(&NomxToken::Is, "`is` after binding subject")?;

        let mut rhs_tokens: Vec<NomxToken> = Vec::new();
        while !self.peek_is_body_terminator() && self.peek() != &NomxToken::Period {
            rhs_tokens.push(self.advance().token.clone());
        }
        if self.peek() == &NomxToken::Period {
            self.advance();
        }

        Ok(NomxStatement::Binding {
            subject,
            rhs_tokens,
            span: NomxSpan::new(start_span.start, self.peek_span().start),
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
        } = &decls[0]
        else {
            panic!("expected Define");
        };
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
        } = &decls[0]
        else {
            panic!("expected Define");
        };
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
    fn parses_binding_body() {
        let src = "define greet:\n  greeting is \"hi\"";
        let decls = parse_nomx(src).unwrap();
        let NomxDecl::Define { body, .. } = &decls[0] else {
            panic!("expected Define");
        };
        assert_eq!(body.len(), 1);
        let NomxStatement::Binding { subject, rhs_tokens, .. } = &body[0];
        assert_eq!(subject, "greeting");
        assert!(
            rhs_tokens
                .iter()
                .any(|t| matches!(t, NomxToken::StringLit(s) if s == "hi")),
            "expected StringLit in rhs, got: {rhs_tokens:?}"
        );
    }

    #[test]
    fn parses_multiple_bindings_separated_by_period() {
        let src = "define foo:\n  x is \"a\".\n  y is \"b\".";
        let decls = parse_nomx(src).unwrap();
        let NomxDecl::Define { body, .. } = &decls[0] else {
            panic!("expected Define");
        };
        assert_eq!(body.len(), 2);
    }

    #[test]
    fn body_ends_at_next_declaration() {
        let src = "define a_fn:\n  x is \"left\"\ndefine b_fn:\n  y is \"right\"";
        let decls = parse_nomx(src).unwrap();
        assert_eq!(decls.len(), 2);
        for d in &decls {
            let NomxDecl::Define { body, .. } = d else {
                panic!("expected Define");
            };
            assert_eq!(body.len(), 1);
        }
    }

    #[test]
    fn parses_record_with_fields() {
        let src = "record user holds:\n  name is text.\n  age is number.";
        let decls = parse_nomx(src).unwrap();
        assert_eq!(decls.len(), 1);
        let NomxDecl::Record { name, fields, .. } = &decls[0] else {
            panic!("expected Record, got {:?}", decls[0]);
        };
        assert_eq!(name, "user");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "name");
        assert_eq!(fields[1].name, "age");
    }

    #[test]
    fn parses_choice_with_variants() {
        // `choice` followed by optional `is one of:`; `is` and `of`
        // are tokens, `one` is an identifier. All skipped per parser.
        let src = "choice status is one of:\n  active.\n  suspended.\n  deleted.";
        let decls = parse_nomx(src).unwrap();
        let NomxDecl::Choice { name, variants, .. } = &decls[0] else {
            panic!("expected Choice, got {:?}", decls[0]);
        };
        assert_eq!(name, "status");
        assert_eq!(variants.len(), 3);
        assert_eq!(variants[0].name, "active");
        assert_eq!(variants[1].name, "suspended");
        assert_eq!(variants[2].name, "deleted");
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
        let NomxDecl::Define { name, param, returns, .. } = &decls[0] else {
            panic!("expected Define");
        };
        assert_eq!(name, "greet");
        assert_eq!(param.as_deref(), Some("name"));
        assert_eq!(returns.as_deref(), Some("greeting"));
    }
}
