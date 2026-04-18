//! End-to-end MCP integration test.
//!
//! Spawns the real `fukura mcp` subcommand as a subprocess and drives it
//! with the exact JSON-RPC wire protocol an MCP client (Claude Code,
//! Cursor, etc.) would use. This is the highest-fidelity check we can
//! run without linking a third-party client: it verifies that fukura
//! ships a working MCP server over stdio, not just that the internal
//! dispatch tables agree.
//!
//! The test also exercises the end-to-end agent loop:
//!
//! 1. Agent hits a cargo error, calls `fukura_classify` to get an
//!    ontology with a stable fingerprint.
//! 2. Agent calls `fukura_record` to persist the error as a note.
//! 3. Agent tries a fix, runs a follow-up command, and reports the
//!    outcome via `fukura_record_attempt`.
//! 4. A *second* agent session calls `fukura_preflight` on a similar
//!    command and receives both the note hit and the measured
//!    effectiveness statistics recorded in step 3.
//!
//! If this test passes, fukura really can act as the shared memory
//! Jensen's "agents banging on tools" picture calls for: every attempt
//! an agent makes gets classified, persisted, and surfaced to the next
//! agent as measured evidence.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use serde_json::{json, Value};

struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpClient {
    fn spawn(repo_path: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_fukura"))
            .args(["mcp", "--repo"])
            .arg(repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn fukura mcp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_line(&req);
        self.read_response(id)
    }

    fn notify(&mut self, method: &str, params: Value) {
        let n = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_line(&n);
    }

    fn call_tool(&mut self, name: &str, args: Value) -> Value {
        let resp = self.request("tools/call", json!({ "name": name, "arguments": args }));
        // Unwrap the MCP content envelope and re-parse the inner JSON text
        // our tools produce. This is the shape every tool in fukura
        // returns: a single "text" content block whose payload is JSON.
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("tool {name} response missing text: {resp:#}"));
        assert_eq!(
            resp["result"]["isError"], false,
            "tool {name} returned error: {text}"
        );
        serde_json::from_str(text)
            .unwrap_or_else(|e| panic!("tool {name} text not JSON ({e}): {text}"))
    }

    fn write_line(&mut self, value: &Value) {
        let mut bytes = serde_json::to_vec(value).unwrap();
        bytes.push(b'\n');
        self.stdin.write_all(&bytes).expect("write stdin");
        self.stdin.flush().expect("flush stdin");
    }

    fn read_response(&mut self, id: u64) -> Value {
        // Server may interleave notifications (none yet, but be robust to
        // future additions). Keep reading until we see our id.
        loop {
            let mut line = String::new();
            let n = self.stdout.read_line(&mut line).expect("read stdout");
            assert!(n > 0, "server closed stdout without replying to id {id}");
            let v: Value = serde_json::from_str(line.trim()).expect("parse json-rpc");
            if v["id"].as_u64() == Some(id) {
                return v;
            }
        }
    }

    fn shutdown(mut self) {
        drop(self.stdin);
        // Give the server a moment to notice EOF before we try to reap.
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn simulates_claude_code_agent_session_end_to_end() {
    let tmp = tempfile::tempdir().expect("tempdir");

    // Bootstrap a repo the same way `fuku init` would. Using the library
    // directly keeps the test focused on MCP wire behaviour.
    {
        let _repo = fukura::repo::FukuraRepo::init(tmp.path(), true).expect("init repo");
    }

    let mut client = McpClient::spawn(tmp.path());

    // --- lifecycle ---
    let init = client.request(
        "initialize",
        json!({
            "protocolVersion": "2025-03-26",
            "clientInfo": { "name": "claude-code-sim", "version": "0.1" },
            "capabilities": {}
        }),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "fukura");
    assert!(init["result"]["protocolVersion"].is_string());

    client.notify("notifications/initialized", json!({}));

    // --- tool discovery (what an MCP client does on connect) ---
    let tools = client.request("tools/list", json!({}));
    let names: Vec<String> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    for expected in [
        "fukura_classify",
        "fukura_search",
        "fukura_record",
        "fukura_preflight",
        "fukura_record_attempt",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "missing tool {expected} in {names:?}"
        );
    }

    // --- step 1: agent hits a cargo error, classifies it ---
    let ontology = client.call_tool(
        "fukura_classify",
        json!({
            "command": "cargo build",
            "exit_code": 101,
            "stderr": "error[E0432]: unresolved import `foo::bar`"
        }),
    );
    assert_eq!(ontology["adapter"], "cargo");
    assert_eq!(ontology["category"], "cargo.compile.e0432");
    let fingerprint = ontology["fingerprint"]
        .as_str()
        .expect("fingerprint present")
        .to_string();
    assert!(fingerprint.starts_with("sha256:"));

    // --- step 2: agent persists the error as a note ---
    let record = client.call_tool(
        "fukura_record",
        json!({
            "invocation": {
                "command": "cargo build",
                "exit_code": 101,
                "stderr": "error[E0432]: unresolved import `foo::bar`"
            },
            "agent_kind": "claude-code",
            "title": "cargo build: unresolved import"
        }),
    );
    assert_eq!(record["ontology_attached"], true);
    let note_id = record["object_id"].as_str().unwrap().to_string();
    assert_eq!(
        record["ontology"]["fingerprint"].as_str().unwrap(),
        fingerprint
    );

    // --- step 3: agent tries a fix and reports the outcome ---
    let attempt = client.call_tool(
        "fukura_record_attempt",
        json!({
            "fingerprint": fingerprint,
            "outcome": "success",
            "suggested_note_id": note_id,
            "next_command": "cargo update -p foo",
            "agent_kind": "claude-code",
        }),
    );
    assert_eq!(attempt["stats"]["success"], 1);
    assert_eq!(attempt["stats"]["total"], 1);

    // Record another success and a failure so the stats have a
    // non-trivial rate (tests that aggregation works across calls).
    client.call_tool(
        "fukura_record_attempt",
        json!({ "fingerprint": fingerprint, "outcome": "success" }),
    );
    let final_attempt = client.call_tool(
        "fukura_record_attempt",
        json!({ "fingerprint": fingerprint, "outcome": "failure" }),
    );
    assert_eq!(final_attempt["stats"]["success"], 2);
    assert_eq!(final_attempt["stats"]["failure"], 1);
    assert!((final_attempt["stats"]["success_rate"].as_f64().unwrap() - 2.0 / 3.0).abs() < 1e-9);

    // --- step 4: a new agent runs preflight on a similar command and
    //     sees both the prior note and the measured effectiveness ---
    let preflight = client.call_tool(
        "fukura_preflight",
        json!({ "command": "cargo build --release" }),
    );
    let warnings = preflight["warnings"].as_array().unwrap();
    assert!(!warnings.is_empty(), "preflight should surface prior note");

    let hit = warnings
        .iter()
        .find(|w| w["note_id"].as_str() == Some(&note_id))
        .expect("our note appears in preflight");
    assert_eq!(hit["fingerprint"].as_str().unwrap(), fingerprint);
    let eff = &hit["effectiveness"];
    assert_eq!(eff["success"], 2);
    assert_eq!(eff["failure"], 1);
    assert_eq!(eff["total"], 3);

    // --- orderly shutdown ---
    client.shutdown();
}

#[test]
fn unknown_tool_returns_is_error_without_crashing_server() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let _repo = fukura::repo::FukuraRepo::init(tmp.path(), true).expect("init repo");

    let mut client = McpClient::spawn(tmp.path());
    client.request("initialize", json!({}));
    client.notify("notifications/initialized", json!({}));

    let resp = client.request(
        "tools/call",
        json!({ "name": "fukura_nonexistent", "arguments": {} }),
    );
    assert_eq!(resp["result"]["isError"], true);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("unknown tool"), "message was: {text}");

    // Server should still answer subsequent requests.
    let tools = client.request("tools/list", json!({}));
    assert!(tools["result"]["tools"].as_array().unwrap().len() >= 5);

    client.shutdown();
}
