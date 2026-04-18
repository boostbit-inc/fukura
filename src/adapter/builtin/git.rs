//! Adapter for common `git` errors.
//!
//! v0.1 covers the high-frequency cases: merge conflicts, non-fast-forward
//! pushes, detached HEAD operations, and auth failures. Branch and ref
//! names are not included in the fingerprint to keep the same logical
//! error deduplicated across branches.

use crate::adapter::{Adapter, InvocationContext};
use crate::domain::ontology::{ErrorOntology, FingerprintInput, Severity};

pub struct GitAdapter;

impl Adapter for GitAdapter {
    fn id(&self) -> &'static str {
        "git"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn matches(&self, ctx: &InvocationContext) -> bool {
        if !ctx.is_failure() {
            return false;
        }
        matches!(ctx.command_head(), Some("git"))
    }

    fn parse(&self, ctx: &InvocationContext) -> Option<ErrorOntology> {
        if !self.matches(ctx) {
            return None;
        }

        let combined = ctx.combined_output().to_lowercase();
        let (category, stderr_pattern, severity) = classify(&combined);

        let command_head = ctx.command_head().map(str::to_owned);
        let fp = FingerprintInput {
            adapter: self.id().to_owned(),
            category: category.to_owned(),
            command_head: command_head.clone(),
            stderr_pattern: stderr_pattern.map(str::to_owned),
            ..Default::default()
        };

        let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
        ontology.severity = severity;
        ontology.signals.exit_code = ctx.exit_code;
        ontology.signals.command_head = command_head;
        ontology.signals.stderr_pattern = stderr_pattern.map(str::to_owned);
        ontology.signals.duration_ms = ctx.duration_ms;
        ontology.tags = vec!["git".into()];
        Some(ontology)
    }
}

fn classify(haystack: &str) -> (&'static str, Option<&'static str>, Severity) {
    if haystack.contains("merge conflict") || haystack.contains("conflict (content)") {
        (
            "git.merge.conflict",
            Some("merge conflict"),
            Severity::Blocking,
        )
    } else if haystack.contains("non-fast-forward") || haystack.contains("rejected") {
        (
            "git.push.non_fast_forward",
            Some("non-fast-forward"),
            Severity::Blocking,
        )
    } else if haystack.contains("authentication failed") || haystack.contains("permission denied") {
        (
            "git.auth.failed",
            Some("authentication failed"),
            Severity::Blocking,
        )
    } else if haystack.contains("not a git repository") {
        (
            "git.repo.missing",
            Some("not a git repository"),
            Severity::Warning,
        )
    } else if haystack.contains("detached head") {
        (
            "git.detached_head",
            Some("detached head"),
            Severity::Warning,
        )
    } else {
        ("git.unknown", None, Severity::Warning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_merge_conflict() {
        let adapter = GitAdapter;
        let ctx = InvocationContext {
            command: "git merge origin/main".into(),
            exit_code: Some(1),
            stderr: Some("CONFLICT (content): Merge conflict in src/main.rs".into()),
            ..Default::default()
        };
        let ontology = adapter.parse(&ctx).unwrap();
        assert_eq!(ontology.category, "git.merge.conflict");
    }

    #[test]
    fn detects_non_fast_forward() {
        let adapter = GitAdapter;
        let ctx = InvocationContext {
            command: "git push origin main".into(),
            exit_code: Some(1),
            stderr: Some("! [rejected]        main -> main (non-fast-forward)".into()),
            ..Default::default()
        };
        let ontology = adapter.parse(&ctx).unwrap();
        assert_eq!(ontology.category, "git.push.non_fast_forward");
    }

    #[test]
    fn fingerprint_stable_across_branches() {
        let adapter = GitAdapter;
        let a = adapter
            .parse(&InvocationContext {
                command: "git push origin feature/a".into(),
                exit_code: Some(1),
                stderr: Some("! [rejected] feature/a -> feature/a (non-fast-forward)".into()),
                ..Default::default()
            })
            .unwrap();
        let b = adapter
            .parse(&InvocationContext {
                command: "git push origin feature/b".into(),
                exit_code: Some(1),
                stderr: Some("! [rejected] feature/b -> feature/b (non-fast-forward)".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
    }
}
