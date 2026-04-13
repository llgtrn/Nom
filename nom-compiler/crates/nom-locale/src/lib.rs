//! `nom-locale` — BCP47 tag parsing + UAX #15 NFC + M3 locale packs (doc 09).

use std::collections::BTreeMap;
use unicode_normalization::UnicodeNormalization;

// ── BCP47 tag ─────────────────────────────────────────────────────────────────

/// A parsed BCP 47 language tag (M3a subset: language + optional script/region/variants).
///
/// Extensions (`u-`, `t-`) and private-use (`x-`) subtags are not expanded;
/// they are captured in `unsupported` if present (always `Some(...)` for M3a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleTag {
    /// ISO 639-1/2/3 language code, canonicalized to lowercase (2-3 ASCII alpha).
    pub language: String,
    /// ISO 15924 script code, canonicalized to title-case (4 ASCII alpha), if present.
    pub script: Option<String>,
    /// ISO 3166-1 region code, canonicalized to uppercase (2 ASCII alpha) or 3 ASCII digits.
    pub region: Option<String>,
    /// BCP 47 variant subtags (5-8 alphanum each).
    pub variants: Vec<String>,
    /// Raw tail of any unsupported extension or private-use subtags (`u-`, `t-`, `x-`).
    /// Populated in M3a; M3b will parse these properly.
    pub unsupported: Option<String>,
}

impl LocaleTag {
    /// Parse a BCP 47 tag string and return a canonical `LocaleTag`.
    ///
    /// Canonical form: lowercase language, title-case script, uppercase region.
    ///
    /// Returns `Err(ParseError)` for syntactically invalid tags.
    /// Tags with unsupported extensions are accepted but record the extension in
    /// `unsupported` (never rejected outright — the rest of the tag is still usable).
    pub fn parse(input: &str) -> Result<LocaleTag, ParseError> {
        if input.is_empty() {
            return Err(ParseError::Empty);
        }

        // Split on `-` (BCP 47 separator).
        let mut parts = input.split('-');

        // ── Language subtag (required) ────────────────────────────────────────
        let raw_lang = parts.next().unwrap_or("");
        if raw_lang.is_empty() {
            return Err(ParseError::Empty);
        }
        let lang_len = raw_lang.len();
        if lang_len < 2 || lang_len > 3 {
            return Err(ParseError::InvalidLanguage(raw_lang.to_string()));
        }
        if !raw_lang.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(ParseError::InvalidLanguage(raw_lang.to_string()));
        }
        // Reject purely numeric language codes.
        if raw_lang.chars().all(|c| c.is_ascii_digit()) {
            return Err(ParseError::InvalidLanguage(raw_lang.to_string()));
        }
        let language = raw_lang.to_ascii_lowercase();

        // ── Optional script + region + variants ──────────────────────────────
        let mut script: Option<String> = None;
        let mut region: Option<String> = None;
        let mut variants: Vec<String> = Vec::new();
        let mut unsupported_parts: Vec<String> = Vec::new();
        let mut in_extension = false;

        for subtag in parts {
            if subtag.is_empty() {
                return Err(ParseError::EmptySubtag);
            }

            let len = subtag.len();

            // Extension / private-use singleton: single ASCII letter or digit.
            if len == 1 && subtag.chars().next().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false) {
                in_extension = true;
                unsupported_parts.push(subtag.to_string());
                continue;
            }

            if in_extension {
                // Collect all subtags after a singleton as part of the extension.
                unsupported_parts.push(subtag.to_string());
                continue;
            }

            // Script subtag: exactly 4 ASCII alpha (title-case canonical).
            if script.is_none() && region.is_none() && variants.is_empty() && len == 4
                && subtag.chars().all(|c| c.is_ascii_alphabetic())
            {
                let mut s = subtag.to_ascii_lowercase();
                // Title-case: uppercase the first char.
                if let Some(first) = s.get_mut(0..1) {
                    first.make_ascii_uppercase();
                }
                script = Some(s);
                continue;
            }

            // Region subtag: 2 ASCII alpha OR 3 ASCII digits.
            if region.is_none() && variants.is_empty()
                && ((len == 2 && subtag.chars().all(|c| c.is_ascii_alphabetic()))
                    || (len == 3 && subtag.chars().all(|c| c.is_ascii_digit())))
            {
                region = Some(subtag.to_ascii_uppercase());
                continue;
            }

