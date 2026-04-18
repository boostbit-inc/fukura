//! Append-only JSONL storage for [`SolutionAttempt`] records.
//!
//! Attempts are the evidence stream for fukura's effectiveness loop. They
//! are written frequently (every time an agent or shell follow-up reveals
//! whether a suggested fix actually worked) and queried in aggregate
//! ("what's the success rate for this fingerprint?"). Append-only JSONL
//! was chosen for four properties:
//!
//! 1. Crash-safe by construction — every line is an independent record.
//! 2. Trivial to back up, sync, and inspect with standard tools.
//! 3. Cheap writes even under contention.
//! 4. Forward-compatible: unknown fields / records are ignored.
//!
//! The index lives alongside the rest of the repository under
//! `.fukura/attempts.jsonl` and is opt-in for consumers that care about
//! effectiveness data.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};

use crate::domain::attempt::{AttemptStats, SolutionAttempt};

const ATTEMPTS_FILE: &str = "attempts.jsonl";

pub struct AttemptStore {
    path: PathBuf,
    /// Serialises writers so concurrent callers never interleave partial
    /// lines. Reads do not need to take the lock; the file format is
    /// robust to a read racing with a write (worst case: a truncated
    /// last line, which `load_all` skips).
    write_lock: Mutex<()>,
}

impl AttemptStore {
    /// Create (or reuse) the store at `<fukura_dir>/attempts.jsonl`.
    /// `fukura_dir` is typically the `.fukura/` directory of a repository.
    pub fn open(fukura_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(fukura_dir)
            .with_context(|| format!("creating {}", fukura_dir.display()))?;
        let path = fukura_dir.join(ATTEMPTS_FILE);
        if !path.exists() {
            File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        }
        Ok(Self {
            path,
            write_lock: Mutex::new(()),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one attempt record. Returns the attempt after it has been
    /// durably written (followed by an fsync).
    pub fn record(&self, attempt: &SolutionAttempt) -> Result<()> {
        let line = serde_json::to_string(attempt)?;
        let _guard = self
            .write_lock
            .lock()
            .map_err(|e| anyhow::anyhow!("attempt store write lock poisoned: {e}"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("opening {}", self.path.display()))?;
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(())
    }

    /// Load every attempt in the file. Malformed lines are skipped
    /// (not surfaced as errors) so a partially-written last line from a
    /// concurrent writer does not poison the whole store.
    pub fn load_all(&self) -> Result<Vec<SolutionAttempt>> {
        let file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).context("opening attempts file"),
        };
        let mut out = Vec::new();
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(a) = serde_json::from_str::<SolutionAttempt>(line) {
                out.push(a);
            }
        }
        Ok(out)
    }

    /// Fold all attempts into a fingerprint → stats map.
    pub fn stats_by_fingerprint(&self) -> Result<HashMap<String, AttemptStats>> {
        let mut map: HashMap<String, AttemptStats> = HashMap::new();
        for a in self.load_all()? {
            map.entry(a.fingerprint.clone())
                .or_default()
                .record(a.outcome);
        }
        Ok(map)
    }

    /// Cheap accessor for a single fingerprint (loads the whole file;
    /// fine for MVP volumes, to be indexed later if the file grows
    /// unboundedly).
    pub fn stats_for(&self, fingerprint: &str) -> Result<AttemptStats> {
        let mut stats = AttemptStats::default();
        for a in self.load_all()? {
            if a.fingerprint == fingerprint {
                stats.record(a.outcome);
            }
        }
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::attempt::AttemptOutcome;

    fn tmp_store() -> (tempfile::TempDir, AttemptStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = AttemptStore::open(dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn round_trips_a_single_attempt() {
        let (_d, store) = tmp_store();
        let a = SolutionAttempt::new("sha256:abc", AttemptOutcome::Success);
        store.record(&a).unwrap();
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].attempt_id, a.attempt_id);
    }

    #[test]
    fn aggregates_stats_across_many_outcomes() {
        let (_d, store) = tmp_store();
        for _ in 0..3 {
            store
                .record(&SolutionAttempt::new("sha256:x", AttemptOutcome::Success))
                .unwrap();
        }
        store
            .record(&SolutionAttempt::new("sha256:x", AttemptOutcome::Failure))
            .unwrap();
        store
            .record(&SolutionAttempt::new("sha256:y", AttemptOutcome::Abandoned))
            .unwrap();

        let by_fp = store.stats_by_fingerprint().unwrap();
        assert_eq!(by_fp.get("sha256:x").unwrap().success, 3);
        assert_eq!(by_fp.get("sha256:x").unwrap().failure, 1);
        assert_eq!(by_fp.get("sha256:x").unwrap().success_rate(), Some(0.75));
        assert_eq!(by_fp.get("sha256:y").unwrap().abandoned, 1);

        let x_only = store.stats_for("sha256:x").unwrap();
        assert_eq!(x_only.total(), 4);
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let (dir, _store) = tmp_store();
        let path = dir.path().join(ATTEMPTS_FILE);
        // Write a valid line, a garbage line, another valid line.
        let valid = SolutionAttempt::new("sha256:a", AttemptOutcome::Success);
        std::fs::write(
            &path,
            format!(
                "{}\n{{not valid json}}\n{}\n",
                serde_json::to_string(&valid).unwrap(),
                serde_json::to_string(&SolutionAttempt::new("sha256:b", AttemptOutcome::Failure))
                    .unwrap()
            ),
        )
        .unwrap();

        let store = AttemptStore::open(dir.path()).unwrap();
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
    }
}
