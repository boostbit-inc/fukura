use std::process::Command;
use tempfile::TempDir;

/// Comprehensive CLI UX tests
/// Tests various user scenarios and use cases
fn get_binary_path() -> std::path::PathBuf {
    std::env::current_dir()
        .expect("Failed to get current dir")
        .join("target")
        .join("debug")
        .join("fukura")
}

fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let binary_path = get_binary_path();

    let output = Command::new(&binary_path)
        .args(["init", "--no-daemon", "--no-hooks"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute init command");

    assert!(output.status.success());
    temp_dir
}

// ============================================================================
// Basic Command Tests
// ============================================================================

#[test]
fn test_help_command() {
    let binary_path = get_binary_path();
    let output = Command::new(&binary_path)
        .arg("--help")
        .output()
        .expect("Failed to execute help command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show main commands
    assert!(stdout.contains("init"));
    assert!(stdout.contains("add"));
    assert!(stdout.contains("search"));
    assert!(stdout.contains("rec"));
    assert!(stdout.contains("log"));
}

#[test]
fn test_version_command() {
    let binary_path = get_binary_path();
    let output = Command::new(&binary_path)
        .arg("--version")
        .output()
        .expect("Failed to execute version command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fuku") || stdout.contains("0.3.4"));
}

// ============================================================================
// Recording Workflow Tests
// ============================================================================

#[test]
fn test_recording_workflow() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Start recording
    let output = Command::new(&binary_path)
        .args(["rec", "Test workflow"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to start recording");

    assert!(output.status.success());

    // Check status
    let status_output = Command::new(&binary_path)
        .args(["rec", "--status"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to check status");

    assert!(status_output.status.success());
    let stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(stdout.contains("Recording in progress") || stdout.contains("Test workflow"));

    // Stop recording
    let done_output = Command::new(&binary_path)
        .args(["done"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to stop recording");

    assert!(done_output.status.success());
}

#[test]
fn test_time_based_recording_workflow() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Test various time formats
    let time_formats = vec!["3m ago", "1h ago", "30m ago"];

    for time_format in time_formats {
        // Clean up
        let _ = Command::new(&binary_path)
            .args(["rec", "--stop"])
            .current_dir(temp_dir.path())
            .output();

        let output = Command::new(&binary_path)
            .args(["rec", "Time-based test", time_format])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to start time-based recording");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Should not fail due to format parsing
            assert!(
                !stderr.to_lowercase().contains("invalid format"),
                "Format should be valid: {}",
                time_format
            );
        }
    }
}

// ============================================================================
// Activity Tracking Tests
// ============================================================================

#[test]
fn test_track_command() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Check status
    let output = Command::new(&binary_path)
        .args(["track", "--status"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to check track status");

    assert!(output.status.success());

    // Enable tracking
    let output = Command::new(&binary_path)
        .args(["track", "--start"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to start tracking");

    assert!(output.status.success());

    // Disable tracking
    let output = Command::new(&binary_path)
        .args(["track", "--stop"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to stop tracking");

    assert!(output.status.success());
}

#[test]
fn test_log_command_basic() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Test log command
    let output = Command::new(&binary_path)
        .args(["log"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute log command");

    assert!(output.status.success());
}

#[test]
fn test_log_command_with_limit() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    let output = Command::new(&binary_path)
        .args(["log", "-n", "5"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute log command");

    assert!(output.status.success());
}

#[test]
fn test_log_command_oneline() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    let output = Command::new(&binary_path)
        .args(["log", "--oneline"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute log command");

    assert!(output.status.success());
}

// ============================================================================
// Note Management Tests
// ============================================================================

#[test]
fn test_add_search_view_workflow() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Add a note
    let output = Command::new(&binary_path)
        .args([
            "add",
            "--title",
            "Test note",
            "--body",
            "Test content",
            "--no-editor",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to add note");

    assert!(output.status.success());

    // Search for it
    let search_output = Command::new(&binary_path)
        .args(["search", "Test"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to search");

    assert!(search_output.status.success());
    let stdout = String::from_utf8_lossy(&search_output.stdout);
    assert!(stdout.contains("Test note"));

    // View latest
    let view_output = Command::new(&binary_path)
        .args(["view", "@latest"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to view");

    assert!(view_output.status.success());
}

// ============================================================================
// Git-like Command Tests
// ============================================================================

#[test]
fn test_git_like_commands() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Status-like commands
    let cmds = vec![vec!["status"], vec!["stats"], vec!["list"]];

    for cmd in cmds {
        let output = Command::new(&binary_path)
            .args(&cmd)
            .current_dir(temp_dir.path())
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute {:?}", cmd));

        assert!(output.status.success(), "Command {:?} should succeed", cmd);
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_invalid_subcommand() {
    let binary_path = get_binary_path();
    let output = Command::new(&binary_path)
        .arg("invalid_command")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("unrecognized") || stderr.to_lowercase().contains("invalid")
    );
}

#[test]
fn test_missing_required_arguments() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // rec without title
    let output = Command::new(&binary_path)
        .args(["rec"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.to_lowercase().contains("required") || stderr.to_lowercase().contains("title")
        );
    }
}

// ============================================================================
// Daemon Management Tests
// ============================================================================

#[test]
fn test_daemon_lifecycle() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Check status
    let output = Command::new(&binary_path)
        .args(["status"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to check status");

    assert!(output.status.success());
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_config_show() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    let output = Command::new(&binary_path)
        .args(["config", "show"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to show config");

    assert!(output.status.success());
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_empty_repository_commands() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // These should work even with empty repo
    let safe_commands = vec![
        vec!["list"],
        vec!["stats"],
        vec!["log"],
        vec!["track", "--status"],
    ];

    for cmd in safe_commands {
        let output = Command::new(&binary_path)
            .args(&cmd)
            .current_dir(temp_dir.path())
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute {:?}", cmd));

        assert!(
            output.status.success(),
            "Command {:?} should work with empty repo",
            cmd
        );
    }
}

#[test]
fn test_special_refs() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Add a note first
    let _add = Command::new(&binary_path)
        .args(["add", "--title", "Test", "--body", "Content", "--no-editor"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to add note");

    // Test special refs
    let refs = vec!["@latest", "@1"];

    for ref_str in refs {
        let output = Command::new(&binary_path)
            .args(["view", ref_str])
            .current_dir(temp_dir.path())
            .output()
            .unwrap_or_else(|_| panic!("Failed to view {}", ref_str));

        assert!(output.status.success(), "Should resolve ref: {}", ref_str);
    }
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_bulk_operations() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Add multiple notes
    for i in 0..10 {
        let output = Command::new(&binary_path)
            .args([
                "add",
                "--title",
                &format!("Note {}", i),
                "--body",
                &format!("Content {}", i),
                "--no-editor",
            ])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to add note");

        assert!(output.status.success(), "Failed to add note {}", i);
    }

    // Search should be fast
    let search_output = Command::new(&binary_path)
        .args(["search", "Note"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to search");

    assert!(search_output.status.success());
}

// ============================================================================
// User Experience Tests
// ============================================================================

#[test]
fn test_helpful_error_messages() {
    let binary_path = get_binary_path();

    // Command without repo
    let output = Command::new(&binary_path)
        .args(["add", "--title", "Test"])
        .current_dir("/tmp")
        .output()
        .expect("Failed to execute");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should mention init
        assert!(
            stderr.to_lowercase().contains("init") || stderr.to_lowercase().contains("repository")
        );
    }
}

#[test]
fn test_alias_suggestions() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Test that visible aliases work
    let output = Command::new(&binary_path)
        .args(["l"]) // Alias for log
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute alias");

    assert!(output.status.success(), "Alias 'l' should work for 'log'");
}

// ============================================================================
// Complex Workflow Tests
// ============================================================================

#[test]
fn test_complete_dev_workflow() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // 1. Start recording
    let _rec = Command::new(&binary_path)
        .args(["rec", "Feature implementation"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to start rec");

    // 2. Check status
    let status = Command::new(&binary_path)
        .args(["rec", "--status"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to check status");

    assert!(status.status.success());

    // 3. Add some notes
    let _add = Command::new(&binary_path)
        .args([
            "add",
            "--title",
            "Implementation notes",
            "--body",
            "Details",
            "--no-editor",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to add note");

    // 4. Stop recording
    let done = Command::new(&binary_path)
        .args(["done"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to finish");

    assert!(done.status.success());

    // 5. View latest
    let view = Command::new(&binary_path)
        .args(["view", "@latest"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to view");

    assert!(view.status.success());

    // 6. Search
    let search = Command::new(&binary_path)
        .args(["search", "implementation"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to search");

    assert!(search.status.success());
}

// ============================================================================
// Time Expression Tests
// ============================================================================

#[test]
fn test_various_time_expressions() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    let valid_times = vec![
        "1m ago",
        "5m ago",
        "30m ago",
        "1h ago",
        "2h ago",
        "1h 30m ago",
        "2h 15m ago",
        "45m ago",
        "1m",
        "2h",
        "30m", // Without "ago"
    ];

    for time_expr in valid_times {
        let _ = Command::new(&binary_path)
            .args(["rec", "--stop"])
            .current_dir(temp_dir.path())
            .output();

        let output = Command::new(&binary_path)
            .args(["rec", "Test", time_expr])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Should not fail due to time format
            assert!(
                !stderr.to_lowercase().contains("invalid format"),
                "Time expression should be valid: {}",
                time_expr
            );
        }
    }
}

// ============================================================================
// Output Format Tests
// ============================================================================

#[test]
fn test_json_output() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();

    // Add note
    let _add = Command::new(&binary_path)
        .args([
            "add",
            "--title",
            "JSON test",
            "--body",
            "Content",
            "--no-editor",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to add");

    // View as JSON
    let output = Command::new(&binary_path)
        .args(["view", "@latest", "--json"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to view JSON");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_ok(),
        "Output should be valid JSON"
    );
}

// ============================================================================
// Interactive Mode Tests (skipped in CI)
// ============================================================================

#[test]
#[ignore = "Requires interactive input"]
fn test_interactive_add() {
    // This would test the quick mode with user input
    // Skipped in automated tests
}

#[test]
#[ignore = "Requires interactive input"]
fn test_interactive_edit() {
    // This would test the editor mode
    // Skipped in automated tests
}
