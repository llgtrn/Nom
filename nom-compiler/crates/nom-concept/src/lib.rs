//! Tier-1 (`.nomtu`) and Tier-2 (`.nom`) file-format types per
//! `research/language-analysis/08-layered-concept-component-architecture.md` §6.
//!
//! `.nomtu` = multi-entity DB2 container (small scope).
//! `.nom`   = multi-concept DB1 container (big scope).
//!
//! This crate defines the AST + parser for both formats.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod acceptance;
pub use acceptance::{
    PredicateBinding, PredicateRewording, PreservationReport,
    bindings_for_concept, check_preservation, has_violations,
    jaccard_similarity, normalize_predicate, predicate_text_hash,
};

pub mod closure;
pub use closure::{ClosureError, ConceptClosure, ConceptGraph, UnresolvedRef};

pub mod mece;
pub use mece::{MeceReport, MeCollision, ObjectiveBinding, check_mece, check_mece_with_required_axes, stub_axis_of};

/// Closed kind set per doc 08 §8.1.
pub const KINDS: &[&str] = &[
    "function", "module", "concept", "screen", "data", "event", "media",
];

/// `.nom` file: 1..N concept declarations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NomFile {
    pub concepts: Vec<ConceptDecl>,
}

/// `.nomtu` file: 1..N entity declarations and/or composition declarations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NomtuFile {
    pub items: Vec<NomtuItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NomtuItem {
    Entity(EntityDecl),
    Composition(CompositionDecl),
}

/// One concept (one DB1 row).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConceptDecl {
    pub name: String,
    pub intent: String,
    pub index: Vec<IndexClause>,
    pub exposes: Vec<String>,
    pub acceptance: Vec<String>,
    pub objectives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexClause {
    Uses(Vec<EntityRef>),
    Extends { base: String, change_set: ChangeSet },
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ChangeSet {
    pub adding: Vec<EntityRef>,
    pub removing: Vec<EntityRef>,
}

/// Effect valence per motivation 02 §9 + motivation 10 §E #4.
/// Genuinely novel: no existing language distinguishes positive/negative effects structurally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectValence {
    /// Positive effect (cache_hit, load_balanced, auto_scaled).
    /// Drives success metrics and dashboards.
    /// Keywords: `benefit` (canonical) + `boon` (synonym).
    Benefit,
    /// Negative effect (timeout, rate_limited, memory_pressure).
    /// Drives alerts, escalation, incident response.
    /// Keywords: `hazard` (canonical) + `bane` (synonym).
    Hazard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectClause {
    pub valence: EffectValence,
    pub effects: Vec<String>,
}

/// One DB2 entity declared inline in a `.nomtu`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDecl {
    pub kind: String,
    pub word: String,
    pub signature: String,
    pub contracts: Vec<ContractClause>,
    pub effects: Vec<EffectClause>,
}

/// A composition emitted by a `.nomtu` (one extra DB2 row).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositionDecl {
    pub word: String,
    pub composes: Vec<EntityRef>,
    pub glue: Option<String>,
    pub contracts: Vec<ContractClause>,
    pub effects: Vec<EffectClause>,
}

/// Reference to an entity. After first build the resolver writes back `hash`.
///
/// Two surface forms (doc 07 §3):
///   v1 (bare word): `the function login_user matching "..."` — `typed_slot = false`
///   v2 (typed slot): `the @Function matching "..."` — `typed_slot = true`, `word = ""`
///   v2 + threshold:  `the @Function matching "..." with at-least 0.85 confidence`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityRef {
    pub kind: Option<String>,
    /// Entity name. Empty string when `typed_slot = true`.
    pub word: String,
    pub hash: Option<String>,
    pub matching: Option<String>,
    /// True when source used the `.nomx v2` typed-slot form `the @Kind matching "..."`.
    /// When true, `word` is "" and the resolver picks a hash from the dict by kind + matching.
    #[serde(default)]
    pub typed_slot: bool,
    /// Per-slot inline confidence threshold (doc 07 §6.3).
    /// Phase-9 corpus-embedding-resolver enforces this. Stub resolver ignores it.
    /// `None` ≡ "use default per-kind threshold" (also ignored by stub).
    /// Valid range: [0.0, 1.0]; enforced at parse time.
    #[serde(default)]
    pub confidence_threshold: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractClause {
    Requires(String),
    Ensures(String),
}

