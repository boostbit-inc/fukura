use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSearchHit {
    pub object_id: String,
    pub title: String,
    pub snippet: String,
    pub tags: Vec<String>,
    pub author: String,
    pub privacy: String,
    pub score: f32,
}

#[derive(Debug, Deserialize)]
struct RemoteSearchResult {
    notes: Vec<RemoteNote>,
    total: usize,
    took_ms: u64,
}

#[derive(Debug, Deserialize)]
struct RemoteNote {
    object_id: String,
    title: String,
    body: String,
    tags: Vec<String>,
    author: RemoteAuthor,
    privacy: String,
}

#[derive(Debug, Deserialize)]
struct RemoteAuthor {
    username: String,
}

pub async fn search_remote(
    remote_url: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<RemoteSearchHit>> {
    let token = std::env::var("FUKURA_TOKEN")
        .or_else(|_| std::env::var("FUKURA_API_TOKEN"))
        .unwrap_or_default();

    let client = Client::new();
    let url = format!(
        "{}/api/v1/search?q={}&limit={}",
        remote_url.trim_end_matches('/'),
        urlencoding::encode(query),
        limit
    );

    let mut request = client.get(&url);
    
    if !token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request
        .send()
        .await
        .with_context(|| "Failed to contact remote hub")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unavailable>".to_string());
        anyhow::bail!("Remote search failed ({}): {}", status, body);
    }

    let result: RemoteSearchResult = response
        .json()
        .await
        .with_context(|| "Failed to decode remote search response")?;

    let hits = result
        .notes
        .into_iter()
        .map(|note| {
            let snippet = if note.body.len() > 150 {
                format!("{}...", &note.body[..150])
            } else {
                note.body.clone()
            };

            RemoteSearchHit {
                object_id: note.object_id,
                title: note.title,
                snippet,
                tags: note.tags,
                author: note.author.username,
                privacy: note.privacy,
                score: 1.0, // TODO: Use actual relevance score
            }
        })
        .collect();

    Ok(hits)
}


