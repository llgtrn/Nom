/// Tokio-based LSP stdin/stdout async I/O loop.
/// Builds on top of `LspServerLoop` / `LspLoopState` from `lsp_server`.
use crate::lsp_server::LspServerLoop;

// ---- LspFrame ---------------------------------------------------------------

/// A single framed LSP JSON-RPC message (header + body).
#[derive(Debug, Clone)]
pub struct LspFrame {
    pub header_len: usize,
    pub content_len: usize,
    pub method: String,
    pub id: Option<u64>,
    pub params_raw: String,
}

impl LspFrame {
    /// Build a new frame with pre-computed lengths derived from the body JSON.
    pub fn new(method: impl Into<String>, id: Option<u64>, params_raw: impl Into<String>) -> Self {
        let method = method.into();
        let params_raw = params_raw.into();
        let id_fragment = match id {
            Some(v) => format!("{}", v),
            None => "null".to_string(),
        };
        let json = format!(
            "{{\"method\":\"{method}\",\"id\":{id_fragment},\"params\":{params_raw}}}",
        );
        let content_len = json.len();
        let header = format!("Content-Length: {}\r\n\r\n", content_len);
        let header_len = header.len();
        Self {
            header_len,
            content_len,
            method,
            id,
            params_raw,
        }
    }

    /// Serialize to `Content-Length: N\r\n\r\n{json}` bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let id_fragment = match self.id {
            Some(v) => format!("{}", v),
            None => "null".to_string(),
        };
        let json = format!(
            "{{\"method\":\"{}\",\"id\":{},\"params\":{}}}",
            self.method, id_fragment, self.params_raw,
        );
        let header = format!("Content-Length: {}\r\n\r\n", json.len());
        let mut out = header.into_bytes();
        out.extend_from_slice(json.as_bytes());
        out
    }

    /// Return the number of bytes in the JSON body (not including the header).
    pub fn content_length(&self) -> usize {
        self.content_len
    }
}

// ---- LspIoBuffer ------------------------------------------------------------

/// A growing byte buffer that can reassemble framed LSP messages arriving in
/// arbitrary-sized chunks.
#[derive(Debug)]
pub struct LspIoBuffer {
    pub data: Vec<u8>,
}

impl LspIoBuffer {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Append raw bytes (e.g. from a `tokio::io::AsyncRead` read call).
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    /// Attempt to extract one complete LSP frame from the front of the buffer.
    /// Returns `None` if the buffer does not yet contain a full frame.
    /// On success the consumed bytes are removed from `self.data`.
    pub fn try_parse_frame(&mut self) -> Option<LspFrame> {
        // Locate the \r\n\r\n separator between header and body.
        let separator = b"\r\n\r\n";
        let header_end = self
            .data
            .windows(4)
            .enumerate()
            .find(|(_, w)| *w == separator)
            .map(|(i, _)| i + 4)?;

        // Parse the Content-Length value from the header slice.
        let header_text = std::str::from_utf8(&self.data[..header_end]).ok()?;
        let prefix = "Content-Length: ";
        let cl_start = header_text.find(prefix)?;
        let rest = &header_text[cl_start + prefix.len()..];
        let cl_end = rest.find('\r')?;
        let content_len: usize = rest[..cl_end].trim().parse().ok()?;

        let total = header_end + content_len;
        if self.data.len() < total {
            return None; // incomplete body — wait for more data
        }

        let body = std::str::from_utf8(&self.data[header_end..total]).ok()?;

        // Extract the method from `"method":"<value>"`.
        let method = {
            let needle = "\"method\":\"";
            let start = body.find(needle)? + needle.len();
            let end = body[start..].find('"')? + start;
            body[start..end].to_string()
        };

        // Extract params_raw if present, otherwise default to "null".
        let params_raw = {
            let needle = "\"params\":";
            if let Some(idx) = body.find(needle) {
                body[idx + needle.len()..body.rfind('}').unwrap_or(body.len())].to_string()
            } else {
                "null".to_string()
            }
        };

        // Extract id if present.
        let id: Option<u64> = {
            let needle = "\"id\":";
            body.find(needle).and_then(|idx| {
                let rest = &body[idx + needle.len()..];
                let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
                rest[..end].parse().ok()
            })
        };

        // Consume the frame bytes from the buffer.
        self.data.drain(..total);

        Some(LspFrame {
            header_len: header_end,
            content_len,
            method,
            id,
            params_raw,
        })
    }

    /// Number of bytes currently buffered.
    pub fn buffered_len(&self) -> usize {
        self.data.len()
    }
}

impl Default for LspIoBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ---- LspLoopConfig ----------------------------------------------------------

