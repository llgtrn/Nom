//! nom-parser: Recursive-descent parser for the Nom language.
//!
//! Consumes a flat `Vec<SpannedToken>` from the lexer and produces a
//! [`nom_ast::SourceFile`] containing typed [`Declaration`]s.
//!
//! # Grammar sketch
//!
//! ```text
//! source_file   ::= declaration* EOF
//! declaration   ::= classifier IDENT statement* (BLANK_LINE | classifier | EOF)
//! statement     ::= need_stmt | require_stmt | effects_stmt | flow_stmt
//!                 | describe_stmt | contract_stmt | implement_stmt
//!                 | given_stmt   | when_stmt     | then_stmt     | and_stmt
//! flow_stmt     ::= "flow" flow_chain
//! flow_chain    ::= flow_step ("->" flow_step)*
//! flow_step     ::= IDENT ("::" IDENT)?   (NomRef)
//!                 | "{" branch_arm+ "}"   (BranchBlock)
//! branch_arm    ::= ("iftrue" | "iffalse") "->" flow_chain
//! need_stmt     ::= "need" nom_ref ("where" constraint)?
//! require_stmt  ::= "require" constraint
//! effects_stmt  ::= "effects" ("only")? "[" IDENT* "]"
//! describe_stmt ::= "describe" STRING
//! contract_stmt ::= "contract" (statement)*
//! implement_stmt::= "implement" IDENT "{" ... "}"
//! ```

use nom_ast::{
    AgentCapabilityStmt, AgentReceiveStmt, AgentScheduleStmt, AgentStateStmt, AgentSuperviseStmt,
    AssignStmt, Block, BlockStmt, BranchArm, BranchBlock, BranchCondition, CallExpr, Classifier,
    CompareOp, Constraint, ContractStmt, Declaration, DescribeStmt, EffectModifier, EffectsStmt,
    EnumDef, EnumVariant, Expr, FlowChain, FlowQualifier, FlowStep, FlowStmt, FnDef, FnParam, ForStmt,
    ImplBlock, ModStmt, OnFailStrategy,
    GraphConstraintStmt, GraphEdgeStmt, GraphNodeStmt, GraphQueryExpr, GraphQueryStmt,
    GraphSetExpr, GraphSetOp, GraphTraverseExpr, Identifier, IfExpr, ImplementStmt, LetStmt,
    Literal, MatchArm, MatchExpr, NeedStmt, NomRef, Pattern, RequireStmt, SourceFile, Span,
    Statement, StructDef, StructField, TestAndStmt, TestGivenStmt, TestThenStmt, TestWhenStmt,
    TraitDef, TypeExpr, TypedParam, UseImport, UseStmt, WhileStmt,
};
use nom_lexer::{SpannedToken, Token};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected token {found} at line {line}, col {col}: expected {expected}")]
    UnexpectedToken {
        found: String,
        expected: String,
        line: usize,
        col: usize,
    },
    #[error("unexpected end of file: expected {expected}")]
    UnexpectedEof { expected: String },
}

type ParseResult<T> = Result<T, ParseError>;

struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    source: Option<String>,
}

impl Parser {
    fn new(tokens: Vec<SpannedToken>) -> Self {
        Self {
            tokens,
            pos: 0,
            source: None,
        }
    }

    fn with_source(tokens: Vec<SpannedToken>, source: String) -> Self {
        Self {
            tokens,
            pos: 0,
            source: Some(source),
        }
    }

