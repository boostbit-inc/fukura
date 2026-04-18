//! End-to-end test for `fuku attempt` CLI. Exercises the exact flow a
//! shell hook would execute: observe a failing command, observe a
//! successful follow-up, and verify the effectiveness loop closed.

use std::process::Command;

fn fuku() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fukura"))
}

#[test]
fn shell_hook_style_loop_produces_success_attempt() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();

    // Bootstrap the repo (same as `fuku init`).
    fukura::repo::FukuraRepo::init(repo, true).unwrap();

    // 1. Failing command observed.
    let status = fuku()
        .args([
            "attempt",
            "observe",
            "--session",
            "sess-e2e",
            "--command",
            "cargo build",
            "--exit-code",
            "101",
            "--stderr",
            "error[E0308]: mismatched types",
        ])
        .current_dir(repo)
        .status()
        .unwrap();
    assert!(status.success(), "observe failure exited non-zero");

    // 2. Successful follow-up closes the loop.
    let status = fuku()
        .args([
            "attempt",
            "observe",
            "--session",
            "sess-e2e",
            "--command",
            "cargo check",
            "--exit-code",
            "0",
        ])
        .current_dir(repo)
        .status()
        .unwrap();
    assert!(status.success(), "observe success exited non-zero");

    // 3. Stats reflect exactly one success.
    let output = fuku()
        .args(["attempt", "stats"])
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(output.status.success(), "stats exited non-zero");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("success=1") && stdout.contains("total=1"),
        "unexpected stats output: {stdout}"
    );
    assert!(
        stdout.contains("rate=100.0%"),
        "success rate missing: {stdout}"
    );

    // 4. Pending file should no longer hold the session (it was resolved).
    let pending_path = repo.join(".fukura").join("pending_attempts.json");
    let pending = std::fs::read_to_string(&pending_path).unwrap_or_default();
    assert!(
        !pending.contains("sess-e2e"),
        "session should be cleared from pending: {pending}"
    );
}

#[test]
fn abandon_records_abandoned_outcome_without_success() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    fukura::repo::FukuraRepo::init(repo, true).unwrap();

    // Fail → abandon (simulates shell exit before resolution).
    fuku()
        .args([
            "attempt",
            "observe",
            "--session",
            "sess-abandon",
            "--command",
            "git push",
            "--exit-code",
            "1",
            "--stderr",
            "! [rejected] main -> main (non-fast-forward)",
        ])
        .current_dir(repo)
        .status()
        .unwrap();

    fuku()
        .args(["attempt", "abandon", "--session", "sess-abandon"])
        .current_dir(repo)
        .status()
        .unwrap();

    let output = fuku()
        .args(["attempt", "stats"])
        .current_dir(repo)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("abandoned=1"),
        "abandoned outcome missing: {stdout}"
    );
    assert!(
        stdout.contains("success=0"),
        "success must be zero: {stdout}"
    );
}