            // Variant subtag: 5-8 alphanum.
            if len >= 5 && len <= 8 && subtag.chars().all(|c| c.is_ascii_alphanumeric()) {
                variants.push(subtag.to_ascii_lowercase());
                continue;
            }

            // Also accept 4-char variant starting with a digit (BCP 47 allows this).
            if len == 4
                && subtag.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                && subtag.chars().all(|c| c.is_ascii_alphanumeric())
            {
                variants.push(subtag.to_ascii_lowercase());
                continue;
            }

            // Unrecognized subtag — treat as start of unsupported extension.
            in_extension = true;
            unsupported_parts.push(subtag.to_string());
        }

        let unsupported = if unsupported_parts.is_empty() {
            None
        } else {
            Some(unsupported_parts.join("-"))
        };

        Ok(LocaleTag {
            language,
            script,
            region,
            variants,
            unsupported,
        })
    }

    /// Render the canonical BCP 47 string (language[-Script][-REGION]).
    ///
    /// Variants and unsupported extensions are omitted from the canonical form
    /// for now (M3a scope).
    pub fn canonical(&self) -> String {
        let mut out = self.language.clone();
        if let Some(ref s) = self.script {
            out.push('-');
            out.push_str(s);
        }
        if let Some(ref r) = self.region {
            out.push('-');
            out.push_str(r);
        }
        out
    }
}

impl std::fmt::Display for LocaleTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.canonical())
    }
}

// ── Parse error ───────────────────────────────────────────────────────────────

/// Errors returned by [`LocaleTag::parse`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("locale tag must not be empty")]
    Empty,
    #[error("invalid language subtag: `{0}` (expected 2–3 ASCII alpha)")]
    InvalidLanguage(String),
    #[error("empty subtag (consecutive `-` separators)")]
    EmptySubtag,
}

// ── NFC normalizer ────────────────────────────────────────────────────────────

/// Normalize a string to Unicode NFC form (UAX #15 canonical composition).
///
/// Uses the `unicode-normalization` crate; allocation is proportional to input length.
pub fn normalize_nfc(s: &str) -> String {
    s.nfc().collect()
}

// ── LocalePack + RegisterMetadata ────────────────────────────────────────────

/// Metadata describing the provenance and license of a locale pack.
#[derive(Debug, Clone)]
pub struct RegisterMetadata {
    /// Human-readable display name for the locale (e.g. "Vietnamese (Vietnam)").
    pub display_name: String,
    /// Pack source identifier. `"baked:vi-v1"` for M3a baked-in stubs;
    /// later `"cldr:v45"` or `"corpus:..."`.
    pub source: String,
    /// SPDX license identifier for the pack data.
    pub license: String,
    /// Registration timestamp as `"epoch-<secs>"`.
    pub registered_at: String,
}

/// A registered locale pack: maps a BCP 47 locale to alias tables used by the
/// Nom parser/resolver for locale-aware keyword and identifier lookup.
#[derive(Debug, Clone)]
pub struct LocalePack {
    /// The BCP 47 locale this pack covers.
    pub id: LocaleTag,
    /// Localized keyword → canonical Nom keyword (e.g. `"là"` → `"is"`).
    /// Populated in M3c from the shipped lexer alias set.
    pub keyword_aliases: BTreeMap<String, String>,
    /// Localized identifier → canonical hash suffix.
    /// Empty in M3a; intended for M3c+ corpus-derived aliases.
    pub nom_aliases: BTreeMap<String, String>,
    /// Pack provenance and license metadata.
    pub register_metadata: RegisterMetadata,
}

// ── vi-VN keyword alias table (M3c) ──────────────────────────────────────────

/// vi-VN keyword_aliases is intentionally empty.
///
/// Per the user's load-bearing directive: Nom borrows Vietnamese GRAMMAR STYLE
/// (classifier phrases, modifier-after-head order, effect valence) but keeps
/// the vocabulary English. The lexer's existing Vietnamese keyword arms
/// (commits 4b04b1d + 5b59f82) are kept-but-not-extended; they are not
/// promoted into the locale pack's queryable alias table because doing so
/// would mislabel the locale pack's purpose.
///
/// Future grammar-style locale features (not keyword translations) go in
/// `LocalePack.register_metadata` or a new grammar-rules field, not here.
const VI_VN_KEYWORD_ALIASES: &[(&str, &str)] = &[];

