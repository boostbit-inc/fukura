//! Wire types for the fukurahub API v1. These mirror the shapes in
//! `docs/fukurahub-api.md` and intentionally stay independent of the
//! local domain models so the client can talk to future hub versions
//! without breaking the internal repo layout.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::attempt::{AttemptStats, SolutionAttempt};
use crate::domain::models::NoteEnvelope;

/// Response to `POST /v1/notes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteUploaded {
    pub object_id: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub ontology: Option<serde_json::Value>,
}

/// One search hit from `GET /v1/notes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteHit {
    pub object_id: String,
    pub title: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub effectiveness: Option<AttemptStats>,
}

/// Full response envelope for `GET /v1/notes`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchPage {
    #[serde(default)]
    pub hits: Vec<NoteHit>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// Query parameters for searching.
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub fingerprint: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub privacy: Vec<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

/// Request body for `POST /v1/attempts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptsBatch {
    pub attempts: Vec<SolutionAttempt>,
}

/// Response for `POST /v1/attempts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptsBatchReceipt {
    pub accepted: u32,
    pub rejected: u32,
    #[serde(default)]
    pub errors: Vec<AttemptsBatchItemError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptsBatchItemError {
    pub index: u32,
    pub code: String,
    pub message: String,
}

/// Response for `GET /v1/attempts/stats`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsPage {
    #[serde(default)]
    pub by_fingerprint: Vec<FingerprintStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintStats {
    pub fingerprint: String,
    #[serde(default)]
    pub category: Option<String>,
    pub stats: AttemptStats,
}

/// Response for `GET /v1/health`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub hub_id: Option<String>,
}

/// Response for `GET /v1/info`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoResponse {
    #[serde(default)]
    pub max_note_bytes: Option<u64>,
    #[serde(default)]
    pub max_attempts_per_batch: Option<u32>,
    #[serde(default)]
    pub rate_limit_per_minute: Option<u32>,
    #[serde(default)]
    pub retained_privacy_tiers: Vec<String>,
    #[serde(default)]
    pub server_time: Option<DateTime<Utc>>,
}

/// Standard error body (§11 of the spec).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: ErrorPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub retryable: bool,
}

/// Convenience re-exports so callers only need `fukura::hub::*`.
pub use crate::domain::attempt::SolutionAttempt as AttemptDto;
pub use crate::domain::models::NoteEnvelope as NoteEnvelopeDto;

// `NoteEnvelope` is re-exported so the HTTP client signatures stay
// self-documenting in downstream code even without adding
// `use fukura::models::...`.
pub type Note = NoteEnvelope;
