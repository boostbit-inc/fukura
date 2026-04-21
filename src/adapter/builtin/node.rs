//! Adapter for Node.js / npm / pnpm / yarn / jest / vitest / tsc failures.
//!
//! Frontend- and backend-JS projects hit a small set of recurring
//! failure modes (dependency resolution, module-not-found, TS
//! diagnostic codes, test runner failures). Capturing them with
//! stable fingerprints rather than falling through to the generic
//! bucket makes fukura useful for the enormous JS/TS population.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::adapter::{Adapter, InvocationContext};
use crate::domain::ontology::{ErrorOntology, FingerprintInput, Severity};

/// npm/pnpm/yarn: "Cannot find module 'X'" / "MODULE_NOT_FOUND".
static MODULE_NOT_FOUND: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:Cannot find module|MODULE_NOT_FOUND)(?:[^'"\n]*)['"](?P<m>[^'"\n]+)['"]"#)
        .expect("valid regex")
});

/// npm ERESOLVE peer dep conflict.
static ERESOLVE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^npm (?:ERR!|error) code (?P<code>ERESOLVE|EACCES|ENOENT|E404|EAI_AGAIN)")
        .expect("valid regex")
});

/// TypeScript diagnostic codes — "error TSxxxx".
static TS_DIAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"error TS(?P<code>\d{3,5}):").expect("valid regex"));

/// Jest / Vitest failing suite — "FAIL path/to/test.ts".
static JEST_FAIL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^FAIL\s+(?P<file>[^\s]+)").expect("valid regex"));

/// Generic node runtime exception — "TypeError: ..." at top of
/// traceback.
static NODE_EXCEPTION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^(?P<kind>[A-Z][A-Za-z0-9_]*Error):\s").expect("valid regex"));

pub struct NodeAdapter;

impl Adapter for NodeAdapter {
    fn id(&self) -> &'static str {
        "node"
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
            Some("node")
                | Some("npm")
                | Some("npx")
                | Some("pnpm")
                | Some("yarn")
                | Some("bun")
                | Some("tsc")
                | Some("jest")
                | Some("vitest")
                | Some("next")
                | Some("vite")
                | Some("eslint")
        )
    }

    fn parse(&self, ctx: &InvocationContext) -> Option<ErrorOntology> {
        if !self.matches(ctx) {
            return None;
        }

        let combined = ctx.combined_output();
        let command_head = ctx.command_head().map(str::to_owned);

        // 1. npm-family canonical error codes.
        if let Some(caps) = ERESOLVE.captures(&combined) {
            let code = caps.name("code").unwrap().as_str().to_string();
            let category = format!("node.npm.{}", code.to_lowercase());
            let fp = FingerprintInput {
                adapter: self.id().to_owned(),
                category: category.clone(),
                error_code: Some(code.clone()),
                command_head: command_head.clone(),
                stderr_pattern: Some(format!("npm code {code}")),
                ..Default::default()
            };
            let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
            ontology.severity = Severity::Blocking;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.error_code = Some(code);
            ontology.signals.command_head = command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["node".into(), "npm".into()];
            return Some(ontology);
        }

        // 2. Cannot find module.
        if let Some(caps) = MODULE_NOT_FOUND.captures(&combined) {
            let m = caps.name("m").unwrap().as_str().to_string();
            let category = "node.module.not_found".to_string();
            let fp = FingerprintInput {
                adapter: self.id().to_owned(),
                category: category.clone(),
                error_code: Some(m.clone()),
                command_head: command_head.clone(),
                stderr_pattern: Some(format!("Cannot find module: {m}")),
                ..Default::default()
            };
            let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
            ontology.severity = Severity::Blocking;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.error_code = Some(m);
            ontology.signals.command_head = command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["node".into(), "module".into()];
            return Some(ontology);
        }

        // 3. TypeScript diagnostic code.
        if let Some(caps) = TS_DIAG.captures(&combined) {
            let code = caps.name("code").unwrap().as_str().to_string();
            let category = format!("node.tsc.ts{}", code);
            let fp = FingerprintInput {
                adapter: self.id().to_owned(),
                category: category.clone(),
                error_code: Some(format!("TS{code}")),
                command_head: command_head.clone(),
                stderr_pattern: Some(format!("error TS{code}: ***")),
                ..Default::default()
            };
            let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
            ontology.severity = Severity::Blocking;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.error_code = Some(format!("TS{code}"));
            ontology.signals.command_head = command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["node".into(), "typescript".into()];
            return Some(ontology);
        }

        // 4. Jest / Vitest FAIL.
        if JEST_FAIL.find(&combined).is_some() {
            let category = "node.test.failure".to_string();
            let fp = FingerprintInput {
                adapter: self.id().to_owned(),
                category: category.clone(),
                error_code: None,
                command_head: command_head.clone(),
                stderr_pattern: Some("FAIL ***".into()),
                ..Default::default()
            };
            let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
            ontology.severity = Severity::Warning;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.command_head = command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["node".into(), "test".into()];
            return Some(ontology);
        }

        // 5. Generic Node exception.
        if let Some(caps) = NODE_EXCEPTION.captures(&combined) {
            let kind = caps.name("kind").unwrap().as_str().to_string();
            let category = format!("node.exception.{}", kind.to_lowercase());
            let fp = FingerprintInput {
                adapter: self.id().to_owned(),
                category: category.clone(),
                error_code: Some(kind.clone()),
                command_head: command_head.clone(),
                stderr_pattern: Some(format!("{kind}: ***")),
                ..Default::default()
            };
            let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
            ontology.severity = Severity::Blocking;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.error_code = Some(kind);
            ontology.signals.command_head = command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["node".into()];
            return Some(ontology);
        }

        None
    }
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
    fn module_not_found_collapses_by_module() {
        let a = NodeAdapter
            .parse(&ctx("node app.js", "Error: Cannot find module 'lodash'"))
            .unwrap();
        let b = NodeAdapter
            .parse(&ctx("node other.js", "Error: Cannot find module 'lodash'"))
            .unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
        assert_eq!(a.category, "node.module.not_found");
    }

    #[test]
    fn eresolve_classified() {
        let o = NodeAdapter
            .parse(&ctx(
                "npm install",
                "npm ERR! code ERESOLVE\nnpm ERR! ERESOLVE could not resolve",
            ))
            .unwrap();
        assert_eq!(o.category, "node.npm.eresolve");
    }

    #[test]
    fn typescript_diagnostic_extracted() {
        let o = NodeAdapter
            .parse(&ctx(
                "tsc --noEmit",
                "src/x.ts(3,5): error TS2322: Type 'string' is not assignable",
            ))
            .unwrap();
        assert_eq!(o.category, "node.tsc.ts2322");
        assert_eq!(o.signals.error_code.as_deref(), Some("TS2322"));
    }

    #[test]
    fn jest_fail_classified_as_test_failure() {
        let o = NodeAdapter
            .parse(&ctx("jest", "FAIL src/foo.test.ts\n  ● should work"))
            .unwrap();
        assert_eq!(o.category, "node.test.failure");
    }

    #[test]
    fn ignores_success() {
        assert!(!NodeAdapter.matches(&InvocationContext {
            command: "npm install".into(),
            exit_code: Some(0),
            ..Default::default()
        }));
    }
}