// ── Built-in pack registry ────────────────────────────────────────────────────

/// Return all baked-in locale packs.
///
/// Contains two packs: `vi-VN` (Vietnamese, Vietnam) and `en-US` (English,
/// United States). Both have empty `keyword_aliases` on purpose: Nom's
/// vocabulary stays English; Vietnamese contributes GRAMMAR STYLE only
/// (classifiers, modifier-after-head, effect valence). Keyword translation
/// would contradict that directive.
pub fn builtin_packs() -> Vec<LocalePack> {
    let vi_vn_aliases: BTreeMap<String, String> = VI_VN_KEYWORD_ALIASES
        .iter()
        .map(|&(loc, canon)| (loc.to_string(), canon.to_string()))
        .collect();

    vec![
        LocalePack {
            id: LocaleTag::parse("vi-VN").expect("vi-VN is valid"),
            keyword_aliases: vi_vn_aliases,
            nom_aliases: BTreeMap::new(),
            register_metadata: RegisterMetadata {
                display_name: "Vietnamese (Vietnam)".to_string(),
                source: "baked:vi-v1-grammar-only".to_string(),
                license: "CC0-1.0".to_string(),
                registered_at: "epoch-0".to_string(),
            },
        },
        LocalePack {
            id: LocaleTag::parse("en-US").expect("en-US is valid"),
            keyword_aliases: BTreeMap::new(),
            nom_aliases: BTreeMap::new(),
            register_metadata: RegisterMetadata {
                display_name: "English (United States)".to_string(),
                source: "baked:en-v1-default".to_string(),
                license: "CC0-1.0".to_string(),
                registered_at: "epoch-0".to_string(),
            },
        },
    ]
}

// ── Confusable API (M3a stub) ─────────────────────────────────────────────────

/// Result of a confusable check between two strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfusableResult {
    /// Both inputs are identical (byte-equal after NFC).
    Equal,
    /// Both inputs differ and no confusable overlap was detected.
    /// (Safe to use together without ambiguity.)
    DifferentSafe,
    /// At least one confusable character pair was detected.
    /// Populated in M3b when UTS #39 confusables.txt data lands.
    Confusable {
        /// `(char in a, visually similar char in b)` pairs.
        pairs: Vec<(char, char)>,
    },
    /// M3a stub: UTS #39 data not yet loaded; check deferred to M3b.
    Deferred,
}

/// Check whether two strings contain visually confusable characters (UTS #39).
///
/// **M3a stub**: always returns [`ConfusableResult::Equal`] for identical inputs
/// and [`ConfusableResult::Deferred`] for all other inputs.  The full detector
/// (loading `confusables.txt` from Unicode.org) ships in M3b.
pub fn is_confusable(a: &str, b: &str) -> ConfusableResult {
    let a_nfc = normalize_nfc(a);
    let b_nfc = normalize_nfc(b);
    if a_nfc == b_nfc {
        ConfusableResult::Equal
    } else {
        ConfusableResult::Deferred
    }
}

// ── apply_locale — lexical keyword substitution (M3c) ────────────────────────

/// Direction of the apply_locale transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyDirection {
    /// Replace localized aliases with their canonical English keywords.
    /// e.g. `"cái hàm là"` → `"the function is"`.
    ToCanonical,
    /// Replace canonical English keywords with their localized aliases.
    /// Uses the diacritic form when both diacritic and ASCII forms map to the
    /// same canonical (diacritic form is listed first in `VI_VN_KEYWORD_ALIASES`).
    FromCanonical,
}

/// A single token substitution made during [`apply_locale`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    /// 1-based line number.
    pub line: usize,
    /// 1-based byte column.
    pub column: usize,
    /// The exact text that was replaced.
    pub from: String,
    /// The text it was replaced with.
    pub to: String,
}

/// Result of [`apply_locale`].
#[derive(Debug, Clone)]
pub struct ApplyReport {
    /// The transformed source text.
    pub output: String,
    /// One entry per substitution made.
    pub replacements: Vec<Replacement>,
    /// Count of alias occurrences found inside string literals (not substituted).
    pub skipped_in_literals: usize,
}

