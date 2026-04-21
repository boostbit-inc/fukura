//! Model Context Protocol (MCP) server.
//!
//! Exposes fukura's adapter pipeline and local note repository to any
//! MCP-compatible agent (Claude Code, Cursor, custom clients, ...).
//!
//! Wire format: JSON-RPC 2.0 over stdio, one JSON object per line. All
//! diagnostic output is written to stderr; stdout is reserved for
//! protocol traffic.
//!
//! v0.1 implements the minimum lifecycle (`initialize`,
//! `notifications/initialized`) and tool surface (`tools/list`,
//! `tools/call`) needed by MCP clients. Resources, prompts, and
//! sampling are not implemented yet.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use crate::adapter::{enrich, InvocationContext};
use crate::domain::attempt::{AttemptOutcome, SolutionAttempt};
use crate::index::SearchSort;
use crate::infrastructure::attempt_storage::AttemptStore;
use crate::models::{Author, Note, Privacy};
use crate::repo::FukuraRepo;

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
const SERVER_NAME: &str = "fukura";

/// Spec version exposed via `initialize`. Bumped when wire-incompatible
/// changes are made to fukura's MCP surface.
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Entry point for the `fuku mcp` subcommand. Discovers a repository
/// (when one is available) and runs the JSON-RPC loop until stdin closes.
pub async fn run(repo_path: Option<PathBuf>) -> Result<()> {
    let repo = match repo_path {
        Some(p) => Some(Arc::new(FukuraRepo::discover(Some(&p))?)),
        None => FukuraRepo::discover(None).ok().map(Arc::new),
    };

    let server = Server::new(repo);
    server.serve_stdio().await
}

struct Server {
    repo: Option<Arc<FukuraRepo>>,
    /// Marks whether the client has completed the initialise handshake.
    /// Tool calls before this are rejected with a JSON-RPC error.
    initialised: Arc<Mutex<bool>>,
}

impl Server {
    fn new(repo: Option<Arc<FukuraRepo>>) -> Self {
        Self {
            repo,
            initialised: Arc::new(Mutex::new(false)),
        }
    }

    async fn serve_stdio(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        let stdout = tokio::io::stdout();
        let writer = Arc::new(Mutex::new(stdout));

        while let Some(line) = reader.next_line().await? {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let response = match self.handle_line(line).await {
                Ok(Some(resp)) => Some(resp),
                Ok(None) => None, // notification: no response expected
                Err(err) => {
                    tracing::error!("mcp dispatch error: {err:#}");
                    Some(error_response(Value::Null, -32603, &err.to_string()))
                }
            };

            if let Some(resp) = response {
                let mut buf = serde_json::to_vec(&resp)?;
                buf.push(b'\n');
                let mut w = writer.lock().await;
                w.write_all(&buf).await?;
                w.flush().await?;
            }
        }

        Ok(())
    }

    async fn handle_line(&self, line: &str) -> Result<Option<Value>> {
        let req: JsonRpcRequest = serde_json::from_str(line).context("invalid JSON-RPC payload")?;

        // Notifications carry no `id` and never receive a response.
        let is_notification = req.id.is_none();

        match req.method.as_str() {
            "initialize" => {
                let result = self.handle_initialize();
                Ok(Some(success_response(
                    req.id.unwrap_or(Value::Null),
                    result,
                )))
            }
            "notifications/initialized" => {
                *self.initialised.lock().await = true;
                Ok(None)
            }
            "tools/list" => {
                let result = json!({ "tools": tool_definitions() });
                Ok(Some(success_response(
                    req.id.unwrap_or(Value::Null),
                    result,
                )))
            }
            "tools/call" => {
                if !*self.initialised.lock().await {
                    return Ok(Some(error_response(
                        req.id.unwrap_or(Value::Null),
                        -32002,
                        "server not initialised",
                    )));
                }
                let id = req.id.clone().unwrap_or(Value::Null);
                match self.dispatch_tool(req.params.unwrap_or(Value::Null)).await {
                    Ok(content) => Ok(Some(success_response(
                        id,
                        json!({ "content": [content], "isError": false }),
                    ))),
                    Err(err) => Ok(Some(success_response(
                        id,
                        json!({
                            "content": [{ "type": "text", "text": format!("error: {err:#}") }],
                            "isError": true
                        }),
                    ))),
                }
            }
            "ping" => Ok(Some(success_response(
                req.id.unwrap_or(Value::Null),
                json!({}),
            ))),
            other => {
                if is_notification {
                    Ok(None)
                } else {
                    Ok(Some(error_response(
                        req.id.unwrap_or(Value::Null),
                        -32601,
                        &format!("method not found: {other}"),
                    )))
                }
            }
        }
    }

