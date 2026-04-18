//! Installer for fukura as a Claude Code MCP server.
//!
//! Claude Code reads its MCP server list from either
//! `~/.claude.json` (user scope) or `<repo>/.mcp.json` (project
//! scope); both files share the same shape:
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "fukura": {
//!       "command": "/usr/local/bin/fukura",
//!       "args": ["mcp"]
//!     }
//!   }
//! }
//! ```
//!
//! This module owns the merge-patch logic so the CLI does not have to
//! know the file format. We intentionally preserve every unknown key
//! and every other MCP server — users may have hand-edited the file,
//! and destroying that work once would make it impossible to recommend
//! `fuku claude-code register` to anyone again.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};

pub const SERVER_NAME: &str = "fukura";
pub const USER_CONFIG: &str = ".claude.json";
pub const PROJECT_CONFIG: &str = ".mcp.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum Scope {
    /// `~/.claude.json` — applies to every Claude Code session for the
    /// current user.
    User,
    /// `<cwd>/.mcp.json` — applies only when Claude Code is launched
    /// from this project directory. Useful for repos that want to
    /// expose their own fukura instance (with `--repo`) without
    /// touching user-global state.
    Project,
}

#[derive(Debug, Clone)]
pub struct RegisterOptions {
    pub scope: Scope,
    /// Absolute path to the fukura binary that Claude Code should
    /// launch. Defaults to the currently-running executable, which is
    /// almost always what the user wants (they just typed `fuku` from
    /// the same install).
    pub binary: PathBuf,
    /// Optional `--repo` to pass through to `fukura mcp`, pinning the
    /// repository the server serves.
    pub repo: Option<PathBuf>,
    /// When true, compute the change but do not write the file. Used by
    /// the CLI's `--dry-run` flag and the tests.
    pub dry_run: bool,
    /// Explicit override of the config file path. When `Some`, the
    /// scope-based `resolve_config_path` is bypassed and the register
    /// writes here instead. Intended for tests and for users who
    /// keep Claude Code config in a non-default location (e.g. a
    /// checked-in project template).
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterOutcome {
    /// The config did not contain a fukura entry; one was added.
    Added { path: PathBuf },
    /// A fukura entry was already present with the same command/args.
    AlreadyPresent { path: PathBuf },
    /// A fukura entry existed but had different command/args; it was
    /// overwritten with the new values.
    Updated { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnregisterOutcome {
    Removed { path: PathBuf },
    NotPresent { path: PathBuf },
}

pub fn register(opts: &RegisterOptions) -> Result<RegisterOutcome> {
    let path = match &opts.config_path {
        Some(p) => p.clone(),
        None => resolve_config_path(opts.scope)?,
    };
    let mut root = load_or_empty_object(&path)?;
    ensure_object(&mut root);

    let desired = desired_server_entry(opts);

    let servers = root
        .get_mut("mcpServers")
        .and_then(Value::as_object_mut)
        .expect("ensure_object guarantees mcpServers object");

    let outcome = match servers.get(SERVER_NAME) {
        Some(existing) if existing == &desired => {
            RegisterOutcome::AlreadyPresent { path: path.clone() }
        }
        Some(_) => RegisterOutcome::Updated { path: path.clone() },
        None => RegisterOutcome::Added { path: path.clone() },
    };

    servers.insert(SERVER_NAME.to_owned(), desired);

    if !opts.dry_run {
        write_atomically(&path, &root)?;
    }
    Ok(outcome)
}

pub fn unregister(scope: Scope, dry_run: bool) -> Result<UnregisterOutcome> {
    unregister_at(scope, dry_run, None)
}

pub fn unregister_at(
    scope: Scope,
    dry_run: bool,
    config_path: Option<PathBuf>,
) -> Result<UnregisterOutcome> {
    let path = match config_path {
        Some(p) => p,
        None => resolve_config_path(scope)?,
    };
    if !path.exists() {
        return Ok(UnregisterOutcome::NotPresent { path });
    }
    let mut root = load_or_empty_object(&path)?;

    let removed = root
        .get_mut("mcpServers")
        .and_then(Value::as_object_mut)
        .and_then(|m| m.remove(SERVER_NAME))
        .is_some();

    let outcome = if removed {
        UnregisterOutcome::Removed { path: path.clone() }
    } else {
        UnregisterOutcome::NotPresent { path: path.clone() }
    };

    if removed && !dry_run {
        write_atomically(&path, &root)?;
    }
    Ok(outcome)
}

fn resolve_config_path(scope: Scope) -> Result<PathBuf> {
    match scope {
        Scope::User => {
            let home = std::env::var_os("HOME").context("HOME environment variable is not set")?;
            Ok(PathBuf::from(home).join(USER_CONFIG))
        }
        Scope::Project => {
            let cwd = std::env::current_dir().context("current directory unreadable")?;
            Ok(cwd.join(PROJECT_CONFIG))
        }
    }
}

fn load_or_empty_object(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if bytes.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(json!({}));
    }
    serde_json::from_slice(&bytes).with_context(|| format!("{} is not valid JSON", path.display()))
}

fn ensure_object(root: &mut Value) {
    if !root.is_object() {
        *root = json!({});
    }
    let obj = root
        .as_object_mut()
        .expect("just ensured root is an object");
    if !obj.contains_key("mcpServers") {
        obj.insert("mcpServers".into(), json!({}));
    }
    // If `mcpServers` exists but is the wrong type, overwrite it rather
    // than keep broken config around.
    if !obj["mcpServers"].is_object() {
        obj.insert("mcpServers".into(), json!({}));
    }
}

fn desired_server_entry(opts: &RegisterOptions) -> Value {
    let mut args = vec!["mcp".to_owned()];
    if let Some(repo) = &opts.repo {
        args.push("--repo".to_owned());
        args.push(repo.display().to_string());
    }
    json!({
        "command": opts.binary.display().to_string(),
        "args": args,
    })
}

fn write_atomically(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let tmp = path.with_extension("json.fukura-tmp");
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("renaming into {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts_for(binary: &Path, path: &Path) -> RegisterOptions {
        // The tests drive the merge logic directly (without touching
        // HOME or cwd) by bypassing `resolve_config_path` — we invoke
        // register_at() in the helper below.
        let _ = path;
        RegisterOptions {
            scope: Scope::User,
            binary: binary.to_path_buf(),
            repo: None,
            dry_run: false,
            config_path: None,
        }
    }

    // Internal test helper: perform register() against an arbitrary
    // path, skipping the HOME-based resolution so tests don't need to
    // mutate process env (which is racy across test threads).
    fn register_at(path: &Path, opts: &RegisterOptions) -> Result<RegisterOutcome> {
        let mut root = load_or_empty_object(path)?;
        ensure_object(&mut root);
        let desired = desired_server_entry(opts);
        let servers = root
            .get_mut("mcpServers")
            .and_then(Value::as_object_mut)
            .unwrap();
        let outcome = match servers.get(SERVER_NAME) {
            Some(existing) if existing == &desired => RegisterOutcome::AlreadyPresent {
                path: path.to_path_buf(),
            },
            Some(_) => RegisterOutcome::Updated {
                path: path.to_path_buf(),
            },
            None => RegisterOutcome::Added {
                path: path.to_path_buf(),
            },
        };
        servers.insert(SERVER_NAME.to_owned(), desired);
        if !opts.dry_run {
            write_atomically(path, &root)?;
        }
        Ok(outcome)
    }

    fn unregister_at(path: &Path) -> Result<UnregisterOutcome> {
        if !path.exists() {
            return Ok(UnregisterOutcome::NotPresent {
                path: path.to_path_buf(),
            });
        }
        let mut root = load_or_empty_object(path)?;
        let removed = root
            .get_mut("mcpServers")
            .and_then(Value::as_object_mut)
            .and_then(|m| m.remove(SERVER_NAME))
            .is_some();
        if removed {
            write_atomically(path, &root)?;
            Ok(UnregisterOutcome::Removed {
                path: path.to_path_buf(),
            })
        } else {
            Ok(UnregisterOutcome::NotPresent {
                path: path.to_path_buf(),
            })
        }
    }

    #[test]
    fn creates_config_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude.json");
        let opts = opts_for(Path::new("/usr/local/bin/fukura"), &path);

        let out = register_at(&path, &opts).unwrap();
        assert_eq!(out, RegisterOutcome::Added { path: path.clone() });

        let v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            v["mcpServers"]["fukura"]["command"],
            "/usr/local/bin/fukura"
        );
        assert_eq!(v["mcpServers"]["fukura"]["args"][0], "mcp");
    }

