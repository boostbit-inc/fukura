//! Mock-server integration test for the fukurahub HTTP client.
//!
//! Stands up an in-process axum server that speaks the v1 API surface
//! defined in `docs/fukurahub-api.md`, then drives it with the real
//! `HttpHubClient`. This confirms the client produces the exact wire
//! shape the spec requires — and, equally importantly, that the
//! spec's request and response shapes round-trip cleanly through the
//! reference types in `src/hub/types.rs`.

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
    Json(envelope): Json<NoteEnvelope>,
) -> impl IntoResponse {
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
    let (addr, _state, _h) = spawn_server().await;
    let base = format!("http://{addr}");
    let client = HttpHubClient::new(HttpHubConfig {
        base_url: base.clone(),
        token: Some("test-token".into()),
        producer: Some("integration-test".into()),
        ..Default::default()
    })
    .unwrap();

    // health (no auth)
    let h = client.health().await.unwrap();
    assert_eq!(h.status, "ok");
    assert_eq!(h.hub_id.as_deref(), Some("test-hub"));

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
    assert_eq!(info.max_note_bytes, Some(262_144));

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
    let (addr, state, _h) = spawn_server().await;
    *state.reject_next.lock().await = true;

    let client = HttpHubClient::new(HttpHubConfig {
        base_url: format!("http://{addr}"),
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
async fn get_note_returns_none_on_404() {
    let (addr, _state, _h) = spawn_server().await;
    let client = HttpHubClient::new(HttpHubConfig {
        base_url: format!("http://{addr}"),
        token: Some("t".into()),
        ..Default::default()
    })
    .unwrap();

    let fetched = client.get_note("sha256:missing").await.unwrap();
    assert!(fetched.is_none());
}
