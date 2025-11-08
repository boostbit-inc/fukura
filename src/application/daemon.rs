use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, info};

use crate::models::{Author, Note, Privacy};
use crate::notification::NotificationManager;
use crate::repo::FukuraRepo;

/// Daemon for monitoring and capturing error patterns
pub struct FukuraDaemon {
    pub repo: Arc<FukuraRepo>,
    pub sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
    pub error_patterns: Arc<RwLock<HashMap<String, ErrorPattern>>>,
    pub repo_path: std::path::PathBuf,
    config: DaemonConfig,
    notification_manager: Option<Arc<NotificationManager>>,
}

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub monitor_interval: Duration,
    pub session_timeout: Duration,
    pub max_sessions: usize,
    pub enable_clipboard_monitoring: bool,
    pub enable_process_monitoring: bool,
    pub error_threshold: f64,
    pub max_commands_per_session: usize, // NEW: Limit commands to prevent memory issues
    pub enable_activity_tracking: bool,  // NEW: Enable comprehensive activity tracking
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            monitor_interval: Duration::from_secs(10), // Reduced frequency for better performance
            session_timeout: Duration::from_secs(600), // 10 minutes - longer timeout
            max_sessions: 50,                          // Reduced for better memory usage
            enable_clipboard_monitoring: false,        // Off by default for privacy
            enable_process_monitoring: false,          // Off by default for performance
            error_threshold: 0.7,
            max_commands_per_session: 1000, // Limit to 1000 commands per session
            enable_activity_tracking: false, // Off by default until explicitly enabled
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub id: String,
    pub start_time: SystemTime,
    pub last_activity: SystemTime,
    pub commands: Vec<CommandEntry>,
    pub errors: Vec<ErrorEntry>,
    pub context: SessionContext,
    pub last_error_command: Option<String>, // Track which command caused the last error
    pub resolution_in_progress: bool,       // Track if we're in error→resolution flow
}

#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub command: String,
    pub exit_code: Option<i32>,
    pub timestamp: SystemTime,
    pub working_directory: String,
}

