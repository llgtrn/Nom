#![deny(unsafe_code)]

use std::collections::HashMap;

use crate::block_transformer::{BlockTransformer, Snapshot, TransformError};
use crate::flavour::MEDIA_ATTACHMENT;
use crate::media::AttachmentProps;

pub struct AttachmentTransformer;

impl BlockTransformer for AttachmentTransformer {
    type Props = AttachmentProps;

    fn from_snapshot(&self, snap: &Snapshot) -> Result<Self::Props, TransformError> {
        if snap.flavour != MEDIA_ATTACHMENT {
            return Err(TransformError::InvalidData);
        }
        let text = snap_to_str(snap)?;
        let kv = parse_kv(&text);

        let source_id = kv
            .get("source_id")
            .cloned()
            .ok_or(TransformError::MissingField("source_id"))?;
        let name = kv
            .get("name")
            .cloned()
            .ok_or(TransformError::MissingField("name"))?;
        let size: u64 = kv.get("size").and_then(|v| v.parse().ok()).unwrap_or(0);
        let mime = kv.get("mime").cloned().unwrap_or_default();
        let embed = kv.get("embed").map(|v| v == "true").unwrap_or(false);
        let caption = kv.get("caption").and_then(|v| {
            if v == "__none__" { None } else { Some(unescape_val(v)) }
        });

        Ok(AttachmentProps { source_id, name, size, mime, embed, caption })
    }

    fn to_snapshot(&self, props: &Self::Props) -> Snapshot {
        let caption_val = match &props.caption {
            Some(c) => escape_val(c),
            None => "__none__".to_string(),
        };
        let data = format!(
            "source_id={}\nname={}\nsize={}\nmime={}\nembed={}\ncaption={}\n",
            escape_val(&props.source_id),
            escape_val(&props.name),
            props.size,
            escape_val(&props.mime),
            props.embed,
            caption_val,
        );
        Snapshot { flavour: MEDIA_ATTACHMENT, version: 1, data: data.into_bytes() }
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
            out.insert(k.to_string(), v.to_string());
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

    fn sample() -> AttachmentProps {
        AttachmentProps {
            source_id: "blob-001".to_string(),
            name: "report.pdf".to_string(),
            size: 204_800,
            mime: "application/pdf".to_string(),
            embed: true,
            caption: Some("Annual report".to_string()),
        }
    }

    #[test]
    fn round_trip() {
        let t = AttachmentTransformer;
        let p = sample();
        let s = t.to_snapshot(&p);
        let back = t.from_snapshot(&s).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn wrong_flavour_rejected() {
        let t = AttachmentTransformer;
        let s = Snapshot { flavour: crate::flavour::PROSE, version: 1, data: vec![] };
        assert!(t.from_snapshot(&s).is_err());
    }

    #[test]
    fn missing_required_fields_return_error() {
        let t = AttachmentTransformer;
        // Missing source_id
        let snap = Snapshot {
            flavour: MEDIA_ATTACHMENT,
            version: 1,
            data: b"name=file.txt\n".to_vec(),
        };
        assert!(matches!(t.from_snapshot(&snap), Err(TransformError::MissingField("source_id"))));

        // Missing name
        let snap2 = Snapshot {
            flavour: MEDIA_ATTACHMENT,
            version: 1,
            data: b"source_id=abc\n".to_vec(),
        };
        assert!(matches!(t.from_snapshot(&snap2), Err(TransformError::MissingField("name"))));
    }

    #[test]
    fn caption_none_vs_some() {
        let t = AttachmentTransformer;
        let with_cap = AttachmentProps {
            source_id: "b".to_string(),
            name: "f".to_string(),
            size: 0,
            mime: "text/plain".to_string(),
            embed: false,
            caption: Some("desc".to_string()),
        };
        let no_cap = AttachmentProps { caption: None, ..with_cap.clone() };

        assert_eq!(t.from_snapshot(&t.to_snapshot(&with_cap)).unwrap().caption, Some("desc".to_string()));
        assert_eq!(t.from_snapshot(&t.to_snapshot(&no_cap)).unwrap().caption, None);
    }

    #[test]
    fn large_size_round_trips() {
        let t = AttachmentTransformer;
        let p = AttachmentProps {
            source_id: "b".to_string(),
            name: "big.zip".to_string(),
            size: u64::MAX,
            mime: "application/zip".to_string(),
            embed: false,
            caption: None,
        };
        assert_eq!(t.from_snapshot(&t.to_snapshot(&p)).unwrap().size, u64::MAX);
    }
}
