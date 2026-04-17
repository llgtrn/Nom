//! LSP client — spawns nom-lsp as child process, communicates via JSON-RPC stdio.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use serde_json::Value;

/// LSP client state
pub struct LspClient {
    child: Child,
    next_id: u64,
}

impl LspClient {
    /// Spawn the nom-lsp binary
    pub fn spawn(lsp_binary: &str) -> Result<Self, String> {
        let child = Command::new(lsp_binary)
            .args(["serve"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn LSP: {e}"))?;

        Ok(Self { child, next_id: 1 })
    }

    /// Send a JSON-RPC request and get the response
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let body = serde_json::to_string(&request).map_err(|e| format!("{e}"))?;
        let message = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

        // Write to stdin
        let stdin = self.child.stdin.as_mut().ok_or("No stdin")?;
        stdin.write_all(message.as_bytes()).map_err(|e| format!("Write error: {e}"))?;
        stdin.flush().map_err(|e| format!("Flush error: {e}"))?;

        // Read response from stdout
        let stdout = self.child.stdout.as_mut().ok_or("No stdout")?;
        let mut reader = BufReader::new(stdout);

        // Read headers
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| format!("Read error: {e}"))?;
            let trimmed = line.trim();
            if trimmed.is_empty() { break; }
            if let Some(len) = trimmed.strip_prefix("Content-Length: ") {
                content_length = len.parse().unwrap_or(0);
            }
        }

        if content_length == 0 {
            return Err("Empty response".to_string());
        }

        // Read body
        let mut body_buf = vec![0u8; content_length];
        std::io::Read::read_exact(&mut reader, &mut body_buf)
            .map_err(|e| format!("Body read error: {e}"))?;

        let response: Value = serde_json::from_slice(&body_buf)
            .map_err(|e| format!("Parse error: {e}"))?;

        Ok(response)
    }

    /// Send a JSON-RPC request and get the response, with a deadline in milliseconds.
    ///
    /// The write to stdin is synchronous (fast). The blocking read from stdout is
    /// off-loaded to a dedicated thread; the calling thread waits at most
    /// `timeout_ms` milliseconds before returning an error.
    pub fn request_with_timeout(
        &mut self,
        method: &str,
        params: Value,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;

        // Build and send the request.
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let body = serde_json::to_string(&request).map_err(|e| format!("{e}"))?;
        let message = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let stdin = self.child.stdin.as_mut().ok_or("No stdin")?;
        stdin.write_all(message.as_bytes()).map_err(|e| format!("Write error: {e}"))?;
        stdin.flush().map_err(|e| format!("Flush error: {e}"))?;

        // Take stdout out of the child so the reader thread can own it.
        let stdout: ChildStdout = self.child.stdout.take().ok_or("No stdout")?;

        let (tx, rx) = std::sync::mpsc::channel::<Result<(Vec<u8>, ChildStdout), String>>();

        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut content_length = 0usize;

            // Read LSP headers.
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Err(e) => {
                        let _ = tx.send(Err(format!("Header read error: {e}")));
                        return;
                    }
                    Ok(0) => {
                        let _ = tx.send(Err("EOF in headers".to_string()));
                        return;
                    }
                    Ok(_) => {}
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break;
                }
                if let Some(len) = trimmed.strip_prefix("Content-Length: ") {
                    content_length = len.parse().unwrap_or(0);
                }
            }

            if content_length == 0 {
                let _ = tx.send(Err("Empty response (content-length 0)".to_string()));
                return;
            }

            // Read body.
            let mut buf = vec![0u8; content_length];
            match std::io::Read::read_exact(&mut reader, &mut buf) {
                Err(e) => {
                    let _ = tx.send(Err(format!("Body read error: {e}")));
                }
                Ok(()) => {
                    // Return the buffer and the unwrapped stdout so the caller can
                    // put it back into Child.
                    let inner = reader.into_inner();
                    let _ = tx.send(Ok((buf, inner)));
                }
            }
        });

        match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
            Ok(Ok((buf, stdout_back))) => {
                // Restore stdout so subsequent requests work.
                self.child.stdout = Some(stdout_back);
                serde_json::from_slice(&buf).map_err(|e| format!("Parse error: {e}"))
            }
            Ok(Err(e)) => {
                // Reader thread failed; stdout is gone — kill & respawn on next call.
                Err(e)
            }
            Err(_) => {
                // Timeout — stdout is gone; the child will be considered dead on next call.
                Err(format!("LSP request timed out after {timeout_ms}ms"))
            }
        }
    }

    /// Send a notification (no response expected)
    pub fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let body = serde_json::to_string(&notification).map_err(|e| format!("{e}"))?;
        let message = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

        let stdin = self.child.stdin.as_mut().ok_or("No stdin")?;
        stdin.write_all(message.as_bytes()).map_err(|e| format!("{e}"))?;
        stdin.flush().map_err(|e| format!("{e}"))?;
        Ok(())
    }

    /// Initialize the LSP connection
    pub fn initialize(&mut self) -> Result<Value, String> {
        self.request("initialize", serde_json::json!({
            "processId": std::process::id(),
            "capabilities": {},
            "rootUri": null,
        }))
    }

    /// Shutdown the LSP
    pub fn shutdown(&mut self) -> Result<(), String> {
        let _ = self.request("shutdown", Value::Null);
        let _ = self.notify("exit", Value::Null);
        let _ = self.child.kill();
        Ok(())
    }

    /// Check if the child process is still running
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// Global LSP client (lazily spawned)
static LSP_CLIENT: std::sync::LazyLock<Mutex<Option<LspClient>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Get or spawn the LSP client
pub fn get_or_spawn_lsp() -> Result<std::sync::MutexGuard<'static, Option<LspClient>>, String> {
    let mut guard = LSP_CLIENT.lock().map_err(|e| format!("Lock error: {e}"))?;
    if guard.is_none() || !guard.as_mut().map(|c| c.is_alive()).unwrap_or(false) {
        // Try to find nom binary
        let nom_binary = which_nom_binary();
        match LspClient::spawn(&nom_binary) {
            Ok(mut client) => {
                let _ = client.initialize();
                *guard = Some(client);
            }
            Err(e) => {
                *guard = None;
                return Err(format!("LSP spawn failed: {e}"));
            }
        }
    }
    Ok(guard)
}

fn which_nom_binary() -> String {
    // Try common locations
    for candidate in ["nom", "nom.exe", "./target/debug/nom", "./target/release/nom"] {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }
    // Try the compiler workspace
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../nom-compiler/target/debug/nom");
    if workspace.exists() {
        return workspace.to_string_lossy().to_string();
    }
    "nom".to_string() // fallback to PATH
}
