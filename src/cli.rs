use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::{bail, ensure, Context, Result};
use axum::extract::{Path as AxumPath, Query as AxumQuery, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{Duration, Utc};
use clap::{ArgAction, Args, Parser, Subcommand};
use colored::Colorize;
use comfy_table::{presets::UTF8_HORIZONTAL_ONLY, Table};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, queue};
use dialoguer::{theme::ColorfulTheme, Editor, Input};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use serde::Deserialize;
use tokio::net::TcpListener;

use crate::config_cmd::{update_redaction, update_remote};
use crate::index::{SearchHit, SearchIndex, SearchSort};
use crate::models::{Author, Note, NoteRecord, Privacy};
use crate::repo::FukuraRepo;
use crate::sync::{pull_note, push_note};

#[derive(Debug, Parser)]
#[command(
    name = "fuku",
    version,
    about = "Curate your team's hard-earned error wisdom."
)]
pub struct Cli {
    #[arg(
        long = "repo",
        global = true,
        value_name = "PATH",
        help = "Path to the repository root (defaults to CWD)"
    )]
    repo: Option<PathBuf>,

    #[arg(long, global = true, action = ArgAction::SetTrue, help = "Suppress celebratory output")]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a new repository
    Init(InitCommand),

    /// Add a new note
    Add(AddCommand),

    /// Search notes
    Search(SearchCommand),

    /// View a note
    View(ViewCommand),

    /// Open note in browser
    Open(OpenCommand),

    /// Start local web server
    Serve(ServeCommand),

    /// Optimize storage (garbage collection)
    Gc(GcCommand),

    /// Push notes to remote
    Push(PushCommand),

    /// Pull notes from remote
    Pull(PullCommand),

    /// Sync notes with remote
    Sync(SyncCommand),

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Manage background daemon
    Daemon(DaemonCommand),
}

#[derive(Debug, Args)]
pub struct InitCommand {
    #[arg(
        value_name = "PATH",
        help = "Directory to initialize (default: current directory)",
        default_value = "."
    )]
    path: PathBuf,

    #[arg(long, help = "Reinitialize existing repository")]
    force: bool,

    #[arg(long, help = "Skip daemon setup")]
    no_daemon: bool,

    #[arg(long, help = "Skip shell hooks")]
    no_hooks: bool,
}

#[derive(Debug, Args)]
pub struct AddCommand {
    #[arg(long, value_name = "TEXT", help = "Note title")]
    title: Option<String>,

    #[arg(long, value_name = "TEXT", help = "Note content")]
    body: Option<String>,

    #[arg(long, value_name = "PATH", help = "Read from file")]
    file: Option<PathBuf>,

    #[arg(long, help = "Read from stdin")]
    stdin: bool,

    #[arg(
        long = "tag",
        value_name = "TAG",
        action = ArgAction::Append,
        help = "Add tag (can be used multiple times)"
    )]
    tags: Vec<String>,

    #[arg(
        long = "meta",
        value_name = "KEY=VALUE",
        action = ArgAction::Append,
        help = "Add metadata (can be used multiple times)"
    )]
    meta: Vec<String>,

    #[arg(
        long = "link",
        value_name = "URL",
        action = ArgAction::Append,
        help = "Add link (can be used multiple times)"
    )]
    links: Vec<String>,

    #[arg(value_enum, long, help = "Privacy level (private/org/public)", default_value_t = Privacy::Private)]
    privacy: Privacy,

    #[arg(long, value_name = "NAME", help = "Author name")]
    author: Option<String>,

    #[arg(long, value_name = "EMAIL", help = "Author email")]
    email: Option<String>,

    #[arg(long, help = "Skip editor")]
    no_editor: bool,
}

#[derive(Debug, Args)]
pub struct SearchCommand {
    #[arg(long, short = 'n', default_value_t = 20, help = "Max results")]
    limit: usize,

    #[arg(value_enum, long, short = 's', default_value_t = SearchSort::Relevance, help = "Sort by (relevance/updated/likes)")]
    sort: SearchSort,

    #[arg(long, help = "Output as JSON")]
    json: bool,

    #[arg(long, help = "Interactive TUI mode")]
    tui: bool,

    #[arg(long, short = 'a', help = "Search all repositories")]
    all_repos: bool,

    #[arg(value_name = "QUERY", help = "Search terms", trailing_var_arg = true)]
    query: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ViewCommand {
    #[arg(value_name = "ID", help = "Note ID or @latest/@1")]
    id: String,

    #[arg(long, help = "Output as JSON")]
    json: bool,
}

#[derive(Debug, Args)]
pub struct OpenCommand {
    #[arg(value_name = "ID", help = "Note ID or @latest/@1")]
    id: String,

    #[arg(
        long,
        value_name = "THEME",
        help = "Theme (light/dark)",
        default_value = "dark"
    )]
    theme: String,

    #[arg(long, help = "Open in browser directly")]
    browser_only: bool,

    #[arg(long, help = "Show URL only")]
    url_only: bool,

    #[arg(
        long,
        value_name = "PORT",
        help = "Local server port",
        default_value = "8080"
    )]
    server_port: Option<u16>,
}

#[derive(Debug, Args)]
pub struct ServeCommand {
    #[arg(
        long,
        value_name = "HOST:PORT",
        default_value = "127.0.0.1:8765",
        help = "Server address"
    )]
    addr: String,

    #[arg(long, default_value_t = 50, help = "Page size")]
    page_size: usize,
}

#[derive(Debug, Args)]
pub struct GcCommand {
    #[arg(long, help = "Remove loose objects")]
    prune: bool,
}

#[derive(Debug, Args)]
pub struct PushCommand {
    #[arg(value_name = "ID", help = "Note ID")]
    id: String,

    #[arg(long, value_name = "URL", help = "Remote URL")]
    remote: Option<String>,
}

#[derive(Debug, Args)]
pub struct PullCommand {
    #[arg(value_name = "ID", help = "Note ID")]
    id: String,

    #[arg(long, value_name = "URL", help = "Remote URL")]
    remote: Option<String>,
}

#[derive(Debug, Args)]
pub struct SyncCommand {
    #[arg(value_name = "ID", help = "Note ID (optional with --all)")]
    id: Option<String>,

    #[arg(long, value_name = "URL", help = "Remote URL")]
    remote: Option<String>,

    #[arg(long, help = "Sync all notes")]
    all: bool,

    #[arg(long, help = "Enable auto-sync")]
    enable_auto: bool,

    #[arg(long, help = "Disable auto-sync")]
    disable_auto: bool,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Configure remote URL
    Remote(RemoteCommand),
    /// Manage redaction rules
    Redact(RedactCommand),
}

#[derive(Debug, Args)]
pub struct RemoteCommand {
    #[arg(long, value_name = "URL", help = "Set remote URL")]
    set: Option<String>,

    #[arg(long, help = "Clear remote URL")]
    clear: bool,

    #[arg(long, help = "Apply globally")]
    global: bool,
}

