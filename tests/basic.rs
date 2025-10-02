use std::collections::BTreeMap;

use chrono::Utc;
use fukura::index::SearchSort;
use fukura::models::{Author, Note, Privacy};
use fukura::repo::FukuraRepo;

fn sample_note() -> Note {
    let now = Utc::now();
    let mut meta = BTreeMap::new();
    meta.insert("os".into(), "macos".into());
    meta.insert("tool".into(), "internal-proxy".into());
    Note {
        title: "Proxy install fails".into(),
        body: "Update the proxy credentials and retry the setup script.".into(),
        tags: vec!["proxy".into(), "install".into()],
        links: vec!["https://internal.example.com/runbook".into()],
        meta,
        solutions: vec![],
        privacy: Privacy::Private,
        created_at: now,
        updated_at: now,
        author: Author {
            name: "Woven Developer".into(),
            email: Some("dev@example.com".into()),
        },
    }
}

#[test]
fn init_and_store_note() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = FukuraRepo::init(tmp.path(), true)?;
    let record = repo.store_note(sample_note())?;
    assert!(
        record.object_id.len() >= 6,
        "object id should be hex-encoded"
    );

    let fetched = repo.load_note(&record.object_id)?;
    assert_eq!(fetched.note.title, "Proxy install fails");
    assert_eq!(fetched.note.tags, vec!["proxy", "install"]);

    let hits = repo.search("proxy", 5, SearchSort::Relevance)?;
    assert!(!hits.is_empty(), "search should surface stored notes");
    Ok(())
}

#[test]
fn resolve_prefixes() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = FukuraRepo::init(tmp.path(), true)?;
    let record = repo.store_note(sample_note())?;
    let short = &record.object_id[..8];
    let resolved = repo.resolve_object_id(short)?;
    assert_eq!(resolved, record.object_id);
    Ok(())
}

#[test]
fn pack_and_prune() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = FukuraRepo::init(tmp.path(), true)?;
    let record = repo.store_note(sample_note())?;
    let object_path = {
        let (prefix, rest) = record.object_id.split_at(2);
        repo.objects_dir().join(prefix).join(rest)
    };
    assert!(object_path.exists());

    repo.pack_loose_objects(true)?;

    assert!(!object_path.exists());
    let loaded = repo.load_note(&record.object_id)?;
    assert_eq!(loaded.note.title, "Proxy install fails");
    Ok(())
}
