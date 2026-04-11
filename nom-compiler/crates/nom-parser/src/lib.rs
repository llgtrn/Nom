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
    BranchArm, BranchBlock, BranchCondition, CallExpr, Classifier, CompareOp, Constraint,
    ContractStmt, Declaration, DescribeStmt, EffectModifier, EffectsStmt, Expr, FlowChain,
    FlowStep, FlowStmt, GraphConstraintStmt, GraphEdgeStmt, GraphNodeStmt, GraphQueryStmt,
    Identifier, ImplementStmt, Literal, NeedStmt, NomRef, RequireStmt, SourceFile, Span,
    Statement, TestAndStmt, TestGivenStmt, TestThenStmt, TestWhenStmt, TypedParam,
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
}

impl Parser {
    fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
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
        self.tokens.get(self.pos).map(|t| &t.token).unwrap_or(&Token::Eof)
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
                    while i < self.tokens.len() && matches!(self.tokens[i].token, Token::Newline | Token::Comment(_)) {
                        i += 1;
                    }
                    // skip the identifier
                    if i < self.tokens.len() && matches!(&self.tokens[i].token, Token::Ident(_)) {
                        i += 1;
                        // skip whitespace
                        while i < self.tokens.len() && matches!(self.tokens[i].token, Token::Newline | Token::Comment(_)) {
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
        while matches!(self.peek_raw(), Token::BlankLine | Token::Newline | Token::Comment(_)) {
            self.pos += 1;
        }
    }

    fn expect_ident(&mut self) -> ParseResult<Identifier> {
        self.skip_whitespace();
        let st = self.tokens.get(self.pos).ok_or_else(|| ParseError::UnexpectedEof {
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
        self.consume_blanks();
        while !matches!(self.peek(), Token::Eof) {
            let decl = self.parse_declaration()?;
            declarations.push(decl);
            self.consume_blanks();
        }
        Ok(SourceFile {
            path: None,
            locale: None,
            declarations,
        })
    }

    fn parse_declaration(&mut self) -> ParseResult<Declaration> {
        self.skip_whitespace();
        let start_span = self.peek_span();

        // consume classifier
        let classifier = match self.peek().clone() {
            Token::System => { self.advance(); Classifier::System }
            Token::Flow => { self.advance(); Classifier::Flow }
            Token::Store => { self.advance(); Classifier::Store }
            Token::Graph => { self.advance(); Classifier::Graph }
            Token::Agent => { self.advance(); Classifier::Agent }
            Token::Test => { self.advance(); Classifier::Test }
            Token::Nom => { self.advance(); Classifier::Nom }
            Token::Gate => { self.advance(); Classifier::Gate }
            Token::Pool => { self.advance(); Classifier::Pool }
            Token::View => { self.advance(); Classifier::View }
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
                Err(e) => return Err(e),
            }
        }

        let end_span = self.peek_span();
        Ok(Declaration {
            classifier,
            name,
            statements,
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.col),
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
            Token::Constraint => Ok(Some(Statement::GraphConstraint(self.parse_graph_constraint()?))),
            // ── Agent-native statements ─────────────────────────────────────
            Token::Capability => Ok(Some(Statement::AgentCapability(self.parse_agent_capability()?))),
            Token::Supervise => Ok(Some(Statement::AgentSupervise(self.parse_agent_supervise()?))),
            Token::Receive => Ok(Some(Statement::AgentReceive(self.parse_agent_receive()?))),
            Token::State => Ok(Some(Statement::AgentState(self.parse_agent_state()?))),
            Token::Schedule => Ok(Some(Statement::AgentSchedule(self.parse_agent_schedule()?))),
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
        let left = self.parse_expr_primary()?;
        let op = match self.peek().clone() {
            Token::Gt => { self.advance(); CompareOp::Gt }
            Token::Lt => { self.advance(); CompareOp::Lt }
            Token::Gte => { self.advance(); CompareOp::Gte }
            Token::Lte => { self.advance(); CompareOp::Lte }
            Token::Eq => { self.advance(); CompareOp::Eq }
            Token::Neq => { self.advance(); CompareOp::Neq }
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
        let right = self.parse_expr_primary()?;
        let end = self.peek_span();
        Ok(Constraint {
            left,
            op,
            right,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
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
            Token::Ident(name) => {
                let span = self.peek_span();
                self.advance();
                Ok(Expr::Ident(Identifier::new(name, span)))
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
        let chain = self.parse_flow_chain()?;
        let end = self.peek_span();
        Ok(FlowStmt {
            chain,
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
                        args.push(self.parse_expr_primary()?);
                        if matches!(self.peek(), Token::Comma) {
                            self.advance();
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
                    arms.push(BranchArm { condition: BranchCondition::IfTrue, chain });
                }
                Token::IfFalse => {
                    self.advance();
                    self.expect_arrow()?;
                    let chain = self.parse_flow_chain()?;
                    arms.push(BranchArm { condition: BranchCondition::IfFalse, chain });
                }
                _ => {
                    // named branch label
                    let _label = self.expect_ident()?;
                    self.expect_arrow()?;
                    let chain = self.parse_flow_chain()?;
                    arms.push(BranchArm { condition: BranchCondition::Named, chain });
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
        // A contract block contains typed params and conditions
        // For now parse a simple set of sub-statements until blank or next decl
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut effects = Vec::new();
        let mut preconditions = Vec::new();
        let mut postconditions = Vec::new();

        // Simple: read ident pairs until boundary
        loop {
            self.skip_whitespace();
            if self.at_statement_boundary() {
                break;
            }
            match self.peek().clone() {
                Token::Ident(kw) if kw == "input" || kw == "in" => {
                    self.advance();
                    let name = self.expect_ident()?;
                    let typ = if matches!(self.peek(), Token::Ident(_)) {
                        Some(self.expect_ident()?)
                    } else { None };
                    let span = name.span;
                    inputs.push(TypedParam { name, typ, span });
                }
                Token::Ident(kw) if kw == "output" || kw == "out" => {
                    self.advance();
                    let name = self.expect_ident()?;
                    let typ = if matches!(self.peek(), Token::Ident(_)) {
                        Some(self.expect_ident()?)
                    } else { None };
                    let span = name.span;
                    outputs.push(TypedParam { name, typ, span });
                }
                Token::Ident(kw) if kw == "pre" => {
                    self.advance();
                    preconditions.push(self.parse_expr_primary()?);
                }
                Token::Ident(kw) if kw == "post" => {
                    self.advance();
                    postconditions.push(self.parse_expr_primary()?);
                }
                Token::Effects => {
                    self.advance();
                    if matches!(self.peek(), Token::LBracket) {
                        self.advance();
                        loop {
                            match self.peek().clone() {
                                Token::RBracket | Token::Eof => { self.advance(); break; }
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

        // Consume everything inside { ... } as raw code
        let mut code = String::new();
        if matches!(self.peek(), Token::LBrace) {
            self.advance(); // '{'
            let mut depth = 1usize;
            while self.pos < self.tokens.len() {
                let tok = self.tokens[self.pos].token.clone();
                self.pos += 1;
                match tok {
                    Token::LBrace => { depth += 1; code.push('{'); }
                    Token::RBrace => {
                        depth -= 1;
                        if depth == 0 { break; }
                        code.push('}');
                    }
                    Token::Newline | Token::BlankLine => code.push('\n'),
                    Token::Ident(s) => { code.push_str(&s); code.push(' '); }
                    Token::StringLit(s) => { code.push('"'); code.push_str(&s); code.push('"'); }
                    Token::Integer(n) => { code.push_str(&n.to_string()); }
                    Token::Float(f) => { code.push_str(&f.to_string()); }
                    Token::Arrow => code.push_str("->"),
                    _ => {}
                }
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
                let val = self.parse_expr_primary()?;
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
                let val = self.parse_expr_primary()?;
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
        let assertion = self.parse_expr_primary()?;
        let end = self.peek_span();
        Ok(TestThenStmt {
            assertion,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_and(&mut self) -> ParseResult<TestAndStmt> {
        let start = self.peek_span();
        self.advance(); // 'and'
        let assertion = self.parse_expr_primary()?;
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
        let expr = self.parse_flow_chain()?;
        let end = self.peek_span();
        Ok(GraphQueryStmt {
            name,
            params,
            expr,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
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
        let left = self.parse_expr_with_field_access()?;
        // Check for comparison operator
        match self.peek().clone() {
            Token::Gt | Token::Lt | Token::Gte | Token::Lte | Token::Eq | Token::Neq => {
                let op = match self.peek().clone() {
                    Token::Gt => { self.advance(); nom_ast::BinOp::Gt }
                    Token::Lt => { self.advance(); nom_ast::BinOp::Lt }
                    Token::Gte => { self.advance(); nom_ast::BinOp::Gte }
                    Token::Lte => { self.advance(); nom_ast::BinOp::Lte }
                    Token::Eq => { self.advance(); nom_ast::BinOp::Eq }
                    Token::Neq => { self.advance(); nom_ast::BinOp::Neq }
                    _ => unreachable!(),
                };
                let right = self.parse_expr_with_field_access()?;
                Ok(Expr::BinaryOp(Box::new(left), op, Box::new(right)))
            }
            _ => Ok(left),
        }
    }

    /// Parse an expression that may have dot-separated field access.
    /// e.g. follows.from
    fn parse_expr_with_field_access(&mut self) -> ParseResult<Expr> {
        let expr = self.parse_expr_primary()?;
        // Check for dot (represented as Ident(".") or we need to handle it)
        // The lexer doesn't have a Dot token; field access like `follows.from`
        // would lex as Ident("follows"), then `.` is part of a number scan or unexpected.
        // Actually, looking at the lexer, `.` by itself would hit the number scanner or
        // be an unexpected char. So `follows.from` would lex as Ident("follows.from")
        // as a single identifier with the dot included... Let's check.
        // Actually no — the lexer only continues idents with alphanumeric/underscore.
        // The `.` would be an unexpected character. So for now, field access won't work
        // with dots in the lexer. We need to handle this differently.
        // The simplest approach: treat Ident tokens that contain dots as field access.
        // But the lexer won't produce those. Let's just return the primary for now.
        // The constraint expression will work with identifiers.
        // Field access via dots would require lexer Dot token support.
        // For now, return the primary expression as-is.
        Ok(expr)
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

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a slice of [`SpannedToken`]s into a [`SourceFile`].
pub fn parse(tokens: Vec<SpannedToken>) -> Result<SourceFile, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.parse_source_file()
}

/// Convenience: lex + parse a source string in one step.
pub fn parse_source(source: &str) -> Result<SourceFile, Box<dyn std::error::Error>> {
    let tokens = nom_lexer::tokenize(source)?;
    Ok(parse(tokens)?)
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
        let src = "flow register\n  describe \"registration\"\n\nflow login\n  describe \"login\"\n";
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
}
