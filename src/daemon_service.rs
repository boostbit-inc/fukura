use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;

use crate::daemon::{FukuraDaemon, DaemonConfig};
use crate::repo::FukuraRepo;

/// Background daemon service management
pub struct DaemonService {
    repo_path: std::path::PathBuf,
}

impl DaemonService {
    pub fn new(repo_path: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
        }
    }

    /// Start the daemon as a background service
    pub fn start_background(&self) -> Result<()> {
        if cfg!(target_os = "windows") {
            self.start_windows_service()
        } else {
            self.start_unix_service()
        }
    }

    /// Stop the background daemon
    pub async fn stop_background(&self) -> Result<()> {
        let pid_file = self.get_pid_file_path();
        
        if !pid_file.exists() {
            return Ok(()); // Already stopped
        }

        let pid = fs::read_to_string(&pid_file).await?;
        
        if cfg!(target_os = "windows") {
            Command::new("taskkill")
                .args(&["/F", "/PID", &pid])
                .output()?;
        } else {
            Command::new("kill")
                .arg(&pid)
                .output()?;
        }

        fs::remove_file(&pid_file).await?;
        Ok(())
    }

    /// Check if daemon is running
    pub async fn is_running(&self) -> bool {
        let pid_file = self.get_pid_file_path();
        
        if !pid_file.exists() {
            return false;
        }

        let pid = match fs::read_to_string(&pid_file).await {
            Ok(pid) => pid.trim().to_string(),
            Err(_) => return false,
        };

        if cfg!(target_os = "windows") {
            let output = Command::new("tasklist")
                .args(&["/FI", &format!("PID eq {}", pid)])
                .output()
                .unwrap_or_else(|_| std::process::Output {
                    status: std::process::ExitStatus::default(),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                });
            
            String::from_utf8_lossy(&output.stdout).contains(&pid)
        } else {
            // Parse PID and use improved process check
            if let Ok(pid_num) = pid.parse::<u32>() {
                self.is_process_running(pid_num)
            } else {
                false
            }
        }
    }

    /// Start daemon as Unix service (using nohup with proper background execution)
    fn start_unix_service(&self) -> Result<()> {
        let exe_path = std::env::current_exe()?;
        let daemon_dir = self.repo_path.join(".fukura");
        let pid_file = self.get_pid_file_path();
        let log_file = daemon_dir.join("daemon.log");

        // Create daemon directory if it doesn't exist
        std::fs::create_dir_all(&daemon_dir)?;

        // Start daemon in background using nohup with proper process detachment
        let mut cmd = Command::new("nohup");
        cmd.arg(&exe_path)
            .args(&["daemon", "--foreground"])
            .current_dir(&self.repo_path)
            .stdout(Stdio::from(std::fs::File::create(&log_file)?))
            .stderr(Stdio::from(std::fs::File::create(&log_file)?))
            .stdin(Stdio::null());

        let child = cmd.spawn()?;
        
        // Write PID file and detach from parent process
        let pid = child.id();
        std::fs::write(&pid_file, pid.to_string())?;
        
        // Detach the child process to prevent zombie processes
        drop(child);

        // Give the daemon time to start and verify it's running
        std::thread::sleep(Duration::from_millis(1000));
        
        if !self.is_process_running(pid) {
            // Clean up PID file if process failed to start
            let _ = std::fs::remove_file(&pid_file);
            return Err(anyhow::anyhow!("Failed to start daemon process"));
        }

        println!("Daemon started successfully with PID: {}", pid);

        Ok(())
    }

    /// Check if a process is running by PID (Unix)
    fn is_process_running(&self, pid: u32) -> bool {
        let output = Command::new("ps")
            .args(&["-p", &pid.to_string()])
            .output()
            .unwrap_or_else(|_| std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        
        output.status.success()
    }

    /// Start daemon as Windows service
    fn start_windows_service(&self) -> Result<()> {
        let exe_path = std::env::current_exe()?;
        let daemon_dir = self.repo_path.join(".fukura");
        let pid_file = self.get_pid_file_path();
        let log_file = daemon_dir.join("daemon.log");

        // Create daemon directory if it doesn't exist
        std::fs::create_dir_all(&daemon_dir)?;

        // Start daemon in background using PowerShell for better control
        let mut cmd = Command::new("powershell");
        cmd.args(&[
            "-Command", 
            &format!(
                "Start-Process -FilePath '{}' -ArgumentList 'daemon' -WindowStyle Hidden -PassThru | Select-Object -ExpandProperty Id | Out-File -FilePath '{}' -Encoding ASCII",
                exe_path.to_string_lossy(),
                pid_file.to_string_lossy()
            )
        ])
        .current_dir(&self.repo_path);

        let _child = cmd.spawn()?;
        
        // Give the daemon a moment to start and write its PID
        std::thread::sleep(Duration::from_millis(500));

        Ok(())
    }

    /// Get the PID file path
    pub fn get_pid_file_path(&self) -> std::path::PathBuf {
        self.repo_path.join(".fukura").join("daemon.pid")
    }

    /// Auto-start daemon when directory is opened
    pub async fn auto_start_if_needed(&self) -> Result<()> {
        if !self.is_running().await {
            self.start_background()?;
        }
        Ok(())
    }
}

/// Enhanced daemon with automatic note generation
pub struct AutoNoteDaemon {
    daemon: FukuraDaemon,
    auto_note_threshold: Duration,
}

impl AutoNoteDaemon {
    pub fn new(repo_path: &Path, config: DaemonConfig) -> Result<Self> {
        let daemon = FukuraDaemon::new(repo_path, config)?;
        
        Ok(Self {
            daemon,
            auto_note_threshold: Duration::from_secs(300), // 5 minutes
        })
    }

