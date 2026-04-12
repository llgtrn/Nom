//! `.nomx` natural-language tokenizer prototype (proposal 05).
//!
//! First concrete code toward the ≥95%-prose grammar track proposed
//! in [research/language-analysis/05-natural-language-syntax.md]. Does
//! NOT modify the existing C-like lexer; coexists alongside as an
//! experimental surface that the parser will learn to consume next.
//!
//! Scope today:
//!   - recognize declaration verbs: `define`, `to`, `record`, `choice`
//!   - recognize control flow: `when`, `unless`, `otherwise`, `for`,
//!     `while`
//!   - recognize linking verbs: `that`, `is`, `takes`, `returns`,
//!     `holds`, `means`
//!   - recognize prepositional operators: `of`, `from`, `with`, `to`,
//!     `then`, `by`, `and`, `or`, `not`
//!   - strip article words at lex time (`a`, `an`, `the`, `that`
//!     when used as article — note: `that` is also a linking verb;
//!     grammar context disambiguates in phase 2)
//!   - preserve identifiers, numbers, strings, punctuation
//!
//! Out of scope: parsing, AST, grammar disambiguation, Vietnamese
//! aliases. All phase-2+ per proposal 05.

/// `.nomx` token kind. Kept separate from [`crate::Token`] so the two
/// grammars can evolve independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NomxToken {
    // Declaration verbs
    Define,
    To,
    Record,
    Choice,

    // Control flow
    When,
    Unless,
    Otherwise,
    For,
    While,

    // Linking verbs
    That,
    Is,
    Takes,
    Returns,
    Holds,
    Means,

    // Prepositional operators
    Of,
    From,
    With,
    ToPrep, // `to` as preposition; disambiguated at parse time from To-decl
    Then,
    By,
    And,
    Or,
    Not,

    // Literals + identifiers
    Identifier(String),
    Number(String),
    StringLit(String),

    // Punctuation
    Colon,
    Comma,
    Period,
    LParen,
    RParen,
    LBrace,
    RBrace,

    Eof,
}

/// A half-open byte range `[start, end)` into the source string.
/// Mirrors `nom_ast::Span` but kept local — the nomx grammar tracks
/// its own spans until the two lexers merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NomxSpan {
    pub start: usize,
    pub end: usize,
}

impl NomxSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Token paired with its source span. Preferred by downstream parser
/// work; `tokenize_nomx` wraps this for callers that only need the
/// bare token stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedNomxToken {
    pub token: NomxToken,
    pub span: NomxSpan,
}

/// Tokenize `source` into `NomxToken`s. Articles (`a`, `an`, `the`)
/// are stripped — they carry no semantic weight in the target grammar
/// (see proposal 05 §4.8). Thin wrapper over
/// [`tokenize_nomx_with_spans`]; drops the span info.
pub fn tokenize_nomx(source: &str) -> Vec<NomxToken> {
    tokenize_nomx_with_spans(source)
        .into_iter()
        .map(|s| s.token)
        .collect()
}

