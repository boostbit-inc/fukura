//! End-to-end test for `fuku claude-code register/unregister`.
//! Drives the real binary against a temp HOME so assertions see the
//! exact JSON we hand Claude Code.

use std::process::Command;

fn fuku(home: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fukura"));
    cmd.env("HOME", home);
    cmd
}

#[test]
fn register_writes_expected_config_and_is_idempotent() {
    let home = tempfile::tempdir().unwrap();

    // First register — creates ~/.claude.json from scratch.
    let out = fuku(home.path())
        .args([
            "claude-code",
            "register",
            "--binary",
            "/fake/fukura",
            "--repo",
            "/tmp/demo",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("Registered fukura"),
        "first register output: {stdout}"
    );

    let cfg_path = home.path().join(".claude.json");
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&cfg_path).unwrap()).unwrap();
    assert_eq!(v["mcpServers"]["fukura"]["command"], "/fake/fukura");
    assert_eq!(v["mcpServers"]["fukura"]["args"][0], "mcp");
    assert_eq!(v["mcpServers"]["fukura"]["args"][1], "--repo");
    assert_eq!(v["mcpServers"]["fukura"]["args"][2], "/tmp/demo");

    // Second register — no changes.
    let out = fuku(home.path())
        .args([
            "claude-code",
            "register",
            "--binary",
            "/fake/fukura",
            "--repo",
            "/tmp/demo",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("already registered"),
        "second register should be idempotent: {stdout}"
    );
}

#[test]
fn unregister_removes_only_fukura_preserving_others() {
    let home = tempfile::tempdir().unwrap();

    // Hand-write a config that already has another MCP server.
    let cfg_path = home.path().join(".claude.json");
    std::fs::write(
        &cfg_path,
        r#"{
          "theme": "dark",
          "mcpServers": {
            "other-tool": { "command": "/bin/other", "args": [] }
          }
        }"#,
    )
    .unwrap();

    // Register fukura alongside other-tool.
    fuku(home.path())
        .args(["claude-code", "register", "--binary", "/fake/fukura"])
        .output()
        .unwrap();

    // Unregister.
    let out = fuku(home.path())
        .args(["claude-code", "unregister"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Removed fukura"), "unexpected: {stdout}");

    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&cfg_path).unwrap()).unwrap();
    assert!(v["mcpServers"].get("fukura").is_none());
    assert_eq!(v["mcpServers"]["other-tool"]["command"], "/bin/other");
    assert_eq!(v["theme"], "dark", "top-level keys must survive");
}

#[test]
fn status_reports_registered_or_not() {
    let home = tempfile::tempdir().unwrap();

    // Before registering.
    let out = fuku(home.path())
        .args(["claude-code", "status"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("does not exist"), "{stdout}");

    fuku(home.path())
        .args(["claude-code", "register", "--binary", "/fake/fukura"])
        .output()
        .unwrap();

    let out = fuku(home.path())
        .args(["claude-code", "status"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("status: registered"), "{stdout}");
    assert!(stdout.contains("/fake/fukura"), "entry missing: {stdout}");
}

#[test]
fn dry_run_does_not_write_file() {
    let home = tempfile::tempdir().unwrap();

    fuku(home.path())
        .args([
            "claude-code",
            "register",
            "--binary",
            "/fake/fukura",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(
        !home.path().join(".claude.json").exists(),
        "dry-run must not create the config file"
    );
}
