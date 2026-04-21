//! Contract test harness for the fukurahub HTTP client.
//!
//! **Two modes.**
//!
//! - *Mock mode (default).* Spins up an in-process axum server that speaks
//!   the v1 API surface defined in `docs/fukurahub-api.md` and drives it
//!   with the real `HttpHubClient`. Used by fukura's own CI.
//! - *External mode.* When `HUB_BASE_URL` is set in the environment, the
//!   tests drive a real hub process at that URL instead. This lets the
//!   fukura-hub repo reuse this file as a contract test from its own CI
//!   (see `docs/fukurahub-alignment.md` §4). Authentication uses the
//!   `HUB_TOKEN` env var (falls back to `"test-token"`, which only the
//!   mock accepts).
//!
//! Tests that manipulate the in-process mock's internal state (e.g.
//! forcing the next upload to return 401) skip themselves in external
//! mode — a real hub has no such hook. Every other test runs in both
//! modes; spec violations on the real server surface as failing tests
//! and get green'd one at a time by P3 slices.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tokio::sync::Mutex;

use fukura::domain::attempt::{AttemptStats, SolutionAttempt};
use fukura::hub::types::{
    AttemptsBatch, AttemptsBatchReceipt, FingerprintStats, HealthResponse, InfoResponse, NoteHit,
    NoteUploaded, SearchPage, SearchQuery, StatsPage,
};
use fukura::hub::{HttpHubClient, HttpHubConfig, HubClient};
use fukura::models::{Author, Note, NoteEnvelope, Privacy};

#[derive(Default, Clone)]
struct FakeState {
    notes: Arc<Mutex<Vec<NoteEnvelope>>>,
    attempts: Arc<Mutex<Vec<SolutionAttempt>>>,
    /// When true, the next upload returns 401 to exercise error handling.
    reject_next: Arc<Mutex<bool>>,
    /// Count of 429s to issue before succeeding. Each failing response
    /// decrements the counter. Used by the retry-loop test.
    throttle_next: Arc<Mutex<u32>>,
    /// Records the Idempotency-Key header seen on each upload so tests
    /// can assert retries reuse the same key.
    seen_idempotency_keys: Arc<Mutex<Vec<String>>>,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: Some("0.1.0".into()),
        hub_id: Some("test-hub".into()),
    })
}

async fn info(headers: HeaderMap) -> impl IntoResponse {
    if headers.get("authorization").is_none() {
        return (StatusCode::UNAUTHORIZED, Json(json!({}))).into_response();
    }
    Json(InfoResponse {
        max_note_bytes: Some(262_144),
        max_attempts_per_batch: Some(500),
        rate_limit_per_minute: Some(600),
        retained_privacy_tiers: vec!["org".into(), "public".into()],
        server_time: Some(chrono::Utc::now()),
    })
    .into_response()
}

async fn upload_note(
    State(state): State<FakeState>,
    headers: HeaderMap,
    Json(envelope): Json<NoteEnvelope>,
) -> impl IntoResponse {
    if let Some(key) = headers.get("idempotency-key").and_then(|v| v.to_str().ok()) {
        state
            .seen_idempotency_keys
            .lock()
            .await
            .push(key.to_string());
    }
    let mut throttle = state.throttle_next.lock().await;
    if *throttle > 0 {
        *throttle -= 1;
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("retry-after", "0")],
            Json(json!({
                "error": { "code": "rate_limited", "message": "slow down", "retryable": true }
            })),
        )
            .into_response();
    }
    drop(throttle);
    if *state.reject_next.lock().await {
        *state.reject_next.lock().await = false;
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": { "code": "unauthorized", "message": "missing token", "retryable": false }
            })),
        )
            .into_response();
    }
    let object_id = format!(
        "sha256:{}",
        hex::encode(
            <sha2::Sha256 as sha2::Digest>::digest(format!(
                "{}|{}",
                envelope.note.title,
                envelope
                    .note
                    .ontology
                    .as_ref()
                    .map(|o| o.fingerprint.as_str())
                    .unwrap_or("")
            ))
            .as_slice()
        )
    );
    state.notes.lock().await.push(envelope.clone());
    (
        StatusCode::CREATED,
        Json(NoteUploaded {
            object_id,
            url: None,
            ontology: envelope
                .note
                .ontology
                .as_ref()
                .map(|o| json!({ "fingerprint": o.fingerprint, "category": o.category })),
        }),
    )
        .into_response()
}

async fn get_note(State(state): State<FakeState>, Path(_id): Path<String>) -> impl IntoResponse {
    match state.notes.lock().await.first().cloned() {
        Some(env) => Json(env).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({}))).into_response(),
    }
}

