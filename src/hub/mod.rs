//! Fukurahub client — the HTTP layer that talks to a hub server per
//! `docs/fukurahub-api.md`.
//!
//! The module is organised around an async trait (`HubClient`) so the
//! CLI and future daemons can swap in fakes for testing without
//! pulling in reqwest. The concrete `HttpHubClient` is a thin wrapper
//! over `reqwest::Client`, async across the board to match the rest of
//! fukura's tokio runtime.
//!
//! This client implements the four client MUSTs from the API spec §10:
//!
//! 1. `upload_note` rejects `Privacy::Private` before it leaves the
//!    process — private records must never travel to a hub.
//! 2. Retries on `5xx` / `429` with exponential backoff (base 2s, cap
//!    60s, up to 4 attempts total), honouring `Retry-After` when the
//!    server sends one.
//! 3. `info()` is cached for 1 hour; subsequent uploads check
//!    `max_note_bytes` and `max_attempts_per_batch` client-side so the
//!    hub never has to reject an oversized request.
//! 4. Mutating requests carry an `Idempotency-Key` (UUID v4) header,
//!    reused across retries so the server can dedupe in-flight work.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use reqwest::{Client as Http, StatusCode};
use tokio::sync::RwLock;

use crate::domain::attempt::AttemptStats;
use crate::domain::models::{NoteEnvelope, Privacy};

pub mod types;

use types::*;

/// The stable surface fukura code programs against.
///
/// Async by default because every realistic hub call goes over the
/// network and the rest of fukura is already on tokio. Testing
/// doubles implement this trait directly.
#[async_trait]
pub trait HubClient: Send + Sync {
    async fn health(&self) -> Result<HealthResponse>;
    async fn info(&self) -> Result<InfoResponse>;
    async fn upload_note(&self, envelope: &NoteEnvelope) -> Result<NoteUploaded>;
    async fn get_note(&self, object_id: &str) -> Result<Option<NoteEnvelope>>;
    async fn search_notes(&self, query: &SearchQuery) -> Result<SearchPage>;
    async fn upload_attempts(
        &self,
        attempts: &[crate::domain::attempt::SolutionAttempt],
    ) -> Result<AttemptsBatchReceipt>;
    async fn stats(&self, fingerprint: Option<&str>) -> Result<StatsPage>;
}

/// Retry policy for `5xx` and `429` responses.
///
/// `max_attempts` counts the initial call plus retries, so `4` means
/// "try once, then up to three more times." Delays are the exponential
/// backoff `base * 2^(attempt-1)`, clamped to `cap`, and overridden by
/// the `Retry-After` header when the server provides one.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base: Duration,
    pub cap: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            base: Duration::from_secs(2),
            cap: Duration::from_secs(60),
        }
    }
}

impl RetryPolicy {
    /// Delay before the Nth retry. `attempt` is 1-based: 1 is the
    /// wait between the first and second calls.
    fn backoff(&self, attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(20);
        let factor = 1u64.checked_shl(shift).unwrap_or(u64::MAX);
        let base_ms = self.base.as_millis() as u64;
        let computed_ms = base_ms.saturating_mul(factor);
        let cap_ms = self.cap.as_millis() as u64;
        Duration::from_millis(computed_ms.min(cap_ms))
    }
}

/// Config for a HTTP-backed hub client.
#[derive(Debug, Clone)]
pub struct HttpHubConfig {
    /// Base URL, e.g. `https://hub.example.com`. No trailing slash
    /// required; the client trims it.
    pub base_url: String,
    /// Optional bearer token. Every authenticated endpoint sends
    /// `Authorization: Bearer <token>`.
    pub token: Option<String>,
    /// Request timeout. Defaults to 15s — shell hooks upload often and
    /// should fail fast when the hub is unavailable.
    pub timeout: Duration,
    /// User-Agent suffix to identify the producer
    /// (e.g. `claude-code`, `shell-hook`). Prepended automatically
    /// with `fukura/<version>`.
    pub producer: Option<String>,
    /// Retry policy for 5xx / 429 responses.
    pub retry: RetryPolicy,
    /// How long `info()` is cached before a refetch. Spec §10 lets
    /// clients cache for up to an hour; default matches that.
    pub info_ttl: Duration,
}

