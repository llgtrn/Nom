use crate::span::SpanContext;

#[derive(Debug, Clone, thiserror::Error)]
pub enum PropagationError {
    #[error("invalid traceparent format")]
    InvalidFormat,
}

pub fn encode_traceparent(ctx: &SpanContext) -> String {
    let trace_hex: String = ctx.trace_id.iter().map(|b| format!("{b:02x}")).collect();
    let span_hex: String = ctx.span_id.iter().map(|b| format!("{b:02x}")).collect();
    let flags = if ctx.sampled { "01" } else { "00" };
    format!("00-{trace_hex}-{span_hex}-{flags}")
}

pub fn decode_traceparent(header: &str) -> Result<SpanContext, PropagationError> {
    let parts: Vec<&str> = header.split('-').collect();
    if parts.len() != 4 {
        return Err(PropagationError::InvalidFormat);
    }
    let version = parts[0];
    let trace_hex = parts[1];
    let span_hex = parts[2];
    let flags_hex = parts[3];

    if version != "00" {
        return Err(PropagationError::InvalidFormat);
    }
    if trace_hex.len() != 32 || span_hex.len() != 16 {
        return Err(PropagationError::InvalidFormat);
    }

    let trace_id = parse_hex_16(trace_hex)?;
    let span_id = parse_hex_8(span_hex)?;

    let sampled = match flags_hex {
        "01" => true,
        "00" => false,
        _ => return Err(PropagationError::InvalidFormat),
    };

    Ok(SpanContext { trace_id, span_id, sampled })
}

fn parse_hex_16(s: &str) -> Result<[u8; 16], PropagationError> {
    let bytes = hex_to_bytes(s)?;
    if bytes.len() != 16 {
        return Err(PropagationError::InvalidFormat);
    }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn parse_hex_8(s: &str) -> Result<[u8; 8], PropagationError> {
    let bytes = hex_to_bytes(s)?;
    if bytes.len() != 8 {
        return Err(PropagationError::InvalidFormat);
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn hex_to_bytes(s: &str) -> Result<Vec<u8>, PropagationError> {
    if s.len() % 2 != 0 {
        return Err(PropagationError::InvalidFormat);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| PropagationError::InvalidFormat)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(sampled: bool) -> SpanContext {
        SpanContext {
            trace_id: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            span_id: [17, 18, 19, 20, 21, 22, 23, 24],
            sampled,
        }
    }

    #[test]
    fn round_trip_sampled() {
        let original = ctx(true);
        let header = encode_traceparent(&original);
        let decoded = decode_traceparent(&header).unwrap();
        assert_eq!(decoded.trace_id, original.trace_id);
        assert_eq!(decoded.span_id, original.span_id);
        assert!(decoded.sampled);
    }

    #[test]
    fn round_trip_not_sampled() {
        let original = ctx(false);
        let header = encode_traceparent(&original);
        let decoded = decode_traceparent(&header).unwrap();
        assert!(!decoded.sampled);
    }

    #[test]
    fn version_not_00_rejected() {
        let header = "01-0102030405060708090a0b0c0d0e0f10-1112131415161718-01";
        assert!(decode_traceparent(header).is_err());
    }

    #[test]
    fn malformed_hex_rejected() {
        let header = "00-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz-1112131415161718-01";
        assert!(decode_traceparent(header).is_err());
    }

    #[test]
    fn wrong_segment_count_rejected() {
        assert!(decode_traceparent("00-abc").is_err());
        assert!(decode_traceparent("").is_err());
    }

    #[test]
    fn empty_header_rejected() {
        assert!(decode_traceparent("").is_err());
    }
}
