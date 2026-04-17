//! Wire protocol for CRDT sync messages.
//!
//! Simple tagged-byte encoding:
//!   [1 byte tag] [8 bytes doc_id_len] [doc_id bytes] [rest…]
//!
//! Tags: 0x01 = Update, 0x02 = Awareness, 0x03 = Query, 0x04 = Sync

use crate::doc_id::DocId;
use thiserror::Error;

const TAG_UPDATE: u8 = 0x01;
const TAG_AWARENESS: u8 = 0x02;
const TAG_QUERY: u8 = 0x03;
const TAG_SYNC: u8 = 0x04;

#[derive(Debug, PartialEq)]
pub enum SyncMessage {
    Update { doc_id: DocId, update_v2: Vec<u8> },
    Awareness { doc_id: DocId, client_id: u64, state: Vec<u8> },
    Query { doc_id: DocId, state_vector: Vec<u8> },
    Sync { doc_id: DocId, update_v2: Vec<u8> },
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("unknown message tag: 0x{0:02x}")]
    InvalidTag(u8),
    #[error("payload truncated")]
    TruncatedPayload,
}

// ---- encoding helpers -------------------------------------------------------

fn write_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_bytes(buf: &mut Vec<u8>, data: &[u8]) {
    write_u64(buf, data.len() as u64);
    buf.extend_from_slice(data);
}

fn write_str(buf: &mut Vec<u8>, s: &str) {
    write_bytes(buf, s.as_bytes());
}

// ---- decoding helpers -------------------------------------------------------

fn read_u64(src: &[u8], pos: &mut usize) -> Result<u64, ProtocolError> {
    let end = *pos + 8;
    if src.len() < end {
        return Err(ProtocolError::TruncatedPayload);
    }
    let v = u64::from_le_bytes(src[*pos..end].try_into().unwrap());
    *pos = end;
    Ok(v)
}

fn read_bytes<'a>(src: &'a [u8], pos: &mut usize) -> Result<&'a [u8], ProtocolError> {
    let len = read_u64(src, pos)? as usize;
    let end = *pos + len;
    if src.len() < end {
        return Err(ProtocolError::TruncatedPayload);
    }
    let slice = &src[*pos..end];
    *pos = end;
    Ok(slice)
}

fn read_doc_id(src: &[u8], pos: &mut usize) -> Result<DocId, ProtocolError> {
    let bytes = read_bytes(src, pos)?;
    let s = std::str::from_utf8(bytes).map_err(|_| ProtocolError::TruncatedPayload)?;
    Ok(DocId(s.to_owned()))
}

// ---- SyncMessage impl -------------------------------------------------------

impl SyncMessage {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            SyncMessage::Update { doc_id, update_v2 } => {
                buf.push(TAG_UPDATE);
                write_str(&mut buf, &doc_id.0);
                write_bytes(&mut buf, update_v2);
            }
            SyncMessage::Awareness { doc_id, client_id, state } => {
                buf.push(TAG_AWARENESS);
                write_str(&mut buf, &doc_id.0);
                write_u64(&mut buf, *client_id);
                write_bytes(&mut buf, state);
            }
            SyncMessage::Query { doc_id, state_vector } => {
                buf.push(TAG_QUERY);
                write_str(&mut buf, &doc_id.0);
                write_bytes(&mut buf, state_vector);
            }
            SyncMessage::Sync { doc_id, update_v2 } => {
                buf.push(TAG_SYNC);
                write_str(&mut buf, &doc_id.0);
                write_bytes(&mut buf, update_v2);
            }
        }
        buf
    }

    pub fn decode(src: &[u8]) -> Result<Self, ProtocolError> {
        if src.is_empty() {
            return Err(ProtocolError::TruncatedPayload);
        }
        let tag = src[0];
        let mut pos = 1usize;
        match tag {
            TAG_UPDATE => {
                let doc_id = read_doc_id(src, &mut pos)?;
                let update_v2 = read_bytes(src, &mut pos)?.to_vec();
                Ok(SyncMessage::Update { doc_id, update_v2 })
            }
            TAG_AWARENESS => {
                let doc_id = read_doc_id(src, &mut pos)?;
                let client_id = read_u64(src, &mut pos)?;
                let state = read_bytes(src, &mut pos)?.to_vec();
                Ok(SyncMessage::Awareness { doc_id, client_id, state })
            }
            TAG_QUERY => {
                let doc_id = read_doc_id(src, &mut pos)?;
                let state_vector = read_bytes(src, &mut pos)?.to_vec();
                Ok(SyncMessage::Query { doc_id, state_vector })
            }
            TAG_SYNC => {
                let doc_id = read_doc_id(src, &mut pos)?;
                let update_v2 = read_bytes(src, &mut pos)?.to_vec();
                Ok(SyncMessage::Sync { doc_id, update_v2 })
            }
            other => Err(ProtocolError::InvalidTag(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(msg: SyncMessage) -> SyncMessage {
        let encoded = msg.encode();
        SyncMessage::decode(&encoded).unwrap()
    }

    #[test]
    fn roundtrip_update() {
        let msg = SyncMessage::Update {
            doc_id: DocId::from("doc-1"),
            update_v2: vec![10, 20, 30],
        };
        assert_eq!(roundtrip(msg), SyncMessage::Update {
            doc_id: DocId::from("doc-1"),
            update_v2: vec![10, 20, 30],
        });
    }

    #[test]
    fn roundtrip_awareness() {
        let msg = SyncMessage::Awareness {
            doc_id: DocId::from("doc-2"),
            client_id: 42,
            state: vec![1, 2],
        };
        assert_eq!(roundtrip(msg), SyncMessage::Awareness {
            doc_id: DocId::from("doc-2"),
            client_id: 42,
            state: vec![1, 2],
        });
    }

    #[test]
    fn roundtrip_query() {
        let msg = SyncMessage::Query {
            doc_id: DocId::from("doc-3"),
            state_vector: vec![0xff],
        };
        assert_eq!(roundtrip(msg), SyncMessage::Query {
            doc_id: DocId::from("doc-3"),
            state_vector: vec![0xff],
        });
    }

    #[test]
    fn roundtrip_sync() {
        let msg = SyncMessage::Sync {
            doc_id: DocId::from("doc-4"),
            update_v2: vec![],
        };
        assert_eq!(roundtrip(msg), SyncMessage::Sync {
            doc_id: DocId::from("doc-4"),
            update_v2: vec![],
        });
    }

    #[test]
    fn invalid_tag_rejected() {
        let result = SyncMessage::decode(&[0xAA, 0x00]);
        assert!(matches!(result, Err(ProtocolError::InvalidTag(0xAA))));
    }

    #[test]
    fn truncated_payload_rejected() {
        // Valid tag but no payload.
        let result = SyncMessage::decode(&[TAG_UPDATE]);
        assert!(matches!(result, Err(ProtocolError::TruncatedPayload)));
    }
}