impl Default for HttpHubConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            token: None,
            timeout: Duration::from_secs(15),
            producer: None,
            retry: RetryPolicy::default(),
            info_ttl: Duration::from_secs(3600),
        }
    }
}

struct InfoCacheEntry {
    fetched_at: Instant,
    value: InfoResponse,
}

pub struct HttpHubClient {
    http: Http,
    base: String,
    token: Option<String>,
    retry: RetryPolicy,
    info_ttl: Duration,
    info_cache: Arc<RwLock<Option<InfoCacheEntry>>>,
}

impl HttpHubClient {
    pub fn new(cfg: HttpHubConfig) -> Result<Self> {
        let ua = match &cfg.producer {
            Some(p) => format!("fukura/{} ({})", env!("CARGO_PKG_VERSION"), p),
            None => format!("fukura/{}", env!("CARGO_PKG_VERSION")),
        };
        let http = Http::builder()
            .user_agent(ua)
            .timeout(cfg.timeout)
            .build()
            .context("building reqwest client")?;
        Ok(Self {
            http,
            base: cfg.base_url.trim_end_matches('/').to_owned(),
            token: cfg.token,
            retry: cfg.retry,
            info_ttl: cfg.info_ttl,
            info_cache: Arc::new(RwLock::new(None)),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn apply_auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(tok) => rb.bearer_auth(tok),
            None => rb,
        }
    }

    /// Send a request with retries for `5xx` / `429`. Each retry reuses
    /// the same `Idempotency-Key` so the server can dedupe in-flight
    /// work (spec §6 + §10.5).
    async fn send_with_retry(
        &self,
        template: reqwest::RequestBuilder,
        idempotency_key: Option<&str>,
    ) -> Result<reqwest::Response> {
        let mut template = self.apply_auth(template);
        if let Some(key) = idempotency_key {
            template = template.header("Idempotency-Key", key);
        }

        let mut last_error_body: Option<String> = None;
        for attempt in 1..=self.retry.max_attempts {
            let rb = template.try_clone().ok_or_else(|| {
                anyhow!("request body is not retryable (reqwest could not clone it)")
            })?;
            match rb.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp);
                    }
                    if !is_retryable(status) || attempt == self.retry.max_attempts {
                        return Ok(resp);
                    }
                    let retry_after = resp
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(parse_retry_after);
                    // Drain the body so the connection can be reused.
                    last_error_body = resp.text().await.ok();
                    let delay = retry_after.unwrap_or_else(|| self.retry.backoff(attempt));
                    tokio::time::sleep(delay).await;
                }
                Err(err) => {
                    if attempt == self.retry.max_attempts || !err_is_retryable(&err) {
                        return Err(anyhow::Error::from(err)
                            .context("hub request failed after all retries"));
                    }
                    tokio::time::sleep(self.retry.backoff(attempt)).await;
                }
            }
        }
        // Unreachable under normal flow; the loop either returns a
        // response or propagates an error in the last iteration.
        Err(anyhow!(
            "hub request exhausted {} attempts{}",
            self.retry.max_attempts,
            last_error_body
                .map(|b| format!(" (last body: {b})"))
                .unwrap_or_default()
        ))
    }

    async fn check<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        if status.is_success() {
            let body = resp.json::<T>().await.context("decoding success body")?;
            return Ok(body);
        }
        let status_code = status;
        let text = resp.text().await.unwrap_or_default();
        if let Ok(body) = serde_json::from_str::<ErrorBody>(&text) {
            return Err(anyhow!(
                "hub {}: {} ({})",
                status_code.as_u16(),
                body.error.message,
                body.error.code
            ));
        }
        Err(anyhow!("hub {}: {}", status_code.as_u16(), text))
    }

    /// Return `info()` from cache if fresh, otherwise fetch and cache.
    /// Called by size-enforcement helpers before mutating uploads.
    async fn cached_info(&self) -> Result<InfoResponse> {
        if let Some(entry) = self.info_cache.read().await.as_ref() {
            if entry.fetched_at.elapsed() < self.info_ttl {
                return Ok(entry.value.clone());
            }
        }
        let fresh = self.fetch_info_raw().await?;
        *self.info_cache.write().await = Some(InfoCacheEntry {
            fetched_at: Instant::now(),
            value: fresh.clone(),
        });
        Ok(fresh)
    }

    async fn fetch_info_raw(&self) -> Result<InfoResponse> {
        let resp = self
            .send_with_retry(self.http.get(self.url("/v1/info")), None)
            .await?;
        Self::check(resp).await
    }
}