/// Apply a locale pack to source text: replace each localized keyword
/// occurrence with its canonical English keyword (or vice versa).
///
/// This is a **lexical pass** — it tokenizes the input stream using the same
/// word-character classifier as the nom-concept lexer and applies the alias
/// map.  String literals (`"..."` and `'...'`) and line comments (`#...`,
/// `//...`) are skipped.
///
/// For `ToCanonical`: the `keyword_aliases` map (localized → canonical) is used.
/// For `FromCanonical`: an inverted map (canonical → diacritic localized) is
/// built on the fly; only unambiguous 1-to-1 mappings are applied.  If
/// multiple localized aliases share a canonical (e.g. `"là"` and `"la"` both
/// map to `"is"`), `FromCanonical` picks whichever appears **first** in the
/// pack's alias table (diacritic form, for vi-VN).
pub fn apply_locale(source: &str, pack: &LocalePack, direction: ApplyDirection) -> ApplyReport {
    // Build the lookup map for the requested direction.
    let lookup: BTreeMap<&str, &str> = match direction {
        ApplyDirection::ToCanonical => {
            pack.keyword_aliases
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect()
        }
        ApplyDirection::FromCanonical => {
            // Invert: canonical → first (diacritic-preferred) localized alias.
            // We use the original const order which puts diacritics first.
            // For packs other than vi-VN, fall back to iterating keyword_aliases
            // in BTreeMap order (alphabetical) — acceptable for non-vi packs.
            let mut inv: BTreeMap<&str, &str> = BTreeMap::new();
            // Walk the original const table to preserve diacritic-first order for vi-VN.
            for &(loc, canon) in VI_VN_KEYWORD_ALIASES {
                inv.entry(canon).or_insert(loc);
            }
            // Also cover any pack aliases not in the const (e.g. custom packs).
            for (loc, canon) in &pack.keyword_aliases {
                inv.entry(canon.as_str()).or_insert(loc.as_str());
            }
            inv
        }
    };

    let mut output = String::with_capacity(source.len() + 64);
    let mut replacements = Vec::new();
    let mut skipped_in_literals: usize = 0;

    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut pos = 0usize;
    let mut line = 1usize;
    let mut line_start = 0usize; // byte offset of current line start

    while pos < len {
        // ── Line comments: # or // ────────────────────────────────────────────
        if bytes[pos] == b'#'
            || (bytes[pos] == b'/' && pos + 1 < len && bytes[pos + 1] == b'/')
        {
            // Copy everything to end of line unchanged.
            let comment_start = pos;
            while pos < len && bytes[pos] != b'\n' {
                pos += 1;
            }
            output.push_str(&source[comment_start..pos]);
            continue;
        }

        // ── String literals: "..." or '...' ──────────────────────────────────
        if bytes[pos] == b'"' || bytes[pos] == b'\'' {
            let quote = bytes[pos];
            let lit_start = pos;
            pos += 1; // consume opening quote
            while pos < len {
                if bytes[pos] == b'\\' {
                    pos += 2; // skip escape sequence
                    continue;
                }
                if bytes[pos] == quote {
                    pos += 1; // consume closing quote
                    break;
                }
                // Check if this character starts a word that is an alias.
                // We count it as skipped_in_literals if we detect an alias token here.
                // We do this at the end (post-scan) rather than inline.
                pos += 1;
            }
            let lit_text = &source[lit_start..pos];
            // Count aliases inside the literal (for the skipped_in_literals counter).
            skipped_in_literals += count_aliases_in_literal(lit_text, &lookup);
            output.push_str(lit_text);
            continue;
        }

        // ── Newline ───────────────────────────────────────────────────────────
        if bytes[pos] == b'\n' {
            output.push('\n');
            pos += 1;
            line += 1;
            line_start = pos;
            continue;
        }

        // ── Word token ────────────────────────────────────────────────────────
        let ch = source[pos..].chars().next().unwrap_or('\0');
        if is_word_start(ch) {
            let word_start = pos;
            let col = pos - line_start + 1; // 1-based byte column
            // Advance past the full word (including underscores and diacritics).
            while pos < len {
                let c = source[pos..].chars().next().unwrap_or('\0');
                if is_word_continue(c) {
                    pos += c.len_utf8();
                } else {
                    break;
                }
            }
            let word = &source[word_start..pos];
            if let Some(&replacement) = lookup.get(word) {
                replacements.push(Replacement {
                    line,
                    column: col,
                    from: word.to_string(),
                    to: replacement.to_string(),
                });
                output.push_str(replacement);
            } else {
                output.push_str(word);
            }
            continue;
        }

        // ── Any other character ───────────────────────────────────────────────
        output.push(ch);
        pos += ch.len_utf8();
    }

    ApplyReport {
        output,
        replacements,
        skipped_in_literals,
    }
}