    fn handle_initialize(&self) -> Value {
        json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {
                "tools": { "listChanged": false }
            },
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION,
            },
        })
    }

    async fn dispatch_tool(&self, params: Value) -> Result<Value> {
        let call: ToolCall = serde_json::from_value(params).context("invalid tool call params")?;
        let args = call.arguments.unwrap_or(Value::Null);

        let text = match call.name.as_str() {
            "fukura_classify" => self.tool_classify(args)?,
            "fukura_search" => self.tool_search(args)?,
            "fukura_record" => self.tool_record(args)?,
            "fukura_preflight" => self.tool_preflight(args)?,
            "fukura_record_attempt" => self.tool_record_attempt(args)?,
            other => anyhow::bail!("unknown tool: {other}"),
        };

        Ok(json!({ "type": "text", "text": text }))
    }

    fn tool_classify(&self, args: Value) -> Result<String> {
        let req: ClassifyRequest = serde_json::from_value(args).context("classify args")?;
        let ctx = req.into_context();
        let ontology = enrich::default_registry().classify(&ctx);
        Ok(serde_json::to_string_pretty(&ontology)?)
    }

    fn tool_search(&self, args: Value) -> Result<String> {
        let req: SearchRequest = serde_json::from_value(args).context("search args")?;
        let repo = self.require_repo()?;
        let limit = req.limit.unwrap_or(10).clamp(1, 100);
        let hits = repo.search(&req.query, limit, SearchSort::Relevance)?;
        Ok(serde_json::to_string_pretty(&hits)?)
    }

    fn tool_record(&self, args: Value) -> Result<String> {
        let req: RecordRequest = serde_json::from_value(args).context("record args")?;
        let repo = self.require_repo()?;
        let ctx = req
            .invocation
            .into_context_with_agent(req.agent_kind.clone());

        let now = chrono::Utc::now();
        let mut note = Note {
            title: req
                .title
                .unwrap_or_else(|| format!("Error: {}", first_token(&ctx.command))),
            body: req.body.unwrap_or_else(|| default_body(&ctx)),
            tags: req.tags.unwrap_or_else(|| {
                let mut t = vec!["mcp".to_string()];
                if let Some(kind) = &ctx.agent_kind {
                    t.push(format!("agent:{kind}"));
                }
                t
            }),
            links: vec![],
            meta: Default::default(),
            solutions: vec![],
            privacy: Privacy::Private,
            created_at: now,
            updated_at: now,
            author: Author {
                name: req
                    .author
                    .unwrap_or_else(|| ctx.agent_kind.clone().unwrap_or_else(|| "mcp".into())),
                email: None,
            },
            ontology: None,
        };

        let attached = enrich::enrich_note(&mut note, &ctx);
        let record = repo.store_note(note)?;

        Ok(serde_json::to_string_pretty(&json!({
            "object_id": record.object_id,
            "ontology_attached": attached,
            "ontology": record.note.ontology,
        }))?)
    }

    fn tool_preflight(&self, args: Value) -> Result<String> {
        let req: PreflightRequest = serde_json::from_value(args).context("preflight args")?;
        let repo = self.require_repo()?;

        // Start with a keyword search off the command head, which is
        // cheap and catches text-level overlaps even when an adapter
        // cannot synthesise a tight fingerprint.
        let head = first_token(&req.command);
        let hits = repo
            .search(head, 5, SearchSort::Relevance)
            .unwrap_or_default();

        // Then ask the adapter registry for a *predicted* fingerprint
        // from the command alone (no stderr / exit_code yet). When the
        // prediction lines up with a note's stored fingerprint, that
        // note is a tighter match than generic keyword hits and gets
        // surfaced first.
        let predicted_fingerprint = {
            let registry = crate::adapter::AdapterRegistry::with_builtins();
            let ctx = crate::adapter::InvocationContext {
                command: req.command.clone(),
                exit_code: Some(1),
                stderr: Some(String::new()),
                ..Default::default()
            };
            registry.synthesise_pre_fingerprint(&ctx)
        };

        let store = AttemptStore::open(repo.dot_dir()).ok();
        let stats_by_fp = store
            .as_ref()
            .and_then(|s| s.stats_by_fingerprint().ok())
            .unwrap_or_default();

        let mut annotated: Vec<(bool, Value)> = hits
            .into_iter()
            .map(|h| {
                // Resolve fingerprint by loading the note; SearchHit does
                // not carry structured ontology data yet. Preflight
                // queries are rare and bounded (<= 5 hits), so the extra
                // load is acceptable in exchange for linking effectiveness
                // stats per result.
                let fp = repo
                    .load_note(&h.object_id)
                    .ok()
                    .and_then(|r| r.note.ontology.map(|o| o.fingerprint));
                let stats = fp.as_deref().and_then(|f| stats_by_fp.get(f));
                let matches_prediction = fp
                    .as_deref()
                    .zip(predicted_fingerprint.as_deref())
                    .map(|(a, b)| a == b)
                    .unwrap_or(false);
                let entry = json!({
                    "note_id": h.object_id,
                    "title": h.title,
                    "tags": h.tags,
                    "snippet": h.summary,
                    "fingerprint": fp,
                    "matches_predicted_fingerprint": matches_prediction,
                    "effectiveness": stats.map(|s| json!({
                        "success": s.success,
                        "failure": s.failure,
                        "abandoned": s.abandoned,
                        "total": s.total(),
                        "success_rate": s.success_rate(),
                    })),
                });
                (matches_prediction, entry)
            })
            .collect();

        // Stable sort: prediction-matches first, everything else in
        // original order.
        annotated.sort_by(|a, b| b.0.cmp(&a.0));
        let warnings: Vec<Value> = annotated.into_iter().map(|(_, v)| v).collect();

        Ok(serde_json::to_string_pretty(&json!({
            "command": req.command,
            "predicted_fingerprint": predicted_fingerprint,
            "warnings": warnings,
        }))?)
    }

    fn tool_record_attempt(&self, args: Value) -> Result<String> {
        let req: RecordAttemptRequest =
            serde_json::from_value(args).context("record_attempt args")?;
        let repo = self.require_repo()?;
        let store = AttemptStore::open(repo.dot_dir())?;

        let mut attempt = SolutionAttempt::new(&req.fingerprint, req.outcome);
        attempt.suggested_note_id = req.suggested_note_id;
        attempt.next_command = req.next_command;
        attempt.agent_kind = req.agent_kind;
        store.record(&attempt)?;

        let stats = store.stats_for(&req.fingerprint)?;
        Ok(serde_json::to_string_pretty(&json!({
            "attempt_id": attempt.attempt_id,
            "fingerprint": attempt.fingerprint,
            "stats": {
                "success": stats.success,
                "failure": stats.failure,
                "abandoned": stats.abandoned,
                "total": stats.total(),
                "success_rate": stats.success_rate(),
            }
        }))?)
    }

    fn require_repo(&self) -> Result<&Arc<FukuraRepo>> {
        self.repo
            .as_ref()
            .context("no fukura repository found in current directory; run `fuku init` first")
    }
}

