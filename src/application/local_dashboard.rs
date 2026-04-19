//! `fukura dashboard` — a browser UI served from the user's own
//! machine, reading their local `.fukura/` store.
//!
//! Solo developers running Claude Code / Cursor all day are the
//! single biggest bottom-up cohort fukura has. They want to see
//! "which errors did I keep hitting this week?" without setting up
//! Postgres, Docker, a hub, or any managed service. This subcommand
//! is that affordance.
//!
//! Shape:
//! - An axum server binds to 127.0.0.1 on a configurable port.
//! - The root path returns a single self-contained HTML document
//!   (CSS + JS inlined — no build pipeline, no external assets).
//! - A few JSON endpoints surface the local AttemptStore /
//!   FukuraRepo data the page needs.
//! - The hosted `/effectiveness` page on fukura-hub intentionally
//!   looks similar to this screen, so "upgrade to team" is a
//!   visual no-op.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::{Query, State};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::attempt_storage::AttemptStore;
use crate::domain::attempt::{AttemptOutcome, SolutionAttempt};
use crate::infrastructure::repo::FukuraRepo;

#[derive(Clone)]
struct LocalState {
    repo: Arc<FukuraRepo>,
}

pub struct DashboardOptions {
    pub port: u16,
    pub open_browser: bool,
}

impl Default for DashboardOptions {
    fn default() -> Self {
        Self {
            port: 8765,
            open_browser: true,
        }
    }
}

pub async fn serve(repo: FukuraRepo, opts: DashboardOptions) -> Result<()> {
    let state = LocalState {
        repo: Arc::new(repo),
    };

    let app = Router::new()
        .route("/", get(index_html))
        .route("/api/local/stats", get(stats))
        .route("/api/local/stats/by-agent", get(stats_by_agent))
        .route("/api/local/notes", get(search_notes))
        .with_state(state);

    let addr: SocketAddr = format!("127.0.0.1:{}", opts.port)
        .parse()
        .context("parsing dashboard bind address")?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding 127.0.0.1:{}", opts.port))?;
    let bound = listener.local_addr()?;

    let url = format!("http://{}", bound);
    println!("▶ fukura dashboard at {url}");
    println!("  Reading {} — serving until you Ctrl-C.", bound);

    if opts.open_browser {
        if let Err(err) = open_in_browser(&url) {
            tracing::debug!("could not auto-open browser: {err:?}");
            println!("  (couldn't auto-open a browser — open the URL above manually)");
        }
    }

    axum::serve(listener, app).await?;
    Ok(())
}

fn open_in_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let prog = "open";
    #[cfg(target_os = "linux")]
    let prog = "xdg-open";
    #[cfg(target_os = "windows")]
    let prog = "cmd";

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        std::process::Command::new(prog)
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new(prog)
            .args(["/C", "start", "", url])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
    }
    Ok(())
}

// ---------- HTML --------------------------------------------------------------

const INDEX_HTML: &str = include_str!("local_dashboard.html");

async fn index_html() -> Html<&'static str> {
    Html(INDEX_HTML)
}

// ---------- JSON endpoints ----------------------------------------------------

async fn stats(State(state): State<LocalState>) -> Response {
    let store = match AttemptStore::open(state.repo.dot_dir()) {
        Ok(s) => s,
        Err(err) => return server_error(format!("opening attempt store: {err}")),
    };
    let attempts = match store.load_all() {
        Ok(a) => a,
        Err(err) => return server_error(format!("loading attempts: {err}")),
    };

    let aggregate = fold_by_fingerprint(&attempts);
    let total = Summary::of(&attempts);

    Json(json!({
        "total": total,
        "by_fingerprint": aggregate,
    }))
    .into_response()
}

async fn stats_by_agent(State(state): State<LocalState>) -> Response {
    let store = match AttemptStore::open(state.repo.dot_dir()) {
        Ok(s) => s,
        Err(err) => return server_error(format!("opening attempt store: {err}")),
    };
    let attempts = match store.load_all() {
        Ok(a) => a,
        Err(err) => return server_error(format!("loading attempts: {err}")),
    };

    let buckets = fold_by_agent(&attempts);
    Json(json!({ "by_agent": buckets })).into_response()
}

#[derive(Deserialize)]
struct NoteQuery {
    fingerprint: Option<String>,
    q: Option<String>,
    limit: Option<usize>,
}

async fn search_notes(
    State(state): State<LocalState>,
    Query(q): Query<NoteQuery>,
) -> Response {
    let limit = q.limit.unwrap_or(20).min(100);
    let query = q.q.as_deref().unwrap_or("");
    let hits = match state
        .repo
        .search(query, limit, crate::infrastructure::index::SearchSort::Relevance)
    {
        Ok(h) => h,
        Err(err) => return server_error(format!("searching notes: {err}")),
    };

    let rows: Vec<Value> = hits
        .into_iter()
        .filter_map(|h| {
            let record = state.repo.load_note(&h.object_id).ok()?;
            let ontology_fp = record
                .note
                .ontology
                .as_ref()
                .map(|o| o.fingerprint.clone());
            if let Some(ref want) = q.fingerprint {
                if ontology_fp.as_deref() != Some(want.as_str()) {
                    return None;
                }
            }
            Some(json!({
                "object_id": h.object_id,
                "title": h.title,
                "fingerprint": ontology_fp,
                "tags": h.tags,
                "summary": h.summary,
                "updated_at": h.updated_at,
            }))
        })
        .collect();

    Json(json!({ "hits": rows })).into_response()
}

