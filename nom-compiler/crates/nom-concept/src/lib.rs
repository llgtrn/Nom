//! Tier-1 (`.nomtu`) and Tier-2 (`.nom`) file-format types per
//! `research/language-analysis/08-layered-concept-component-architecture.md` §6.
//!
//! `.nomtu` = multi-entity DB2 container (small scope).
//! `.nom`   = multi-concept DB1 container (big scope).
//!
//! This crate defines the AST + parser. The `.nom` parser lands in a follow-up commit.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Closed kind set per doc 08 §8.1.
pub const KINDS: &[&str] = &[
    "function", "module", "concept", "screen", "data", "event", "media",
];

/// `.nom` file: 1..N concept declarations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NomFile {
    pub concepts: Vec<ConceptDecl>,
}

/// `.nomtu` file: 1..N entity declarations and/or composition declarations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NomtuFile {
    pub items: Vec<NomtuItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NomtuItem {
    Entity(EntityDecl),
    Composition(CompositionDecl),
}

/// One concept (one DB1 row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConceptDecl {
    pub name: String,
    pub intent: String,
    pub index: Vec<IndexClause>,
    pub exposes: Vec<String>,
    pub acceptance: Vec<String>,
    pub objectives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexClause {
    Uses(Vec<EntityRef>),
    Extends { base: String, change_set: ChangeSet },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChangeSet {
    pub adding: Vec<EntityRef>,
    pub removing: Vec<EntityRef>,
}

/// One DB2 entity declared inline in a `.nomtu`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDecl {
    pub kind: String,
    pub word: String,
    pub signature: String,
    pub contracts: Vec<ContractClause>,
}

/// A composition emitted by a `.nomtu` (one extra DB2 row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionDecl {
    pub word: String,
    pub composes: Vec<EntityRef>,
    pub glue: Option<String>,
    pub contracts: Vec<ContractClause>,
}

/// Reference to an entity. After first build the resolver writes back `hash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityRef {
    pub kind: Option<String>,
    pub word: String,
    pub hash: Option<String>,
    pub matching: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractClause {
    Requires(String),
    Ensures(String),
}

#[derive(Debug, Error)]
pub enum ConceptError {
    #[error("parser not yet implemented for `.nom` files (doc 08 §6.2)")]
    NomParserUnimplemented,
    #[error("unknown kind `{0}`; closed set per doc 08 §8.1: {KINDS:?}")]
    UnknownKind(String),
    #[error("parse error at position {position}: expected {expected}, found {found}")]
    ParseError {
        expected: String,
        found: String,
        position: usize,
    },
    #[error("empty input: a `.nomtu` file must contain at least one declaration")]
    EmptyInput,
}

// ── Lexer ────────────────────────────────────────────────────────────────────

mod lex {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Tok {
        The,
        Is,
        Composes,
        Then,
        With,
        Requires,
        Ensures,
        Matching,
        At,
        Dot,
        Comma,
        /// A kind keyword ("function", "module", …).
        Kind(String),
        /// A bare word: `[a-z0-9_]+`.
        Word(String),
        /// A double-quoted string (content without the quotes).
        Quoted(String),
    }

    /// Byte position in the source.
    #[derive(Debug, Clone)]
    pub struct Spanned {
        pub tok: Tok,
        pub pos: usize,
    }

