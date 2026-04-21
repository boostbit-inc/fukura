//! Adapter for Python failures — pytest, pip/uv, direct `python` runs.
//!
//! The generic fallback was producing `generic.unknown` for every
//! Python error, which killed the "fukura classifies my stack" pitch
//! for any Python-first team. This adapter recognises the common
//! high-volume failure modes so pytest / pip / module-resolution
//! errors fingerprint cleanly instead.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::adapter::{Adapter, InvocationContext};
use crate::domain::ontology::{ErrorOntology, FingerprintInput, Severity};

/// ModuleNotFoundError / ImportError — by far the most common class.
/// Fingerprint on the missing module name so every "please pip install
/// X" captures as the same problem regardless of which file raised.
static MODULE_NOT_FOUND: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?:ModuleNotFoundError|ImportError): No module named ['\x22](?P<m>[^'\x22]+)['\x22]",
    )
    .expect("valid regex")
});

/// Generic exception line — "SomeError: message here".
static EXCEPTION_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(?P<kind>[A-Z][A-Za-z0-9_]*(?:Error|Exception)):").expect("valid regex")
});

/// pip's ERESOLVE / No matching distribution pattern.
static PIP_DIST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"No matching distribution found for (?P<pkg>[A-Za-z0-9_.\-]+)")
        .expect("valid regex")
});

/// pytest "collected 0 items" or summary-line failures.
static PYTEST_SUMMARY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^(?:FAILED|ERROR)\s+(?P<test>[^ \n]+)").expect("valid regex"));

pub struct PythonAdapter;

impl Adapter for PythonAdapter {
    fn id(&self) -> &'static str {
        "python"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn matches(&self, ctx: &InvocationContext) -> bool {
        if !ctx.is_failure() {
            return false;
        }
        match ctx.command_head() {
            Some("python") | Some("python3") | Some("pytest") | Some("pip") | Some("pip3")
            | Some("uv") | Some("poetry") | Some("ruff") | Some("mypy") => true,
            _ => {
                // Also match if stderr contains a Python traceback — many
                // wrappers (make, npm, shell scripts) ultimately invoke
                // python and surface its traceback.
                let out = ctx.combined_output();
                out.contains("Traceback (most recent call last)")
            }
        }
    }

