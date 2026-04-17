#![deny(unsafe_code)]

use std::collections::HashMap;

use crate::block_transformer::{BlockTransformer, Snapshot, TransformError};
use crate::flavour::MEDIA_IMAGE;
use crate::media::ImageProps;

pub struct ImageTransformer;

impl BlockTransformer for ImageTransformer {
    type Props = ImageProps;

    fn from_snapshot(&self, snap: &Snapshot) -> Result<Self::Props, TransformError> {
        if snap.flavour != MEDIA_IMAGE {
            return Err(TransformError::InvalidData);
        }
        let text = snap_to_str(snap)?;
        let kv = parse_kv(&text);

        let source_id = kv
            .get("source_id")
            .cloned()
            .ok_or(TransformError::MissingField("source_id"))?;
        let xywh = kv.get("xywh").cloned().unwrap_or_else(|| "0 0 0 0".to_string());
        let rotate: f32 = kv
            .get("rotate")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        let width: u32 = kv.get("width").and_then(|v| v.parse().ok()).unwrap_or(0);
        let height: u32 = kv.get("height").and_then(|v| v.parse().ok()).unwrap_or(0);
        let caption = kv.get("caption").and_then(|v| {
            if v == "__none__" { None } else { Some(unescape_val(v)) }
        });
        let index = kv.get("index").cloned().unwrap_or_else(|| "a0".to_string());

        Ok(ImageProps { source_id, xywh, rotate, width, height, caption, index })
    }

    fn to_snapshot(&self, props: &Self::Props) -> Snapshot {
        let caption_val = match &props.caption {
            Some(c) => escape_val(c),
            None => "__none__".to_string(),
        };
        let data = format!(
            "source_id={}\nxywh={}\nrotate={}\nwidth={}\nheight={}\ncaption={}\nindex={}\n",
            escape_val(&props.source_id),
            escape_val(&props.xywh),
            props.rotate,
            props.width,
            props.height,
            caption_val,
            escape_val(&props.index),
        );
        Snapshot { flavour: MEDIA_IMAGE, version: 1, data: data.into_bytes() }
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

    fn sample() -> ImageProps {
        ImageProps::new("blob-xyz".to_string(), 1920, 1080)
            .with_caption("A landscape")
    }

    #[test]
    fn round_trip() {
        let t = ImageTransformer;
        let p = sample();
        let s = t.to_snapshot(&p);
        let back = t.from_snapshot(&s).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn wrong_flavour_rejected() {
        let t = ImageTransformer;
        let s = Snapshot { flavour: crate::flavour::PROSE, version: 1, data: vec![] };
        assert!(t.from_snapshot(&s).is_err());
    }

    #[test]
    fn missing_source_id_returns_error() {
        let t = ImageTransformer;
        let snap = Snapshot {
            flavour: MEDIA_IMAGE,
            version: 1,
            data: b"width=100\nheight=100\n".to_vec(),
        };
        assert!(matches!(t.from_snapshot(&snap), Err(TransformError::MissingField("source_id"))));
    }

    #[test]
    fn caption_none_vs_some() {
        let t = ImageTransformer;
        let with_cap = ImageProps::new("b1".to_string(), 100, 100).with_caption("cap");
        let no_cap = ImageProps::new("b2".to_string(), 100, 100);

        assert_eq!(t.from_snapshot(&t.to_snapshot(&with_cap)).unwrap().caption, Some("cap".to_string()));
        assert_eq!(t.from_snapshot(&t.to_snapshot(&no_cap)).unwrap().caption, None);
    }

    #[test]
    fn rotate_round_trips() {
        let t = ImageTransformer;
        let mut p = sample();
        p.rotate = 45.5;
        let back = t.from_snapshot(&t.to_snapshot(&p)).unwrap();
        assert!((back.rotate - 45.5_f32).abs() < 1e-3);
    }
}