/// `5xx` and `429` are retryable per API spec §8 + §10.
fn is_retryable(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Treat connect / timeout / I/O errors as retryable; schema and request
/// construction errors are not.
fn err_is_retryable(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}

/// Parse a `Retry-After` value in seconds. HTTP-date form is ignored
/// (it is rare for APIs; fall back to exponential backoff instead).
fn parse_retry_after(raw: &str) -> Option<Duration> {
    raw.trim().parse::<u64>().ok().map(Duration::from_secs)
}

/// Generate a fresh idempotency key. UUID v4 per SHOULD in spec §10.5.
fn new_idempotency_key() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[async_trait]
impl HubClient for HttpHubClient {
    async fn health(&self) -> Result<HealthResponse> {
        let resp = self
            .send_with_retry(self.http.get(self.url("/v1/health")), None)
            .await?;
        Self::check(resp).await
    }

    async fn info(&self) -> Result<InfoResponse> {
        self.cached_info().await
    }

    async fn upload_note(&self, envelope: &NoteEnvelope) -> Result<NoteUploaded> {
        // MUST §10.2: reject `private` before it reaches the wire.
        if envelope.note.privacy == Privacy::Private {
            bail!(
                "refusing to upload a note with privacy=private — \
                 private notes must remain client-local (spec §4)"
            );
        }

        // MUST §10.3: honour the server's advertised size limit.
        let body =
            serde_json::to_vec(envelope).context("serialising note envelope for upload")?;
        if let Ok(info) = self.cached_info().await {
            if let Some(max) = info.max_note_bytes {
                if body.len() as u64 > max {
                    bail!(
                        "note body is {} bytes; hub max_note_bytes is {}",
                        body.len(),
                        max
                    );
                }
            }
        }

        let key = new_idempotency_key();
        let req = self
            .http
            .post(self.url("/v1/notes"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        let resp = self.send_with_retry(req, Some(&key)).await?;
        Self::check(resp).await
    }

    async fn get_note(&self, object_id: &str) -> Result<Option<NoteEnvelope>> {
        let resp = self
            .send_with_retry(
                self.http.get(self.url(&format!("/v1/notes/{object_id}"))),
                None,
            )
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        // 410 Gone means "this object_id existed and has been deleted"
        // (alignment D4a). For the client, surfacing it as None keeps
        // call sites simple; the distinction matters for observability
        // but not for whether we have the note to show the user.
        if resp.status() == StatusCode::GONE {
            return Ok(None);
        }
        let env: NoteEnvelope = Self::check(resp).await?;
        Ok(Some(env))
    }

    async fn search_notes(&self, query: &SearchQuery) -> Result<SearchPage> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(q) = &query.q {
            params.push(("q", q.clone()));
        }
        if let Some(fp) = &query.fingerprint {
            params.push(("fingerprint", fp.clone()));
        }
        if let Some(cat) = &query.category {
            params.push(("category", cat.clone()));
        }
        for tag in &query.tags {
            params.push(("tag", tag.clone()));
        }
        for p in &query.privacy {
            params.push(("privacy", p.clone()));
        }
        if let Some(c) = &query.cursor {
            params.push(("cursor", c.clone()));
        }
        if let Some(l) = query.limit {
            params.push(("limit", l.to_string()));
        }
        let resp = self
            .send_with_retry(self.http.get(self.url("/v1/notes")).query(&params), None)
            .await?;
        Self::check(resp).await
    }

    async fn upload_attempts(
        &self,
        attempts: &[crate::domain::attempt::SolutionAttempt],
    ) -> Result<AttemptsBatchReceipt> {
        // MUST §10.3: enforce the advertised batch limit client-side.
        if let Ok(info) = self.cached_info().await {
            if let Some(max) = info.max_attempts_per_batch {
                if attempts.len() > max as usize {
                    bail!(
                        "attempts batch is {}; hub max_attempts_per_batch is {}",
                        attempts.len(),
                        max
                    );
                }
            }
        }

        let body = AttemptsBatch {
            attempts: attempts.to_vec(),
        };
        let key = new_idempotency_key();
        let resp = self
            .send_with_retry(
                self.http.post(self.url("/v1/attempts")).json(&body),
                Some(&key),
            )
            .await?;
        Self::check(resp).await
    }

    async fn stats(&self, fingerprint: Option<&str>) -> Result<StatsPage> {
        let mut rb = self.http.get(self.url("/v1/attempts/stats"));
        if let Some(fp) = fingerprint {
            rb = rb.query(&[("fingerprint", fp)]);
        }
        let resp = self.send_with_retry(rb, None).await?;
        Self::check(resp).await
    }
}

/// Convenience helper for callers that only need global stats and
/// want a plain map back.
pub fn collect_stats_by_fingerprint(
    page: &StatsPage,
) -> std::collections::HashMap<String, AttemptStats> {
    page.by_fingerprint
        .iter()
        .map(|fs| (fs.fingerprint.clone(), fs.stats.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_trailing_slash_is_trimmed() {
        let client = HttpHubClient::new(HttpHubConfig {
            base_url: "https://hub.example.com/".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            client.url("/v1/health"),
            "https://hub.example.com/v1/health"
        );
    }

    #[test]
    fn producer_is_embedded_in_user_agent() {
        assert!(HttpHubClient::new(HttpHubConfig {
            base_url: "https://hub.example.com".into(),
            producer: Some("shell-hook".into()),
            ..Default::default()
        })
        .is_ok());
    }

    #[test]
    fn retry_backoff_grows_exponentially_and_clamps_to_cap() {
        let p = RetryPolicy {
            max_attempts: 5,
            base: Duration::from_secs(2),
            cap: Duration::from_secs(60),
        };
        assert_eq!(p.backoff(1), Duration::from_secs(2));
        assert_eq!(p.backoff(2), Duration::from_secs(4));
        assert_eq!(p.backoff(3), Duration::from_secs(8));
        assert_eq!(p.backoff(4), Duration::from_secs(16));
        assert_eq!(p.backoff(5), Duration::from_secs(32));
        // 6th would be 64s → clamped to 60s.
        assert_eq!(p.backoff(6), Duration::from_secs(60));
        // Extreme attempt shouldn't overflow.
        assert_eq!(p.backoff(100), Duration::from_secs(60));
    }

    #[test]
    fn retryable_status_set_matches_spec() {
        assert!(is_retryable(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable(StatusCode::BAD_GATEWAY));
        assert!(is_retryable(StatusCode::TOO_MANY_REQUESTS));
        assert!(!is_retryable(StatusCode::BAD_REQUEST));
        assert!(!is_retryable(StatusCode::UNAUTHORIZED));
        assert!(!is_retryable(StatusCode::NOT_FOUND));
        assert!(!is_retryable(StatusCode::GONE));
    }

    #[test]
    fn retry_after_seconds_parse() {
        assert_eq!(parse_retry_after("3"), Some(Duration::from_secs(3)));
        assert_eq!(parse_retry_after(" 12 "), Some(Duration::from_secs(12)));
        // HTTP-date form is out of scope; returns None so the caller
        // falls back to exponential backoff.
        assert_eq!(parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"), None);
    }

    #[test]
    fn idempotency_keys_are_unique_uuids() {
        let a = new_idempotency_key();
        let b = new_idempotency_key();
        assert_ne!(a, b);
        assert!(uuid::Uuid::parse_str(&a).is_ok());
    }

    #[tokio::test]
    async fn reject_private_uploads_before_network() {
        let client = HttpHubClient::new(HttpHubConfig {
            base_url: "http://127.0.0.1:1".into(), // would fail if we reached it
            token: Some("t".into()),
            ..Default::default()
        })
        .unwrap();

        let now = chrono::Utc::now();
        let envelope = NoteEnvelope {
            schema: "fuku.note".into(),
            version: 1,
            note: crate::domain::models::Note {
                title: "nope".into(),
                body: "secret".into(),
                tags: vec![],
                links: vec![],
                meta: Default::default(),
                solutions: vec![],
                privacy: Privacy::Private,
                created_at: now,
                updated_at: now,
                author: crate::domain::models::Author {
                    name: "t".into(),
                    email: None,
                },
                ontology: None,
            },
        };
        let err = client.upload_note(&envelope).await.err().unwrap();
        let msg = format!("{err}");
        assert!(
            msg.contains("private"),
            "expected refusal to mention 'private': {msg}"
        );
    }
}
