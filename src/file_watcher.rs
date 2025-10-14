use anyhow::Result;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::activity::{Activity, FileChangeActivity, FileChangeType};

/// File system watcher for tracking file changes
pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    activity_tx: mpsc::Sender<Activity>,
    watch_paths: Vec<PathBuf>,
    exclude_patterns: Vec<String>,
    max_file_size: u64, // Maximum file size to process (bytes)
}

impl FileWatcher {
    pub fn new(activity_tx: mpsc::Sender<Activity>, watch_paths: Vec<PathBuf>) -> Self {
        Self {
            watcher: None,
            activity_tx,
            watch_paths,
            exclude_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "__pycache__".to_string(),
                ".fukura".to_string(),
            ],
            max_file_size: 100 * 1024, // 100KB default
        }
    }

    pub fn with_max_file_size(mut self, size_bytes: u64) -> Self {
        self.max_file_size = size_bytes;
        self
    }

    pub fn with_exclusions(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = patterns;
        self
    }

    /// Start watching for file changes
    pub async fn start_watching(&mut self, session_id: String) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            Config::default(),
        )?;

        // Watch all specified paths
        for path in &self.watch_paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)?;
                debug!("Watching path: {:?}", path);
            }
        }

        self.watcher = Some(watcher);

        // Spawn event processor
        let activity_tx = self.activity_tx.clone();
        let exclude_patterns = self.exclude_patterns.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(e) =
                    Self::process_event(event, &activity_tx, &session_id, &exclude_patterns).await
                {
                    warn!("Error processing file event: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Process file system event
    async fn process_event(
        event: Event,
        tx: &mpsc::Sender<Activity>,
        session_id: &str,
        exclude_patterns: &[String],
    ) -> Result<()> {
        // Filter out excluded paths
        for path in &event.paths {
            if Self::should_exclude(path, exclude_patterns) {
                return Ok(());
            }
        }

        let change_type = match event.kind {
            EventKind::Create(_) => FileChangeType::Created,
            EventKind::Modify(_) => FileChangeType::Modified,
            EventKind::Remove(_) => FileChangeType::Deleted,
            _ => return Ok(()), // Ignore other events
        };

        for path in event.paths {
            if !path.is_file() {
                continue;
            }

            let file_activity = FileChangeActivity::new(path.clone(), change_type.clone());

            // Calculate file size and skip if too large
            let file_activity = if let Ok(metadata) = std::fs::metadata(&path) {
                let mut activity = file_activity;
                activity.size_bytes = metadata.len();

                // Skip files that are too large to avoid memory issues
                if activity.size_bytes > 100 * 1024 {
                    // 100KB limit
                    debug!(
                        "Skipping large file: {:?} ({} bytes)",
                        path, activity.size_bytes
                    );
                    continue;
                }

                activity
            } else {
                file_activity
            };

            let activity = Activity::file_change(session_id.to_string(), file_activity);

            // Use try_send to avoid blocking
            if let Err(e) = tx.try_send(activity) {
                debug!("Failed to send file activity: {}", e);
            } else {
                debug!("Recorded file change: {:?}", path);
            }
        }

        Ok(())
    }

    fn should_exclude(path: &Path, exclude_patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in exclude_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        // Exclude hidden files
        if let Some(file_name) = path.file_name() {
            if file_name.to_string_lossy().starts_with('.') {
                return true;
            }
        }

        false
    }

    /// Calculate diff for a file change (if possible)
    pub async fn calculate_diff(
        &self,
        _path: &Path,
        change_type: &FileChangeType,
    ) -> Option<(String, usize, usize)> {
        // Only calculate diff for modifications
        if !matches!(change_type, FileChangeType::Modified) {
            return None;
        }

        // TODO: Implement diff calculation
        // This would integrate with git diff or implement a simple line-based diff
        // For now, return None

        None
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        if let Some(_watcher) = self.watcher.take() {
            debug!("File watcher stopped");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_watcher_creation() {
        let (tx, _rx) = mpsc::channel(10);
        let watcher = FileWatcher::new(tx, vec![PathBuf::from("/tmp")]);
        assert_eq!(watcher.watch_paths.len(), 1);
    }

    #[test]
    fn test_exclusion_patterns() {
        assert!(FileWatcher::should_exclude(
            Path::new("/tmp/project/node_modules/file.js"),
            &["node_modules".to_string()]
        ));

        assert!(FileWatcher::should_exclude(
            Path::new("/tmp/project/.git/config"),
            &[".git".to_string()]
        ));

        assert!(!FileWatcher::should_exclude(
            Path::new("/tmp/project/src/main.rs"),
            &["node_modules".to_string(), ".git".to_string()]
        ));
    }

    #[test]
    fn test_hidden_file_exclusion() {
        assert!(FileWatcher::should_exclude(Path::new("/tmp/.hidden"), &[]));
    }
}