// ---------------------------------------------------------------------------
// Tool definitions exposed via `tools/list`.
// ---------------------------------------------------------------------------

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "fukura_classify",
            "description": "Classify a command invocation into an Error Knowledge Protocol (EKP) ontology without persisting anything. Returns null when no adapter recognises the invocation (typically a successful command).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Full command line that was run" },
                    "exit_code": { "type": "integer", "description": "Exit status of the command" },
                    "stdout": { "type": "string" },
                    "stderr": { "type": "string" },
                    "working_directory": { "type": "string" },
                    "duration_ms": { "type": "integer" }
                },
                "required": ["command", "exit_code"]
            }
        }),
        json!({
            "name": "fukura_search",
            "description": "Full-text search the local fukura repository for prior notes matching a query.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 10 }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "fukura_record",
            "description": "Classify an invocation, build a note from it, persist it to the local repository, and return the new note id along with the attached ontology. Use this from autonomous agents to log every failing tool call so the next run benefits from the lesson.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "invocation": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string" },
                            "exit_code": { "type": "integer" },
                            "stdout": { "type": "string" },
                            "stderr": { "type": "string" },
                            "working_directory": { "type": "string" },
                            "duration_ms": { "type": "integer" }
                        },
                        "required": ["command", "exit_code"]
                    },
                    "title": { "type": "string" },
                    "body": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "author": { "type": "string" },
                    "agent_kind": { "type": "string", "description": "e.g. 'claude-code', 'cursor', 'devin'" }
                },
                "required": ["invocation"]
            }
        }),
        json!({
            "name": "fukura_preflight",
            "description": "Look up prior notes related to the given command before it is run. Each returned warning carries an `effectiveness` object with measured success / failure / abandoned counts, so agents can prefer solutions that historically worked.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "fukura_record_attempt",
            "description": "Record the outcome of trying a solution against an EKP fingerprint. Agents (and shell hooks) call this after running a follow-up command so fukura can measure which solutions actually work. Outcomes: success (next command succeeded), failure (next command also failed), abandoned (no resolution reached).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "fingerprint": { "type": "string", "description": "EKP fingerprint of the error being attempted against" },
                    "outcome": { "type": "string", "enum": ["success", "failure", "abandoned"] },
                    "suggested_note_id": { "type": "string" },
                    "next_command": { "type": "string" },
                    "agent_kind": { "type": "string" }
                },
                "required": ["fingerprint", "outcome"]
            }
        }),
    ]
}

