//! `nom-locale` — BCP47 tag parsing + UAX #15 NFC + M3a scaffold for M3 locale packs (doc 09).

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
///
/// M3a: both alias maps are empty; they are populated in M3c.
#[derive(Debug, Clone)]
pub struct LocalePack {
    /// The BCP 47 locale this pack covers.
    pub id: LocaleTag,
    /// Localized keyword → canonical Nom keyword (e.g. `"xác định"` → `"define"`).
    /// Empty in M3a; populated in M3c.
    pub keyword_aliases: BTreeMap<String, String>,
    /// Localized identifier → canonical hash suffix.
    /// Empty in M3a; intended for M3c+ corpus-derived aliases.
    pub nom_aliases: BTreeMap<String, String>,
    /// Pack provenance and license metadata.
    pub register_metadata: RegisterMetadata,
}

// ── Built-in pack registry ────────────────────────────────────────────────────

/// Return all baked-in locale packs for M3a.
///
/// Currently contains two stubs: `vi-VN` (Vietnamese, Vietnam) and `en-US`
/// (English, United States).  Alias tables are empty until M3c populates them.
pub fn builtin_packs() -> Vec<LocalePack> {
    vec![
        LocalePack {
            id: LocaleTag::parse("vi-VN").expect("vi-VN is valid"),
            keyword_aliases: BTreeMap::new(),
            nom_aliases: BTreeMap::new(),
            register_metadata: RegisterMetadata {
                display_name: "Vietnamese (Vietnam)".to_string(),
                source: "baked:vi-v1-stub".to_string(),
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
        for pack in &packs {
            assert!(
                pack.keyword_aliases.is_empty(),
                "keyword_aliases must be empty in M3a"
            );
        }
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
}