/// Same as [`tokenize_nomx`] but each token carries its source span.
/// Parser diagnostics + LSP hover/goto-def will consume this form.
///
/// Whitespace separates tokens; newlines and indentation are not
/// meaningful at this layer. Sentence boundary (`.`) and comma (`,`)
/// are preserved as structural punctuation.
pub fn tokenize_nomx_with_spans(source: &str) -> Vec<SpannedNomxToken> {
    let mut out: Vec<SpannedNomxToken> = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0usize;

    let push = |out: &mut Vec<SpannedNomxToken>, tok: NomxToken, start: usize, end: usize| {
        out.push(SpannedNomxToken {
            token: tok,
            span: NomxSpan::new(start, end),
        });
    };

    while i < bytes.len() {
        let c = bytes[i];
        let tok_start = i;
        match c {
            b' ' | b'\t' | b'\r' | b'\n' => {
                i += 1;
            }
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b':' => {
                i += 1;
                push(&mut out, NomxToken::Colon, tok_start, i);
            }
            b',' => {
                i += 1;
                push(&mut out, NomxToken::Comma, tok_start, i);
            }
            b'.' => {
                i += 1;
                push(&mut out, NomxToken::Period, tok_start, i);
            }
            b'(' => {
                i += 1;
                push(&mut out, NomxToken::LParen, tok_start, i);
            }
            b')' => {
                i += 1;
                push(&mut out, NomxToken::RParen, tok_start, i);
            }
            b'{' => {
                i += 1;
                push(&mut out, NomxToken::LBrace, tok_start, i);
            }
            b'}' => {
                i += 1;
                push(&mut out, NomxToken::RBrace, tok_start, i);
            }
            b'"' => {
                i += 1;
                let content_start = i;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
                let lit = std::str::from_utf8(&bytes[content_start..i])
                    .unwrap_or("")
                    .to_string();
                if i < bytes.len() {
                    i += 1;
                }
                push(&mut out, NomxToken::StringLit(lit), tok_start, i);
            }
            c if c.is_ascii_digit() => {
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                let s = std::str::from_utf8(&bytes[tok_start..i])
                    .unwrap_or("")
                    .to_string();
                push(&mut out, NomxToken::Number(s), tok_start, i);
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
                {
                    i += 1;
                }
                let word = std::str::from_utf8(&bytes[tok_start..i]).unwrap_or("");
                if let Some(tok) = keyword_token(word) {
                    push(&mut out, tok, tok_start, i);
                } else if is_article(word) {
                    // Stripped per §4.8.
                } else {
                    push(&mut out, NomxToken::Identifier(word.to_string()), tok_start, i);
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    push(&mut out, NomxToken::Eof, i, i);
    out
}

/// Canonical keyword set from proposal 06.
fn keyword_token(word: &str) -> Option<NomxToken> {
    match word {
        "define" => Some(NomxToken::Define),
        "to" => Some(NomxToken::To),
        "record" => Some(NomxToken::Record),
        "choice" => Some(NomxToken::Choice),
        "when" => Some(NomxToken::When),
        "unless" => Some(NomxToken::Unless),
        "otherwise" => Some(NomxToken::Otherwise),
        "for" => Some(NomxToken::For),
        "while" => Some(NomxToken::While),
        "that" => Some(NomxToken::That),
        "is" => Some(NomxToken::Is),
        "takes" => Some(NomxToken::Takes),
        "returns" => Some(NomxToken::Returns),
        "holds" => Some(NomxToken::Holds),
        "means" => Some(NomxToken::Means),
        "of" => Some(NomxToken::Of),
        "from" => Some(NomxToken::From),
        "with" => Some(NomxToken::With),
        "then" => Some(NomxToken::Then),
        "by" => Some(NomxToken::By),
        "and" => Some(NomxToken::And),
        "or" => Some(NomxToken::Or),
        "not" => Some(NomxToken::Not),
        _ => None,
    }
}

fn is_article(word: &str) -> bool {
    matches!(word, "a" | "an" | "the" | "which" | "who" | "whose")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_yields_only_eof() {
        assert_eq!(tokenize_nomx(""), vec![NomxToken::Eof]);
    }

    #[test]
    fn define_sentence_tokenizes() {
        let src = "define greet that takes a name and returns a greeting:";
        let toks = tokenize_nomx(src);
        use NomxToken::*;
        assert_eq!(
            toks,
            vec![
                Define,
                Identifier("greet".to_string()),
                That,
                Takes,
                Identifier("name".to_string()), // `a` stripped
                And,
                Returns,
                Identifier("greeting".to_string()), // `a` stripped
                Colon,
                Eof,
            ]
        );
    }

    #[test]
    fn articles_are_stripped() {
        let toks = tokenize_nomx("the user is a person");
        use NomxToken::*;
        assert_eq!(
            toks,
            vec![
                // `the` stripped
                Identifier("user".to_string()),
                Is,
                // `a` stripped
                Identifier("person".to_string()),
                Eof,
            ]
        );
    }

    #[test]
    fn when_otherwise_branches_tokenize() {
        let toks = tokenize_nomx("when the user is logged in, show the dashboard. otherwise, show the landing page.");
        assert!(toks.contains(&NomxToken::When));
        assert!(toks.contains(&NomxToken::Otherwise));
        assert!(toks.contains(&NomxToken::Period));
        assert!(toks.contains(&NomxToken::Comma));
    }

    #[test]
    fn string_literal_preserved() {
        let toks = tokenize_nomx("respond with \"hello\"");
        assert!(matches!(
            toks.iter().find(|t| matches!(t, NomxToken::StringLit(_))),
            Some(NomxToken::StringLit(s)) if s == "hello"
        ));
    }

    #[test]
    fn prepositional_operators_tokenize() {
        let src = "the greeting of the user with not a name from the list";
        let toks = tokenize_nomx(src);
        use NomxToken::*;
        assert!(toks.contains(&Of));
        assert!(toks.contains(&With));
        assert!(toks.contains(&Not));
        assert!(toks.contains(&From));
    }

    #[test]
    fn hello_nomx_sample_tokenizes_expected_shape() {
        // Loads examples/hello.nomx and asserts the token stream
        // contains the canonical declaration-form shape:
        //   Define Identifier That Takes Identifier And Returns
        //   Identifier Colon ... StringLit ...
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/hello.nomx");
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let toks = tokenize_nomx(&src);
        use NomxToken::*;
        // Declaration prefix: Define <name> That Takes <param> And Returns <ret>
        let idx_define = toks.iter().position(|t| *t == Define).unwrap();
        assert!(matches!(&toks[idx_define + 1], Identifier(n) if n == "greet"));
        assert_eq!(toks[idx_define + 2], That);
        assert_eq!(toks[idx_define + 3], Takes);
        assert!(matches!(&toks[idx_define + 4], Identifier(n) if n == "name"));
        assert_eq!(toks[idx_define + 5], And);
        assert_eq!(toks[idx_define + 6], Returns);
        assert!(matches!(&toks[idx_define + 7], Identifier(n) if n == "greeting"));
        assert_eq!(toks[idx_define + 8], Colon);
        // Body contains the string literal and the `is` linking verb.
        assert!(toks.iter().any(|t| matches!(t, StringLit(s) if s == "hello ")));
        assert!(toks.contains(&Is));
    }

    #[test]
    fn spans_point_at_source_bytes() {
        let src = "define greet";
        let spanned = tokenize_nomx_with_spans(src);
        // Find Define + Identifier; their spans must slice to the
        // matching substrings in the original source.
        let define_tok = spanned.iter().find(|s| s.token == NomxToken::Define).unwrap();
        assert_eq!(&src[define_tok.span.start..define_tok.span.end], "define");
        let ident = spanned
            .iter()
            .find(|s| matches!(&s.token, NomxToken::Identifier(n) if n == "greet"))
            .unwrap();
        assert_eq!(&src[ident.span.start..ident.span.end], "greet");
        // Eof span points at source length.
        let eof = spanned.last().unwrap();
        assert_eq!(eof.token, NomxToken::Eof);
        assert_eq!(eof.span.start, src.len());
        assert_eq!(eof.span.end, src.len());
    }

    #[test]
    fn comment_line_is_skipped() {
        let toks = tokenize_nomx("# a comment\ndefine x");
        use NomxToken::*;
        assert_eq!(
            toks,
            vec![Define, Identifier("x".to_string()), Eof]
        );
    }
}