#[derive(Debug, Args)]
pub struct RedactCommand {
    #[arg(
        long = "set",
        value_name = "NAME=REGEX",
        action = ArgAction::Append,
        help = "Add redaction rule"
    )]
    set: Vec<String>,

    #[arg(
        long = "unset",
        value_name = "NAME",
        action = ArgAction::Append,
        help = "Remove redaction rule"
    )]
    unset: Vec<String>,
}

#[derive(Debug, Args)]
pub struct DaemonCommand {
    #[arg(long, help = "Show daemon status and information")]
    status: bool,

    #[arg(long, help = "Stop the daemon")]
    stop: bool,

    #[arg(long, help = "Run daemon in foreground (for debugging)")]
    foreground: bool,

    #[arg(long, help = "Install shell hooks")]
    install_hooks: bool,

    #[arg(long, help = "Uninstall shell hooks")]
    uninstall_hooks: bool,

    #[arg(long, help = "Check shell hooks status")]
    hooks_status: bool,

    #[arg(long, help = "Enable error notifications")]
    notifications_enable: bool,

    #[arg(long, help = "Disable error notifications")]
    notifications_disable: bool,

    #[arg(long, help = "Check notification status")]
    notifications_status: bool,

    #[arg(long, help = "Test notifications (send test notification)")]
    test_notification: bool,

    #[arg(long, hide = true)]
    background: bool,

    #[arg(long, hide = true)]
    record_command: Option<String>,

    #[arg(long, hide = true)]
    record_error: Option<String>,

    #[arg(long, hide = true)]
    check_solutions: bool,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Init(cmd) => handle_init(&cli, cmd)?,
        Commands::Add(cmd) => handle_add(&cli, cmd).await?,
        Commands::Search(cmd) => handle_search(&cli, cmd)?,
        Commands::View(cmd) => handle_view(&cli, cmd)?,
        Commands::Open(cmd) => handle_open(&cli, cmd)?,
        Commands::Serve(cmd) => handle_serve(&cli, cmd).await?,
        Commands::Gc(cmd) => handle_gc(&cli, cmd)?,
        Commands::Push(cmd) => handle_push(&cli, cmd).await?,
        Commands::Pull(cmd) => handle_pull(&cli, cmd).await?,
        Commands::Sync(cmd) => handle_sync(&cli, cmd).await?,
        Commands::Config { command } => handle_config(&cli, command)?,
        Commands::Daemon(cmd) => handle_daemon(&cli, cmd).await?,
    }
    Ok(())
}

fn handle_init(cli: &Cli, cmd: &InitCommand) -> Result<()> {
    let path = if cmd.path == Path::new(".") {
        std::env::current_dir()?
    } else {
        cmd.path.clone()
    };
    let repo = FukuraRepo::init(&path, cmd.force)?;

    if !cli.quiet {
        println!(
            "{} Initialized Fukura vault at {}",
            "".bold().cyan(),
            repo.root().display()
        );
        println!();
    }

    // Interactive setup for daemon (unless --no-daemon is specified)
    let start_daemon = if cmd.no_daemon {
        false
    } else if cli.quiet {
        true // Default to true in quiet mode
    } else {
        // Ask user interactively
        println!(
            "{} Fukura can automatically capture errors and solutions in the background.",
            "".cyan()
        );
        println!(
            "{} This helps build your knowledge base without manual effort.",
            "  ".dimmed()
        );
        dialoguer::Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Enable automatic error capture daemon?")
            .default(true)
            .interact()?
    };

    if start_daemon {
        if !cli.quiet {
            println!("{} Starting automatic error capture daemon...", "".green());
        }

        // Start daemon in background
        crate::daemon_service::start_background_daemon(&repo)?;

        // Save daemon preference
        let mut config = repo.config()?;
        config.daemon_enabled = Some(true);
        config.save(&repo.config_path())?;

        if !cli.quiet {
            println!("{} Automatic error capture is now active!", "".blue());
        }
    } else if !cli.quiet {
        println!(
            "{} Daemon disabled. Use 'fuku daemon' to start it later.",
            "".blue()
        );
    }

    // Install shell hooks automatically
    if !cmd.no_hooks {
        let hook_manager = crate::hooks::HookManager::new(repo.root());
        if let Err(e) = hook_manager.install_hooks() {
            if !cli.quiet {
                println!(
                    "{} Warning: Could not install shell hooks: {}",
                    "".yellow(),
                    e
                );
            }
        } else if !cli.quiet {
            println!("{} Shell hooks installed successfully", "".green());
        }
    }

    if !cli.quiet {
        println!();
        println!("{} Quick Start Guide:", "".cyan());
        println!("  • Add a note:        fuku add --title 'Error solution'");
        println!("  • Search notes:      fuku search 'keyword'");
        println!("  • Check daemon:      fuku daemon --status");
        println!("  • Sync to remote:    fuku sync --enable-auto");
    }

    Ok(())
}

