//! Adapter for `docker` / `docker compose` / buildx failures.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::adapter::{Adapter, InvocationContext};
use crate::domain::ontology::{ErrorOntology, FingerprintInput, Severity};

/// Docker CLI error code: "ERROR: Cannot connect to the Docker daemon"
static DAEMON: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"Cannot connect to the Docker daemon").expect("valid regex"));

/// buildx / dockerfile parse error
static DOCKERFILE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^Dockerfile:\d+").expect("valid regex"));

/// "ERROR: for <service>  ... " compose failure
static COMPOSE_SVC: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)ERROR: for (?P<svc>[^\s]+)").expect("valid regex"));

/// Image pull / push: "pull access denied", "manifest unknown", "denied"
static REGISTRY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(pull access denied|manifest unknown|unauthorized: authentication required|denied: requested access)")
        .expect("valid regex")
});

pub struct DockerAdapter;

impl Adapter for DockerAdapter {
    fn id(&self) -> &'static str {
        "docker"
    }

    fn priority(&self) -> i32 {
        75
    }

    fn matches(&self, ctx: &InvocationContext) -> bool {
        if !ctx.is_failure() {
            return false;
        }
        matches!(ctx.command_head(), Some("docker") | Some("podman"))
    }

    fn parse(&self, ctx: &InvocationContext) -> Option<ErrorOntology> {
        if !self.matches(ctx) {
            return None;
        }

        let combined = ctx.combined_output();
        let command_head = ctx.command_head().map(str::to_owned);

        if DAEMON.is_match(&combined) {
            return Some(emit(
                self.id(),
                "docker.daemon.not_running",
                "Cannot connect to the Docker daemon".into(),
                command_head,
                ctx,
                vec!["docker".into(), "daemon".into()],
            ));
        }

        if let Some(caps) = COMPOSE_SVC.captures(&combined) {
            let svc = caps.name("svc").unwrap().as_str().to_string();
            let category = "docker.compose.service_failure".to_string();
            let fp = FingerprintInput {
                adapter: "docker".to_owned(),
                category: category.clone(),
                error_code: Some(svc.clone()),
                command_head: command_head.clone(),
                stderr_pattern: Some(format!("ERROR: for {svc}")),
                ..Default::default()
            };
            let mut ontology = ErrorOntology::new("docker", category, fp.compute());
            ontology.severity = Severity::Blocking;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.error_code = Some(svc);
            ontology.signals.command_head = command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["docker".into(), "compose".into()];
            return Some(ontology);
        }

        if REGISTRY.is_match(&combined) {
            return Some(emit(
                self.id(),
                "docker.registry.unauthorized",
                "registry auth failure".into(),
                command_head,
                ctx,
                vec!["docker".into(), "registry".into()],
            ));
        }

        if DOCKERFILE.is_match(&combined) {
            return Some(emit(
                self.id(),
                "docker.build.parse_error",
                "Dockerfile parse error".into(),
                command_head,
                ctx,
                vec!["docker".into(), "build".into()],
            ));
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
    fn daemon_not_running_classified() {
        let o = DockerAdapter
            .parse(&ctx(
                "docker ps",
                "Cannot connect to the Docker daemon at unix:///var/run/docker.sock",
            ))
            .unwrap();
        assert_eq!(o.category, "docker.daemon.not_running");
    }

    #[test]
    fn compose_service_failure_fingerprints_on_service() {
        let a = DockerAdapter
            .parse(&ctx(
                "docker compose up",
                "ERROR: for db  Container db exited",
            ))
            .unwrap();
        let b = DockerAdapter
            .parse(&ctx(
                "docker compose up",
                "ERROR: for db  Container db exited later",
            ))
            .unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
        assert_eq!(a.signals.error_code.as_deref(), Some("db"));
    }

    #[test]
    fn registry_auth_classified() {
        let o = DockerAdapter
            .parse(&ctx(
                "docker push myimg",
                "denied: requested access to the resource is denied",
            ))
            .unwrap();
        assert_eq!(o.category, "docker.registry.unauthorized");
    }

    #[test]
    fn ignores_success() {
        assert!(!DockerAdapter.matches(&InvocationContext {
            command: "docker ps".into(),
            exit_code: Some(0),
            ..Default::default()
        }));
    }
}
