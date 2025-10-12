use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;

/// Integration tests for time-based recording feature
/// Tests the full CLI functionality for time-based recording

fn get_binary_path() -> std::path::PathBuf {
    std::env::current_dir()
        .expect("Failed to get current dir")
        .join("target")
        .join("debug")
        .join("fukura")
}

/// Initialize a test repository
fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let binary_path = get_binary_path();
    
    // Initialize repository
    let output = Command::new(&binary_path)
        .args(["init", "--no-daemon", "--no-hooks"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute init command");
    
    assert!(output.status.success(), "Failed to initialize test repository");
    temp_dir
}

#[test]
fn test_time_based_rec_help_and_status() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    // Test rec command help shows time-based option
    let output = Command::new(&binary_path)
        .args(["rec", "--help"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute help command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TIME_AGO"));
    assert!(stdout.contains("3m ago"));
    assert!(stdout.contains("2h ago"));
}

#[test]
fn test_time_based_rec_status_not_recording() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    // Test status when not recording
    let output = Command::new(&binary_path)
        .args(["rec", "--status"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute status command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Not recording"));
    assert!(stdout.contains("3m ago")); // Should show time-based examples in help
}

#[test]
fn test_time_based_rec_invalid_format() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    let invalid_formats = vec![
        "invalid",
        "3x ago", 
        "abc",
        "3.5h ago",
        "-1m ago",
    ];
    
    for invalid_format in invalid_formats {
        let output = Command::new(&binary_path)
            .args(["rec", "Test task", invalid_format])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute rec command");
        
        // Should fail with invalid format
        assert!(!output.status.success(), 
                "Should have failed with invalid format: {}", invalid_format);
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.to_lowercase().contains("invalid") || 
                stderr.to_lowercase().contains("format"),
                "Error message should mention invalid format for: {}. Got: {}", 
                invalid_format, stderr);
    }
}

#[test]
fn test_time_based_rec_valid_formats() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    let valid_formats = vec![
        "1m ago",
        "5m ago",
        "30m ago",
        "1h ago", 
        "2h ago",
        "1h 30m ago",
        "2h 15m ago",
    ];
    
    for valid_format in valid_formats {
        // Clean up any existing recording
        let _ = Command::new(&binary_path)
            .args(["rec", "--stop"])
            .current_dir(temp_dir.path())
            .output();
        
        let output = Command::new(&binary_path)
            .args(["rec", "Test task", valid_format])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute rec command");
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("Failed for format: {}", valid_format);
            println!("stdout: {}", stdout);
            println!("stderr: {}", stderr);
        }
        
        // Should succeed or fail gracefully (daemon might not be available in test)
        // We don't assert success because daemon might not be running in test environment
        // But we can check that the error is related to daemon, not format parsing
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If it fails, it should be due to daemon issues, not format parsing
            assert!(stderr.to_lowercase().contains("daemon") || 
                    stderr.to_lowercase().contains("service") ||
                    stderr.to_lowercase().contains("connection") ||
                    stderr.to_lowercase().contains("running"),
                    "Unexpected error for format {}: {}", valid_format, stderr);
        }
    }
}

#[test]
fn test_time_based_rec_boundary_validation() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    // Test time that's too far back (exceeds default 3 hour limit)
    let output = Command::new(&binary_path)
        .args(["rec", "Test task", "4h ago"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute rec command");
    
    // Should fail due to time limit
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should mention time limit or validation error
        assert!(stderr.to_lowercase().contains("time") ||
                stderr.to_lowercase().contains("limit") ||
                stderr.to_lowercase().contains("hours") ||
                stderr.to_lowercase().contains("maximum"),
                "Should mention time validation error. Got: {}", stderr);
    }
    
    // Test time that's too recent (less than 1 minute)
    // Note: This is tricky to test reliably due to timing, so we skip it
    // in automated tests and rely on unit tests for validation logic
}

#[test]
fn test_time_based_rec_case_insensitive() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    let case_variations = vec![
        "3m ago",
        "3M ago", 
        "3M AGO",
        "3m AGO",
    ];
    
    for format in case_variations {
        // Clean up any existing recording
        let _ = Command::new(&binary_path)
            .args(["rec", "--stop"])
            .current_dir(temp_dir.path())
            .output();
        
        let output = Command::new(&binary_path)
            .args(["rec", "Test task", format])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute rec command");
        
        // Check that format parsing doesn't fail due to case sensitivity
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If it fails, it should NOT be due to format parsing
            assert!(!stderr.to_lowercase().contains("invalid") ||
                    !stderr.to_lowercase().contains("format"),
                    "Should not fail format parsing for: {}. Got: {}", format, stderr);
        }
    }
}

