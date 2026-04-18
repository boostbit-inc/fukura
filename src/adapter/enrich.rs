//! Glue between the adapter registry and the [`Note`] domain model.
//!
//! Auto-capture sites construct a [`Note`] from raw shell context and then
//! ask this module to enrich it with an EKP ontology. Enrichment never
//! fails the caller: when no adapter classifies the context the note is
//! returned unchanged.

use std::sync::OnceLock;

use crate::adapter::{AdapterRegistry, InvocationContext};
use crate::domain::models::Note;

static DEFAULT_REGISTRY: OnceLock<AdapterRegistry> = OnceLock::new();

/// Process-wide registry initialised on first use. Cheap to call repeatedly
/// from hot paths.
pub fn default_registry() -> &'static AdapterRegistry {
    DEFAULT_REGISTRY.get_or_init(AdapterRegistry::with_builtins)
}

/// Classify `ctx` with the default registry and, when a match is found,
/// attach the resulting ontology to `note`.
///
/// Side effects on the note:
/// - `note.ontology` is set to the produced ontology;
/// - tags from the ontology are merged into `note.tags` (deduplicated,
///   stable order);
/// - `note.meta` gains the `ekp.*` keys from
///   [`ErrorOntology::flatten_meta`](crate::domain::ontology::ErrorOntology::flatten_meta).
///
/// Returns `true` when an ontology was attached.
pub fn enrich_note(note: &mut Note, ctx: &InvocationContext) -> bool {
    let Some(ontology) = default_registry().classify(ctx) else {
        return false;
    };

    for (key, value) in ontology.flatten_meta() {
        note.meta.entry(key).or_insert(value);
    }

    let mut tags = std::mem::take(&mut note.tags);
    for tag in &ontology.tags {
        if !tags.iter().any(|t| t == tag) {
            tags.push(tag.clone());
        }
    }
    note.tags = tags;

    note.ontology = Some(ontology);
    true
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::models::{Author, Privacy};

    fn empty_note() -> Note {
        Note {
            title: "t".into(),
            body: "b".into(),
            tags: vec!["error".into()],
            links: vec![],
            meta: BTreeMap::new(),
            solutions: vec![],
            privacy: Privacy::Private,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            author: Author {
                name: "tester".into(),
                email: None,
            },
            ontology: None,
        }
    }

    #[test]
    fn enriches_cargo_failure() {
        let mut note = empty_note();
        let ctx = InvocationContext {
            command: "cargo build".into(),
            exit_code: Some(101),
            stderr: Some("error[E0308]: mismatched types".into()),
            ..Default::default()
        };
        assert!(enrich_note(&mut note, &ctx));

        let ontology = note.ontology.as_ref().unwrap();
        assert_eq!(ontology.adapter, "cargo");
        assert_eq!(ontology.category, "cargo.compile.e0308");

        assert_eq!(
            note.meta.get("ekp.adapter").map(String::as_str),
            Some("cargo")
        );
        assert!(note.tags.contains(&"cargo".to_string()));
        assert!(note.tags.contains(&"rust".to_string()));
        // Pre-existing tag is preserved.
        assert!(note.tags.contains(&"error".to_string()));
    }

    #[test]
    fn leaves_note_alone_on_success() {
        let mut note = empty_note();
        let ctx = InvocationContext {
            command: "ls".into(),
            exit_code: Some(0),
            ..Default::default()
        };
        assert!(!enrich_note(&mut note, &ctx));
        assert!(note.ontology.is_none());
    }

    #[test]
    fn meta_does_not_overwrite_existing_keys() {
        let mut note = empty_note();
        note.meta.insert("ekp.adapter".into(), "preserved".into());
        let ctx = InvocationContext {
            command: "cargo build".into(),
            exit_code: Some(101),
            stderr: Some("error[E0308]: mismatched types".into()),
            ..Default::default()
        };
        assert!(enrich_note(&mut note, &ctx));
        assert_eq!(
            note.meta.get("ekp.adapter").map(String::as_str),
            Some("preserved")
        );
    }
}
