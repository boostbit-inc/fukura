//! Fallback adapter. Matches any failing invocation and produces a minimal
//! ontology. Guarantees EKP conformance even when no specialised adapter
//! recognises the error.

use crate::adapter::{Adapter, InvocationContext};
use crate::domain::ontology::{ErrorOntology, FingerprintInput, Severity};

pub struct GenericAdapter;

impl Adapter for GenericAdapter {
    fn id(&self) -> &'static str {
        "generic"
    }

    fn priority(&self) -> i32 {
        0
    }

    fn matches(&self, ctx: &InvocationContext) -> bool {
        ctx.is_failure()
    }

    fn parse(&self, ctx: &InvocationContext) -> Option<ErrorOntology> {
        if !ctx.is_failure() {
            return None;
        }

        let command_head = ctx.command_head().map(str::to_owned);
        let fp = FingerprintInput {
            adapter: self.id().to_owned(),
            category: "generic.unknown".to_owned(),
            command_head: command_head.clone(),
            ..Default::default()
        };

        let mut ontology = ErrorOntology::new(self.id(), "generic.unknown", fp.compute());
        ontology.severity = Severity::Warning;
        ontology.signals.exit_code = ctx.exit_code;
        ontology.signals.command_head = command_head;
        ontology.signals.duration_ms = ctx.duration_ms;

        if let Some(stderr) = &ctx.stderr {
            let excerpt = truncate(stderr, 512);
            if !excerpt.is_empty() {
                ontology.raw_excerpt = Some(excerpt);
            }
        }
        Some(ontology)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_owned()
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_matches_any_failure() {
        let adapter = GenericAdapter;
        assert!(adapter.matches(&InvocationContext {
            command: "foo".into(),
            exit_code: Some(1),
            ..Default::default()
        }));
        assert!(!adapter.matches(&InvocationContext {
            command: "foo".into(),
            exit_code: Some(0),
            ..Default::default()
        }));
    }

    #[test]
    fn generic_produces_stable_fingerprint_per_command_head() {
        let adapter = GenericAdapter;
        let a = adapter
            .parse(&InvocationContext {
                command: "make deploy".into(),
                exit_code: Some(2),
                stderr: Some("boom".into()),
                ..Default::default()
            })
            .unwrap();
        let b = adapter
            .parse(&InvocationContext {
                // different stderr, same command head — same fingerprint.
                command: "make deploy target=prod".into(),
                exit_code: Some(3),
                stderr: Some("different message".into()),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(a.fingerprint, b.fingerprint);
    }
}