#[derive(Debug, Error)]
pub enum ConceptError {
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
    /// Token variants produced by the lexer.
    ///
    /// `Eq` is intentionally NOT derived: `NumberLit(f64)` contains `f64`
    /// which does not implement `Eq` (NaN != NaN). All comparisons use
    /// `PartialEq`, which is sufficient throughout the parser and tests.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Tok {
        The,
        Is,
        Composes,
        Then,
        With,
        Requires,
        Ensures,
        Matching,
        Benefit,
        Hazard,
        At,
        Dot,
        Comma,
        // .nom keywords
        Intended,
        To,
        Uses,
        Extends,
        Adding,
        Removing,
        Exposes,
        This,
        Works,
        When,
        Favor,
        /// `at-least` compound keyword for confidence threshold clauses (doc 07 §6.3).
        /// Emitted by the lexer when it sees the word `at` followed immediately by
        /// the literal sequence `-least` (hyphen + the word `least`).
        AtLeast,
        /// A decimal number literal: `[0-9]+(.[0-9]+)?`.
        /// Used for confidence threshold values in typed-slot refs.
        NumberLit(f64),
        /// A kind keyword ("function", "module", …).
        Kind(String),
        /// A bare word: `[a-z0-9_]+`.
        Word(String),
        /// A double-quoted string (content without the quotes).
        Quoted(String),
        /// `@<CapitalizedIdent>` — typed-slot kind marker (doc 07 §3).
        /// The captured string is the kind name WITHOUT the leading `@`.
        /// Example: `@Function` → `AtKind("Function")`.
        /// Distinct from `At` (used for bare `@` before a hash hex-word).
        AtKind(String),
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
        /// Overflow buffer for multi-token expansions (e.g. `khi` → This Works When).
        /// Drained before scanning new input bytes.
        pending: Vec<Spanned>,
    }

    impl<'a> Lexer<'a> {
        pub fn new(src: &'a str) -> Self {
            Lexer { src, pos: 0, pending: Vec::new() }
        }

        fn skip_whitespace(&mut self) {
            while self.pos < self.src.len()
                && self.src.as_bytes()[self.pos].is_ascii_whitespace()
            {
                self.pos += 1;
            }
        }

        pub fn next(&mut self) -> Option<Spanned> {
            // Drain any tokens buffered by a multi-token lexer expansion first.
            if !self.pending.is_empty() {
                return Some(self.pending.remove(0));
            }

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
                // Peek at the next character to decide between:
                //   `@<UppercaseLetter>…` → AtKind (typed-slot, doc 07 §3)
                //   `@<anything else>`    → At     (hash separator, existing)
                let next_ch = self.src[self.pos..].chars().next().unwrap_or('\0');
                if next_ch.is_ascii_uppercase() {
                    // Consume the identifier (UppercaseLetter followed by [A-Za-z0-9_]*)
                    let kind_start = self.pos;
                    while self.pos < self.src.len() {
                        let c = self.src[self.pos..].chars().next().unwrap();
                        if c.is_ascii_alphanumeric() || c == '_' {
                            self.pos += c.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let kind_name = self.src[kind_start..self.pos].to_string();
                    return Some(Spanned { tok: Tok::AtKind(kind_name), pos: start });
                }
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

            // Decimal number literal: `[0-9]+(.[0-9]+)?`
            // Exponents (e.g. `1.5e10`) are intentionally not supported — confidence
            // thresholds are plain decimals in [0.0, 1.0].
            if b.is_ascii_digit() {
                let num_start = self.pos;
                while self.pos < self.src.len() && self.src.as_bytes()[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
                // Optional fractional part `.digits`
                if self.pos < self.src.len() && self.src.as_bytes()[self.pos] == b'.' {
                    // Peek one byte further to check for a digit (avoids consuming a
                    // trailing `.` that terminates a statement).
                    if self.pos + 1 < self.src.len() && self.src.as_bytes()[self.pos + 1].is_ascii_digit() {
                        self.pos += 1; // consume `.`
                        while self.pos < self.src.len() && self.src.as_bytes()[self.pos].is_ascii_digit() {
                            self.pos += 1;
                        }
                    }
                }
                let num_str = &self.src[num_start..self.pos];
                let value: f64 = num_str.parse().unwrap_or(0.0);
                return Some(Spanned { tok: Tok::NumberLit(value), pos: start });
            }

            // Bare word / keyword token.
            //
            // Accepted character classes:
            //   • ASCII lowercase a–z
            //   • ASCII digit 0–9
            //   • ASCII underscore _
            //
            // Keyword tokens are English-only ASCII.  Function/word names are
            // English-only ASCII.  No Unicode in identifiers.
            //
            // We use char-based iteration (not byte-based) so single-byte ASCII
            // codepoints advance `self.pos` correctly.
            if is_word_start_char(self.src[self.pos..].chars().next().unwrap_or('\0')) {
                let word_start = self.pos;
                while self.pos < self.src.len() {
                    let c = self.src[self.pos..].chars().next().unwrap();
                    if is_word_continue_char(c) {
                        self.pos += c.len_utf8();
                    } else {
                        break;
                    }
                }
                let word = &self.src[word_start..self.pos];
                let tok = match word {
                    // ── English keywords ─────────────────────────────────────
                    "the"      => Tok::The,
                    "is"       => Tok::Is,
                    "composes" => Tok::Composes,
                    "then"     => Tok::Then,
                    "with"     => Tok::With,
                    "requires" => Tok::Requires,
                    "ensures"  => Tok::Ensures,
                    "matching" => Tok::Matching,
                    "intended" => Tok::Intended,
                    "to"       => Tok::To,
                    "uses"     => Tok::Uses,
                    "extends"  => Tok::Extends,
                    "adding"   => Tok::Adding,
                    "removing" => Tok::Removing,
                    "exposes"  => Tok::Exposes,
                    "this"     => Tok::This,
                    // ── `at-least` compound keyword (doc 07 §6.3) ────────────
                    // When the word `at` is followed immediately by the byte
                    // sequence `-least` (hyphen then the word `least`), consume
                    // all three segments and emit `AtLeast`.  If the remainder
                    // is something other than `-least` (e.g. `at` alone, `at_most`,
                    // or `at` as prose), fall through to `Tok::Word("at")`.
                    "at" => {
                        // The parser position is already past `at`.
                        // Check if the very next byte is `-`.
                        if self.pos < self.src.len() && self.src.as_bytes()[self.pos] == b'-' {
                            // Peek at the word after `-`.
                            let after_hyphen = self.pos + 1;
                            if after_hyphen < self.src.len()
                                && is_word_start_char(
                                    self.src[after_hyphen..].chars().next().unwrap_or('\0'),
                                )
                            {
                                let word2_start = after_hyphen;
                                let mut word2_end = after_hyphen;
                                while word2_end < self.src.len() {
                                    let c = self.src[word2_end..].chars().next().unwrap();
                                    if is_word_continue_char(c) {
                                        word2_end += c.len_utf8();
                                    } else {
                                        break;
                                    }
                                }
                                let word2 = &self.src[word2_start..word2_end];
                                if word2 == "least" {
                                    // Consume `-least` (1 byte for `-` + len of `least`).
                                    self.pos = word2_end;
                                    Tok::AtLeast
                                } else {
                                    Tok::Word("at".to_string())
                                }
                            } else {
                                Tok::Word("at".to_string())
                            }
                        } else {
                            Tok::Word("at".to_string())
                        }
                    }
                    "works"    => Tok::Works,
                    "when"     => Tok::When,
                    "favor"    => Tok::Favor,
                    // ── Effect valence keywords (English only) ───────────────
                    "benefit"  => Tok::Benefit,   // canonical positive
                    "boon"     => Tok::Benefit,   // English synonym
                    "hazard"   => Tok::Hazard,    // canonical negative
                    "bane"     => Tok::Hazard,    // English synonym
                    // ── Kind nouns (English) ─────────────────────────────────
                    "function" | "module" | "concept" | "screen"
                    | "data"   | "event"  | "media"   => Tok::Kind(word.to_string()),
                    _             => Tok::Word(word.to_string()),
                };
                return Some(Spanned { tok, pos: start });
            }

            // Skip anything else (e.g. uppercase, punctuation, non-keyword
            // Unicode) as an opaque char so the rest-of-prose collector can
            // gather it.  We advance by the char's UTF-8 byte length so we
            // never land mid-codepoint.
            let ch = self.src[self.pos..].chars().next().unwrap_or('\0');
            let ch_len = ch.len_utf8();
            self.pos += ch_len;
            Some(Spanned {
                tok: Tok::Word(ch.to_string()),
                pos: start,
            })
        }

        /// Peek at the next token without consuming it.
        pub fn peek(&mut self) -> Option<Tok> {
            let saved_pos = self.pos;
            let saved_pending = self.pending.clone();
            let result = self.next().map(|s| s.tok);
            self.pos = saved_pos;
            self.pending = saved_pending;
            result
        }

        pub fn position(&self) -> usize {
            self.pos
        }
    }

    // ── character-class predicates ────────────────────────────────────────────

    /// Returns `true` if `c` may start an identifier / keyword token.
    ///
    /// Accepted: ASCII lowercase a–z, ASCII underscore _.
    /// Rejected: uppercase, digits at start, all Unicode.
    pub fn is_word_start_char(c: char) -> bool {
        c.is_ascii_lowercase() || c == '_'
    }

    /// Returns `true` if `c` may continue an identifier / keyword token.
    ///
    /// Same set as `is_word_start_char` plus ASCII digits 0–9.
    pub fn is_word_continue_char(c: char) -> bool {
        c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()
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
            Tok::The        => "`the`".into(),
            Tok::Is         => "`is`".into(),
            Tok::Composes   => "`composes`".into(),
            Tok::Then       => "`then`".into(),
            Tok::With       => "`with`".into(),
            Tok::Requires   => "`requires`".into(),
            Tok::Ensures    => "`ensures`".into(),
            Tok::Matching   => "`matching`".into(),
            Tok::Benefit    => "`benefit`".into(),
            Tok::Hazard     => "`hazard`".into(),
            Tok::Intended   => "`intended`".into(),
            Tok::To         => "`to`".into(),
            Tok::Uses       => "`uses`".into(),
            Tok::Extends    => "`extends`".into(),
            Tok::Adding     => "`adding`".into(),
            Tok::Removing   => "`removing`".into(),
            Tok::Exposes    => "`exposes`".into(),
            Tok::This       => "`this`".into(),
            Tok::Works      => "`works`".into(),
            Tok::When       => "`when`".into(),
            Tok::Favor      => "`favor`".into(),
            Tok::At         => "`@`".into(),
            Tok::Dot        => "`.`".into(),
            Tok::Comma      => "`,`".into(),
            Tok::AtLeast    => "`at-least`".into(),
            Tok::NumberLit(n) => format!("`{n}`"),
            Tok::Kind(k)    => format!("`{k}`"),
            Tok::Word(w)    => format!("`{w}`"),
            Tok::Quoted(q)  => format!("`\"{q}\"`"),
            Tok::AtKind(k)  => format!("`@{k}`"),
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
    ///
    /// CONSTRAINT: word (function/entity/variable names) MUST be pure ASCII.
    /// Vietnamese diacritic characters are permitted in KEYWORDS only (they are
    /// mapped to Tok variants before the parser sees them).  A word token that
    /// contains non-ASCII bytes means the user wrote a diacritic function name,
    /// which is not supported — we reject it with a ParseError so the error is
    /// surfaced at the exact source position rather than silently accepted.
    fn expect_word(lex: &mut Lexer<'_>) -> Result<String, ConceptError> {
        let pos = lex.position();
        match lex.next() {
            None => Err(err_expected("an ASCII word", "end of input", pos)),
            Some(s) => match s.tok {
                Tok::Word(w) => {
                    if !w.is_ascii() {
                        Err(ConceptError::ParseError {
                            expected: "an ASCII identifier (function/entity names must be ASCII; \
                                       Vietnamese diacritics are for keywords only)".to_string(),
                            found: format!("`{w}` (contains non-ASCII characters)"),
                            position: s.pos,
                        })
                    } else {
                        Ok(w)
                    }
                }
                other => Err(err_expected("an ASCII word", &tok_display(&other), s.pos)),
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

    /// Collect tokens as prose until we hit `.`, a contract-clause keyword
    /// (`requires` / `ensures`), or an effect keyword (`benefit` / `hazard`).
    /// Does NOT consume the terminator.
    ///
    /// Returns the collected text with normalized spacing (words joined by " ").
    fn collect_prose(lex: &mut Lexer<'_>) -> String {
        let mut parts: Vec<String> = Vec::new();
        loop {
            match lex.peek() {
                None => break,
                Some(Tok::Dot) => break,
                Some(Tok::Requires) | Some(Tok::Ensures) => break,
                Some(Tok::Benefit) | Some(Tok::Hazard) => break,
                // For EntityRef scanning inside compositions we also stop at
                // `then` and `with` – but those are not reached from here.
                _ => {}
            }
            if let Some(s) = lex.next() {
                match &s.tok {
                    Tok::Dot => { /* should not happen given peek above */ break }
                    _ => parts.push(tok_surface(&s.tok)),
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
                Some(Tok::Benefit) | Some(Tok::Hazard) => break,
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
            Tok::The        => "the".to_string(),
            Tok::Is         => "is".to_string(),
            Tok::Composes   => "composes".to_string(),
            Tok::Then       => "then".to_string(),
            Tok::With       => "with".to_string(),
            Tok::Requires   => "requires".to_string(),
            Tok::Ensures    => "ensures".to_string(),
            Tok::Matching   => "matching".to_string(),
            Tok::Benefit    => "benefit".to_string(),
            Tok::Hazard     => "hazard".to_string(),
            Tok::Intended   => "intended".to_string(),
            Tok::To         => "to".to_string(),
            Tok::Uses       => "uses".to_string(),
            Tok::Extends    => "extends".to_string(),
            Tok::Adding     => "adding".to_string(),
            Tok::Removing   => "removing".to_string(),
            Tok::Exposes    => "exposes".to_string(),
            Tok::This       => "this".to_string(),
            Tok::Works      => "works".to_string(),
            Tok::When       => "when".to_string(),
            Tok::Favor      => "favor".to_string(),
            Tok::At         => "@".to_string(),
            Tok::Comma      => ",".to_string(),
            Tok::AtLeast    => "at-least".to_string(),
            Tok::NumberLit(n) => n.to_string(),
            Tok::Kind(k)    => k.clone(),
            Tok::Word(w)    => w.clone(),
            Tok::Quoted(q)  => format!("\"{}\"", q),
            Tok::Dot        => ".".to_string(),
            Tok::AtKind(k)  => format!("@{k}"),
        }
    }

    // ── contract + effect clauses ─────────────────────────────────────────────

    /// Parse zero or more interleaved `requires`/`ensures` contract clauses
    /// and `benefit`/`hazard` effect clauses (in any order).
    ///
    /// Each clause is self-terminating with its own `.`.
    /// Returns `(contracts, effects)` preserving source order within each vec.
    fn parse_contract_or_effect_clauses(
        lex: &mut Lexer<'_>,
    ) -> Result<(Vec<ContractClause>, Vec<EffectClause>), ConceptError> {
        let mut contracts = Vec::new();
        let mut effects   = Vec::new();
        loop {
            match lex.peek() {
                Some(Tok::Requires) => {
                    lex.next(); // consume `requires`
                    let pred = collect_prose(lex);
                    expect_dot(lex)?;
                    contracts.push(ContractClause::Requires(pred.trim().to_string()));
                }
                Some(Tok::Ensures) => {
                    lex.next(); // consume `ensures`
                    let pred = collect_prose(lex);
                    expect_dot(lex)?;
                    contracts.push(ContractClause::Ensures(pred.trim().to_string()));
                }
                Some(Tok::Benefit) | Some(Tok::Hazard) => {
                    let valence = if lex.peek() == Some(Tok::Benefit) {
                        lex.next(); // consume `benefit`
                        EffectValence::Benefit
                    } else {
                        lex.next(); // consume `hazard`
                        EffectValence::Hazard
                    };
                    // Parse comma-separated effect names until `.`
                    let mut effect_names: Vec<String> = Vec::new();
                    effect_names.push(expect_word(lex)?);
                    while lex.peek() == Some(Tok::Comma) {
                        lex.next(); // consume `,`
                        match lex.peek() {
                            Some(Tok::Dot) | None => break,
                            _ => effect_names.push(expect_word(lex)?),
                        }
                    }
                    expect_dot(lex)?;
                    effects.push(EffectClause { valence, effects: effect_names });
                }
                _ => break,
            }
        }
        Ok((contracts, effects))
    }

    // ── entity ref ───────────────────────────────────────────────────────────

    /// Parse an entity reference after consuming `the`.
    ///
    /// Two forms (doc 07 §3):
    ///   v1: `the Kind Word ("@" Hash)? ("matching" "Phrase")?`
    ///   v2: `the @Kind ("matching" "Phrase")?`
    ///
    /// The `the` token must already have been consumed by the caller.
    fn parse_entity_ref_after_the(lex: &mut Lexer<'_>) -> Result<EntityRef, ConceptError> {
        let pos = lex.position();
        // Check for typed-slot form: `@CapitalizedKind`
        match lex.peek() {
            Some(Tok::AtKind(_)) => {
                // v2 typed-slot form
                let (kind_name, at_pos) = match lex.next() {
                    Some(s) => match s.tok {
                        Tok::AtKind(k) => (k, s.pos),
                        _ => unreachable!(),
                    },
                    None => return Err(err_expected("an @Kind token", "end of input", pos)),
                };
                // Validate against closed kind set
                let kind_lower = kind_name.to_lowercase();
                if !super::KINDS.contains(&kind_lower.as_str()) {
                    return Err(ConceptError::UnknownKind(format!("@{kind_name}")));
                }
                // Optional `matching "..."` clause
                let matching = if lex.peek() == Some(Tok::Matching) {
                    lex.next(); // consume `matching`
                    let pos2 = lex.position();
                    match lex.next() {
                        Some(s) => match s.tok {
                            Tok::Quoted(q) => Some(q),
                            other => return Err(err_expected(
                                "a quoted string after `matching`",
                                &tok_display(&other),
                                s.pos,
                            )),
                        },
                        None => return Err(err_expected(
                            "a quoted string after `matching`",
                            "end of input",
                            pos2,
                        )),
                    }
                } else {
                    None
                };

                // Optional `with at-least <number> confidence` clause (doc 07 §6.3).
                // Only valid on typed-slot refs; applies to `the @Kind (matching "...")? with at-least N confidence`.
                let confidence_threshold = if lex.peek() == Some(Tok::With) {
                    lex.next(); // consume `with`
                    // Next must be `at-least`
                    let pos3 = lex.position();
                    match lex.next() {
                        Some(s) if s.tok == Tok::AtLeast => {}
                        Some(s) => return Err(err_expected(
                            "`at-least` after `with` in confidence clause",
                            &tok_display(&s.tok),
                            s.pos,
                        )),
                        None => return Err(err_expected(
                            "`at-least` after `with` in confidence clause",
                            "end of input",
                            pos3,
                        )),
                    }
                    // Next must be a number literal
                    let pos4 = lex.position();
                    let n = match lex.next() {
                        Some(s) => match s.tok {
                            Tok::NumberLit(n) => n,
                            other => return Err(err_expected(
                                "a number in [0.0, 1.0] after `at-least`",
                                &tok_display(&other),
                                s.pos,
                            )),
                        },
                        None => return Err(err_expected(
                            "a number in [0.0, 1.0] after `at-least`",
                            "end of input",
                            pos4,
                        )),
                    };
                    // Range check: [0.0, 1.0]
                    if !(0.0..=1.0).contains(&n) {
                        return Err(ConceptError::ParseError {
                            expected: "a confidence threshold in [0.0, 1.0]".to_string(),
                            found: format!("{n} (out of range)"),
                            position: pos4,
                        });
                    }
                    // Next must be the word `confidence`
                    let pos5 = lex.position();
                    match lex.next() {
                        Some(s) => match s.tok {
                            Tok::Word(ref w) if w == "confidence" => {}
                            other => return Err(err_expected(
                                "`confidence` after threshold value",
                                &tok_display(&other),
                                s.pos,
                            )),
                        },
                        None => return Err(err_expected(
                            "`confidence` after threshold value",
                            "end of input",
                            pos5,
                        )),
                    }
                    Some(n)
                } else {
                    None
                };

                let _ = at_pos; // silence unused warning
                return Ok(EntityRef {
                    kind: Some(kind_lower),
                    word: String::new(),
                    hash: None,
                    matching,
                    typed_slot: true,
                    confidence_threshold,
                });
            }
            _ => {}
        }

        // v1 bare-word form: `Kind Word (@hash)? (matching "...")?`
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
            let pos2 = lex.position();
            match lex.next() {
                Some(s) => match s.tok {
                    Tok::Quoted(q) => Some(q),
                    other => return Err(err_expected("a quoted string after `matching`", &tok_display(&other), s.pos)),
                },
                None => return Err(err_expected("a quoted string after `matching`", "end of input", pos2)),
            }
        } else {
            None
        };

        Ok(EntityRef { kind: Some(kind), word, hash, matching, typed_slot: false, confidence_threshold: None })
    }

    /// Parse `"the" Kind Word ("@" Hash)? ("matching" Phrase)?`
    /// Convenience wrapper that first consumes `the`.
    fn parse_entity_ref(lex: &mut Lexer<'_>) -> Result<EntityRef, ConceptError> {
        expect_the(lex)?;
        parse_entity_ref_after_the(lex)
    }

    // ── entity decl ──────────────────────────────────────────────────────────

    /// Parse `"the" Kind Word "is" SignatureBody ContractClause* EffectClause* "."`
    /// (the leading `the` has already been consumed by the dispatch in
    ///  `parse_entity_or_composition`).
    fn parse_entity_decl(lex: &mut Lexer<'_>, kind: String, word: String) -> Result<EntityDecl, ConceptError> {
        expect_is(lex)?;
        // Collect signature prose; stops before `.` or a contract/effect keyword.
        let signature = collect_prose(lex).trim().to_string();
        // Consume the `.` that terminates the signature line.
        expect_dot(lex)?;
        // Collect zero or more contract/effect clauses (each consumes its own `.`).
        let (contracts, effects) = parse_contract_or_effect_clauses(lex)?;
        // No additional closing `.` — the last clause's `.` (or the signature's
        // `.` when there are no clauses) already terminated the declaration.
        Ok(EntityDecl { kind, word, signature, contracts, effects })
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

        let (contracts, effects) = parse_contract_or_effect_clauses(lex)?;
        // When there are no contracts or effects, a `.` terminates the composition.
        // When clauses are present, the last clause's `.` already terminated it.
        if contracts.is_empty() && effects.is_empty() {
            expect_dot(lex)?;
        }

        Ok(CompositionDecl { word, composes, glue, contracts, effects })
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

    // ── .nom prose collector ────────────────────────────────────────────────

    /// Is this token the start of a new top-level clause in a ConceptDecl?
    fn is_concept_clause_start(tok: &Tok) -> bool {
        matches!(
            tok,
            Tok::Uses
            | Tok::Extends
            | Tok::Exposes
            | Tok::This
            | Tok::Favor
        )
    }

    /// Collect prose for intent / acceptance, stopping at `.` or any clause-
    /// start keyword. Does NOT consume the terminator.
    fn collect_concept_prose(lex: &mut Lexer<'_>) -> String {
        let mut parts: Vec<String> = Vec::new();
        loop {
            match lex.peek() {
                None => break,
                Some(Tok::Dot) => break,
                Some(ref t) if is_concept_clause_start(t) => break,
                _ => {}
            }
            if let Some(s) = lex.next() {
                parts.push(tok_surface(&s.tok));
            }
        }
        parts.join(" ")
    }

    // ── .nom entity-ref list ─────────────────────────────────────────────────

    /// Parse `"the" Kind Word ("," "the" Kind Word)*` into a Vec<EntityRef>.
    /// Used inside `uses` and `adding`/`removing` clauses.
    /// Stops when the next token is not `,` or when the comma is not followed
    /// by `the`. Consumes the closing `.`.
    fn parse_entity_ref_list(lex: &mut Lexer<'_>) -> Result<Vec<EntityRef>, ConceptError> {
        let mut refs = Vec::new();
        refs.push(parse_entity_ref(lex)?);
        loop {
            if lex.peek() != Some(Tok::Comma) {
                break;
            }
            // Peek two tokens ahead: comma then `the` → another entity ref
            lex.next(); // consume comma
            // If next token is not `the`, we've consumed a trailing comma
            // before a terminator — put nothing back (the comma was separator
            // before `.`).
            match lex.peek() {
                Some(Tok::The) => {
                    refs.push(parse_entity_ref(lex)?);
                }
                _ => break,
            }
        }
        Ok(refs)
    }

    // ── .nom index clauses ───────────────────────────────────────────────────

    /// Parse one IndexClause:
    ///   `uses EntityRef (, EntityRef)* .`
    ///   `extends the concept Word with adding EntityRef+ (removing EntityRef+)? .`
    fn parse_index_clause(lex: &mut Lexer<'_>) -> Result<IndexClause, ConceptError> {
        let pos = lex.position();
        match lex.peek() {
            Some(Tok::Uses) => {
                lex.next(); // consume `uses`
                let refs = parse_entity_ref_list(lex)?;
                expect_dot(lex)?;
                Ok(IndexClause::Uses(refs))
            }
            Some(Tok::Extends) => {
                lex.next(); // consume `extends`
                // expect `the concept Word`
                expect_the(lex)?;
                // `concept` is lexed as Tok::Kind("concept")
                let pos2 = lex.position();
                match lex.next() {
                    Some(s) => match s.tok {
                        Tok::Kind(ref k) if k == "concept" => {}
                        other => return Err(err_expected("`concept`", &tok_display(&other), s.pos)),
                    },
                    None => return Err(err_expected("`concept`", "end of input", pos2)),
                }
                let base = expect_word(lex)?;
                // expect `with`
                let pos3 = lex.position();
                match lex.next() {
                    Some(s) if s.tok == Tok::With => {}
                    Some(s) => return Err(err_expected("`with`", &tok_display(&s.tok), s.pos)),
                    None => return Err(err_expected("`with`", "end of input", pos3)),
                }
                let change_set = parse_change_set(lex)?;
                expect_dot(lex)?;
                Ok(IndexClause::Extends { base, change_set })
            }
            other => {
                let found = other.as_ref().map(tok_display).unwrap_or_else(|| "end of input".into());
                Err(err_expected("`uses` or `extends`", &found, pos))
            }
        }
    }

    /// Parse `adding EntityRef+ (removing EntityRef+)?`
    fn parse_change_set(lex: &mut Lexer<'_>) -> Result<ChangeSet, ConceptError> {
        let pos = lex.position();
        // expect `adding`
        match lex.next() {
            Some(s) if s.tok == Tok::Adding => {}
            Some(s) => return Err(err_expected("`adding`", &tok_display(&s.tok), s.pos)),
            None => return Err(err_expected("`adding`", "end of input", pos)),
        }
        let adding = parse_entity_ref_list(lex)?;

        let removing = if lex.peek() == Some(Tok::Removing) {
            lex.next(); // consume `removing`
            parse_entity_ref_list(lex)?
        } else {
            Vec::new()
        };

        Ok(ChangeSet { adding, removing })
    }

    // ── .nom concept decl ────────────────────────────────────────────────────

    /// Parse one ConceptDecl starting after the leading `the concept` has been
    /// consumed:
    ///   `Word is intended to IntentPhrase . IndexClause+ ExposesClause?
    ///    AcceptanceClause* ObjectiveClause?`
    fn parse_concept_decl(lex: &mut Lexer<'_>) -> Result<ConceptDecl, ConceptError> {
        let name = expect_word(lex)?;
        expect_is(lex)?;

        // `intended to`
        let pos = lex.position();
        match lex.next() {
            Some(s) if s.tok == Tok::Intended => {}
            Some(s) => return Err(err_expected("`intended`", &tok_display(&s.tok), s.pos)),
            None => return Err(err_expected("`intended`", "end of input", pos)),
        }
        let pos2 = lex.position();
        match lex.next() {
            Some(s) if s.tok == Tok::To => {}
            Some(s) => return Err(err_expected("`to`", &tok_display(&s.tok), s.pos)),
            None => return Err(err_expected("`to`", "end of input", pos2)),
        }

        let intent = collect_concept_prose(lex).trim().to_string();
        expect_dot(lex)?;

        // One or more IndexClauses
        let mut index = Vec::new();
        loop {
            match lex.peek() {
                Some(Tok::Uses) | Some(Tok::Extends) => {
                    index.push(parse_index_clause(lex)?);
                }
                _ => break,
            }
        }
        if index.is_empty() {
            let pos3 = lex.position();
            return Err(err_expected("`uses` or `extends` (at least one index clause required)", "none", pos3));
        }

        // Optional exposes clause
        let exposes = if lex.peek() == Some(Tok::Exposes) {
            lex.next(); // consume `exposes`
            let mut names = Vec::new();
            // collect comma-separated words until `.`
            names.push(expect_word(lex)?);
            while lex.peek() == Some(Tok::Comma) {
                lex.next(); // consume `,`
                match lex.peek() {
                    Some(Tok::Dot) | None => break,
                    _ => names.push(expect_word(lex)?),
                }
            }
            expect_dot(lex)?;
            names
        } else {
            Vec::new()
        };

        // Zero or more acceptance clauses: `this works when Prose .`
        let mut acceptance = Vec::new();
        loop {
            if lex.peek() != Some(Tok::This) {
                break;
            }
            lex.next(); // consume `this`
            let pos4 = lex.position();
            match lex.next() {
                Some(s) if s.tok == Tok::Works => {}
                Some(s) => return Err(err_expected("`works`", &tok_display(&s.tok), s.pos)),
                None => return Err(err_expected("`works`", "end of input", pos4)),
            }
            let pos5 = lex.position();
            match lex.next() {
                Some(s) if s.tok == Tok::When => {}
                Some(s) => return Err(err_expected("`when`", &tok_display(&s.tok), s.pos)),
                None => return Err(err_expected("`when`", "end of input", pos5)),
            }
            let pred = collect_concept_prose(lex).trim().to_string();
            expect_dot(lex)?;
            acceptance.push(pred);
        }

        // Optional objective clause: `favor QualityName (then QualityName)* .`
        let objectives = if lex.peek() == Some(Tok::Favor) {
            lex.next(); // consume `favor`
            let mut names = Vec::new();
            names.push(expect_word(lex)?);
            while lex.peek() == Some(Tok::Then) {
                lex.next(); // consume `then`
                names.push(expect_word(lex)?);
            }
            expect_dot(lex)?;
            names
        } else {
            Vec::new()
        };

        Ok(ConceptDecl { name, intent, index, exposes, acceptance, objectives })
    }

    // ── .nom public entry point ──────────────────────────────────────────────

    pub fn parse_nom(src: &str) -> Result<NomFile, ConceptError> {
        let trimmed = src.trim();
        if trimmed.is_empty() {
            return Err(ConceptError::EmptyInput);
        }

        let mut lex = Lexer::new(src);
        let mut concepts = Vec::new();

        loop {
            match lex.peek() {
                None => break,
                Some(Tok::The) => {
                    lex.next(); // consume `the`
                    // Next must be `concept` (Kind("concept"))
                    let pos = lex.position();
                    match lex.next() {
                        Some(s) => match s.tok {
                            Tok::Kind(ref k) if k == "concept" => {}
                            other => return Err(err_expected("`concept`", &tok_display(&other), s.pos)),
                        },
                        None => return Err(err_expected("`concept`", "end of input", pos)),
                    }
                    concepts.push(parse_concept_decl(&mut lex)?);
                }
                Some(other) => {
                    let pos = lex.position();
                    return Err(err_expected("`the concept`", &tok_display(&other), pos));
                }
            }
        }

        if concepts.is_empty() {
            return Err(ConceptError::EmptyInput);
        }

        Ok(NomFile { concepts })
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
pub fn parse_nom(src: &str) -> Result<NomFile, ConceptError> {
    parse::parse_nom(src)
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
            effects: vec![],
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
                typed_slot: false,
                confidence_threshold: None,
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

    /// Regression: empty inputs return EmptyInput for both parsers.
    #[test]
    fn parse_empty_inputs_return_empty_input_error() {
        assert!(matches!(parse_nom(""), Err(ConceptError::EmptyInput)));
        assert!(matches!(parse_nom("   \n  "), Err(ConceptError::EmptyInput)));
        assert!(matches!(parse_nomtu(""), Err(ConceptError::EmptyInput)));
    }

    // ── .nom parser tests ────────────────────────────────────────────────────

    const AUTH_NOM_FIXTURE: &str = r#"
the concept authentication_jwt_basic is
  intended to let users with valid tokens reach the dashboard.

  uses the module auth_jwt_session_compose,
       the function logout_session_invalidate_all,
       the function refresh_session_rotate.

  exposes auth_jwt_session_compose, logout_session_invalidate_all.

  this works when users with valid tokens reach the dashboard
                within two hundred milliseconds.
  this works when invalid tokens are rejected
                before any database read.

  favor security then speed.
"#;

    /// n01: empty input returns EmptyInput.
    #[test]
    fn n01_empty_input_is_error() {
        assert!(matches!(parse_nom(""), Err(ConceptError::EmptyInput)));
        assert!(matches!(parse_nom("   \n  "), Err(ConceptError::EmptyInput)));
    }

    /// n02: the doc 08 §6.3 fixture parses to exactly the specified shape.
    #[test]
    fn n02_auth_fixture_full_shape() {
        let f = parse_nom(AUTH_NOM_FIXTURE).expect("should parse");
        assert_eq!(f.concepts.len(), 1);

        let c = &f.concepts[0];
        assert_eq!(c.name, "authentication_jwt_basic");
        assert_eq!(c.intent, "let users with valid tokens reach the dashboard");

        // index: one Uses clause with 3 entity refs
        assert_eq!(c.index.len(), 1);
        match &c.index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 3);
                assert_eq!(refs[0].word, "auth_jwt_session_compose");
                assert_eq!(refs[0].kind.as_deref(), Some("module"));
                assert_eq!(refs[1].word, "logout_session_invalidate_all");
                assert_eq!(refs[1].kind.as_deref(), Some("function"));
                assert_eq!(refs[2].word, "refresh_session_rotate");
                assert_eq!(refs[2].kind.as_deref(), Some("function"));
            }
            _ => panic!("expected Uses clause"),
        }

        // exposes
        assert_eq!(c.exposes, vec!["auth_jwt_session_compose", "logout_session_invalidate_all"]);

        // acceptance
        assert_eq!(c.acceptance.len(), 2);
        assert!(c.acceptance[0].contains("valid tokens reach the dashboard"), "got: {}", c.acceptance[0]);
        assert!(c.acceptance[1].contains("invalid tokens are rejected"), "got: {}", c.acceptance[1]);

        // objectives
        assert_eq!(c.objectives, vec!["security", "speed"]);
    }

    /// n03: concept with two separate `uses` clauses.
    #[test]
    fn n03_two_uses_clauses() {
        let src = r#"
the concept two_uses_example is
  intended to demonstrate multiple index clauses.

  uses the function alpha_compute.

  uses the module beta_pipeline.

  favor correctness.
"#;
        let f = parse_nom(src).expect("should parse");
        assert_eq!(f.concepts.len(), 1);
        let c = &f.concepts[0];
        assert_eq!(c.index.len(), 2);
        match &c.index[0] {
            IndexClause::Uses(refs) => assert_eq!(refs[0].word, "alpha_compute"),
            _ => panic!("expected Uses"),
        }
        match &c.index[1] {
            IndexClause::Uses(refs) => assert_eq!(refs[0].word, "beta_pipeline"),
            _ => panic!("expected Uses"),
        }
    }

    /// n04: `extends the concept X with adding Y, Z removing W.`
    #[test]
    fn n04_extends_with_change_set() {
        let src = r#"
the concept extended_auth is
  intended to extend base auth with refresh support.

  extends the concept authentication_jwt_basic with
    adding the function refresh_session_rotate,
           the function revoke_session_all
    removing the function logout_session_invalidate_all.

  favor security.
"#;
        let f = parse_nom(src).expect("should parse");
        let c = &f.concepts[0];
        assert_eq!(c.index.len(), 1);
        match &c.index[0] {
            IndexClause::Extends { base, change_set } => {
                assert_eq!(base, "authentication_jwt_basic");
                assert_eq!(change_set.adding.len(), 2);
                assert_eq!(change_set.adding[0].word, "refresh_session_rotate");
                assert_eq!(change_set.adding[1].word, "revoke_session_all");
                assert_eq!(change_set.removing.len(), 1);
                assert_eq!(change_set.removing[0].word, "logout_session_invalidate_all");
            }
            _ => panic!("expected Extends"),
        }
    }

    /// n05: concept with no `exposes` clause → exposes is empty vec.
    #[test]
    fn n05_no_exposes_clause_gives_empty_vec() {
        let src = r#"
the concept minimal_concept is
  intended to demonstrate that the public surface is optional.

  uses the function do_the_thing.
"#;
        let f = parse_nom(src).expect("should parse");
        let c = &f.concepts[0];
        assert!(c.exposes.is_empty(), "exposes should be empty");
    }

    /// n06: multiple `this works when` predicates → all captured.
    #[test]
    fn n06_multiple_acceptance_clauses() {
        let src = r#"
the concept multi_acceptance is
  intended to verify multiple acceptance clauses.

  uses the function check_a.

  this works when condition alpha holds within five seconds.
  this works when condition beta holds without errors.
  this works when condition gamma completes on first try.
"#;
        let f = parse_nom(src).expect("should parse");
        let c = &f.concepts[0];
        assert_eq!(c.acceptance.len(), 3);
        assert!(c.acceptance[0].contains("condition alpha"));
        assert!(c.acceptance[1].contains("condition beta"));
        assert!(c.acceptance[2].contains("condition gamma"));
    }

    /// n07: `favor speed then size then readability` → 3-element objectives.
    #[test]
    fn n07_three_element_objectives() {
        let src = r#"
the concept perf_concept is
  intended to optimize for multiple qualities.

  uses the function fast_compute.

  favor speed then size then readability.
"#;
        let f = parse_nom(src).expect("should parse");
        let c = &f.concepts[0];
        assert_eq!(c.objectives, vec!["speed", "size", "readability"]);
    }

    /// n08: multi-concept file → 2 ConceptDecls.
    #[test]
    fn n08_multi_concept_file() {
        let src = r#"
the concept first_concept is
  intended to do the first thing.

  uses the function alpha_compute.

  favor speed.

the concept second_concept is
  intended to do the second thing.

  uses the module beta_pipeline.

  favor correctness then clarity.
"#;
        let f = parse_nom(src).expect("should parse");
        assert_eq!(f.concepts.len(), 2);
        assert_eq!(f.concepts[0].name, "first_concept");
        assert_eq!(f.concepts[0].objectives, vec!["speed"]);
        assert_eq!(f.concepts[1].name, "second_concept");
        assert_eq!(f.concepts[1].objectives, vec!["correctness", "clarity"]);
    }

    /// n09 (bonus): missing `intended to` after `is` → parse error mentioning `intended`.
    #[test]
    fn n09_missing_intended_to_returns_error() {
        let src = r#"
the concept bad_concept is
  uses the function something.
"#;
        match parse_nom(src) {
            Err(ConceptError::ParseError { expected, .. }) => {
                assert!(
                    expected.contains("intended"),
                    "error should mention `intended`, got: {expected}"
                );
            }
            other => panic!("expected ParseError mentioning `intended`, got {:?}", other),
        }
    }

    /// n10: agent_demo's `agent.nom` fixture parses with objectives = ["security",
    /// "composability", "speed"] in that order — verifies the parser preserves the
    /// dream-objective ranking mandated by doc 08 §6.2.
    #[test]
    fn n10_agent_demo_objectives_order() {
        // Inline fixture matching examples/agent_demo/agent.nom exactly (minus
        // the "llm" word which is plain prose — the parser handles it fine).
        let src = r#"
the concept minimal_safe_agent is
  intended to compose a small set of tools an llm can plan with safely.

  uses the concept agent_safety_policy,
       the function read_file matching "read text from a workspace path",
       the function write_file matching "write text to a workspace path",
       the function list_dir matching "list files in a workspace directory",
       the @Function matching "fetch the body of an https url",
       the function search_web matching "search the web and return result links",
       the function run_command matching "run an allowed shell command".

  exposes read_file, write_file, list_dir, fetch_url, search_web, run_command.

  this works when the safety policy is composed.
  this works when every exposed tool has at least one require clause.

  favor security then composability then speed.
"#;
        let f = parse_nom(src).expect("agent.nom fixture should parse");
        assert_eq!(f.concepts.len(), 1);
        let c = &f.concepts[0];
        assert_eq!(c.name, "minimal_safe_agent");
        // Order is significant — security must outrank composability must outrank speed.
        assert_eq!(
            c.objectives,
            vec!["security", "composability", "speed"],
            "objectives must preserve favor-then ordering from agent.nom"
        );
        // Spot-check: 2 acceptance clauses.
        assert_eq!(c.acceptance.len(), 2);
        // Spot-check: exposes list has 6 entries.
        assert_eq!(c.exposes.len(), 6);
    }

    // ── Effect valence tests ─────────────────────────────────────────────────

    /// e01: entity with one benefit clause.
    #[test]
    fn e01_entity_with_one_benefit_clause() {
        let src = "the function fetch_url is given a url, returns text.\n  benefit cache_hit.";
        let f = parse_nomtu(src).expect("should parse");
        assert_eq!(f.items.len(), 1);
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.effects.len(), 1);
                assert_eq!(e.effects[0].valence, EffectValence::Benefit);
                assert_eq!(e.effects[0].effects, vec!["cache_hit"]);
                assert!(e.contracts.is_empty());
            }
            _ => panic!("expected Entity"),
        }
    }

    /// e02: entity with one hazard clause.
    #[test]
    fn e02_entity_with_one_hazard_clause() {
        let src = "the function fetch_url is given a url, returns text.\n  hazard timeout.";
        let f = parse_nomtu(src).expect("should parse");
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.effects.len(), 1);
                assert_eq!(e.effects[0].valence, EffectValence::Hazard);
                assert_eq!(e.effects[0].effects, vec!["timeout"]);
            }
            _ => panic!("expected Entity"),
        }
    }

    /// e03: entity with both benefit and hazard clauses.
    #[test]
    fn e03_entity_with_both_valences() {
        let src = r#"the function fetch_url is given a url, returns text.
  benefit cache_hit.
  hazard timeout."#;
        let f = parse_nomtu(src).expect("should parse");
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.effects.len(), 2);
                assert_eq!(e.effects[0].valence, EffectValence::Benefit);
                assert_eq!(e.effects[0].effects, vec!["cache_hit"]);
                assert_eq!(e.effects[1].valence, EffectValence::Hazard);
                assert_eq!(e.effects[1].effects, vec!["timeout"]);
            }
            _ => panic!("expected Entity"),
        }
    }

    /// e04: effect clause with multiple effect names.
    #[test]
    fn e04_entity_with_multi_effect_clause() {
        let src = "the function fetch_url is given a url, returns text.\n  benefit cache_hit, load_balanced, auto_scaled.";
        let f = parse_nomtu(src).expect("should parse");
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.effects.len(), 1);
                assert_eq!(e.effects[0].valence, EffectValence::Benefit);
                assert_eq!(e.effects[0].effects, vec!["cache_hit", "load_balanced", "auto_scaled"]);
            }
            _ => panic!("expected Entity"),
        }
    }

    /// e05: English synonym `boon` lexes as Benefit.
    #[test]
    fn e05_boon_synonym_lexes_as_benefit() {
        use super::lex::{Lexer, Tok};
        let src = "boon cache_hit.";
        let mut l = Lexer::new(src);
        let first = l.next().expect("should have first token");
        assert_eq!(first.tok, Tok::Benefit, "boon must lex as Benefit");

        // Also verify full parse
        let full = "the function fetch_url is given a url, returns text.\n  boon cache_hit.";
        let f = parse_nomtu(full).expect("boon should parse");
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.effects.len(), 1);
                assert_eq!(e.effects[0].valence, EffectValence::Benefit);
                assert_eq!(e.effects[0].effects, vec!["cache_hit"]);
            }
            _ => panic!("expected Entity"),
        }
    }

    /// e06: English synonym `bane` lexes as Hazard.
    #[test]
    fn e06_bane_synonym_lexes_as_hazard() {
        use super::lex::{Lexer, Tok};
        let src = "bane timeout.";
        let mut l = Lexer::new(src);
        let first = l.next().expect("should have first token");
        assert_eq!(first.tok, Tok::Hazard, "bane must lex as Hazard");

        // Also verify full parse
        let full = "the function fetch_url is given a url, returns text.\n  bane timeout.";
        let f = parse_nomtu(full).expect("bane should parse");
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.effects.len(), 1);
                assert_eq!(e.effects[0].valence, EffectValence::Hazard);
                assert_eq!(e.effects[0].effects, vec!["timeout"]);
            }
            _ => panic!("expected Entity"),
        }
    }

    /// e07: both canonical keywords `benefit` and `hazard` in sequence.
    #[test]
    fn e07_canonical_keywords_benefit_and_hazard() {
        let src = r#"the function fetch_url is given a url, returns text.
  benefit cache_hit.
  hazard timeout."#;
        let f = parse_nomtu(src).expect("canonical keywords should parse");
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.effects.len(), 2);
                assert_eq!(e.effects[0].valence, EffectValence::Benefit);
                assert_eq!(e.effects[0].effects, vec!["cache_hit"]);
                assert_eq!(e.effects[1].valence, EffectValence::Hazard);
                assert_eq!(e.effects[1].effects, vec!["timeout"]);
            }
            _ => panic!("expected Entity"),
        }
    }

    /// e08: entity with effects round-trips through serde JSON.
    #[test]
    fn e08_effects_round_trip_through_serde() {
        let src = r#"the function fetch_url is given a url, returns text.
  requires the url scheme is https.
  benefit cache_hit, fast_path.
  hazard timeout, dns_failure."#;
        let f = parse_nomtu(src).expect("should parse");
        let json = serde_json::to_string(&f).expect("should serialize");
        let back: NomtuFile = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(f, back);
        // Spot-check field values survive the round-trip.
        match &back.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.contracts.len(), 1);
                assert_eq!(e.effects.len(), 2);
                assert_eq!(e.effects[0].valence, EffectValence::Benefit);
                assert_eq!(e.effects[0].effects, vec!["cache_hit", "fast_path"]);
                assert_eq!(e.effects[1].valence, EffectValence::Hazard);
                assert_eq!(e.effects[1].effects, vec!["timeout", "dns_failure"]);
            }
            _ => panic!("expected Entity"),
        }
    }

    /// e09: composition declares aggregated effects (benefit + hazard).
    #[test]
    fn e09_composition_with_effects() {
        let src = r#"the module web_pipeline composes
  the function fetch_url then
  the function parse_html
  benefit cache_hit.
  hazard timeout, rate_limited."#;
        let f = parse_nomtu(src).expect("should parse");
        assert_eq!(f.items.len(), 1);
        match &f.items[0] {
            NomtuItem::Composition(c) => {
                assert_eq!(c.word, "web_pipeline");
                assert_eq!(c.composes.len(), 2);
                assert!(c.contracts.is_empty());
                assert_eq!(c.effects.len(), 2);
                assert_eq!(c.effects[0].valence, EffectValence::Benefit);
                assert_eq!(c.effects[0].effects, vec!["cache_hit"]);
                assert_eq!(c.effects[1].valence, EffectValence::Hazard);
                assert_eq!(c.effects[1].effects, vec!["timeout", "rate_limited"]);
            }
            _ => panic!("expected Composition"),
        }
    }

    /// e10 (negative): `effects timeout.` without a valence prefix → parse error.
    /// The word `effects` is not a keyword — it lexes as a Tok::Word and the
    /// parser will see it where a new item or end-of-input is expected, producing
    /// a parse error.
    #[test]
    fn e10_effect_clause_with_unknown_valence_fails() {
        let src = r#"the function fetch_url is given a url, returns text.
  effects timeout."#;
        match parse_nomtu(src) {
            Err(ConceptError::ParseError { .. }) => {
                // Expected: unknown valence keyword rejected
            }
            Err(other) => panic!("expected ParseError, got {:?}", other),
            Ok(_) => panic!("expected parse failure for bare `effects` keyword"),
        }
    }

    // ── .nomx v2 (keyed) typed-slot tests ────────────────────────────────────

    /// ak01: `the @Function matching "verifies tokens"` parses to a typed-slot EntityRef.
    #[test]
    fn ak01_parse_at_function_with_matching() {
        let src = r#"
the concept tokenize_test is
  intended to test the typed-slot reference form.

  uses the @Function matching "verifies tokens".

  favor correctness.
"#;
        let f = parse_nom(src).expect("typed-slot concept should parse");
        let c = &f.concepts[0];
        assert_eq!(c.index.len(), 1);
        match &c.index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 1);
                let r = &refs[0];
                assert_eq!(r.kind.as_deref(), Some("function"));
                assert_eq!(r.word, "");
                assert!(r.typed_slot, "typed_slot must be true");
                assert_eq!(r.matching.as_deref(), Some("verifies tokens"));
                assert!(r.hash.is_none());
            }
            _ => panic!("expected Uses clause"),
        }
    }

    /// ak02: `the @Screen matching "home page"` in a full concept's uses clause.
    #[test]
    fn ak02_parse_at_screen_in_concept_uses() {
        let src = r#"
the concept x is
  intended to demonstrate at-screen.

  uses the @Screen matching "home page".

  this works when the screen resolves.
"#;
        let f = parse_nom(src).expect("at-screen concept should parse");
        let c = &f.concepts[0];
        match &c.index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 1);
                let r = &refs[0];
                assert_eq!(r.kind.as_deref(), Some("screen"));
                assert!(r.typed_slot);
                assert_eq!(r.matching.as_deref(), Some("home page"));
            }
            _ => panic!("expected Uses"),
        }
        assert_eq!(c.acceptance.len(), 1);
    }

    /// ak03: `the @Banana matching "..."` → lex-time error (unknown kind).
    #[test]
    fn ak03_parse_at_kind_unknown_fails() {
        let src = r#"
the concept bad_ref is
  intended to test unknown at-kind.

  uses the @Banana matching "something".

  favor correctness.
"#;
        match parse_nom(src) {
            Err(ConceptError::UnknownKind(k)) => {
                assert!(k.contains("Banana"), "error should mention `Banana`, got: {k}");
            }
            other => panic!("expected UnknownKind for @Banana, got {:?}", other),
        }
    }

    /// ak04: `@a1b2c3d4` (lowercase hex after @) is NOT AtKind — it's At + Word.
    #[test]
    fn ak04_at_lowercase_is_hash_not_at_kind() {
        use super::lex::{Lexer, Tok};

        let src = "foo@a1b2c3d4";
        let mut l = Lexer::new(src);
        let toks: Vec<Tok> = {
            let mut out = Vec::new();
            while let Some(s) = l.next() {
                out.push(s.tok);
            }
            out
        };
        // Should be: Word("foo"), At, Word("a1b2c3d4")
        assert!(toks.len() >= 3, "expected at least 3 tokens: {:?}", toks);
        assert!(matches!(&toks[0], Tok::Word(w) if w == "foo"));
        assert_eq!(toks[1], Tok::At, "@ before lowercase must be Tok::At");
        assert!(matches!(&toks[2], Tok::Word(w) if w == "a1b2c3d4"));
    }

    /// ak05: `the @Function` (no matching clause) → typed_slot=true, matching=None.
    #[test]
    fn ak05_at_kind_no_matching_clause() {
        let src = r#"
the concept no_match is
  intended to test at-kind without matching clause.

  uses the @Function.

  favor correctness.
"#;
        let f = parse_nom(src).expect("at-kind without matching should parse");
        let c = &f.concepts[0];
        match &c.index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 1);
                let r = &refs[0];
                assert!(r.typed_slot);
                assert_eq!(r.kind.as_deref(), Some("function"));
                assert!(r.matching.is_none(), "matching should be None when absent");
            }
            _ => panic!("expected Uses"),
        }
    }

    /// ak06: concept uses BOTH `the function login` (v1) AND `the @Function matching "..."` (v2).
    #[test]
    fn ak06_mixed_v1_and_v2_in_one_concept() {
        let src = r#"
the concept mixed_ref is
  intended to demonstrate both v1 and v2 references.

  uses the function login_user,
       the @Function matching "validates credentials".

  favor security.
"#;
        let f = parse_nom(src).expect("mixed v1+v2 concept should parse");
        let c = &f.concepts[0];
        match &c.index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 2);
                // v1 ref
                assert!(!refs[0].typed_slot);
                assert_eq!(refs[0].word, "login_user");
                assert_eq!(refs[0].kind.as_deref(), Some("function"));
                // v2 ref
                assert!(refs[1].typed_slot);
                assert_eq!(refs[1].word, "");
                assert_eq!(refs[1].kind.as_deref(), Some("function"));
                assert_eq!(refs[1].matching.as_deref(), Some("validates credentials"));
            }
            _ => panic!("expected Uses"),
        }
    }

    /// ak07: EntityRef with typed_slot=true round-trips through serde JSON.
    #[test]
    fn ak07_at_kind_round_trips_through_serde() {
        let src = r#"
the concept serde_slot is
  intended to verify JSON round-trip of typed-slot refs.

  uses the @Module matching "authentication pipeline".

  favor correctness.
"#;
        let f = parse_nom(src).expect("should parse");
        let json = serde_json::to_string(&f).expect("should serialize");
        let back: NomFile = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(f, back);
        // Spot-check typed_slot survived.
        match &back.concepts[0].index[0] {
            IndexClause::Uses(refs) => {
                assert!(refs[0].typed_slot, "typed_slot must survive JSON round-trip");
                assert_eq!(refs[0].kind.as_deref(), Some("module"));
                assert_eq!(refs[0].matching.as_deref(), Some("authentication pipeline"));
            }
            _ => panic!("expected Uses"),
        }
    }

    /// ak08: every kind in the closed set has a working `@Kind` form.
    #[test]
    fn ak08_all_closed_set_kinds_lex_as_at_kind() {
        use super::lex::{Lexer, Tok};

        // Build source that uses every closed-set kind.
        // We just lex the @Kind tokens directly.
        let kind_tokens = [
            ("@Function", "Function"),
            ("@Module",   "Module"),
            ("@Concept",  "Concept"),
            ("@Screen",   "Screen"),
            ("@Data",     "Data"),
            ("@Event",    "Event"),
            ("@Media",    "Media"),
        ];

        for (input, expected_name) in &kind_tokens {
            let mut l = Lexer::new(input);
            let tok = l.next().expect("should produce a token");
            match &tok.tok {
                Tok::AtKind(k) => {
                    assert_eq!(k, expected_name, "AtKind name mismatch for {input}");
                }
                other => panic!("expected AtKind for {input}, got {:?}", other),
            }
        }

        // Also verify that each validates against KINDS (no UnknownKind error).
        let concept_template = |kind_str: &str| -> String {
            format!(r#"
the concept ck_{lc} is
  intended to test at-kind {lc}.

  uses the @{k}.

  favor correctness.
"#, lc = kind_str.to_lowercase(), k = kind_str)
        };

        for kind in &["Function", "Module", "Concept", "Screen", "Data", "Event", "Media"] {
            let src = concept_template(kind);
            parse_nom(&src).unwrap_or_else(|e| panic!("@{kind} should parse, got: {:?}", e));
        }
    }

    // ── confidence threshold tests (doc 07 §6.3) ─────────────────────────────

    /// ct01: full syntax `the @Function matching "..." with at-least 0.85 confidence`
    /// parses to EntityRef with confidence_threshold = Some(0.85).
    #[test]
    fn ct01_parse_typed_slot_with_threshold() {
        let src = r#"
the concept ct01 is
  intended to test confidence threshold.

  uses the @Function matching "user authentication" with at-least 0.85 confidence.

  favor correctness.
"#;
        let nom_file = parse_nom(src).expect("should parse");
        match &nom_file.concepts[0].index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 1);
                let r = &refs[0];
                assert!(r.typed_slot, "must be typed slot");
                assert_eq!(r.kind.as_deref(), Some("function"));
                assert_eq!(r.matching.as_deref(), Some("user authentication"));
                let t = r.confidence_threshold.expect("threshold must be Some");
                assert!((t - 0.85).abs() < 1e-10, "threshold must be 0.85, got {t}");
            }
            _ => panic!("expected Uses"),
        }
    }

    /// ct02: threshold values 0.0 and 1.0 are inclusive.
    #[test]
    fn ct02_threshold_zero_and_one_inclusive() {
        for (value_str, expected) in [("0.0", 0.0_f64), ("1.0", 1.0_f64)] {
            let src = format!(r#"
the concept ct02_{label} is
  intended to test edge threshold.

  uses the @Function matching "x" with at-least {value_str} confidence.

  favor correctness.
"#, label = if expected == 0.0 { "zero" } else { "one" });
            let nom_file = parse_nom(&src)
                .unwrap_or_else(|e| panic!("threshold {value_str} should parse, got: {e:?}"));
            match &nom_file.concepts[0].index[0] {
                IndexClause::Uses(refs) => {
                    let t = refs[0].confidence_threshold.expect("threshold must be Some");
                    assert!((t - expected).abs() < 1e-10,
                        "threshold {value_str}: expected {expected}, got {t}");
                }
                _ => panic!("expected Uses"),
            }
        }
    }

    /// ct03: threshold below 0.0 is rejected with ParseError.
    #[test]
    fn ct03_threshold_above_one_rejected() {
        // Note: negative literals start with `-` which is not in the identifier
        // charset; `1.5` is the easiest above-1.0 test.
        let src = r#"
the concept ct03 is
  intended to test out-of-range threshold.

  uses the @Function matching "x" with at-least 1.5 confidence.

  favor correctness.
"#;
        assert!(
            matches!(parse_nom(src), Err(ConceptError::ParseError { .. })),
            "threshold 1.5 must produce ParseError"
        );
    }

    /// ct04: threshold without `with` keyword is a parse error
    /// (`the @Function matching "x" at-least 0.85 confidence` — missing `with`).
    #[test]
    fn ct04_at_least_without_with_rejected() {
        // The `at-least` compound without a leading `with` ends up in a `uses`
        // list with a dot terminator missing or wrong token sequence → ParseError.
        let src = r#"
the concept ct04 is
  intended to test missing with keyword.

  uses the @Function matching "x" at-least 0.85 confidence.

  favor correctness.
"#;
        assert!(
            parse_nom(src).is_err(),
            "missing `with` before `at-least` must produce an error"
        );
    }

    /// ct05: `with at-least N` without trailing `confidence` word is a parse error.
    #[test]
    fn ct05_with_at_least_missing_confidence_word_rejected() {
        let src = r#"
the concept ct05 is
  intended to test missing confidence word.

  uses the @Function matching "x" with at-least 0.85.

  favor correctness.
"#;
        assert!(
            matches!(parse_nom(src), Err(ConceptError::ParseError { .. })),
            "missing trailing `confidence` word must produce ParseError"
        );
    }

    /// ct06: serde JSON round-trip preserves confidence_threshold = Some(0.85).
    #[test]
    fn ct06_serde_round_trip_preserves_threshold() {
        let eref = EntityRef {
            kind: Some("function".to_string()),
            word: String::new(),
            hash: None,
            matching: Some("auth".to_string()),
            typed_slot: true,
            confidence_threshold: Some(0.85),
        };
        let nomtu = NomtuFile {
            items: vec![NomtuItem::Composition(
                crate::CompositionDecl {
                    word: "test_compose".to_string(),
                    composes: vec![eref],
                    glue: None,
                    contracts: vec![],
                    effects: vec![],
                }
            )],
        };
        let json = serde_json::to_string(&nomtu).expect("serialize");
        let back: NomtuFile = serde_json::from_str(&json).expect("deserialize");
        match &back.items[0] {
            NomtuItem::Composition(c) => {
                let t = c.composes[0].confidence_threshold.expect("threshold must survive round-trip");
                assert!((t - 0.85).abs() < 1e-10, "threshold must be 0.85, got {t}");
            }
            _ => panic!("expected Composition"),
        }
    }

    /// ct07: `cai @Hash a1b2` (bare `at` followed by a hash word) does not
    /// false-fire the `at-least` compound — `at` stays as `Tok::Word("at")`.
    #[test]
    fn ct07_at_token_alone_still_works() {
        // The existing `cai @Hash` form: `cai @Function` → entity ref.
        // More importantly, plain `at` in prose should not become AtLeast.
        use super::lex::{Lexer, Tok};

        // `at` not followed by `-least` → Word("at")
        let mut l = Lexer::new("at");
        let tok = l.next().expect("should produce a token");
        assert_eq!(tok.tok, Tok::Word("at".to_string()), "bare `at` must remain Word(\"at\")");

        // `at_most` (underscore, not hyphen) → Word("at_most")
        let mut l2 = Lexer::new("at_most");
        let tok2 = l2.next().expect("should produce a token");
        assert_eq!(tok2.tok, Tok::Word("at_most".to_string()), "`at_most` must be Word");

        // `at-least` (compound) → AtLeast
        let mut l3 = Lexer::new("at-least");
        let tok3 = l3.next().expect("should produce a token");
        assert_eq!(tok3.tok, Tok::AtLeast, "`at-least` must lex as AtLeast");
    }

    /// ct08: number literal lexing works for key values.
    #[test]
    fn ct08_number_literal_lexing() {
        use super::lex::{Lexer, Tok};

        let cases: &[(&str, f64)] = &[
            ("0",    0.0),
            ("1",    1.0),
            ("0.5",  0.5),
            ("0.85", 0.85),
            ("0.0",  0.0),
            ("1.0",  1.0),
        ];
        for (input, expected) in cases {
            let mut l = Lexer::new(input);
            let tok = l.next().expect("should produce a token");
            match tok.tok {
                Tok::NumberLit(n) => {
                    assert!((n - expected).abs() < 1e-10,
                        "input `{input}`: expected {expected}, got {n}");
                }
                other => panic!("input `{input}`: expected NumberLit, got {:?}", other),
            }
        }
    }

    // ── W4-A2: closed-keyword-set strictness lock (doc 13 §5 A2) ────────────
    //
    // These tests pin the invariant that `.nomx v2` keywords are
    // case-sensitive, exact-match, no-synonym. Any future refactor that
    // adds case-insensitive or fuzzy matching to the concept lexer will
    // fail these — forcing the change to be an explicit, reviewed decision.
    //
    // Strictness model: CoreNLP's Annotator pipeline classifies every token
    // or refuses the input. Nom's v2 keywords follow the same discipline —
    // misspelled or case-varied keywords degrade to `Tok::Word(s)` which
    // the parser rejects at the next grammar step, never silently promoted
    // into a reserved token.

    /// ct09a: case variants of `matching` are NOT promoted to `Tok::Matching`.
    ///
    /// Strictness invariant: uppercase chars are NOT valid word-starts
    /// (see `is_word_start_char`), so variants like `Matching` fall
    /// through to the catch-all branch and emit a single-char `Word(M)`
    /// token. Whatever the shape of the fallback, the ONLY thing that
    /// matters is: it must NOT lex as `Tok::Matching`.
    #[test]
    fn ct09a_matching_keyword_is_case_sensitive() {
        use super::lex::{Lexer, Tok};
        for variant in &["Matching", "MATCHING", "MATCHing", "matchIng"] {
            let mut l = Lexer::new(variant);
            let tok = l.next().expect("should produce a token");
            assert_ne!(
                tok.tok,
                Tok::Matching,
                "`{variant}` must NOT lex as the reserved Matching token"
            );
        }
        // Sanity: exact lowercase does promote.
        let mut l = Lexer::new("matching");
        assert_eq!(l.next().expect("token").tok, Tok::Matching);
    }

    /// ct09b: case variants of `with` / `confidence` / `the` / `is` do
    /// NOT promote to their reserved tokens — they hit the single-char
    /// fallback per ct09a's invariant.
    #[test]
    fn ct09b_core_keywords_case_sensitive() {
        use super::lex::{Lexer, Tok};
        // (input, forbidden-reserved-token) — the input MUST NOT produce
        // the forbidden token. The actual token it produces may be a
        // single-char Word (uppercase letters fall through), and that's
        // fine — it will fail the parser's grammar check downstream.
        let cases: &[(&str, Tok)] = &[
            ("With",       Tok::With),
            ("WITH",       Tok::With),
            ("Confidence", Tok::Word("confidence".to_string())), // `Confidence` must not fool the parser's `Word if w == "confidence"` check
            ("The",        Tok::The),
            ("THE",        Tok::The),
            ("IS",         Tok::Is),
            ("Is",         Tok::Is),
        ];
        for (input, forbidden) in cases {
            let mut l = Lexer::new(input);
            let tok = l.next().expect("should produce a token");
            assert_ne!(
                &tok.tok, forbidden,
                "`{input}` must NOT lex as {forbidden:?}"
            );
        }
        // Sanity: lowercase promotions work.
        for (input, expected) in &[
            ("with",       Tok::With),
            ("the",        Tok::The),
            ("is",         Tok::Is),
        ] {
            let mut l = Lexer::new(input);
            assert_eq!(&l.next().expect("token").tok, expected);
        }
    }

    /// ct09c: near-miss synonyms of `matching` never promote.
    ///
    /// `match`, `matches`, `matched`, `matchy` must stay Words. The v2
    /// parser only accepts exact `matching` per doc 07 §6.1.
    #[test]
    fn ct09c_matching_has_no_synonyms() {
        use super::lex::{Lexer, Tok};
        for variant in &["match", "matches", "matched", "matchy"] {
            let mut l = Lexer::new(variant);
            let tok = l.next().expect("should produce a token");
            assert_eq!(
                tok.tok,
                Tok::Word(variant.to_string()),
                "`{variant}` must lex as Word, never Matching"
            );
        }
    }

    /// ct09d: `at_least` / `atleast` / `at-Least` / `At-least` do NOT
    /// lex as `Tok::AtLeast`. Only the exact compound `at-least` (ASCII
    /// lowercase, hyphen-joined) wins.
    #[test]
    fn ct09d_at_least_compound_is_exact_only() {
        use super::lex::{Lexer, Tok};

        // Underscore variant — covered already by ct07 but re-pinned here.
        let mut l1 = Lexer::new("at_least");
        assert_eq!(
            l1.next().expect("token").tok,
            Tok::Word("at_least".to_string()),
            "`at_least` (underscore) must NOT lex as AtLeast"
        );

        // `atleast` (no separator) — whole word falls to Word.
        let mut l2 = Lexer::new("atleast");
        assert_eq!(
            l2.next().expect("token").tok,
            Tok::Word("atleast".to_string()),
            "`atleast` (no separator) must lex as Word"
        );

        // Case variant `At-least` — `At` never maps to `Tok::At*`; it is
        // a generic word start. Current lexer sees `At` as Word start and
        // then the hyphen terminates it; token boundary breaks the compound.
        let mut l3 = Lexer::new("At-least");
        let tok3 = l3.next().expect("token");
        assert_ne!(
            tok3.tok,
            Tok::AtLeast,
            "`At-least` (capital A) must NOT lex as AtLeast"
        );

        // Exact form still wins.
        let mut l4 = Lexer::new("at-least");
        assert_eq!(l4.next().expect("token").tok, Tok::AtLeast);
    }

    /// ct09e: kind nouns are lowercase-exact. `Function`, `FUNCTION`
    /// stay as `Tok::Word(...)`, never as `Tok::Kind(...)`.
    #[test]
    fn ct09e_kind_nouns_are_lowercase_exact() {
        use super::lex::{Lexer, Tok};
        for variant in &[
            "Function", "FUNCTION", "Module", "Concept", "Screen", "DATA",
            "Event", "Media",
        ] {
            let mut l = Lexer::new(variant);
            let tok = l.next().expect("should produce a token");
            matches!(tok.tok, Tok::Word(_))
                .then_some(())
                .unwrap_or_else(|| panic!(
                    "`{variant}` must lex as Word, not Kind — got {:?}",
                    tok.tok
                ));
        }
        // Sanity: lowercase promotes.
        for canonical in &[
            "function", "module", "concept", "screen", "data", "event", "media",
        ] {
            let mut l = Lexer::new(canonical);
            match l.next().expect("token").tok {
                Tok::Kind(ref w) if w == canonical => {}
                other => panic!(
                    "`{canonical}` must lex as Kind({canonical}), got {other:?}"
                ),
            }
        }
    }

    // ── W4-A1: mandatory kind marker on every entity ref (doc 13 §5 A1) ───
    //
    // Every `the <entity_ref>` must have either a v1 `Kind Word` form or
    // a v2 `@Kind` form. Omitting the kind entirely is a hard parse error.
    // These tests pin the current strictness so future refactors can't
    // accidentally allow bare-prose entity refs.

    /// ct10a: `the matching "x"` (no kind, no word) is rejected.
    #[test]
    fn ct10a_entity_ref_without_kind_rejected() {
        let src = r#"
the concept ct10a is
  intended to test rejection of kindless entity refs.

  uses the matching "something".

  favor correctness.
"#;
        assert!(
            parse_nom(src).is_err(),
            "`the matching \"x\"` (no kind) must be rejected"
        );
    }

    /// ct10b: `the @NotAKind matching "x"` (unknown kind) is rejected.
    #[test]
    fn ct10b_entity_ref_with_unknown_kind_rejected() {
        let src = r#"
the concept ct10b is
  intended to test rejection of unknown @Kind values.

  uses the @Banana matching "something".

  favor correctness.
"#;
        let result = parse_nom(src);
        assert!(
            matches!(result, Err(ConceptError::UnknownKind(_))),
            "`the @Banana` must produce UnknownKind, got {result:?}"
        );
    }

    /// ct10c: `the login_user` (v1 word-only, no kind keyword) is rejected.
    /// Note: v1 bare-word form requires BOTH `Kind` and `Word` — omitting
    /// the kind and supplying only a word falls through to `expect_kind`
    /// which returns `UnknownKind` for the first `Word` token it sees.
    #[test]
    fn ct10c_entity_ref_v1_word_without_kind_rejected() {
        let src = r#"
the concept ct10c is
  intended to test rejection of kindless v1 refs.

  uses the login_user matching "something".

  favor correctness.
"#;
        let result = parse_nom(src);
        assert!(
            result.is_err(),
            "`the login_user matching ...` (no kind keyword) must be rejected, got {result:?}"
        );
    }

    /// ct11: UTF-8 string literals survive verbatim through the parser
    /// (doc 17 §I4 smoke). Non-ASCII content inside `"..."` — author
    /// names, multilingual prose in `matching` clauses, math symbols —
    /// must land in the AST byte-identical to source. Identifiers remain
    /// ASCII-only (enforced elsewhere by `is_word_start_char`).
    ///
    /// Scope: string literals (Quoted tokens) only. Intent prose outside
    /// quotes is a separate surface and currently ASCII-reliant because
    /// the lexer's fallthrough branch emits non-ASCII chars as single-
    /// char Word tokens, which the prose collector may rewrite or drop.
    /// Lift that restriction under a future wedge if non-ASCII prose is
    /// needed inside `intended to …` sentences.
    #[test]
    fn ct11_utf8_string_literals_verbatim() {
        let src = r#"
the concept ct11 is
  intended to test multilingual string literals survive verbatim.

  uses the @Function matching "こんにちは greeting flow" with at-least 0.85 confidence.
  uses the @Function matching "Blaž Hrastnik's rendering path" with at-least 0.8 confidence.
  uses the @Function matching "π-radians to degrees" with at-least 0.8 confidence.

  favor correctness.
"#;
        let parsed = parse_nom(src).expect("UTF-8 in matching clauses must parse");
        let json = serde_json::to_string(&parsed).expect("must serialize");
        for needle in [
            "こんにちは greeting flow",
            "Blaž Hrastnik",
            "π-radians to degrees",
        ] {
            assert!(
                json.contains(needle),
                "UTF-8 matching string {needle:?} must survive verbatim; json = {json}"
            );
        }
    }

    /// ct10d: sanity — `the function login_user matching "x"` (v1 with kind)
    /// and `the @Function matching "x"` (v2 typed-slot) both parse cleanly.
    #[test]
    fn ct10d_kind_bearing_entity_refs_parse() {
        let v1_src = r#"
the concept ct10d_v1 is
  intended to smoke-test v1 kind-bearing ref.

  uses the function login_user matching "auth flow".

  favor correctness.
"#;
        let v2_src = r#"
the concept ct10d_v2 is
  intended to smoke-test v2 typed-slot ref.

  uses the @Function matching "auth flow".

  favor correctness.
"#;
        parse_nom(v1_src).expect("v1 kind-bearing ref must parse");
        parse_nom(v2_src).expect("v2 typed-slot ref must parse");
    }
}
