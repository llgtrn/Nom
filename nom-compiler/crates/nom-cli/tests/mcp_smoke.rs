//! Smoke test for `nom mcp serve`.
//!
//! Spawns the MCP server with a temp dict, sends JSON-RPC requests over
//! stdin, reads responses from stdout, and asserts protocol correctness.
//!
//! The test is gated on a child-process stdin/stdout plumbing approach that
//! works on all platforms supported by the workspace (Windows + POSIX).

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn nom_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nom"))
}

fn make_tmpdir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("nom-mcp-{tag}-{pid}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create tmp");
    dir
}

/// Send one JSON-RPC line and return the parsed response line.
fn exchange(
    stdin: &mut std::process::ChildStdin,
    stdout_lines: &mut std::io::Lines<BufReader<std::process::ChildStdout>>,
    payload: &str,
) -> serde_json::Value {
    writeln!(stdin, "{payload}").expect("write to mcp stdin");
    stdin.flush().expect("flush");
    let line = stdout_lines
        .next()
        .expect("expected response line")
        .expect("io");
    serde_json::from_str(&line).expect("response is valid JSON")
}

#[test]
fn mcp_initialize_handshake() {
    let tmp = make_tmpdir("init");
    let dict_path = tmp.join("nomdict.db");

    let mut child = Command::new(nom_bin())
        .args(["mcp", "--dict", &dict_path.to_string_lossy()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn nom mcp serve");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout_buf = BufReader::new(child.stdout.take().expect("stdout"));
    let mut lines = stdout_buf.lines();

    // ── initialize ────────────────────────────────────────────────────
    let resp = exchange(
        &mut stdin,
        &mut lines,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    assert_eq!(resp["jsonrpc"], "2.0", "jsonrpc field");
    assert_eq!(resp["id"], 1, "echoed id");
    assert_eq!(
        resp["result"]["protocolVersion"], "2024-11-05",
        "protocol version"
    );
    assert_eq!(
        resp["result"]["serverInfo"]["name"], "nom-mcp",
        "server name"
    );

    // ── tools/list ────────────────────────────────────────────────────
    let resp2 = exchange(
        &mut stdin,
        &mut lines,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    );
    assert_eq!(resp2["id"], 2, "echoed id for tools/list");
    let tools = resp2["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 3, "exactly 3 tools");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"list_nomtu"), "list_nomtu present");
    assert!(names.contains(&"get_nomtu"), "get_nomtu present");
    assert!(names.contains(&"search_nomtu"), "search_nomtu present");

    // ── list_nomtu on empty dict ───────────────────────────────────────
    let resp3 = exchange(
        &mut stdin,
        &mut lines,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_nomtu","arguments":{"limit":5}}}"#,
    );
    assert_eq!(resp3["id"], 3, "echoed id for list_nomtu");
    // Empty dict → content[0].text starts with "0 entries matched."
    let text = resp3["result"]["content"][0]["text"]
        .as_str()
        .expect("content text");
    assert!(text.contains("0 entries matched"), "empty dict: {text}");

    // ── search_nomtu on empty dict ────────────────────────────────────
    let resp4 = exchange(
        &mut stdin,
        &mut lines,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"search_nomtu","arguments":{"query":"hash"}}}"#,
    );
    assert_eq!(resp4["id"], 4);
    let text4 = resp4["result"]["content"][0]["text"]
        .as_str()
        .expect("content text");
    assert!(
        text4.contains("0 entries matched"),
        "empty dict search: {text4}"
    );

    // ── get_nomtu with unknown hash ───────────────────────────────────
    let resp5 = exchange(
        &mut stdin,
        &mut lines,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_nomtu","arguments":{"hash":"deadbeef"}}}"#,
    );
    assert_eq!(resp5["id"], 5);
    // Expect an error result (entry not found or prefix too short)
    assert!(
        resp5.get("error").is_some()
            || resp5["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .is_empty(),
        "should error on unknown hash"
    );

    // ── ping ──────────────────────────────────────────────────────────
    let resp6 = exchange(
        &mut stdin,
        &mut lines,
        r#"{"jsonrpc":"2.0","id":6,"method":"ping","params":{}}"#,
    );
    assert_eq!(resp6["id"], 6, "ping echoed id");
    assert!(resp6.get("error").is_none(), "ping should not error");

    // Shut down
    drop(stdin);
    let _ = child.wait();
}