async fn handle_add(cli: &Cli, cmd: &AddCommand) -> Result<()> {
    let repo = open_repo(cli)?;
    let now = chrono::Utc::now();
    let title = match &cmd.title {
        Some(t) => t.clone(),
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Title")
            .interact_text()?,
    };

    let mut body = if let Some(explicit) = &cmd.body {
        explicit.clone()
    } else if let Some(path) = &cmd.file {
        fs::read_to_string(path)
            .with_context(|| format!("Failed to read body from {}", path.display()))?
    } else if cmd.stdin || !is_terminal::is_terminal(std::io::stdin()) {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else if cmd.no_editor {
        // Interactive inline input without external editor
        get_interactive_body()?
    } else {
        String::new()
    };

    if body.trim().is_empty() && !cmd.no_editor {
        // Try interactive input first, fallback to editor if needed
        if let Ok(interactive_body) = get_interactive_body() {
            if !interactive_body.trim().is_empty() {
                body = interactive_body;
            } else if let Some(buffer) =
                Editor::new().edit("# jot down the diagnosis, commands, or code snippets here\n")?
            {
                body = buffer;
            }
        } else if let Some(buffer) =
            Editor::new().edit("# jot down the diagnosis, commands, or code snippets here\n")?
        {
            body = buffer;
        }
    }

    if body.trim().is_empty() {
        bail!("Note body cannot be empty");
    }

    let tags = normalize_tags(cmd.tags.clone());
    let meta = parse_meta(cmd.meta.clone())?;
    let author = resolve_author(cmd.author.as_deref(), cmd.email.as_deref());

    let note = Note {
        title: title.trim().to_string(),
        body: body.trim().to_string(),
        tags,
        links: cmd.links.clone(),
        meta,
        solutions: vec![],
        privacy: cmd.privacy.clone(),
        created_at: now,
        updated_at: now,
        author,
    };

    let record = repo.store_note(note)?;

    if !cli.quiet {
        println!(
            "{} Captured {} ({})",
            "".green(),
            record.note.title.bold(),
            record.object_id
        );
        if !record.note.tags.is_empty() {
            println!("{}  #{}", "".dimmed(), record.note.tags.join(" #"));
        }
        if let Some(latest) = repo.latest()? {
            println!("{}  Latest note → {}", "".dimmed(), latest);
        }
    }

    // Auto-sync if enabled
    let config = repo.config()?;
    if config.auto_sync.unwrap_or(false) {
        if let Some(remote) = &config.default_remote {
            if !cli.quiet {
                println!("{} Auto-syncing to remote...", "".blue());
            }
            match push_note(&repo, &record.object_id, remote).await {
                Ok(remote_id) => {
                    if !cli.quiet {
                        println!("{} Auto-synced → {}", "".green(), remote_id);
                    }
                }
                Err(e) => {
                    if !cli.quiet {
                        println!("{} Auto-sync failed: {}", "".yellow(), e);
                        println!(
                            "{} Use 'fuku sync {}' to retry",
                            "".cyan(),
                            record.object_id
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_search(cli: &Cli, cmd: &SearchCommand) -> Result<()> {
    let query = if cmd.query.is_empty() {
        String::new()
    } else {
        cmd.query.join(" ")
    };

    if cmd.all_repos {
        // Search across all local Fukura repositories
        return search_all_repos(cli, &query, cmd.limit, cmd.sort, cmd.json);
    }

    let repo = open_repo(cli)?;
    if cmd.tui {
        run_search_tui(&repo, &query, cmd.sort, cmd.limit)?;
        return Ok(());
    }
    let hits = repo.search(&query, cmd.limit, cmd.sort)?;
    if cmd.json {
        let json = serde_json::to_string_pretty(&hits)?;
        println!("{}", json);
        return Ok(());
    }
    render_search_table(&hits);
    if let Some(first) = hits.first() {
        let short_id = &first.object_id[..8.min(first.object_id.len())];
        println!(
            "{} View: fuku view {} · Open: fuku open {}",
            "Hint".bold().dimmed(),
            short_id,
            short_id
        );
    }
    Ok(())
}

fn search_all_repos(
    _cli: &Cli,
    query: &str,
    limit: usize,
    sort: SearchSort,
    json: bool,
) -> Result<()> {
    use std::collections::HashMap;

    let home = std::env::var("HOME").context("HOME not set")?;
    let home_path = std::path::PathBuf::from(&home);

    let mut all_hits: Vec<SearchHit> = Vec::new();
    let mut repo_map: HashMap<String, String> = HashMap::new();

    // Search in common directories
    let search_dirs = vec![
        home_path.join("work"),
        home_path.join("projects"),
        home_path.join("dev"),
        home_path.join("src"),
        home_path,
    ];

    for base_dir in search_dirs {
        if !base_dir.exists() {
            continue;
        }
        find_and_search_repos(&base_dir, query, limit, sort, &mut all_hits, &mut repo_map)?;
    }

    // Sort by relevance/date
    match sort {
        SearchSort::Relevance => all_hits.sort_by(|a, b| b.likes.cmp(&a.likes)),
        SearchSort::Updated => all_hits.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        SearchSort::Likes => all_hits.sort_by(|a, b| b.likes.cmp(&a.likes)),
    }

    all_hits.truncate(limit);

    if json {
        let json = serde_json::to_string_pretty(&all_hits)?;
        println!("{}", json);
        return Ok(());
    }

    if all_hits.is_empty() {
        println!("No notes found across all repositories.");
        return Ok(());
    }

    println!("Search Results (across {} repositories)", repo_map.len());
    render_search_table(&all_hits);

    if let Some(first) = all_hits.first() {
        if let Some(repo_path) = repo_map.get(&first.object_id) {
            let short_id = &first.object_id[..8.min(first.object_id.len())];
            println!(
                "{} View: fuku view {} --repo {}",
                "Hint".bold().dimmed(),
                short_id,
                repo_path
            );
        }
    }

    Ok(())
}

fn find_and_search_repos(
    dir: &std::path::Path,
    query: &str,
    limit: usize,
    sort: SearchSort,
    all_hits: &mut Vec<SearchHit>,
    repo_map: &mut std::collections::HashMap<String, String>,
) -> Result<()> {
    // Limit recursion depth
    const MAX_DEPTH: usize = 3;
    find_and_search_repos_recursive(dir, query, limit, sort, all_hits, repo_map, 0, MAX_DEPTH)
}

#[allow(clippy::too_many_arguments)]
fn find_and_search_repos_recursive(
    dir: &std::path::Path,
    query: &str,
    limit: usize,
    sort: SearchSort,
    all_hits: &mut Vec<SearchHit>,
    repo_map: &mut std::collections::HashMap<String, String>,
    depth: usize,
    max_depth: usize,
) -> Result<()> {
    use std::fs;

    if depth > max_depth {
        return Ok(());
    }

    // Check if this directory has .fukura
    let fukura_dir = dir.join(".fukura");
    if fukura_dir.exists() && fukura_dir.is_dir() {
        // Found a Fukura repository
        if let Ok(repo) = FukuraRepo::open(dir) {
            if let Ok(hits) = repo.search(query, limit, sort) {
                for hit in hits {
                    repo_map.insert(hit.object_id.clone(), dir.display().to_string());
                    all_hits.push(hit);
                }
            }
        }
        return Ok(()); // Don't recurse into subdirectories of a repo
    }

    // Recurse into subdirectories
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let path = entry.path();
                    // Skip hidden directories and common exclude patterns
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with('.')
                            || name_str == "node_modules"
                            || name_str == "target"
                        {
                            continue;
                        }
                    }
                    let _ = find_and_search_repos_recursive(
                        &path,
                        query,
                        limit,
                        sort,
                        all_hits,
                        repo_map,
                        depth + 1,
                        max_depth,
                    );
                }
            }
        }
    }

    Ok(())
}

fn handle_view(cli: &Cli, cmd: &ViewCommand) -> Result<()> {
    let repo = open_repo(cli)?;
    let resolved = repo.resolve_object_id(&cmd.id)?;
    let record = repo.load_note(&resolved)?;
    if cmd.json {
        let json = serde_json::to_string_pretty(&record)?;
        println!("{}", json);
    } else {
        render_note(&record);
    }
    Ok(())
}

fn handle_open(cli: &Cli, cmd: &OpenCommand) -> Result<()> {
    let repo = open_repo(cli)?;
    let resolved = repo.resolve_object_id(&cmd.id)?;
    let record = repo.load_note(&resolved)?;
    let theme = cmd.theme.to_lowercase();
    let html = render_note_html(&record, &theme)?;
    let filename = format!("fuku-{}.html", resolved);

    if cmd.url_only {
        // Just show the URL for manual opening
        let file_path = std::env::temp_dir().join(&filename);
        fs::write(&file_path, html)?;

        if !cli.quiet {
            println!("{} Note saved to: {}", "".blue(), file_path.display());
            println!("{} Open this file in your browser manually", "".yellow());
        }
        return Ok(());
    }

    if cmd.browser_only {
        // Try direct browser opening only
        let file_path = std::env::temp_dir().join(&filename);
        fs::write(&file_path, html)?;

        match crate::browser::BrowserOpener::open(&file_path) {
            Ok(()) => {
                if !cli.quiet {
                    println!("{} Opened note in your browser", "".magenta());
                }
            }
            Err(e) => {
                if !cli.quiet {
                    println!("{} Could not open browser: {}", "".red(), e);
                    println!("{} File saved to: {}", "".blue(), file_path.display());
                }
                return Err(e);
            }
        }
    } else {
        // Use smart opening with server fallback
        match crate::browser::BrowserOpener::open_with_server(&html, &filename) {
            Ok(()) => {
                if !cli.quiet {
                    println!("{} Opened note in your browser", "".magenta());
                }
            }
            Err(e) => {
                // Fallback: save to file and show path
                let file_path = std::env::temp_dir().join(&filename);
                fs::write(&file_path, html)?;

                if !cli.quiet {
                    println!(
                        "{} Could not open browser automatically: {}",
                        "".yellow(),
                        e
                    );
                    println!(
                        "{} Please open this file manually: {}",
                        "".blue(),
                        file_path.display()
                    );
                }
            }
        }
    }

    Ok(())
}

fn handle_gc(cli: &Cli, cmd: &GcCommand) -> Result<()> {
    let repo = open_repo(cli)?;
    let report = repo.pack_loose_objects(cmd.prune)?;
    if !cli.quiet {
        println!(
            "{} Packed {} objects into {}",
            "".blue(),
            report.object_count,
            report.pack_file.display()
        );
        if cmd.prune {
            println!("{} Pruned {} loose objects", "".dimmed(), report.pruned);
        }
    }
    Ok(())
}

async fn handle_push(cli: &Cli, cmd: &PushCommand) -> Result<()> {
    let repo = open_repo(cli)?;
    let resolved = repo.resolve_object_id(&cmd.id)?;
    let remote = determine_remote(&repo, cmd.remote.as_deref())?;
    let remote_id = push_note(&repo, &resolved, &remote).await?;
    if !cli.quiet {
        println!("{} Pushed {} → {}", "".green(), resolved, remote_id);
    }
    Ok(())
}

async fn handle_pull(cli: &Cli, cmd: &PullCommand) -> Result<()> {
    let repo = open_repo(cli)?;
    let remote = determine_remote(&repo, cmd.remote.as_deref())?;
    let remote_id = repo
        .resolve_object_id(&cmd.id)
        .unwrap_or_else(|_| cmd.id.clone());
    let local_id = pull_note(&repo, &remote_id, &remote).await?;
    if !cli.quiet {
        println!("{} Pulled {} → {}", "".cyan(), remote_id, local_id);
    }
    Ok(())
}

async fn handle_sync(cli: &Cli, cmd: &SyncCommand) -> Result<()> {
    let repo = open_repo(cli)?;
    let mut config = repo.config()?;

    // Handle auto-sync enable/disable
    if cmd.enable_auto {
        config.auto_sync = Some(true);
        config.save(&repo.config_path())?;
        if !cli.quiet {
            println!("{} Auto-sync enabled", "".green());
            println!(
                "{} Notes will automatically sync to remote after creation",
                "".cyan()
            );
        }
        return Ok(());
    }

    if cmd.disable_auto {
        config.auto_sync = Some(false);
        config.save(&repo.config_path())?;
        if !cli.quiet {
            println!("{} Auto-sync disabled", "".yellow());
        }
        return Ok(());
    }

    // Determine remote
    let remote = determine_remote(&repo, cmd.remote.as_deref())?;

    // Sync single note
    if let Some(id) = &cmd.id {
        let resolved = repo.resolve_object_id(id)?;
        let remote_id = push_note(&repo, &resolved, &remote).await?;
        if !cli.quiet {
            println!("{} Synced {} → {}", "".green(), resolved, remote_id);
        }
        return Ok(());
    }

    // Sync all private notes
    if cmd.all {
        if !cli.quiet {
            println!("{} Syncing all private notes...", "".blue());
        }

        // Get all notes and filter private ones
        let all_notes = repo.list_all_notes()?;
        let mut synced_count = 0;

        for note_record in all_notes {
            if note_record.note.privacy == Privacy::Private {
                match push_note(&repo, &note_record.object_id, &remote).await {
                    Ok(_) => {
                        synced_count += 1;
                        if !cli.quiet {
                            println!("{} Synced: {}", "  [OK]".green(), note_record.note.title);
                        }
                    }
                    Err(e) => {
                        if !cli.quiet {
                            println!(
                                "{} Failed to sync {}: {}",
                                "  [FAIL]".red(),
                                note_record.note.title,
                                e
                            );
                        }
                    }
                }
            }
        }

        if !cli.quiet {
            println!("{} Synced {} notes", "".green(), synced_count);
        }
        return Ok(());
    }

    // No ID and no --all flag
    bail!("Please specify a note ID or use --all to sync all notes");
}

fn handle_config(cli: &Cli, cmd: &ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Remote(remote) => {
            ensure!(
                !(remote.clear && remote.set.is_some()),
                "Use either --set or --clear, not both"
            );

            if remote.global {
                // Handle global config
                let config_path = crate::config::FukuraConfig::global_config_path()?;
                let mut config = crate::config::FukuraConfig::load(&config_path)?;

                if remote.clear {
                    config.default_remote = None;
                    config.save(&config_path)?;
                    if !cli.quiet {
                        println!("{} Global remote cleared", "".yellow());
                    }
                } else if let Some(url) = &remote.set {
                    config.default_remote = Some(url.clone());
                    config.save(&config_path)?;
                    if !cli.quiet {
                        println!(
                            "{} Global remote set to {} (applies to all projects)",
                            "".yellow(),
                            url
                        );
                    }
                }
            } else {
                // Handle local config
                let repo = open_repo(cli)?;
                let next = if remote.clear {
                    update_remote(&repo, None)?
                } else {
                    let value = remote
                        .set
                        .as_deref()
                        .context("Specify --set <url> or --clear")?;
                    update_remote(&repo, Some(value))?
                };
                if !cli.quiet {
                    match next {
                        Some(url) => println!("{} Remote set to {}", "".yellow(), url),
                        None => println!("{} Remote cleared", "".yellow()),
                    }
                }
            }
        }
        ConfigCommand::Redact(redact) => {
            let repo = open_repo(cli)?;
            let mut additions = Vec::new();
            for item in &redact.set {
                additions.push(parse_redaction_entry(item)?);
            }
            let report = update_redaction(&repo, additions, redact.unset.clone())?;
            if !cli.quiet {
                if !report.set.is_empty() {
                    println!("{} Updated patterns:", "".magenta());
                    for (key, pattern) in report.set {
                        println!("  {} = {}", key.cyan(), pattern);
                    }
                }
                if !report.removed.is_empty() {
                    println!("{} Removed:", "".magenta());
                    for key in report.removed {
                        println!("  {}", key);
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_redaction_entry(entry: &str) -> Result<(String, String)> {
    let (key, value) = entry
        .split_once('=')
        .context("Redaction entries must be NAME=REGEX")?;
    ensure!(!key.trim().is_empty(), "Redaction name cannot be empty");
    ensure!(
        !value.trim().is_empty(),
        "Redaction pattern cannot be empty"
    );
    Ok((key.trim().to_string(), value.to_string()))
}

fn get_interactive_body() -> Result<String> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::Input;

    println!(" Enter your note content (press Ctrl+D or Ctrl+Z when finished):");

    let mut body = String::new();
    while let Ok(line) = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(">")
        .allow_empty(true)
        .interact_text()
    {
        if line.trim().is_empty() {
            break;
        }
        body.push_str(&line);
        body.push('\n');
    }

    if body.trim().is_empty() {
        bail!("Note body cannot be empty");
    }

    Ok(body.trim().to_string())
}

fn determine_remote(repo: &FukuraRepo, override_url: Option<&str>) -> Result<String> {
    if let Some(url) = override_url {
        return Ok(url.trim().to_string());
    }
    if let Some(default_remote) = repo.config()?.default_remote {
        return Ok(default_remote);
    }
    bail!("Remote URL not configured. Use --remote or `fuku config remote --set <url>`.")
}

async fn handle_serve(cli: &Cli, cmd: &ServeCommand) -> Result<()> {
    let repo = open_repo(cli)?;
    let addr = cmd.addr.clone();
    let listener = TcpListener::bind(&addr).await?;
    let state = ServeState {
        repo: Arc::new(repo.clone()),
        index: Arc::new(SearchIndex::open_or_create(&repo)?),
        default_limit: cmd.page_size,
    };
    let app = Router::new()
        .route("/healthz", get(health))
        .route("/notes", get(list_notes).post(create_note))
        .route("/notes/:id", get(show_note))
        .with_state(state);
    if !cli.quiet {
        println!("{} Serving at http://{}", "".bright_blue(), addr);
    }
    axum::serve(listener, app).await?;
    Ok(())
}

fn open_repo(cli: &Cli) -> Result<FukuraRepo> {
    match &cli.repo {
        Some(path) => FukuraRepo::open(path),
        None => FukuraRepo::discover(None),
    }
}

fn normalize_tags(raw: Vec<String>) -> Vec<String> {
    let mut tags = raw
        .into_iter()
        .map(|t| t.trim().to_lowercase().replace(' ', "-"))
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    tags
}

fn parse_meta(raw: Vec<String>) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for item in raw {
        let Some((key, value)) = item.split_once('=') else {
            bail!("Invalid meta '{}': use key=value", item);
        };
        map.insert(key.trim().to_string(), value.trim().to_string());
    }
    Ok(map)
}

fn resolve_author(name: Option<&str>, email: Option<&str>) -> Author {
    let default_name = name
        .map(|s| s.to_string())
        .or_else(|| std::env::var("GIT_AUTHOR_NAME").ok())
        .unwrap_or_else(|| {
            std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_string())
        });
    let default_email = email
        .map(|s| s.to_string())
        .or_else(|| std::env::var("GIT_AUTHOR_EMAIL").ok())
        .or_else(|| std::env::var("EMAIL").ok());
    Author {
        name: default_name,
        email: default_email,
    }
}

fn render_search_table(hits: &[SearchHit]) {
    if hits.is_empty() {
        println!(
            "{} No results found. Try broader search terms.",
            "Info:".dimmed()
        );
        return;
    }
    let mut table = Table::new();
    table
        .load_preset(UTF8_HORIZONTAL_ONLY)
        .set_header(vec!["#", "Title", "Likes", "Updated", "By", "Tags"]);
    for (idx, hit) in hits.iter().enumerate() {
        table.add_row(vec![
            format!("{:>2}", idx + 1),
            hit.title.clone(),
            hit.likes.to_string(),
            hit.updated_at.format("%Y-%m-%d").to_string(),
            hit.author.clone(),
            hit.tags.join(", "),
        ]);
    }
    println!("{}", " Results".bold());
    println!("{}", table);
}

fn render_note(record: &NoteRecord) {
    let note = &record.note;
    println!("{}", note.title.bold());
    println!(
        "{} {} · {}",
        "".cyan(),
        record.object_id,
        note.updated_at.format("%Y-%m-%d %H:%M UTC")
    );
    if !note.tags.is_empty() {
        println!("{} #{}", "".yellow(), note.tags.join(" #"));
    }
    if !note.links.is_empty() {
        println!("{}", " Links".bold());
        for link in &note.links {
            println!("  - {}", link);
        }
    }
    println!();
    println!("{}", note.body);
    if !note.meta.is_empty() {
        println!();
        println!("{}", " Meta".bold());
        for (key, value) in &note.meta {
            println!("  {} = {}", key.cyan(), value);
        }
    }
}

fn render_note_html(record: &NoteRecord, theme: &str) -> Result<String> {
    let background = match theme {
        "light" => "#fdfdfd",
        _ => "#0f172a",
    };
    let foreground = match theme {
        "light" => "#111827",
        _ => "#e2e8f0",
    };
    let accent = match theme {
        "light" => "#2563eb",
        _ => "#38bdf8",
    };
    let mut body_html = String::new();
    pulldown_cmark::html::push_html(
        &mut body_html,
        pulldown_cmark::Parser::new(&record.note.body),
    );
    let tags = if record.note.tags.is_empty() {
        String::new()
    } else {
        format!(
            "<div class=\"tags\">{}</div>",
            record
                .note
                .tags
                .iter()
                .map(|t| format!("<span>{}</span>", t))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    let meta = if record.note.meta.is_empty() {
        String::new()
    } else {
        let items = record
            .note
            .meta
            .iter()
            .map(|(k, v)| format!("<li><strong>{}</strong> {}</li>", k, v))
            .collect::<Vec<_>>()
            .join("\n");
        format!("<section><h2>Meta</h2><ul>{}</ul></section>", items)
    };
    let links = if record.note.links.is_empty() {
        String::new()
    } else {
        let items = record
            .note
            .links
            .iter()
            .map(|l| format!("<li><a href=\"{0}\">{0}</a></li>", l))
            .collect::<Vec<_>>()
            .join("\n");
        format!("<section><h2>Links</h2><ul>{}</ul></section>", items)
    };
    Ok(format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>{title}</title>
  <style>
    :root {{
      color-scheme: {scheme};
    }}
    body {{
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
      background: {background};
      color: {foreground};
      margin: 0;
      padding: 3rem 1.5rem 4rem;
      display: flex;
      justify-content: center;
    }}
    main {{
      max-width: 720px;
      width: 100%;
      background: rgba(15,23,42,0.03);
      backdrop-filter: blur(12px);
      border-radius: 24px;
      padding: 2.5rem 3rem;
      box-shadow: 0 24px 48px rgba(15,23,42,0.18);
    }}
    header h1 {{
      font-size: 2.6rem;
      letter-spacing: -0.03em;
      margin-bottom: 1rem;
    }}
    header .meta {{
      display: flex;
      gap: 1rem;
      align-items: center;
      color: rgba(255,255,255,0.65);
      text-transform: uppercase;
      font-weight: 600;
      letter-spacing: 0.12em;
      font-size: 0.78rem;
    }}
    .tags {{
      margin: 1.5rem 0 0;
      display: flex;
      gap: 0.75rem;
      flex-wrap: wrap;
    }}
    .tags span {{
      background: rgba(56,189,248,0.12);
      color: {accent};
      padding: 0.35rem 0.75rem;
      border-radius: 999px;
      font-size: 0.85rem;
      font-weight: 600;
    }}
    section {{ margin-top: 2.5rem; }}
    section h2 {{
      font-size: 1.1rem;
      text-transform: uppercase;
      letter-spacing: 0.16em;
      color: rgba(255,255,255,0.5);
      margin-bottom: 0.8rem;
    }}
    section ul {{
      list-style: none;
      padding: 0;
      margin: 0;
      display: grid;
      gap: 0.4rem;
    }}
    section ul li {{
      padding: 0.6rem 0.8rem;
      background: rgba(15,23,42,0.12);
      border-radius: 12px;
    }}
    a {{ color: {accent}; text-decoration: none; font-weight: 600; }}
    a:hover {{ text-decoration: underline; }}
    article {{
      margin-top: 2rem;
      line-height: 1.7;
      font-size: 1.02rem;
    }}
    article pre {{
      background: rgba(15,23,42,0.85);
      color: #e2e8f0;
      padding: 1rem 1.5rem;
      border-radius: 16px;
      overflow-x: auto;
      font-size: 0.9rem;
    }}
    article code {{
      font-family: 'JetBrains Mono', 'Fira Code', monospace;
      background: rgba(15,23,42,0.35);
      padding: 0.2rem 0.45rem;
      border-radius: 8px;
      font-size: 0.9rem;
    }}
    footer {{
      margin-top: 3rem;
      display: flex;
      justify-content: space-between;
      color: rgba(255,255,255,0.35);
      font-size: 0.8rem;
    }}
  </style>
</head>
<body>
  <main>
    <header>
      <h1>{title}</h1>
      <div class="meta">
        <span>{updated}</span>
        <span>{author}</span>
        <span>{privacy}</span>
      </div>
      {tags}
    </header>
    <article>{body}</article>
    {links}
    {meta}
    <footer>
      <span>Fukura · {object_id}</span>
      <span>{created}</span>
    </footer>
  </main>
</body>
</html>
"#,
        title = html_escape::encode_text(&record.note.title),
        scheme = if theme == "light" { "light" } else { "dark" },
        background = background,
        foreground = foreground,
        accent = accent,
        tags = tags,
        body = body_html,
        links = links,
        meta = meta,
        updated = record.note.updated_at.format("%Y-%m-%d %H:%M UTC"),
        created = record.note.created_at.format("%Y-%m-%d %H:%M UTC"),
        author = html_escape::encode_text(&record.note.author.name),
        privacy = format_privacy(&record.note.privacy)
            .to_string()
            .to_uppercase(),
        object_id = record.object_id,
    ))
}

struct TuiCleanup;

impl Drop for TuiCleanup {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = queue!(stdout, LeaveAlternateScreen);
        let _ = stdout.flush();
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum FocusArea {
    Results,
    Filters,
}

#[derive(Clone)]
enum TimeFilter {
    All,
    Days(u32),
}

impl TimeFilter {
    fn matches(&self, timestamp: &chrono::DateTime<Utc>) -> bool {
        match self {
            TimeFilter::All => true,
            TimeFilter::Days(days) => {
                let threshold = Utc::now() - Duration::days(*days as i64);
                timestamp >= &threshold
            }
        }
    }

    fn label(&self) -> &'static str {
        match self {
            TimeFilter::All => "All time",
            TimeFilter::Days(7) => "Last 7 days",
            TimeFilter::Days(30) => "Last 30 days",
            TimeFilter::Days(90) => "Last 90 days",
            TimeFilter::Days(_) => "Custom",
        }
    }
}

fn apply_filters(
    hits: &[SearchHit],
    selected_tags: &HashSet<String>,
    time_filter: &TimeFilter,
) -> Vec<SearchHit> {
    hits.iter()
        .filter(|hit| {
            (selected_tags.is_empty()
                || selected_tags
                    .iter()
                    .all(|tag| hit.tags.iter().any(|t| t == tag)))
                && time_filter.matches(&hit.updated_at)
        })
        .cloned()
        .collect()
}

fn run_search_tui(repo: &FukuraRepo, query: &str, sort: SearchSort, limit: usize) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let guard = TuiCleanup;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut all_results = repo.search(query, limit, sort)?;
    let available_tags = repo.collect_tags().unwrap_or_default();
    let mut selected_tags: HashSet<String> = HashSet::new();
    let mut time_filter = TimeFilter::All;
    let mut displayed = apply_filters(&all_results, &selected_tags, &time_filter);

    let mut result_state = ListState::default();
    if !displayed.is_empty() {
        result_state.select(Some(0));
    }
    let mut tag_state = ListState::default();
    if !available_tags.is_empty() {
        tag_state.select(Some(0));
    }

    let mut focus = FocusArea::Results;
    let mut cached: Option<NoteRecord> = None;

    loop {
        if let Some(selected) = result_state.selected() {
            if let Some(hit) = displayed.get(selected) {
                if cached.as_ref().map(|n| &n.object_id) != Some(&hit.object_id) {
                    cached = repo.load_note(&hit.object_id).ok();
                }
            } else {
                cached = None;
            }
        } else {
            cached = None;
        }

        terminal.draw(|frame| {
            let size = frame.area();
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
                .split(size);

            let filter_block = Block::default()
                .title("Filters")
                .borders(Borders::ALL)
                .border_style(match focus {
                    FocusArea::Filters => Style::default().fg(Color::Cyan),
                    FocusArea::Results => Style::default(),
                });
            frame.render_widget(filter_block.clone(), columns[0]);
            let filter_inner = filter_block.inner(columns[0]);
            let filter_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(filter_inner);

            let tag_items: Vec<ListItem> = if available_tags.is_empty() {
                vec![ListItem::new("No tags indexed yet.")]
            } else {
                available_tags
                    .iter()
                    .map(|tag| {
                        let active = selected_tags.contains(tag);
                        let marker = if active { "☑" } else { "☐" };
                        ListItem::new(Span::styled(
                            format!("{} {}", marker, tag),
                            Style::default().fg(if active { Color::Magenta } else { Color::Gray }),
                        ))
                    })
                    .collect()
            };
            let highlight = if matches!(focus, FocusArea::Filters) {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            };
            let tag_list = List::new(tag_items)
                .block(Block::default().borders(Borders::ALL).title("Tags"))
                .highlight_style(highlight)
                .highlight_symbol(" ");
            frame.render_stateful_widget(tag_list, filter_chunks[0], &mut tag_state);

            let filter_help = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("Time:", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" {}", time_filter.label())),
                ]),
                Line::from("1:All  2:7d  3:30d  4:90d"),
                Line::from("Space: toggle tag | f: clear filters"),
                Line::from("Tab: switch focus"),
            ])
            .wrap(Wrap { trim: true });
            frame.render_widget(filter_help, filter_chunks[1]);

            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(columns[1]);

            let items: Vec<ListItem> = if displayed.is_empty() {
                vec![ListItem::new(
                    "No matches for the current filters.".to_string(),
                )]
            } else {
                displayed
                    .iter()
                    .map(|hit| {
                        let tags = if hit.tags.is_empty() {
                            String::new()
                        } else {
                            format!(" #{}", hit.tags.join(" #"))
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                hit.title.clone(),
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(" "),
                            Span::styled(
                                hit.updated_at.format("%Y-%m-%d").to_string(),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::raw(tags),
                        ]))
                    })
                    .collect()
            };
            let result_highlight = if matches!(focus, FocusArea::Results) {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            };
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Results ({})", displayed.len())),
                )
                .highlight_style(result_highlight)
                .highlight_symbol(" ");
            frame.render_stateful_widget(list, main_chunks[0], &mut result_state);

            let detail_block = Block::default().title("Preview").borders(Borders::ALL);
            let detail = if let Some(note) = &cached {
                Paragraph::new(note.note.body.clone()).wrap(Wrap { trim: false })
            } else {
                Paragraph::new("Select a note to preview.")
            };
            frame.render_widget(detail.block(detail_block), main_chunks[1]);
        })?;

        if event::poll(StdDuration::from_millis(350))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('o') => {
                        if let Some(note) = &cached {
                            let _ = handle_open_inline(note);
                        }
                    }
                    KeyCode::Char('r') => {
                        all_results = repo.search(query, limit, sort)?;
                        displayed = apply_filters(&all_results, &selected_tags, &time_filter);
                        if displayed.is_empty() {
                            result_state.select(None);
                        } else {
                            result_state.select(Some(0));
                        }
                        cached = None;
                    }
                    KeyCode::Tab => {
                        focus = match focus {
                            FocusArea::Results if !available_tags.is_empty() => {
                                if tag_state.selected().is_none() {
                                    tag_state.select(Some(0));
                                }
                                FocusArea::Filters
                            }
                            FocusArea::Results => FocusArea::Results,
                            FocusArea::Filters => {
                                if displayed.is_empty() {
                                    result_state.select(None);
                                } else if result_state.selected().is_none() {
                                    result_state.select(Some(0));
                                }
                                FocusArea::Results
                            }
                        };
                    }
                    KeyCode::Char('f') => {
                        selected_tags.clear();
                        time_filter = TimeFilter::All;
                        displayed = apply_filters(&all_results, &selected_tags, &time_filter);
                        if displayed.is_empty() {
                            result_state.select(None);
                        } else {
                            result_state.select(Some(0));
                        }
                        cached = None;
                    }
                    KeyCode::Char('1') => {
                        time_filter = TimeFilter::All;
                        displayed = apply_filters(&all_results, &selected_tags, &time_filter);
                        if displayed.is_empty() {
                            result_state.select(None);
                        } else {
                            result_state.select(Some(0));
                        }
                        cached = None;
                    }
                    KeyCode::Char('2') => {
                        time_filter = TimeFilter::Days(7);
                        displayed = apply_filters(&all_results, &selected_tags, &time_filter);
                        if displayed.is_empty() {
                            result_state.select(None);
                        } else {
                            result_state.select(Some(0));
                        }
                        cached = None;
                    }
                    KeyCode::Char('3') => {
                        time_filter = TimeFilter::Days(30);
                        displayed = apply_filters(&all_results, &selected_tags, &time_filter);
                        if displayed.is_empty() {
                            result_state.select(None);
                        } else {
                            result_state.select(Some(0));
                        }
                        cached = None;
                    }
                    KeyCode::Char('4') => {
                        time_filter = TimeFilter::Days(90);
                        displayed = apply_filters(&all_results, &selected_tags, &time_filter);
                        if displayed.is_empty() {
                            result_state.select(None);
                        } else {
                            result_state.select(Some(0));
                        }
                        cached = None;
                    }
                    KeyCode::Char(' ') if matches!(focus, FocusArea::Filters) => {
                        if let Some(selected) = tag_state.selected() {
                            if let Some(tag) = available_tags.get(selected) {
                                if !tag.is_empty() {
                                    if selected_tags.contains(tag) {
                                        selected_tags.remove(tag);
                                    } else {
                                        selected_tags.insert(tag.clone());
                                    }
                                    displayed =
                                        apply_filters(&all_results, &selected_tags, &time_filter);
                                    if displayed.is_empty() {
                                        result_state.select(None);
                                    } else {
                                        result_state.select(Some(0));
                                    }
                                    cached = None;
                                }
                            }
                        }
                    }
                    KeyCode::Up => match focus {
                        FocusArea::Results => {
                            if let Some(current) = result_state.selected() {
                                let new = current.saturating_sub(1);
                                result_state.select(Some(new));
                            }
                        }
                        FocusArea::Filters => {
                            if let Some(current) = tag_state.selected() {
                                let new = current.saturating_sub(1);
                                tag_state.select(Some(new));
                            }
                        }
                    },
                    KeyCode::Down => match focus {
                        FocusArea::Results => {
                            if let Some(current) = result_state.selected() {
                                if current + 1 < displayed.len() {
                                    result_state.select(Some(current + 1));
                                }
                            } else if !displayed.is_empty() {
                                result_state.select(Some(0));
                            }
                        }
                        FocusArea::Filters => {
                            if let Some(current) = tag_state.selected() {
                                if current + 1 < available_tags.len() {
                                    tag_state.select(Some(current + 1));
                                }
                            }
                        }
                    },
                    KeyCode::Enter => {
                        if matches!(focus, FocusArea::Results) {
                            if let Some(note) = &cached {
                                render_note(note);
                            }
                        }
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
    terminal.show_cursor()?;
    drop(terminal);
    drop(guard);
    Ok(())
}

