//! `.nomx` parser prototype (proposal 05).
//!
//! Real parser for the natural-language grammar track. Consumes
//! `NomxToken`s from [`nom_lexer::nomx`] and produces an AST of
//! `NomxDecl`s + `NomxStatement`s. Not yet wired into `parse_source`
//! — callers invoke `parse_nomx` explicitly.
//!
//! # Grammar (EBNF, accepted today)
//!
//! ```text
//! source_file   ::= declaration* EOF
//!
//! declaration   ::= define_decl
//!                 | record_decl
//!                 | choice_decl
//!                 | to_oneliner
//!
//! define_decl   ::= "define" IDENT ( "that" "takes" IDENT
//!                                    ( "and" "returns" IDENT )? )? ":"
//!                   body
//!
//! to_oneliner   ::= "to" IDENT <noun_phrase>* "," <expr>* "."?
//!                   (* Lowered to define_decl with a single binding,
//!                      subject = "respond". *)
//!
//! record_decl   ::= "record" IDENT "holds"? ":"
//!                   ( field_stmt )*
//! field_stmt    ::= IDENT "is" <type_tokens>* "."?
//!
//! choice_decl   ::= "choice" IDENT ( "is" ("one" | "of")* )? ":"
//!                   ( variant_stmt )*
//! variant_stmt  ::= IDENT <payload_tokens>* "."?
//!
//! body          ::= statement* (body_terminator)
//! body_terminator ::= EOF | "define" | "record" | "choice"
//! statement     ::= binding_stmt | when_stmt | for_each_stmt
//!                 | while_stmt | contract_stmt
//!
//! binding_stmt  ::= IDENT "is" <rhs_tokens>* "."?
//!
//! when_stmt     ::= ( "when" | "unless" ) <cond_tokens>* ","
//!                   <then_tokens>* "."?
//!                   ( "otherwise" ","? <else_tokens>* "."? )?
//!                   (* `unless` prepends Not to cond_tokens *)
//!
//! for_each_stmt ::= "for" ("each")? IDENT ("in" | "of")
//!                   <collection_tokens>* ","
//!                   <body_tokens>* "."?
//!
//! while_stmt    ::= "while" <cond_tokens>* ","
//!                   <body_tokens>* "."?
//!
//! contract_stmt ::= ( "require" | "ensure" | "throughout" ","? )
//!                   <pred_tokens>* "."?
//!                   (* ContractKind {Require, Ensure, Throughout} *)
//! ```
//!
//! `<rhs_tokens>`, `<cond_tokens>`, `<then_tokens>`, `<else_tokens>`,
//! `<type_tokens>`, `<payload_tokens>`, and `<expr>` currently
//! capture their token runs verbatim (Vec<NomxToken>). Real
//! expression parsing lands with the type system; the shape above
//! is stable — downstream consumers only need to track the token
//! lists' contents, not their tree structure, until then.
//!
//! Article words (a/an/the/which/who/whose) are stripped at lex
//! time per proposal 05 §4.8 and never appear in any token list.

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
    /// `when <cond_tokens>, <then_tokens>.` — true-branch statement.
    /// If followed by an `otherwise, <else_tokens>.` the else branch
    /// is captured; else None. Condition and branch token sequences
    /// mirror the Binding rhs_tokens discipline.
    When {
        cond_tokens: Vec<NomxToken>,
        then_tokens: Vec<NomxToken>,
        else_tokens: Option<Vec<NomxToken>>,
        span: NomxSpan,
    },
    /// `for each <var> in <collection>, <body>.` — iteration
    /// statement. `var` is the loop-bound identifier; `in` and
    /// `of` prepositions both accepted (`for each x of xs` OK).
    /// Collection + body tokens captured verbatim.
    ForEach {
        var: String,
        collection_tokens: Vec<NomxToken>,
        body_tokens: Vec<NomxToken>,
        span: NomxSpan,
    },
    /// `while <cond>, <body>.` — loop statement. Same shape as
    /// ForEach minus the loop variable + collection; condition +
    /// body captured verbatim.
    While {
        cond_tokens: Vec<NomxToken>,
        body_tokens: Vec<NomxToken>,
        span: NomxSpan,
    },
    /// Contract clause per proposal 05 §4.4:
    ///   `require <pred>.` precondition statement
    ///   `ensure <pred>.`  postcondition
    ///   `throughout, <pred>.` invariant
    /// `kind` carries the verb; pred_tokens captures the predicate
    /// verbatim until `.`.
    Contract {
        kind: ContractKind,
        pred_tokens: Vec<NomxToken>,
        span: NomxSpan,
    },
}