    fn parse(&self, ctx: &InvocationContext) -> Option<ErrorOntology> {
        if !self.matches(ctx) {
            return None;
        }

        let combined = ctx.combined_output();

        // Priority 1: ModuleNotFoundError / ImportError — module name is the
        // stable key, makes "pip install X" fixes collapse.
        if let Some(caps) = MODULE_NOT_FOUND.captures(&combined) {
            let module = caps.name("m").unwrap().as_str().to_string();
            let category = "python.import.module_not_found".to_string();
            let mut fp = FingerprintInput {
                adapter: self.id().to_owned(),
                category: category.clone(),
                error_code: Some(module.clone()),
                command_head: ctx.command_head().map(str::to_owned),
                stderr_pattern: Some(format!("ModuleNotFoundError: {module}")),
                ..Default::default()
            };
            fp.error_code = Some(module.clone());
            let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
            ontology.severity = Severity::Blocking;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.error_code = Some(module);
            ontology.signals.command_head = fp.command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["python".into(), "import".into()];
            return Some(ontology);
        }

        // Priority 2: pip can't find a package.
        if let Some(caps) = PIP_DIST.captures(&combined) {
            let pkg = caps.name("pkg").unwrap().as_str().to_string();
            let category = "python.pip.no_distribution".to_string();
            let fp = FingerprintInput {
                adapter: self.id().to_owned(),
                category: category.clone(),
                error_code: Some(pkg.clone()),
                command_head: ctx.command_head().map(str::to_owned),
                stderr_pattern: Some(format!("No matching distribution: {pkg}")),
                ..Default::default()
            };
            let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
            ontology.severity = Severity::Blocking;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.error_code = Some(pkg);
            ontology.signals.command_head = fp.command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["python".into(), "pip".into()];
            return Some(ontology);
        }

        // Priority 3: pytest failure — use the test's dotted path as a
        // signature component (stripping line numbers).
        if ctx.command_head() == Some("pytest") {
            if let Some(caps) = PYTEST_SUMMARY.captures(&combined) {
                let test = caps.name("test").unwrap().as_str().to_string();
                let category = "python.pytest.failure".to_string();
                let fp = FingerprintInput {
                    adapter: self.id().to_owned(),
                    category: category.clone(),
                    error_code: None,
                    command_head: Some("pytest".into()),
                    stderr_pattern: Some(format!("FAILED {test}")),
                    ..Default::default()
                };
                let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
                ontology.severity = Severity::Warning;
                ontology.signals.exit_code = ctx.exit_code;
                ontology.signals.command_head = Some("pytest".into());
                ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
                ontology.tags = vec!["python".into(), "pytest".into()];
                return Some(ontology);
            }
        }

        // Priority 4: generic Python exception line — "TypeError: ..."
        if let Some(caps) = EXCEPTION_LINE.captures(&combined) {
            let kind = caps.name("kind").unwrap().as_str().to_string();
            let category = format!("python.exception.{}", kind.to_lowercase());
            let fp = FingerprintInput {
                adapter: self.id().to_owned(),
                category: category.clone(),
                error_code: Some(kind.clone()),
                command_head: ctx.command_head().map(str::to_owned),
                stderr_pattern: Some(format!("{kind}: ***")),
                ..Default::default()
            };
            let mut ontology = ErrorOntology::new(self.id(), category, fp.compute());
            ontology.severity = Severity::Blocking;
            ontology.signals.exit_code = ctx.exit_code;
            ontology.signals.error_code = Some(kind);
            ontology.signals.command_head = fp.command_head.clone();
            ontology.signals.stderr_pattern = fp.stderr_pattern.clone();
            ontology.tags = vec!["python".into()];
            return Some(ontology);
        }

        // Matched the adapter but no specific signal — let generic handle.
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
    fn module_not_found_fingerprints_on_module_name() {
        let a = PythonAdapter
            .parse(&ctx(
                "python script.py",
                "ModuleNotFoundError: No module named 'requests'",
            ))
            .unwrap();
        let b = PythonAdapter
            .parse(&ctx(
                "python other.py",
                "ModuleNotFoundError: No module named 'requests'",
            ))
            .unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
        assert_eq!(a.category, "python.import.module_not_found");
        assert_eq!(a.signals.error_code.as_deref(), Some("requests"));
    }

    #[test]
    fn different_modules_get_different_fingerprints() {
        let a = PythonAdapter
            .parse(&ctx(
                "python x.py",
                "ModuleNotFoundError: No module named 'requests'",
            ))
            .unwrap();
        let b = PythonAdapter
            .parse(&ctx(
                "python x.py",
                "ModuleNotFoundError: No module named 'pandas'",
            ))
            .unwrap();
        assert_ne!(a.fingerprint, b.fingerprint);
    }

    #[test]
    fn pip_no_distribution_classified() {
        let o = PythonAdapter
            .parse(&ctx(
                "pip install broken-pkg",
                "ERROR: No matching distribution found for broken-pkg",
            ))
            .unwrap();
        assert_eq!(o.category, "python.pip.no_distribution");
        assert_eq!(o.signals.error_code.as_deref(), Some("broken-pkg"));
    }

    #[test]
    fn generic_exception_classified_by_kind() {
        let o = PythonAdapter
            .parse(&ctx(
                "python x.py",
                "Traceback (most recent call last):\n  File...\nTypeError: 'int' object is not iterable",
            ))
            .unwrap();
        assert_eq!(o.category, "python.exception.typeerror");
    }

    #[test]
    fn matches_non_python_command_if_traceback_present() {
        let ctx = InvocationContext {
            command: "make build".into(),
            exit_code: Some(2),
            stderr: Some(
                "Traceback (most recent call last):\n  File foo.py\nValueError: bad".into(),
            ),
            ..Default::default()
        };
        assert!(PythonAdapter.matches(&ctx));
    }

    #[test]
    fn ignores_success() {
        let ctx = InvocationContext {
            command: "python".into(),
            exit_code: Some(0),
            stderr: Some(String::new()),
            ..Default::default()
        };
        assert!(!PythonAdapter.matches(&ctx));
    }
}
