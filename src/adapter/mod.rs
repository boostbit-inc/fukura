//! Environment adapters.
//!
//! An adapter observes a developer's invocation context (command, exit code,
//! stdout, stderr, cwd, etc.) and, when it recognises a kind of error it
//! knows about, produces an [`ErrorOntology`](crate::domain::ontology::ErrorOntology)
//! document conforming to EKP v1 (see `docs/ekp-spec.md`).
//!
//! Adapters are the unit of extensibility that lets fukura scale to
//! heterogeneous environments (cargo, kubectl, bazel, company-internal
//! CLIs) without the core needing to know about any of them. A minimal
//! generic adapter is always registered so that every failed invocation
//! yields at least one ontology.

use std::sync::Arc;

use crate::domain::ontology::ErrorOntology;

pub mod builtin;
pub mod enrich;

/// Raw context handed to adapters. Adapters MUST NOT mutate this value.
#[derive(Debug, Clone, Default)]
pub struct InvocationContext {
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub working_directory: Option<String>,
    pub duration_ms: Option<u64>,
    /// Best-effort identification of the shell producing the invocation
    /// (e.g. "zsh", "bash", "fish"). Optional — not all capture paths know
    /// this.
    pub shell: Option<String>,
    /// When non-`None`, indicates that an autonomous agent produced this
    /// invocation rather than a human at a terminal. Used to distinguish
    /// human vs. agent effectiveness signals.
    pub agent_kind: Option<String>,
}

impl InvocationContext {
    /// Returns the first token of the command (e.g. `cargo` for
    /// `cargo build --release`). Handles leading whitespace and quoted
    /// tokens naively; adapters that need more structure should parse the
    /// command themselves.
    pub fn command_head(&self) -> Option<&str> {
        self.command.split_whitespace().next()
    }

    /// Concatenation of stderr + stdout for pattern matching. Adapters
    /// should prefer stderr when a distinction matters.
    pub fn combined_output(&self) -> String {
        let mut out = String::new();
        if let Some(s) = &self.stderr {
            out.push_str(s);
        }
        if let Some(s) = &self.stdout {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(s);
        }
        out
    }

    pub fn is_failure(&self) -> bool {
        matches!(self.exit_code, Some(code) if code != 0)
    }
}

/// Stable ID of an adapter. Lowercase `[a-z0-9_-]+`.
pub type AdapterId = &'static str;

pub trait Adapter: Send + Sync {
    fn id(&self) -> AdapterId;

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Higher priority adapters win when multiple match.
    /// Convention: generic = 0, language/tool specific = 50,
    /// environment/organisation specific = 100+.
    fn priority(&self) -> i32 {
        50
    }

    /// Decide whether this adapter can meaningfully parse `ctx`.
    /// MUST be side-effect free.
    fn matches(&self, ctx: &InvocationContext) -> bool;

    /// Build an ontology from the context. Returns `None` if, despite
    /// `matches` returning `true`, the adapter cannot extract useful
    /// structure (e.g. ambiguous output).
    fn parse(&self, ctx: &InvocationContext) -> Option<ErrorOntology>;

    /// Optional: given a context that only knows the command (no exit
    /// code, no stderr yet), return the fingerprint the adapter expects
    /// a *likely* failure would produce. Used by preflight to tighten
    /// matches before a command actually runs.
    ///
    /// The default implementation attempts `parse` against a synthesised
    /// failing context (exit_code=1, empty stderr) and returns the
    /// resulting fingerprint. Adapters whose fingerprint formula needs
    /// real stderr / exit_code to be meaningful should return `None`.
    fn synthesise_pre_fingerprint(&self, ctx: &InvocationContext) -> Option<String> {
        let mut synthetic = ctx.clone();
        if synthetic.exit_code.is_none() {
            synthetic.exit_code = Some(1);
        }
        if synthetic.stderr.is_none() {
            synthetic.stderr = Some(String::new());
        }
        self.parse(&synthetic).map(|o| o.fingerprint)
    }
}