    pub struct Lexer<'a> {
        src: &'a str,
        pos: usize,
    }

    impl<'a> Lexer<'a> {
        pub fn new(src: &'a str) -> Self {
            Lexer { src, pos: 0 }
        }

        fn skip_whitespace(&mut self) {
            while self.pos < self.src.len()
                && self.src.as_bytes()[self.pos].is_ascii_whitespace()
            {
                self.pos += 1;
            }
        }

        pub fn next(&mut self) -> Option<Spanned> {
            self.skip_whitespace();
            if self.pos >= self.src.len() {
                return None;
            }
            let start = self.pos;
            let b = self.src.as_bytes()[self.pos];

            // Single-char tokens
            if b == b'.' {
                self.pos += 1;
                return Some(Spanned { tok: Tok::Dot, pos: start });
            }
            if b == b',' {
                self.pos += 1;
                return Some(Spanned { tok: Tok::Comma, pos: start });
            }
            if b == b'@' {
                self.pos += 1;
                return Some(Spanned { tok: Tok::At, pos: start });
            }

            // Double-quoted string
            if b == b'"' {
                self.pos += 1; // skip opening "
                let content_start = self.pos;
                while self.pos < self.src.len() && self.src.as_bytes()[self.pos] != b'"' {
                    self.pos += 1;
                }
                let content = self.src[content_start..self.pos].to_string();
                if self.pos < self.src.len() {
                    self.pos += 1; // skip closing "
                }
                return Some(Spanned { tok: Tok::Quoted(content), pos: start });
            }

            // Bare word: [a-z0-9_]+
            if b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit() {
                let word_start = self.pos;
                while self.pos < self.src.len() {
                    let c = self.src.as_bytes()[self.pos];
                    if c.is_ascii_lowercase() || c == b'_' || c.is_ascii_digit() {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                let word = &self.src[word_start..self.pos];
                let tok = match word {
                    "the"      => Tok::The,
                    "is"       => Tok::Is,
                    "composes" => Tok::Composes,
                    "then"     => Tok::Then,
                    "with"     => Tok::With,
                    "requires" => Tok::Requires,
                    "ensures"  => Tok::Ensures,
                    "matching" => Tok::Matching,
                    "function" | "module" | "concept" | "screen"
                    | "data"   | "event"  | "media"   => Tok::Kind(word.to_string()),
                    _          => Tok::Word(word.to_string()),
                };
                return Some(Spanned { tok, pos: start });
            }

            // Skip anything else (e.g. uppercase, punctuation in prose) as an
            // opaque byte so the rest-of-prose collector can gather it.
            self.pos += 1;
            Some(Spanned {
                tok: Tok::Word(String::from_utf8_lossy(&[b]).into_owned()),
                pos: start,
            })
        }

        /// Peek at the next token without consuming it.
        pub fn peek(&mut self) -> Option<Tok> {
            let saved = self.pos;
            let result = self.next().map(|s| s.tok);
            self.pos = saved;
            result
        }

        pub fn position(&self) -> usize {
            self.pos
        }
    }
}

// ── Parser ───────────────────────────────────────────────────────────────────

mod parse {
    use super::lex::Tok;
    use super::*;

    // Re-export for convenience inside this module.
    type Lexer<'a> = super::lex::Lexer<'a>;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn err_expected(expected: &str, found: &str, pos: usize) -> ConceptError {
        ConceptError::ParseError {
            expected: expected.to_string(),
            found: found.to_string(),
            position: pos,
        }
    }

    fn tok_display(tok: &Tok) -> String {
        match tok {
            Tok::The      => "`the`".into(),
            Tok::Is       => "`is`".into(),
            Tok::Composes => "`composes`".into(),
            Tok::Then     => "`then`".into(),
            Tok::With     => "`with`".into(),
            Tok::Requires => "`requires`".into(),
            Tok::Ensures  => "`ensures`".into(),
            Tok::Matching => "`matching`".into(),
            Tok::At       => "`@`".into(),
            Tok::Dot      => "`.`".into(),
            Tok::Comma    => "`,`".into(),
            Tok::Kind(k)  => format!("`{k}`"),
            Tok::Word(w)  => format!("`{w}`"),
            Tok::Quoted(q) => format!("`\"{q}\"`"),
        }
    }

    /// Expect a specific token variant; return its position on success.
    fn expect(lex: &mut Lexer<'_>, want: &Tok) -> Result<usize, ConceptError> {
        let pos = lex.position();
        match lex.next() {
            None => Err(err_expected(&tok_display(want), "end of input", pos)),
            Some(s) => {
                if std::mem::discriminant(&s.tok) == std::mem::discriminant(want) {
                    Ok(s.pos)
                } else {
                    Err(err_expected(&tok_display(want), &tok_display(&s.tok), s.pos))
                }
            }
        }
    }

    /// Expect `the` keyword.
    fn expect_the(lex: &mut Lexer<'_>) -> Result<usize, ConceptError> {
        expect(lex, &Tok::The)
    }

    /// Expect a kind token; return the kind string.
    fn expect_kind(lex: &mut Lexer<'_>) -> Result<String, ConceptError> {
        let pos = lex.position();
        match lex.next() {
            None => Err(err_expected("a kind keyword", "end of input", pos)),
            Some(s) => match s.tok {
                Tok::Kind(k) => Ok(k),
                Tok::Word(w) => Err(ConceptError::UnknownKind(w)),
                other => Err(err_expected("a kind keyword", &tok_display(&other), s.pos)),
            },
        }
    }

    /// Expect a bare word; return it.
    fn expect_word(lex: &mut Lexer<'_>) -> Result<String, ConceptError> {
        let pos = lex.position();
        match lex.next() {
            None => Err(err_expected("a word", "end of input", pos)),
            Some(s) => match s.tok {
                Tok::Word(w) => Ok(w),
                other => Err(err_expected("a word", &tok_display(&other), s.pos)),
            },
        }
    }

    /// Expect `is`.
    fn expect_is(lex: &mut Lexer<'_>) -> Result<usize, ConceptError> {
        expect(lex, &Tok::Is)
    }

    /// Expect `.`
    fn expect_dot(lex: &mut Lexer<'_>) -> Result<usize, ConceptError> {
        let pos = lex.position();
        match lex.next() {
            None => Err(ConceptError::ParseError {
                expected: "`.` to terminate declaration".to_string(),
                found: "end of input".to_string(),
                position: pos,
            }),
            Some(s) => match s.tok {
                Tok::Dot => Ok(s.pos),
                other => Err(ConceptError::ParseError {
                    expected: "`.` to terminate declaration".to_string(),
                    found: tok_display(&other),
                    position: s.pos,
                }),
            },
        }
    }

    // ── prose collector ──────────────────────────────────────────────────────

    /// Collect tokens as prose until we hit `.` or a contract-clause keyword
    /// (`requires` / `ensures`). Does NOT consume the terminator.
    ///
    /// Returns the collected text with normalized spacing (words joined by " ").
    fn collect_prose(lex: &mut Lexer<'_>) -> String {
        let mut parts: Vec<String> = Vec::new();
        loop {
            match lex.peek() {
                None => break,
                Some(Tok::Dot) => break,
                Some(Tok::Requires) | Some(Tok::Ensures) => break,
                // For EntityRef scanning inside compositions we also stop at
                // `then` and `with` – but those are not reached from here.
                _ => {}
            }
            if let Some(s) = lex.next() {
                match &s.tok {
                    Tok::Quoted(q) => parts.push(format!("\"{}\"", q)),
                    Tok::Dot => { /* should not happen given peek above */ break }
                    _ => {
                        // Reconstruct the surface form of the token.
                        let text = match &s.tok {
                            Tok::The      => "the".to_string(),
                            Tok::Is       => "is".to_string(),
                            Tok::Composes => "composes".to_string(),
                            Tok::Then     => "then".to_string(),
                            Tok::With     => "with".to_string(),
                            Tok::Requires => "requires".to_string(),
                            Tok::Ensures  => "ensures".to_string(),
                            Tok::Matching => "matching".to_string(),
                            Tok::At       => "@".to_string(),
                            Tok::Comma    => ",".to_string(),
                            Tok::Kind(k)  => k.clone(),
                            Tok::Word(w)  => w.clone(),
                            Tok::Quoted(q) => format!("\"{}\"", q),
                            Tok::Dot      => ".".to_string(),
                        };
                        parts.push(text);
                    }
                }
            }
        }
        parts.join(" ")
    }

    /// Same as `collect_prose` but also stops at `then` and `with`.
    fn collect_prose_composition(lex: &mut Lexer<'_>) -> String {
        let mut parts: Vec<String> = Vec::new();
        loop {
            match lex.peek() {
                None => break,
                Some(Tok::Dot) => break,
                Some(Tok::Requires) | Some(Tok::Ensures) => break,
                Some(Tok::Then) | Some(Tok::With) => break,
                Some(Tok::The) => break,
                _ => {}
            }
            if let Some(s) = lex.next() {
                let text = tok_surface(&s.tok);
                parts.push(text);
            }
        }
        parts.join(" ")
    }

    fn tok_surface(tok: &Tok) -> String {
        match tok {
            Tok::The      => "the".to_string(),
            Tok::Is       => "is".to_string(),
            Tok::Composes => "composes".to_string(),
            Tok::Then     => "then".to_string(),
            Tok::With     => "with".to_string(),
            Tok::Requires => "requires".to_string(),
            Tok::Ensures  => "ensures".to_string(),
            Tok::Matching => "matching".to_string(),
            Tok::At       => "@".to_string(),
            Tok::Comma    => ",".to_string(),
            Tok::Kind(k)  => k.clone(),
            Tok::Word(w)  => w.clone(),
            Tok::Quoted(q) => format!("\"{}\"", q),
            Tok::Dot      => ".".to_string(),
        }
    }

    // ── contract clauses ─────────────────────────────────────────────────────

    fn parse_contract_clauses(lex: &mut Lexer<'_>) -> Result<Vec<ContractClause>, ConceptError> {
        let mut clauses = Vec::new();
        loop {
            match lex.peek() {
                Some(Tok::Requires) => {
                    lex.next(); // consume `requires`
                    let pred = collect_prose(lex);
                    expect_dot(lex)?;
                    clauses.push(ContractClause::Requires(pred.trim().to_string()));
                }
                Some(Tok::Ensures) => {
                    lex.next(); // consume `ensures`
                    let pred = collect_prose(lex);
                    expect_dot(lex)?;
                    clauses.push(ContractClause::Ensures(pred.trim().to_string()));
                }
                _ => break,
            }
        }
        Ok(clauses)
    }

    // ── entity ref ───────────────────────────────────────────────────────────

    /// Parse `"the" Kind Word ("@" Hash)? ("matching" Phrase)?`
    fn parse_entity_ref(lex: &mut Lexer<'_>) -> Result<EntityRef, ConceptError> {
        expect_the(lex)?;
        let kind = expect_kind(lex)?;
        let word = expect_word(lex)?;

        // Optional @hash
        let hash = if lex.peek() == Some(Tok::At) {
            lex.next(); // consume @
            Some(expect_word(lex)?)
        } else {
            None
        };

        // Optional matching "..."
        let matching = if lex.peek() == Some(Tok::Matching) {
            lex.next(); // consume `matching`
            let pos = lex.position();
            match lex.next() {
                Some(s) => match s.tok {
                    Tok::Quoted(q) => Some(q),
                    other => return Err(err_expected("a quoted string after `matching`", &tok_display(&other), s.pos)),
                },
                None => return Err(err_expected("a quoted string after `matching`", "end of input", pos)),
            }
        } else {
            None
        };

        Ok(EntityRef { kind: Some(kind), word, hash, matching })
    }

    // ── entity decl ──────────────────────────────────────────────────────────

    /// Parse `"the" Kind Word "is" SignatureBody ContractClause* "."`
    /// (the leading `the` has already been consumed by the dispatch in
    ///  `parse_entity_or_composition`).
    fn parse_entity_decl(lex: &mut Lexer<'_>, kind: String, word: String) -> Result<EntityDecl, ConceptError> {
        expect_is(lex)?;
        // Collect signature prose; stops before `.` or a contract keyword.
        let signature = collect_prose(lex).trim().to_string();
        // Consume the `.` that terminates the signature line.
        expect_dot(lex)?;
        // Collect zero or more contract clauses (each consumes its own `.`).
        let contracts = parse_contract_clauses(lex)?;
        // No additional closing `.` — the last clause's `.` (or the signature's
        // `.` when there are no clauses) already terminated the declaration.
        Ok(EntityDecl { kind, word, signature, contracts })
    }

    // ── composition decl ─────────────────────────────────────────────────────

    /// Parse composition after we've already consumed `the module Word composes`.
    fn parse_composition_decl(lex: &mut Lexer<'_>, word: String) -> Result<CompositionDecl, ConceptError> {
        // First entity ref
        let first_ref = parse_entity_ref(lex)?;
        let mut composes = vec![first_ref];

        // `then` EntityRef*
        while lex.peek() == Some(Tok::Then) {
            lex.next(); // consume `then`
            composes.push(parse_entity_ref(lex)?);
        }

        // Optional `with` Glue (quoted or unquoted prose up to contract/dot)
        let glue = if lex.peek() == Some(Tok::With) {
            lex.next(); // consume `with`
            // glue may be a quoted string or bare prose
            let pos = lex.position();
            match lex.peek() {
                Some(Tok::Quoted(_)) => {
                    if let Some(s) = lex.next() {
                        match s.tok {
                            Tok::Quoted(q) => Some(q),
                            _ => unreachable!(),
                        }
                    } else {
                        return Err(err_expected("glue string after `with`", "end of input", pos));
                    }
                }
                _ => {
                    let prose = collect_prose_composition(lex).trim().to_string();
                    if prose.is_empty() { None } else { Some(prose) }
                }
            }
        } else {
            None
        };

        let contracts = parse_contract_clauses(lex)?;
        // When there are no contracts, a `.` terminates the composition.
        // When contracts are present, the last clause's `.` already terminated it.
        if contracts.is_empty() {
            expect_dot(lex)?;
        }

        Ok(CompositionDecl { word, composes, glue, contracts })
    }

    // ── top-level dispatch ───────────────────────────────────────────────────

    fn parse_item(lex: &mut Lexer<'_>) -> Result<NomtuItem, ConceptError> {
        // Every item starts with `the`
        expect_the(lex)?;

        // Peek at the kind token
        let pos_after_the = lex.position();
        let kind_or_err = match lex.next() {
            None => return Err(err_expected("a kind keyword", "end of input", pos_after_the)),
            Some(s) => match s.tok {
                Tok::Kind(k) => Ok((k, s.pos)),
                Tok::Word(w) => Err((w, s.pos)),
                other => return Err(err_expected("a kind keyword", &tok_display(&other), s.pos)),
            },
        };

        let (kind, _kind_pos) = match kind_or_err {
            Ok(pair) => pair,
            Err((w, _wpos)) => return Err(ConceptError::UnknownKind(w)),
        };

        // Get the word name
        let word = expect_word(lex)?;

        // Is this `the module X composes …` or `the <kind> X is …`?
        if kind == "module" {
            // Could be either; peek at next token
            match lex.peek() {
                Some(Tok::Composes) => {
                    lex.next(); // consume `composes`
                    let comp = parse_composition_decl(lex, word)?;
                    return Ok(NomtuItem::Composition(comp));
                }
                _ => {
                    // Fall through to entity decl
                }
            }
        }

        let entity = parse_entity_decl(lex, kind, word)?;
        Ok(NomtuItem::Entity(entity))
    }

    // ── public entry point ───────────────────────────────────────────────────

    pub fn parse_nomtu(src: &str) -> Result<NomtuFile, ConceptError> {
        let trimmed = src.trim();
        if trimmed.is_empty() {
            return Err(ConceptError::EmptyInput);
        }

        let mut lex = Lexer::new(src);
        let mut items = Vec::new();

        loop {
            // Skip whitespace; if nothing left, stop
            match lex.peek() {
                None => break,
                _ => {}
            }
            items.push(parse_item(&mut lex)?);
        }

        if items.is_empty() {
            return Err(ConceptError::EmptyInput);
        }

        Ok(NomtuFile { items })
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a `.nom` source text into a [`NomFile`].
///
/// Stub: lexer integration lands in a follow-up commit.
pub fn parse_nom(_src: &str) -> Result<NomFile, ConceptError> {
    Err(ConceptError::NomParserUnimplemented)
}

/// Parse a `.nomtu` source text into a [`NomtuFile`].
pub fn parse_nomtu(src: &str) -> Result<NomtuFile, ConceptError> {
    parse::parse_nomtu(src)
}

/// True if `kind` is in the closed set per doc 08 §8.1.
pub fn is_known_kind(kind: &str) -> bool {
    KINDS.contains(&kind)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── pre-existing tests (unchanged) ───────────────────────────────────────

    #[test]
    fn closed_kind_set_has_seven_members() {
        assert_eq!(KINDS.len(), 7);
        for k in ["function", "module", "concept", "screen", "data", "event", "media"] {
            assert!(is_known_kind(k));
        }
        assert!(!is_known_kind("class"));
        assert!(!is_known_kind("trait"));
    }

    #[test]
    fn ast_constructs_and_round_trips_through_json() {
        let entity = EntityDecl {
            kind: "function".to_string(),
            word: "validate_token_jwt_hmac_sha256".to_string(),
            signature: "given a token of text, returns yes or no".to_string(),
            contracts: vec![
                ContractClause::Requires("the token is non-empty".to_string()),
                ContractClause::Ensures("the result reflects whether the signature verifies".to_string()),
            ],
        };
        let nomtu = NomtuFile { items: vec![NomtuItem::Entity(entity.clone())] };
        let json = serde_json::to_string(&nomtu).unwrap();
        let back: NomtuFile = serde_json::from_str(&json).unwrap();
        assert_eq!(nomtu, back);
    }

    #[test]
    fn concept_with_index_round_trips() {
        let concept = ConceptDecl {
            name: "concept_authentication_jwt_basic".to_string(),
            intent: "let users with valid tokens reach the dashboard".to_string(),
            index: vec![IndexClause::Uses(vec![EntityRef {
                kind: Some("module".to_string()),
                word: "auth_jwt_session_compose".to_string(),
                hash: Some("a1b2c3d4".to_string()),
                matching: None,
            }])],
            exposes: vec!["auth_jwt_session_compose".to_string()],
            acceptance: vec![
                "users with valid tokens reach the dashboard within 200 ms".to_string(),
            ],
            objectives: vec!["security".to_string(), "speed".to_string()],
        };
        let nom = NomFile { concepts: vec![concept] };
        let json = serde_json::to_string(&nom).unwrap();
        let back: NomFile = serde_json::from_str(&json).unwrap();
        assert_eq!(nom, back);
    }

    // ── new parser tests ─────────────────────────────────────────────────────

    const AUTH_FIXTURE: &str = r#"
the function validate_token_jwt_hmac_sha256 is
  given a token of text, returns yes or no.
  requires the token is non-empty.
  ensures the result reflects whether the token's signature verifies.

the function issue_session_jwt_short_lived is
  given a user identity, returns a session token of text.
  ensures the token expires within fifteen minutes.

the module auth_jwt_session_compose composes
  the function validate_token_jwt_hmac_sha256 then
  the function issue_session_jwt_short_lived
  with "validate first; only issue when the token verifies."
  ensures no session is issued for an invalid token.
"#;

    /// Test 1: empty input returns an error.
    #[test]
    fn t01_empty_input_is_error() {
        assert!(matches!(parse_nomtu(""), Err(ConceptError::EmptyInput)));
        assert!(matches!(parse_nomtu("   \n  "), Err(ConceptError::EmptyInput)));
    }

    /// Test 2: the full doc 08 §6.3 fixture parses to 2 entities + 1 composition.
    #[test]
    fn t02_auth_fixture_parses_correctly() {
        let f = parse_nomtu(AUTH_FIXTURE).expect("should parse");
        assert_eq!(f.items.len(), 3, "expected 3 items");

        // First entity
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.kind, "function");
                assert_eq!(e.word, "validate_token_jwt_hmac_sha256");
                assert!(!e.signature.is_empty(), "signature should not be empty");
                assert_eq!(e.contracts.len(), 2);
            }
            _ => panic!("item 0 should be Entity"),
        }

        // Second entity
        match &f.items[1] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.kind, "function");
                assert_eq!(e.word, "issue_session_jwt_short_lived");
                assert_eq!(e.contracts.len(), 1);
                assert!(matches!(&e.contracts[0], ContractClause::Ensures(_)));
            }
            _ => panic!("item 1 should be Entity"),
        }

        // Composition
        match &f.items[2] {
            NomtuItem::Composition(c) => {
                assert_eq!(c.word, "auth_jwt_session_compose");
                assert_eq!(c.composes.len(), 2);
                assert!(c.glue.is_some(), "glue should be present");
                assert_eq!(c.contracts.len(), 1);
                assert!(matches!(&c.contracts[0], ContractClause::Ensures(_)));
            }
            _ => panic!("item 2 should be Composition"),
        }
    }

    /// Test 3: single entity with no contracts.
    #[test]
    fn t03_single_entity_no_contracts() {
        let src = "the function hash_password is given a password, returns a digest.";
        let f = parse_nomtu(src).expect("should parse");
        assert_eq!(f.items.len(), 1);
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.kind, "function");
                assert_eq!(e.word, "hash_password");
                assert!(e.contracts.is_empty());
            }
            _ => panic!("should be Entity"),
        }
    }

    /// Test 4: entity with both requires and ensures.
    #[test]
    fn t04_entity_with_requires_and_ensures() {
        let src = r#"
the data user_record is a collection of user fields.
  requires the record has a valid id.
  ensures all fields are properly typed.
"#;
        let f = parse_nomtu(src).expect("should parse");
        assert_eq!(f.items.len(), 1);
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.contracts.len(), 2);
                assert!(matches!(&e.contracts[0], ContractClause::Requires(_)));
                assert!(matches!(&e.contracts[1], ContractClause::Ensures(_)));
            }
            _ => panic!("should be Entity"),
        }
    }

    /// Test 5: entity ref with @hash in a composition.
    #[test]
    fn t05_composition_entity_ref_with_hash() {
        let src = r#"
the module auth_v2 composes
  the function validate_token_jwt_hmac_sha256@a1b2c3d4 then
  the function issue_session_jwt_short_lived@deadbeef.
"#;
        let f = parse_nomtu(src).expect("should parse");
        assert_eq!(f.items.len(), 1);
        match &f.items[0] {
            NomtuItem::Composition(c) => {
                assert_eq!(c.composes.len(), 2);
                assert_eq!(c.composes[0].hash.as_deref(), Some("a1b2c3d4"));
                assert_eq!(c.composes[1].hash.as_deref(), Some("deadbeef"));
            }
            _ => panic!("should be Composition"),
        }
    }

    /// Test 6: composition with matching clause.
    #[test]
    fn t06_composition_with_matching_clause() {
        let src = r#"
the module search_pipeline composes
  the function tokenize_input matching "text tokenizer" then
  the function rank_results matching "bm25 ranker".
"#;
        let f = parse_nomtu(src).expect("should parse");
        match &f.items[0] {
            NomtuItem::Composition(c) => {
                assert_eq!(c.composes[0].matching.as_deref(), Some("text tokenizer"));
                assert_eq!(c.composes[1].matching.as_deref(), Some("bm25 ranker"));
            }
            _ => panic!("should be Composition"),
        }
    }

    /// Test 7: unknown kind returns UnknownKind error.
    #[test]
    fn t07_unknown_kind_returns_error() {
        let src = "the trait foo is does something.";
        match parse_nomtu(src) {
            Err(ConceptError::UnknownKind(k)) => assert_eq!(k, "trait"),
            other => panic!("expected UnknownKind(\"trait\"), got {:?}", other),
        }
    }

    /// Test 8: missing terminating `.` returns a parse error mentioning `.`.
    #[test]
    fn t08_missing_dot_returns_parse_error() {
        let src = "the function do_thing is performs an action";
        match parse_nomtu(src) {
            Err(ConceptError::ParseError { expected, .. }) => {
                assert!(expected.contains('.'), "error should mention `.`, got: {expected}");
            }
            other => panic!("expected ParseError, got {:?}", other),
        }
    }

    /// Regression: the old stub now removed.
    #[test]
    fn parse_stubs_return_unimplemented() {
        assert!(matches!(parse_nom(""), Err(ConceptError::NomParserUnimplemented)));
        // parse_nomtu no longer returns NomtuParserUnimplemented – it returns EmptyInput instead
        assert!(matches!(parse_nomtu(""), Err(ConceptError::EmptyInput)));
    }
}
