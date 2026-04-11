//! nom-lexer: Tokenizer for the Nom writing-style syntax.
//!
//! Nom source files look like natural-language declarations:
//!   flow register
//!     need hash::argon2 where security>0.9
//!     effects only [database]
//!     flow request->validate->hash->store->response
//!
//! Tokens are scanned left-to-right, preserving span information.

use nom_ast::Span;
use thiserror::Error;

/// Every distinct token type produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Classifier keywords ──────────────────────────────────────────────────
    System,
    Flow,
    Store,
    Graph,
    Agent,
    Test,
    Nom,
    Gate,
    Pool,
    View,

    // ── Declaration keywords ─────────────────────────────────────────────────
    Need,
    Require,
    Effects,
    Where,
    Only,
    Describe,

    // ── Flow keywords ────────────────────────────────────────────────────────
    Branch,
    IfTrue,
    IfFalse,

    // ── Test keywords ────────────────────────────────────────────────────────
    Given,
    When,
    Then,
    And,
    Contract,
    Implement,

    // ── Graph keywords ───────────────────────────────────────────────────────
    Node,
    Edge,
    Query,
    Constraint,

    // ── Agent keywords ──────────────────────────────────────────────────────
    Capability,
    Supervise,
    Receive,
    State,
    Schedule,
    Every,

    // ── Operators ────────────────────────────────────────────────────────────
    Arrow,    // ->
    ColCol,   // ::
    Plus,     // +
    Gt,       // >
    Lt,       // <
    Gte,      // >=
    Lte,      // <=
    Eq,       // =
    Neq,      // !=
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    LParen,   // (
    RParen,   // )
    Comma,    // ,

    // ── Literals ─────────────────────────────────────────────────────────────
    /// An integer literal, e.g. `42`
    Integer(i64),
    /// A floating-point literal, e.g. `0.9`
    Float(f64),
    /// A quoted string literal, e.g. `"hello world"`
    StringLit(String),

    // ── Identifiers ──────────────────────────────────────────────────────────
    Ident(String),

    // ── Whitespace / structure ────────────────────────────────────────────────
    /// A newline character.
    Newline,
    /// A completely blank line (two consecutive newlines, or a line with only spaces).
    BlankLine,
    /// A comment starting with `#`.
    Comment(String),

    // ── End of file ──────────────────────────────────────────────────────────
    Eof,
}