/// Count how many top-level word tokens inside a string literal match an alias.
///
/// The literal text includes its surrounding quote characters.  We skip the
/// opening and closing quote, then scan word tokens against the lookup map.
fn count_aliases_in_literal<'a>(lit: &'a str, lookup: &BTreeMap<&str, &str>) -> usize {
    let mut count = 0;
    let bytes = lit.as_bytes();
    let len = bytes.len();
    // Skip opening quote (first byte) and closing quote (last byte).
    let mut pos = 1usize;
    let end = if len > 0 { len - 1 } else { 0 };
    while pos < end {
        let ch = lit[pos..].chars().next().unwrap_or('\0');
        if is_word_start(ch) {
            let word_start = pos;
            while pos < end {
                let c = lit[pos..].chars().next().unwrap_or('\0');
                if is_word_continue(c) {
                    pos += c.len_utf8();
                } else {
                    break;
                }
            }
            let word = &lit[word_start..pos];
            if lookup.contains_key(word) {
                count += 1;
            }
        } else {
            pos += ch.len_utf8();
        }
    }
    count
}

// ── Word character classifier (mirrors nom-concept lexer helpers) ─────────────
//
// Duplicated here deliberately to avoid a dependency on nom-concept.
// Must stay in sync with `is_word_start_char` / `is_word_continue_char`
// from commit 5b59f82.

/// Returns `true` if `c` may start a keyword/identifier token.
#[inline]
fn is_word_start(c: char) -> bool {
    c.is_ascii_lowercase() || c == '_' || is_vn_diacritic(c)
}

/// Returns `true` if `c` may continue a keyword/identifier token.
#[inline]
fn is_word_continue(c: char) -> bool {
    c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit() || is_vn_diacritic(c)
}

