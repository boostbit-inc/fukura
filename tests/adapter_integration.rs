//! End-to-end checks that the adapter pipeline attaches ontology data to
//! notes that travel through the repository (store → load).

use chrono::Utc;
use fukura::adapter::{enrich::enrich_note, InvocationContext};
use fukura::models::{Author, Note, Privacy};
use fukura::repo::FukuraRepo;
use std::collections::BTreeMap;

fn skeleton_note(title: &str) -> Note {
    let now = Utc::now();
    Note {
        title: title.into(),
        body: "stub".into(),
        tags: vec!["error".into(), "auto-captured".into()],
        links: vec![],
        meta: BTreeMap::new(),
        solutions: vec![],
        privacy: Privacy::Private,
        created_at: now,
        updated_at: now,
        author: Author {
            name: "tester".into(),
            email: None,
        },
        ontology: None,
    }
}

#[test]
fn cargo_failure_round_trips_ontology_through_repo() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = FukuraRepo::init(tmp.path(), true).expect("init");

    let mut note = skeleton_note("cargo build failed");
    let ctx = InvocationContext {
        command: "cargo build --release".into(),
        exit_code: Some(101),
        stderr: Some("error[E0432]: unresolved import `foo::bar`".into()),
        ..Default::default()
    };
    assert!(enrich_note(&mut note, &ctx));

    let record = repo.store_note(note).expect("store");
    let loaded = repo.load_note(&record.object_id).expect("load");

    let ontology = loaded
        .note
        .ontology
        .as_ref()
        .expect("ontology should round-trip");
    assert_eq!(ontology.adapter, "cargo");
    assert_eq!(ontology.category, "cargo.compile.e0432");
    assert!(ontology.fingerprint.starts_with("sha256:"));

    // Adapter-derived tags survive serialisation.
    assert!(loaded.note.tags.contains(&"cargo".to_string()));
    assert!(loaded.note.tags.contains(&"rust".to_string()));
    // ekp.* meta keys are visible to plain meta consumers.
    assert_eq!(
        loaded.note.meta.get("ekp.adapter").map(String::as_str),
        Some("cargo")
    );
}

#[test]
fn unknown_invocation_yields_generic_ontology() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = FukuraRepo::init(tmp.path(), true).expect("init");

    let mut note = skeleton_note("mystery tool failed");
    let ctx = InvocationContext {
        command: "frobnicate --hard".into(),
        exit_code: Some(127),
        stderr: Some("frobnicate: command not found".into()),
        ..Default::default()
    };
    assert!(enrich_note(&mut note, &ctx));

    let record = repo.store_note(note).expect("store");
    let loaded = repo.load_note(&record.object_id).expect("load");

    let ontology = loaded.note.ontology.as_ref().expect("generic fallback");
    assert_eq!(ontology.adapter, "generic");
    assert_eq!(ontology.category, "generic.unknown");
}

#[test]
fn successful_invocation_does_not_attach_ontology() {
    let mut note = skeleton_note("did nothing");
    let ctx = InvocationContext {
        command: "echo hi".into(),
        exit_code: Some(0),
        ..Default::default()
    };
    assert!(!enrich_note(&mut note, &ctx));
    assert!(note.ontology.is_none());
}