fn server_error(msg: String) -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": msg })),
    )
        .into_response()
}

// ---------- aggregation -------------------------------------------------------

#[derive(serde::Serialize, Default)]
struct Summary {
    success: u32,
    failure: u32,
    abandoned: u32,
    total: u32,
    success_rate: Option<f64>,
    unique_fingerprints: usize,
}

impl Summary {
    fn of(attempts: &[SolutionAttempt]) -> Self {
        let mut s = Summary::default();
        let mut seen = std::collections::HashSet::new();
        for a in attempts {
            match a.outcome {
                AttemptOutcome::Success => s.success += 1,
                AttemptOutcome::Failure => s.failure += 1,
                AttemptOutcome::Abandoned => s.abandoned += 1,
            }
            seen.insert(a.fingerprint.clone());
        }
        s.total = s.success + s.failure + s.abandoned;
        let denom = s.success + s.failure;
        s.success_rate = if denom == 0 {
            None
        } else {
            Some(s.success as f64 / denom as f64)
        };
        s.unique_fingerprints = seen.len();
        s
    }
}

fn fold_by_fingerprint(attempts: &[SolutionAttempt]) -> Vec<Value> {
    let mut map: HashMap<String, (u32, u32, u32)> = HashMap::new();
    for a in attempts {
        let entry = map.entry(a.fingerprint.clone()).or_default();
        match a.outcome {
            AttemptOutcome::Success => entry.0 += 1,
            AttemptOutcome::Failure => entry.1 += 1,
            AttemptOutcome::Abandoned => entry.2 += 1,
        }
    }
    let mut rows: Vec<Value> = map
        .into_iter()
        .map(|(fp, (s, f, a))| {
            let total = s + f + a;
            let denom = s + f;
            let rate: Option<f64> = if denom == 0 {
                None
            } else {
                Some(s as f64 / denom as f64)
            };
            json!({
                "fingerprint": fp,
                "success": s,
                "failure": f,
                "abandoned": a,
                "total": total,
                "success_rate": rate,
                "pain": (total as f64) * (1.0 - rate.unwrap_or(0.0)),
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        b["pain"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["pain"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

fn fold_by_agent(attempts: &[SolutionAttempt]) -> Vec<Value> {
    let mut map: HashMap<String, (u32, u32, u32)> = HashMap::new();
    for a in attempts {
        let kind = a.agent_kind.clone().unwrap_or_else(|| "human".to_string());
        let entry = map.entry(kind).or_default();
        match a.outcome {
            AttemptOutcome::Success => entry.0 += 1,
            AttemptOutcome::Failure => entry.1 += 1,
            AttemptOutcome::Abandoned => entry.2 += 1,
        }
    }
    map.into_iter()
        .map(|(k, (s, f, a))| {
            let total = s + f + a;
            let denom = s + f;
            let rate: Option<f64> = if denom == 0 {
                None
            } else {
                Some(s as f64 / denom as f64)
            };
            json!({
                "agent_kind": k,
                "success": s,
                "failure": f,
                "abandoned": a,
                "total": total,
                "success_rate": rate,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(fp: &str, outcome: AttemptOutcome, agent: Option<&str>) -> SolutionAttempt {
        let mut a = SolutionAttempt::new(fp, outcome);
        a.agent_kind = agent.map(|s| s.to_string());
        a
    }

    #[test]
    fn summary_counts_and_rate() {
        let attempts = vec![
            mk("a", AttemptOutcome::Success, Some("claude-code")),
            mk("a", AttemptOutcome::Failure, None),
            mk("b", AttemptOutcome::Abandoned, None),
        ];
        let s = Summary::of(&attempts);
        assert_eq!(s.total, 3);
        assert_eq!(s.unique_fingerprints, 2);
        assert_eq!(s.success_rate, Some(0.5));
    }

    #[test]
    fn fold_by_agent_maps_none_to_human() {
        let attempts = vec![
            mk("a", AttemptOutcome::Success, None),
            mk("a", AttemptOutcome::Failure, Some("cursor")),
        ];
        let buckets = fold_by_agent(&attempts);
        assert_eq!(buckets.len(), 2);
        assert!(buckets
            .iter()
            .any(|b| b["agent_kind"] == "human"));
        assert!(buckets.iter().any(|b| b["agent_kind"] == "cursor"));
    }

    #[test]
    fn fold_by_fingerprint_sorts_by_pain() {
        let attempts = vec![
            mk("easy", AttemptOutcome::Success, None),
            mk("easy", AttemptOutcome::Success, None),
            mk("easy", AttemptOutcome::Success, None),
            mk("hard", AttemptOutcome::Failure, None),
            mk("hard", AttemptOutcome::Failure, None),
            mk("hard", AttemptOutcome::Failure, None),
            mk("hard", AttemptOutcome::Abandoned, None),
        ];
        let rows = fold_by_fingerprint(&attempts);
        // "hard" (4 failures, 0 successes) has higher pain than "easy".
        assert_eq!(rows[0]["fingerprint"], "hard");
    }
}