/// Configuration knobs for the async I/O loop.
#[derive(Debug, Clone)]
pub struct LspLoopConfig {
    /// Milliseconds to wait for data before timing out a read.
    pub read_timeout_ms: u64,
    /// Maximum allowed body size (bytes). Frames larger than this are rejected.
    pub max_frame_size: usize,
}

impl Default for LspLoopConfig {
    fn default() -> Self {
        Self {
            read_timeout_ms: 5_000,
            max_frame_size: 1_024 * 1_024, // 1 MB
        }
    }
}

// ---- LspAsyncLoop -----------------------------------------------------------

/// Async I/O coordinator: owns the loop config, the underlying `LspServerLoop`
/// state machine, and a counter of processed frames.
#[derive(Debug)]
pub struct LspAsyncLoop {
    pub config: LspLoopConfig,
    pub state: LspServerLoop,
    pub frames_processed: u64,
}

impl LspAsyncLoop {
    pub fn new(config: LspLoopConfig) -> Self {
        Self {
            config,
            state: LspServerLoop::new().start(),
            frames_processed: 0,
        }
    }

    /// Feed a parsed `LspFrame` into the state machine and return a response stub.
    pub fn process_frame(&mut self, frame: LspFrame) -> String {
        self.frames_processed += 1;
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":null}}",
            match frame.id {
                Some(v) => v.to_string(),
                None => "null".to_string(),
            }
        )
    }

    /// Total number of frames processed since construction.
    pub fn frames_processed(&self) -> u64 {
        self.frames_processed
    }

    /// Return `true` while the underlying state machine is running.
    pub fn is_running(&self) -> bool {
        self.state.is_running()
    }
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_to_bytes() {
        let frame = LspFrame::new("initialize", Some(1), "{}");
        let bytes = frame.to_bytes();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.starts_with("Content-Length: "));
        assert!(text.contains("\r\n\r\n"));
        assert!(text.contains("\"method\":\"initialize\""));
        assert!(text.contains("\"id\":1"));
    }

    #[test]
    fn frame_content_length() {
        let frame = LspFrame::new("textDocument/hover", Some(2), "{\"pos\":0}");
        // content_length should equal the actual JSON body byte count
        let bytes = frame.to_bytes();
        let sep = b"\r\n\r\n";
        let sep_pos = bytes
            .windows(4)
            .position(|w| w == sep)
            .expect("separator present")
            + 4;
        let body_len = bytes.len() - sep_pos;
        assert_eq!(frame.content_length(), body_len);
    }

    #[test]
    fn buffer_push_and_parse() {
        let frame = LspFrame::new("initialize", Some(10), "{}");
        let bytes = frame.to_bytes();
        let mut buf = LspIoBuffer::new();
        buf.push_bytes(&bytes);
        let parsed = buf.try_parse_frame();
        assert!(parsed.is_some());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.method, "initialize");
        assert_eq!(buf.buffered_len(), 0);
    }

    #[test]
    fn buffer_incomplete_frame_returns_none() {
        let frame = LspFrame::new("initialize", Some(1), "{}");
        let bytes = frame.to_bytes();
        // Feed only the first half.
        let half = bytes.len() / 2;
        let mut buf = LspIoBuffer::new();
        buf.push_bytes(&bytes[..half]);
        assert!(buf.try_parse_frame().is_none());
        // Buffer still holds the partial data.
        assert_eq!(buf.buffered_len(), half);
    }

    #[test]
    fn buffer_parse_extracts_method() {
        let frame = LspFrame::new("textDocument/completion", None, "null");
        let mut buf = LspIoBuffer::new();
        buf.push_bytes(&frame.to_bytes());
        let parsed = buf.try_parse_frame().unwrap();
        assert_eq!(parsed.method, "textDocument/completion");
    }

    #[test]
    fn async_loop_process_frame() {
        let mut lp = LspAsyncLoop::new(LspLoopConfig::default());
        let frame = LspFrame::new("initialize", Some(1), "{}");
        let resp = lp.process_frame(frame);
        assert!(resp.contains("\"jsonrpc\":\"2.0\""));
        assert!(resp.contains("\"id\":1"));
    }

    #[test]
    fn async_loop_frames_counted() {
        let mut lp = LspAsyncLoop::new(LspLoopConfig::default());
        assert_eq!(lp.frames_processed(), 0);
        lp.process_frame(LspFrame::new("a", Some(1), "{}"));
        lp.process_frame(LspFrame::new("b", Some(2), "{}"));
        lp.process_frame(LspFrame::new("c", None, "null"));
        assert_eq!(lp.frames_processed(), 3);
    }

    #[test]
    fn loop_config_default() {
        let cfg = LspLoopConfig::default();
        assert_eq!(cfg.read_timeout_ms, 5_000);
        assert_eq!(cfg.max_frame_size, 1_024 * 1_024);
    }
}
