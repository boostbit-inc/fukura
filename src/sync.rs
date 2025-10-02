use anyhow::{Context, Result};
use reqwest::Client;

use crate::models::NoteRecord;
use crate::repo::FukuraRepo;

fn normalize_remote(remote: &str) -> String {
    remote.trim_end_matches('/').to_string()
}

pub async fn push_note(repo: &FukuraRepo, object_id: &str, remote: &str) -> Result<String> {
    let record = repo
        .load_note(object_id)
        .with_context(|| format!("Failed to load note {}", object_id))?;
    let client = Client::new();
    let url = format!("{}/v1/notes", normalize_remote(remote));
    let response = client
        .post(url)
        .json(&record)
        .send()
        .await
        .with_context(|| "Failed to contact remote hub")?;
    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unavailable>".to_string());
        anyhow::bail!("Remote returned {}: {}", status, body);
    }
    let remote_record: NoteRecord = response
        .json()
        .await
        .with_context(|| "Failed to decode hub response")?;
    Ok(remote_record.object_id)
}

pub async fn pull_note(repo: &FukuraRepo, object_id: &str, remote: &str) -> Result<String> {
    let client = Client::new();
    let url = format!("{}/v1/notes/{}", normalize_remote(remote), object_id);
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| "Failed to contact remote hub")?;
    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unavailable>".to_string());
        anyhow::bail!("Remote returned {}: {}", status, body);
    }
    let remote_record: NoteRecord = response
        .json()
        .await
        .with_context(|| "Failed to decode hub response")?;
    let local = repo.store_note(remote_record.note)?;
    Ok(local.object_id)
}
