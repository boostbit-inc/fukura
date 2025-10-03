use chrono::Utc;
use fukura::models::{Author, Note, Privacy};
use fukura::repo::FukuraRepo;
use std::collections::BTreeMap;
use std::time::Instant;
use tempfile::TempDir;

fn create_test_note(title: &str, body: &str) -> Note {
    let now = Utc::now();
    let mut meta = BTreeMap::new();
    meta.insert("test".into(), "performance".into());

    Note {
        title: title.into(),
        body: body.into(),
        tags: vec!["performance".into(), "test".into()],
        links: vec![],
        meta,
        solutions: vec![],
        privacy: Privacy::Private,
        created_at: now,
        updated_at: now,
        author: Author {
            name: "Performance Tester".into(),
            email: Some("perf@test.com".into()),
        },
    }
}

#[test]
fn test_bulk_insert_performance() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    let start = Instant::now();
    let timeout = std::time::Duration::from_secs(120); // 2 minute timeout

    // Insert 50 notes (reduced for faster testing)
    for i in 0..50 {
        let note = create_test_note(
            &format!("Performance Test Note {}", i),
            &format!(
                "This is a performance test note number {} with some content to make it realistic.",
                i
            ),
        );
        repo.store_note(note).expect("Failed to store note");
        
        // Check timeout
        if start.elapsed() > timeout {
            panic!("Test timed out after {:?}", timeout);
        }
    }

    let duration = start.elapsed();
    println!("Bulk insert of 50 notes took: {:?}", duration);

    // Should complete within reasonable time (adjust threshold as needed)
    assert!(
        duration.as_millis() < 60000, // 1 minute timeout
        "Bulk insert took too long: {:?}",
        duration
    );
}

#[test]
fn test_search_performance() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Insert test data
    for i in 0..50 {
        let note = create_test_note(
            &format!("Search Test Note {}", i),
            &format!(
                "This note contains searchable content about topic {}",
                i % 10
            ),
        );
        repo.store_note(note).expect("Failed to store note");
    }

    let start = Instant::now();

    // Perform multiple searches
    for _ in 0..10 {
        let hits = repo
            .search("searchable", 10, fukura::index::SearchSort::Relevance)
            .expect("Search failed");
        assert!(!hits.is_empty());
    }

    let duration = start.elapsed();
    println!("10 searches took: {:?}", duration);

    // Should complete within reasonable time
    assert!(
        duration.as_millis() < 2000,
        "Search took too long: {:?}",
        duration
    );
}

#[test]
fn test_memory_usage() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Insert many notes to test memory usage
    for i in 0..1000 {
        let note = create_test_note(
            &format!("Memory Test Note {}", i),
            &format!(
                "This is note {} with content to test memory usage patterns.",
                i
            ),
        );
        repo.store_note(note).expect("Failed to store note");

        // Every 100 notes, verify we can still search
        if i % 100 == 0 && i > 0 {
            let hits = repo
                .search("memory", 5, fukura::index::SearchSort::Relevance)
                .expect("Search failed");
            assert!(!hits.is_empty());
        }
    }
}
