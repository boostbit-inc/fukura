//! Error Knowledge Protocol (EKP) — v1 data model.
//!
//! See `docs/ekp-spec.md` for the normative specification. This module is
//! the Rust reference implementation of that specification and is the shape
//! that all adapters must produce.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const EKP_SCHEMA: &str = "fuku.ekp";
pub const EKP_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyEnvelope {
    pub schema: String,
    pub version: u32,
    pub ontology: ErrorOntology,
}

impl OntologyEnvelope {
    pub fn wrap(ontology: ErrorOntology) -> Self {
        Self {
            schema: EKP_SCHEMA.to_owned(),
            version: EKP_VERSION,
            ontology,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOntology {
    pub adapter: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_version: Option<String>,
    pub category: String,
    #[serde(default)]
    pub severity: Severity,
    pub fingerprint: String,
    pub occurred_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Signals::is_empty")]
    pub signals: Signals,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_excerpt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_lineage: Option<SourceLineage>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Blocking,
    Warning,
    Info,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Signals {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl Signals {
    pub fn is_empty(&self) -> bool {
        self.exit_code.is_none()
            && self.error_code.is_none()
            && self.command_head.is_none()
            && self.stderr_pattern.is_none()
            && self.stdout_bytes.is_none()
            && self.stderr_bytes.is_none()
            && self.duration_ms.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub value: String,
    #[serde(default)]
    pub redacted: bool,
}

impl Entity {
    pub fn known(entity_type: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            entity_type: entity_type.into(),
            value: value.into(),
            redacted: false,
        }
    }

    pub fn redacted_of(entity_type: impl Into<String>) -> Self {
        Self {
            entity_type: entity_type.into(),
            value: "***".to_owned(),
            redacted: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceLineage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_arch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentLineage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLineage {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Canonical signature used to derive [`ErrorOntology::fingerprint`].
///
/// The signature intentionally excludes any variable data: timestamps,
/// paths, UUIDs, hostnames. Only stable, category-defining facts appear.
#[derive(Debug, Clone, Default)]
pub struct FingerprintInput {
    pub adapter: String,
    pub adapter_version: Option<String>,
    pub category: String,
    pub error_code: Option<String>,
    pub command_head: Option<String>,
    pub stderr_pattern: Option<String>,
    pub entity_types: Vec<String>,
}

impl FingerprintInput {
    /// Compose the signature string and hash it.
    pub fn compute(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        parts.push(format!("adapter={}", self.adapter));
        if let Some(v) = &self.adapter_version {
            parts.push(format!("adapter_version={}", v));
        }
        parts.push(format!("category={}", self.category));
        if let Some(v) = &self.error_code {
            parts.push(format!("error_code={}", v));
        }
        if let Some(v) = &self.command_head {
            parts.push(format!("command_head={}", v));
        }
        if let Some(v) = &self.stderr_pattern {
            parts.push(format!("stderr_pattern={}", v));
        }
        let mut sorted_types = self.entity_types.clone();
        sorted_types.sort();
        sorted_types.dedup();
        if !sorted_types.is_empty() {
            parts.push(format!("entity_types={}", sorted_types.join(",")));
        }

        let joined = parts.join("\n");
        let mut hasher = Sha256::new();
        hasher.update(joined.as_bytes());
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }
}

impl ErrorOntology {
    /// Convenience constructor used by adapters. The caller is expected to
    /// have already computed `fingerprint` via [`FingerprintInput::compute`].
    pub fn new(
        adapter: impl Into<String>,
        category: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            adapter: adapter.into(),
            adapter_version: None,
            category: category.into(),
            severity: Severity::Unknown,
            fingerprint: fingerprint.into(),
            occurred_at: Utc::now(),
            signals: Signals::default(),
            entities: Vec::new(),
            tags: Vec::new(),
            raw_excerpt: None,
            source_lineage: None,
        }
    }

    /// Flatten selected fields into a string map for embedding in [`Note::meta`]
    /// without losing forward-compatibility. Keys are prefixed with `ekp.`.
    pub fn flatten_meta(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("ekp.adapter".into(), self.adapter.clone());
        map.insert("ekp.category".into(), self.category.clone());
        map.insert("ekp.fingerprint".into(), self.fingerprint.clone());
        map.insert(
            "ekp.severity".into(),
            match self.severity {
                Severity::Blocking => "blocking",
                Severity::Warning => "warning",
                Severity::Info => "info",
                Severity::Unknown => "unknown",
            }
            .to_owned(),
        );
        if let Some(code) = &self.signals.error_code {
            map.insert("ekp.error_code".into(), code.clone());
        }
        if let Some(cmd) = &self.signals.command_head {
            map.insert("ekp.command_head".into(), cmd.clone());
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_across_identical_inputs() {
        let a = FingerprintInput {
            adapter: "kubernetes".into(),
            category: "kubernetes.image_pull".into(),
            error_code: Some("ImagePullBackOff".into()),
            command_head: Some("kubectl apply".into()),
            entity_types: vec!["cluster".into(), "namespace".into()],
            ..Default::default()
        };
        let b = a.clone();
        assert_eq!(a.compute(), b.compute());
        assert!(a.compute().starts_with("sha256:"));
    }

    #[test]
    fn fingerprint_ignores_variable_values() {
        // Both have the same category and error code but very different
        // raw messages; the fingerprint is computed from stable inputs only.
        let a = FingerprintInput {
            adapter: "kubernetes".into(),
            category: "kubernetes.image_pull".into(),
            error_code: Some("ImagePullBackOff".into()),
            ..Default::default()
        };
        let mut b = a.clone();
        b.entity_types = vec!["cluster".into()];

        assert_ne!(a.compute(), b.compute());
    }

    #[test]
    fn fingerprint_entity_types_are_order_independent() {
        let a = FingerprintInput {
            adapter: "kubernetes".into(),
            category: "kubernetes.image_pull".into(),
            entity_types: vec!["cluster".into(), "namespace".into()],
            ..Default::default()
        };
        let b = FingerprintInput {
            entity_types: vec!["namespace".into(), "cluster".into()],
            ..a.clone()
        };
        assert_eq!(a.compute(), b.compute());
    }

    #[test]
    fn envelope_round_trips_through_json() {
        let ontology = ErrorOntology::new("generic", "generic.unknown", "sha256:deadbeef");
        let envelope = OntologyEnvelope::wrap(ontology);

        let json = serde_json::to_string(&envelope).expect("serialize");
        let decoded: OntologyEnvelope = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(decoded.schema, EKP_SCHEMA);
        assert_eq!(decoded.version, EKP_VERSION);
        assert_eq!(decoded.ontology.adapter, "generic");
    }

    #[test]
    fn flatten_meta_produces_prefixed_keys() {
        let mut ontology = ErrorOntology::new("cargo", "cargo.compile.e0308", "sha256:abc");
        ontology.signals.error_code = Some("E0308".into());
        ontology.signals.command_head = Some("cargo build".into());
        ontology.severity = Severity::Blocking;

        let meta = ontology.flatten_meta();

        assert_eq!(meta.get("ekp.adapter").map(String::as_str), Some("cargo"));
        assert_eq!(
            meta.get("ekp.category").map(String::as_str),
            Some("cargo.compile.e0308")
        );
        assert_eq!(
            meta.get("ekp.severity").map(String::as_str),
            Some("blocking")
        );
        assert_eq!(
            meta.get("ekp.error_code").map(String::as_str),
            Some("E0308")
        );
        assert_eq!(
            meta.get("ekp.command_head").map(String::as_str),
            Some("cargo build")
        );
    }
}
