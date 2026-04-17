#![deny(unsafe_code)]

use std::collections::HashMap;

use crate::block_transformer::{BlockTransformer, Snapshot, TransformError};
use crate::flavour::NOMX;
use crate::nomx::{NomxLang, NomxProps};

pub struct NomxTransformer;

impl BlockTransformer for NomxTransformer {
    type Props = NomxProps;

    fn from_snapshot(&self, snap: &Snapshot) -> Result<Self::Props, TransformError> {
        if snap.flavour != NOMX {
            return Err(TransformError::InvalidData);
        }
        let text = snap_to_str(snap)?;
        let kv = parse_kv(&text);

        let source = kv.get("source").cloned().unwrap_or_default();
        let lang = match kv.get("lang").map(String::as_str) {
            Some("nom") => NomxLang::Nom,
            _ => NomxLang::Nomx,
        };
        let wrap = kv.get("wrap").map(|v| v == "true").unwrap_or(false);
        let line_numbers = kv.get("line_numbers").map(|v| v == "true").unwrap_or(true);
        let caption = kv.get("caption").and_then(|v| {
            if v == "__none__" { None } else { Some(v.clone()) }
        });

        Ok(NomxProps { source, lang, wrap, caption, line_numbers })
    }

    fn to_snapshot(&self, props: &Self::Props) -> Snapshot {
        let lang = match props.lang {
            NomxLang::Nom => "nom",
            NomxLang::Nomx => "nomx",
        };
        let caption_val = match &props.caption {
            Some(c) => escape_val(c),
            None => "__none__".to_string(),
        };
        let data = format!(
            "source={}\nlang={}\nwrap={}\nline_numbers={}\ncaption={}\n",
            escape_val(&props.source),
            lang,
            props.wrap,
            props.line_numbers,
            caption_val,
        );
        Snapshot { flavour: NOMX, version: 1, data: data.into_bytes() }
    }
}

fn snap_to_str(snap: &Snapshot) -> Result<String, TransformError> {
    std::str::from_utf8(&snap.data)
        .map(|s| s.to_string())
        .map_err(|_| TransformError::InvalidData)
}

fn parse_kv(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            out.insert(k.to_string(), unescape_val(v));
        }
    }
    out
}

fn escape_val(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "\\r")
}

fn unescape_val(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some(other) => { out.push('\\'); out.push(other); }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> NomxProps {
        NomxProps {
            source: "define add that x + y".to_string(),
            lang: NomxLang::Nom,
            wrap: true,
            caption: Some("Example".to_string()),
            line_numbers: false,
        }
    }

    #[test]
    fn round_trip() {
        let t = NomxTransformer;
        let p = sample();
        let s = t.to_snapshot(&p);
        let back = t.from_snapshot(&s).unwrap();
        assert_eq!(back.source, p.source);
        assert_eq!(back.lang, p.lang);
        assert_eq!(back.wrap, p.wrap);
        assert_eq!(back.caption, p.caption);
        assert_eq!(back.line_numbers, p.line_numbers);
    }

    #[test]
    fn wrong_flavour_rejected() {
        let t = NomxTransformer;
        let s = Snapshot { flavour: crate::flavour::PROSE, version: 1, data: vec![] };
        assert!(t.from_snapshot(&s).is_err());
    }

    #[test]
    fn missing_fields_use_defaults() {
        let t = NomxTransformer;
        let snap = Snapshot { flavour: NOMX, version: 1, data: b"source=hi\n".to_vec() };
        let p = t.from_snapshot(&snap).unwrap();
        assert_eq!(p.source, "hi");
        assert_eq!(p.lang, NomxLang::Nomx);
        assert!(!p.wrap);
        assert!(p.line_numbers);
        assert!(p.caption.is_none());
    }

    #[test]
    fn caption_none_vs_some() {
        let t = NomxTransformer;
        let with_caption = NomxProps { caption: Some("note".to_string()), ..NomxProps::default() };
        let without_caption = NomxProps { caption: None, ..NomxProps::default() };

        let s1 = t.to_snapshot(&with_caption);
        let s2 = t.to_snapshot(&without_caption);

        assert_eq!(t.from_snapshot(&s1).unwrap().caption, Some("note".to_string()));
        assert_eq!(t.from_snapshot(&s2).unwrap().caption, None);
    }

    #[test]
    fn source_with_newlines_round_trips() {
        let t = NomxTransformer;
        let p = NomxProps { source: "line1\nline2\nline3".to_string(), ..NomxProps::default() };
        let s = t.to_snapshot(&p);
        assert_eq!(t.from_snapshot(&s).unwrap().source, "line1\nline2\nline3");
    }
}