#[derive(Debug, Clone)]
pub struct ErrorEntry {
    pub message: String,
    pub normalized: String,
    pub source: String, // stderr, clipboard, etc.
    pub timestamp: SystemTime,
    pub stderr_output: Option<String>, // Actual stderr content from command
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub working_directory: String,
    pub git_branch: Option<String>,
    pub git_status: Option<String>,
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ErrorPattern {
    pub normalized_message: String,
    pub fingerprint: String,
    pub occurrences: u32,
    pub last_seen: SystemTime,
    pub solutions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SolutionHit {
    pub note_id: String,
    pub title: String,
    pub snippet: String,
    pub confidence: f64,
}

impl FukuraDaemon {
    /// Create a new daemon instance
    pub fn new(repo_path: &Path, config: DaemonConfig) -> Result<Self> {
        let repo = Arc::new(FukuraRepo::discover(Some(repo_path))?);
        let notification_manager = NotificationManager::new(repo_path).ok().map(Arc::new);

        Ok(Self {
            repo,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            error_patterns: Arc::new(RwLock::new(HashMap::new())),
            repo_path: repo_path.to_path_buf(),
            config,
            notification_manager,
        })
    }

    /// Get commands from all sessions since a specific time
    pub async fn get_commands_since(&self, since: SystemTime) -> Vec<CommandEntry> {
        let sessions = self.sessions.read().await;
        let mut commands = Vec::new();

        for session in sessions.values() {
            for command in &session.commands {
                if command.timestamp >= since {
                    commands.push(command.clone());
                }
            }
        }

        // Sort by timestamp
        commands.sort_by_key(|cmd| cmd.timestamp);
        commands
    }

    /// Create a recording session from historical commands
    pub async fn create_recording_from_time(&self, since: SystemTime, title: String) -> Result<()> {
        let commands = self.get_commands_since(since).await;

        if commands.is_empty() {
            anyhow::bail!("No commands found since the specified time");
        }

        // Create recording file with historical commands
        let recording_file = self.repo_path.join(".fukura").join("recording");
        let timestamp = since.duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
        let content = format!(
            "{}|{}|{}",
            timestamp,
            title,
            commands
                .iter()
                .map(|cmd| format!(
                    "{}:{}",
                    cmd.timestamp
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    cmd.command
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );

        std::fs::write(&recording_file, content)?;
        Ok(())
    }

    /// Start the daemon
    pub async fn start(&self) -> Result<()> {
        info!("Starting Fukura daemon...");

        // Load existing error patterns
        self.load_error_patterns().await?;

        // Start monitoring tasks
        let sessions1 = self.sessions.clone();
        let sessions2 = self.sessions.clone();
        let sessions3 = self.sessions.clone();
        let error_patterns = self.error_patterns.clone();
        let config1 = self.config.clone();
        let config2 = self.config.clone();
        let repo = self.repo.clone();
        let repo_path = self.repo_path.clone();
        let notif_mgr = self.notification_manager.clone();

        // Session cleanup task
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                Self::cleanup_sessions(&sessions1, &config1).await;
            }
        });

        // Error pattern analysis task
        tokio::spawn(async move {
            let mut interval = time::interval(config2.monitor_interval);
            loop {
                interval.tick().await;
                Self::analyze_error_patterns(&error_patterns).await;
            }
        });

        // Auto note generation task
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                Self::auto_generate_notes(&sessions2, &repo, &repo_path).await;
            }
        });

        // Start Unix Domain Socket server for IPC (best practice)
        let sessions_for_server = sessions3.clone();
        let socket_path = self.get_socket_path();
        tokio::spawn(async move {
            if let Err(e) =
                Self::start_socket_server(sessions_for_server, notif_mgr, socket_path).await
            {
                tracing::error!("Socket server error: {}", e);
            }
        });

        info!("Fukura daemon started successfully");
        Ok(())
    }

    /// Get socket path for IPC
    fn get_socket_path(&self) -> std::path::PathBuf {
        self.repo_path.join(".fukura").join("daemon.sock")
    }

    /// Start IPC server for shell hook communication (BEST PRACTICE: Unix Socket / Named Pipe)
    async fn start_socket_server(
        sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
        notif_mgr: Option<Arc<NotificationManager>>,
        socket_path: std::path::PathBuf,
    ) -> Result<()> {
        #[cfg(unix)]
        {
            Self::start_unix_socket_server(sessions, notif_mgr, socket_path).await
        }

        #[cfg(windows)]
        {
            Self::start_named_pipe_server(sessions, notif_mgr, socket_path).await
        }
    }

    #[cfg(unix)]
    async fn start_unix_socket_server(
        sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
        notif_mgr: Option<Arc<NotificationManager>>,
        socket_path: std::path::PathBuf,
    ) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixListener;

        // Remove old socket if exists
        let _ = std::fs::remove_file(&socket_path);

        // Create Unix socket
        let listener = UnixListener::bind(&socket_path)?;
        info!("IPC socket server listening on {:?}", socket_path);

        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let sessions = sessions.clone();
                    let notif_mgr = notif_mgr.clone();

                    tokio::spawn(async move {
                        let mut buffer = vec![0u8; 4096];
                        match stream.read(&mut buffer).await {
                            Ok(n) if n > 0 => {
                                let data = &buffer[..n];
                                if let Ok(msg) = String::from_utf8(data.to_vec()) {
                                    // Parse message: "session_id|command|exit_code|working_dir|stderr"
                                    let parts: Vec<&str> = msg.trim().split('|').collect();
                                    if parts.len() >= 4 {
                                        let session_id = parts[0];
                                        let command = parts[1];
                                        let exit_code = parts[2].parse::<i32>().unwrap_or(0);
                                        let working_dir = parts[3];
                                        let stderr_content =
                                            if parts.len() >= 5 { parts[4] } else { "" };

                                        // Record command
                                        let mut sessions = sessions.write().await;

                                        if !sessions.contains_key(session_id) {
                                            sessions.insert(
                                                session_id.to_string(),
                                                ActiveSession {
                                                    id: session_id.to_string(),
                                                    start_time: SystemTime::now(),
                                                    last_activity: SystemTime::now(),
                                                    commands: Vec::new(),
                                                    errors: Vec::new(),
                                                    context: SessionContext {
                                                        working_directory: working_dir.to_string(),
                                                        git_branch: None,
                                                        git_status: None,
                                                        environment: HashMap::new(),
                                                    },
                                                    last_error_command: None,
                                                    resolution_in_progress: false,
                                                },
                                            );
                                        }

                                        if let Some(session) = sessions.get_mut(session_id) {
                                            session.commands.push(CommandEntry {
                                                command: command.to_string(),
                                                exit_code: Some(exit_code),
                                                timestamp: SystemTime::now(),
                                                working_directory: working_dir.to_string(),
                                            });
                                            session.last_activity = SystemTime::now();

                                            // Check if error and send notification
                                            if exit_code != 0 {
                                                let error_message = if !stderr_content.is_empty() {
                                                    format!(
                                                        "Command '{}' failed: {}",
                                                        command, stderr_content
                                                    )
                                                } else {
                                                    format!(
                                                        "Command '{}' failed with exit code {}",
                                                        command, exit_code
                                                    )
                                                };

                                                session.errors.push(ErrorEntry {
                                                    message: error_message.clone(),
                                                    normalized: error_message.clone(),
                                                    source: "shell".to_string(),
                                                    timestamp: SystemTime::now(),
                                                    stderr_output: if !stderr_content.is_empty() {
                                                        Some(stderr_content.to_string())
                                                    } else {
                                                        None
                                                    },
                                                });

                                                // BEST PRACTICE: Create note immediately (like Git commit)
                                                // Users can access via: fuku search, fuku view @latest
                                                drop(sessions);
                                                let wd_path = std::path::PathBuf::from(working_dir);
                                                let repo_clone = Arc::new(
                                                    FukuraRepo::discover(Some(&wd_path))
                                                        .unwrap_or_else(|_| {
                                                            FukuraRepo::discover(None)
                                                                .expect("Failed to discover repo")
                                                        }),
                                                );

                                                let body_text = if !stderr_content.is_empty() {
                                                    format!(
                                                        "## Command Failed\n\n```bash\n$ {}\n\n# Error output:\n{}\n```\n\n**Exit Code**: {}\n\n**Working Directory**: `{}`\n\n**Time**: {}",
                                                        command,
                                                        stderr_content,
                                                        exit_code,
                                                        working_dir,
                                                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
                                                    )
                                                } else {
                                                    format!(
                                                        "## Command Failed\n\n```bash\n$ {}\n```\n\n**Exit Code**: {}\n\n**Error**: {}\n\n**Working Directory**: `{}`\n\n**Time**: {}",
                                                        command,
                                                        exit_code,
                                                        error_message,
                                                        working_dir,
                                                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
                                                    )
                                                };

                                                let note = Note {
                                                    title: format!("Error: {}", command),
                                                    body: body_text,
                                                    tags: vec![
                                                        "error".to_string(),
                                                        "auto-captured".to_string(),
                                                    ],
                                                    links: vec![],
                                                    meta: std::collections::BTreeMap::from([
                                                        (
                                                            "exit_code".to_string(),
                                                            exit_code.to_string(),
                                                        ),
                                                        (
                                                            "working_dir".to_string(),
                                                            working_dir.to_string(),
                                                        ),
                                                    ]),
                                                    solutions: vec![],
                                                    privacy: Privacy::Private,
                                                    created_at: chrono::Utc::now(),
                                                    updated_at: chrono::Utc::now(),
                                                    author: Author {
                                                        name: std::env::var("USER").unwrap_or_else(
                                                            |_| "unknown".to_string(),
                                                        ),
                                                        email: None,
                                                    },
                                                };

                                                if let Ok(record) = repo_clone.store_note(note) {
                                                    tracing::info!(
                                                        "Note created: {} for error: {}",
                                                        &record.object_id[..8],
                                                        command
                                                    );

                                                    // WORLD-CLASS: Search for similar errors and solutions
                                                    let similar_solutions =
                                                        Self::find_similar_solutions(
                                                            &repo_clone,
                                                            command,
                                                            exit_code,
                                                        );

                                                    // Send intelligent notification
                                                    if let Some(ref nm) = notif_mgr {
                                                        tracing::info!(
                                                            "Sending notification for error: {}",
                                                            command
                                                        );
                                                        if let Ok(solutions) = similar_solutions {
                                                            if !solutions.is_empty() {
                                                                tracing::info!(
                                                                    "Found {} solutions",
                                                                    solutions.len()
                                                                );
                                                                if let Err(e) = nm
                                                                    .notify_error_with_solutions(
                                                                        command,
                                                                        &error_message,
                                                                        &record.object_id,
                                                                        &solutions,
                                                                    )
                                                                {
                                                                    tracing::error!(
                                                                        "Notification failed: {}",
                                                                        e
                                                                    );
                                                                } else {
                                                                    tracing::info!("Notification sent successfully with solutions");
                                                                }
                                                            } else if let Err(e) = nm
                                                                .notify_error_with_id(
                                                                    command,
                                                                    &error_message,
                                                                    &record.object_id,
                                                                )
                                                            {
                                                                tracing::error!(
                                                                    "Notification failed: {}",
                                                                    e
                                                                );
                                                            } else {
                                                                tracing::info!("Notification sent successfully");
                                                            }
                                                        } else if let Err(e) = nm
                                                            .notify_error_with_id(
                                                                command,
                                                                &error_message,
                                                                &record.object_id,
                                                            )
                                                        {
                                                            tracing::error!(
                                                                "Notification failed: {}",
                                                                e
                                                            );
                                                        } else {
                                                            tracing::info!(
                                                                "Notification sent successfully"
                                                            );
                                                        }
                                                    } else {
                                                        tracing::warn!(
                                                            "Notification manager not available"
                                                        );
                                                    }
                                                }
                                            }
                                        }

                                        // Send response
                                        let _ = stream.write_all(b"OK\n").await;
                                    }
                                }
                            }
                            _ => {}
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Socket accept error: {}", e);
                }
            }
        }
    }

    #[cfg(windows)]
    async fn start_named_pipe_server(
        sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
        notif_mgr: Option<Arc<NotificationManager>>,
        _socket_path: std::path::PathBuf,
    ) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

        let pipe_name = r"\\.\pipe\fukura_daemon";

        loop {
            let server = ServerOptions::new()
                .first_pipe_instance(true)
                .create(pipe_name)?;

            let sessions = sessions.clone();
            let notif_mgr = notif_mgr.clone();

            tokio::spawn(async move {
                let mut server = server;
                server.connect().await.ok();

                let mut buffer = vec![0u8; 4096];
                match server.read(&mut buffer).await {
                    Ok(n) if n > 0 => {
                        let data = &buffer[..n];
                        if let Ok(msg) = String::from_utf8(data.to_vec()) {
                            let parts: Vec<&str> = msg.trim().split('|').collect();
                            if parts.len() >= 4 {
                                let session_id = parts[0];
                                let command = parts[1];
                                let exit_code = parts[2].parse::<i32>().unwrap_or(0);
                                let working_dir = parts[3];

                                let mut sessions = sessions.write().await;

                                if !sessions.contains_key(session_id) {
                                    sessions.insert(
                                        session_id.to_string(),
                                        ActiveSession {
                                            id: session_id.to_string(),
                                            start_time: SystemTime::now(),
                                            last_activity: SystemTime::now(),
                                            commands: Vec::new(),
                                            errors: Vec::new(),
                                            context: SessionContext {
                                                working_directory: working_dir.to_string(),
                                                git_branch: None,
                                                git_status: None,
                                                environment: HashMap::new(),
                                            },
                                            last_error_command: None,
                                            resolution_in_progress: false,
                                        },
                                    );
                                }

                                if let Some(session) = sessions.get_mut(session_id) {
                                    session.commands.push(CommandEntry {
                                        command: command.to_string(),
                                        exit_code: Some(exit_code),
                                        timestamp: SystemTime::now(),
                                        working_directory: working_dir.to_string(),
                                    });
                                    session.last_activity = SystemTime::now();

                                    if exit_code != 0 {
                                        let error_message = format!(
                                            "Command '{}' failed with exit code {}",
                                            command, exit_code
                                        );
                                        session.errors.push(ErrorEntry {
                                            message: error_message.clone(),
                                            normalized: error_message.clone(),
                                            source: "shell".to_string(),
                                            timestamp: SystemTime::now(),
                                            stderr_output: None,
                                        });

                                        drop(sessions);
                                        let wd_path = std::path::PathBuf::from(working_dir);
                                        let repo_clone = Arc::new(
                                            FukuraRepo::discover(Some(&wd_path)).unwrap_or_else(
                                                |_| {
                                                    FukuraRepo::discover(None)
                                                        .expect("Failed to discover repo")
                                                },
                                            ),
                                        );

                                        let note = Note {
                                            title: format!("Error: {}", command),
                                            body: format!(
                                                "## Command Failed\n\n```\n{}\n```\n\n**Exit Code**: {}\n\n**Error**: {}\n\n**Working Directory**: {}\n\n**Time**: {}",
                                                command,
                                                exit_code,
                                                error_message,
                                                working_dir,
                                                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
                                            ),
                                            tags: vec!["error".to_string(), "auto-captured".to_string()],
                                            links: vec![],
                                            meta: std::collections::BTreeMap::from([
                                                ("exit_code".to_string(), exit_code.to_string()),
                                                ("working_dir".to_string(), working_dir.to_string()),
                                            ]),
                                            solutions: vec![],
                                            privacy: Privacy::Private,
                                            created_at: chrono::Utc::now(),
                                            updated_at: chrono::Utc::now(),
                                            author: Author {
                                                name: std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string()),
                                                email: None,
                                            },
                                        };

                                        if let Ok(record) = repo_clone.store_note(note) {
                                            tracing::info!(
                                                "Note created: {} for error: {}",
                                                &record.object_id[..8],
                                                command
                                            );

                                            // WORLD-CLASS: Search for similar errors and solutions
                                            let similar_solutions = Self::find_similar_solutions(
                                                &repo_clone,
                                                command,
                                                exit_code,
                                            );

                                            // Send intelligent notification
                                            if let Some(ref nm) = notif_mgr {
                                                tracing::info!(
                                                    "Sending notification for error: {}",
                                                    command
                                                );
                                                if let Ok(solutions) = similar_solutions {
                                                    if !solutions.is_empty() {
                                                        tracing::info!(
                                                            "Found {} solutions",
                                                            solutions.len()
                                                        );
                                                        if let Err(e) = nm
                                                            .notify_error_with_solutions(
                                                                command,
                                                                &error_message,
                                                                &record.object_id,
                                                                &solutions,
                                                            )
                                                        {
                                                            tracing::error!(
                                                                "Notification failed: {}",
                                                                e
                                                            );
                                                        } else {
                                                            tracing::info!("Notification sent successfully with solutions");
                                                        }
                                                    } else if let Err(e) = nm.notify_error_with_id(
                                                        command,
                                                        &error_message,
                                                        &record.object_id,
                                                    ) {
                                                        tracing::error!(
                                                            "Notification failed: {}",
                                                            e
                                                        );
                                                    } else {
                                                        tracing::info!(
                                                            "Notification sent successfully"
                                                        );
                                                    }
                                                } else if let Err(e) = nm.notify_error_with_id(
                                                    command,
                                                    &error_message,
                                                    &record.object_id,
                                                ) {
                                                    tracing::error!("Notification failed: {}", e);
                                                } else {
                                                    tracing::info!(
                                                        "Notification sent successfully"
                                                    );
                                                }
                                            } else {
                                                tracing::warn!(
                                                    "Notification manager not available"
                                                );
                                            }
                                        }
                                    }
                                }

                                let _ = server.write_all(b"OK\n").await;
                            }
                        }
                    }
                    _ => {}
                }
            });
        }
    }

    /// Stop the daemon
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Fukura daemon...");

        // Save error patterns
        self.save_error_patterns().await?;

        // Clean up active sessions
        self.sessions.write().await.clear();

        info!("Fukura daemon stopped");
        Ok(())
    }

    /// Record a command execution
    pub async fn record_command(
        &self,
        session_id: &str,
        command: &str,
        exit_code: Option<i32>,
        working_dir: &str,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        if !sessions.contains_key(session_id) {
            sessions.insert(
                session_id.to_string(),
                ActiveSession {
                    id: session_id.to_string(),
                    start_time: SystemTime::now(),
                    last_activity: SystemTime::now(),
                    commands: Vec::new(),
                    errors: Vec::new(),
                    context: SessionContext {
                        working_directory: working_dir.to_string(),
                        git_branch: self.get_git_branch(working_dir).await.ok(),
                        git_status: self.get_git_status(working_dir).await.ok(),
                        environment: self.get_environment_context().await,
                    },
                    last_error_command: None,
                    resolution_in_progress: false,
                },
            );
        }

        if let Some(session) = sessions.get_mut(session_id) {
            session.commands.push(CommandEntry {
                command: command.to_string(),
                exit_code,
                timestamp: SystemTime::now(),
                working_directory: working_dir.to_string(),
            });
            session.last_activity = SystemTime::now();

            // INSTANT RESOLUTION DETECTION
            if let Some(code) = exit_code {
                if code != 0 {
                    // Error - start tracking
                    session.last_error_command = Some(command.to_string());
                    session.resolution_in_progress = true;
                    self.analyze_command_error(session, command, code).await;
                } else if session.resolution_in_progress {
                    // Success after error - INSTANT note creation!
                    let session_clone = session.clone();
                    drop(sessions);

                    // Create resolution note immediately
                    tokio::spawn(async move {
                        if let Err(e) = Self::create_instant_resolution_note(session_clone).await {
                            tracing::error!("Failed to create resolution note: {}", e);
                        }
                    });

                    // Reset tracking
                    let mut sessions = self.sessions.write().await;
                    if let Some(s) = sessions.get_mut(session_id) {
                        s.resolution_in_progress = false;
                        s.last_error_command = None;
                    }
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Record an error message
    pub async fn record_error(&self, session_id: &str, message: &str, source: &str) -> Result<()> {
        let normalized = self.normalize_error_message(message);
        let fingerprint = self.create_error_fingerprint(&normalized);

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.errors.push(ErrorEntry {
                message: message.to_string(),
                normalized: normalized.clone(),
                source: source.to_string(),
                timestamp: SystemTime::now(),
                stderr_output: None,
            });
            session.last_activity = SystemTime::now();
        }

        // Update error patterns
        let mut patterns = self.error_patterns.write().await;
        if let Some(pattern) = patterns.get_mut(&fingerprint) {
            pattern.occurrences += 1;
            pattern.last_seen = SystemTime::now();
        } else {
            patterns.insert(
                fingerprint.clone(),
                ErrorPattern {
                    normalized_message: normalized,
                    fingerprint,
                    occurrences: 1,
                    last_seen: SystemTime::now(),
                    solutions: Vec::new(),
                },
            );
        }

        Ok(())
    }

    /// Check for known solutions to current errors
    pub async fn check_solutions(&self, session_id: &str) -> Result<Vec<Solution>> {
        let sessions = self.sessions.read().await;
        let patterns = self.error_patterns.read().await;

        if let Some(session) = sessions.get(session_id) {
            let mut solutions = Vec::new();

            for error in &session.errors {
                if let Some(pattern) =
                    patterns.get(&self.create_error_fingerprint(&error.normalized))
                {
                    for solution in &pattern.solutions {
                        solutions.push(Solution {
                            error_pattern: pattern.normalized_message.clone(),
                            solution: solution.clone(),
                            confidence: self.calculate_confidence(pattern),
                        });
                    }
                }
            }

            return Ok(solutions);
        }

        Ok(Vec::new())
    }

    /// Create a session for manual tracking
    pub async fn create_session(&self, working_dir: &str) -> Result<String> {
        let session_id = self.generate_session_id();

        let session = ActiveSession {
            id: session_id.clone(),
            start_time: SystemTime::now(),
            last_activity: SystemTime::now(),
            commands: Vec::new(),
            errors: Vec::new(),
            context: SessionContext {
                working_directory: working_dir.to_string(),
                git_branch: self.get_git_branch(working_dir).await.ok(),
                git_status: self.get_git_status(working_dir).await.ok(),
                environment: self.get_environment_context().await,
            },
            last_error_command: None,
            resolution_in_progress: false,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        Ok(session_id)
    }

    /// Get session data for manual recording
    pub async fn get_session_data(&self, session_id: &str) -> Result<Option<ActiveSession>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).cloned())
    }

    /// End a session and generate summary
    pub async fn end_session(&self, session_id: &str, success: bool) -> Result<Option<Note>> {
        let mut sessions = self.sessions.write().await;

        if let Some(session) = sessions.remove(session_id) {
            if success && !session.errors.is_empty() {
                // Generate a note from the session
                return Ok(Some(self.generate_session_note(&session).await?));
            }
        }

        Ok(None)
    }

    // Private helper methods

    async fn cleanup_sessions(
        sessions: &Arc<RwLock<HashMap<String, ActiveSession>>>,
        config: &DaemonConfig,
    ) {
        let mut sessions_guard = sessions.write().await;
        let now = SystemTime::now();

        // Optimize: collect keys to remove first to avoid borrow checker issues
        let mut to_remove = Vec::new();
        let mut session_activities: Vec<(String, SystemTime)> = Vec::new();

        for (id, session) in sessions_guard.iter() {
            if now
                .duration_since(session.last_activity)
                .unwrap_or(Duration::from_secs(0))
                >= config.session_timeout
            {
                to_remove.push(id.clone());
            } else {
                session_activities.push((id.clone(), session.last_activity));
            }
        }

        // Remove timed out sessions
        for id in to_remove {
            sessions_guard.remove(&id);
        }

        // Limit number of sessions efficiently
        if sessions_guard.len() > config.max_sessions {
            session_activities.sort_by_key(|(_, last_activity)| *last_activity);
            let to_remove_count = session_activities.len() - config.max_sessions;
            for (id, _) in session_activities.iter().take(to_remove_count) {
                sessions_guard.remove(id);
            }
        }
    }

    async fn analyze_error_patterns(patterns: &Arc<RwLock<HashMap<String, ErrorPattern>>>) {
        let patterns_guard = patterns.read().await;

        for (_, pattern) in patterns_guard.iter() {
            if pattern.occurrences > 5 {
                debug!(
                    "Frequent error pattern: {} ({} occurrences)",
                    pattern.normalized_message, pattern.occurrences
                );
            }
        }
    }

    fn create_error_fingerprint(&self, normalized: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn calculate_confidence(&self, pattern: &ErrorPattern) -> f64 {
        // Simple confidence calculation based on occurrence count
        (pattern.occurrences as f64).min(10.0) / 10.0
    }

    async fn analyze_command_error(
        &self,
        session: &mut ActiveSession,
        command: &str,
        exit_code: i32,
    ) {
        // Analyze if this command failure is part of a known error pattern
        let error_message = format!("Command '{}' failed with exit code {}", command, exit_code);
        let error_entry = ErrorEntry {
            message: error_message.clone(),
            normalized: self.normalize_error_message(&format!("Command failed: {}", command)),
            source: "command".to_string(),
            timestamp: SystemTime::now(),
            stderr_output: None,
        };

        session.errors.push(error_entry);

        // Send notification
        if let Some(ref notif_mgr) = self.notification_manager {
            let _ = notif_mgr.notify_error(command, &error_message);
        }
    }

    async fn get_git_branch(&self, working_dir: &str) -> Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(working_dir)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get git branch"))
        }
    }

    async fn get_git_status(&self, working_dir: &str) -> Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(working_dir)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get git status"))
        }
    }

    async fn get_environment_context(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Capture relevant environment variables
        for (key, value) in std::env::vars() {
            if key.starts_with("FUKURA_")
                || key == "PATH"
                || key == "SHELL"
                || key == "USER"
                || key == "HOME"
            {
                env.insert(key, value);
            }
        }

        env
    }

    fn generate_session_id(&self) -> String {
        use sha2::{Digest, Sha256};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut hasher = Sha256::new();
        hasher.update(timestamp.to_string().as_bytes());
        format!("session_{:x}", hasher.finalize())
    }

    async fn generate_session_note(&self, session: &ActiveSession) -> Result<Note> {
        let title = self.generate_session_title(session);
        let body = self.generate_session_body(session);
        let tags = self.generate_session_tags(session);

        let now = chrono::Utc::now();
        let author = Author {
            name: std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
            email: Some(std::env::var("EMAIL").unwrap_or_default()),
        };

        Ok(Note {
            title,
            body,
            tags,
            links: Vec::new(),
            meta: std::collections::BTreeMap::new(),
            solutions: Vec::new(),
            privacy: Privacy::Private,
            created_at: now,
            updated_at: now,
            author,
        })
    }

    fn generate_session_title(&self, session: &ActiveSession) -> String {
        if let Some(last_error) = session.errors.last() {
            format!("Solution for: {}", last_error.normalized)
        } else {
            format!(
                "Session from {}",
                session
                    .start_time
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            )
        }
    }

    fn generate_session_body(&self, session: &ActiveSession) -> String {
        let mut body = String::new();

        body.push_str("# Problem\n\n");
        for error in &session.errors {
            body.push_str(&format!("- {}\n", error.message));
        }

        body.push_str("\n# Solution Steps\n\n");
        for (i, cmd) in session.commands.iter().enumerate() {
            if cmd.exit_code == Some(0) || i == session.commands.len() - 1 {
                body.push_str(&format!("{}. {}\n", i + 1, cmd.command));
            }
        }

        body.push_str(&format!(
            "\n# Context\n\n- Working Directory: {}\n",
            session.context.working_directory
        ));
        if let Some(branch) = &session.context.git_branch {
            body.push_str(&format!("- Git Branch: {}\n", branch));
        }

        body
    }

    fn generate_session_tags(&self, session: &ActiveSession) -> Vec<String> {
        let mut tags = vec!["auto-generated".to_string(), "session".to_string()];

        // Add tags based on commands used
        for cmd in &session.commands {
            if cmd.command.contains("npm") || cmd.command.contains("yarn") {
                tags.push("javascript".to_string());
            } else if cmd.command.contains("cargo") || cmd.command.contains("rust") {
                tags.push("rust".to_string());
            } else if cmd.command.contains("docker") {
                tags.push("docker".to_string());
            } else if cmd.command.contains("git") {
                tags.push("git".to_string());
            }
        }

        tags.sort();
        tags.dedup();
        tags
    }

    async fn load_error_patterns(&self) -> Result<()> {
        // Load error patterns from the repository
        // This would typically read from a patterns file in .fukura/
        debug!("Loading error patterns...");
        Ok(())
    }

    async fn save_error_patterns(&self) -> Result<()> {
        // Save error patterns to the repository
        debug!("Saving error patterns...");
        Ok(())
    }

    /// Normalize error messages by replacing paths and line numbers
    pub fn normalize_error_message(&self, error: &str) -> String {
        let mut normalized = error.to_string();

        // Replace file paths with generic paths
        let path_regex = regex::Regex::new(r"/[^\s:]+\.rs").unwrap();
        normalized = path_regex
            .replace_all(&normalized, "/path/to/file")
            .to_string();

        // Keep line numbers as they are (don't normalize them)
        normalized
    }

    /// Auto-generate notes from completed sessions with errors
    async fn auto_generate_notes(
        sessions: &Arc<RwLock<HashMap<String, ActiveSession>>>,
        repo: &Arc<FukuraRepo>,
        repo_path: &std::path::Path,
    ) {
        let sessions_guard = sessions.read().await;
        let now = SystemTime::now();
        let timeout = Duration::from_secs(300); // 5 minutes

        for (session_id, session) in sessions_guard.iter() {
            // Check if session has been inactive for timeout period and has errors
            if now
                .duration_since(session.last_activity)
                .unwrap_or_default()
                > timeout
            {
                let has_errors = session.commands.iter().any(|cmd| cmd.exit_code != Some(0));

                if has_errors {
                    info!(
                        "Auto-generating note for session {} with errors",
                        session_id
                    );

                    // Create note from session
                    let note = Self::create_note_from_session_data(session, repo_path);

                    // Store the note
                    if let Ok(_record) = repo.store_note(note) {
                        info!("Auto-generated note for session {}", session_id);
                    }
                }
            }
        }
    }

    /// Create instant resolution note when error is solved (WORLD-CLASS)
    async fn create_instant_resolution_note(session: ActiveSession) -> Result<()> {
        // Discover repo
        let repo = match FukuraRepo::discover(Some(std::path::Path::new(
            &session.context.working_directory,
        ))) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };

        // Get recent commands (last 10 or until error)
        let recent_commands: Vec<_> = session.commands.iter().rev().take(10).collect();

        // Find error commands and solution commands
        let mut error_cmd = None;
        let mut solution_steps = Vec::new();

        for cmd in recent_commands.iter().rev() {
            if let Some(code) = cmd.exit_code {
                if code != 0 && error_cmd.is_none() {
                    error_cmd = Some(cmd);
                } else if code == 0 && error_cmd.is_some() {
                    solution_steps.push(cmd);
                }
            }
        }

        let error = match error_cmd {
            Some(e) => e,
            None => return Ok(()), // No error found
        };

        // Generate title
        let title = format!("Solved: {}", error.command);

        // Generate body
        let mut body = String::new();
        body.push_str("## 🎯 Problem Solved\n\n");
        body.push_str("### ❌ Error Encountered\n\n");
        body.push_str(&format!("```bash\n$ {}\n", error.command));

        // Add stderr if available
        if let Some(error_entry) = session.errors.last() {
            if let Some(ref stderr) = error_entry.stderr_output {
                body.push_str(&format!("\n# Error output:\n{}\n", stderr));
            }
        }

        body.push_str("```\n");
        body.push_str(&format!("Exit code: {}\n\n", error.exit_code.unwrap_or(1)));

        if !solution_steps.is_empty() {
            body.push_str("### ✅ Solution Steps (Auto-detected)\n\n");
            for (idx, cmd) in solution_steps.iter().enumerate() {
                body.push_str(&format!("{}. `{}`\n", idx + 1, cmd.command));
            }
            body.push('\n');
        }

        body.push_str("### 📋 Recent Command History\n\n");
        for cmd in recent_commands.iter().rev() {
            let status = match cmd.exit_code {
                Some(0) => "✅",
                Some(_) => "❌",
                None => "⏳",
            };
            body.push_str(&format!("{} `{}`\n", status, cmd.command));
        }

        if let Some(ref branch) = session.context.git_branch {
            body.push_str(&format!("\n**Git Branch**: `{}`\n", branch));
        }

        // Extract tags
        let mut tags = vec!["auto-solved".to_string(), "resolution".to_string()];
        let cmd_lower = error.command.to_lowercase();
        if cmd_lower.contains("cargo") || cmd_lower.contains("rust") {
            tags.push("rust".to_string());
        }
        if cmd_lower.contains("npm") || cmd_lower.contains("node") {
            tags.push("nodejs".to_string());
        }
        if cmd_lower.contains("docker") {
            tags.push("docker".to_string());
        }
        if cmd_lower.contains("git") {
            tags.push("git".to_string());
        }
        if cmd_lower.contains("python") || cmd_lower.contains("pip") {
            tags.push("python".to_string());
        }
        tags.sort();
        tags.dedup();

        let note = Note {
            title: title.clone(),
            body,
            tags,
            links: vec![],
            meta: std::collections::BTreeMap::from([
                ("auto-resolution".to_string(), "true".to_string()),
                ("error_command".to_string(), error.command.clone()),
                (
                    "solution_steps".to_string(),
                    solution_steps.len().to_string(),
                ),
            ]),
            solutions: vec![],
            privacy: Privacy::Private,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            author: Author {
                name: std::env::var("USER").unwrap_or_else(|_| "auto".to_string()),
                email: None,
            },
        };

        match repo.store_note(note) {
            Ok(record) => {
                tracing::info!(
                    "✨ Auto-resolution note created: {} ({})",
                    title,
                    &record.object_id[..8]
                );

                // Send success notification
                if let Ok(notif) = NotificationManager::new(repo.root()) {
                    let _summary = "Fukura: Problem Solved! 🎉";
                    let body_text = format!(
                        "Error: {}\n\nSolved with {} step(s)\n\nView: fuku view @latest",
                        error.command,
                        solution_steps.len()
                    );
                    let _ = notif.notify_solution_found(&body_text, solution_steps.len());
                }

                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to create resolution note: {}", e);
                Err(e)
            }
        }
    }

    /// Find similar solutions for an error (WORLD-CLASS)
    fn find_similar_solutions(
        repo: &Arc<FukuraRepo>,
        command: &str,
        _exit_code: i32,
    ) -> Result<Vec<SolutionHit>> {
        // Search for similar commands in past notes
        let query = Self::extract_search_terms(command);

        match repo.search(&query, 10, crate::index::SearchSort::Relevance) {
            Ok(hits) => {
                let mut solutions = Vec::new();
                for hit in hits {
                    // Check if note has solutions or is marked as resolved
                    if let Ok(record) = repo.load_note(&hit.object_id) {
                        // Check for solution indicators
                        let has_solution = record.note.tags.contains(&"solved".to_string())
                            || record.note.tags.contains(&"solution".to_string())
                            || record.note.body.to_lowercase().contains("solution:")
                            || record.note.body.to_lowercase().contains("fix:")
                            || record.note.body.to_lowercase().contains("resolved")
                            || !record.note.solutions.is_empty();

                        if has_solution {
                            solutions.push(SolutionHit {
                                note_id: record.object_id.clone(),
                                title: record.note.title.clone(),
                                snippet: Self::extract_solution_snippet(&record.note.body),
                                confidence: hit.likes as f64 / 10.0, // Use likes as confidence indicator
                            });
                        }
                    }
                }
                Ok(solutions)
            }
            Err(e) => Err(e),
        }
    }

    /// Extract search terms from command (remove noise words)
    fn extract_search_terms(command: &str) -> String {
        let noise_words = ["cd", "ls", "cat", "echo", "mkdir", "rm", "cp", "mv"];
        let words: Vec<&str> = command
            .split_whitespace()
            .filter(|w| !noise_words.contains(w) && !w.starts_with('-'))
            .take(3) // Use first 3 meaningful words
            .collect();
        words.join(" ")
    }

    /// Extract solution snippet from note body
    fn extract_solution_snippet(body: &str) -> String {
        // Look for solution markers
        for marker in ["## Solution", "## Fix", "**Solution**:", "**Fix**:"] {
            if let Some(idx) = body.find(marker) {
                let start = idx + marker.len();
                let snippet = &body[start..];
                if let Some(end_idx) = snippet.find('\n') {
                    return snippet[..end_idx.min(200)].trim().to_string();
                }
                return snippet[..snippet.len().min(200)].trim().to_string();
            }
        }

        // Fallback: first 150 chars
        body.chars().take(150).collect()
    }

    /// Create a note from session data
    fn create_note_from_session_data(
        session: &ActiveSession,
        _repo_path: &std::path::Path,
    ) -> Note {
        let mut body = format!("## Session: {}\n\n", session.id);

        for (i, cmd) in session.commands.iter().enumerate() {
            body.push_str(&format!("### Step {}: {}\n", i + 1, cmd.command));
            if cmd.exit_code != Some(0) {
                body.push_str(&format!(
                    "**Error (exit code: {})**: Command failed\n\n",
                    cmd.exit_code.unwrap_or(-1)
                ));
            } else {
                body.push_str(" Success\n\n");
            }
        }

        body.push_str("## Generated by Fukura Daemon\n");
        body.push_str(
            "This note was automatically generated from a development session with errors.\n",
        );

        Note {
            title: format!("Auto-generated: Session {}", session.id),
            body,
            tags: vec!["auto-generated".into(), "session".into(), "error".into()],
            links: vec![],
            meta: std::collections::BTreeMap::new(),
            solutions: vec![],
            privacy: Privacy::Private,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            author: Author {
                name: "Fukura Daemon".into(),
                email: None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Solution {
    pub error_pattern: String,
    pub solution: String,
    pub confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_daemon_creation() {
        let temp_dir = TempDir::new().unwrap();
        let _repo = FukuraRepo::init(temp_dir.path(), true).unwrap();

        let config = DaemonConfig::default();
        let daemon = FukuraDaemon::new(temp_dir.path(), config);

        assert!(daemon.is_ok());
    }

    #[tokio::test]
    async fn test_error_normalization() {
        let temp_dir = TempDir::new().unwrap();
        let _repo = FukuraRepo::init(temp_dir.path(), true).unwrap();

        let config = DaemonConfig::default();
        let daemon = FukuraDaemon::new(temp_dir.path(), config).unwrap();

        let normalized = daemon
            .normalize_error_message("Error: /home/user/project/src/main.rs:42:5: expected `;`");

        assert!(normalized.contains("/path/to/file"));
        assert!(normalized.contains("42:5"));
    }
}