// ---------------------------------------------------------------------------
// JSON-RPC helpers and request DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    id: Option<Value>,
}

fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message }})
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    name: String,
    #[serde(default)]
    arguments: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ClassifyRequest {
    command: String,
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
    working_directory: Option<String>,
    duration_ms: Option<u64>,
}

impl ClassifyRequest {
    fn into_context(self) -> InvocationContext {
        InvocationContext {
            command: self.command,
            exit_code: self.exit_code,
            stdout: self.stdout,
            stderr: self.stderr,
            working_directory: self.working_directory,
            duration_ms: self.duration_ms,
            shell: None,
            agent_kind: None,
        }
    }

    fn into_context_with_agent(self, agent: Option<String>) -> InvocationContext {
        let mut ctx = self.into_context();
        ctx.agent_kind = agent;
        ctx
    }
}

#[derive(Debug, Deserialize)]
struct SearchRequest {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RecordRequest {
    invocation: ClassifyRequest,
    title: Option<String>,
    body: Option<String>,
    tags: Option<Vec<String>>,
    author: Option<String>,
    agent_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PreflightRequest {
    command: String,
}

#[derive(Debug, Deserialize)]
struct RecordAttemptRequest {
    fingerprint: String,
    outcome: AttemptOutcome,
    #[serde(default)]
    suggested_note_id: Option<String>,
    #[serde(default)]
    next_command: Option<String>,
    #[serde(default)]
    agent_kind: Option<String>,
}

fn first_token(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or(s)
}

fn default_body(ctx: &InvocationContext) -> String {
    let mut body = String::from("## Invocation\n\n```bash\n");
    body.push_str(&ctx.command);
    body.push_str("\n```\n\n");
    if let Some(code) = ctx.exit_code {
        body.push_str(&format!("Exit code: {code}\n\n"));
    }
    if let Some(stderr) = &ctx.stderr {
        body.push_str("### stderr\n\n```\n");
        body.push_str(stderr);
        body.push_str("\n```\n");
    }
    body
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ontology::ErrorOntology;

    fn server_without_repo() -> Server {
        Server::new(None)
    }

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let server = server_without_repo();
        let resp = server
            .handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(resp["result"]["serverInfo"]["name"], "fukura");
        assert!(resp["result"]["protocolVersion"].is_string());
        assert_eq!(resp["id"], 1);
    }

