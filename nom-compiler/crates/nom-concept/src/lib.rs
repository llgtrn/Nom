//! Tier-1 (`.nomtu`) and Tier-2 (`.nom`) file-format types per
//! `research/language-analysis/08-layered-concept-component-architecture.md` §6.
//!
//! `.nomtu` = multi-entity DB2 container (small scope).
//! `.nom`   = multi-concept DB1 container (big scope).
//!
//! This crate defines the AST + parser for both formats.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod closure;
pub use closure::{ClosureError, ConceptClosure, ConceptGraph, UnresolvedRef};

pub mod mece;
pub use mece::{MeceReport, MeCollision, ObjectiveBinding, check_mece, stub_axis_of};

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

            // Bare word / keyword token.
            //
            // Accepted character classes:
            //   • ASCII lowercase a–z
            //   • ASCII digit 0–9
            //   • ASCII underscore _
            //   • Vietnamese lowercase letters with diacritics (the closed set used
            //     by keyword aliases below).  Multi-word Vietnamese keywords use
            //     underscore-joining (e.g. `bảo_đảm`, NOT `bảo đảm`).  This is a
            //     deliberate design choice: programming languages have always used
            //     delimiter characters between identifier-position tokens, and the
            //     underscore form is unambiguous to lex without whitespace lookahead.
            //
            // CONSTRAINT: Vietnamese diacritics are accepted ONLY in keyword tokens.
            //   Function / word names MUST remain pure ASCII.  This is enforced at
            //   the parser level: after lexing, `Tok::Word` carries whatever raw
            //   string the scanner produced.  If a caller uses a diacritic name the
            //   lexer will produce a Word token containing diacritic characters.
            //   The parser's `expect_word` accepts any Word, but the test suite
            //   (test #7 `unicode_only_in_keywords_function_names_stay_ascii`)
            //   documents and verifies that diacritic words that are NOT in the
            //   keyword table produce an UnknownKind / ParseError rather than a
            //   well-formed declaration.
            //
            // We use char-based iteration (not byte-based) so that multi-byte
            // UTF-8 codepoints advance `self.pos` by the correct byte count.
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
                // Vietnamese ASCII-transliteration aliases (motivation 02 locale packs).
                // Maps canonical VN ASCII forms to the same Tok variants as English
                // keywords so the parser is completely unaware of the source language.
                //
                // `cai` (classifier "cái") is the article alias for `the`.
                // Vietnamese has no direct equivalent of the English article; `cai`
                // acts as the noun-classifier that precedes kind nouns, mirroring how
                // `the` precedes kind nouns in English entity references.
                //
                // `khi` and `muc_dich` are compressed multi-word aliases expanded
                // at lex time: `khi` → This Works When; `muc_dich` → Intended To.
                // Expansion is local here so the parser needs no changes.
                //
                // Vietnamese diacritic forms (NEW): same keyword mapping as ASCII
                // aliases above, but written with native diacritics.  The underscore
                // join rule applies identically: `bảo_đảm`, `kết_hợp`, etc.
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
                    "works"    => Tok::Works,
                    "when"     => Tok::When,
                    "favor"    => Tok::Favor,
                    // ── Vietnamese ASCII-transliteration aliases ──────────────
                    "cai"      => Tok::The,       // cái  → the  (classifier article)
                    "la"       => Tok::Is,        // là   → is
                    "ket_hop"  => Tok::Composes,  // kết hợp  → composes
                    "roi"      => Tok::Then,      // rồi → then
                    "voi"      => Tok::With,      // với → with
                    "can"      => Tok::Requires,  // cần → requires
                    "bao_dam"  => Tok::Ensures,   // bảo đảm → ensures
                    "khop"     => Tok::Matching,  // khớp → matching
                    "dung"     => Tok::Uses,      // dùng → uses
                    "mo_rong"  => Tok::Extends,   // mở rộng → extends
                    "them"     => Tok::Adding,    // thêm → adding
                    "bot"      => Tok::Removing,  // bớt → removing
                    "bay_ra"   => Tok::Exposes,   // bày ra → exposes
                    "uu_tien"  => Tok::Favor,     // ưu tiên → favor
                    // khi (khi) → this works when  [lexer-side expansion]
                    "khi" => {
                        self.pending.push(Spanned { tok: Tok::Works, pos: start });
                        self.pending.push(Spanned { tok: Tok::When,  pos: start });
                        Tok::This
                    }
                    // muc_dich (mục đích) → intended to  [lexer-side expansion]
                    "muc_dich" => {
                        self.pending.push(Spanned { tok: Tok::To, pos: start });
                        Tok::Intended
                    }
                    // ── Vietnamese diacritic keyword aliases ─────────────────
                    // Each diacritic form maps to the exact same Tok variant as
                    // its ASCII counterpart.  Only KEYWORDS get diacritic forms;
                    // function/word names must stay ASCII (see constraint above).
                    "là"       => Tok::Is,        // là   → is   (= la)
                    "cái"      => Tok::The,       // cái  → the  (= cai)
                    "cần"      => Tok::Requires,  // cần  → requires  (= can)
                    "bảo_đảm"  => Tok::Ensures,   // bảo đảm → ensures  (= bao_dam)
                    "kết_hợp"  => Tok::Composes,  // kết hợp → composes  (= ket_hop)
                    "rồi"      => Tok::Then,      // rồi  → then  (= roi)
                    "với"      => Tok::With,      // với  → with  (= voi)
                    "khớp"     => Tok::Matching,  // khớp → matching  (= khop)
                    "dùng"     => Tok::Uses,      // dùng → uses  (= dung)
                    "mở_rộng"  => Tok::Extends,   // mở rộng → extends  (= mo_rong)
                    "thêm"     => Tok::Adding,    // thêm → adding  (= them)
                    "bớt"      => Tok::Removing,  // bớt  → removing  (= bot)
                    "bày_ra"   => Tok::Exposes,   // bày ra → exposes  (= bay_ra)
                    "ưu_tiên"  => Tok::Favor,     // ưu tiên → favor  (= uu_tien)
                    // mục_đích → intended to  [lexer-side expansion; = muc_dich]
                    "mục_đích" => {
                        self.pending.push(Spanned { tok: Tok::To, pos: start });
                        Tok::Intended
                    }
                    // ── Kind nouns (English) ─────────────────────────────────
                    "function" | "module" | "concept" | "screen"
                    | "data"   | "event"  | "media"   => Tok::Kind(word.to_string()),
                    // ── Kind nouns (Vietnamese ASCII aliases) ────────────────
                    "ham"         => Tok::Kind("function".to_string()), // hàm
                    "mo_dun"      => Tok::Kind("module".to_string()),   // mô đun
                    "khai_niem"   => Tok::Kind("concept".to_string()),  // khái niệm
                    "man_hinh"    => Tok::Kind("screen".to_string()),   // màn hình
                    "du_lieu"     => Tok::Kind("data".to_string()),     // dữ liệu
                    "su_kien"     => Tok::Kind("event".to_string()),    // sự kiện
                    "phuong_tien" => Tok::Kind("media".to_string()),    // phương tiện
                    // ── Kind nouns (Vietnamese diacritic aliases) ────────────
                    "hàm"         => Tok::Kind("function".to_string()), // hàm  (= ham)
                    "mô_đun"      => Tok::Kind("module".to_string()),   // mô đun  (= mo_dun)
                    "khái_niệm"   => Tok::Kind("concept".to_string()),  // khái niệm  (= khai_niem)
                    "màn_hình"    => Tok::Kind("screen".to_string()),   // màn hình  (= man_hinh)
                    "dữ_liệu"     => Tok::Kind("data".to_string()),     // dữ liệu  (= du_lieu)
                    "sự_kiện"     => Tok::Kind("event".to_string()),    // sự kiện  (= su_kien)
                    "phương_tiện" => Tok::Kind("media".to_string()),    // phương tiện  (= phuong_tien)
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
    /// Accepted:
    ///   • ASCII lowercase a–z
    ///   • ASCII underscore _  (e.g. `_reserved`)
    ///   • Vietnamese lowercase letters with diacritics (closed set used by
    ///     the diacritic keyword aliases above).
    ///
    /// Rejected: ASCII uppercase, digits at start, other Unicode letters.
    pub fn is_word_start_char(c: char) -> bool {
        c.is_ascii_lowercase() || c == '_' || is_vietnamese_diacritic_lower(c)
    }

    /// Returns `true` if `c` may continue an identifier / keyword token.
    ///
    /// Same set as `is_word_start_char` plus ASCII digits 0–9.
    pub fn is_word_continue_char(c: char) -> bool {
        c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit() || is_vietnamese_diacritic_lower(c)
    }

    /// Returns `true` for exactly the Vietnamese lowercase diacritic characters
    /// that appear in the closed keyword alias set.  This is an explicit
    /// whitelist — we do NOT use a Unicode category check such as
    /// `c.is_alphabetic()` because:
    ///   (a) We want to keep the set small and auditable.
    ///   (b) We do not want to silently accept arbitrary Unicode letters in
    ///       identifiers (function names must stay ASCII).
    ///   (c) We have no `unicode-xid` dependency and want to avoid adding one.
    ///
    /// The set covers every diacritic codepoint that appears in the keyword
    /// table: là / cái / cần / bảo_đảm / kết_hợp / rồi / với / khớp / dùng /
    /// mở_rộng / thêm / bớt / bày_ra / ưu_tiên / mục_đích / hàm / mô_đun /
    /// khái_niệm / màn_hình / dữ_liệu / sự_kiện / phương_tiện.
    #[inline]
    fn is_vietnamese_diacritic_lower(c: char) -> bool {
        matches!(
            c,
            // à á â ã ä å – base `a` with tone/diacritic
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' |
            // ả ã ạ ặ ấ ầ ẩ ẫ ậ ắ ằ ẳ ẵ ặ
            'ả' | 'ạ' | 'ặ' | 'ấ' | 'ầ' | 'ẩ' | 'ẫ' | 'ậ' | 'ắ' | 'ằ' | 'ẳ' | 'ẵ' |
            // è é ê – base `e`
            'è' | 'é' | 'ê' |
            'ẻ' | 'ẽ' | 'ẹ' | 'ế' | 'ề' | 'ể' | 'ễ' | 'ệ' |
            // ì í – base `i`
            'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' |
            // ò ó ô õ – base `o`
            'ò' | 'ó' | 'ô' | 'õ' |
            'ỏ' | 'ọ' | 'ố' | 'ồ' | 'ổ' | 'ỗ' | 'ộ' | 'ớ' | 'ờ' | 'ở' | 'ỡ' | 'ợ' |
            // ơ – base `o` with horn (U+01A1)
            'ơ' |
            // ù ú – base `u`
            'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ứ' | 'ừ' | 'ử' | 'ữ' | 'ự' |
            // ư – base `u` with horn (U+01B0)
            'ư' |
            // ỳ ý – base `y`
            'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' |
            // đ – d with stroke (U+0111)
            'đ'
        )
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
            Tok::Intended => "`intended`".into(),
            Tok::To       => "`to`".into(),
            Tok::Uses     => "`uses`".into(),
            Tok::Extends  => "`extends`".into(),
            Tok::Adding   => "`adding`".into(),
            Tok::Removing => "`removing`".into(),
            Tok::Exposes  => "`exposes`".into(),
            Tok::This     => "`this`".into(),
            Tok::Works    => "`works`".into(),
            Tok::When     => "`when`".into(),
            Tok::Favor    => "`favor`".into(),
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
            Tok::Intended => "intended".to_string(),
            Tok::To       => "to".to_string(),
            Tok::Uses     => "uses".to_string(),
            Tok::Extends  => "extends".to_string(),
            Tok::Adding   => "adding".to_string(),
            Tok::Removing => "removing".to_string(),
            Tok::Exposes  => "exposes".to_string(),
            Tok::This     => "this".to_string(),
            Tok::Works    => "works".to_string(),
            Tok::When     => "when".to_string(),
            Tok::Favor    => "favor".to_string(),
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

    // ── Vietnamese alias tests ───────────────────────────────────────────────

    /// vn01: `cai ham foo la "x" .` lexes as the same token sequence as
    /// `the function foo is "x" .`  (raw Tok comparison, no parser involved).
    #[test]
    fn vn_alias_la_lexes_as_is() {
        use super::lex::{Lexer, Tok};

        fn lex_all(src: &str) -> Vec<Tok> {
            let mut l = Lexer::new(src);
            let mut out = Vec::new();
            while let Some(s) = l.next() {
                out.push(s.tok);
            }
            out
        }

        let english    = lex_all(r#"the function foo is "x" ."#);
        let vietnamese = lex_all(r#"cai ham foo la "x" ."#);
        assert_eq!(english, vietnamese, "VN aliases must produce identical token stream");
    }

    /// vn02: full Vietnamese entity declaration parses correctly.
    #[test]
    fn vn_alias_full_entity_decl_parses() {
        // `cai ham validate_token la given a token of text, returns yes or no.`
        // `can the token is non-empty.`
        let src = r#"cai ham validate_token la given a token of text, returns yes or no.
can the token is non-empty."#;
        let f = parse_nomtu(src).expect("VN entity decl should parse");
        assert_eq!(f.items.len(), 1);
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.kind, "function");
                assert_eq!(e.word, "validate_token");
                assert_eq!(e.contracts.len(), 1);
                assert!(matches!(&e.contracts[0], ContractClause::Requires(_)));
            }
            _ => panic!("expected Entity"),
        }
    }

    /// vn03: Vietnamese composition keywords parse correctly.
    #[test]
    fn vn_alias_composition_parses() {
        let src = r#"cai mo_dun foo ket_hop cai ham bar roi cai ham baz voi "glue" ."#;
        let f = parse_nomtu(src).expect("VN composition should parse");
        assert_eq!(f.items.len(), 1);
        match &f.items[0] {
            NomtuItem::Composition(c) => {
                assert_eq!(c.word, "foo");
                assert_eq!(c.composes.len(), 2);
                assert_eq!(c.composes[0].word, "bar");
                assert_eq!(c.composes[0].kind.as_deref(), Some("function"));
                assert_eq!(c.composes[1].word, "baz");
                assert_eq!(c.composes[1].kind.as_deref(), Some("function"));
                assert_eq!(c.glue.as_deref(), Some("glue"));
            }
            _ => panic!("expected Composition"),
        }
    }

    /// vn04: full Vietnamese concept with `muc_dich`, `dung`, and `uu_tien`.
    #[test]
    fn vn_alias_concept_with_uses_and_favor() {
        let src = r#"
cai khai_niem auth la muc_dich let users in.
  dung cai ham login.
  uu_tien security roi speed.
"#;
        let f = parse_nom(src).expect("VN concept should parse");
        assert_eq!(f.concepts.len(), 1);
        let c = &f.concepts[0];
        assert_eq!(c.name, "auth");
        assert_eq!(c.intent, "let users in");
        assert_eq!(c.index.len(), 1);
        match &c.index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 1);
                assert_eq!(refs[0].word, "login");
                assert_eq!(refs[0].kind.as_deref(), Some("function"));
            }
            _ => panic!("expected Uses"),
        }
        assert_eq!(c.objectives, vec!["security", "speed"]);
    }

    /// vn05: `khi` expands to `This Works When` — acceptance predicate is captured.
    #[test]
    fn vn_khi_compresses_to_this_works_when() {
        use super::lex::{Lexer, Tok};

        let src = "khi the user is logged in.";
        let mut l = Lexer::new(src);
        let toks: Vec<Tok> = {
            let mut out = Vec::new();
            while let Some(s) = l.next() {
                out.push(s.tok);
            }
            out
        };
        // First three tokens must be This, Works, When
        assert!(toks.len() >= 3, "expected at least 3 tokens");
        assert_eq!(toks[0], Tok::This);
        assert_eq!(toks[1], Tok::Works);
        assert_eq!(toks[2], Tok::When);
        // Also verify full parse works
        let src2 = r#"
cai khai_niem foo la muc_dich do things.
  dung cai ham bar.
  khi something happens.
"#;
        let f = parse_nom(src2).expect("khi acceptance clause should parse");
        let c = &f.concepts[0];
        assert_eq!(c.acceptance.len(), 1);
        assert!(c.acceptance[0].contains("something"));
    }

    /// vn06: mixed English and Vietnamese keywords in the same concept both parse.
    #[test]
    fn mixed_english_vietnamese_in_one_concept() {
        let src = r#"
the concept mixed_auth is
  intended to test locale mixing.

  uses the function foo, cai ham bar.

  favor security.
"#;
        let f = parse_nom(src).expect("mixed VN+EN concept should parse");
        let c = &f.concepts[0];
        assert_eq!(c.index.len(), 1);
        match &c.index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 2);
                assert_eq!(refs[0].word, "foo");
                assert_eq!(refs[0].kind.as_deref(), Some("function"));
                assert_eq!(refs[1].word, "bar");
                assert_eq!(refs[1].kind.as_deref(), Some("function"));
            }
            _ => panic!("expected Uses"),
        }
    }

    /// vn07: parse a Vietnamese concept, serialize the AST, deserialize, assert
    /// the shape is equivalent (round-trip through serde_json).
    #[test]
    fn vn_aliases_round_trip_through_serde() {
        let src = r#"
cai khai_niem session_auth la muc_dich authenticate user sessions.
  dung cai ham issue_token.
  khi valid credentials are provided.
  uu_tien security roi speed.
"#;
        let f = parse_nom(src).expect("VN concept should parse");
        let json = serde_json::to_string(&f).expect("should serialize");
        let back: NomFile = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(f, back);
        // Spot-check field values survive the round-trip.
        assert_eq!(back.concepts[0].name, "session_auth");
        assert_eq!(back.concepts[0].objectives, vec!["security", "speed"]);
        assert_eq!(back.concepts[0].acceptance.len(), 1);
    }

    /// vn08: small Vietnamese agent demo — one concept declaration parses without error.
    #[test]
    fn agent_demo_in_vietnamese_parses() {
        let src = r#"
cai khai_niem tac_nhan_don_gian la muc_dich compose a small set of tools safely.

  dung cai ham doc_file,
       cai ham ghi_file,
       cai ham lay_url.

  bay_ra doc_file, ghi_file, lay_url.

  khi the safety policy is composed.
  khi every tool has at least one contract.

  uu_tien bao_mat roi toc_do.
"#;
        let f = parse_nom(src).expect("Vietnamese agent demo should parse");
        assert_eq!(f.concepts.len(), 1);
        let c = &f.concepts[0];
        assert_eq!(c.name, "tac_nhan_don_gian");
        assert_eq!(c.index.len(), 1);
        match &c.index[0] {
            IndexClause::Uses(refs) => assert_eq!(refs.len(), 3),
            _ => panic!("expected Uses"),
        }
        assert_eq!(c.exposes.len(), 3);
        assert_eq!(c.acceptance.len(), 2);
        assert_eq!(c.objectives, vec!["bao_mat", "toc_do"]);
    }

    // ── Vietnamese diacritic keyword tests ──────────────────────────────────

    /// vnd1: `cái hàm foo là "x".` lexes identically to `cai ham foo la "x".`
    /// which in turn is identical to `the function foo is "x".`.
    #[test]
    fn vn_diacritic_la_lexes_as_is() {
        use super::lex::{Lexer, Tok};

        fn lex_all(src: &str) -> Vec<Tok> {
            let mut l = Lexer::new(src);
            let mut out = Vec::new();
            while let Some(s) = l.next() {
                out.push(s.tok);
            }
            out
        }

        let english    = lex_all(r#"the function foo is "x" ."#);
        let vn_ascii   = lex_all(r#"cai ham foo la "x" ."#);
        let vn_diacritic = lex_all(r#"cái hàm foo là "x" ."#);
        assert_eq!(english, vn_ascii,    "ASCII VN must match English token stream");
        assert_eq!(english, vn_diacritic, "diacritic VN must match English token stream");
    }

    /// vnd2: a concept written entirely in diacritic VN parses to one ConceptDecl
    /// with intent captured, one Uses clause, and two objectives.
    #[test]
    fn vn_diacritic_full_concept_parses() {
        let src = r#"
cái khái_niệm auth là mục_đích let users in.
  dùng cái hàm login.
  ưu_tiên security rồi speed.
"#;
        let f = parse_nom(src).expect("full diacritic VN concept should parse");
        assert_eq!(f.concepts.len(), 1);
        let c = &f.concepts[0];
        assert_eq!(c.name, "auth");
        assert_eq!(c.intent, "let users in");
        assert_eq!(c.index.len(), 1);
        match &c.index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 1);
                assert_eq!(refs[0].word, "login");
                assert_eq!(refs[0].kind.as_deref(), Some("function"));
            }
            _ => panic!("expected Uses clause"),
        }
        assert_eq!(c.objectives, vec!["security", "speed"]);
    }

    /// vnd3: composition written with diacritic keywords parses correctly.
    #[test]
    fn vn_diacritic_composition_parses() {
        let src = r#"cái mô_đun foo kết_hợp cái hàm bar rồi cái hàm baz với "glue"."#;
        let f = parse_nomtu(src).expect("diacritic VN composition should parse");
        assert_eq!(f.items.len(), 1);
        match &f.items[0] {
            NomtuItem::Composition(c) => {
                assert_eq!(c.word, "foo");
                assert_eq!(c.composes.len(), 2);
                assert_eq!(c.composes[0].word, "bar");
                assert_eq!(c.composes[0].kind.as_deref(), Some("function"));
                assert_eq!(c.composes[1].word, "baz");
                assert_eq!(c.composes[1].kind.as_deref(), Some("function"));
                assert_eq!(c.glue.as_deref(), Some("glue"));
            }
            _ => panic!("expected Composition"),
        }
    }

    /// vnd4: entity with both `cần` (requires) and `bảo_đảm` (ensures) contract clauses.
    #[test]
    fn vn_diacritic_contracts() {
        let src = r#"cái hàm check_token là given a token, returns ok.
cần the token is non-empty.
bảo_đảm the result is valid."#;
        let f = parse_nomtu(src).expect("diacritic contract clauses should parse");
        assert_eq!(f.items.len(), 1);
        match &f.items[0] {
            NomtuItem::Entity(e) => {
                assert_eq!(e.contracts.len(), 2);
                assert!(matches!(&e.contracts[0], ContractClause::Requires(_)));
                assert!(matches!(&e.contracts[1], ContractClause::Ensures(_)));
            }
            _ => panic!("expected Entity"),
        }
    }

    /// vnd5: `mục_đích` expands to the Intended To token sequence (mirrors
    /// existing `muc_dich` test for `vn_alias_concept_with_uses_and_favor`).
    #[test]
    fn vn_diacritic_muc_dich_compresses_correctly() {
        use super::lex::{Lexer, Tok};

        let src = "mục_đích do things.";
        let mut l = Lexer::new(src);
        let toks: Vec<Tok> = {
            let mut out = Vec::new();
            while let Some(s) = l.next() {
                out.push(s.tok);
            }
            out
        };
        // First two tokens must be Intended, To (same as muc_dich expansion).
        assert!(toks.len() >= 2, "expected at least 2 tokens");
        assert_eq!(toks[0], Tok::Intended, "mục_đích must expand to Intended first");
        assert_eq!(toks[1], Tok::To,       "mục_đích must expand to To second");
    }

    /// vnd6: same document uses BOTH ASCII `cai ham` AND diacritic `cái hàm`.
    /// Both parse correctly — the lexer is form-agnostic.
    #[test]
    fn mixed_ascii_and_diacritic_in_one_doc() {
        let src = r#"
cái khái_niệm mixed_forms là mục_đích test form agnosticism.
  dùng cái hàm foo,
       cai ham bar.
  ưu_tiên correctness.
"#;
        let f = parse_nom(src).expect("mixed ASCII + diacritic concept should parse");
        let c = &f.concepts[0];
        assert_eq!(c.name, "mixed_forms");
        assert_eq!(c.index.len(), 1);
        match &c.index[0] {
            IndexClause::Uses(refs) => {
                assert_eq!(refs.len(), 2);
                assert_eq!(refs[0].word, "foo");
                assert_eq!(refs[1].word, "bar");
                for r in refs {
                    assert_eq!(r.kind.as_deref(), Some("function"));
                }
            }
            _ => panic!("expected Uses"),
        }
        assert_eq!(c.objectives, vec!["correctness"]);
    }

    /// vnd7: a function name written with Vietnamese diacritics (`xác_thực`)
    /// must FAIL — function names must stay ASCII.  The diacritic name is not
    /// in the keyword table so it becomes a `Tok::Word`; the `the` / `cái`
    /// article + kind consume fine, but `expect_word` accepts the diacritic
    /// word as the identifier.  Then `expect_is` / `expect_kind` will fail
    /// because the next token won't be `is` — instead we'll see the `là` token
    /// (or end-of-input), not an `is`.
    ///
    /// This test demonstrates the enforcement path: if you use a VN diacritic
    /// name as the function word, the parse fails at the `is` position.
    #[test]
    fn unicode_only_in_keywords_function_names_stay_ascii() {
        // `xác_thực` is not a keyword so it becomes a bare Word token.
        // The parser tries `the function <word> is ...`; it reads `xác_thực`
        // as the word, then expects `is` but finds end-of-input (or garbage)
        // → ParseError.
        let src = r#"cái hàm xác_thực là given a user, returns ok."#;
        // `xác_thực` is not a keyword.  The lexer will produce:
        //   Tok::The  Tok::Kind("function")  Tok::Word("xác_thực")  Tok::Is ...
        // BUT `x` is ASCII so `xác_thực` starts with `x` — the scanner will
        // stop at `á` (not a word-continue byte for ASCII) and produce
        // Tok::Word("x") then several individual diacritic-char Word tokens.
        // The parser will see Tok::Word("x") as the entity name, then the next
        // token is not Tok::Is → ParseError.
        match parse_nomtu(src) {
            Err(ConceptError::ParseError { .. }) => {
                // Expected: parser rejected the diacritic function name
            }
            Err(other) => panic!("expected ParseError, got {:?}", other),
            Ok(f) => panic!(
                "expected parse failure for diacritic function name, got {:?}", f
            ),
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
       the function fetch_url matching "fetch the body of an https url",
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
}
