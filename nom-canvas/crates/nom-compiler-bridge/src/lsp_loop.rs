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

// ---- LspSyncDriver ----------------------------------------------------------

use std::io::{BufRead, Write};

/// Synchronous LSP I/O driver (std::io, no tokio required).
pub struct LspSyncDriver<R: BufRead, W: Write> {
    pub reader: R,
    pub writer: W,
    pub config: LspLoopConfig,
}

impl<R: BufRead, W: Write> LspSyncDriver<R, W> {
    pub fn new(reader: R, writer: W, config: LspLoopConfig) -> Self {
        Self { reader, writer, config }
    }

    /// Read one LSP frame from the reader (Content-Length framing).
    pub fn read_frame(&mut self) -> Option<LspFrame> {
        let mut header = String::new();
        let mut content_length: usize = 0;
        loop {
            header.clear();
            if self.reader.read_line(&mut header).ok()? == 0 {
                return None;
            }
            let trimmed = header.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(rest) = trimmed.strip_prefix("Content-Length: ") {
                content_length = rest.parse().ok()?;
            }
        }
        if content_length == 0 {
            return None;
        }
        if content_length > self.config.max_frame_size {
            return None;
        }
        use std::io::Read;
        let mut buf = String::new();
        (&mut self.reader as &mut dyn BufRead)
            .take(content_length as u64)
            .read_to_string(&mut buf)
            .ok()?;
        if buf.len() < content_length {
            return None;
        }
        // Parse method, id, params_raw from the body JSON.
        let method = {
            let needle = "\"method\":\"";
            let start = buf.find(needle)? + needle.len();
            let end = buf[start..].find('"')? + start;
            buf[start..end].to_string()
        };
        let params_raw = {
            let needle = "\"params\":";
            if let Some(idx) = buf.find(needle) {
                buf[idx + needle.len()..buf.rfind('}').unwrap_or(buf.len())].to_string()
            } else {
                "null".to_string()
            }
        };
        let id: Option<u64> = {
            let needle = "\"id\":";
            buf.find(needle).and_then(|idx| {
                let rest = &buf[idx + needle.len()..];
                let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
                rest[..end].parse().ok()
            })
        };
        let header_bytes = format!("Content-Length: {}\r\n\r\n", content_length);
        Some(LspFrame {
            header_len: header_bytes.len(),
            content_len: content_length,
            method,
            id,
            params_raw,
        })
    }

    /// Write one LSP frame to the writer.
    pub fn write_frame(&mut self, content: &str) -> std::io::Result<()> {
        write!(self.writer, "Content-Length: {}\r\n\r\n{}", content.len(), content)?;
        self.writer.flush()
    }
}

#[cfg(test)]
mod lsp_sync_tests {
    use super::*;
    use std::io::BufReader;

    fn make_lsp_msg(body: &str) -> Vec<u8> {
        format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
    }

    #[test]
    fn test_read_frame_basic() {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let msg = make_lsp_msg(body);
        let cursor = std::io::Cursor::new(msg);
        let reader = BufReader::new(cursor);
        let writer = Vec::<u8>::new();
        let config = LspLoopConfig::default();
        let mut driver = LspSyncDriver::new(reader, writer, config);
        let frame = driver.read_frame();
        assert!(frame.is_some());
        let f = frame.unwrap();
        assert_eq!(f.method, "initialize");
    }

    #[test]
    fn test_write_frame() {
        let body = r#"{"jsonrpc":"2.0","result":{},"id":1}"#;
        let cursor = std::io::Cursor::new(vec![]);
        let reader = BufReader::new(cursor);
        let mut out = Vec::<u8>::new();
        let config = LspLoopConfig::default();
        let mut driver = LspSyncDriver::new(reader, &mut out, config);
        driver.write_frame(body).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Content-Length:"));
        assert!(output.contains("jsonrpc"));
    }

    #[test]
    fn test_lsp_sync_driver_new() {
        let cursor = std::io::Cursor::new(vec![]);
        let reader = BufReader::new(cursor);
        let writer = Vec::<u8>::new();
        let config = LspLoopConfig { read_timeout_ms: 500, max_frame_size: 512 };
        let driver = LspSyncDriver::new(reader, writer, config);
        assert_eq!(driver.config.read_timeout_ms, 500);
        assert_eq!(driver.config.max_frame_size, 512);
    }

    #[test]
    fn test_read_frame_empty() {
        let cursor = std::io::Cursor::new(vec![]);
        let reader = BufReader::new(cursor);
        let writer = Vec::<u8>::new();
        let config = LspLoopConfig::default();
        let mut driver = LspSyncDriver::new(reader, writer, config);
        let frame = driver.read_frame();
        assert!(frame.is_none());
    }

    #[test]
    fn test_read_multiple_frames() {
        let body1 = r#"{"id":1,"method":"a","params":null}"#;
        let body2 = r#"{"id":2,"method":"b","params":null}"#;
        let mut data = make_lsp_msg(body1);
        data.extend(make_lsp_msg(body2));
        let cursor = std::io::Cursor::new(data);
        let reader = BufReader::new(cursor);
        let writer = Vec::<u8>::new();
        let config = LspLoopConfig::default();
        let mut driver = LspSyncDriver::new(reader, writer, config);
        let f1 = driver.read_frame();
        let f2 = driver.read_frame();
        assert!(f1.is_some());
        assert!(f2.is_some());
        assert_eq!(f1.unwrap().method, "a");
        assert_eq!(f2.unwrap().method, "b");
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