impl Token {
    pub fn is_classifier(&self) -> bool {
        matches!(
            self,
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
}

/// A token paired with its source location.
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

impl SpannedToken {
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}

/// Errors that can occur during lexing.
#[derive(Debug, Error)]
pub enum LexError {
    #[error("unexpected character {ch:?} at line {line}, col {col}")]
    UnexpectedChar { ch: char, line: usize, col: usize },
    #[error("unterminated string literal starting at line {line}, col {col}")]
    UnterminatedString { line: usize, col: usize },
}

/// Lexer state.
struct Lexer<'src> {
    #[allow(dead_code)]
    src: &'src str,
    chars: std::iter::Peekable<std::str::CharIndices<'src>>,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'src> Lexer<'src> {
    fn new(src: &'src str) -> Self {
        Self {
            src,
            chars: src.char_indices().peekable(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let next = self.chars.next();
        if let Some((idx, ch)) = next {
            self.pos = idx;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        next
    }

    fn span_at(&self, start_pos: usize, start_line: usize, start_col: usize) -> Span {
        Span::new(start_pos, self.pos, start_line, start_col)
    }

    fn keyword_or_ident(s: &str) -> Token {
        match s {
            "system" => Token::System,
            "flow" => Token::Flow,
            "store" => Token::Store,
            "graph" => Token::Graph,
            "agent" => Token::Agent,
            "test" => Token::Test,
            "nom" => Token::Nom,
            "gate" => Token::Gate,
            "pool" => Token::Pool,
            "view" => Token::View,
            "need" => Token::Need,
            "require" => Token::Require,
            "effects" => Token::Effects,
            "where" => Token::Where,
            "only" => Token::Only,
            "describe" => Token::Describe,
            "branch" => Token::Branch,
            "iftrue" => Token::IfTrue,
            "iffalse" => Token::IfFalse,
            "given" => Token::Given,
            "when" => Token::When,
            "then" => Token::Then,
            "and" => Token::And,
            "contract" => Token::Contract,
            "implement" => Token::Implement,
            "node" => Token::Node,
            "edge" => Token::Edge,
            "query" => Token::Query,
            "constraint" => Token::Constraint,
            "capability" => Token::Capability,
            "supervise" => Token::Supervise,
            "receive" => Token::Receive,
            "state" => Token::State,
            "schedule" => Token::Schedule,
            "every" => Token::Every,
            _ => Token::Ident(s.to_owned()),
        }
    }

    fn scan_number(&mut self, first: char, start: usize, line: usize, col: usize) -> SpannedToken {
        let mut s = String::new();
        s.push(first);
        let mut is_float = false;
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_digit() => {
                    self.advance();
                    s.push(c);
                }
                Some('.') => {
                    // peek ahead for second char to distinguish `1.0` from `1.` at end
                    self.advance();
                    s.push('.');
                    is_float = true;
                }
                Some(c) if c.is_ascii_alphabetic() => {
                    // consume unit suffix like `50ms`, `1gb` — treat as part of ident
                    self.advance();
                    s.push(c);
                    let _ = is_float; // will become ident; reset
                    loop {
                        match self.peek() {
                            Some(c2) if c2.is_ascii_alphanumeric() || c2 == '_' => {
                                self.advance();
                                s.push(c2);
                            }
                            _ => break,
                        }
                    }
                    let span = self.span_at(start, line, col);
                    return SpannedToken::new(Token::Ident(s), span);
                }
                _ => break,
            }
        }
        let span = self.span_at(start, line, col);
        if is_float {
            let v: f64 = s.parse().unwrap_or(0.0);
            SpannedToken::new(Token::Float(v), span)
        } else {
            let v: i64 = s.parse().unwrap_or(0);
            SpannedToken::new(Token::Integer(v), span)
        }
    }

    fn scan_string(&mut self, start: usize, line: usize, col: usize) -> Result<SpannedToken, LexError> {
        let mut s = String::new();
        loop {
            match self.advance() {
                Some((_, '"')) => break,
                Some((_, '\\')) => {
                    match self.advance() {
                        Some((_, 'n')) => s.push('\n'),
                        Some((_, 't')) => s.push('\t'),
                        Some((_, '"')) => s.push('"'),
                        Some((_, '\\')) => s.push('\\'),
                        Some((_, c)) => s.push(c),
                        None => return Err(LexError::UnterminatedString { line, col }),
                    }
                }
                Some((_, c)) => s.push(c),
                None => return Err(LexError::UnterminatedString { line, col }),
            }
        }
        let span = self.span_at(start, line, col);
        Ok(SpannedToken::new(Token::StringLit(s), span))
    }

    fn scan_comment(&mut self, start: usize, line: usize, col: usize) -> SpannedToken {
        let mut s = String::new();
        loop {
            match self.peek() {
                Some('\n') | None => break,
                Some(c) => {
                    self.advance();
                    s.push(c);
                }
            }
        }
        let span = self.span_at(start, line, col);
        SpannedToken::new(Token::Comment(s.trim().to_owned()), span)
    }

    fn tokenize(mut self) -> Result<Vec<SpannedToken>, LexError> {
        let mut tokens: Vec<SpannedToken> = Vec::new();
        // Track consecutive newlines for blank-line detection
        let mut consecutive_newlines: usize = 0;

        while let Some(&(idx, ch)) = self.chars.peek() {
            let start = idx;
            let line = self.line;
            let col = self.col;

            // Handle newlines specially for blank-line detection
            if ch == '\n' {
                self.advance();
                consecutive_newlines += 1;
                if consecutive_newlines == 1 {
                    tokens.push(SpannedToken::new(
                        Token::Newline,
                        Span::new(start, start + 1, line, col),
                    ));
                } else if consecutive_newlines == 2 {
                    // Replace the last Newline with a BlankLine
                    if let Some(last) = tokens.last_mut() {
                        if last.token == Token::Newline {
                            last.token = Token::BlankLine;
                        }
                    }
                }
                // 3+ consecutive newlines: already BlankLine, just skip extras
                continue;
            }

            // Any non-newline resets the counter
            consecutive_newlines = 0;

            match ch {
                // Skip horizontal whitespace
                ' ' | '\t' | '\r' => {
                    self.advance();
                }

                '#' => {
                    self.advance();
                    tokens.push(self.scan_comment(start, line, col));
                }

                '"' => {
                    self.advance();
                    tokens.push(self.scan_string(start, line, col)?);
                }

                '-' => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        tokens.push(SpannedToken::new(
                            Token::Arrow,
                            Span::new(start, start + 2, line, col),
                        ));
                    } else {
                        // lone minus — treat as ident-like continuation
                        tokens.push(SpannedToken::new(
                            Token::Ident("-".to_owned()),
                            Span::new(start, start + 1, line, col),
                        ));
                    }
                }

                ':' => {
                    self.advance();
                    if self.peek() == Some(':') {
                        self.advance();
                        tokens.push(SpannedToken::new(
                            Token::ColCol,
                            Span::new(start, start + 2, line, col),
                        ));
                    } else {
                        tokens.push(SpannedToken::new(
                            Token::Ident(":".to_owned()),
                            Span::new(start, start + 1, line, col),
                        ));
                    }
                }

                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(SpannedToken::new(Token::Gte, Span::new(start, start + 2, line, col)));
                    } else {
                        tokens.push(SpannedToken::new(Token::Gt, Span::new(start, start + 1, line, col)));
                    }
                }

                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(SpannedToken::new(Token::Lte, Span::new(start, start + 2, line, col)));
                    } else {
                        tokens.push(SpannedToken::new(Token::Lt, Span::new(start, start + 1, line, col)));
                    }
                }

                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(SpannedToken::new(Token::Neq, Span::new(start, start + 2, line, col)));
                    } else {
                        return Err(LexError::UnexpectedChar { ch: '!', line, col });
                    }
                }

                '=' => {
                    self.advance();
                    tokens.push(SpannedToken::new(Token::Eq, Span::new(start, start + 1, line, col)));
                }

                '+' => {
                    self.advance();
                    tokens.push(SpannedToken::new(Token::Plus, Span::new(start, start + 1, line, col)));
                }

                '{' => {
                    self.advance();
                    tokens.push(SpannedToken::new(Token::LBrace, Span::new(start, start + 1, line, col)));
                }

                '}' => {
                    self.advance();
                    tokens.push(SpannedToken::new(Token::RBrace, Span::new(start, start + 1, line, col)));
                }

                '[' => {
                    self.advance();
                    tokens.push(SpannedToken::new(Token::LBracket, Span::new(start, start + 1, line, col)));
                }

                ']' => {
                    self.advance();
                    tokens.push(SpannedToken::new(Token::RBracket, Span::new(start, start + 1, line, col)));
                }

                '(' => {
                    self.advance();
                    tokens.push(SpannedToken::new(Token::LParen, Span::new(start, start + 1, line, col)));
                }

                ')' => {
                    self.advance();
                    tokens.push(SpannedToken::new(Token::RParen, Span::new(start, start + 1, line, col)));
                }

                ',' => {
                    self.advance();
                    tokens.push(SpannedToken::new(Token::Comma, Span::new(start, start + 1, line, col)));
                }

                c if c.is_ascii_digit() => {
                    self.advance();
                    tokens.push(self.scan_number(c, start, line, col));
                }

                c if c.is_alphabetic() || c == '_' => {
                    self.advance();
                    let mut s = String::new();
                    s.push(c);
                    loop {
                        match self.peek() {
                            Some(nc) if nc.is_alphanumeric() || nc == '_' => {
                                self.advance();
                                s.push(nc);
                            }
                            _ => break,
                        }
                    }
                    let span = self.span_at(start, line, col);
                    tokens.push(SpannedToken::new(Self::keyword_or_ident(&s), span));
                }

                other => {
                    return Err(LexError::UnexpectedChar {
                        ch: other,
                        line,
                        col,
                    });
                }
            }
        }

        tokens.push(SpannedToken::new(Token::Eof, Span::new(self.pos, self.pos, self.line, self.col)));
        Ok(tokens)
    }
}

