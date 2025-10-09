use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use regex::Regex;

static DEFAULT_PATTERNS: Lazy<Vec<(&str, &str)>> = Lazy::new(|| {
    vec![
        // AWS credentials
        ("aws_access_key", r"AKIA[0-9A-Z]{16}"),
        (
            "aws_secret_key",
            r#"(?i)aws.{0,20}secret.{0,20}['"][0-9a-zA-Z/+=]{40}['"]"#,
        ),
        // API keys and tokens
        ("bearer_token", r"(?i)bearer [a-z0-9\._\-]{20,}"),
        (
            "api_key",
            r#"(?i)api[_-]?key['"]?\s*[:=]\s*['"]?[a-z0-9]{20,}"#,
        ),
        ("github_token", r"ghp_[a-zA-Z0-9]{36}"),
        ("github_oauth", r"gho_[a-zA-Z0-9]{36}"),
        // Generic secrets and passwords
        ("password", r#"(?i)password['"]?\s*[:=]\s*['"]?[^\s'"]{6,}"#),
        (
            "generic_secret",
            r#"(?i)secret[_-]?key['"]?\s*[:=]\s*['"]?[a-z0-9]{20,}"#,
        ),
        // Database connection strings
        ("database_url", r"(?i)(postgres|mysql|mongodb)://[^\s]+"),
        // Private keys
        ("private_key", r"-----BEGIN (RSA |EC )?PRIVATE KEY-----"),
        // JWT tokens
        (
            "jwt",
            r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*",
        ),
        // IP addresses (optional, can be disabled)
        ("ipv4", r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b"),
        // Email addresses
        ("email", r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"),
    ]
});

#[derive(Clone)]
pub struct Redactor {
    rules: Vec<Rule>,
}

#[derive(Clone)]
struct Rule {
    pub regex: Regex,
    pub replacement: String,
}

impl Redactor {
    pub fn default_with_overrides(overrides: &BTreeMap<String, String>) -> Self {
        let mut rules = Vec::new();
        for (name, pattern) in DEFAULT_PATTERNS.iter() {
            let key = overrides.get(*name).cloned();
            if let Some(value) = key {
                if value.trim().is_empty() {
                    continue;
                }
                let regex = Regex::new(&value).unwrap_or_else(|_| Regex::new(pattern).unwrap());
                rules.push(Rule {
                    regex,
                    replacement: format!("__{}_REDACTED__", name.to_uppercase()),
                });
            } else {
                rules.push(Rule {
                    regex: Regex::new(pattern).expect("valid default regex"),
                    replacement: format!("__{}_REDACTED__", name.to_uppercase()),
                });
            }
        }
        Self { rules }
    }

    pub fn redact(&self, input: &str) -> String {
        let mut redacted = input.to_owned();
        for rule in &self.rules {
            redacted = rule
                .regex
                .replace_all(&redacted, rule.replacement.as_str())
                .to_string();
        }
        redacted
    }
}
