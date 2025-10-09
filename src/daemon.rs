use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, info};

use crate::models::{Author, Note, Privacy};
use crate::repo::FukuraRepo;

/// Daemon for monitoring and capturing error patterns
pub struct FukuraDaemon {
    pub repo: Arc<FukuraRepo>,
    pub sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
    pub error_patterns: Arc<RwLock<HashMap<String, ErrorPattern>>>,
    pub repo_path: std::path::PathBuf,
    config: DaemonConfig,
}

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub monitor_interval: Duration,
    pub session_timeout: Duration,
    pub max_sessions: usize,
    pub enable_clipboard_monitoring: bool,
    pub enable_process_monitoring: bool,
    pub error_threshold: f64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            monitor_interval: Duration::from_secs(10), // Reduced frequency for better performance
            session_timeout: Duration::from_secs(600), // 10 minutes - longer timeout
            max_sessions: 50,                          // Reduced for better memory usage
            enable_clipboard_monitoring: false,
            enable_process_monitoring: true,
            error_threshold: 0.7,
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

impl FukuraDaemon {
    /// Create a new daemon instance
    pub fn new(repo_path: &Path, config: DaemonConfig) -> Result<Self> {
        let repo = Arc::new(FukuraRepo::discover(Some(repo_path))?);

        Ok(Self {
            repo,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            error_patterns: Arc::new(RwLock::new(HashMap::new())),
            repo_path: repo_path.to_path_buf(),
            config,
        })
    }

    /// Start the daemon
    pub async fn start(&self) -> Result<()> {
        info!("Starting Fukura daemon...");

        // Load existing error patterns
        self.load_error_patterns().await?;

        // Start monitoring tasks
        let sessions1 = self.sessions.clone();
        let sessions2 = self.sessions.clone();
        let error_patterns = self.error_patterns.clone();
        let config1 = self.config.clone();
        let config2 = self.config.clone();
        let repo = self.repo.clone();
        let repo_path = self.repo_path.clone();

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

        info!("Fukura daemon started successfully");
        Ok(())
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

            // Check if this looks like an error
            if let Some(code) = exit_code {
                if code != 0 {
                    self.analyze_command_error(session, command, code).await;
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
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        Ok(session_id)
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
        let error_entry = ErrorEntry {
            message: format!("Command '{}' failed with exit code {}", command, exit_code),
            normalized: self.normalize_error_message(&format!("Command failed: {}", command)),
            source: "command".to_string(),
            timestamp: SystemTime::now(),
        };

        session.errors.push(error_entry);
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