/// Tokenize a Nom source string.
///
/// Returns a flat vector of [`SpannedToken`] on success, or a [`LexError`] on the
/// first unrecognised character or unterminated string.
pub fn tokenize(source: &str) -> Result<Vec<SpannedToken>, LexError> {
    Lexer::new(source).tokenize()
}

/// Convenience wrapper that panics on error — useful in tests.
pub fn tokenize_unchecked(source: &str) -> Vec<SpannedToken> {
    tokenize(source).expect("lex error")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        tokenize(src).unwrap().into_iter().map(|t| t.token).collect()
    }

    #[test]
    fn classifier_keywords() {
        let src = "system flow store graph agent test nom gate pool view";
        let t = toks(src);
        assert!(t.contains(&Token::System));
        assert!(t.contains(&Token::Flow));
        assert!(t.contains(&Token::Nom));
        assert!(t.contains(&Token::View));
    }

    #[test]
    fn arrow_operator() {
        let t = toks("a->b");
        assert_eq!(t[1], Token::Arrow);
    }

    #[test]
    fn double_colon() {
        let t = toks("hash::argon2");
        assert_eq!(t[1], Token::ColCol);
        assert_eq!(t[2], Token::Ident("argon2".to_owned()));
    }

    #[test]
    fn string_literal() {
        let t = toks(r#""hello world""#);
        assert_eq!(t[0], Token::StringLit("hello world".to_owned()));
    }

    #[test]
    fn blank_line_detection() {
        let t = toks("a\n\nb");
        assert!(t.contains(&Token::BlankLine));
    }

    #[test]
    fn comment() {
        let t = toks("# this is a comment\na");
        assert!(matches!(t[0], Token::Comment(_)));
    }

    #[test]
    fn number_integer() {
        let t = toks("42");
        assert_eq!(t[0], Token::Integer(42));
    }

    #[test]
    fn number_float() {
        let t = toks("0.9");
        assert_eq!(t[0], Token::Float(0.9));
    }

    #[test]
    fn comparison_operators() {
        let t = toks(">= <= != = > <");
        assert_eq!(t[0], Token::Gte);
        assert_eq!(t[1], Token::Lte);
        assert_eq!(t[2], Token::Neq);
        assert_eq!(t[3], Token::Eq);
        assert_eq!(t[4], Token::Gt);
        assert_eq!(t[5], Token::Lt);
    }
}