    /// Start the enhanced daemon with automatic note generation
    pub async fn start(&self) -> Result<()> {
        // Start the base daemon
        self.daemon.start().await?;

        // Start automatic note generation task
        let sessions = self.daemon.sessions.clone();
        let repo_path = self.daemon.repo_path.clone();
        let repo = Arc::new(FukuraRepo::discover(Some(&repo_path))?);
        let threshold = self.auto_note_threshold;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                Self::generate_notes_from_sessions(&sessions, &repo, threshold).await;
            }
        });

        Ok(())
    }

    /// Generate notes from completed sessions
    async fn generate_notes_from_sessions(
        sessions: &Arc<tokio::sync::RwLock<std::collections::HashMap<String, crate::daemon::ActiveSession>>>,
        repo: &Arc<FukuraRepo>,
        threshold: Duration,
    ) {
        let mut sessions_guard = sessions.write().await;
        let now = std::time::SystemTime::now();
        
        let mut completed_sessions = Vec::new();
        
        // Find sessions that have been inactive for the threshold duration
        for (id, session) in sessions_guard.iter() {
            if now.duration_since(session.last_activity).unwrap_or_default() > threshold && !session.errors.is_empty() {
                completed_sessions.push(id.clone());
            }
        }
        
        // Generate notes for completed sessions
        for session_id in completed_sessions {
            if let Some(session) = sessions_guard.remove(&session_id) {
                if let Ok(note) = Self::create_note_from_session(&session).await {
                    if let Err(e) = repo.store_note(note) {
                        eprintln!("Failed to store auto-generated note: {}", e);
                    }
                }
            }
        }
    }

    /// Create a note from a completed session
    async fn create_note_from_session(session: &crate::daemon::ActiveSession) -> Result<crate::models::Note> {
        let title = Self::generate_title_from_session(session);
        let body = Self::generate_body_from_session(session);
        let tags = Self::generate_tags_from_session(session);
        
        let now = chrono::Utc::now();
        let author = crate::models::Author {
            name: std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
            email: Some(std::env::var("EMAIL").unwrap_or_default()),
        };

        Ok(crate::models::Note {
            title,
            body,
            tags,
            links: Vec::new(),
            meta: std::collections::BTreeMap::new(),
            solutions: Vec::new(),
            privacy: crate::models::Privacy::Private,
            created_at: now,
            updated_at: now,
            author,
        })
    }

    fn generate_title_from_session(session: &crate::daemon::ActiveSession) -> String {
        if let Some(last_error) = session.errors.last() {
            // Extract key error information for title
            let error_words: Vec<&str> = last_error.normalized.split_whitespace().take(5).collect();
            format!("Auto-captured: {}", error_words.join(" "))
        } else {
            format!("Auto-captured session from {}", session.start_time.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())
        }
    }

    fn generate_body_from_session(session: &crate::daemon::ActiveSession) -> String {
        let mut body = String::new();
        
        body.push_str("# Auto-Captured Error Session\n\n");
        body.push_str(&format!("**Session Duration:** {} seconds\n", 
            session.last_activity.duration_since(session.start_time).unwrap_or_default().as_secs()));
        body.push_str(&format!("**Working Directory:** {}\n\n", session.context.working_directory));
        
        if let Some(branch) = &session.context.git_branch {
            body.push_str(&format!("**Git Branch:** {}\n\n", branch));
        }
        
        // Add errors
        if !session.errors.is_empty() {
            body.push_str("## Errors Encountered\n\n");
            for error in &session.errors {
                body.push_str(&format!("- **{}**: {}\n", error.source, error.message));
            }
            body.push_str("\n");
        }
        
        // Add successful commands (solution steps)
        let successful_commands: Vec<_> = session.commands.iter()
            .filter(|cmd| cmd.exit_code == Some(0))
            .collect();
            
        if !successful_commands.is_empty() {
            body.push_str("## Solution Steps\n\n");
            for (i, cmd) in successful_commands.iter().enumerate() {
                body.push_str(&format!("{}. `{}`\n", i + 1, cmd.command));
            }
            body.push_str("\n");
        }
        
        // Add all commands for context
        body.push_str("## All Commands\n\n");
        for (i, cmd) in session.commands.iter().enumerate() {
            let status = if cmd.exit_code == Some(0) { "✅" } else { "❌" };
            body.push_str(&format!("{}. {} `{}`\n", i + 1, status, cmd.command));
        }
        
        body
    }

    fn generate_tags_from_session(session: &crate::daemon::ActiveSession) -> Vec<String> {
        let mut tags = vec!["auto-captured".to_string(), "session".to_string()];
        
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
            } else if cmd.command.contains("python") || cmd.command.contains("pip") {
                tags.push("python".to_string());
            }
        }
        
        // Add tags based on errors
        for error in &session.errors {
            if error.message.to_lowercase().contains("permission") {
                tags.push("permissions".to_string());
            } else if error.message.to_lowercase().contains("network") || error.message.to_lowercase().contains("connection") {
                tags.push("network".to_string());
            } else if error.message.to_lowercase().contains("memory") {
                tags.push("memory".to_string());
            }
        }
        
        tags.sort();
        tags.dedup();
        tags
    }
}

/// Start background daemon (called from CLI)
pub fn start_background_daemon(repo: &FukuraRepo) -> Result<()> {
    let daemon_service = DaemonService::new(repo.root());
    daemon_service.start_background()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_daemon_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let service = DaemonService::new(temp_dir.path());
        assert!(!service.is_running().await);
    }

    #[test]
    fn test_auto_note_daemon_creation() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FukuraRepo::init(temp_dir.path(), true).unwrap();
        let config = DaemonConfig::default();
        let daemon = AutoNoteDaemon::new(repo.root(), config);
        assert!(daemon.is_ok());
    }
}
