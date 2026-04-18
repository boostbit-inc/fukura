//! Effectiveness tracker — the glue that turns a stream of shell
//! invocations into recorded [`SolutionAttempt`] records.
//!
//! The tracker is a pure state machine: it holds, per session, the
//! fingerprint of the most recent *failing* invocation ("pending"), and
//! reacts to three events:
//!
//! - `observe(failure)` — start or replace the pending fingerprint.
//! - `observe(success)` — when a pending fingerprint exists, write a
//!   `Success` attempt and clear it.
//! - `abandon(session)` — when a session ends with a pending fingerprint
//!   still unresolved, write an `Abandoned` attempt.
//!
//! Consecutive failures overwrite the pending fingerprint rather than
//! chaining; v0.1 optimises for the common case of "tried one thing, got
//! one outcome". Fancier semantics (record failures against the
//! prior-pending, rolling window, etc.) are deliberately deferred.
//!
//! The tracker never panics on missing adapters: when classification
//! yields nothing (rare, since the generic fallback catches most
//! failures) it simply skips the event.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::adapter::{enrich, InvocationContext};
use crate::domain::attempt::{AttemptOutcome, SolutionAttempt};
use crate::infrastructure::attempt_storage::AttemptStore;

const PENDING_FILE: &str = "pending_attempts.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pending {
    fingerprint: String,
    failed_command: String,
    #[serde(default)]
    agent_kind: Option<String>,
}

/// Per-session tracker. Thread-safe via an internal `Mutex`.
#[derive(Clone)]
pub struct EffectivenessTracker {
    store: Arc<AttemptStore>,
    pending: Arc<Mutex<HashMap<String, Pending>>>,
}