/// Registry of adapters. Adapters are tried in priority order (descending)
/// and the first one whose `parse` returns `Some` wins. The generic
/// fallback adapter, registered with priority `0`, guarantees the registry
/// always returns an ontology for a failing invocation.
#[derive(Clone, Default)]
pub struct AdapterRegistry {
    adapters: Vec<Arc<dyn Adapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: Vec::new(),
        }
    }

    /// Default registry with all built-in adapters.
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        reg.register(Arc::new(builtin::CargoAdapter));
        reg.register(Arc::new(builtin::DockerAdapter));
        reg.register(Arc::new(builtin::GitAdapter));
        reg.register(Arc::new(builtin::KubernetesAdapter));
        reg.register(Arc::new(builtin::NodeAdapter));
        reg.register(Arc::new(builtin::PythonAdapter));
        reg.register(Arc::new(builtin::TerraformAdapter));
        // Generic must be registered last so that it only wins when nothing
        // else claims the context. Priority resolves the actual order but we
        // keep insertion order deterministic for easier debugging.
        reg.register(Arc::new(builtin::GenericAdapter));
        reg
    }

    pub fn register(&mut self, adapter: Arc<dyn Adapter>) {
        self.adapters.push(adapter);
        // Sort descending by priority so iteration picks the best first.
        self.adapters
            .sort_by_key(|a| std::cmp::Reverse(a.priority()));
    }

    pub fn adapters(&self) -> &[Arc<dyn Adapter>] {
        &self.adapters
    }

    /// Synthesise a likely fingerprint from command context alone.
    /// Returns the first `Some` from adapters in priority order. Useful
    /// for preflight: predict the fingerprint *before* running the
    /// command so matches against prior notes can be tighter than
    /// keyword search.
    pub fn synthesise_pre_fingerprint(&self, ctx: &InvocationContext) -> Option<String> {
        for adapter in &self.adapters {
            if let Some(fp) = adapter.synthesise_pre_fingerprint(ctx) {
                return Some(fp);
            }
        }
        None
    }

    /// Classify a context into an ontology, or `None` when no adapter
    /// matches *and* the generic fallback is not registered.
    pub fn classify(&self, ctx: &InvocationContext) -> Option<ErrorOntology> {
        for adapter in &self.adapters {
            if adapter.matches(ctx) {
                if let Some(ontology) = adapter.parse(ctx) {
                    return Some(ontology);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_picks_highest_priority_match() {
        let mut reg = AdapterRegistry::new();
        reg.register(Arc::new(builtin::GenericAdapter));
        reg.register(Arc::new(builtin::CargoAdapter));

        let ctx = InvocationContext {
            command: "cargo build".into(),
            exit_code: Some(101),
            stderr: Some("error[E0308]: mismatched types\n  --> src/main.rs:3:5".into()),
            ..Default::default()
        };

        let ontology = reg.classify(&ctx).expect("adapter should classify");
        assert_eq!(ontology.adapter, "cargo");
        assert_eq!(ontology.category, "cargo.compile.e0308");
    }

    #[test]
    fn registry_falls_back_to_generic() {
        let reg = AdapterRegistry::with_builtins();

        let ctx = InvocationContext {
            command: "some-unknown-tool --flag".into(),
            exit_code: Some(2),
            stderr: Some("it broke".into()),
            ..Default::default()
        };

        let ontology = reg.classify(&ctx).expect("generic fallback");
        assert_eq!(ontology.adapter, "generic");
        assert_eq!(ontology.category, "generic.unknown");
    }

    #[test]
    fn generic_does_not_match_successful_invocations() {
        let reg = AdapterRegistry::with_builtins();
        let ctx = InvocationContext {
            command: "ls".into(),
            exit_code: Some(0),
            ..Default::default()
        };
        assert!(reg.classify(&ctx).is_none());
    }
}
