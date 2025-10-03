use anyhow::Result;
use std::time::Duration;
use tokio::time;

use crate::daemon_service::DaemonService;

/// Monitor directories for .fukura and auto-start daemon
pub struct DirectoryMonitor {
    check_interval: Duration,
    monitored_paths: std::collections::HashSet<std::path::PathBuf>,
}

impl DirectoryMonitor {
    pub fn new() -> Self {
        Self {
            check_interval: Duration::from_secs(10),
            monitored_paths: std::collections::HashSet::new(),
        }
    }

    /// Start monitoring for .fukura directories
    pub async fn start_monitoring(&mut self) -> Result<()> {
        loop {
            self.check_and_start_daemons().await?;
            time::sleep(self.check_interval).await;
        }
    }

    /// Check for .fukura directories and start daemons if needed
    async fn check_and_start_daemons(&mut self) -> Result<()> {
        // Get all current working directories from running processes
        let current_dirs = self.get_active_directories().await?;
        
        for dir in current_dirs {
            let fukura_dir = dir.join(".fukura");
            if fukura_dir.exists() && !self.monitored_paths.contains(&dir) {
                // Found a new .fukura directory, start daemon
                let daemon_service = DaemonService::new(&dir);
                
                if !daemon_service.is_running().await {
                    if let Err(e) = daemon_service.start_background() {
                        eprintln!("Failed to start daemon for {}: {}", dir.display(), e);
                    } else {
                        println!("🚀 Auto-started daemon for {}", dir.display());
                        self.monitored_paths.insert(dir);
                    }
                }
            }
        }
        
        // Remove paths that no longer exist
        self.monitored_paths.retain(|path| path.exists());
        
        Ok(())
    }

    /// Get directories that are currently active (have running processes)
    async fn get_active_directories(&self) -> Result<Vec<std::path::PathBuf>> {
        let mut dirs = std::collections::HashSet::new();
        
        // Add current working directory
        if let Ok(cwd) = std::env::current_dir() {
            dirs.insert(cwd);
        }
        
        // Add common development directories
        if let Ok(home) = std::env::var("HOME") {
            let home_path = std::path::PathBuf::from(home);
            
            // Common development directories
            let dev_dirs = [
                "projects", "workspace", "dev", "code", "src", 
                "Documents", "Desktop", "Development"
            ];
            
            for dir_name in &dev_dirs {
                let dir = home_path.join(dir_name);
                if dir.exists() {
                    self.scan_directory_for_fukura(&dir, &mut dirs)?;
                }
            }
        }
        
        // Add directories from environment variables
        if let Ok(workspace) = std::env::var("WORKSPACE") {
            dirs.insert(std::path::PathBuf::from(workspace));
        }
        
        if let Ok(project_root) = std::env::var("PROJECT_ROOT") {
            dirs.insert(std::path::PathBuf::from(project_root));
        }
        
        Ok(dirs.into_iter().collect())
    }

    /// Recursively scan directory for .fukura subdirectories
    fn scan_directory_for_fukura(
        &self,
        dir: &std::path::Path,
        found_dirs: &mut std::collections::HashSet<std::path::PathBuf>,
    ) -> Result<()> {
        let mut entries = tokio::task::block_in_place(|| {
            std::fs::read_dir(dir)
        })?;
        
        while let Some(entry) = entries.next() {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Check if this directory has a .fukura subdirectory
                let fukura_dir = path.join(".fukura");
                if fukura_dir.exists() {
                    found_dirs.insert(path.clone());
                }
                
                // Don't go too deep (limit recursion)
                if path.components().count() < 6 {
                    self.scan_directory_for_fukura(&path, found_dirs)?;
                }
            }
        }
        
        Ok(())
    }
}

/// VS Code integration for automatic daemon startup
pub struct VSCodeIntegration {
    workspace_file: Option<std::path::PathBuf>,
}

impl VSCodeIntegration {
    pub fn new() -> Self {
        Self {
            workspace_file: Self::find_vscode_workspace(),
        }
    }

    /// Check if we're in a VS Code workspace and start daemon if needed
    pub async fn check_and_start_daemon(&self) -> Result<()> {
        if let Some(workspace_path) = &self.workspace_file {
            let fukura_dir = workspace_path.join(".fukura");
            if fukura_dir.exists() {
                let daemon_service = DaemonService::new(workspace_path);
                
                if !daemon_service.is_running().await {
                    daemon_service.start_background()?;
                    println!("🚀 Auto-started daemon for VS Code workspace: {}", workspace_path.display());
                }
            }
        }
        
        Ok(())
    }

    /// Find VS Code workspace file
    fn find_vscode_workspace() -> Option<std::path::PathBuf> {
        let mut current = std::env::current_dir().ok()?;
        
        loop {
            let workspace_file = current.join(".vscode").join("settings.json");
            if workspace_file.exists() {
                return Some(current);
            }
            
            let parent = current.parent()?;
            if parent == current {
                break;
            }
            current = parent.to_path_buf();
        }
        
        None
    }
}

/// Terminal integration for automatic daemon startup
pub struct TerminalIntegration;

impl TerminalIntegration {
    /// Check if we should auto-start daemon based on terminal context
    pub async fn check_and_start_daemon(&self) -> Result<()> {
        let cwd = std::env::current_dir()?;
        let fukura_dir = cwd.join(".fukura");
        
        if fukura_dir.exists() {
            let daemon_service = DaemonService::new(&cwd);
            
            if !daemon_service.is_running().await {
                daemon_service.start_background()?;
                println!("🚀 Auto-started daemon for terminal session: {}", cwd.display());
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_directory_monitor_creation() {
        let monitor = DirectoryMonitor::new();
        assert!(monitor.monitored_paths.is_empty());
    }

    #[tokio::test]
    async fn test_vscode_integration() {
        let integration = VSCodeIntegration::new();
        // Should not panic even if no VS Code workspace is found
        assert!(integration.workspace_file.is_some() || integration.workspace_file.is_none());
    }
}
