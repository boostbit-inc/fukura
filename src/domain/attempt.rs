//! Solution attempt records — the write side of the effectiveness loop.
//!
//! When an error is surfaced (by an adapter, by a note, or by an agent),
//! the next action taken and its outcome becomes evidence about which
//! solutions actually work. Recording these attempts gives fukura a way
//! to sort suggestions by measured success rate rather than by recency
//! or keyword match alone.
//!
//! v0.1 covers the minimum viable schema: fingerprint, outcome, optional
//! suggested-note linkage, optional agent lineage. See
//! `docs/ekp-spec.md` §9 for the spec-level description.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const ATTEMPT_SCHEMA: &str = "fuku.ekp.attempt";
pub const ATTEMPT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AttemptOutcome {
    /// The next command after the error succeeded.
    Success,
    /// The next command after the error also failed.
    Failure,
    /// The user or agent gave up without a successful follow-up.
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionAttempt {
    pub schema: String,
    pub version: u32,
    pub attempt_id: String,
    /// EKP fingerprint of the error being attempted against.
    pub fingerprint: String,
    pub outcome: AttemptOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_note_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_kind: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl SolutionAttempt {
    pub fn new(fingerprint: impl Into<String>, outcome: AttemptOutcome) -> Self {
        Self {
            schema: ATTEMPT_SCHEMA.to_owned(),
            version: ATTEMPT_VERSION,
            attempt_id: uuid::Uuid::new_v4().to_string(),
            fingerprint: fingerprint.into(),
            outcome,
            suggested_note_id: None,
            next_command: None,
            agent_kind: None,
            occurred_at: Utc::now(),
        }
    }
}

/// Aggregated effectiveness statistics for a single fingerprint.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttemptStats {
    pub success: u32,
    pub failure: u32,
    pub abandoned: u32,
}

impl AttemptStats {
    pub fn total(&self) -> u32 {
        self.success + self.failure + self.abandoned
    }

    /// Success rate as a value in `[0.0, 1.0]`, or `None` when no attempts
    /// have been recorded (callers should treat this as "unknown" rather
    /// than "0% success").
    pub fn success_rate(&self) -> Option<f64> {
        let total = self.total();
        if total == 0 {
            None
        } else {
            Some(self.success as f64 / total as f64)
        }
    }

    pub fn record(&mut self, outcome: AttemptOutcome) {
        match outcome {
            AttemptOutcome::Success => self.success += 1,
            AttemptOutcome::Failure => self.failure += 1,
            AttemptOutcome::Abandoned => self.abandoned += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_track_outcomes() {
        let mut s = AttemptStats::default();
        s.record(AttemptOutcome::Success);
        s.record(AttemptOutcome::Success);
        s.record(AttemptOutcome::Failure);
        s.record(AttemptOutcome::Abandoned);

        assert_eq!(s.success, 2);
        assert_eq!(s.failure, 1);
        assert_eq!(s.abandoned, 1);
        assert_eq!(s.total(), 4);
        assert_eq!(s.success_rate(), Some(0.5));
    }

    #[test]
    fn empty_stats_have_no_rate() {
        let s = AttemptStats::default();
        assert_eq!(s.success_rate(), None);
    }

    #[test]
    fn attempt_round_trips_through_json() {
        let mut a = SolutionAttempt::new("sha256:abc", AttemptOutcome::Success);
        a.suggested_note_id = Some("note-123".into());
        a.next_command = Some("cargo update".into());
        a.agent_kind = Some("claude-code".into());

        let json = serde_json::to_string(&a).unwrap();
        let b: SolutionAttempt = serde_json::from_str(&json).unwrap();
        assert_eq!(a.attempt_id, b.attempt_id);
        assert_eq!(b.outcome, AttemptOutcome::Success);
        assert_eq!(b.agent_kind.as_deref(), Some("claude-code"));
    }
}