fn handle_open_inline(record: &NoteRecord) -> Result<()> {
    let html = render_note_html(record, "dark")?;
    let filename = format!("fuku-{}.html", record.object_id);

    // Use the new cross-platform browser opener
    crate::browser::BrowserOpener::open_with_server(&html, &filename).or_else(|_| {
        // Fallback: save to file and try direct opening
        let file_path = std::env::temp_dir().join(&filename);
        fs::write(&file_path, html)?;
        crate::browser::BrowserOpener::open(&file_path)
    })?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[derive(Debug, Deserialize)]
struct ListParams {
    q: Option<String>,
    limit: Option<usize>,
    sort: Option<SearchSort>,
}

#[derive(Clone)]
struct ServeState {
    repo: Arc<FukuraRepo>,
    index: Arc<SearchIndex>,
    default_limit: usize,
}

async fn list_notes(
    State(state): State<ServeState>,
    AxumQuery(params): AxumQuery<ListParams>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    let limit = params.limit.unwrap_or(state.default_limit);
    let sort = params.sort.unwrap_or(SearchSort::Updated);
    match state.index.search(&query, limit, sort) {
        Ok(results) => Json(results).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "search failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn show_note(
    State(state): State<ServeState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match state.repo.resolve_object_id(&id) {
        Ok(resolved) => match state.repo.load_note(&resolved) {
            Ok(note) => Json(note).into_response(),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        },
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn create_note(
    State(state): State<ServeState>,
    Json(payload): Json<NoteRecord>,
) -> impl IntoResponse {
    match state.repo.store_note(payload.note) {
        Ok(record) => Json(record).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "failed to store note");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn format_privacy(privacy: &Privacy) -> String {
    match privacy {
        Privacy::Private => "private".into(),
        Privacy::Org => "org".into(),
        Privacy::Public => "public".into(),
    }
}

async fn handle_daemon(cli: &Cli, cmd: &DaemonCommand) -> Result<()> {
    let repo = open_repo(cli)?;

    // Handle shell hooks management
    if cmd.install_hooks {
        let hook_manager = crate::hooks::HookManager::new(repo.root());
        hook_manager.install_hooks()?;
        if !cli.quiet {
            println!("{} Shell hooks installed successfully", "".green());
        }
        return Ok(());
    }

    if cmd.uninstall_hooks {
        let hook_manager = crate::hooks::HookManager::new(repo.root());
        hook_manager.uninstall_hooks()?;
        if !cli.quiet {
            println!("{} Shell hooks uninstalled", "".yellow());
        }
        return Ok(());
    }

    if cmd.hooks_status {
        let hook_manager = crate::hooks::HookManager::new(repo.root());
        let installed = hook_manager.are_hooks_installed()?;
        if !cli.quiet {
            if installed {
                println!("{} Shell hooks are installed", "".green());
            } else {
                println!("{} Shell hooks are not installed", "".red());
                println!("{} Use 'fuku daemon --install-hooks' to install", "".cyan());
            }
        }
        return Ok(());
    }

    // Handle notification settings
    if cmd.notifications_enable {
        let mut notif_mgr = crate::notification::NotificationManager::new(repo.root())?;
        notif_mgr.enable()?;
        if !cli.quiet {
            println!("{} Error notifications enabled", "".green());
        }
        return Ok(());
    }

    if cmd.notifications_disable {
        let mut notif_mgr = crate::notification::NotificationManager::new(repo.root())?;
        notif_mgr.disable()?;
        if !cli.quiet {
            println!("{} Error notifications disabled", "".yellow());
        }
        return Ok(());
    }

    if cmd.notifications_status {
        let notif_mgr = crate::notification::NotificationManager::new(repo.root())?;
        if !cli.quiet {
            if notif_mgr.is_enabled() {
                println!("{} Notifications: {}", "".blue(), "Enabled".green());
            } else {
                println!("{} Notifications: {}", "".blue(), "Disabled".red());
                println!(
                    "{} Use 'fuku daemon --notifications-enable' to enable",
                    "".cyan()
                );
            }
        }
        return Ok(());
    }
    
    if cmd.test_notification {
        let notif_mgr = crate::notification::NotificationManager::new(repo.root())?;
        if !cli.quiet {
            println!("{} Sending test notification...", "".blue());
        }
        match notif_mgr.send_test_notification() {
            Ok(_) => {
                if !cli.quiet {
                    println!("{} Test notification sent successfully", "".green());
                    println!("{} Check your notification center", "".cyan());
                }
            }
            Err(e) => {
                if !cli.quiet {
                    println!("{} Failed to send notification: {}", "".red(), e);
                }
            }
        }
        return Ok(());
    }

    let config = crate::daemon::DaemonConfig::default();
    let daemon = crate::daemon::FukuraDaemon::new(repo.root(), config)?;

    if cmd.status {
        // Show daemon status
        let daemon_service = crate::daemon_service::DaemonService::new(repo.root());
        let config = repo.config()?;

        if !cli.quiet {
            if daemon_service.is_running().await {
                println!("{} Daemon status: {}", "".blue(), "Running".green());
                println!(
                    "{} PID file: {}",
                    "".blue(),
                    daemon_service.get_pid_file_path().display()
                );

                // Show what daemon monitors
                println!("\n{} Monitoring:", "".cyan());
                println!("  • Command executions and exit codes");
                println!("  • Error messages from stderr");
                println!("  • Working directory and git context");
                println!("  • Session timeout: 10 minutes (default)");

                // Show what gets recorded
                println!("\n{} Recording:", "".cyan());
                println!(
                    "  • All data stored locally in {}",
                    repo.root().join(".fukura").display()
                );
                println!("  • Private by default (use 'fuku sync' to share)");
                println!("  • Auto-generated notes after 5 min inactivity");

                // Show configuration
                println!("\n{} Configuration:", "".cyan());
                println!(
                    "  • Auto-sync: {}",
                    if config.auto_sync.unwrap_or(false) {
                        "enabled".green()
                    } else {
                        "disabled".red()
                    }
                );
                if let Some(remote) = &config.default_remote {
                    println!("  • Default remote: {}", remote);
                }
            } else {
                println!("{} Daemon status: {}", "".blue(), "Stopped".red());
                println!("{} Run 'fuku daemon' to start monitoring", "".cyan());
            }
        }
    } else if cmd.stop {
        // Stop daemon
        let daemon_service = crate::daemon_service::DaemonService::new(repo.root());

        if daemon_service.is_running().await {
            daemon_service.stop_background().await?;
            if !cli.quiet {
                println!("{} Daemon stopped", "".red());
            }
        } else if !cli.quiet {
            println!("{} Daemon is not running", "".blue());
        }
    } else if cmd.record_command.is_some() || cmd.record_error.is_some() || cmd.check_solutions {
        // Handle individual commands
        let session_id = "cli_session";

        if let Some(command) = &cmd.record_command {
            let exit_code = std::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .status()
                .map(|s| s.code().unwrap_or(1))
                .unwrap_or(1);

            daemon
                .record_command(session_id, command, Some(exit_code), ".")
                .await?;
        }

        if let Some(error) = &cmd.record_error {
            daemon.record_error(session_id, error, "cli").await?;
        }

        if cmd.check_solutions {
            let solutions = daemon.check_solutions(session_id).await?;
            if !solutions.is_empty() {
                if !cli.quiet {
                    println!(
                        "{} Found {} potential solutions:",
                        "".yellow(),
                        solutions.len()
                    );
                    for solution in solutions {
                        println!(
                            "  - {} (confidence: {:.1}%)",
                            solution.solution,
                            solution.confidence * 100.0
                        );
                    }
                }
            } else if !cli.quiet {
                println!("{} No solutions found", "".red());
            }
        }
    } else {
        // Start daemon
        if cmd.foreground {
            if !cli.quiet {
                println!("{} Starting daemon in foreground...", "".green());
                println!("{} Press Ctrl+C to stop", "".blue());
            }
            daemon.start().await?;

            // Keep running until interrupted
            tokio::signal::ctrl_c().await?;
            daemon.stop().await?;
            if !cli.quiet {
                println!("{} Daemon stopped", "".red());
            }
        } else {
            // Start daemon in background (default)
            let daemon_service = crate::daemon_service::DaemonService::new(repo.root());

            if daemon_service.is_running().await {
                if !cli.quiet {
                    println!("{} Daemon is already running", "".green());
                    println!("{} Use 'fukura daemon --status' to check status", "".blue());
                }
            } else {
                daemon_service.start_background()?;
                if !cli.quiet {
                    println!("{} Daemon started in background", "".green());
                    println!("{} Now monitoring for errors automatically", "".blue());
                    println!("{} Use 'fukura daemon --status' to check status", "".blue());
                }
            }
        }
    }

    Ok(())
}
