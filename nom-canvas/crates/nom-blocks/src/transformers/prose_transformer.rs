#![deny(unsafe_code)]

use std::collections::HashMap;

use crate::block_transformer::{BlockTransformer, Snapshot, TransformError};
use crate::flavour::PROSE;
use crate::prose::{ProseKind, ProseProps, TextAlign};

pub struct ProseTransformer;

impl BlockTransformer for ProseTransformer {
    type Props = ProseProps;

    fn from_snapshot(&self, snap: &Snapshot) -> Result<Self::Props, TransformError> {
        if snap.flavour != PROSE {
            return Err(TransformError::InvalidData);
        }
        let text = snap_to_str(snap)?;
        let kv = parse_kv(&text);

        let body_text = kv.get("text").cloned().unwrap_or_default();
        let align = match kv.get("align").map(String::as_str) {
            Some("center") => TextAlign::Center,
            Some("right") => TextAlign::Right,
            Some("justify") => TextAlign::Justify,
            _ => TextAlign::Left,
        };
        let kind = match kv.get("kind").map(String::as_str) {
            Some("h1") => ProseKind::H1,
            Some("h2") => ProseKind::H2,
            Some("h3") => ProseKind::H3,
            Some("h4") => ProseKind::H4,
            Some("h5") => ProseKind::H5,
            Some("h6") => ProseKind::H6,
            Some("quote") => ProseKind::Quote,
            Some("bulleted") => ProseKind::Bulleted,
            Some("numbered") => ProseKind::Numbered,
            Some("todo") => ProseKind::Todo,
            _ => ProseKind::Text,
        };
        let collapsed = kv.get("collapsed").map(|v| v == "true").unwrap_or(false);

        Ok(ProseProps { text: body_text, text_align: align, kind, collapsed })
    }

    fn to_snapshot(&self, props: &Self::Props) -> Snapshot {
        let align = match props.text_align {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
            TextAlign::Justify => "justify",
        };
        let kind = match props.kind {
            ProseKind::Text => "text",
            ProseKind::H1 => "h1",
            ProseKind::H2 => "h2",
            ProseKind::H3 => "h3",
            ProseKind::H4 => "h4",
            ProseKind::H5 => "h5",
            ProseKind::H6 => "h6",
            ProseKind::Quote => "quote",
            ProseKind::Bulleted => "bulleted",
            ProseKind::Numbered => "numbered",
            ProseKind::Todo => "todo",
        };
        let data = format!(
            "text={}\nalign={}\nkind={}\ncollapsed={}\n",
            escape_val(&props.text),
            align,
            kind,
            props.collapsed
        );
        Snapshot { flavour: PROSE, version: 1, data: data.into_bytes() }
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

    #[test]
    fn round_trip() {
        let t = ProseTransformer;
        let props = ProseProps {
            text: "hello".to_string(),
            text_align: TextAlign::Center,
            kind: ProseKind::H2,
            collapsed: true,
        };
        let s = t.to_snapshot(&props);
        let back = t.from_snapshot(&s).unwrap();
        assert_eq!(back.text, props.text);
        assert_eq!(back.text_align, props.text_align);
        assert_eq!(back.kind, props.kind);
        assert_eq!(back.collapsed, props.collapsed);
    }

    #[test]
    fn wrong_flavour_rejected() {
        let t = ProseTransformer;
        let s = Snapshot { flavour: crate::flavour::NOMX, version: 1, data: vec![] };
        assert!(t.from_snapshot(&s).is_err());
    }

    #[test]
    fn text_with_newlines_round_trips() {
        let t = ProseTransformer;
        let props = ProseProps {
            text: "line1\nline2".to_string(),
            text_align: TextAlign::Left,
            kind: ProseKind::Text,
            collapsed: false,
        };
        let s = t.to_snapshot(&props);
        assert_eq!(t.from_snapshot(&s).unwrap().text, "line1\nline2");
    }

    #[test]
    fn default_kind_when_missing() {
        let t = ProseTransformer;
        let snap = Snapshot { flavour: PROSE, version: 1, data: b"text=hi\n".to_vec() };
        assert_eq!(t.from_snapshot(&snap).unwrap().kind, ProseKind::Text);
    }

    #[test]
    fn all_prose_kinds_round_trip() {
        let t = ProseTransformer;
        for kind in [
            ProseKind::H1,
            ProseKind::H2,
            ProseKind::H3,
            ProseKind::H4,
            ProseKind::H5,
            ProseKind::H6,
            ProseKind::Quote,
            ProseKind::Bulleted,
            ProseKind::Numbered,
            ProseKind::Todo,
        ] {
            let props = ProseProps { text: String::new(), text_align: TextAlign::Left, kind: kind.clone(), collapsed: false };
            let s = t.to_snapshot(&props);
            assert_eq!(t.from_snapshot(&s).unwrap().kind, kind);
        }
    }
}