    // ── Token navigation ─────────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        // skip comments for the lookahead
        let mut i = self.pos;
        while i < self.tokens.len() {
            match &self.tokens[i].token {
                Token::Comment(_) | Token::Newline => i += 1,
                t => return t,
            }
        }
        &Token::Eof
    }

    fn peek_raw(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    fn peek_span(&self) -> Span {
        let mut i = self.pos;
        while i < self.tokens.len() {
            match &self.tokens[i].token {
                Token::Comment(_) | Token::Newline => i += 1,
                _ => return self.tokens[i].span,
            }
        }
        self.tokens.last().map(|t| t.span).unwrap_or_default()
    }

    fn advance(&mut self) -> &SpannedToken {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.tokens.len() {
            match &self.tokens[self.pos].token {
                Token::Comment(_) | Token::Newline => {
                    self.pos += 1;
                }
                _ => break,
            }
        }
    }

    fn at_statement_boundary(&self) -> bool {
        match self.peek() {
            Token::BlankLine | Token::Eof => true,
            t if is_classifier(t) => {
                // `flow` is both a classifier and a statement keyword.
                // It's a statement boundary (new declaration) only when
                // followed by IDENT + newline/blank/EOF (i.e. a declaration header).
                // When followed by IDENT -> ... it's a flow statement inside
                // the current declaration.
                if matches!(t, Token::Flow) {
                    // Look ahead: skip Flow, see if there's IDENT then Arrow
                    let mut i = self.pos;
                    // skip the Flow token
                    i += 1;
                    // skip whitespace tokens
                    while i < self.tokens.len()
                        && matches!(self.tokens[i].token, Token::Newline | Token::Comment(_))
                    {
                        i += 1;
                    }
                    // flow::qualifier is always a flow STATEMENT, not a new declaration
                    if i < self.tokens.len() && matches!(self.tokens[i].token, Token::ColCol) {
                        return false;
                    }
                    // skip the identifier
                    if i < self.tokens.len() && matches!(&self.tokens[i].token, Token::Ident(_)) {
                        i += 1;
                        // skip whitespace
                        while i < self.tokens.len()
                            && matches!(self.tokens[i].token, Token::Newline | Token::Comment(_))
                        {
                            i += 1;
                        }
                        // If next is Arrow, it's a flow STATEMENT (not a new declaration)
                        if i < self.tokens.len() && matches!(self.tokens[i].token, Token::Arrow) {
                            return false;
                        }
                    }
                    true
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    fn consume_blanks(&mut self) {
        while matches!(
            self.peek_raw(),
            Token::BlankLine | Token::Newline | Token::Comment(_)
        ) {
            self.pos += 1;
        }
    }

    fn expect_ident(&mut self) -> ParseResult<Identifier> {
        self.skip_whitespace();
        let st = self
            .tokens
            .get(self.pos)
            .ok_or_else(|| ParseError::UnexpectedEof {
                expected: "identifier".to_owned(),
            })?;
        let span = st.span;
        match &st.token {
            Token::Ident(s) => {
                let name = s.clone();
                self.pos += 1;
                Ok(Identifier::new(name, span))
            }
            // classifier keywords can appear as identifiers (e.g., declaration names)
            t if is_classifier(t) => {
                let name = classifier_str(t).to_owned();
                self.pos += 1;
                Ok(Identifier::new(name, span))
            }
            // statement keywords can appear as identifiers in some contexts
            t if is_statement_keyword(t) => {
                let name = statement_keyword_str(t).to_owned();
                self.pos += 1;
                Ok(Identifier::new(name, span))
            }
            t => Err(ParseError::UnexpectedToken {
                found: format!("{t:?}"),
                expected: "identifier".to_owned(),
                line: span.line,
                col: span.col,
            }),
        }
    }

    // ── Top-level ────────────────────────────────────────────────────────────

    fn parse_source_file(&mut self) -> ParseResult<SourceFile> {
        let mut declarations = Vec::new();
        let mut errors: Vec<ParseError> = Vec::new();
        self.consume_blanks();
        while !matches!(self.peek(), Token::Eof) {
            match self.parse_declaration() {
                Ok(decl) => declarations.push(decl),
                Err(e) => {
                    errors.push(e);
                    // Recovery: skip to next declaration boundary (blank line or classifier)
                    self.recover_to_next_declaration();
                }
            }
            self.consume_blanks();
        }
        // If we parsed nothing and had errors, return the first error
        if declarations.is_empty() && !errors.is_empty() {
            return Err(errors.remove(0));
        }
        // If we parsed some declarations but had errors, report via eprintln
        // (partial success — return what we could parse)
        for err in &errors {
            eprintln!("nom: parse warning (recovered): {err}");
        }
        Ok(SourceFile {
            path: None,
            locale: None,
            declarations,
        })
    }

    /// Skip tokens until we reach a blank line, a classifier keyword, or EOF.
    fn recover_to_next_declaration(&mut self) {
        loop {
            match self.peek() {
                Token::Eof => break,
                Token::BlankLine => {
                    self.advance();
                    break;
                }
                tok if is_classifier(tok) => break, // Don't consume the classifier
                _ => { self.advance(); }
            }
        }
    }

    /// Skip tokens until we reach a newline, blank line, statement keyword, classifier, or EOF.
    fn recover_to_next_statement(&mut self) {
        loop {
            match self.peek() {
                Token::Eof | Token::BlankLine => break,
                Token::Newline => {
                    self.advance();
                    break;
                }
                tok if is_classifier(tok) || is_statement_keyword(tok) => break,
                _ => { self.advance(); }
            }
        }
    }

    fn parse_declaration(&mut self) -> ParseResult<Declaration> {
        self.skip_whitespace();
        let start_span = self.peek_span();

        // consume classifier
        let classifier = match self.peek().clone() {
            Token::System => {
                self.advance();
                Classifier::System
            }
            Token::Flow => {
                self.advance();
                Classifier::Flow
            }
            Token::Store => {
                self.advance();
                Classifier::Store
            }
            Token::Graph => {
                self.advance();
                Classifier::Graph
            }
            Token::Agent => {
                self.advance();
                Classifier::Agent
            }
            Token::Test => {
                self.advance();
                Classifier::Test
            }
            Token::Nom => {
                self.advance();
                Classifier::Nom
            }
            Token::Gate => {
                self.advance();
                Classifier::Gate
            }
            Token::Pool => {
                self.advance();
                Classifier::Pool
            }
            Token::View => {
                self.advance();
                Classifier::View
            }
            other => {
                return Err(ParseError::UnexpectedToken {
                    found: format!("{other:?}"),
                    expected: "classifier keyword".to_owned(),
                    line: start_span.line,
                    col: start_span.col,
                });
            }
        };

        let name = self.expect_ident()?;
        let mut statements = Vec::new();

        // consume newline after header
        self.skip_newlines_not_blank();

        // parse statements until blank line, next classifier, or EOF
        loop {
            self.skip_whitespace();
            if self.at_statement_boundary() {
                break;
            }
            match self.parse_statement() {
                Ok(Some(stmt)) => statements.push(stmt),
                Ok(None) => break,
                Err(e) => {
                    // Recovery within declaration: skip to next newline/statement keyword
                    eprintln!("nom: parse warning (recovered): {e}");
                    self.recover_to_next_statement();
                }
            }
        }

        let end_span = self.peek_span();
        Ok(Declaration {
            classifier,
            name,
            statements,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.line,
                start_span.col,
            ),
        })
    }

    fn skip_newlines_not_blank(&mut self) {
        while matches!(self.peek_raw(), Token::Newline | Token::Comment(_)) {
            self.pos += 1;
        }
    }

    fn parse_statement(&mut self) -> ParseResult<Option<Statement>> {
        self.skip_whitespace();
        match self.peek().clone() {
            Token::Need => Ok(Some(Statement::Need(self.parse_need()?))),
            Token::Require => Ok(Some(Statement::Require(self.parse_require()?))),
            Token::Effects => Ok(Some(Statement::Effects(self.parse_effects()?))),
            Token::Flow => Ok(Some(Statement::Flow(self.parse_flow_stmt()?))),
            Token::Describe => Ok(Some(Statement::Describe(self.parse_describe()?))),
            Token::Contract => Ok(Some(Statement::Contract(self.parse_contract()?))),
            Token::Implement => Ok(Some(Statement::Implement(self.parse_implement()?))),
            Token::Given => Ok(Some(Statement::Given(self.parse_given()?))),
            Token::When => Ok(Some(Statement::When(self.parse_when()?))),
            Token::Then => Ok(Some(Statement::Then(self.parse_then()?))),
            Token::And => Ok(Some(Statement::And(self.parse_and()?))),
            // ── Graph-native statements ─────────────────────────────────────
            Token::Node => Ok(Some(Statement::GraphNode(self.parse_graph_node()?))),
            Token::Edge => Ok(Some(Statement::GraphEdge(self.parse_graph_edge()?))),
            Token::Query => Ok(Some(Statement::GraphQuery(self.parse_graph_query()?))),
            Token::Constraint => Ok(Some(Statement::GraphConstraint(
                self.parse_graph_constraint()?,
            ))),
            // ── Agent-native statements ─────────────────────────────────────
            Token::Capability => Ok(Some(Statement::AgentCapability(
                self.parse_agent_capability()?,
            ))),
            Token::Supervise => Ok(Some(Statement::AgentSupervise(
                self.parse_agent_supervise()?,
            ))),
            Token::Receive => Ok(Some(Statement::AgentReceive(self.parse_agent_receive()?))),
            Token::State => Ok(Some(Statement::AgentState(self.parse_agent_state()?))),
            Token::Schedule => Ok(Some(Statement::AgentSchedule(self.parse_agent_schedule()?))),
            // ── Imperative statements ───────────────────────────────────────
            Token::Let => Ok(Some(Statement::Let(self.parse_let_stmt()?))),
            Token::If => Ok(Some(Statement::If(self.parse_if_expr()?))),
            Token::For => Ok(Some(Statement::For(self.parse_for_stmt()?))),
            Token::While => Ok(Some(Statement::While(self.parse_while_stmt()?))),
            Token::Match => Ok(Some(Statement::Match(self.parse_match_expr()?))),
            Token::Return => Ok(Some(self.parse_return_stmt()?)),
            Token::Fn => Ok(Some(Statement::FnDef(self.parse_fn_def(false)?))),
            Token::Pub => {
                // peek ahead: pub fn, pub struct, pub enum, pub trait
                let start = self.peek_span();
                self.advance(); // consume 'pub'
                match self.peek().clone() {
                    Token::Fn => Ok(Some(Statement::FnDef(self.parse_fn_def(true)?))),
                    Token::Struct => Ok(Some(Statement::StructDef(self.parse_struct_def(true)?))),
                    Token::Enum => Ok(Some(Statement::EnumDef(self.parse_enum_def(true)?))),
                    Token::Trait => Ok(Some(Statement::TraitDef(self.parse_trait_def(true)?))),
                    other => Err(ParseError::UnexpectedToken {
                        found: format!("{other:?}"),
                        expected: "fn, struct, enum, or trait after pub".to_owned(),
                        line: start.line,
                        col: start.col,
                    }),
                }
            }
            Token::Struct => Ok(Some(Statement::StructDef(self.parse_struct_def(false)?))),
            Token::Enum => Ok(Some(Statement::EnumDef(self.parse_enum_def(false)?))),
            // ── Trait / impl ──────────────────────────────────────────────
            Token::Trait => Ok(Some(Statement::TraitDef(self.parse_trait_def(false)?))),
            Token::Impl => Ok(Some(Statement::ImplBlock(self.parse_impl_block()?))),
            // ── Module system ──────────────────────────────────────────────
            Token::Use => Ok(Some(Statement::Use(self.parse_use_stmt()?))),
            Token::Mod => Ok(Some(Statement::Mod(self.parse_mod_stmt()?))),
            Token::Eof | Token::BlankLine => Ok(None),
            t if is_classifier(&t) => Ok(None),
            Token::Newline => {
                self.advance();
                Ok(Some(self.parse_statement()?.unwrap_or_else(|| {
                    // return a no-op; caller will loop
                    Statement::Describe(DescribeStmt {
                        text: String::new(),
                        span: Span::default(),
                    })
                })))
            }
            _ => Ok(None),
        }
    }

    // ── need statement ───────────────────────────────────────────────────────

    fn parse_need(&mut self) -> ParseResult<NeedStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'need'
        let reference = self.parse_nom_ref()?;
        let constraint = if matches!(self.peek(), Token::Where) {
            self.advance(); // consume 'where'
            Some(self.parse_constraint()?)
        } else {
            None
        };
        let end = self.peek_span();
        Ok(NeedStmt {
            reference,
            constraint,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_nom_ref(&mut self) -> ParseResult<NomRef> {
        let start = self.peek_span();
        let word = self.expect_ident()?;
        let variant = if matches!(self.peek(), Token::ColCol) {
            self.advance(); // consume '::'
            Some(self.expect_ident()?)
        } else {
            None
        };
        let end = self.peek_span();
        Ok(NomRef {
            word,
            variant,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── require statement ────────────────────────────────────────────────────

    fn parse_require(&mut self) -> ParseResult<RequireStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'require'
        let constraint = self.parse_constraint()?;
        let end = self.peek_span();
        Ok(RequireStmt {
            constraint,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_constraint(&mut self) -> ParseResult<Constraint> {
        let start = self.peek_span();
        let left = self.parse_additive_expr()?;
        let op = match self.peek().clone() {
            Token::Gt => {
                self.advance();
                CompareOp::Gt
            }
            Token::Lt => {
                self.advance();
                CompareOp::Lt
            }
            Token::Gte => {
                self.advance();
                CompareOp::Gte
            }
            Token::Lte => {
                self.advance();
                CompareOp::Lte
            }
            Token::Eq => {
                self.advance();
                CompareOp::Eq
            }
            Token::Neq => {
                self.advance();
                CompareOp::Neq
            }
            other => {
                let s = self.peek_span();
                return Err(ParseError::UnexpectedToken {
                    found: format!("{other:?}"),
                    expected: "comparison operator".to_owned(),
                    line: s.line,
                    col: s.col,
                });
            }
        };
        let right = self.parse_additive_expr()?;
        let end = self.peek_span();
        Ok(Constraint {
            left,
            op,
            right,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_logical_and()?;
        while matches!(self.peek(), Token::Or) {
            self.advance();
            let right = self.parse_logical_and()?;
            expr = Expr::BinaryOp(Box::new(expr), nom_ast::BinOp::Or, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_comparison_expr()?;
        while matches!(self.peek(), Token::And) {
            self.advance();
            let right = self.parse_comparison_expr()?;
            expr = Expr::BinaryOp(Box::new(expr), nom_ast::BinOp::And, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_comparison_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_additive_expr()?;
        loop {
            let op = match self.peek().clone() {
                Token::Gt => nom_ast::BinOp::Gt,
                Token::Lt => nom_ast::BinOp::Lt,
                Token::Gte => nom_ast::BinOp::Gte,
                Token::Lte => nom_ast::BinOp::Lte,
                // `==` is Token::EqEq; `Token::Eq` (single `=`) is assignment
                // and is NOT a valid equality operator. This matches the
                // Nom lexer's two-token split — see crates/nom-lexer.
                Token::EqEq => nom_ast::BinOp::Eq,
                Token::Neq => nom_ast::BinOp::Neq,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive_expr()?;
            expr = Expr::BinaryOp(Box::new(expr), op, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_additive_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_multiplicative_expr()?;
        loop {
            let op = match self.peek().clone() {
                Token::Plus => nom_ast::BinOp::Add,
                Token::Minus => nom_ast::BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative_expr()?;
            expr = Expr::BinaryOp(Box::new(expr), op, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_multiplicative_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_unary_expr()?;
        loop {
            let op = match self.peek().clone() {
                Token::Star => nom_ast::BinOp::Mul,
                Token::Slash => nom_ast::BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary_expr()?;
            expr = Expr::BinaryOp(Box::new(expr), op, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_unary_expr(&mut self) -> ParseResult<Expr> {
        // Unary negation: -expr
        if matches!(self.peek(), Token::Minus) {
            self.advance();
            let expr = self.parse_unary_expr()?;
            return Ok(Expr::UnaryOp(nom_ast::UnaryOp::Neg, Box::new(expr)));
        }
        // Unary not: !expr
        if matches!(self.peek(), Token::Bang) {
            self.advance();
            let expr = self.parse_unary_expr()?;
            return Ok(Expr::UnaryOp(nom_ast::UnaryOp::Not, Box::new(expr)));
        }
        // Ref: &expr, &mut expr
        if matches!(self.peek(), Token::Ampersand) {
            self.advance();
            let mutable = matches!(self.peek(), Token::Mut);
            if mutable { self.advance(); }
            let expr = self.parse_unary_expr()?;
            let op = if mutable { nom_ast::UnaryOp::RefMut } else { nom_ast::UnaryOp::Ref };
            return Ok(Expr::UnaryOp(op, Box::new(expr)));
        }
        // Array literal: [expr, expr, ...]
        if matches!(self.peek(), Token::LBracket) {
            self.advance();
            let mut items = Vec::new();
            while !matches!(self.peek(), Token::RBracket | Token::Eof) {
                items.push(self.parse_expr()?);
                if matches!(self.peek(), Token::Comma) { self.advance(); }
            }
            if matches!(self.peek(), Token::RBracket) { self.advance(); }
            return Ok(Expr::Array(items));
        }
        self.parse_postfix_expr()
    }

    fn parse_postfix_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_expr_primary()?;

        // Qualified path `Enum::Variant`: while the current expr is a plain
        // Ident and the next token is `::`, consume the `::` and next ident,
        // merging both names into a single dotted identifier
        // (`"Enum::Variant"`). The subsequent LParen branch of the postfix
        // loop then produces `Expr::Call` with the qualified callee, which is
        // how enum variant construction is represented for the LLVM backend.
        while matches!(&expr, Expr::Ident(_)) && matches!(self.peek(), Token::ColCol) {
            self.advance(); // consume '::'
            let next = self.expect_ident()?;
            if let Expr::Ident(prev) = expr {
                let merged = format!("{}::{}", prev.name, next.name);
                expr = Expr::Ident(Identifier::new(merged, prev.span));
            }
        }

        // Postfix loop: field access, method calls, index, function calls
        loop {
            match self.peek().clone() {
                Token::Dot => {
                    self.advance(); // '.'
                    // Tuple field access: `pair.0`, `pair.1`, ... — accept
                    // an integer token after `.` and turn the digits into the
                    // field name. Keeps the AST unchanged; the codegen path
                    // detects numeric field names and emits `extractvalue`.
                    if let Token::Integer(n) = self.peek().clone() {
                        let span = self.peek_span();
                        self.advance();
                        if n < 0 {
                            return Err(ParseError::UnexpectedToken {
                                found: format!("{}", n),
                                expected: "non-negative tuple index".to_owned(),
                                line: span.line,
                                col: span.col,
                            });
                        }
                        let field = Identifier::new(format!("{}", n), span);
                        expr = Expr::FieldAccess(Box::new(expr), field);
                        continue;
                    }
                    let field = self.expect_ident()?;
                    // Check for method call: expr.method(args)
                    if matches!(self.peek(), Token::LParen) {
                        self.advance(); // '('
                        let mut args = Vec::new();
                        while !matches!(self.peek(), Token::RParen | Token::Eof) {
                            args.push(self.parse_expr()?);
                            if matches!(self.peek(), Token::Comma) { self.advance(); }
                        }
                        if matches!(self.peek(), Token::RParen) { self.advance(); }
                        expr = Expr::MethodCall(Box::new(expr), field, args);
                    } else {
                        expr = Expr::FieldAccess(Box::new(expr), field);
                    }
                }
                Token::LBracket => {
                    self.advance(); // '['
                    let lo = self.parse_expr()?;
                    // Range slice: `[lo..hi]`.
                    let index = if matches!(self.peek(), Token::DotDot) {
                        self.advance(); // '..'
                        let hi = self.parse_expr()?;
                        Expr::Range(Box::new(lo), Box::new(hi))
                    } else {
                        lo
                    };
                    if matches!(self.peek(), Token::RBracket) { self.advance(); }
                    expr = Expr::Index(Box::new(expr), Box::new(index));
                }
                Token::LParen if matches!(&expr, Expr::Ident(_)) => {
                    // Function call: name(args)
                    self.advance(); // '('
                    let mut args = Vec::new();
                    while !matches!(self.peek(), Token::RParen | Token::Eof) {
                        args.push(self.parse_expr()?);
                        if matches!(self.peek(), Token::Comma) { self.advance(); }
                    }
                    if matches!(self.peek(), Token::RParen) { self.advance(); }
                    if let Expr::Ident(id) = expr {
                        expr = Expr::Call(CallExpr {
                            callee: id,
                            args,
                            span: Span::default(),
                        });
                    }
                }
                Token::Question => {
                    // Try/propagate operator: expr?
                    self.advance();
                    expr = Expr::Try(Box::new(expr));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    /// Lookahead: assuming `self.peek()` is `LBrace`, decide whether the
    /// contents look like a struct-literal body (`ident : ...`) as opposed
    /// to a block. Does not mutate parser state.
    fn looks_like_struct_literal_body(&self) -> bool {
        // Find the index of the LBrace after any whitespace/comments/newlines
        // that `peek()` already skips. We need to scan the raw token stream
        // because `peek()` only returns the token, not its index.
        let mut i = self.pos;
        while i < self.tokens.len() {
            match &self.tokens[i].token {
                Token::Comment(_) | Token::Newline => i += 1,
                _ => break,
            }
        }
        if !matches!(self.tokens.get(i).map(|t| &t.token), Some(Token::LBrace)) {
            return false;
        }
        i += 1;
        while i < self.tokens.len() {
            match &self.tokens[i].token {
                Token::Comment(_) | Token::Newline => i += 1,
                _ => break,
            }
        }
        // Empty `{}` is allowed as a zero-field struct literal.
        if matches!(self.tokens.get(i).map(|t| &t.token), Some(Token::RBrace)) {
            return true;
        }
        // `ident :` => struct literal body.
        if !matches!(self.tokens.get(i).map(|t| &t.token), Some(Token::Ident(_))) {
            return false;
        }
        i += 1;
        while i < self.tokens.len() {
            match &self.tokens[i].token {
                Token::Comment(_) | Token::Newline => i += 1,
                _ => break,
            }
        }
        matches!(self.tokens.get(i).map(|t| &t.token), Some(Token::Colon))
    }

    /// Parse `Name { field: expr, field: expr, ... }` given that `Name` has
    /// already been consumed. Caller must have validated via
    /// `looks_like_struct_literal_body` that the upcoming token is `LBrace`.
    fn parse_struct_literal(&mut self, name: Identifier) -> ParseResult<Expr> {
        // Consume '{'. Use skip_whitespace first so the following advance()
        // actually lands on the `{` (advance() is raw and does not skip
        // whitespace/newlines/comments itself). Same pattern applies at every
        // explicit token consumption in this method.
        self.skip_whitespace();
        if !matches!(self.peek_raw(), Token::LBrace) {
            let s = self.peek_span();
            return Err(ParseError::UnexpectedToken {
                found: format!("{:?}", self.peek()),
                expected: "'{' in struct literal".to_owned(),
                line: s.line,
                col: s.col,
            });
        }
        self.advance(); // '{'
        let mut fields: Vec<(Identifier, Expr)> = Vec::new();
        loop {
            self.skip_whitespace();
            if matches!(self.peek_raw(), Token::RBrace | Token::Eof) {
                break;
            }
            let field_name = self.expect_ident()?;
            self.skip_whitespace();
            if !matches!(self.peek_raw(), Token::Colon) {
                let s = self.peek_span();
                return Err(ParseError::UnexpectedToken {
                    found: format!("{:?}", self.peek()),
                    expected: "':' after struct field name".to_owned(),
                    line: s.line,
                    col: s.col,
                });
            }
            self.advance(); // ':'
            let value = self.parse_expr()?;
            fields.push((field_name, value));
            self.skip_whitespace();
            if matches!(self.peek_raw(), Token::Comma) {
                self.advance();
            }
        }
        self.skip_whitespace();
        if matches!(self.peek_raw(), Token::RBrace) {
            self.advance();
        }
        Ok(Expr::StructInit { name, fields })
    }

    fn parse_expr_primary(&mut self) -> ParseResult<Expr> {
        self.skip_whitespace();
        match self.peek().clone() {
            Token::Integer(n) => {
                self.advance();
                Ok(Expr::Literal(Literal::Integer(n)))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expr::Literal(Literal::Number(f)))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Expr::Literal(Literal::Text(s)))
            }
            Token::Bool(value) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(value)))
            }
            Token::NoneLit => {
                self.advance();
                Ok(Expr::Literal(Literal::None))
            }
            Token::LParen => {
                self.advance();
                let first = self.parse_expr()?;
                // Tuple literal: `(a, b, ...)`. A single `(expr)` is still
                // treated as parenthesized-expr; only when we see a comma do
                // we commit to a tuple.
                if matches!(self.peek(), Token::Comma) {
                    let mut items = vec![first];
                    while matches!(self.peek(), Token::Comma) {
                        self.advance();
                        // Trailing comma tolerated: `(a, b,)`.
                        if matches!(self.peek(), Token::RParen) {
                            break;
                        }
                        items.push(self.parse_expr()?);
                    }
                    if matches!(self.peek(), Token::RParen) {
                        self.advance();
                        return Ok(Expr::TupleExpr(items));
                    }
                    let s = self.peek_span();
                    return Err(ParseError::UnexpectedToken {
                        found: format!("{:?}", self.peek()),
                        expected: ")".to_owned(),
                        line: s.line,
                        col: s.col,
                    });
                }
                if matches!(self.peek(), Token::RParen) {
                    self.advance();
                    Ok(first)
                } else {
                    let s = self.peek_span();
                    Err(ParseError::UnexpectedToken {
                        found: format!("{:?}", self.peek()),
                        expected: ")".to_owned(),
                        line: s.line,
                        col: s.col,
                    })
                }
            }
            Token::Ident(_)
            | Token::System
            | Token::Flow
            | Token::Store
            | Token::Graph
            | Token::Agent
            | Token::Test
            | Token::Nom
            | Token::Gate
            | Token::Pool
            | Token::View
            | Token::Node
            | Token::Edge
            | Token::Query
            | Token::Constraint
            | Token::Capability
            | Token::Supervise
            | Token::Receive
            | Token::State
            | Token::Schedule
            | Token::Every
            | Token::Need
            | Token::Require
            | Token::Effects
            | Token::Where
            | Token::Only
            | Token::Describe
            | Token::Given
            | Token::When
            | Token::Then
            | Token::And
            | Token::Contract
            | Token::Implement
            | Token::Branch => {
                let id = self.expect_ident()?;
                // Struct literal: `Name { field: expr, ... }`. We only commit
                // to this when the character after `{` (skipping whitespace /
                // comments / newlines) looks like `ident :`. This avoids
                // false positives in contexts where `Ident { ... }` happens
                // to precede a block (not common in Nom's grammar but kept
                // conservative for safety).
                if matches!(self.peek(), Token::LBrace)
                    && self.looks_like_struct_literal_body()
                {
                    return self.parse_struct_literal(id);
                }
                Ok(Expr::Ident(id))
            }
            other => {
                let s = self.peek_span();
                Err(ParseError::UnexpectedToken {
                    found: format!("{other:?}"),
                    expected: "expression".to_owned(),
                    line: s.line,
                    col: s.col,
                })
            }
        }
    }

    // ── effects statement ────────────────────────────────────────────────────

    fn parse_effects(&mut self) -> ParseResult<EffectsStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'effects'

        let modifier = match self.peek().clone() {
            Token::Only => {
                self.advance();
                Some(EffectModifier::Only)
            }
            Token::Ident(s) if s == "good" => {
                self.advance();
                Some(EffectModifier::Good)
            }
            Token::Ident(s) if s == "bad" => {
                self.advance();
                Some(EffectModifier::Bad)
            }
            _ => None,
        };

        let mut effects = Vec::new();
        if matches!(self.peek(), Token::LBracket) {
            self.advance(); // '['
            loop {
                match self.peek().clone() {
                    Token::RBracket | Token::Eof => {
                        self.advance();
                        break;
                    }
                    Token::Ident(s) => {
                        let span = self.peek_span();
                        self.advance();
                        effects.push(Identifier::new(s, span));
                    }
                    _ => {
                        // try to consume as ident
                        let id = self.expect_ident()?;
                        effects.push(id);
                    }
                }
            }
        }

        let end = self.peek_span();
        Ok(EffectsStmt {
            modifier,
            effects,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── flow statement ───────────────────────────────────────────────────────

    fn parse_flow_stmt(&mut self) -> ParseResult<FlowStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'flow'

        // Check for flow qualifier: flow::once, flow::stream, flow::scheduled
        let qualifier = if matches!(self.peek(), Token::ColCol) {
            self.advance(); // consume '::'
            match self.peek().clone() {
                Token::Ident(ref s) if s == "once" => {
                    self.advance();
                    FlowQualifier::Once
                }
                Token::Ident(ref s) if s == "stream" => {
                    self.advance();
                    FlowQualifier::Stream
                }
                Token::Ident(ref s) if s == "scheduled" => {
                    self.advance();
                    FlowQualifier::Scheduled
                }
                _ => {
                    let sp = self.peek_span();
                    return Err(ParseError::UnexpectedToken {
                        expected: "once, stream, or scheduled".into(),
                        found: format!("{:?}", self.peek()),
                        line: sp.line,
                        col: sp.col,
                    });
                }
            }
        } else {
            FlowQualifier::Once
        };

        let chain = self.parse_flow_chain()?;

        // Check for optional `onfail` clause (soft keyword)
        let on_fail = if matches!(self.peek(), Token::Ident(s) if s == "onfail") {
            self.advance(); // consume 'onfail'
            match self.peek().clone() {
                Token::Ident(ref s) if s == "abort" => {
                    self.advance();
                    OnFailStrategy::Abort
                }
                Token::Ident(ref s) if s == "restart_from" => {
                    self.advance();
                    let node_ref = self.parse_nom_ref()?;
                    OnFailStrategy::RestartFrom(node_ref.word)
                }
                Token::Ident(ref s) if s == "retry" => {
                    self.advance();
                    match self.peek().clone() {
                        Token::Integer(n) => {
                            self.advance();
                            OnFailStrategy::Retry(n as u32)
                        }
                        _ => {
                            let sp = self.peek_span();
                            return Err(ParseError::UnexpectedToken {
                                expected: "retry count (integer)".into(),
                                found: format!("{:?}", self.peek()),
                                line: sp.line,
                                col: sp.col,
                            });
                        }
                    }
                }
                Token::Ident(ref s) if s == "skip" => {
                    self.advance();
                    OnFailStrategy::Skip
                }
                Token::Ident(ref s) if s == "escalate" => {
                    self.advance();
                    OnFailStrategy::Escalate
                }
                _ => {
                    let sp = self.peek_span();
                    return Err(ParseError::UnexpectedToken {
                        expected: "abort, restart_from, retry, skip, or escalate".into(),
                        found: format!("{:?}", self.peek()),
                        line: sp.line,
                        col: sp.col,
                    });
                }
            }
        } else {
            OnFailStrategy::default()
        };

        let end = self.peek_span();
        Ok(FlowStmt {
            qualifier,
            chain,
            on_fail,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_flow_chain(&mut self) -> ParseResult<FlowChain> {
        let mut steps = Vec::new();
        steps.push(self.parse_flow_step()?);
        while matches!(self.peek(), Token::Arrow) {
            self.advance(); // '->'
            steps.push(self.parse_flow_step()?);
        }
        Ok(FlowChain { steps })
    }

    fn parse_flow_step(&mut self) -> ParseResult<FlowStep> {
        match self.peek().clone() {
            Token::LBrace => self.parse_branch_block().map(FlowStep::Branch),
            Token::StringLit(s) => {
                self.advance();
                Ok(FlowStep::Literal(Literal::Text(s)))
            }
            Token::Integer(n) => {
                self.advance();
                Ok(FlowStep::Literal(Literal::Integer(n)))
            }
            Token::Float(f) => {
                self.advance();
                Ok(FlowStep::Literal(Literal::Number(f)))
            }
            _ => {
                // NomRef or call
                let nom_ref = self.parse_nom_ref()?;
                if matches!(self.peek(), Token::LParen) {
                    // it's a call: callee(args)
                    self.advance(); // '('
                    let mut args = Vec::new();
                    while !matches!(self.peek(), Token::RParen | Token::Eof) {
                        args.push(self.parse_expr()?);
                        if matches!(self.peek(), Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let end = self.peek_span();
                    self.advance(); // ')'
                    Ok(FlowStep::Call(CallExpr {
                        callee: nom_ref.word,
                        args,
                        span: end,
                    }))
                } else {
                    Ok(FlowStep::Ref(nom_ref))
                }
            }
        }
    }

    fn parse_branch_block(&mut self) -> ParseResult<BranchBlock> {
        let start = self.peek_span();
        self.advance(); // '{'
        let mut arms = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek().clone() {
                Token::RBrace | Token::Eof => {
                    self.advance();
                    break;
                }
                Token::IfTrue => {
                    self.advance();
                    self.expect_arrow()?;
                    let chain = self.parse_flow_chain()?;
                    arms.push(BranchArm {
                        condition: BranchCondition::IfTrue,
                        label: Some("iftrue".to_owned()),
                        chain,
                    });
                }
                Token::IfFalse => {
                    self.advance();
                    self.expect_arrow()?;
                    let chain = self.parse_flow_chain()?;
                    arms.push(BranchArm {
                        condition: BranchCondition::IfFalse,
                        label: Some("iffalse".to_owned()),
                        chain,
                    });
                }
                _ => {
                    // named branch label
                    let label = self.expect_ident()?;
                    self.expect_arrow()?;
                    let chain = self.parse_flow_chain()?;
                    arms.push(BranchArm {
                        condition: BranchCondition::Named,
                        label: Some(label.name),
                        chain,
                    });
                }
            }
            // optional comma between arms
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            }
        }
        let end = self.peek_span();
        Ok(BranchBlock {
            arms,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn expect_arrow(&mut self) -> ParseResult<()> {
        if matches!(self.peek(), Token::Arrow) {
            self.advance();
            Ok(())
        } else {
            let s = self.peek_span();
            Err(ParseError::UnexpectedToken {
                found: format!("{:?}", self.peek()),
                expected: "->".to_owned(),
                line: s.line,
                col: s.col,
            })
        }
    }

    // ── describe statement ───────────────────────────────────────────────────

    fn parse_describe(&mut self) -> ParseResult<DescribeStmt> {
        let start = self.peek_span();
        self.advance(); // 'describe'
        self.skip_whitespace();
        match self.peek().clone() {
            Token::StringLit(s) => {
                let end = self.peek_span();
                self.advance();
                Ok(DescribeStmt {
                    text: s,
                    span: Span::new(start.start, end.end, start.line, start.col),
                })
            }
            other => {
                let s = self.peek_span();
                Err(ParseError::UnexpectedToken {
                    found: format!("{other:?}"),
                    expected: "string literal".to_owned(),
                    line: s.line,
                    col: s.col,
                })
            }
        }
    }

    // ── contract block ───────────────────────────────────────────────────────

    fn parse_contract(&mut self) -> ParseResult<ContractStmt> {
        let start = self.peek_span();
        self.advance(); // 'contract'
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut effects = Vec::new();
        let mut preconditions = Vec::new();
        let mut postconditions = Vec::new();

        loop {
            self.skip_whitespace();
            if self.at_statement_boundary() {
                break;
            }
            match self.peek().clone() {
                Token::Ident(kw) if kw == "input" || kw == "in" => {
                    self.advance();
                    self.parse_contract_params(&mut inputs)?;
                }
                Token::In => {
                    self.advance();
                    self.parse_contract_params(&mut inputs)?;
                }
                Token::Ident(kw) if kw == "output" || kw == "out" => {
                    self.advance();
                    self.parse_contract_params(&mut outputs)?;
                }
                Token::Ident(kw) if kw == "pre" => {
                    self.advance();
                    preconditions.push(self.parse_expr()?);
                }
                Token::Ident(kw) if kw == "post" => {
                    self.advance();
                    postconditions.push(self.parse_expr()?);
                }
                Token::Effects => {
                    self.advance();
                    if matches!(self.peek(), Token::LBracket) {
                        self.advance();
                        loop {
                            match self.peek().clone() {
                                Token::RBracket | Token::Eof => {
                                    self.advance();
                                    break;
                                }
                                Token::Ident(s) => {
                                    let span = self.peek_span();
                                    self.advance();
                                    effects.push(Identifier::new(s, span));
                                }
                                _ => break,
                            }
                        }
                    }
                }
                _ => break,
            }
        }

        let end = self.peek_span();
        Ok(ContractStmt {
            inputs,
            outputs,
            effects,
            preconditions,
            postconditions,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── implement block ──────────────────────────────────────────────────────

    fn parse_implement(&mut self) -> ParseResult<ImplementStmt> {
        let start = self.peek_span();
        self.advance(); // 'implement'
        let lang = self.expect_ident()?;
        let language = lang.name.clone();

        let mut code = String::new();
        if matches!(self.peek(), Token::LBrace) {
            let open_brace = self.peek_span();
            self.advance(); // '{'
            if let Token::RawCode(raw) = self.peek().clone() {
                self.advance();
                code = raw;
                if matches!(self.peek(), Token::RBrace) {
                    self.advance();
                }
            } else {
                code = self
                    .capture_raw_block(open_brace)
                    .unwrap_or_else(|| self.reconstruct_block_fallback());
            }
        }

        let end = self.peek_span();
        Ok(ImplementStmt {
            language,
            code: code.trim().to_owned(),
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── test statements ──────────────────────────────────────────────────────

    fn parse_given(&mut self) -> ParseResult<TestGivenStmt> {
        let start = self.peek_span();
        self.advance(); // 'given'
        let subject = self.expect_ident()?;
        let mut config = Vec::new();
        // consume key=value pairs on same "line" (before next newline/blank/keyword)
        while matches!(self.peek(), Token::Ident(_)) {
            let key = self.expect_ident()?;
            if matches!(self.peek(), Token::Eq) {
                self.advance();
                let val = self.parse_expr()?;
                config.push((key, val));
            } else {
                break;
            }
        }
        let end = self.peek_span();
        Ok(TestGivenStmt {
            subject,
            config,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_when(&mut self) -> ParseResult<TestWhenStmt> {
        let start = self.peek_span();
        self.advance(); // 'when'
        let action = self.expect_ident()?;
        let mut config = Vec::new();
        while matches!(self.peek(), Token::Ident(_)) {
            let key = self.expect_ident()?;
            if matches!(self.peek(), Token::Eq) {
                self.advance();
                let val = self.parse_expr()?;
                config.push((key, val));
            } else {
                break;
            }
        }
        let end = self.peek_span();
        Ok(TestWhenStmt {
            action,
            config,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_then(&mut self) -> ParseResult<TestThenStmt> {
        let start = self.peek_span();
        self.advance(); // 'then'
        let assertion = self.parse_expr()?;
        let end = self.peek_span();
        Ok(TestThenStmt {
            assertion,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_and(&mut self) -> ParseResult<TestAndStmt> {
        let start = self.peek_span();
        self.advance(); // 'and'
        let assertion = self.parse_expr()?;
        let end = self.peek_span();
        Ok(TestAndStmt {
            assertion,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }
    // ── graph node statement ──────────────────────────────────────────────

    /// node user(name text, age number)
    fn parse_graph_node(&mut self) -> ParseResult<GraphNodeStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'node'
        let name = self.expect_ident()?;
        let fields = self.parse_typed_param_list()?;
        let end = self.peek_span();
        Ok(GraphNodeStmt {
            name,
            fields,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── graph edge statement ──────────────────────────────────────────────

    /// edge follows(from user, to user, weight real)
    fn parse_graph_edge(&mut self) -> ParseResult<GraphEdgeStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'edge'
        let name = self.expect_ident()?;
        let all_fields = self.parse_typed_param_list()?;
        // Extract 'from' and 'to' typed params, rest are extra fields
        let mut from_type = None;
        let mut to_type = None;
        let mut fields = Vec::new();
        for f in all_fields {
            if f.name.name == "from" {
                from_type = f.typ;
            } else if f.name.name == "to" {
                to_type = f.typ;
            } else {
                fields.push(f);
            }
        }
        let default_span = Span::default();
        let end = self.peek_span();
        Ok(GraphEdgeStmt {
            name,
            from_type: from_type.unwrap_or_else(|| Identifier::new("unknown", default_span)),
            to_type: to_type.unwrap_or_else(|| Identifier::new("unknown", default_span)),
            fields,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── graph query statement ─────────────────────────────────────────────

    /// query friends_of(user) = user->follows->user
    fn parse_graph_query(&mut self) -> ParseResult<GraphQueryStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'query'
        let name = self.expect_ident()?;
        let params = self.parse_typed_param_list()?;
        // consume '='
        if matches!(self.peek(), Token::Eq) {
            self.advance();
        }
        let expr = self.parse_graph_query_expr()?;
        let end = self.peek_span();
        Ok(GraphQueryStmt {
            name,
            params,
            expr,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_graph_query_expr(&mut self) -> ParseResult<GraphQueryExpr> {
        let mut expr = self.parse_graph_query_primary()?;
        while matches!(self.peek(), Token::Arrow) {
            self.advance();
            let edge = self.parse_nom_ref()?;
            self.expect_arrow()?;
            let target = self.parse_graph_query_primary()?;
            let source_span = Self::graph_query_expr_span(&expr);
            let target_span = Self::graph_query_expr_span(&target);
            expr = GraphQueryExpr::Traverse(GraphTraverseExpr {
                source: Box::new(expr),
                edge,
                target: Box::new(target),
                span: Span::new(
                    source_span.start,
                    target_span.end,
                    source_span.line,
                    source_span.col,
                ),
            });
        }
        Ok(expr)
    }

    fn parse_graph_query_primary(&mut self) -> ParseResult<GraphQueryExpr> {
        match self.peek().clone() {
            Token::LBrace => {
                let block = self.parse_branch_block()?;
                self.graph_query_expr_from_branch_block(block)
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_graph_query_expr()?;
                if matches!(self.peek(), Token::RParen) {
                    self.advance();
                    Ok(expr)
                } else {
                    let span = self.peek_span();
                    Err(ParseError::UnexpectedToken {
                        found: format!("{:?}", self.peek()),
                        expected: ")".to_owned(),
                        line: span.line,
                        col: span.col,
                    })
                }
            }
            _ => {
                let nom_ref = self.parse_nom_ref()?;
                if nom_ref.variant.is_none() && matches!(self.peek(), Token::LParen) {
                    if let Some(op) = Self::graph_set_op_from_name(&nom_ref.word.name) {
                        return self.parse_graph_set_expr(op, nom_ref.word.span);
                    }
                }
                Ok(GraphQueryExpr::Ref(nom_ref))
            }
        }
    }

    fn parse_graph_set_expr(&mut self, op: GraphSetOp, start: Span) -> ParseResult<GraphQueryExpr> {
        self.advance(); // '('
        let mut operands = Vec::new();
        while !matches!(self.peek(), Token::RParen | Token::Eof) {
            operands.push(self.parse_graph_query_expr()?);
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        let end = self.peek_span();
        if matches!(self.peek(), Token::RParen) {
            self.advance();
        } else {
            return Err(ParseError::UnexpectedToken {
                found: format!("{:?}", self.peek()),
                expected: ")".to_owned(),
                line: end.line,
                col: end.col,
            });
        }
        let min_operands = if op == GraphSetOp::Difference { 2 } else { 2 };
        if operands.len() < min_operands {
            return Err(ParseError::UnexpectedToken {
                found: format!("{} operands", operands.len()),
                expected: format!("at least {min_operands} operands"),
                line: end.line,
                col: end.col,
            });
        }
        if op == GraphSetOp::Difference && operands.len() != 2 {
            return Err(ParseError::UnexpectedToken {
                found: format!("{} operands", operands.len()),
                expected: "exactly 2 operands for difference()".to_owned(),
                line: end.line,
                col: end.col,
            });
        }
        Ok(GraphQueryExpr::SetOp(GraphSetExpr {
            op,
            operands,
            span: Span::new(start.start, end.end, start.line, start.col),
        }))
    }

    fn graph_query_expr_from_branch_block(
        &self,
        block: BranchBlock,
    ) -> ParseResult<GraphQueryExpr> {
        let op = Self::legacy_graph_set_op(&block);
        let mut operands = Vec::new();
        for arm in block.arms {
            operands.push(self.graph_query_expr_from_flow_chain(arm.chain)?);
        }
        Ok(GraphQueryExpr::SetOp(GraphSetExpr {
            op,
            operands,
            span: block.span,
        }))
    }

    fn graph_query_expr_from_flow_chain(&self, chain: FlowChain) -> ParseResult<GraphQueryExpr> {
        let mut iter = chain.steps.into_iter();
        let first = iter.next().ok_or_else(|| ParseError::UnexpectedEof {
            expected: "graph query flow step".to_owned(),
        })?;
        let mut expr = self.graph_query_expr_from_flow_step(first)?;

        let remaining = iter.collect::<Vec<_>>();
        let mut index = 0usize;
        while index < remaining.len() {
            let step = remaining[index].clone();
            match step {
                FlowStep::Branch(block) => {
                    let branch_expr = self.graph_query_expr_from_branch_block(block)?;
                    let source_span = Self::graph_query_expr_span(&expr);
                    let target_span = Self::graph_query_expr_span(&branch_expr);
                    expr = GraphQueryExpr::SetOp(GraphSetExpr {
                        op: GraphSetOp::Union,
                        operands: vec![expr, branch_expr],
                        span: Span::new(
                            source_span.start,
                            target_span.end,
                            source_span.line,
                            source_span.col,
                        ),
                    });
                    index += 1;
                }
                FlowStep::Ref(edge) => {
                    let Some(target_step) = remaining.get(index + 1).cloned() else {
                        return Err(ParseError::UnexpectedEof {
                            expected: "graph query target after edge".to_owned(),
                        });
                    };
                    let target = self.graph_query_expr_from_flow_step(target_step)?;
                    let source_span = Self::graph_query_expr_span(&expr);
                    let target_span = Self::graph_query_expr_span(&target);
                    expr = GraphQueryExpr::Traverse(GraphTraverseExpr {
                        source: Box::new(expr),
                        edge,
                        target: Box::new(target),
                        span: Span::new(
                            source_span.start,
                            target_span.end,
                            source_span.line,
                            source_span.col,
                        ),
                    });
                    index += 2;
                }
                other => {
                    return Err(ParseError::UnexpectedToken {
                        found: format!("{other:?}"),
                        expected: "graph query ref or branch".to_owned(),
                        line: self.peek_span().line,
                        col: self.peek_span().col,
                    });
                }
            }
        }

        Ok(expr)
    }

    fn graph_query_expr_from_flow_step(&self, step: FlowStep) -> ParseResult<GraphQueryExpr> {
        match step {
            FlowStep::Ref(reference) => Ok(GraphQueryExpr::Ref(reference)),
            FlowStep::Branch(block) => self.graph_query_expr_from_branch_block(block),
            other => Err(ParseError::UnexpectedToken {
                found: format!("{other:?}"),
                expected: "graph query ref or branch".to_owned(),
                line: self.peek_span().line,
                col: self.peek_span().col,
            }),
        }
    }

    fn graph_set_op_from_name(name: &str) -> Option<GraphSetOp> {
        match name.to_ascii_lowercase().as_str() {
            "union" | "merge" => Some(GraphSetOp::Union),
            "intersect" | "intersection" | "common" => Some(GraphSetOp::Intersection),
            "difference" | "except" | "minus" => Some(GraphSetOp::Difference),
            _ => None,
        }
    }

    fn legacy_graph_set_op(block: &BranchBlock) -> GraphSetOp {
        let intersection_keywords = ["intersect", "intersection", "and", "all", "common"];
        if !block.arms.is_empty()
            && block.arms.iter().all(|arm| {
                arm.label.as_deref().is_some_and(|label| {
                    intersection_keywords.contains(&label.to_ascii_lowercase().as_str())
                })
            })
        {
            GraphSetOp::Intersection
        } else {
            GraphSetOp::Union
        }
    }

    fn graph_query_expr_span(expr: &GraphQueryExpr) -> Span {
        match expr {
            GraphQueryExpr::Ref(reference) => reference.span,
            GraphQueryExpr::Traverse(traverse) => traverse.span,
            GraphQueryExpr::SetOp(set) => set.span,
        }
    }

    // ── graph constraint statement ────────────────────────────────────────

    /// constraint no_self_follow = follows.from != follows.to
    fn parse_graph_constraint(&mut self) -> ParseResult<GraphConstraintStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'constraint'
        let name = self.expect_ident()?;
        // consume '='
        if matches!(self.peek(), Token::Eq) {
            self.advance();
        }
        let expr = self.parse_constraint_expr()?;
        let end = self.peek_span();
        Ok(GraphConstraintStmt {
            name,
            expr,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    /// Parse a constraint expression that may contain field access and comparison.
    /// e.g. follows.from != follows.to
    fn parse_constraint_expr(&mut self) -> ParseResult<Expr> {
        self.parse_expr()
    }

    // ── agent capability statement ────────────────────────────────────────

    /// capability [network observe]
    fn parse_agent_capability(&mut self) -> ParseResult<AgentCapabilityStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'capability'
        let mut capabilities = Vec::new();
        if matches!(self.peek(), Token::LBracket) {
            self.advance(); // '['
            loop {
                match self.peek().clone() {
                    Token::RBracket | Token::Eof => {
                        self.advance();
                        break;
                    }
                    _ => {
                        let id = self.expect_ident()?;
                        capabilities.push(id);
                    }
                }
            }
        }
        let end = self.peek_span();
        Ok(AgentCapabilityStmt {
            capabilities,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── agent supervise statement ─────────────────────────────────────────

    /// supervise restart_on_failure max_retries=3
    fn parse_agent_supervise(&mut self) -> ParseResult<AgentSuperviseStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'supervise'
        let strategy = self.expect_ident()?;
        let mut params = Vec::new();
        // consume key=value pairs
        while matches!(self.peek(), Token::Ident(_)) {
            let key = self.expect_ident()?;
            if matches!(self.peek(), Token::Eq) {
                self.advance();
                let val = self.parse_expr_primary()?;
                params.push((key, val));
            } else {
                break;
            }
        }
        let end = self.peek_span();
        Ok(AgentSuperviseStmt {
            strategy,
            params,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── agent receive statement ───────────────────────────────────────────

    /// receive message->classify->route
    fn parse_agent_receive(&mut self) -> ParseResult<AgentReceiveStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'receive'
        let chain = self.parse_flow_chain()?;
        let end = self.peek_span();
        Ok(AgentReceiveStmt {
            chain,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── agent state statement ─────────────────────────────────────────────

    /// state active
    fn parse_agent_state(&mut self) -> ParseResult<AgentStateStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'state'
        let state = self.expect_ident()?;
        let end = self.peek_span();
        Ok(AgentStateStmt {
            state,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── agent schedule statement ──────────────────────────────────────────

    /// schedule every 5m check_health
    fn parse_agent_schedule(&mut self) -> ParseResult<AgentScheduleStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'schedule'
        // expect 'every'
        if matches!(self.peek(), Token::Every) {
            self.advance();
        }
        // interval is a number+unit like "5m" — lexer produces this as Ident("5m")
        let interval = self.expect_ident()?;
        let action = self.parse_flow_chain()?;
        let end = self.peek_span();
        Ok(AgentScheduleStmt {
            interval: interval.name,
            action,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── shared helpers ──────────────────────────────────────────────────────

    /// Parse a parenthesized list of typed params: (name type, name type, ...)
    /// Returns empty vec if no opening paren.
    fn parse_typed_param_list(&mut self) -> ParseResult<Vec<TypedParam>> {
        let mut params = Vec::new();
        if !matches!(self.peek(), Token::LParen) {
            return Ok(params);
        }
        self.advance(); // '('
        loop {
            self.skip_whitespace();
            match self.peek().clone() {
                Token::RParen | Token::Eof => {
                    self.advance();
                    break;
                }
                _ => {
                    let name = self.expect_ident()?;
                    let span = name.span;
                    let typ = if matches!(self.peek(), Token::Ident(_))
                        || is_keyword_usable_as_type(self.peek())
                    {
                        Some(self.expect_ident()?)
                    } else {
                        None
                    };
                    params.push(TypedParam { name, typ, span });
                    if matches!(self.peek(), Token::Comma) {
                        self.advance();
                    }
                }
            }
        }
        Ok(params)
    }

    fn parse_contract_params(&mut self, target: &mut Vec<TypedParam>) -> ParseResult<()> {
        while !self.is_contract_clause_boundary() {
            target.push(self.parse_contract_param()?);
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            }
        }
        Ok(())
    }

    fn parse_contract_param(&mut self) -> ParseResult<TypedParam> {
        let name = self.expect_ident()?;
        let span = name.span;
        let typ = if matches!(self.peek(), Token::LParen) {
            self.advance();
            self.skip_whitespace();
            let parsed_type = if matches!(self.peek(), Token::RParen | Token::Eof) {
                None
            } else {
                Some(self.expect_ident()?)
            };

            let mut depth = 1usize;
            while self.pos < self.tokens.len() {
                let token = self.tokens[self.pos].token.clone();
                self.pos += 1;
                match token {
                    Token::LParen => depth += 1,
                    Token::RParen => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            parsed_type
        } else if matches!(self.peek(), Token::Ident(_)) || is_keyword_usable_as_type(self.peek()) {
            Some(self.expect_ident()?)
        } else {
            None
        };

        Ok(TypedParam { name, typ, span })
    }

    fn is_contract_clause_boundary(&self) -> bool {
        if self.at_statement_boundary() {
            return true;
        }

        match self.peek() {
            Token::Effects => true,
            Token::Ident(kw)
                if matches!(
                    kw.as_str(),
                    "input" | "in" | "output" | "out" | "pre" | "post"
                ) =>
            {
                true
            }
            _ => false,
        }
    }

    // ── Imperative parsing ────────────────────────────────────────────────────

    /// Parse a type annotation: `text`, `number`, `list[text]`, `fn(text) -> number`
    fn parse_type_expr(&mut self) -> ParseResult<TypeExpr> {
        match self.peek().clone() {
            Token::LParen => {
                self.advance(); // '('
                if matches!(self.peek(), Token::RParen) {
                    self.advance();
                    return Ok(TypeExpr::Unit);
                }
                let mut types = vec![self.parse_type_expr()?];
                while matches!(self.peek(), Token::Comma) {
                    self.advance();
                    types.push(self.parse_type_expr()?);
                }
                if matches!(self.peek(), Token::RParen) {
                    self.advance();
                }
                if types.len() == 1 {
                    Ok(types.remove(0))
                } else {
                    Ok(TypeExpr::Tuple(types))
                }
            }
            Token::Ampersand => {
                self.advance();
                let mutable = matches!(self.peek(), Token::Mut);
                if mutable { self.advance(); }
                let inner = self.parse_type_expr()?;
                Ok(TypeExpr::Ref { mutable, inner: Box::new(inner) })
            }
            Token::Fn => {
                self.advance(); // 'fn'
                if matches!(self.peek(), Token::LParen) { self.advance(); }
                let mut params = Vec::new();
                while !matches!(self.peek(), Token::RParen | Token::Eof) {
                    params.push(self.parse_type_expr()?);
                    if matches!(self.peek(), Token::Comma) { self.advance(); }
                }
                if matches!(self.peek(), Token::RParen) { self.advance(); }
                let ret = if matches!(self.peek(), Token::Arrow) {
                    self.advance();
                    Box::new(self.parse_type_expr()?)
                } else {
                    Box::new(TypeExpr::Unit)
                };
                Ok(TypeExpr::Function { params, ret })
            }
            _ => {
                let name = self.expect_ident()?;
                if matches!(self.peek(), Token::LBracket) {
                    self.advance(); // '['
                    let mut args = vec![self.parse_type_expr()?];
                    while matches!(self.peek(), Token::Comma) {
                        self.advance();
                        args.push(self.parse_type_expr()?);
                    }
                    if matches!(self.peek(), Token::RBracket) { self.advance(); }
                    Ok(TypeExpr::Generic(name, args))
                } else {
                    Ok(TypeExpr::Named(name))
                }
            }
        }
    }

    /// Parse a block: { stmt; stmt; expr }
    fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.peek_span();
        if !matches!(self.peek(), Token::LBrace) {
            return Err(ParseError::UnexpectedToken {
                found: format!("{:?}", self.peek()),
                expected: "'{'".to_owned(),
                line: start.line,
                col: start.col,
            });
        }
        self.advance(); // '{'
        self.skip_whitespace();

        let mut stmts = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            self.skip_whitespace();
            if matches!(self.peek(), Token::RBrace | Token::Eof) { break; }

            let bs = self.parse_block_stmt()?;
            stmts.push(bs);

            // consume optional semicolons and newlines
            while matches!(self.peek_raw(), Token::Semicolon | Token::Newline) {
                self.advance();
            }
        }

        let end = self.peek_span();
        if matches!(self.peek(), Token::RBrace) { self.advance(); }
        Ok(Block { stmts, span: Span::new(start.start, end.end, start.line, start.col) })
    }

    fn parse_block_stmt(&mut self) -> ParseResult<BlockStmt> {
        match self.peek().clone() {
            Token::Let => Ok(BlockStmt::Let(self.parse_let_stmt()?)),
            Token::If => Ok(BlockStmt::If(self.parse_if_expr()?)),
            Token::For => Ok(BlockStmt::For(self.parse_for_stmt()?)),
            Token::While => Ok(BlockStmt::While(self.parse_while_stmt()?)),
            Token::Match => Ok(BlockStmt::Match(self.parse_match_expr()?)),
            Token::Return => {
                self.advance(); // 'return'
                self.skip_whitespace();
                if matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Newline | Token::Eof) {
                    Ok(BlockStmt::Return(None))
                } else {
                    Ok(BlockStmt::Return(Some(self.parse_expr()?)))
                }
            }
            Token::Break => { self.advance(); Ok(BlockStmt::Break) }
            Token::Continue => { self.advance(); Ok(BlockStmt::Continue) }
            _ => {
                let expr = self.parse_expr()?;
                // Check for assignment: expr = value
                if matches!(self.peek(), Token::Eq) {
                    self.advance();
                    let value = self.parse_expr()?;
                    let span = Span::default();
                    Ok(BlockStmt::Assign(AssignStmt { target: expr, value, span }))
                } else {
                    Ok(BlockStmt::Expr(expr))
                }
            }
        }
    }

    /// let x: type = value
    fn parse_let_stmt(&mut self) -> ParseResult<LetStmt> {
        let start = self.peek_span();
        self.advance(); // 'let'
        let mutable = matches!(self.peek(), Token::Mut);
        if mutable { self.advance(); }
        let name = self.expect_ident()?;
        let type_ann = if matches!(self.peek(), Token::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        let value = if matches!(self.peek(), Token::Eq) {
            self.advance();
            self.parse_expr()?
        } else {
            Expr::Literal(Literal::None)
        };
        let end = self.peek_span();
        Ok(LetStmt {
            name,
            mutable,
            type_ann,
            value,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    /// if condition { body } else if ... { ... } else { ... }
    fn parse_if_expr(&mut self) -> ParseResult<IfExpr> {
        let start = self.peek_span();
        self.advance(); // 'if'
        let condition = Box::new(self.parse_expr()?);
        let then_body = self.parse_block()?;

        let mut else_ifs = Vec::new();
        let mut else_body = None;

        while matches!(self.peek(), Token::Else) {
            self.advance(); // 'else'
            if matches!(self.peek(), Token::If) {
                self.advance(); // 'if'
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                else_ifs.push((cond, body));
            } else {
                else_body = Some(self.parse_block()?);
                break;
            }
        }

        let end = self.peek_span();
        Ok(IfExpr {
            condition,
            then_body,
            else_ifs,
            else_body,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    /// for item in iterable { body }
    fn parse_for_stmt(&mut self) -> ParseResult<ForStmt> {
        let start = self.peek_span();
        self.advance(); // 'for'
        let binding = self.expect_ident()?;
        // expect 'in'
        if matches!(self.peek(), Token::In) {
            self.advance();
        }
        let iterable = self.parse_expr()?;
        let body = self.parse_block()?;
        let end = self.peek_span();
        Ok(ForStmt {
            binding,
            iterable,
            body,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    /// while condition { body }
    fn parse_while_stmt(&mut self) -> ParseResult<WhileStmt> {
        let start = self.peek_span();
        self.advance(); // 'while'
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        let end = self.peek_span();
        Ok(WhileStmt {
            condition,
            body,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    /// match expr { pattern => body, ... }
    fn parse_match_expr(&mut self) -> ParseResult<MatchExpr> {
        let start = self.peek_span();
        self.advance(); // 'match'
        let subject = Box::new(self.parse_expr()?);

        if !matches!(self.peek(), Token::LBrace) {
            return Err(ParseError::UnexpectedToken {
                found: format!("{:?}", self.peek()),
                expected: "'{'".to_owned(),
                line: start.line,
                col: start.col,
            });
        }
        self.advance(); // '{'
        self.skip_whitespace();

        let mut arms = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            self.skip_whitespace();
            if matches!(self.peek(), Token::RBrace) { break; }
            let pattern = self.parse_pattern()?;
            if matches!(self.peek(), Token::FatArrow) { self.advance(); }
            self.skip_whitespace();
            // Arm body: either a `{ ... }` block (statement list) or a single
            // expression (the common `pat => expr,` ergonomic form used in
            // the self-hosting lexer). Single-expression arms are wrapped in
            // a one-statement synthetic block.
            let body = if matches!(self.peek(), Token::LBrace) {
                self.parse_block()?
            } else {
                let expr_start = self.peek_span();
                let expr = self.parse_expr()?;
                Block {
                    stmts: vec![BlockStmt::Expr(expr)],
                    span: expr_start,
                }
            };
            arms.push(MatchArm { pattern, body });
            // skip commas and whitespace between arms
            while matches!(self.peek_raw(), Token::Comma | Token::Newline | Token::Semicolon) {
                self.advance();
            }
        }

        let end = self.peek_span();
        if matches!(self.peek(), Token::RBrace) { self.advance(); }
        Ok(MatchExpr { subject, arms, span: Span::new(start.start, end.end, start.line, start.col) })
    }

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        match self.peek().clone() {
            Token::Ident(ref s) if s == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::Integer(i) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Integer(i)))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Number(f)))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Text(s)))
            }
            Token::Bool(b) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(b)))
            }
            Token::NoneLit => {
                self.advance();
                Ok(Pattern::Literal(Literal::None))
            }
            _ => {
                let mut name = self.expect_ident()?;
                // Qualified variant pattern: `Enum::Variant` — merge the two
                // idents into one (`"Enum::Variant"`) before continuing so
                // the downstream match-compiler sees the owning enum.
                while matches!(self.peek(), Token::ColCol) {
                    self.advance(); // consume '::'
                    let next = self.expect_ident()?;
                    let merged = format!("{}::{}", name.name, next.name);
                    name = Identifier::new(merged, name.span);
                }
                if matches!(self.peek(), Token::LParen) {
                    self.advance(); // '('
                    let mut sub = Vec::new();
                    while !matches!(self.peek(), Token::RParen | Token::Eof) {
                        sub.push(self.parse_pattern()?);
                        if matches!(self.peek(), Token::Comma) { self.advance(); }
                    }
                    if matches!(self.peek(), Token::RParen) { self.advance(); }
                    Ok(Pattern::Variant(name, sub))
                } else if name.name.contains("::") {
                    // `E::B` with no parens — still a variant (unit variant).
                    Ok(Pattern::Variant(name, Vec::new()))
                } else {
                    Ok(Pattern::Binding(name))
                }
            }
        }
    }

    /// return [expr]
    fn parse_return_stmt(&mut self) -> ParseResult<Statement> {
        self.advance(); // 'return'
        self.skip_whitespace();
        if matches!(self.peek(), Token::Semicolon | Token::Newline | Token::BlankLine | Token::Eof | Token::RBrace) {
            Ok(Statement::Return(None))
        } else {
            Ok(Statement::Return(Some(self.parse_expr()?)))
        }
    }

    /// fn name(param: type, ...) -> return_type { body }
    fn parse_fn_def(&mut self, is_pub: bool) -> ParseResult<FnDef> {
        let start = self.peek_span();
        let is_async = matches!(self.peek(), Token::Async);
        if is_async { self.advance(); }

        self.advance(); // 'fn'
        let name = self.expect_ident()?;

        // Parse params
        let mut params = Vec::new();
        if matches!(self.peek(), Token::LParen) {
            self.advance(); // '('
            while !matches!(self.peek(), Token::RParen | Token::Eof) {
                // Handle `self` parameter (no type annotation required)
                if matches!(self.peek(), Token::Self_) {
                    let span = self.peek_span();
                    self.advance(); // consume 'self'
                    let pname = Identifier::new("self", span);
                    let ptype = TypeExpr::Named(Identifier::new("Self", span));
                    params.push(FnParam { name: pname, type_ann: ptype });
                    if matches!(self.peek(), Token::Comma) { self.advance(); }
                    continue;
                }
                let pname = self.expect_ident()?;
                if matches!(self.peek(), Token::Colon) { self.advance(); }
                let ptype = self.parse_type_expr()?;
                params.push(FnParam { name: pname, type_ann: ptype });
                if matches!(self.peek(), Token::Comma) { self.advance(); }
            }
            if matches!(self.peek(), Token::RParen) { self.advance(); }
        }

        // Return type
        let return_type = if matches!(self.peek(), Token::Arrow) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        // Body: optional (trait method signatures may have no body)
        let body = if matches!(self.peek(), Token::LBrace) {
            self.parse_block()?
        } else {
            Block { stmts: vec![], span: Span::default() }
        };

        let end = self.peek_span();
        Ok(FnDef {
            name,
            params,
            return_type,
            body,
            is_async,
            is_pub,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    /// struct Name { field: type, ... }
    fn parse_struct_def(&mut self, is_pub: bool) -> ParseResult<StructDef> {
        let start = self.peek_span();
        self.advance(); // 'struct'
        let name = self.expect_ident()?;

        let mut fields = Vec::new();
        if matches!(self.peek(), Token::LBrace) {
            self.advance(); // '{'
            self.skip_whitespace();
            while !matches!(self.peek(), Token::RBrace | Token::Eof) {
                self.skip_whitespace();
                if matches!(self.peek(), Token::RBrace) { break; }
                let field_pub = matches!(self.peek(), Token::Pub);
                if field_pub { self.advance(); }
                let fname = self.expect_ident()?;
                if matches!(self.peek(), Token::Colon) { self.advance(); }
                let ftype = self.parse_type_expr()?;
                fields.push(StructField { name: fname, type_ann: ftype, is_pub: field_pub });
                while matches!(self.peek_raw(), Token::Comma | Token::Newline | Token::Semicolon) {
                    self.advance();
                }
            }
            if matches!(self.peek(), Token::RBrace) { self.advance(); }
        }

        let end = self.peek_span();
        Ok(StructDef {
            name,
            fields,
            is_pub,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    /// enum Name { Variant1, Variant2(type), ... }
    fn parse_enum_def(&mut self, is_pub: bool) -> ParseResult<EnumDef> {
        let start = self.peek_span();
        self.advance(); // 'enum'
        let name = self.expect_ident()?;

        let mut variants = Vec::new();
        if matches!(self.peek(), Token::LBrace) {
            self.advance(); // '{'
            self.skip_whitespace();
            while !matches!(self.peek(), Token::RBrace | Token::Eof) {
                self.skip_whitespace();
                if matches!(self.peek(), Token::RBrace) { break; }
                let vname = self.expect_ident()?;
                let mut vfields = Vec::new();
                if matches!(self.peek(), Token::LParen) {
                    self.advance(); // '('
                    while !matches!(self.peek(), Token::RParen | Token::Eof) {
                        vfields.push(self.parse_type_expr()?);
                        if matches!(self.peek(), Token::Comma) { self.advance(); }
                    }
                    if matches!(self.peek(), Token::RParen) { self.advance(); }
                }
                variants.push(EnumVariant { name: vname, fields: vfields });
                while matches!(self.peek_raw(), Token::Comma | Token::Newline | Token::Semicolon) {
                    self.advance();
                }
            }
            if matches!(self.peek(), Token::RBrace) { self.advance(); }
        }

        let end = self.peek_span();
        Ok(EnumDef {
            name,
            variants,
            is_pub,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    // ── Trait / impl ────────────────────────────────────────────────────────

    /// trait Name { fn method(self) -> type ... }
    fn parse_trait_def(&mut self, is_pub: bool) -> ParseResult<TraitDef> {
        let start = self.peek_span();
        self.advance(); // consume 'trait'
        let name = self.expect_ident()?;

        let mut methods = Vec::new();
        if matches!(self.peek(), Token::LBrace) {
            self.advance(); // '{'
            self.skip_whitespace();
            while !matches!(self.peek(), Token::RBrace | Token::Eof) {
                self.skip_whitespace();
                if matches!(self.peek(), Token::RBrace) { break; }
                // Parse method signatures: fn name(params) -> type { body }?
                // Methods may or may not have bodies (abstract vs default)
                let method_pub = matches!(self.peek(), Token::Pub);
                if method_pub { self.advance(); }
                if matches!(self.peek(), Token::Fn) {
                    methods.push(self.parse_fn_def(method_pub)?);
                } else {
                    // Skip unexpected tokens inside trait
                    self.advance();
                }
                // consume optional separators
                while matches!(self.peek_raw(), Token::Comma | Token::Newline | Token::Semicolon) {
                    self.advance();
                }
            }
            if matches!(self.peek(), Token::RBrace) { self.advance(); }
        }

        let end = self.peek_span();
        Ok(TraitDef {
            name,
            methods,
            is_pub,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    /// impl TraitName for TypeName { methods } or impl TypeName { methods }
    fn parse_impl_block(&mut self) -> ParseResult<ImplBlock> {
        let start = self.peek_span();
        self.advance(); // consume 'impl'
        let first_ident = self.expect_ident()?;

        // Check if this is `impl Trait for Type` or `impl Type`
        let (trait_name, target_type) = if matches!(self.peek(), Token::For) {
            self.advance(); // consume 'for'
            let target = self.expect_ident()?;
            (Some(first_ident), target)
        } else {
            (None, first_ident)
        };

        let mut methods = Vec::new();
        if matches!(self.peek(), Token::LBrace) {
            self.advance(); // '{'
            self.skip_whitespace();
            while !matches!(self.peek(), Token::RBrace | Token::Eof) {
                self.skip_whitespace();
                if matches!(self.peek(), Token::RBrace) { break; }
                let method_pub = matches!(self.peek(), Token::Pub);
                if method_pub { self.advance(); }
                if matches!(self.peek(), Token::Fn) {
                    methods.push(self.parse_fn_def(method_pub)?);
                } else {
                    self.advance();
                }
                while matches!(self.peek_raw(), Token::Comma | Token::Newline | Token::Semicolon) {
                    self.advance();
                }
            }
            if matches!(self.peek(), Token::RBrace) { self.advance(); }
        }

        let end = self.peek_span();
        Ok(ImplBlock {
            trait_name,
            target_type,
            methods,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn capture_raw_block(&mut self, open_brace: Span) -> Option<String> {
        let source = self.source.as_ref()?;
        let body_start = open_brace.end;
        let mut depth = 1usize;

        while self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            match token.token {
                Token::LBrace => depth += 1,
                Token::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return source
                            .get(body_start..token.span.start)
                            .map(ToOwned::to_owned);
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn reconstruct_block_fallback(&mut self) -> String {
        let mut code = String::new();
        let mut depth = 1usize;
        while self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].token.clone();
            self.pos += 1;
            match tok {
                Token::LBrace => {
                    depth += 1;
                    code.push('{');
                }
                Token::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    code.push('}');
                }
                Token::Newline | Token::BlankLine => code.push('\n'),
                Token::Comment(s) => {
                    code.push('#');
                    code.push_str(&s);
                }
                Token::Ident(s) => {
                    code.push_str(&s);
                    code.push(' ');
                }
                Token::StringLit(s) => {
                    code.push('"');
                    code.push_str(&s);
                    code.push('"');
                }
                Token::Bool(true) => code.push_str("true"),
                Token::Bool(false) => code.push_str("false"),
                Token::NoneLit => code.push_str("none"),
                Token::Or => code.push_str("or"),
                Token::Integer(n) => code.push_str(&n.to_string()),
                Token::Float(f) => code.push_str(&f.to_string()),
                Token::Arrow => code.push_str("->"),
                Token::ColCol => code.push_str("::"),
                Token::Plus => code.push('+'),
                Token::Minus => code.push('-'),
                Token::Star => code.push('*'),
                Token::Slash => code.push('/'),
                Token::Dot => code.push('.'),
                Token::DotDot => code.push_str(".."),
                Token::Eq => code.push('='),
                Token::Neq => code.push_str("!="),
                Token::Gt => code.push('>'),
                Token::Lt => code.push('<'),
                Token::Gte => code.push_str(">="),
                Token::Lte => code.push_str("<="),
                Token::Comma => code.push(','),
                Token::LParen => code.push('('),
                Token::RParen => code.push(')'),
                Token::LBracket => code.push('['),
                Token::RBracket => code.push(']'),
                other if is_classifier(&other) || is_statement_keyword(&other) => {
                    code.push_str(statement_or_classifier_str(&other));
                    code.push(' ');
                }
                _ => {}
            }
        }
        code.trim().to_owned()
    }

    // ── Module system ───────────────────────────────────────────────────────

    /// Parse a `use` statement: `use path::item`, `use path::{a, b}`, `use path::*`
    fn parse_use_stmt(&mut self) -> ParseResult<UseStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'use'

        // Parse the first identifier segment
        let mut path: Vec<Identifier> = vec![self.expect_ident()?];

        // Parse :: separated path segments
        // We keep consuming ident :: pairs. The last segment determines the import kind.
        loop {
            if !matches!(self.peek(), Token::ColCol) {
                break;
            }
            self.advance(); // consume '::'

            // Check for glob: use math::*
            if matches!(self.peek(), Token::Star) {
                let end_span = self.peek_span();
                self.advance(); // consume '*'
                return Ok(UseStmt {
                    path,
                    imports: UseImport::Glob,
                    span: Span::new(start.start, end_span.end, start.line, start.col),
                });
            }

            // Check for multi-import: use math::{sin, cos}
            if matches!(self.peek(), Token::LBrace) {
                self.advance(); // consume '{'
                let mut items = Vec::new();
                loop {
                    self.skip_whitespace();
                    if matches!(self.peek(), Token::RBrace) {
                        break;
                    }
                    items.push(self.expect_ident()?);
                    self.skip_whitespace();
                    if matches!(self.peek(), Token::Comma) {
                        self.advance(); // consume ','
                    }
                }
                let end_span = self.peek_span();
                self.advance(); // consume '}'
                return Ok(UseStmt {
                    path,
                    imports: UseImport::Multiple(items),
                    span: Span::new(start.start, end_span.end, start.line, start.col),
                });
            }

            // Regular identifier — could be another path segment or the final import
            let ident = self.expect_ident()?;
            path.push(ident);
        }

        // No :: after last segment, so the last path element is the imported item
        // e.g., `use math::sin` → path=["math", "sin"], import=Single("sin")
        if path.len() < 2 {
            // `use math` with no :: — treat the single ident as both path and import
            let item = path[0].clone();
            let end_span = item.span;
            return Ok(UseStmt {
                path: vec![],
                imports: UseImport::Single(item),
                span: Span::new(start.start, end_span.end, start.line, start.col),
            });
        }

        let item = path.pop().unwrap();
        let end_span = item.span;
        Ok(UseStmt {
            path,
            imports: UseImport::Single(item),
            span: Span::new(start.start, end_span.end, start.line, start.col),
        })
    }

    /// Parse a `mod` statement: `mod utils`
    fn parse_mod_stmt(&mut self) -> ParseResult<ModStmt> {
        let start = self.peek_span();
        self.advance(); // consume 'mod'
        let name = self.expect_ident()?;
        let end_span = name.span;
        Ok(ModStmt {
            name,
            span: Span::new(start.start, end_span.end, start.line, start.col),
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Check if a token is a statement keyword that can also appear as an identifier.
fn is_statement_keyword(t: &Token) -> bool {
    matches!(
        t,
        Token::Node
            | Token::Edge
            | Token::Query
            | Token::Constraint
            | Token::Capability
            | Token::Supervise
            | Token::Receive
            | Token::State
            | Token::Schedule
            | Token::Every
            | Token::Need
            | Token::Require
            | Token::Effects
            | Token::Where
            | Token::Only
            | Token::Describe
            | Token::Given
            | Token::When
            | Token::Then
            | Token::And
            | Token::Contract
            | Token::Implement
            | Token::Branch
    )
}

fn statement_keyword_str(t: &Token) -> &'static str {
    match t {
        Token::Node => "node",
        Token::Edge => "edge",
        Token::Query => "query",
        Token::Constraint => "constraint",
        Token::Capability => "capability",
        Token::Supervise => "supervise",
        Token::Receive => "receive",
        Token::State => "state",
        Token::Schedule => "schedule",
        Token::Every => "every",
        Token::Need => "need",
        Token::Require => "require",
        Token::Effects => "effects",
        Token::Where => "where",
        Token::Only => "only",
        Token::Describe => "describe",
        Token::Given => "given",
        Token::When => "when",
        Token::Then => "then",
        Token::And => "and",
        Token::Or => "or",
        Token::Contract => "contract",
        Token::Implement => "implement",
        Token::Branch => "branch",
        _ => "unknown",
    }
}

/// Check if a keyword token can be used as a type name in typed param lists.
fn is_keyword_usable_as_type(t: &Token) -> bool {
    // Any identifier-like token can serve as a type
    matches!(t, Token::Ident(_)) || is_statement_keyword(t)
}

fn is_classifier(t: &Token) -> bool {
    matches!(
        t,
        Token::System
            | Token::Flow
            | Token::Store
            | Token::Graph
            | Token::Agent
            | Token::Test
            | Token::Nom
            | Token::Gate
            | Token::Pool
            | Token::View
    )
}

fn classifier_str(t: &Token) -> &'static str {
    match t {
        Token::System => "system",
        Token::Flow => "flow",
        Token::Store => "store",
        Token::Graph => "graph",
        Token::Agent => "agent",
        Token::Test => "test",
        Token::Nom => "nom",
        Token::Gate => "gate",
        Token::Pool => "pool",
        Token::View => "view",
        _ => "unknown",
    }
}

fn statement_or_classifier_str(t: &Token) -> &'static str {
    if is_classifier(t) {
        classifier_str(t)
    } else {
        statement_keyword_str(t)
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a slice of [`SpannedToken`]s into a [`SourceFile`].
pub fn parse(tokens: Vec<SpannedToken>) -> Result<SourceFile, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.parse_source_file()
}

/// Convenience: lex + parse a source string in one step.
pub fn parse_source(source: &str) -> Result<SourceFile, Box<dyn std::error::Error>> {
    let tokens = nom_lexer::tokenize(source)?;
    let mut parser = Parser::with_source(tokens, source.to_owned());
    Ok(parser.parse_source_file()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> SourceFile {
        parse_source(src).expect("parse failed")
    }

    #[test]
    fn parse_simple_flow() {
        let src = "flow register\n  need hash::argon2 where security>0.9\n";
        let sf = parse_ok(src);
        assert_eq!(sf.declarations.len(), 1);
        let decl = &sf.declarations[0];
        assert_eq!(decl.classifier, Classifier::Flow);
        assert_eq!(decl.name.name, "register");
    }

    #[test]
    fn parse_two_declarations() {
        let src =
            "flow register\n  describe \"registration\"\n\nflow login\n  describe \"login\"\n";
        let sf = parse_ok(src);
        assert_eq!(sf.declarations.len(), 2);
        assert_eq!(sf.declarations[0].name.name, "register");
        assert_eq!(sf.declarations[1].name.name, "login");
    }

    #[test]
    fn parse_effects_only() {
        let src = "flow upload\n  effects only [network database]\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        assert!(matches!(stmt, Statement::Effects(_)));
    }

    #[test]
    fn parse_describe_string() {
        let src = "nom hash\n  describe \"hashing function\"\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Describe(d) => assert_eq!(d.text, "hashing function"),
            _ => panic!("expected describe"),
        }
    }

    #[test]
    fn parse_contract_doc_style_and_rich_preconditions() {
        let src = "nom scorer\n  contract\n  in document(text) query(text)\n  out score(real range 0.0 to 1.0)\n  effects [cpu]\n  pre document.length > 0 and query.length > 0\n  post deterministic\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::Contract(contract) => {
                assert_eq!(contract.inputs.len(), 2);
                assert_eq!(contract.inputs[0].name.name, "document");
                assert_eq!(
                    contract.inputs[0].typ.as_ref().map(|t| t.name.as_str()),
                    Some("text")
                );
                assert_eq!(contract.inputs[1].name.name, "query");
                assert_eq!(contract.outputs.len(), 1);
                assert_eq!(contract.outputs[0].name.name, "score");
                assert_eq!(
                    contract.outputs[0].typ.as_ref().map(|t| t.name.as_str()),
                    Some("real")
                );
                assert_eq!(contract.effects.len(), 1);
                assert_eq!(contract.effects[0].name, "cpu");
                assert_eq!(contract.preconditions.len(), 1);
                assert_eq!(contract.postconditions.len(), 1);
            }
            _ => panic!("expected contract"),
        }
    }

    #[test]
    fn parse_implement_preserves_raw_code() {
        let src = "nom scorer\n  implement rust {\n    fn score(doc: &str) -> f32 {\n        if doc.is_empty() {\n            return 0.0;\n        }\n        let total = (1 + 2) * 3;\n        total as f32\n    }\n  }\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::Implement(implement) => {
                assert!(implement.code.contains("fn score(doc: &str) -> f32 {"));
                assert!(implement.code.contains("if doc.is_empty() {"));
                assert!(implement.code.contains("return 0.0;"));
                assert!(implement.code.contains("let total = (1 + 2) * 3;"));
            }
            _ => panic!("expected implement"),
        }
    }

    #[test]
    fn parse_graph_constraint_with_field_access() {
        let src = "graph social\n  constraint no_self_follow = follows.from != follows.to\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::GraphConstraint(graph_constraint) => match &graph_constraint.expr {
                Expr::BinaryOp(left, nom_ast::BinOp::Neq, right) => {
                    assert!(matches!(&**left, Expr::FieldAccess(_, field) if field.name == "from"));
                    assert!(matches!(&**right, Expr::FieldAccess(_, field) if field.name == "to"));
                }
                other => panic!("expected != field access expression, got {other:?}"),
            },
            _ => panic!("expected graph constraint"),
        }
    }

    #[test]
    fn parse_agent_schedule_with_expression_args() {
        let src =
            "agent monitor\n  schedule every 5m check_health(status(code + 1), retries * 2)\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::AgentSchedule(schedule) => {
                assert_eq!(schedule.interval, "5m");
                assert_eq!(schedule.action.steps.len(), 1);
                assert!(matches!(&schedule.action.steps[0], FlowStep::Call(_)));
            }
            _ => panic!("expected agent schedule"),
        }
    }

    #[test]
    fn parse_test_assertion_uses_precedence() {
        let src = "test auth\n  then latency + overhead * 2 < 50 or cached = true\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::Then(then_stmt) => {
                assert!(matches!(
                    then_stmt.assertion,
                    Expr::BinaryOp(_, nom_ast::BinOp::Or, _)
                ));
            }
            _ => panic!("expected then"),
        }
    }

    #[test]
    fn parse_graph_query_legacy_branch_becomes_set_expr() {
        let src = "graph social\n  query common_friends(a user, b user) = { intersect->a->follows->user, intersect->b->follows->user }\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::GraphQuery(query) => match &query.expr {
                GraphQueryExpr::SetOp(set) => {
                    assert_eq!(set.op, GraphSetOp::Intersection);
                    assert_eq!(set.operands.len(), 2);
                }
                other => panic!("expected set expr, got {other:?}"),
            },
            _ => panic!("expected graph query"),
        }
    }

    #[test]
    fn parse_graph_query_first_class_algebra_with_suffix() {
        let src = "graph social\n  query visible_posts(a user, b user) = difference(union(a->follows->user, b->follows->user), a)->authored->post\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::GraphQuery(query) => match &query.expr {
                GraphQueryExpr::Traverse(GraphTraverseExpr {
                    source,
                    edge,
                    target,
                    ..
                }) => {
                    assert_eq!(edge.word.name, "authored");
                    assert!(
                        matches!(&**target, GraphQueryExpr::Ref(reference) if reference.word.name == "post")
                    );
                    assert!(
                        matches!(&**source, GraphQueryExpr::SetOp(set) if set.op == GraphSetOp::Difference)
                    );
                }
                other => panic!("expected traverse expr, got {other:?}"),
            },
            _ => panic!("expected graph query"),
        }
    }

    #[test]
    fn parse_let_and_fn_in_nom_declaration() {
        let src = "nom math\n  fn add(a: number, b: number) -> number {\n    let result: number = a + b\n    return result\n  }\n";
        let sf = parse_ok(src);
        assert_eq!(sf.declarations.len(), 1);
        match &sf.declarations[0].statements[0] {
            Statement::FnDef(fndef) => {
                assert_eq!(fndef.name.name, "add");
                assert_eq!(fndef.params.len(), 2);
                assert_eq!(fndef.params[0].name.name, "a");
                assert!(fndef.return_type.is_some());
                assert_eq!(fndef.body.stmts.len(), 2);
            }
            other => panic!("expected FnDef, got {other:?}"),
        }
    }

    #[test]
    fn parse_struct_def() {
        let src = "nom types\n  struct Point {\n    x: number,\n    y: number\n  }\n";
        let sf = parse_ok(src);
        assert_eq!(sf.declarations[0].statements.len(), 1);
        match &sf.declarations[0].statements[0] {
            Statement::StructDef(s) => {
                assert_eq!(s.name.name, "Point");
                assert_eq!(s.fields.len(), 2);
                assert_eq!(s.fields[0].name.name, "x");
                assert_eq!(s.fields[1].name.name, "y");
            }
            other => panic!("expected StructDef, got {other:?}"),
        }
    }

    #[test]
    fn parse_enum_def() {
        let src = "nom types\n  enum Color {\n    Red,\n    Green,\n    Blue\n  }\n";
        let sf = parse_ok(src);
        assert_eq!(sf.declarations[0].statements.len(), 1);
        match &sf.declarations[0].statements[0] {
            Statement::EnumDef(e) => {
                assert_eq!(e.name.name, "Color");
                assert_eq!(e.variants.len(), 3);
            }
            other => panic!("expected EnumDef, got {other:?}"),
        }
    }

    #[test]
    fn parse_if_simple() {
        let src = "nom control\n  if x > 0 {\n    let y = x\n  }\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::If(if_expr) => {
                assert_eq!(if_expr.then_body.stmts.len(), 1);
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn parse_natural_language_aliases() {
        // "define" → fn, "kind" → struct, "choice" → enum, "take" → let, "give" → return
        let src = "nom math\n  kind Point {\n    x: number,\n    y: number\n  }\n  choice Color {\n    Red,\n    Blue\n  }\n  define add(a: number, b: number) -> number {\n    take result = a + b\n    give result\n  }\n";
        let sf = parse_ok(src);
        let stmts = &sf.declarations[0].statements;
        assert_eq!(stmts.len(), 3, "expected 3 statements: kind, choice, define");
        match &stmts[0] {
            Statement::StructDef(s) => assert_eq!(s.name.name, "Point"),
            other => panic!("expected StructDef from 'kind', got {other:?}"),
        }
        match &stmts[1] {
            Statement::EnumDef(e) => {
                assert_eq!(e.name.name, "Color");
                assert_eq!(e.variants.len(), 2);
            }
            other => panic!("expected EnumDef from 'choice', got {other:?}"),
        }
        match &stmts[2] {
            Statement::FnDef(f) => {
                assert_eq!(f.name.name, "add");
                assert_eq!(f.params.len(), 2);
                // Check the body has "take" (let) and "give" (return)
                assert_eq!(f.body.stmts.len(), 2);
                assert!(matches!(f.body.stmts[0], BlockStmt::Let(_)));
                assert!(matches!(f.body.stmts[1], BlockStmt::Return(Some(_))));
            }
            other => panic!("expected FnDef from 'define', got {other:?}"),
        }
    }

    #[test]
    fn parse_method_calls_and_array_access() {
        let src = "nom expr_test\n  fn test_expr() -> bool {\n    let items = [1, 2, 3]\n    let len = items.len()\n    let first = items[0]\n    let valid = !false\n    give len > 0\n  }\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::FnDef(f) => {
                assert_eq!(f.name.name, "test_expr");
                assert_eq!(f.body.stmts.len(), 5);
                // items = [1, 2, 3] — array literal
                if let BlockStmt::Let(let_stmt) = &f.body.stmts[0] {
                    assert!(matches!(&let_stmt.value, Expr::Array(_)));
                } else {
                    panic!("expected let with array");
                }
                // len = items.len() — method call
                if let BlockStmt::Let(let_stmt) = &f.body.stmts[1] {
                    assert!(matches!(&let_stmt.value, Expr::MethodCall(_, _, _)));
                } else {
                    panic!("expected let with method call");
                }
                // first = items[0] — index access
                if let BlockStmt::Let(let_stmt) = &f.body.stmts[2] {
                    assert!(matches!(&let_stmt.value, Expr::Index(_, _)));
                } else {
                    panic!("expected let with index");
                }
                // valid = !false — unary not
                if let BlockStmt::Let(let_stmt) = &f.body.stmts[3] {
                    assert!(matches!(&let_stmt.value, Expr::UnaryOp(nom_ast::UnaryOp::Not, _)));
                } else {
                    panic!("expected let with unary not");
                }
            }
            other => panic!("expected FnDef, got {other:?}"),
        }
    }

    // ── Flow qualifier tests (ADOPT-5: aspect-qualified flows) ──────────────

    #[test]
    fn parses_flow_qualifier_once() {
        let src = "system test\n  flow::once request->response\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => assert_eq!(f.qualifier, FlowQualifier::Once),
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    #[test]
    fn parses_flow_qualifier_stream() {
        let src = "system test\n  flow::stream events->process->output\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => assert_eq!(f.qualifier, FlowQualifier::Stream),
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    #[test]
    fn parses_flow_qualifier_scheduled() {
        let src = "system test\n  flow::scheduled daily->cleanup->report\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => assert_eq!(f.qualifier, FlowQualifier::Scheduled),
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    #[test]
    fn default_flow_qualifier_is_once() {
        let src = "system test\n  flow request->response\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => assert_eq!(f.qualifier, FlowQualifier::Once),
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    // ── OnFail strategy tests (ADOPT-4: supervision tree for flows) ──────────

    #[test]
    fn parses_flow_onfail_abort() {
        let src = "system test\n  flow request->response onfail abort\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => assert_eq!(f.on_fail, OnFailStrategy::Abort),
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    #[test]
    fn parses_flow_onfail_retry() {
        let src = "system test\n  flow request->hash->store onfail retry 3\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => assert_eq!(f.on_fail, OnFailStrategy::Retry(3)),
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    #[test]
    fn parses_flow_onfail_restart_from() {
        let src = "system test\n  flow request->hash->store onfail restart_from hash\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => {
                match &f.on_fail {
                    OnFailStrategy::RestartFrom(id) => assert_eq!(id.name, "hash"),
                    other => panic!("expected RestartFrom, got {other:?}"),
                }
            }
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    #[test]
    fn parses_flow_onfail_skip() {
        let src = "system test\n  flow request->hash->store onfail skip\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => assert_eq!(f.on_fail, OnFailStrategy::Skip),
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    #[test]
    fn parses_flow_onfail_escalate() {
        let src = "system test\n  flow request->hash->store onfail escalate\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => assert_eq!(f.on_fail, OnFailStrategy::Escalate),
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    #[test]
    fn default_onfail_is_abort() {
        let src = "system test\n  flow request->response\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => assert_eq!(f.on_fail, OnFailStrategy::Abort),
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    #[test]
    fn flow_qualifier_with_onfail() {
        let src = "system test\n  flow::stream events->process->output onfail retry 5\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::Flow(f) => {
                assert_eq!(f.qualifier, FlowQualifier::Stream);
                assert_eq!(f.on_fail, OnFailStrategy::Retry(5));
            }
            other => panic!("expected Flow, got {other:?}"),
        }
    }

    // ── Module system tests ────────────────────────────────────────────────

    #[test]
    fn parses_use_single() {
        let src = "system test\n  use math::sin\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::Use(u) => {
                assert_eq!(u.path.len(), 1);
                assert_eq!(u.path[0].name, "math");
                match &u.imports {
                    UseImport::Single(name) => assert_eq!(name.name, "sin"),
                    other => panic!("expected Single import, got {other:?}"),
                }
            }
            other => panic!("expected Use statement, got {other:?}"),
        }
    }

    #[test]
    fn parses_use_nested_path() {
        let src = "system test\n  use math::trig::sin\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::Use(u) => {
                assert_eq!(u.path.len(), 2);
                assert_eq!(u.path[0].name, "math");
                assert_eq!(u.path[1].name, "trig");
                match &u.imports {
                    UseImport::Single(name) => assert_eq!(name.name, "sin"),
                    other => panic!("expected Single import, got {other:?}"),
                }
            }
            other => panic!("expected Use statement, got {other:?}"),
        }
    }

    #[test]
    fn parses_use_multiple() {
        let src = "system test\n  use math::{sin, cos, tan}\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::Use(u) => {
                assert_eq!(u.path.len(), 1);
                assert_eq!(u.path[0].name, "math");
                match &u.imports {
                    UseImport::Multiple(items) => {
                        assert_eq!(items.len(), 3);
                        assert_eq!(items[0].name, "sin");
                        assert_eq!(items[1].name, "cos");
                        assert_eq!(items[2].name, "tan");
                    }
                    other => panic!("expected Multiple import, got {other:?}"),
                }
            }
            other => panic!("expected Use statement, got {other:?}"),
        }
    }

    #[test]
    fn parses_use_glob() {
        let src = "system test\n  use math::*\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::Use(u) => {
                assert_eq!(u.path.len(), 1);
                assert_eq!(u.path[0].name, "math");
                assert!(matches!(&u.imports, UseImport::Glob));
            }
            other => panic!("expected Use statement, got {other:?}"),
        }
    }

    #[test]
    fn parses_mod_declaration() {
        let src = "system test\n  mod utils\n";
        let sf = parse_ok(src);
        let stmt = &sf.declarations[0].statements[0];
        match stmt {
            Statement::Mod(m) => {
                assert_eq!(m.name.name, "utils");
            }
            other => panic!("expected Mod statement, got {other:?}"),
        }
    }

    #[test]
    fn parses_multiple_use_and_mod() {
        let src = "system test\n  mod math\n  use math::sin\n  use math::cos\n";
        let sf = parse_ok(src);
        let stmts = &sf.declarations[0].statements;
        assert_eq!(stmts.len(), 3);
        assert!(matches!(&stmts[0], Statement::Mod(_)));
        assert!(matches!(&stmts[1], Statement::Use(_)));
        assert!(matches!(&stmts[2], Statement::Use(_)));
    }

    #[test]
    fn parses_try_expression_question_mark() {
        let src = "nom test\n  fn do_stuff() -> Result {\n    let x = some_call()?\n    return Ok(x)\n  }\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::FnDef(fndef) => {
                assert_eq!(fndef.name.name, "do_stuff");
                // The let statement's value should be a Try wrapping a Call
                match &fndef.body.stmts[0] {
                    BlockStmt::Let(let_stmt) => {
                        assert!(matches!(&let_stmt.value, Expr::Try(inner) if matches!(&**inner, Expr::Call(_))));
                    }
                    other => panic!("expected Let, got {other:?}"),
                }
            }
            other => panic!("expected FnDef, got {other:?}"),
        }
    }

    #[test]
    fn parses_chained_try_operators() {
        let src = "nom test\n  fn chain() -> Result {\n    let val = a()?.method()?\n    return val\n  }\n";
        let sf = parse_ok(src);
        match &sf.declarations[0].statements[0] {
            Statement::FnDef(fndef) => {
                match &fndef.body.stmts[0] {
                    BlockStmt::Let(let_stmt) => {
                        // Should be Try(MethodCall(Try(Call(...)), ...))
                        assert!(matches!(&let_stmt.value, Expr::Try(_)));
                    }
                    other => panic!("expected Let, got {other:?}"),
                }
            }
            other => panic!("expected FnDef, got {other:?}"),
        }
    }

    #[test]
    fn parses_prelude_result_and_option_enums() {
        let src = "nom prelude\n  enum Result {\n    Ok(text),\n    Err(text)\n  }\n  enum Option {\n    Some(text),\n    None\n  }\n";
        let sf = parse_ok(src);
        assert_eq!(sf.declarations[0].name.name, "prelude");
        let stmts = &sf.declarations[0].statements;
        assert_eq!(stmts.len(), 2);
        match &stmts[0] {
            Statement::EnumDef(e) => {
                assert_eq!(e.name.name, "Result");
                assert_eq!(e.variants.len(), 2);
                assert_eq!(e.variants[0].name.name, "Ok");
                assert_eq!(e.variants[1].name.name, "Err");
            }
            other => panic!("expected EnumDef, got {other:?}"),
        }
        match &stmts[1] {
            Statement::EnumDef(e) => {
                assert_eq!(e.name.name, "Option");
                assert_eq!(e.variants.len(), 2);
                assert_eq!(e.variants[0].name.name, "Some");
                assert_eq!(e.variants[1].name.name, "None");
            }
            other => panic!("expected EnumDef, got {other:?}"),
        }
    }

    // ── Trait / impl tests ───────────────────────────���────────────────────

    #[test]
    fn parses_trait_definition() {
        let source = "nom test\n  trait Display {\n    fn display(self) -> text\n  }\n";
        let sf = parse_ok(source);
        match &sf.declarations[0].statements[0] {
            Statement::TraitDef(t) => {
                assert_eq!(t.name.name, "Display");
                assert_eq!(t.methods.len(), 1);
                assert_eq!(t.methods[0].name.name, "display");
                assert_eq!(t.methods[0].params.len(), 1);
                assert_eq!(t.methods[0].params[0].name.name, "self");
                assert!(t.methods[0].body.stmts.is_empty(), "abstract method should have empty body");
            }
            other => panic!("expected TraitDef, got {other:?}"),
        }
    }

    #[test]
    fn parses_impl_block_for_trait() {
        let source = "nom test\n  impl Display for Point {\n    fn display(self) -> text {\n      return \"Point\"\n    }\n  }\n";
        let sf = parse_ok(source);
        match &sf.declarations[0].statements[0] {
            Statement::ImplBlock(i) => {
                assert_eq!(i.trait_name.as_ref().unwrap().name, "Display");
                assert_eq!(i.target_type.name, "Point");
                assert_eq!(i.methods.len(), 1);
                assert_eq!(i.methods[0].name.name, "display");
            }
            other => panic!("expected ImplBlock, got {other:?}"),
        }
    }

    #[test]
    fn parses_inherent_impl() {
        let source = "nom test\n  impl Point {\n    fn new(x: number, y: number) -> Point {\n      return x\n    }\n  }\n";
        let sf = parse_ok(source);
        match &sf.declarations[0].statements[0] {
            Statement::ImplBlock(i) => {
                assert!(i.trait_name.is_none());
                assert_eq!(i.target_type.name, "Point");
                assert_eq!(i.methods.len(), 1);
                assert_eq!(i.methods[0].name.name, "new");
            }
            other => panic!("expected ImplBlock, got {other:?}"),
        }
    }

    #[test]
    fn parses_vietnamese_trait_alias() {
        let source = "nom test\n  behavior Printable {\n    fn print(self) -> text\n  }\n";
        let sf = parse_ok(source);
        match &sf.declarations[0].statements[0] {
            Statement::TraitDef(t) => {
                assert_eq!(t.name.name, "Printable");
                assert_eq!(t.methods.len(), 1);
            }
            other => panic!("expected TraitDef from behavior alias, got {other:?}"),
        }
    }

    #[test]
    fn parses_vietnamese_impl_alias() {
        let source = "nom test\n  apply Display for Point {\n    fn display(self) -> text {\n      return \"Point\"\n    }\n  }\n";
        let sf = parse_ok(source);
        match &sf.declarations[0].statements[0] {
            Statement::ImplBlock(i) => {
                assert_eq!(i.trait_name.as_ref().unwrap().name, "Display");
                assert_eq!(i.target_type.name, "Point");
            }
            other => panic!("expected ImplBlock from apply alias, got {other:?}"),
        }
    }

    #[test]
    fn parses_pub_trait() {
        let source = "nom test\n  pub trait Serializable {\n    fn serialize(self) -> text\n  }\n";
        let sf = parse_ok(source);
        match &sf.declarations[0].statements[0] {
            Statement::TraitDef(t) => {
                assert!(t.is_pub);
                assert_eq!(t.name.name, "Serializable");
            }
            other => panic!("expected TraitDef, got {other:?}"),
        }
    }

    #[test]
    fn parses_trait_with_default_method() {
        let source = "nom test\n  trait Greet {\n    fn greet(self) -> text {\n      return \"hello\"\n    }\n  }\n";
        let sf = parse_ok(source);
        match &sf.declarations[0].statements[0] {
            Statement::TraitDef(t) => {
                assert_eq!(t.name.name, "Greet");
                assert_eq!(t.methods.len(), 1);
                assert!(!t.methods[0].body.stmts.is_empty(), "default method should have body");
            }
            other => panic!("expected TraitDef, got {other:?}"),
        }
    }

    #[test]
    fn parses_impl_with_multiple_methods() {
        let source = "nom test\n  impl Display for Point {\n    fn display(self) -> text {\n      return \"Point\"\n    }\n    fn debug(self) -> text {\n      return \"Point{}\"\n    }\n  }\n";
        let sf = parse_ok(source);
        match &sf.declarations[0].statements[0] {
            Statement::ImplBlock(i) => {
                assert_eq!(i.methods.len(), 2);
                assert_eq!(i.methods[0].name.name, "display");
                assert_eq!(i.methods[1].name.name, "debug");
            }
            other => panic!("expected ImplBlock, got {other:?}"),
        }
    }
}
