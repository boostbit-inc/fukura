//! Per-repository state tracked between `fuku hub` invocations.
//!
//! The first user of this is `fuku hub sync-attempts`, which needs to
//! know which attempts have already been pushed to the hub so it
//! only ships the delta next time (handoff §4 quick win #2).
//! Storing this next to the store it describes keeps the format
//! human-inspectable and lives in the same `.fukura/` directory
//! everything else fukura writes to.
//!
//! The file is JSON (not JSONL) because it represents *scalar state*
//! — there's one value per key, not an append-only log. Concurrent
//! modification by two `fuku hub` processes is not a design concern:
//! the store is user-per-repo, and if two invocations race, the
//! second one's `last_synced_at` simply overwrites the first with an
//! equally-or-more-recent timestamp. Either way attempts are never
//! lost — the remote uses `attempt_id` UNIQUE for idempotency.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const HUB_STATE_FILE: &str = "hub_state.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HubStateData {
    /// Upper bound of `occurred_at` for attempts already known to the
    /// hub. The next sync ships only attempts strictly newer than this.
    /// `None` means "nothing has ever been synced from this repo".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_attempts_synced_at: Option<DateTime<Utc>>,
}

pub struct HubState {
    path: PathBuf,
    data: HubStateData,
}

impl HubState {
    pub fn open(fukura_dir: &Path) -> Result<Self> {
        fs::create_dir_all(fukura_dir)
            .with_context(|| format!("creating {}", fukura_dir.display()))?;
        let path = fukura_dir.join(HUB_STATE_FILE);
        let data = match fs::read_to_string(&path) {
            Ok(text) if !text.trim().is_empty() => serde_json::from_str(&text).unwrap_or_default(),
            _ => HubStateData::default(),
        };
        Ok(Self { path, data })
    }

    pub fn last_attempts_synced_at(&self) -> Option<DateTime<Utc>> {
        self.data.last_attempts_synced_at
    }

    pub fn mark_attempts_synced_up_to(&mut self, timestamp: DateTime<Utc>) -> Result<()> {
        // Only move the marker forward. An older value may be passed
        // in from a partial retry; ignore it.
        if self
            .data
            .last_attempts_synced_at
            .map(|prev| timestamp <= prev)
            .unwrap_or(false)
        {
            return Ok(());
        }
        self.data.last_attempts_synced_at = Some(timestamp);
        self.save()
    }

    fn save(&self) -> Result<()> {
        let text = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.path, text).with_context(|| format!("writing {}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use tempfile::TempDir;

    #[test]
    fn roundtrip_through_disk() {
        let dir = TempDir::new().unwrap();
        let now = Utc::now();

        {
            let mut s = HubState::open(dir.path()).unwrap();
            assert_eq!(s.last_attempts_synced_at(), None);
            s.mark_attempts_synced_up_to(now).unwrap();
        }

        let reopened = HubState::open(dir.path()).unwrap();
        assert_eq!(reopened.last_attempts_synced_at(), Some(now));
    }

    #[test]
    fn marker_only_moves_forward() {
        let dir = TempDir::new().unwrap();
        let mut s = HubState::open(dir.path()).unwrap();
        let later = Utc::now();
        let earlier = later - Duration::seconds(10);

        s.mark_attempts_synced_up_to(later).unwrap();
        s.mark_attempts_synced_up_to(earlier).unwrap();
        assert_eq!(s.last_attempts_synced_at(), Some(later));
    }

    #[test]
    fn corrupt_file_is_ignored() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(HUB_STATE_FILE), "{ not valid json").unwrap();
        let s = HubState::open(dir.path()).unwrap();
        assert_eq!(s.last_attempts_synced_at(), None);
    }
}