async fn search_notes(
    State(state): State<FakeState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let hits: Vec<NoteHit> = state
        .notes
        .lock()
        .await
        .iter()
        .filter(|env| match params.get("fingerprint") {
            Some(fp) => env
                .note
                .ontology
                .as_ref()
                .map(|o| o.fingerprint == *fp)
                .unwrap_or(false),
            None => true,
        })
        .map(|env| NoteHit {
            object_id: "sha256:deadbeef".into(),
            title: env.note.title.clone(),
            category: env.note.ontology.as_ref().map(|o| o.category.clone()),
            fingerprint: env.note.ontology.as_ref().map(|o| o.fingerprint.clone()),
            tags: env.note.tags.clone(),
            summary: Some(env.note.body.chars().take(80).collect()),
            updated_at: Some(env.note.updated_at),
            effectiveness: None,
        })
        .collect();
    Json(SearchPage {
        hits,
        next_cursor: None,
    })
}

async fn upload_attempts(
    State(state): State<FakeState>,
    Json(batch): Json<AttemptsBatch>,
) -> impl IntoResponse {
    let accepted = batch.attempts.len() as u32;
    state.attempts.lock().await.extend(batch.attempts);
    (
        StatusCode::ACCEPTED,
        Json(AttemptsBatchReceipt {
            accepted,
            rejected: 0,
            errors: vec![],
        }),
    )
}

async fn stats(
    State(state): State<FakeState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let attempts = state.attempts.lock().await.clone();
    let mut by: std::collections::HashMap<String, AttemptStats> = Default::default();
    for a in attempts {
        if let Some(fp) = params.get("fingerprint") {
            if &a.fingerprint != fp {
                continue;
            }
        }
        by.entry(a.fingerprint.clone())
            .or_default()
            .record(a.outcome);
    }
    Json(StatsPage {
        by_fingerprint: by
            .into_iter()
            .map(|(fingerprint, stats)| FingerprintStats {
                fingerprint,
                category: None,
                stats,
            })
            .collect(),
    })
}

async fn spawn_server() -> (SocketAddr, FakeState, tokio::task::JoinHandle<()>) {
    let state = FakeState::default();
    let app = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/info", get(info))
        .route("/v1/notes", post(upload_note).get(search_notes))
        .route("/v1/notes/{object_id}", get(get_note))
        .route("/v1/attempts", post(upload_attempts))
        .route("/v1/attempts/stats", get(stats))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, state, handle)
}

/// Target the tests run against.
///
/// `Mock` is the default and uses the in-process axum server. `External`
/// is selected by setting `HUB_BASE_URL` and drives a real hub instead.
struct TestTarget {
    base_url: String,
    mock_state: Option<FakeState>,
    _mock_handle: Option<tokio::task::JoinHandle<()>>,
}

