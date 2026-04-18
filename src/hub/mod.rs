//! Fukurahub client — the HTTP layer that talks to a hub server per
//! `docs/fukurahub-api.md`.
//!
//! The module is organised around an async trait (`HubClient`) so the
//! CLI and future daemons can swap in fakes for testing without
//! pulling in reqwest. The concrete `HttpHubClient` is a thin wrapper
//! over `reqwest::Client`, async across the board to match the rest of
//! fukura's tokio runtime.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::{Client as Http, StatusCode};

use crate::domain::attempt::AttemptStats;
use crate::domain::models::NoteEnvelope;

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
}

impl Default for HttpHubConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            token: None,
            timeout: Duration::from_secs(15),
            producer: None,
        }
    }
}

pub struct HttpHubClient {
    http: Http,
    base: String,
    token: Option<String>,
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
}

#[async_trait]
impl HubClient for HttpHubClient {
    async fn health(&self) -> Result<HealthResponse> {
        let resp = self.http.get(self.url("/v1/health")).send().await?;
        Self::check(resp).await
    }

    async fn info(&self) -> Result<InfoResponse> {
        let resp = self
            .apply_auth(self.http.get(self.url("/v1/info")))
            .send()
            .await?;
        Self::check(resp).await
    }

    async fn upload_note(&self, envelope: &NoteEnvelope) -> Result<NoteUploaded> {
        let req = self.apply_auth(self.http.post(self.url("/v1/notes")).json(envelope));
        Self::check(req.send().await?).await
    }

    async fn get_note(&self, object_id: &str) -> Result<Option<NoteEnvelope>> {
        let resp = self
            .apply_auth(self.http.get(self.url(&format!("/v1/notes/{object_id}"))))
            .send()
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
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
            .apply_auth(self.http.get(self.url("/v1/notes")).query(&params))
            .send()
            .await?;
        Self::check(resp).await
    }

    async fn upload_attempts(
        &self,
        attempts: &[crate::domain::attempt::SolutionAttempt],
    ) -> Result<AttemptsBatchReceipt> {
        let body = AttemptsBatch {
            attempts: attempts.to_vec(),
        };
        let resp = self
            .apply_auth(self.http.post(self.url("/v1/attempts")).json(&body))
            .send()
            .await?;
        Self::check(resp).await
    }

    async fn stats(&self, fingerprint: Option<&str>) -> Result<StatsPage> {
        let mut rb = self.http.get(self.url("/v1/attempts/stats"));
        if let Some(fp) = fingerprint {
            rb = rb.query(&[("fingerprint", fp)]);
        }
        Self::check(self.apply_auth(rb).send().await?).await
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
        // We can't inspect the UA header directly from the client, but
        // constructing the client with a producer should not fail.
        assert!(HttpHubClient::new(HttpHubConfig {
            base_url: "https://hub.example.com".into(),
            producer: Some("shell-hook".into()),
            ..Default::default()
        })
        .is_ok());
    }
}