impl EffectivenessTracker {
    pub fn new(store: Arc<AttemptStore>) -> Self {
        Self {
            store,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Feed one invocation observation. Returns the fingerprint associated
    /// with any side effect (either a freshly-pending failure or the
    /// resolved pending fingerprint that was just written as a success),
    /// purely for callers that want to log or correlate.
    pub fn observe(&self, session_id: &str, ctx: &InvocationContext) -> Result<Option<String>> {
        if ctx.is_failure() {
            let Some(ontology) = enrich::default_registry().classify(ctx) else {
                return Ok(None);
            };
            let fp = ontology.fingerprint.clone();
            let mut pending = self
                .pending
                .lock()
                .map_err(|e| anyhow::anyhow!("pending lock poisoned: {e}"))?;
            pending.insert(
                session_id.to_owned(),
                Pending {
                    fingerprint: fp.clone(),
                    failed_command: ctx.command.clone(),
                    agent_kind: ctx.agent_kind.clone(),
                },
            );
            Ok(Some(fp))
        } else {
            let prior = {
                let mut pending = self
                    .pending
                    .lock()
                    .map_err(|e| anyhow::anyhow!("pending lock poisoned: {e}"))?;
                pending.remove(session_id)
            };
            let Some(prior) = prior else {
                return Ok(None);
            };
            let mut attempt = SolutionAttempt::new(&prior.fingerprint, AttemptOutcome::Success);
            attempt.next_command = Some(ctx.command.clone());
            attempt.agent_kind = prior.agent_kind;
            self.store.record(&attempt)?;
            Ok(Some(prior.fingerprint))
        }
    }

    /// Abandon whatever pending fingerprint the session is carrying.
    /// No-op when the session has nothing pending.
    pub fn abandon(&self, session_id: &str) -> Result<Option<String>> {
        let prior = {
            let mut pending = self
                .pending
                .lock()
                .map_err(|e| anyhow::anyhow!("pending lock poisoned: {e}"))?;
            pending.remove(session_id)
        };
        let Some(prior) = prior else {
            return Ok(None);
        };
        let mut attempt = SolutionAttempt::new(&prior.fingerprint, AttemptOutcome::Abandoned);
        attempt.next_command = Some(prior.failed_command);
        attempt.agent_kind = prior.agent_kind;
        self.store.record(&attempt)?;
        Ok(Some(prior.fingerprint))
    }

    /// Whether a session currently has a pending fingerprint — mostly for
    /// tests and diagnostics.
    pub fn has_pending(&self, session_id: &str) -> bool {
        self.pending
            .lock()
            .map(|p| p.contains_key(session_id))
            .unwrap_or(false)
    }

    /// Load any previously-persisted pending entries from
    /// `<fukura_dir>/pending_attempts.json`. Missing file is not an
    /// error — it is the normal state for a fresh repository.
    /// Malformed content is also silently ignored: the tracker would
    /// rather lose one round of pending state than refuse to start.
    pub fn load_pending(&self, fukura_dir: &Path) -> Result<()> {
        let path = fukura_dir.join(PENDING_FILE);
        if !path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let loaded: HashMap<String, Pending> = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        let mut pending = self
            .pending
            .lock()
            .map_err(|e| anyhow::anyhow!("pending lock poisoned: {e}"))?;
        *pending = loaded;
        Ok(())
    }

    /// Durably write the current pending map to the JSON file. Intended
    /// to be called after every `observe` / `abandon` in CLI-driven use
    /// cases (shell hooks); daemons can keep the state in-memory and
    /// skip this entirely.
    pub fn save_pending(&self, fukura_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(fukura_dir)
            .with_context(|| format!("creating {}", fukura_dir.display()))?;
        let snapshot = {
            let pending = self
                .pending
                .lock()
                .map_err(|e| anyhow::anyhow!("pending lock poisoned: {e}"))?;
            pending.clone()
        };
        let path = fukura_dir.join(PENDING_FILE);
        let tmp = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(&snapshot)?;
        std::fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("renaming into {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::attempt::{AttemptOutcome, AttemptStats};

    fn setup() -> (tempfile::TempDir, EffectivenessTracker, Arc<AttemptStore>) {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(AttemptStore::open(dir.path()).unwrap());
        let tracker = EffectivenessTracker::new(store.clone());
        (dir, tracker, store)
    }

    fn failure(command: &str) -> InvocationContext {
        InvocationContext {
            command: command.into(),
            exit_code: Some(1),
            stderr: Some("error[E0308]: mismatched types".into()),
            ..Default::default()
        }
    }

    fn success(command: &str) -> InvocationContext {
        InvocationContext {
            command: command.into(),
            exit_code: Some(0),
            ..Default::default()
        }
    }

    fn stats(store: &AttemptStore) -> AttemptStats {
        let all = store.load_all().unwrap();
        let mut s = AttemptStats::default();
        for a in all {
            s.record(a.outcome);
        }
        s
    }

    #[test]
    fn success_after_failure_is_recorded() {
        let (_dir, tracker, store) = setup();

        tracker.observe("sess-1", &failure("cargo build")).unwrap();
        assert!(tracker.has_pending("sess-1"));
        tracker.observe("sess-1", &success("cargo check")).unwrap();
        assert!(!tracker.has_pending("sess-1"));

        let s = stats(&store);
        assert_eq!(s.success, 1);
        assert_eq!(s.total(), 1);
    }

    #[test]
    fn success_without_pending_is_a_no_op() {
        let (_dir, tracker, store) = setup();
        tracker.observe("sess-1", &success("echo hi")).unwrap();
        assert_eq!(stats(&store).total(), 0);
    }

    #[test]
    fn abandon_records_abandoned_attempt() {
        let (_dir, tracker, store) = setup();
        tracker.observe("sess-1", &failure("cargo build")).unwrap();
        tracker.abandon("sess-1").unwrap();
        assert!(!tracker.has_pending("sess-1"));

        let s = stats(&store);
        assert_eq!(s.abandoned, 1);
    }

    #[test]
    fn consecutive_failures_overwrite_pending() {
        let (_dir, tracker, store) = setup();
        tracker.observe("sess-1", &failure("cargo build")).unwrap();
        tracker.observe("sess-1", &failure("cargo test")).unwrap();
        // Only one pending, only one resolution when success comes in.
        tracker.observe("sess-1", &success("cargo fmt")).unwrap();

        let s = stats(&store);
        assert_eq!(s.success, 1, "single success attempt: {s:?}");
    }

    #[test]
    fn sessions_are_isolated() {
        let (_dir, tracker, store) = setup();
        tracker.observe("sess-a", &failure("cargo build")).unwrap();
        tracker.observe("sess-b", &failure("git push")).unwrap();

        // Success in session b must not resolve session a.
        tracker
            .observe("sess-b", &success("git push --force"))
            .unwrap();
        assert!(tracker.has_pending("sess-a"));
        assert!(!tracker.has_pending("sess-b"));

        let s = stats(&store);
        assert_eq!(s.success, 1);
    }

    #[test]
    fn next_command_is_captured_on_success_attempt() {
        let (_dir, tracker, store) = setup();
        tracker.observe("sess-1", &failure("cargo build")).unwrap();
        tracker
            .observe("sess-1", &success("cargo update -p foo"))
            .unwrap();

        let all = store.load_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].outcome, AttemptOutcome::Success);
        assert_eq!(all[0].next_command.as_deref(), Some("cargo update -p foo"));
    }

    #[test]
    fn agent_kind_propagates_from_failure_to_attempt() {
        let (_dir, tracker, store) = setup();
        let mut ctx = failure("cargo build");
        ctx.agent_kind = Some("claude-code".into());
        tracker.observe("sess-1", &ctx).unwrap();
        tracker.observe("sess-1", &success("cargo check")).unwrap();

        let all = store.load_all().unwrap();
        assert_eq!(all[0].agent_kind.as_deref(), Some("claude-code"));
    }
}