impl TestTarget {
    /// Resolve the target for a test. If `HUB_BASE_URL` is set, point at
    /// that URL; otherwise spawn an in-process mock. The mock handle is
    /// held so the server lives for the lifetime of the test.
    async fn start() -> Self {
        if let Ok(url) = std::env::var("HUB_BASE_URL") {
            Self {
                base_url: url.trim_end_matches('/').to_string(),
                mock_state: None,
                _mock_handle: None,
            }
        } else {
            let (addr, state, handle) = spawn_server().await;
            Self {
                base_url: format!("http://{addr}"),
                mock_state: Some(state),
                _mock_handle: Some(handle),
            }
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Resolve the bearer token to send. Defaults to `"test-token"`
    /// (which only the in-process mock accepts). External runners must
    /// export `HUB_TOKEN` to a token the real hub will honour.
    fn token(&self) -> String {
        std::env::var("HUB_TOKEN").unwrap_or_else(|_| "test-token".to_string())
    }

    /// Borrow the mock state for tests that need to manipulate it.
    /// Returns `None` in external mode; callers should skip or adjust.
    fn mock_state(&self) -> Option<&FakeState> {
        self.mock_state.as_ref()
    }
}

/// True when the harness is driving a real external server.
fn is_external_mode() -> bool {
    std::env::var("HUB_BASE_URL").is_ok()
}

/// Skip the current test in external mode unless the hub has declared
/// the given P3 slice landed via `HUB_SLICE_<N>=1`. In mock mode the
/// in-process axum implements every slice, so the markers are ignored.
///
/// Returns `true` if the test should early-return. Callers write:
///
/// ```ignore
/// if external_requires_slice(3, "what this test needs") {
///     return;
/// }
/// ```
///
/// The convention lets fukura-hub's CI start with `HUB_SLICE_1=1`
/// today and add `HUB_SLICE_2=1`, `HUB_SLICE_3=1`, ... as each slice
/// merges. Tests that depend on an unlanded slice skip with a clear
/// reason; tests that DO depend on a landed slice must pass, so a
/// regression breaks CI (merge gate).
fn external_requires_slice(slice: u8, what: &str) -> bool {
    if !is_external_mode() {
        return false;
    }
    let marker = format!("HUB_SLICE_{slice}");
    if std::env::var(&marker).is_ok() {
        return false;
    }
    eprintln!(
        "SKIP (external mode): {what} — requires P3 Slice {slice}. \
         Set {marker}=1 in CI once that slice lands on hub's main."
    );
    true
}

fn sample_note() -> NoteEnvelope {
    let now = chrono::Utc::now();
    NoteEnvelope {
        schema: "fuku.note".into(),
        version: 1,
        note: Note {
            title: "cargo build: unresolved import".into(),
            body: "tried `cargo update`, fixed it".into(),
            tags: vec!["cargo".into(), "rust".into()],
            links: vec![],
            meta: Default::default(),
            solutions: vec![],
            privacy: Privacy::Org,
            created_at: now,
            updated_at: now,
            author: Author {
                name: "tester".into(),
                email: None,
            },
            ontology: Some(fukura::ontology::ErrorOntology::new(
                "cargo",
                "cargo.compile.e0432",
                "sha256:test-fingerprint",
            )),
        },
    }
}

#[tokio::test]
async fn client_round_trips_against_mock_server() {
    let target = TestTarget::start().await;
    // The mega roundtrip exercises /v1/health, /v1/info, /v1/notes,
    // /v1/attempts*. Slices 1-3 all need to have landed before it can
    // pass against a real hub, so gate it on the latest slice it
    // needs.
    if external_requires_slice(3, "/v1/health, /v1/info, /v1/notes round-trip") {
        return;
    }
    let base = target.base_url().to_string();
    let client = HttpHubClient::new(HttpHubConfig {
        base_url: base.clone(),
        token: Some(target.token()),
        producer: Some("integration-test".into()),
        ..Default::default()
    })
    .unwrap();

    // health (no auth)
    let h = client.health().await.unwrap();
    assert_eq!(h.status, "ok");
    if !is_external_mode() {
        // hub_id is a mock-specific literal; a real server will advertise
        // whatever value it pins for itself.
        assert_eq!(h.hub_id.as_deref(), Some("test-hub"));
    }

    // info (auth required — fails without token)
    let no_auth = HttpHubClient::new(HttpHubConfig {
        base_url: base.clone(),
        ..Default::default()
    })
    .unwrap();
    assert!(
        no_auth.info().await.is_err(),
        "info without token must fail"
    );

    let info = client.info().await.unwrap();
    if !is_external_mode() {
        // The mock hard-codes this; real servers advertise their own
        // limits, which the spec only requires to be present.
        assert_eq!(info.max_note_bytes, Some(262_144));
    } else {
        assert!(
            info.max_note_bytes.is_some(),
            "real hub must advertise max_note_bytes"
        );
    }

    // upload + fetch
    let envelope = sample_note();
    let up = client.upload_note(&envelope).await.unwrap();
    assert!(up.object_id.starts_with("sha256:"));

    let fetched = client.get_note(&up.object_id).await.unwrap().unwrap();
    assert_eq!(fetched.note.title, envelope.note.title);

    // search by fingerprint
    let q = SearchQuery {
        fingerprint: Some("sha256:test-fingerprint".into()),
        ..Default::default()
    };
    let page = client.search_notes(&q).await.unwrap();
    assert_eq!(page.hits.len(), 1);
    assert_eq!(
        page.hits[0].fingerprint.as_deref(),
        Some("sha256:test-fingerprint")
    );

    // upload attempts
    let receipt = client
        .upload_attempts(&[
            SolutionAttempt::new(
                "sha256:test-fingerprint",
                fukura::domain::attempt::AttemptOutcome::Success,
            ),
            SolutionAttempt::new(
                "sha256:test-fingerprint",
                fukura::domain::attempt::AttemptOutcome::Failure,
            ),
        ])
        .await
        .unwrap();
    assert_eq!(receipt.accepted, 2);

    let stats = client.stats(Some("sha256:test-fingerprint")).await.unwrap();
    let s = &stats.by_fingerprint[0];
    assert_eq!(s.stats.success, 1);
    assert_eq!(s.stats.failure, 1);
    assert_eq!(s.stats.success_rate(), Some(0.5));
}

#[tokio::test]
async fn client_surfaces_standard_error_body() {
    let target = TestTarget::start().await;
    let Some(state) = target.mock_state() else {
        eprintln!(
            "SKIP client_surfaces_standard_error_body in external mode: \
             requires in-process mock to force a 401 on an authenticated \
             upload. Real-hub auth failures are covered by the \
             no-token assertion in the round-trip test."
        );
        return;
    };
    *state.reject_next.lock().await = true;

    let client = HttpHubClient::new(HttpHubConfig {
        base_url: target.base_url().to_string(),
        token: Some("t".into()),
        ..Default::default()
    })
    .unwrap();

    let err = client.upload_note(&sample_note()).await.err().unwrap();
    let msg = format!("{err}");
    assert!(msg.contains("401"), "expected 401 in error: {msg}");
    assert!(
        msg.contains("unauthorized") || msg.contains("missing token"),
        "error body should surface: {msg}"
    );
}

#[tokio::test]
async fn upload_retries_on_429_and_reuses_idempotency_key() {
    let target = TestTarget::start().await;
    let Some(state) = target.mock_state() else {
        eprintln!(
            "SKIP upload_retries_on_429_and_reuses_idempotency_key in external mode: \
             requires mock state to count forced 429 responses."
        );
        return;
    };
    // Make the first two uploads 429; the third should succeed.
    *state.throttle_next.lock().await = 2;

    let client = HttpHubClient::new(HttpHubConfig {
        base_url: target.base_url().to_string(),
        token: Some(target.token()),
        retry: fukura::hub::RetryPolicy {
            max_attempts: 4,
            base: std::time::Duration::from_millis(10),
            cap: std::time::Duration::from_millis(50),
        },
        ..Default::default()
    })
    .unwrap();

    let envelope = sample_note();
    let uploaded = client.upload_note(&envelope).await.unwrap();
    assert!(uploaded.object_id.starts_with("sha256:"));

    let keys = state.seen_idempotency_keys.lock().await.clone();
    assert_eq!(
        keys.len(),
        3,
        "expected exactly 3 upload attempts (2×429 then 200), saw {keys:?}"
    );
    assert_eq!(
        keys.iter().collect::<std::collections::HashSet<_>>().len(),
        1,
        "every retry should reuse the same Idempotency-Key; saw {keys:?}"
    );
    assert!(
        uuid::Uuid::parse_str(&keys[0]).is_ok(),
        "Idempotency-Key must be a UUID; saw {:?}",
        keys[0]
    );
}

#[tokio::test]
async fn get_note_returns_none_on_404() {
    let target = TestTarget::start().await;
    if external_requires_slice(3, "GET /v1/notes/{id}") {
        return;
    }
    let client = HttpHubClient::new(HttpHubConfig {
        base_url: target.base_url().to_string(),
        token: Some(target.token()),
        ..Default::default()
    })
    .unwrap();

    let fetched = client.get_note("sha256:missing").await.unwrap();
    assert!(fetched.is_none());
}

/// Slice-1-focused external-mode smoke: exercise only the two
/// endpoints that Slice 1 added, so fukura-hub's CI starts seeing a
/// real contract-test pass/fail signal the moment the slice lands.
/// In mock mode this is redundant with the mega round-trip above, so
/// skip to keep local test time down.
#[tokio::test]
async fn attempts_upload_and_stats_against_external_hub() {
    let target = TestTarget::start().await;
    if target.mock_state().is_some() {
        // Covered by client_round_trips_against_mock_server.
        return;
    }
    if external_requires_slice(1, "POST /v1/attempts + GET /v1/attempts/stats") {
        return;
    }

    let client = HttpHubClient::new(HttpHubConfig {
        base_url: target.base_url().to_string(),
        token: Some(target.token()),
        ..Default::default()
    })
    .unwrap();

    // Upload a tiny batch with a deterministic attempt_id so we can
    // re-post and assert idempotency.
    let fingerprint = "blake3:contract-test-slice1";
    let deterministic_id = "00000000-0000-4000-8000-000000001111";
    let mut once = SolutionAttempt::new(
        fingerprint,
        fukura::domain::attempt::AttemptOutcome::Success,
    );
    once.attempt_id = deterministic_id.to_string();

    let first = client.upload_attempts(&[once.clone()]).await.unwrap();
    assert_eq!(first.accepted, 1, "first upload should accept");
    let second = client.upload_attempts(&[once]).await.unwrap();
    assert_eq!(
        second.accepted, 1,
        "reposting the same attempt_id MUST still count as accepted (spec §6)"
    );

    let stats = client.stats(Some(fingerprint)).await.unwrap();
    let found = stats
        .by_fingerprint
        .iter()
        .find(|s| s.fingerprint == fingerprint)
        .expect("stats should include the fingerprint we just uploaded");
    assert!(
        found.stats.success >= 1,
        "stats should reflect at least our one success"
    );
}