    #[tokio::test]
    async fn initialized_notification_produces_no_response() {
        let server = server_without_repo();
        let resp = server
            .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .await
            .unwrap();
        assert!(resp.is_none());
        assert!(*server.initialised.lock().await);
    }

    #[tokio::test]
    async fn tools_list_includes_all_four_tools() {
        let server = server_without_repo();
        let resp = server
            .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)
            .await
            .unwrap()
            .unwrap();

        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"fukura_classify"));
        assert!(names.contains(&"fukura_search"));
        assert!(names.contains(&"fukura_record"));
        assert!(names.contains(&"fukura_preflight"));
    }

    #[tokio::test]
    async fn tools_call_before_initialise_is_rejected() {
        let server = server_without_repo();
        let resp = server
            .handle_line(
                r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"fukura_classify","arguments":{"command":"cargo build","exit_code":1}}}"#,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resp["error"]["code"], -32002);
    }

    #[tokio::test]
    async fn classify_tool_returns_cargo_ontology() {
        let server = server_without_repo();
        // Mark initialised manually for the test.
        *server.initialised.lock().await = true;

        let resp = server
            .handle_line(
                r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"fukura_classify","arguments":{"command":"cargo build","exit_code":101,"stderr":"error[E0308]: mismatched types"}}}"#,
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let ontology: ErrorOntology = serde_json::from_str(text).unwrap();
        assert_eq!(ontology.adapter, "cargo");
        assert_eq!(ontology.category, "cargo.compile.e0308");
    }

    #[tokio::test]
    async fn record_tool_requires_repository() {
        let server = server_without_repo();
        *server.initialised.lock().await = true;

        let resp = server
            .handle_line(
                r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"fukura_record","arguments":{"invocation":{"command":"cargo build","exit_code":101}}}}"#,
            )
            .await
            .unwrap()
            .unwrap();

        // Tool errors are returned as isError content rather than JSON-RPC errors.
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("no fukura repository"));
    }

    #[tokio::test]
    async fn record_tool_persists_and_attaches_ontology() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = Arc::new(FukuraRepo::init(tmp.path(), true).unwrap());
        let server = Server::new(Some(repo.clone()));
        *server.initialised.lock().await = true;

        let resp = server
            .handle_line(
                r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"fukura_record","arguments":{"invocation":{"command":"cargo build","exit_code":101,"stderr":"error[E0308]: mismatched types"},"agent_kind":"claude-code"}}}"#,
            )
            .await
            .unwrap()
            .unwrap();

        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["ontology_attached"], true);

        let id = parsed["object_id"].as_str().unwrap();
        let loaded = repo.load_note(id).unwrap();
        assert_eq!(
            loaded.note.ontology.unwrap().category,
            "cargo.compile.e0308"
        );
    }

    #[tokio::test]
    async fn unknown_method_yields_method_not_found() {
        let server = server_without_repo();
        let resp = server
            .handle_line(r#"{"jsonrpc":"2.0","id":99,"method":"does/not/exist"}"#)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }
}
