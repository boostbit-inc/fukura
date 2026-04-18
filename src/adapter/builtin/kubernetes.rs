//! Adapter for Kubernetes / kubectl errors.
//!
//! This is the canonical reference for environment-style adapters.
//! Organisation-specific adapters (for internal platforms with custom
//! vocabulary) are conceptually descendants of this one: they recognise
//! the same taxonomy of errors but decorate entities with domain-specific
//! cluster/namespace vocabulary.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::adapter::{Adapter, InvocationContext};
use crate::domain::ontology::{Entity, ErrorOntology, FingerprintInput, Severity};

static NAMESPACE_FLAG: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:-n|--namespace)[ =](?P<ns>[A-Za-z0-9_.-]+)").expect("valid regex")
});

static CONTEXT_FLAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"--context[ =](?P<ctx>[A-Za-z0-9_.-]+)").expect("valid regex"));

pub struct KubernetesAdapter;

impl Adapter for KubernetesAdapter {
    fn id(&self) -> &'static str {
        "kubernetes"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn matches(&self, ctx: &InvocationContext) -> bool {
        if !ctx.is_failure() {
            return false;
        }
        matches!(
            ctx.command_head(),
            Some("kubectl") | Some("helm") | Some("k9s")
        )
    }

    fn parse(&self, ctx: &InvocationContext) -> Option<ErrorOntology> {
        if !self.matches(ctx) {
            return None;
        }

        let combined = ctx.combined_output();
        let (category, error_code, severity, stderr_pattern) = classify(&combined);

        let command_head = ctx.command_head().map(str::to_owned);
        let entities = extract_entities(&ctx.command);
        let entity_types: Vec<String> = entities.iter().map(|e| e.entity_type.clone()).collect();

        let fp = FingerprintInput {
            adapter: self.id().to_owned(),
            category: category.to_owned(),
            error_code: error_code.map(str::to_owned),
            command_head: command_head.clone(),
            stderr_pattern: stderr_pattern.map(str::to_owned),
            entity_types,
            ..Default::default()
        };

        let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
        ontology.severity = severity;
        ontology.signals.exit_code = ctx.exit_code;
        ontology.signals.error_code = error_code.map(str::to_owned);
        ontology.signals.command_head = command_head;
        ontology.signals.stderr_pattern = stderr_pattern.map(str::to_owned);
        ontology.signals.duration_ms = ctx.duration_ms;
        ontology.entities = entities;
        ontology.tags = vec!["kubernetes".into()];
        Some(ontology)
    }
}

fn classify(
    haystack: &str,
) -> (
    &'static str,
    Option<&'static str>,
    Severity,
    Option<&'static str>,
) {
    let lower = haystack.to_lowercase();
    if lower.contains("imagepullbackoff") || lower.contains("errimagepull") {
        (
            "kubernetes.image_pull",
            Some("ImagePullBackOff"),
            Severity::Blocking,
            Some("ImagePullBackOff"),
        )
    } else if lower.contains("crashloopbackoff") {
        (
            "kubernetes.crash_loop",
            Some("CrashLoopBackOff"),
            Severity::Blocking,
            Some("CrashLoopBackOff"),
        )
    } else if lower.contains("oomkilled") || lower.contains("out of memory") {
        (
            "kubernetes.oom",
            Some("OOMKilled"),
            Severity::Blocking,
            Some("OOMKilled"),
        )
    } else if lower.contains("forbidden") || lower.contains("unauthorized") {
        (
            "kubernetes.rbac.forbidden",
            Some("Forbidden"),
            Severity::Blocking,
            Some("forbidden"),
        )
    } else if lower.contains("connection refused") || lower.contains("no such host") {
        (
            "kubernetes.connectivity",
            None,
            Severity::Blocking,
            Some("connection refused"),
        )
    } else if lower.contains("not found") {
        (
            "kubernetes.not_found",
            Some("NotFound"),
            Severity::Warning,
            Some("not found"),
        )
    } else {
        ("kubernetes.unknown", None, Severity::Warning, None)
    }
}

fn extract_entities(command: &str) -> Vec<Entity> {
    let mut entities = Vec::new();
    if let Some(ns) = NAMESPACE_FLAG
        .captures(command)
        .and_then(|c| c.name("ns"))
        .map(|m| m.as_str().to_owned())
    {
        entities.push(Entity::known("namespace", ns));
    }
    if let Some(ctx) = CONTEXT_FLAG
        .captures(command)
        .and_then(|c| c.name("ctx"))
        .map(|m| m.as_str().to_owned())
    {
        entities.push(Entity::known("cluster", ctx));
    }
    entities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_image_pull_backoff() {
        let adapter = KubernetesAdapter;
        let ctx = InvocationContext {
            command: "kubectl get pods -n payments".into(),
            exit_code: Some(1),
            stderr: Some(
                "NAME   READY   STATUS             RESTARTS\napi-0  0/1     ImagePullBackOff   3"
                    .into(),
            ),
            ..Default::default()
        };
        let ontology = adapter.parse(&ctx).unwrap();
        assert_eq!(ontology.category, "kubernetes.image_pull");
        assert!(ontology
            .entities
            .iter()
            .any(|e| e.entity_type == "namespace" && e.value == "payments"));
    }

    #[test]
    fn fingerprint_ignores_pod_names_but_keeps_entity_types() {
        let adapter = KubernetesAdapter;
        let a = adapter
            .parse(&InvocationContext {
                command: "kubectl get pods -n payments --context prod-us-east".into(),
                exit_code: Some(1),
                stderr: Some("api-abc ImagePullBackOff".into()),
                ..Default::default()
            })
            .unwrap();
        let b = adapter
            .parse(&InvocationContext {
                command: "kubectl get pods -n payments --context prod-us-east".into(),
                exit_code: Some(1),
                stderr: Some("api-xyz ImagePullBackOff".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);

        // But a different namespace / cluster combination (different entity
        // *types* or presence) would still hash the same because we hash
        // types, not values. Same types → same fingerprint.
        let c = adapter
            .parse(&InvocationContext {
                command: "kubectl get pods -n billing --context staging-eu".into(),
                exit_code: Some(1),
                stderr: Some("api-foo ImagePullBackOff".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(a.fingerprint, c.fingerprint);
    }

    #[test]
    fn non_k8s_commands_do_not_match() {
        let adapter = KubernetesAdapter;
        assert!(!adapter.matches(&InvocationContext {
            command: "docker ps".into(),
            exit_code: Some(1),
            ..Default::default()
        }));
    }
}