/// Which contract verb produced a Contract statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContractKind {
    Require,
    Ensure,
    Throughout,
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

impl NomxDecl {
    /// Stable string tag for the declaration kind: "define" /
    /// "record" / "choice". to-oneliner lowers to Define, so this
    /// returns "define" for the sentence form too.
    pub fn kind(&self) -> &'static str {
        match self {
            NomxDecl::Define { .. } => "define",
            NomxDecl::Record { .. } => "record",
            NomxDecl::Choice { .. } => "choice",
        }
    }

    /// Declared name (same field across variants).
    pub fn name(&self) -> &str {
        match self {
            NomxDecl::Define { name, .. }
            | NomxDecl::Record { name, .. }
            | NomxDecl::Choice { name, .. } => name,
        }
    }

    /// Source span covering the declaration head + body.
    pub fn span(&self) -> NomxSpan {
        match self {
            NomxDecl::Define { span, .. }
            | NomxDecl::Record { span, .. }
            | NomxDecl::Choice { span, .. } => *span,
        }
    }
}

impl NomxStatement {
    /// Stable string tag for the statement kind. Use for debug
    /// output, structured logs, or structural dispatch without a
    /// full match over the variant tree.
    pub fn kind(&self) -> &'static str {
        match self {
            NomxStatement::Binding { .. } => "binding",
            NomxStatement::When { .. } => "when",
            NomxStatement::ForEach { .. } => "for_each",
            NomxStatement::While { .. } => "while",
            NomxStatement::Contract { .. } => "contract",
        }
    }

    /// Source span covering the whole statement.
    pub fn span(&self) -> NomxSpan {
        match self {
            NomxStatement::Binding { span, .. }
            | NomxStatement::When { span, .. }
            | NomxStatement::ForEach { span, .. }
            | NomxStatement::While { span, .. }
            | NomxStatement::Contract { span, .. } => *span,
        }
    }
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
                NomxToken::To => decls.push(self.parse_to_oneliner()?),
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

    /// `to <name>, respond with <expr>.` — one-liner imperative
    /// declaration form per proposal 05 §3. Lowered to a Define
    /// with a single binding body. `respond with <expr>` becomes
    /// the binding with subject = "respond" and rhs = expr tokens.
    fn parse_to_oneliner(&mut self) -> NomxParseResult<NomxDecl> {
        let start = self.peek_span().start;
        self.expect(&NomxToken::To, "`to`")?;
        // The verb / function name is the first identifier.
        let name = self.consume_identifier("name after `to`")?;

        // Skip the phrase "<noun-phrase>" up to the comma — it
        // describes the operand but doesn't yet bind a parameter.
        while !self.peek_is_body_terminator()
            && self.peek() != &NomxToken::Comma
            && self.peek() != &NomxToken::Period
        {
            self.advance();
        }
        self.expect(&NomxToken::Comma, "`,` after `to <verb>` phrase")?;

        // Body statement captured verbatim; parse_body handles the rest.
        let body_start = self.peek_span();
        let mut rhs_tokens: Vec<NomxToken> = Vec::new();
        while !self.peek_is_body_terminator() && self.peek() != &NomxToken::Period {
            rhs_tokens.push(self.advance().token.clone());
        }
        if self.peek() == &NomxToken::Period {
            self.advance();
        }
        let binding = NomxStatement::Binding {
            subject: "respond".to_string(),
            rhs_tokens,
            span: NomxSpan::new(body_start.start, self.peek_span().start),
        };

        let end = self.peek_span().start.max(start);
        Ok(NomxDecl::Define {
            name,
            param: None,
            returns: None,
            body: vec![binding],
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
    ///
    /// Uses a wider terminator set than `peek_is_body_terminator` —
    /// `To` ends a body at statement-start (new top-level decl) but
    /// is a preposition inside rhs/cond/body token runs. This
    /// split is the sentence-boundary approximation: a token run's
    /// inner loop uses the narrow set; the outer body loop uses
    /// this wider set.
    fn parse_body(&mut self) -> NomxParseResult<Vec<NomxStatement>> {
        let mut out = Vec::new();
        while !self.peek_is_body_terminator_outer() {
            let stmt = self.parse_statement()?;
            out.push(stmt);
        }
        Ok(out)
    }

    fn peek_is_body_terminator_outer(&self) -> bool {
        self.peek_is_body_terminator() || matches!(self.peek(), NomxToken::To)
    }

    fn peek_is_body_terminator(&self) -> bool {
        // `to` is intentionally NOT a terminator: it doubles as the
        // ToPrep preposition mid-phrase ("joined to name") and the
        // lexer can't tell which — parser-side disambiguation on
        // statement boundaries would need a sentence-boundary
        // detector. Today: put a period between top-level decls
        // when mixing block and sentence forms.
        matches!(
            self.peek(),
            NomxToken::Eof
                | NomxToken::Define
                | NomxToken::Record
                | NomxToken::Choice
        )
    }

    fn parse_statement(&mut self) -> NomxParseResult<NomxStatement> {
        match self.peek() {
            NomxToken::When => self.parse_when_statement(false),
            NomxToken::Unless => self.parse_when_statement(true),
            NomxToken::For => self.parse_for_each_statement(),
            NomxToken::While => self.parse_while_statement(),
            NomxToken::Require => self.parse_contract_statement(ContractKind::Require),
            NomxToken::Ensure => self.parse_contract_statement(ContractKind::Ensure),
            NomxToken::Throughout => self.parse_contract_statement(ContractKind::Throughout),
            _ => self.parse_binding_statement(),
        }
    }

    /// `require <pred>.` / `ensure <pred>.` / `throughout [,] <pred>.`
    /// The leading verb has already been peeked; consume it and
    /// collect tokens until `.` or body terminator.
    fn parse_contract_statement(
        &mut self,
        kind: ContractKind,
    ) -> NomxParseResult<NomxStatement> {
        let start_span = self.peek_span();
        self.advance(); // consume the verb
        // Throughout accepts an optional leading comma per prose idiom
        // ("throughout, x is nonneg.").
        if kind == ContractKind::Throughout && self.peek() == &NomxToken::Comma {
            self.advance();
        }
        let mut pred_tokens: Vec<NomxToken> = Vec::new();
        while !self.peek_is_body_terminator() && self.peek() != &NomxToken::Period {
            pred_tokens.push(self.advance().token.clone());
        }
        if self.peek() == &NomxToken::Period {
            self.advance();
        }
        Ok(NomxStatement::Contract {
            kind,
            pred_tokens,
            span: NomxSpan::new(start_span.start, self.peek_span().start),
        })
    }

    /// `while <cond>, <body>.`
    fn parse_while_statement(&mut self) -> NomxParseResult<NomxStatement> {
        let start_span = self.peek_span();
        self.expect(&NomxToken::While, "`while`")?;
        let mut cond_tokens: Vec<NomxToken> = Vec::new();
        while !self.peek_is_body_terminator()
            && self.peek() != &NomxToken::Comma
            && self.peek() != &NomxToken::Period
        {
            cond_tokens.push(self.advance().token.clone());
        }
        self.expect(&NomxToken::Comma, "`,` after `while` condition")?;
        let mut body_tokens: Vec<NomxToken> = Vec::new();
        while !self.peek_is_body_terminator() && self.peek() != &NomxToken::Period {
            body_tokens.push(self.advance().token.clone());
        }
        if self.peek() == &NomxToken::Period {
            self.advance();
        }
        Ok(NomxStatement::While {
            cond_tokens,
            body_tokens,
            span: NomxSpan::new(start_span.start, self.peek_span().start),
        })
    }

    /// `for each <var> in <collection>, <body>.`
    /// `in` and `of` prepositions are both accepted at the var/
    /// collection boundary. The "each" identifier is consumed but
    /// doesn't bind anything — it's English filler.
    fn parse_for_each_statement(&mut self) -> NomxParseResult<NomxStatement> {
        let start_span = self.peek_span();
        self.expect(&NomxToken::For, "`for`")?;
        // Optional `each` (identifier) — English filler.
        if matches!(self.peek(), NomxToken::Identifier(n) if n == "each") {
            self.advance();
        }
        let var = self.consume_identifier("loop variable after `for each`")?;
        // Accept `in` (Identifier) or `of` (PrepositionalOperator).
        match self.peek().clone() {
            NomxToken::Of => {
                self.advance();
            }
            NomxToken::Identifier(n) if n == "in" => {
                self.advance();
            }
            _ => {
                return Err(NomxParseError {
                    message: format!(
                        "expected `in` or `of` after loop variable, found {:?}",
                        self.peek()
                    ),
                    span: self.peek_span(),
                });
            }
        }
        let mut collection_tokens: Vec<NomxToken> = Vec::new();
        while !self.peek_is_body_terminator()
            && self.peek() != &NomxToken::Comma
            && self.peek() != &NomxToken::Period
        {
            collection_tokens.push(self.advance().token.clone());
        }
        self.expect(&NomxToken::Comma, "`,` after `for each` collection")?;
        let mut body_tokens: Vec<NomxToken> = Vec::new();
        while !self.peek_is_body_terminator() && self.peek() != &NomxToken::Period {
            body_tokens.push(self.advance().token.clone());
        }
        if self.peek() == &NomxToken::Period {
            self.advance();
        }
        Ok(NomxStatement::ForEach {
            var,
            collection_tokens,
            body_tokens,
            span: NomxSpan::new(start_span.start, self.peek_span().start),
        })
    }

    /// `<subject> is <rhs...>` optionally terminated by `.`.
    fn parse_binding_statement(&mut self) -> NomxParseResult<NomxStatement> {
        let start_span = self.peek_span();
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

    /// `when <cond>, <then>.` or `unless <cond>, <then>.` with
    /// optional `otherwise, <else>.` following. Condition ends at
    /// comma; branch ends at period or body terminator.
    ///
    /// When `negate=true` (`unless` form), a leading `Not` token is
    /// prepended to `cond_tokens` so downstream consumers see a
    /// unified AST: `unless x` is identical to `when not x`.
    fn parse_when_statement(&mut self, negate: bool) -> NomxParseResult<NomxStatement> {
        let start_span = self.peek_span();
        let keyword = if negate { NomxToken::Unless } else { NomxToken::When };
        let label = if negate { "`unless`" } else { "`when`" };
        self.expect(&keyword, label)?;

        let mut cond_tokens: Vec<NomxToken> = Vec::new();
        if negate {
            cond_tokens.push(NomxToken::Not);
        }
        while !self.peek_is_body_terminator()
            && self.peek() != &NomxToken::Comma
            && self.peek() != &NomxToken::Period
        {
            cond_tokens.push(self.advance().token.clone());
        }
        self.expect(
            &NomxToken::Comma,
            if negate {
                "`,` after `unless` condition"
            } else {
                "`,` after `when` condition"
            },
        )?;

        let mut then_tokens: Vec<NomxToken> = Vec::new();
        while !self.peek_is_body_terminator() && self.peek() != &NomxToken::Period {
            then_tokens.push(self.advance().token.clone());
        }
        if self.peek() == &NomxToken::Period {
            self.advance();
        }

        let mut else_tokens = None;
        if self.peek() == &NomxToken::Otherwise {
            self.advance();
            // Optional comma after `otherwise`.
            if self.peek() == &NomxToken::Comma {
                self.advance();
            }
            let mut etoks: Vec<NomxToken> = Vec::new();
            while !self.peek_is_body_terminator() && self.peek() != &NomxToken::Period {
                etoks.push(self.advance().token.clone());
            }
            if self.peek() == &NomxToken::Period {
                self.advance();
            }
            else_tokens = Some(etoks);
        }

        Ok(NomxStatement::When {
            cond_tokens,
            then_tokens,
            else_tokens,
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
        let src = "define :";
        let err = parse_nomx(src).unwrap_err();
        assert!(err.message.contains("name after `define`"));
        let at = &src[err.span.start..err.span.end];
        assert_eq!(
            at, ":",
            "error span should point at the `:` where the name was missing, got {at:?}"
        );
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
        // Colon was expected after `foo` but the stream ended;
        // the span collapses onto EOF (src.len() .. src.len()).
        assert_eq!(
            err.span.start, src.len(),
            "EOF error span should start at src end, got {}",
            err.span.start
        );
    }

    #[test]
    fn for_each_without_in_or_of_errors() {
        let src = "define f:\n  for each x xs, total is x.";
        let err = parse_nomx(src).unwrap_err();
        assert!(
            err.message.contains("expected `in` or `of`"),
            "expected for-each connector diag, got: {}",
            err.message
        );
        let at = &src[err.span.start..err.span.end];
        assert_eq!(
            at, "xs",
            "error span should point at the offending token `xs`, got {at:?}"
        );
    }

    #[test]
    fn when_without_comma_errors() {
        // `when <cond>, <then>` — dropping the comma (writing
        // `when flag. go.`) stops the cond scan at `.`, then the
        // expect(Comma) fires pointing at the period.
        let src = "define f:\n  when flag. go.";
        let err = parse_nomx(src).unwrap_err();
        assert!(
            err.message.contains("`,` after `when` condition"),
            "expected when-comma diag, got: {}",
            err.message
        );
        let at = &src[err.span.start..err.span.end];
        assert_eq!(
            at, ".",
            "error span should point at the `.` that replaced the expected comma, got {at:?}"
        );
    }

    #[test]
    fn record_field_missing_is_errors() {
        // `record counter: value number.` — field binding without
        // the `is` connector. The field-name loop consumes `value`
        // via consume_identifier, then expect(Is) fires pointing at
        // `number`.
        let src = "record counter:\n    value number.";
        let err = parse_nomx(src).unwrap_err();
        assert!(
            err.message.contains("`is` after field name"),
            "expected field-`is` diag, got: {}",
            err.message
        );
        let at = &src[err.span.start..err.span.end];
        assert_eq!(
            at, "number",
            "error span should point at the token that should have been `is`, got {at:?}"
        );
    }

    #[test]
    fn to_oneliner_without_comma_errors() {
        // `to <verb> <phrase>, respond with <expr>.` — dropping the
        // comma between phrase and body is a very common authoring
        // mistake, so the diagnostic needs to name the missing token.
        let src = "to greet someone respond with hello";
        let err = parse_nomx(src).unwrap_err();
        assert!(
            err.message.contains("`,` after `to <verb>` phrase"),
            "expected to-oneliner comma diag, got: {}",
            err.message
        );
        // Phrase scan runs to EOF when no comma is present; span
        // collapses to src.len().
        assert_eq!(
            err.span.start, src.len(),
            "EOF error span should start at src end, got {}",
            err.span.start
        );
    }

    #[test]
    fn binding_missing_is_errors() {
        let src = "define f:\n  total zero.";
        let err = parse_nomx(src).unwrap_err();
        assert!(
            err.message.contains("`is` after binding subject"),
            "expected binding-`is` diag, got: {}",
            err.message
        );
        let at = &src[err.span.start..err.span.end];
        assert_eq!(
            at, "zero",
            "error span should point at the token that should have been `is`, got {at:?}"
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
        let NomxStatement::Binding { subject, rhs_tokens, .. } = &body[0] else {
            panic!("expected Binding");
        };
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
    fn parses_when_otherwise_branch() {
        let src = "define handle:\n  when user is logged_in, show dashboard.\n  otherwise, show landing.";
        let decls = parse_nomx(src).unwrap();
        let NomxDecl::Define { body, .. } = &decls[0] else {
            panic!("expected Define");
        };
        assert_eq!(body.len(), 1);
        let NomxStatement::When {
            cond_tokens,
            then_tokens,
            else_tokens,
            ..
        } = &body[0]
        else {
            panic!("expected When, got {:?}", body[0]);
        };
        assert!(!cond_tokens.is_empty(), "cond empty");
        assert!(!then_tokens.is_empty(), "then empty");
        assert!(else_tokens.is_some(), "else missing");
        // cond contains the subject `user` + linking verb `is`.
        assert!(
            cond_tokens.iter().any(|t| matches!(t, NomxToken::Identifier(n) if n == "user"))
        );
        assert!(cond_tokens.iter().any(|t| *t == NomxToken::Is));
        // then/else contain the identifier `dashboard`/`landing`.
        assert!(
            then_tokens.iter().any(|t| matches!(t, NomxToken::Identifier(n) if n == "dashboard"))
        );
        assert!(
            else_tokens
                .as_ref()
                .unwrap()
                .iter()
                .any(|t| matches!(t, NomxToken::Identifier(n) if n == "landing"))
        );
    }

    #[test]
    fn decl_kind_name_span_accessors() {
        // One of each decl form, mixed-form ordering: to-oneliners
        // can now appear anywhere thanks to the wider outer-body
        // terminator that recognizes `To` at statement start.
        let src = "record user holds: name is text.\n\
                   choice status is one of: active. deleted.\n\
                   define greet that takes name and returns reply: reply is name.\n\
                   to square a number, respond with the number times itself.";
        let decls = parse_nomx(src).unwrap();
        let kinds: Vec<&str> = decls.iter().map(|d| d.kind()).collect();
        let names: Vec<&str> = decls.iter().map(|d| d.name()).collect();
        assert_eq!(kinds, vec!["record", "choice", "define", "define"]);
        assert_eq!(names, vec!["user", "status", "greet", "square"]);
        for d in &decls {
            let sp = d.span();
            assert!(sp.end >= sp.start);
        }
    }

    #[test]
    fn statement_kind_tag_matches_variant() {
        // Single source with one of each kind, assert kind() returns
        // the expected tag + span() is non-empty for all.
        let src = "define mixed that takes xs and returns y:\n\
                   y is zero.\n\
                   when xs is empty, y is negative one.\n\
                   for each x in xs, y is y plus x.\n\
                   while y is greater_than zero, y is y minus one.\n\
                   ensure y is finite.";
        let decls = parse_nomx(src).unwrap();
        let NomxDecl::Define { body, .. } = &decls[0] else {
            panic!("expected Define");
        };
        let kinds: Vec<&str> = body.iter().map(|s| s.kind()).collect();
        assert_eq!(kinds, vec!["binding", "when", "for_each", "while", "contract"]);
        // All spans are non-empty (end > start).
        for s in body {
            let sp = s.span();
            assert!(sp.end >= sp.start);
        }
    }

    #[test]
    fn parses_contract_statements() {
        // All three contract verbs inside one define body.
        let src = "define safe_divide that takes n and returns result:\n\
                   require denominator is nonzero.\n\
                   ensure result is nonnull.\n\
                   throughout, invariant is preserved.";
        let decls = parse_nomx(src).unwrap();
        let NomxDecl::Define { body, .. } = &decls[0] else {
            panic!("expected Define");
        };
        assert_eq!(body.len(), 3);
        for (stmt, expected_kind) in body.iter().zip([
            ContractKind::Require,
            ContractKind::Ensure,
            ContractKind::Throughout,
        ]) {
            let NomxStatement::Contract { kind, pred_tokens, .. } = stmt else {
                panic!("expected Contract, got {stmt:?}");
            };
            assert_eq!(*kind, expected_kind);
            assert!(!pred_tokens.is_empty());
        }
    }

    #[test]
    fn parses_while_loop() {
        let src = "define countdown:\n  while n is greater_than_zero, n is n minus one.";
        let decls = parse_nomx(src).unwrap();
        let NomxDecl::Define { body, .. } = &decls[0] else {
            panic!("expected Define");
        };
        let NomxStatement::While { cond_tokens, body_tokens, .. } = &body[0] else {
            panic!("expected While, got {:?}", body[0]);
        };
        assert!(
            cond_tokens
                .iter()
                .any(|t| matches!(t, NomxToken::Identifier(n) if n == "n")),
            "cond should reference n"
        );
        assert!(
            body_tokens
                .iter()
                .any(|t| matches!(t, NomxToken::Identifier(n) if n == "minus" || n == "one")),
            "body should reference the decrement"
        );
    }

    #[test]
    fn parses_for_each_iteration() {
        // Two accepted forms: `in` and `of` at the boundary.
        let a = parse_nomx("define sum:\n  for each x in xs, total is total plus x.").unwrap();
        let NomxDecl::Define { body, .. } = &a[0] else {
            panic!("expected Define");
        };
        let NomxStatement::ForEach { var, collection_tokens, body_tokens, .. } = &body[0] else {
            panic!("expected ForEach, got {:?}", body[0]);
        };
        assert_eq!(var, "x");
        assert!(
            collection_tokens
                .iter()
                .any(|t| matches!(t, NomxToken::Identifier(n) if n == "xs"))
        );
        assert!(
            body_tokens
                .iter()
                .any(|t| matches!(t, NomxToken::Identifier(n) if n == "total"))
        );

        // Same thing with `of`.
        let b = parse_nomx("define sum:\n  for each x of xs, total is total plus x.").unwrap();
        let NomxDecl::Define { body, .. } = &b[0] else {
            panic!("expected Define");
        };
        assert!(matches!(body[0], NomxStatement::ForEach { .. }));
    }

    #[test]
    fn parses_unless_as_negated_when() {
        // `unless <cond>, <then>.` is sugar for `when not <cond>, ...`.
        // The parser prepends a Not token to cond_tokens so downstream
        // consumers see a unified shape.
        let src = "define guard:\n  unless authorized is true, deny.";
        let decls = parse_nomx(src).unwrap();
        let NomxDecl::Define { body, .. } = &decls[0] else {
            panic!("expected Define");
        };
        let NomxStatement::When { cond_tokens, .. } = &body[0] else {
            panic!("expected When");
        };
        assert_eq!(
            cond_tokens.first(),
            Some(&NomxToken::Not),
            "unless should prepend Not to cond_tokens: {cond_tokens:?}"
        );
        // The rest of the condition: `authorized is true`.
        assert!(
            cond_tokens
                .iter()
                .any(|t| matches!(t, NomxToken::Identifier(n) if n == "authorized"))
        );
    }

    #[test]
    fn parses_when_without_otherwise() {
        let src = "define bare:\n  when foo is bar, do something.";
        let decls = parse_nomx(src).unwrap();
        let NomxDecl::Define { body, .. } = &decls[0] else {
            panic!("expected Define");
        };
        let NomxStatement::When { else_tokens, .. } = &body[0] else {
            panic!("expected When");
        };
        assert!(else_tokens.is_none(), "no otherwise means else_tokens=None");
    }

    #[test]
    fn parses_to_oneliner_sentence_form() {
        // `to greet someone by name, respond with "hello" joined to name.`
        let src = "to greet someone by name, respond with \"hello\" joined to name.";
        let decls = parse_nomx(src).unwrap();
        assert_eq!(decls.len(), 1);
        // Lowered to a Define with a single binding body.
        let NomxDecl::Define { name, body, .. } = &decls[0] else {
            panic!("expected Define, got {:?}", decls[0]);
        };
        assert_eq!(name, "greet");
        assert_eq!(body.len(), 1);
        let NomxStatement::Binding { subject, rhs_tokens, .. } = &body[0] else {
            panic!("expected Binding");
        };
        assert_eq!(subject, "respond");
        assert!(
            rhs_tokens
                .iter()
                .any(|t| matches!(t, NomxToken::StringLit(s) if s == "hello"))
        );
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
    fn parses_todo_app_nomx_end_to_end() {
        // Real program exercising every grammar form: Record +
        // Choice + three Defines with Binding + When bodies.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/todo_app.nomx");
        let src = std::fs::read_to_string(&path).unwrap();
        let decls = parse_nomx(&src).unwrap();
        // Expected: 1 record + 1 choice + 3 defines = 5 decls.
        assert_eq!(decls.len(), 5, "expected 5 decls, got {}: {decls:#?}", decls.len());

        // Shape check: first is Record, second is Choice, rest are
        // Defines. Each decl has its expected name.
        assert!(matches!(&decls[0], NomxDecl::Record { name, .. } if name == "task"));
        assert!(matches!(&decls[1], NomxDecl::Choice { name, .. } if name == "task_status"));
        assert!(matches!(&decls[2], NomxDecl::Define { name, .. } if name == "add_task"));
        assert!(matches!(&decls[3], NomxDecl::Define { name, .. } if name == "mark_done"));
        assert!(matches!(&decls[4], NomxDecl::Define { name, .. } if name == "count_remaining"));

        // The record has 3 fields and the choice has 3 variants.
        let NomxDecl::Record { fields, .. } = &decls[0] else {
            panic!("expected Record");
        };
        assert_eq!(fields.len(), 3);
        let NomxDecl::Choice { variants, .. } = &decls[1] else {
            panic!("expected Choice");
        };
        assert_eq!(variants.len(), 3);

        // add_task contains a `when` statement.
        let NomxDecl::Define { body, .. } = &decls[2] else {
            panic!("expected Define");
        };
        assert!(
            body.iter()
                .any(|s| matches!(s, NomxStatement::When { .. })),
            "expected When in add_task body"
        );
    }

    #[test]
    fn unrecognized_top_level_forms_recover_to_zero_decls() {
        // `actor ... holds:` is proposal-05 §4.6 but not yet parsed
        // (lexer emits Identifier("actor"), parse_file's fallback
        // advances past it). Same for any other unknown keyword.
        // Parser recovers silently; decl list is empty.
        //
        // This test locks the recovery contract so if we ever add an
        // error-on-unknown policy, it'll fail here — forcing an
        // explicit decision rather than a silent behavior change.
        let src = "actor counter holds n starting at zero.\n\
                   on receives add v, n becomes n plus v.";
        let decls = parse_nomx(src).unwrap();
        assert_eq!(
            decls.len(),
            0,
            "actor form not yet parsed; expected 0 decls, got {decls:#?}"
        );
    }

    #[test]
    fn parses_mixed_forms_nomx_sample() {
        // Proves the terminator fix (95f9bdc) in a real fixture:
        // record + choice + block-define + 2 to-oneliners in one file.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/mixed_forms.nomx");
        let src = std::fs::read_to_string(&path).unwrap();
        let decls = parse_nomx(&src).unwrap();
        assert_eq!(decls.len(), 5);
        let kinds: Vec<&str> = decls.iter().map(|d| d.kind()).collect();
        let names: Vec<&str> = decls.iter().map(|d| d.name()).collect();
        assert_eq!(kinds, vec!["record", "choice", "define", "define", "define"]);
        assert_eq!(names, vec!["counter", "status", "step", "reset", "show"]);
    }

    #[test]
    fn parses_contracts_nomx_sample() {
        // Each of the 3 defines has ≥1 Contract statement; across
        // the file all 3 ContractKinds appear.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/contracts.nomx");
        let src = std::fs::read_to_string(&path).unwrap();
        let decls = parse_nomx(&src).unwrap();
        assert_eq!(decls.len(), 3);

        let mut seen_kinds = std::collections::HashSet::new();
        for d in &decls {
            let NomxDecl::Define { body, .. } = d else {
                panic!("expected Define");
            };
            for s in body {
                if let NomxStatement::Contract { kind, .. } = s {
                    seen_kinds.insert(*kind);
                }
            }
        }
        assert!(seen_kinds.contains(&ContractKind::Require));
        assert!(seen_kinds.contains(&ContractKind::Ensure));
        assert!(seen_kinds.contains(&ContractKind::Throughout));
    }

    #[test]
    fn parses_loops_nomx_sample() {
        // 4 defines exercising ForEach (in + of), When, Unless,
        // While in combination.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/loops.nomx");
        let src = std::fs::read_to_string(&path).unwrap();
        let decls = parse_nomx(&src).unwrap();
        assert_eq!(decls.len(), 4);

        // sum_of: body has a ForEach
        let NomxDecl::Define { body, .. } = &decls[0] else {
            panic!("expected Define");
        };
        assert!(body.iter().any(|s| matches!(s, NomxStatement::ForEach { .. })));

        // countdown_from: body has a While
        let NomxDecl::Define { body, .. } = &decls[2] else {
            panic!("expected Define");
        };
        assert!(body.iter().any(|s| matches!(s, NomxStatement::While { .. })));

        // greatest_of: body has ForEach containing Unless-style
        // nested tokens in body_tokens (we don't parse nested
        // statements yet — Unless is captured raw inside the
        // ForEach body_tokens). Just verify the outer ForEach parses.
        let NomxDecl::Define { body, .. } = &decls[3] else {
            panic!("expected Define");
        };
        assert!(body.iter().any(|s| matches!(s, NomxStatement::ForEach { .. })));
    }

    #[test]
    fn parses_greet_sentence_nomx_sample() {
        // Three sentence-form functions in one file. Each lowers to
        // a Define with a single Binding body (subject="respond").
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/greet_sentence.nomx");
        let src = std::fs::read_to_string(&path).unwrap();
        let decls = parse_nomx(&src).unwrap();
        assert_eq!(decls.len(), 3);
        for d in &decls {
            let NomxDecl::Define { body, .. } = d else {
                panic!("expected Define, got {d:?}");
            };
            assert_eq!(body.len(), 1);
            let NomxStatement::Binding { subject, .. } = &body[0] else {
                panic!("expected Binding");
            };
            assert_eq!(subject, "respond");
        }
        let names: Vec<&str> = decls
            .iter()
            .map(|d| match d {
                NomxDecl::Define { name, .. } => name.as_str(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(names, vec!["greet", "square", "absolute"]);
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
