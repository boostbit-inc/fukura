use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fukura::index::SearchSort;
use fukura::models::{Author, Note, Privacy};
use fukura::repo::FukuraRepo;
use std::collections::BTreeMap;
use tempfile::TempDir;

fn create_benchmark_data(repo: &FukuraRepo, count: usize) -> Vec<String> {
    let mut object_ids = Vec::new();

    for i in 0..count {
        let now = Utc::now();
        let mut meta = BTreeMap::new();
        meta.insert("benchmark".into(), "true".into());
        meta.insert("index".into(), i.to_string());

        let note = Note {
            title: format!("Benchmark Note {}", i),
            body: format!(
                "This is benchmark note number {} with some content about various topics like programming, \
                troubleshooting, debugging, and software development. It contains keywords like error, \
                fix, solution, problem, issue, and resolution. The content is designed to test search \
                performance with realistic text patterns and vocabulary.",
                i
            ),
            tags: vec![
                "benchmark".to_string(),
                "test".to_string(),
                format!("tag{}", i % 10),
            ],
            links: vec![format!("https://example.com/note/{}", i)],
            meta,
            solutions: vec![],
            privacy: Privacy::Private,
            created_at: now,
            updated_at: now,
            author: Author {
                name: "Benchmark Author".into(),
                email: Some("benchmark@test.com".into()),
            },
        };

        let record = repo.store_note(note).expect("Failed to store note");
        object_ids.push(record.object_id);
    }

    object_ids
}

fn bench_search_relevance(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Create test data
    let _object_ids = create_benchmark_data(&repo, 1000);

    c.bench_function("search_relevance", |b| {
        b.iter(|| {
            let results = repo
                .search(
                    black_box("programming error fix"),
                    black_box(10),
                    SearchSort::Relevance,
                )
                .expect("Search failed");
            black_box(results)
        })
    });
}

fn bench_search_updated(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Create test data
    let _object_ids = create_benchmark_data(&repo, 1000);

    c.bench_function("search_updated", |b| {
        b.iter(|| {
            let results = repo
                .search(
                    black_box("troubleshooting solution"),
                    black_box(10),
                    SearchSort::Updated,
                )
                .expect("Search failed");
            black_box(results)
        })
    });
}

fn bench_search_likes(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Create test data
    let _object_ids = create_benchmark_data(&repo, 1000);

    c.bench_function("search_likes", |b| {
        b.iter(|| {
            let results = repo
                .search(
                    black_box("debugging issue"),
                    black_box(10),
                    SearchSort::Likes,
                )
                .expect("Search failed");
            black_box(results)
        })
    });
}

fn bench_store_note(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    c.bench_function("store_note", |b| {
        b.iter(|| {
            let now = Utc::now();
            let mut meta = BTreeMap::new();
            meta.insert("benchmark".into(), "true".into());

            let note = Note {
                title: "Benchmark Store Note".into(),
                body: "This is a benchmark note for testing store performance.".into(),
                tags: vec!["benchmark".to_string(), "store".to_string()],
                links: vec![],
                meta,
                solutions: vec![],
                privacy: Privacy::Private,
                created_at: now,
                updated_at: now,
                author: Author {
                    name: "Benchmark Author".into(),
                    email: Some("benchmark@test.com".into()),
                },
            };

            let record = repo.store_note(note).expect("Failed to store note");
            black_box(record)
        })
    });
}

fn bench_load_note(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Pre-populate with test data
    let object_ids = create_benchmark_data(&repo, 100);

    c.bench_function("load_note", |b| {
        b.iter(|| {
            let object_id = &object_ids[black_box(0)];
            let note = repo.load_note(object_id).expect("Failed to load note");
            black_box(note)
        })
    });
}

criterion_group!(
    benches,
    bench_search_relevance,
    bench_search_updated,
    bench_search_likes,
    bench_store_note,
    bench_load_note
);
criterion_main!(benches);
