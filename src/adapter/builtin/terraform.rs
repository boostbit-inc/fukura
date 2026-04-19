//! Adapter for Terraform / OpenTofu failures.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::adapter::{Adapter, InvocationContext};
use crate::domain::ontology::{ErrorOntology, FingerprintInput, Severity};

/// State lock held by another process / CI job.
static STATE_LOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"Error: Error acquiring the state lock|Lock Info:\s*ID:\s*(?P<id>[a-f0-9-]+)")
        .expect("valid regex")
});

/// Generic Terraform "Error: <title>" diagnostic.
static DIAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^Error:\s+(?P<title>[^\n]+)").expect("valid regex"));

/// Missing provider / required argument.
static PROVIDER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(Failed to install provider|required_providers block missing|Unsupported provider|Invalid provider configuration)",
    )
    .expect("valid regex")
});

pub struct TerraformAdapter;

impl Adapter for TerraformAdapter {
    fn id(&self) -> &'static str {
        "terraform"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn matches(&self, ctx: &InvocationContext) -> bool {
        if !ctx.is_failure() {
            return false;
        }
        matches!(
            ctx.command_head(),
            Some("terraform") | Some("tofu") | Some("opentofu")
        )
    }

    fn parse(&self, ctx: &InvocationContext) -> Option<ErrorOntology> {
        if !self.matches(ctx) {
            return None;
        }

        let combined = ctx.combined_output();
        let command_head = ctx.command_head().map(str::to_owned);

        if STATE_LOCK.is_match(&combined) {
            return Some(emit(
                self.id(),
                "terraform.state.lock_held",
                "state lock held".into(),
                command_head,
                ctx,
                vec!["terraform".into(), "state".into()],
            ));
        }

        if PROVIDER.is_match(&combined) {
            return Some(emit(
                self.id(),
                "terraform.provider.error",
                "provider install/config error".into(),
                command_head,
                ctx,
                vec!["terraform".into(), "provider".into()],
            ));
        }

        if let Some(caps) = DIAG.captures(&combined) {
            let title = caps
                .name("title")
                .unwrap()
                .as_str()
                .chars()
                .take(80)
                .collect::<String>();
            // Hash on the error-title noun only; strip variable suffixes
            // (paths, resource names) by keeping just the first two words.
            let key = title
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join("_");
            let category = format!(
                "terraform.diag.{}",
                key.to_lowercase().replace(':', "").replace('/', "_")
            );
            let fp = FingerprintInput {
                adapter: self.id().to_owned(),
                category: category.clone(),
                error_code: Some(key.clone()),
                command_head: command_head.clone(),
                stderr_pattern: Some(format!("Error: {key} ***")),
                ..Default::default()
            };
            let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
            ontology.severity = Severity::Blocking;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.error_code = Some(key);
            ontology.signals.command_head = command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["terraform".into()];
            return Some(ontology);
        }

        None
    }
}

fn emit(
    adapter: &'static str,
    category: &'static str,
    stderr_pattern: String,
    command_head: Option<String>,
    ctx: &InvocationContext,
    tags: Vec<String>,
) -> ErrorOntology {
    let fp = FingerprintInput {
        adapter: adapter.to_owned(),
        category: category.to_owned(),
        error_code: None,
        command_head: command_head.clone(),
        stderr_pattern: Some(stderr_pattern.clone()),
        ..Default::default()
    };
    let mut ontology = ErrorOntology::new(adapter, category, fp.compute());
    ontology.severity = Severity::Blocking;
    ontology.signals.exit_code = ctx.exit_code;
    ontology.signals.command_head = command_head;
    ontology.signals.stderr_pattern = Some(stderr_pattern);
    ontology.tags = tags;
    ontology
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(cmd: &str, stderr: &str) -> InvocationContext {
        InvocationContext {
            command: cmd.into(),
            exit_code: Some(1),
            stderr: Some(stderr.into()),
            ..Default::default()
        }
    }

    #[test]
    fn state_lock_classified() {
        let o = TerraformAdapter
            .parse(&ctx(
                "terraform apply",
                "Error: Error acquiring the state lock\nLock Info:\n  ID: abc-123",
            ))
            .unwrap();
        assert_eq!(o.category, "terraform.state.lock_held");
    }

    #[test]
    fn provider_issue_classified() {
        let o = TerraformAdapter
            .parse(&ctx(
                "terraform init",
                "Error: Failed to install provider\n\nError while installing hashicorp/aws",
            ))
            .unwrap();
        assert_eq!(o.category, "terraform.provider.error");
    }

    #[test]
    fn diag_fingerprints_by_title_nouns() {
        let a = TerraformAdapter
            .parse(&ctx(
                "terraform plan",
                "Error: Reference to undeclared resource\n...somewhere in main.tf",
            ))
            .unwrap();
        let b = TerraformAdapter
            .parse(&ctx(
                "terraform plan",
                "Error: Reference to undeclared resource\n...different file",
            ))
            .unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
    }

    #[test]
    fn ignores_success() {
        assert!(!TerraformAdapter.matches(&InvocationContext {
            command: "terraform plan".into(),
            exit_code: Some(0),
            ..Default::default()
        }));
    }
}
