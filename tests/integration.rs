use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let binary_path = get_binary_path();
    let output = Command::new(&binary_path)
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fuku"));
    assert!(stdout.contains("Commands:"));
}

fn get_binary_path() -> std::path::PathBuf {
    std::env::current_dir()
        .expect("Failed to get current dir")
        .join("target")
        .join("debug")
        .join("fukura")
}

#[test]
fn test_cli_init() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    let binary_path = get_binary_path();
    let output = Command::new(&binary_path)
        .args(["init", "--no-daemon", "--no-hooks"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!("Init command failed with status: {:?}", output.status);
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    assert!(output.status.success());
    assert!(repo_path.join(".fukura").exists());
}

#[test]
fn test_cli_version() {
    let binary_path = get_binary_path();
    let output = Command::new(&binary_path)
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!("Command failed with status: {:?}", output.status);
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Version output: {}", stdout);
    assert!(stdout.contains("fuku") || stdout.contains("0."));
}

#[test]
fn test_cli_invalid_command() {
    let binary_path = get_binary_path();
    let output = Command::new(&binary_path)
        .arg("invalid-command")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
}

#[test]
fn test_cli_search_without_repo() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    let binary_path = get_binary_path();
    let output = Command::new(&binary_path)
        .args(["search", "test"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute command");

    // Should fail because no repo is initialized
    assert!(!output.status.success());
}

#[test]
fn test_new_features() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    let binary_path = get_binary_path();

    // Initialize repo
    Command::new(&binary_path)
        .args(["init", "--no-daemon", "--no-hooks"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute init");

    // Test add with quick mode
    let output = Command::new(&binary_path)
        .args([
            "add",
            "--title",
            "Test Note",
            "--body",
            "Test content",
            "--no-editor",
        ])
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute add");

    assert!(output.status.success());

    // Test list command
    let output = Command::new(&binary_path)
        .arg("list")
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Note"));

    // Test stats command
    let output = Command::new(&binary_path)
        .arg("stats")
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute stats");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Repository Statistics"));

    // Test config show
    let output = Command::new(&binary_path)
        .args(["config", "show"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute config show");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configuration"));

    // Test @latest reference
    let output = Command::new(&binary_path)
        .args(["view", "@latest"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute view @latest");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Note"));

    // Test edit with tags
    let output = Command::new(&binary_path)
        .args(["edit", "@latest", "--add-tag", "test"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute edit");

    assert!(output.status.success());
}