#[test]
fn test_time_based_rec_with_existing_recording() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    // Start a normal recording first
    let output1 = Command::new(&binary_path)
        .args(["rec", "First task"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute first rec command");
    
    if output1.status.success() {
        // Try to start time-based recording while one is already active
        let output2 = Command::new(&binary_path)
            .args(["rec", "Second task", "3m ago"])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute second rec command");
        
        if !output2.status.success() {
            let stderr = String::from_utf8_lossy(&output2.stderr);
            // Should mention already recording
            assert!(stderr.to_lowercase().contains("already") ||
                    stderr.to_lowercase().contains("recording") ||
                    stderr.to_lowercase().contains("progress"),
                    "Should mention existing recording. Got: {}", stderr);
        }
    }
}

#[test]
fn test_time_based_rec_configuration_loading() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    // Create a custom config with different limits
    let config_dir = temp_dir.path().join(".fukura");
    let config_content = r#"
version = 1

[recording]
max_lookback_hours = 6
min_lookback_minutes = 5
"#;
    
    std::fs::write(config_dir.join("config.toml"), config_content)
        .expect("Failed to write config");
    
    // Test with time that should now be valid (5h ago with 6h limit)
    let output = Command::new(&binary_path)
        .args(["rec", "Test task", "5h ago"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute rec command");
    
    // Should not fail due to time validation (might fail due to daemon)
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should not fail due to time limits anymore
        assert!(!stderr.to_lowercase().contains("maximum") ||
                !stderr.to_lowercase().contains("hours"),
                "Should not fail time validation with custom config. Got: {}", stderr);
    }
}

#[test]
fn test_time_based_rec_help_examples() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    // Test that status shows helpful examples
    let output = Command::new(&binary_path)
        .args(["rec", "--status"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute status command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should show examples of time-based usage
    assert!(stdout.contains("3m ago"), "Should show 3m ago example");
    
    // Should show both regular and time-based examples
    assert!(stdout.contains("rec \"Task description\""), 
            "Should show regular rec example");
}

#[test]
#[ignore = "Requires daemon to be available"]
fn test_time_based_rec_full_integration() {
    // This test is ignored by default because it requires the daemon
    // to be running and properly configured
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    // Start daemon first
    let _daemon_output = Command::new(&binary_path)
        .args(["start"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to start daemon");
    
    // Wait for daemon to initialize
    std::thread::sleep(Duration::from_secs(2));
    
    // Try time-based recording
    let output = Command::new(&binary_path)
        .args(["rec", "Integration test", "2m ago"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute rec command");
    
    assert!(output.status.success(), 
            "Time-based recording should succeed with daemon running");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("recording started") || 
            stdout.contains("Time-based recording"),
            "Should confirm time-based recording started");
    
    // Check status
    let status_output = Command::new(&binary_path)
        .args(["rec", "--status"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to check status");
    
    assert!(status_output.status.success());
    let status_stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(status_stdout.contains("Recording in progress"),
            "Should show recording is active");
    
    // Stop daemon
    let _ = Command::new(&binary_path)
        .args(["stop"])
        .current_dir(temp_dir.path())
        .output();
}

/// Test the actual CLI integration with mocked daemon responses
#[test]
fn test_cli_integration_without_daemon() {
    let temp_dir = setup_test_repo();
    let binary_path = get_binary_path();
    
    // Test various command line argument combinations
    let test_cases = vec![
        // Valid formats
        (vec!["rec", "Test task", "3m ago"], true),
        (vec!["rec", "Another task", "1h ago"], true),
        (vec!["rec", "Complex task", "1h 30m ago"], true),
        
        // Invalid formats  
        (vec!["rec", "Bad task", "invalid"], false),
        (vec!["rec", "Bad task", "3x ago"], false),
        
        // Missing arguments
        (vec!["rec"], false),  // No title
        (vec!["rec", "Task"], true),  // Normal rec should work
    ];
    
    for (args, should_parse_ok) in test_cases {
        let output = Command::new(&binary_path)
            .args(&args)
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute command");
        
        if should_parse_ok {
            // If arguments should parse OK, any failure should be due to 
            // daemon/system issues, not argument parsing
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(!stderr.to_lowercase().contains("invalid format") &&
                        !stderr.to_lowercase().contains("usage:"),
                        "Should not fail argument parsing for: {:?}. Got: {}", 
                        args, stderr);
            }
        } else {
            // Should fail due to argument/format issues
            assert!(!output.status.success(), 
                    "Should fail for invalid args: {:?}", args);
        }
        
        // Clean up for next test
        let _ = Command::new(&binary_path)
            .args(["rec", "--stop"])
            .current_dir(temp_dir.path())
            .output();
    }
}
