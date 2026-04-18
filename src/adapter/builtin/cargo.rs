//! Adapter for Rust's cargo / rustc errors.
//!
//! Recognises diagnostic codes of the form `E0xxx` (e.g. `E0308`) and
//! classifies them under `cargo.compile.<code>`. Falls back to
//! `cargo.compile.unknown` when a failure comes from `cargo` but no
//! diagnostic code could be extracted.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::adapter::{Adapter, InvocationContext};
use crate::domain::ontology::{ErrorOntology, FingerprintInput, Severity};

static DIAG_CODE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"error\[(?P<code>E\d{3,5})\]").expect("valid regex"));

pub struct CargoAdapter;

impl Adapter for CargoAdapter {
    fn id(&self) -> &'static str {
        "cargo"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn matches(&self, ctx: &InvocationContext) -> bool {
        if !ctx.is_failure() {
            return false;
        }
        matches!(ctx.command_head(), Some("cargo") | Some("rustc"))
    }

    fn parse(&self, ctx: &InvocationContext) -> Option<ErrorOntology> {
        if !self.matches(ctx) {
            return None;
        }

        let combined = ctx.combined_output();
        let error_code = DIAG_CODE
            .captures(&combined)
            .and_then(|c| c.name("code"))
            .map(|m| m.as_str().to_owned());

        let category = match &error_code {
            Some(code) => format!("cargo.compile.{}", code.to_lowercase()),
            None => "cargo.compile.unknown".to_owned(),
        };

        let command_head = ctx.command_head().map(str::to_owned);
        let stderr_pattern = error_code
            .as_deref()
            .map(|code| format!("error[{}]: ***", code));

        let fp = FingerprintInput {
            adapter: self.id().to_owned(),
            category: category.clone(),
            error_code: error_code.clone(),
            command_head: command_head.clone(),
            stderr_pattern: stderr_pattern.clone(),
            ..Default::default()
        };

        let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
        ontology.severity = Severity::Blocking;
        ontology.signals.exit_code = ctx.exit_code;
        ontology.signals.error_code = error_code;
        ontology.signals.command_head = command_head;
        ontology.signals.stderr_pattern = stderr_pattern;
        ontology.signals.duration_ms = ctx.duration_ms;
        ontology.tags = vec!["rust".into(), "cargo".into()];
        Some(ontology)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_diagnostic_code() {
        let adapter = CargoAdapter;
        let ctx = InvocationContext {
            command: "cargo build".into(),
            exit_code: Some(101),
            stderr: Some("error[E0308]: mismatched types\n   --> src/main.rs:3:5".into()),
            ..Default::default()
        };
        let ontology = adapter.parse(&ctx).unwrap();
        assert_eq!(ontology.category, "cargo.compile.e0308");
        assert_eq!(ontology.signals.error_code.as_deref(), Some("E0308"));
    }

    #[test]
    fn same_diagnostic_same_fingerprint_across_files() {
        let adapter = CargoAdapter;
        let a = adapter
            .parse(&InvocationContext {
                command: "cargo build".into(),
                exit_code: Some(101),
                stderr: Some("error[E0308]: mismatched types\n   --> src/a.rs:3:5".into()),
                ..Default::default()
            })
            .unwrap();
        let b = adapter
            .parse(&InvocationContext {
                command: "cargo build".into(),
                exit_code: Some(101),
                stderr: Some("error[E0308]: mismatched types\n   --> src/b.rs:42:1".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
    }

    #[test]
    fn rejects_non_cargo_commands() {
        let adapter = CargoAdapter;
        assert!(!adapter.matches(&InvocationContext {
            command: "make build".into(),
            exit_code: Some(1),
            ..Default::default()
        }));
    }

    #[test]
    fn falls_back_when_no_diagnostic_code_found() {
        let adapter = CargoAdapter;
        let ontology = adapter
            .parse(&InvocationContext {
                command: "cargo test".into(),
                exit_code: Some(101),
                stderr: Some("test failed".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(ontology.category, "cargo.compile.unknown");
        assert!(ontology.signals.error_code.is_none());
    }
}
