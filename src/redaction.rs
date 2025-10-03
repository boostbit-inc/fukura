use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use regex::Regex;

static DEFAULT_PATTERNS: Lazy<Vec<(&str, &str)>> = Lazy::new(|| {
    vec![
        ("aws_access_key", r"AKIA[0-9A-Z]{16}"),
        ("bearer_token", r"(?i)bearer [a-z0-9\._\-]{20,}"),
        ("generic_secret", r"(?i)secret[a-zA-Z0-9]+"),
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