/// Closed whitelist of Vietnamese lowercase diacritic characters that appear
/// in the keyword alias table.  Explicit set — no `unicode-xid` dependency.
///
/// Mirrors `is_vietnamese_diacritic_lower` from nom-concept commit 5b59f82.
#[inline]
fn is_vn_diacritic(c: char) -> bool {
    matches!(
        c,
        // à á â ã ä å
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' |
        // ả ạ ặ ấ ầ ẩ ẫ ậ ắ ằ ẳ ẵ
        'ả' | 'ạ' | 'ặ' | 'ấ' | 'ầ' | 'ẩ' | 'ẫ' | 'ậ' | 'ắ' | 'ằ' | 'ẳ' | 'ẵ' |
        // è é ê
        'è' | 'é' | 'ê' |
        'ẻ' | 'ẽ' | 'ẹ' | 'ế' | 'ề' | 'ể' | 'ễ' | 'ệ' |
        // ì í
        'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' |
        // ò ó ô õ
        'ò' | 'ó' | 'ô' | 'õ' |
        'ỏ' | 'ọ' | 'ố' | 'ồ' | 'ổ' | 'ỗ' | 'ộ' | 'ớ' | 'ờ' | 'ở' | 'ỡ' | 'ợ' |
        // ơ (U+01A1)
        'ơ' |
        // ù ú
        'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ứ' | 'ừ' | 'ử' | 'ữ' | 'ự' |
        // ư (U+01B0)
        'ư' |
        // ỳ ý
        'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' |
        // đ (U+0111)
        'đ'
    )
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_language() {
        let tag = LocaleTag::parse("vi").unwrap();
        assert_eq!(tag.language, "vi");
        assert!(tag.script.is_none());
        assert!(tag.region.is_none());
        assert!(tag.variants.is_empty());
    }

    #[test]
    fn parse_with_region() {
        let tag = LocaleTag::parse("vi-VN").unwrap();
        assert_eq!(tag.language, "vi");
        assert_eq!(tag.region, Some("VN".to_string()));
        assert!(tag.script.is_none());
    }

    #[test]
    fn parse_with_script_and_region() {
        let tag = LocaleTag::parse("zh-Hant-TW").unwrap();
        assert_eq!(tag.language, "zh");
        assert_eq!(tag.script, Some("Hant".to_string()));
        assert_eq!(tag.region, Some("TW".to_string()));
    }

    #[test]
    fn parse_lowercase_input_canonicalizes() {
        // "VI-vn" → language=vi, region=VN (title-case script not applicable here)
        let tag = LocaleTag::parse("VI-vn").unwrap();
        assert_eq!(tag.language, "vi");
        assert_eq!(tag.region, Some("VN".to_string()));
        assert_eq!(tag.canonical(), "vi-VN");
    }

    #[test]
    fn parse_rejects_empty() {
        assert_eq!(LocaleTag::parse(""), Err(ParseError::Empty));
    }

    #[test]
    fn parse_rejects_numeric_language() {
        // Purely numeric language codes are invalid.
        assert!(LocaleTag::parse("123").is_err());
    }

    #[test]
    fn parse_rejects_too_long_language() {
        // 4+ char language subtag without special handling is invalid.
        assert!(LocaleTag::parse("engl").is_err());
    }

    #[test]
    fn parse_unsupported_extension_flagged() {
        // BCP 47 extension subtag `u-ca-gregory` — M3a accepts the language
        // part and captures the extension in `unsupported`.
        let tag = LocaleTag::parse("en-u-ca-gregory").unwrap();
        assert_eq!(tag.language, "en");
        assert!(
            tag.unsupported.is_some(),
            "expected unsupported to be Some for extension subtag"
        );
    }

    #[test]
    fn normalize_nfc_composes_precomposed() {
        // U+1EBF "ế" (e + COMBINING CIRCUMFLEX ACCENT + COMBINING ACUTE ACCENT in NFD)
        // Both forms must normalize to the same NFC code point.
        let precomposed = "\u{1EBF}"; // ế as a single code point
        let decomposed = "e\u{0302}\u{0301}"; // e + combining circumflex + combining acute
        assert_eq!(
            normalize_nfc(precomposed),
            normalize_nfc(decomposed),
            "NFC should compose decomposed Vietnamese characters"
        );
        assert_eq!(normalize_nfc(decomposed), precomposed);
    }

    #[test]
    fn builtin_packs_contains_vi_and_en() {
        let packs = builtin_packs();
        assert_eq!(packs.len(), 2);
        let tags: Vec<String> = packs.iter().map(|p| p.id.canonical()).collect();
        assert!(tags.contains(&"vi-VN".to_string()), "vi-VN must be present");
        assert!(tags.contains(&"en-US".to_string()), "en-US must be present");
    }

    #[test]
    fn is_confusable_equal_returns_equal() {
        assert_eq!(is_confusable("hello", "hello"), ConfusableResult::Equal);
        // NFC-equal inputs must also return Equal.
        let a = "\u{1EBF}";
        let b = "e\u{0302}\u{0301}";
        assert_eq!(is_confusable(a, b), ConfusableResult::Equal);
    }

    #[test]
    fn is_confusable_different_returns_deferred_m3a() {
        // M3a stub always returns Deferred for non-equal inputs.
        assert_eq!(
            is_confusable("hello", "he1lo"),
            ConfusableResult::Deferred
        );
    }

    // ── M3c tests — apply_locale on a synthetic pack (vocab-agnostic) ─────────
    //
    // The shipped vi-VN pack has empty keyword_aliases per the user's
    // "Vietnamese grammar, English vocabulary" directive. These tests build
    // a synthetic test pack (not registered as a builtin) so the apply_locale
    // machinery is still exercised. Any future grammar-oriented locale pack
    // that DOES carry a keyword map will use the same code path.

    fn test_pack_with_aliases(pairs: &[(&str, &str)]) -> LocalePack {
        let keyword_aliases: BTreeMap<String, String> = pairs
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect();
        LocalePack {
            id: LocaleTag::parse("xx-Test").expect("xx-Test is valid"),
            keyword_aliases,
            nom_aliases: BTreeMap::new(),
            register_metadata: RegisterMetadata {
                display_name: "Test pack".to_string(),
                source: "test-fixture".to_string(),
                license: "CC0-1.0".to_string(),
                registered_at: "epoch-0".to_string(),
            },
        }
    }

    #[test]
    fn vi_vn_pack_keyword_aliases_is_empty_by_directive() {
        let packs = builtin_packs();
        let vi = packs.iter().find(|p| p.id.canonical() == "vi-VN").unwrap();
        assert!(
            vi.keyword_aliases.is_empty(),
            "vi-VN pack must have empty keyword_aliases — Vietnamese contributes \
             GRAMMAR STYLE only; vocabulary stays English"
        );
        assert_eq!(vi.register_metadata.source, "baked:vi-v1-grammar-only");
    }

    #[test]
    fn apply_to_canonical_basic() {
        let pack = test_pack_with_aliases(&[("alpha", "first"), ("beta", "second")]);
        let report = apply_locale("alpha beta", &pack, ApplyDirection::ToCanonical);
        assert_eq!(report.output, "first second");
        assert_eq!(report.replacements.len(), 2);
        assert_eq!(report.replacements[0].from, "alpha");
        assert_eq!(report.replacements[0].to, "first");
        assert_eq!(report.replacements[0].line, 1);
        assert_eq!(report.replacements[0].column, 1);
    }

    #[test]
    fn apply_to_canonical_ignores_string_literals() {
        let pack = test_pack_with_aliases(&[("alpha", "first")]);
        let src = r#"alpha x = "alpha unchanged""#;
        let report = apply_locale(src, &pack, ApplyDirection::ToCanonical);
        assert_eq!(report.replacements.len(), 1);
        assert_eq!(report.replacements[0].from, "alpha");
        assert_eq!(report.skipped_in_literals, 1);
        assert!(report.output.starts_with("first"));
    }

    #[test]
    fn apply_to_canonical_ignores_line_comments() {
        let pack = test_pack_with_aliases(&[("alpha", "first")]);
        let src = "# alpha\nalpha";
        let report = apply_locale(src, &pack, ApplyDirection::ToCanonical);
        assert_eq!(report.replacements.len(), 1);
        assert_eq!(report.replacements[0].line, 2);
        assert_eq!(report.output, "# alpha\nfirst");
    }

    #[test]
    fn apply_roundtrip_through_both_directions() {
        let pack = test_pack_with_aliases(&[("alpha", "first"), ("beta", "second")]);
        let canonical = "first second";
        let to_loc = apply_locale(canonical, &pack, ApplyDirection::FromCanonical);
        assert_eq!(to_loc.output, "alpha beta");
        let back = apply_locale(&to_loc.output, &pack, ApplyDirection::ToCanonical);
        assert_eq!(back.output, canonical);
    }

    #[test]
    fn apply_multiword_phrase_expands() {
        // Single underscore-joined token expands to a multi-word canonical.
        let pack = test_pack_with_aliases(&[("alpha_beta", "first second")]);
        let report = apply_locale("alpha_beta", &pack, ApplyDirection::ToCanonical);
        assert_eq!(report.output, "first second");
        assert_eq!(report.replacements.len(), 1);
        assert_eq!(report.replacements[0].from, "alpha_beta");
        assert_eq!(report.replacements[0].to, "first second");
    }

    #[test]
    fn apply_unknown_token_unchanged() {
        let pack = test_pack_with_aliases(&[("alpha", "first")]);
        let report = apply_locale("xyzzy", &pack, ApplyDirection::ToCanonical);
        assert_eq!(report.output, "xyzzy");
        assert!(report.replacements.is_empty());
    }

    #[test]
    fn apply_on_empty_vi_vn_pack_is_noop() {
        // The shipped vi-VN pack has no aliases → apply_locale is a no-op.
        let packs = builtin_packs();
        let vi = packs.iter().find(|p| p.id.canonical() == "vi-VN").unwrap();
        let report = apply_locale("the function is", vi, ApplyDirection::ToCanonical);
        assert_eq!(report.output, "the function is");
        assert!(report.replacements.is_empty());
    }
}
