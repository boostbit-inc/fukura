use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "fukura", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fukura"));
    assert!(stdout.contains("Commands:"));
}

#[test]
fn test_cli_init() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Build the binary first
    let build_output = Command::new("cargo")
        .args(["build", "--bin", "fukura"])
        .output()
        .expect("Failed to build binary");

    assert!(build_output.status.success(), "Failed to build binary");

    // Run the binary directly
    let binary_path = std::env::current_dir()
        .expect("Failed to get current dir")
        .join("target")
        .join("debug")
        .join("fukura");

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
    let output = Command::new("cargo")
        .args(["run", "--bin", "fukura", "--", "--version"])
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
    assert!(stdout.contains("fukura") || stdout.contains("fuku") || stdout.contains("0.1.0"));
}

#[test]
fn test_cli_invalid_command() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "fukura", "--", "invalid-command"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
}

#[test]
fn test_cli_search_without_repo() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Build the binary first
    let build_output = Command::new("cargo")
        .args(["build", "--bin", "fukura"])
        .output()
        .expect("Failed to build binary");

    assert!(build_output.status.success(), "Failed to build binary");

    // Run the binary directly
    let binary_path = std::env::current_dir()
        .expect("Failed to get current dir")
        .join("target")
        .join("debug")
        .join("fukura");

    let output = Command::new(&binary_path)
        .args(["search", "test"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute command");

    // Should fail because no repo is initialized
    assert!(!output.status.success());
}