    #[test]
    fn is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude.json");
        let opts = opts_for(Path::new("/usr/local/bin/fukura"), &path);

        let first = register_at(&path, &opts).unwrap();
        assert!(matches!(first, RegisterOutcome::Added { .. }));
        let second = register_at(&path, &opts).unwrap();
        assert!(matches!(second, RegisterOutcome::AlreadyPresent { .. }));
    }

    #[test]
    fn preserves_other_mcp_servers_and_top_level_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude.json");
        std::fs::write(
            &path,
            r#"{
              "theme": "dark",
              "mcpServers": {
                "other-tool": { "command": "/bin/other", "args": [] }
              }
            }"#,
        )
        .unwrap();

        let opts = opts_for(Path::new("/bin/fukura"), &path);
        register_at(&path, &opts).unwrap();

        let v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["mcpServers"]["other-tool"]["command"], "/bin/other");
        assert_eq!(v["mcpServers"]["fukura"]["command"], "/bin/fukura");
    }

    #[test]
    fn repo_arg_is_passed_through() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude.json");
        let mut opts = opts_for(Path::new("/bin/fukura"), &path);
        opts.repo = Some(PathBuf::from("/home/me/project"));

        register_at(&path, &opts).unwrap();
        let v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let args = v["mcpServers"]["fukura"]["args"].as_array().unwrap();
        assert_eq!(args[0], "mcp");
        assert_eq!(args[1], "--repo");
        assert_eq!(args[2], "/home/me/project");
    }

    #[test]
    fn different_command_triggers_update_not_add() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude.json");
        register_at(&path, &opts_for(Path::new("/old/path/fukura"), &path)).unwrap();
        let out = register_at(&path, &opts_for(Path::new("/new/path/fukura"), &path)).unwrap();
        assert!(matches!(out, RegisterOutcome::Updated { .. }));

        let v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(v["mcpServers"]["fukura"]["command"], "/new/path/fukura");
    }

    #[test]
    fn unregister_removes_only_fukura() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude.json");
        std::fs::write(
            &path,
            r#"{
              "mcpServers": {
                "other-tool": { "command": "/bin/other", "args": [] },
                "fukura": { "command": "/bin/fukura", "args": ["mcp"] }
              }
            }"#,
        )
        .unwrap();

        let out = unregister_at(&path).unwrap();
        assert!(matches!(out, UnregisterOutcome::Removed { .. }));

        let v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(v["mcpServers"].get("fukura").is_none());
        assert_eq!(v["mcpServers"]["other-tool"]["command"], "/bin/other");
    }

    #[test]
    fn unregister_is_no_op_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude.json");
        let out = unregister_at(&path).unwrap();
        assert!(matches!(out, UnregisterOutcome::NotPresent { .. }));
    }

    #[test]
    fn non_object_mcp_servers_is_replaced_not_crashed() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude.json");
        std::fs::write(&path, r#"{ "mcpServers": "not an object" }"#).unwrap();

        let opts = opts_for(Path::new("/bin/fukura"), &path);
        register_at(&path, &opts).unwrap();
        let v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(v["mcpServers"]["fukura"]["command"], "/bin/fukura");
    }
}
