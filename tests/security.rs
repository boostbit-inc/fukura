use chrono::Utc;
use fukura::models::{Author, Note, Privacy};
use fukura::repo::FukuraRepo;
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

fn create_malicious_note() -> Note {
    let now = Utc::now();
    let mut meta = BTreeMap::new();

    // Test various potentially malicious inputs
    meta.insert("path_traversal".into(), "../../etc/passwd".into());
    meta.insert(
        "script_injection".into(),
        "<script>alert('xss')</script>".into(),
    );
    meta.insert("sql_injection".into(), "'; DROP TABLE notes; --".into());

    Note {
        title: "../../etc/passwd".into(), // Path traversal attempt
        body: "<script>alert('xss')</script>'; DROP TABLE notes; --".into(), // Multiple injection attempts
        tags: vec![
            "../../etc/passwd".into(),
            "<script>alert('xss')</script>".into(),
            "'; DROP TABLE notes; --".into(),
            "normal_tag".into(),
        ],
        links: vec![
            "javascript:alert('xss')".into(),
            "https://evil.com".into(),
            "normal_link".into(),
        ],
        meta,
        solutions: vec![],
        privacy: Privacy::Private,
        created_at: now,
        updated_at: now,
        author: Author {
            name: "<script>alert('xss')</script>".into(),
            email: Some("evil@hacker.com".into()),
        },
    }
}

#[test]
fn test_malicious_input_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Store malicious note
    let record = repo
        .store_note(create_malicious_note())
        .expect("Failed to store malicious note");

    // Verify it was stored safely (no path traversal occurred)
    let repo_path = temp_dir.path();
    let fukura_dir = repo_path.join(".fukura");
    assert!(fukura_dir.exists());

    // Verify no files were created outside the repo
    let etc_passwd = repo_path.join("etc").join("passwd");
    assert!(!etc_passwd.exists());

    // Retrieve the note and verify content is preserved as-is
    let retrieved = repo
        .load_note(&record.object_id)
        .expect("Failed to load note");
    assert_eq!(retrieved.note.title, "../../etc/passwd");
    assert!(retrieved
        .note
        .body
        .contains("<script>alert('xss')</script>"));
    assert!(retrieved
        .note
        .tags
        .contains(&"../../etc/passwd".to_string()));
}

#[test]
fn test_file_permissions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Check that repo directory has appropriate permissions
    let fukura_dir = temp_dir.path().join(".fukura");
    let metadata = fs::metadata(&fukura_dir).expect("Failed to get metadata");

    // On Unix systems, check permissions (this will be a no-op on Windows)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        // Should not be world-writable
        assert_eq!(mode & 0o002, 0, "Repository should not be world-writable");
    }
}

#[test]
fn test_sensitive_data_redaction() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Create note with potentially sensitive data
    let now = Utc::now();
    let mut meta = BTreeMap::new();
    meta.insert("password".into(), "password=mysecret123".into());
    meta.insert("api_key".into(), "api_key=sk1234567890abcdefghij".into());
    meta.insert("aws_key".into(), "AKIAIOSFODNN7EXAMPLE".into());

    let note = Note {
        title: "Sensitive Data Test".into(),
        body: "Config: password=\"secret999\", api_key=\"sk1234567890abcdefghij\", AWS Key: AKIAIOSFODNN7EXAMPLE, Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.ODIxNDU".into(),
        tags: vec!["sensitive".into()],
        links: vec![],
        meta,
        solutions: vec![],
        privacy: Privacy::Private,
        created_at: now,
        updated_at: now,
        author: Author {
            name: "Security Tester".into(),
            email: Some("security@test.com".into()),
        },
    };

    let record = repo.store_note(note).expect("Failed to store note");
    let retrieved = repo
        .load_note(&record.object_id)
        .expect("Failed to load note");

    // Verify sensitive data is redacted (security feature working correctly)
    // This test ensures the redaction mechanism is working properly

    // Body should be redacted
    assert!(!retrieved.note.body.contains("AKIAIOSFODNN7EXAMPLE"), "AWS key should be redacted");
    assert!(!retrieved.note.body.contains("secret999"), "Password should be redacted");
    assert!(retrieved.note.body.contains("__AWS_ACCESS_KEY_REDACTED__") || 
            retrieved.note.body.contains("__PASSWORD_REDACTED__"), "Redaction markers should be present");

    // Meta fields should also be redacted
    let aws_value = retrieved.note.meta.get("aws_key").unwrap();
    assert!(!aws_value.contains("AKIAIOSFODNN7EXAMPLE"), "AWS key in meta should be redacted");
}

#[test]
fn test_large_input_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = FukuraRepo::init(temp_dir.path(), true).expect("Failed to init repo");

    // Create note with very large content
    let large_content = "A".repeat(1_000_000); // 1MB of content

    let now = Utc::now();
    let note = Note {
        title: "Large Content Test".into(),
        body: large_content.clone(),
        tags: vec!["large".into()],
        links: vec![],
        meta: BTreeMap::new(),
        solutions: vec![],
        privacy: Privacy::Private,
        created_at: now,
        updated_at: now,
        author: Author {
            name: "Large Content Tester".into(),
            email: Some("large@test.com".into()),
        },
    };

    // Should handle large content gracefully
    let record = repo.store_note(note).expect("Failed to store large note");
    let retrieved = repo
        .load_note(&record.object_id)
        .expect("Failed to load large note");

    assert_eq!(retrieved.note.body, large_content);
}
