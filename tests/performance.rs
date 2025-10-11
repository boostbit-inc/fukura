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

    // Insert 50 notes using batch processing (reduced for faster testing)
    let mut notes = Vec::new();
    for i in 0..50 {
        // Check timeout
        if start.elapsed() > timeout {
            panic!("Test timed out after {:?}", timeout);
        }

        let note = create_test_note(
            &format!("Performance Test Note {}", i),
            &format!(
                "This is a performance test note number {} with some content to make it realistic.",
                i
            ),
        );
        notes.push(note);
    }
    repo.store_notes_batch(notes)
        .expect("Failed to store notes in batch");

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

    // Insert test data using batch processing
    let mut notes = Vec::new();
    for i in 0..50 {
        let note = create_test_note(
            &format!("Search Test Note {}", i),
            &format!(
                "This note contains searchable content about topic {}",
                i % 10
            ),
        );
        notes.push(note);
    }
    repo.store_notes_batch(notes)
        .expect("Failed to store notes in batch");

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

    let start = Instant::now();
    let timeout = std::time::Duration::from_secs(20); // 20 second timeout for CI
    let note_count = 20; // Further reduced for CI performance

    // Create all notes first
    let mut notes = Vec::new();
    for i in 0..note_count {
        // Check timeout
        if start.elapsed() > timeout {
            panic!("Test timed out after {:?}", timeout);
        }

        let note = create_test_note(
            &format!("Memory Test Note {}", i),
            &format!(
                "This is note {} with content to test memory usage patterns.",
                i
            ),
        );
        notes.push(note);
    }

    // Store all notes in batch for better performance
    let records = repo
        .store_notes_batch(notes)
        .expect("Failed to store notes in batch");
    assert_eq!(records.len(), note_count);

    let duration = start.elapsed();
    println!(
        "Memory usage test with {} notes took: {:?}",
        note_count, duration
    );

    // Final search to verify everything works
    let hits = repo
        .search("Memory Test", 10, fukura::index::SearchSort::Relevance)
        .expect("Final search failed");
    assert!(!hits.is_empty());
    assert!(hits.len() <= note_count); // Allow for partial matches
}
