use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

use crate::activity::{Activity, ActivityFilter, ActivitySession};
use crate::performance::{PerformanceMetrics, RateLimiter};

/// Activity monitoring configuration
#[derive(Debug, Clone)]
pub struct ActivityMonitorConfig {
    pub enable_file_monitoring: bool,
    pub enable_clipboard_monitoring: bool,
    pub enable_app_monitoring: bool,
    pub max_clipboard_length: usize,
    pub max_file_size_kb: u64,
    pub max_activities_per_session: usize,
}

impl Default for ActivityMonitorConfig {
    fn default() -> Self {
        Self {
            enable_file_monitoring: true,
            enable_clipboard_monitoring: false, // Privacy: off by default
            enable_app_monitoring: true,
            max_clipboard_length: 1000,
            max_file_size_kb: 100,
            max_activities_per_session: 10000,
        }
    }
}

/// Central activity monitoring hub
pub struct ActivityMonitor {
    config: ActivityMonitorConfig,
    current_session: Arc<RwLock<Option<ActivitySession>>>,
    activity_tx: mpsc::Sender<Activity>,
    #[allow(dead_code)]
    activity_rx: Arc<RwLock<mpsc::Receiver<Activity>>>,
    filters: Vec<Box<dyn ActivityFilter>>,
    metrics: Arc<PerformanceMetrics>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

impl ActivityMonitor {
    pub fn new(config: ActivityMonitorConfig) -> Self {
        let (activity_tx, activity_rx) = mpsc::channel(1000);

        // Rate limiter: max 1000 activities per second to prevent overwhelming the system
        let rate_limiter = RateLimiter::new(1000, std::time::Duration::from_secs(1));

        Self {
            config,
            current_session: Arc::new(RwLock::new(None)),
            activity_tx,
            activity_rx: Arc::new(RwLock::new(activity_rx)),
            filters: Vec::new(),
            metrics: Arc::new(PerformanceMetrics::new()),
            rate_limiter: Arc::new(RwLock::new(rate_limiter)),
        }
    }

    pub fn add_filter(&mut self, filter: Box<dyn ActivityFilter>) {
        self.filters.push(filter);
    }

    /// Start a new activity session
    pub async fn start_session(&self, title: String) -> Result<String> {
        let mut session_guard = self.current_session.write().await;

        if session_guard.is_some() {
            anyhow::bail!("Session already in progress");
        }

        let session = ActivitySession::new(title);
        let session_id = session.id.clone();
        *session_guard = Some(session);

        info!("Started activity session: {}", session_id);
        Ok(session_id)
    }

    /// Stop current session and return it
    pub async fn stop_session(&self) -> Result<Option<ActivitySession>> {
        let mut session_guard = self.current_session.write().await;

        if let Some(mut session) = session_guard.take() {
            session.finish();
            info!("Stopped activity session: {}", session.id);
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    /// Record an activity
    pub async fn record_activity(&self, activity: Activity) -> Result<()> {
        // Check rate limit to prevent overwhelming the system
        {
            let mut limiter = self.rate_limiter.write().await;
            if !limiter.should_allow() {
                debug!("Rate limit reached, skipping activity");
                return Ok(());
            }
        }

        self.metrics.record_activity_processed();

        // Apply all filters
        let mut filtered_activity = Some(activity);
        for filter in &self.filters {
            if let Some(act) = filtered_activity {
                filtered_activity = filter.filter(act);
            } else {
                self.metrics.record_activity_filtered();
                break;
            }
        }

        if let Some(activity) = filtered_activity {
            // Add to current session if one exists
            let mut session_guard = self.current_session.write().await;
            if let Some(session) = session_guard.as_mut() {
                // Check session size limit
                if session.activities.len() < self.config.max_activities_per_session {
                    session.add_activity(activity.clone());
                } else {
                    debug!("Session activity limit reached, skipping activity");
                    return Ok(());
                }
            }

            // Send to channel for processing (non-blocking)
            if let Err(e) = self.activity_tx.try_send(activity) {
                debug!("Failed to send activity to channel: {}", e);
            }
        }

        Ok(())
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> crate::performance::PerformanceStats {
        self.metrics.get_stats()
    }

    /// Get current session info
    pub async fn get_session_info(&self) -> Option<(String, usize, SystemTime)> {
        let session_guard = self.current_session.read().await;
        session_guard
            .as_ref()
            .map(|s| (s.title.clone(), s.activities.len(), s.start_time))
    }

    /// Start monitoring (spawns background tasks)
    pub async fn start_monitoring(&self, watch_paths: Vec<PathBuf>) -> Result<()> {
        info!("Starting activity monitoring");

        // File monitoring
        if self.config.enable_file_monitoring {
            let tx = self.activity_tx.clone();
            let session = self.current_session.clone();
            let paths = watch_paths.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::monitor_files(tx, session, paths).await {
                    error!("File monitoring error: {}", e);
                }
            });
        }

        // Clipboard monitoring
        if self.config.enable_clipboard_monitoring {
            let tx = self.activity_tx.clone();
            let session = self.current_session.clone();
            let max_len = self.config.max_clipboard_length;

            tokio::spawn(async move {
                if let Err(e) = Self::monitor_clipboard(tx, session, max_len).await {
                    error!("Clipboard monitoring error: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Monitor file changes
    async fn monitor_files(
        _tx: mpsc::Sender<Activity>,
        _session: Arc<RwLock<Option<ActivitySession>>>,
        _paths: Vec<PathBuf>,
    ) -> Result<()> {
        info!("File monitoring started");

        // TODO: Implement using notify crate
        // For now, this is a placeholder

        Ok(())
    }

    /// Monitor clipboard changes
    async fn monitor_clipboard(
        _tx: mpsc::Sender<Activity>,
        _session: Arc<RwLock<Option<ActivitySession>>>,
        _max_length: usize,
    ) -> Result<()> {
        info!("Clipboard monitoring started");

        let _last_content = String::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

        loop {
            interval.tick().await;

            // Get session ID
            let _session_id = {
                let session_guard = _session.read().await;
                if let Some(s) = session_guard.as_ref() {
                    s.id.clone()
                } else {
                    continue; // No active session
                }
            };

            // TODO: Implement actual clipboard reading
            // This is a placeholder that would use platform-specific APIs
            // - macOS: NSPasteboard
            // - Linux: xclip/xsel
            // - Windows: Win32 API

            // Placeholder for now
            let _current_content = String::new();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_activity_monitor_creation() {
        let monitor = ActivityMonitor::new(ActivityMonitorConfig::default());
        assert!(monitor.filters.is_empty());
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let monitor = ActivityMonitor::new(ActivityMonitorConfig::default());

        // Start session
        let session_id = monitor.start_session("Test".to_string()).await.unwrap();
        assert!(!session_id.is_empty());

        // Check session info
        let info = monitor.get_session_info().await;
        assert!(info.is_some());

        // Stop session
        let session = monitor.stop_session().await.unwrap();
        assert!(session.is_some());
        assert!(session.unwrap().end_time.is_some());
    }

    #[tokio::test]
    async fn test_activity_recording() {
        let monitor = ActivityMonitor::new(ActivityMonitorConfig::default());

        // Start session
        let session_id = monitor.start_session("Test".to_string()).await.unwrap();

        // Record activity
        let activity = Activity::command(
            session_id.clone(),
            crate::activity::CommandActivity::new("test command".to_string(), "/tmp".to_string()),
        );

        monitor.record_activity(activity).await.unwrap();

        // Check session has activity
        let session = monitor.stop_session().await.unwrap().unwrap();
        assert_eq!(session.activities.len(), 1);
    }

    #[tokio::test]
    async fn test_session_limit() {
        let config = ActivityMonitorConfig {
            max_activities_per_session: 2,
            ..Default::default()
        };

        let monitor = ActivityMonitor::new(config);
        let session_id = monitor.start_session("Test".to_string()).await.unwrap();

        // Record 3 activities, but only 2 should be stored
        for i in 0..3 {
            let activity = Activity::command(
                session_id.clone(),
                crate::activity::CommandActivity::new(format!("command {}", i), "/tmp".to_string()),
            );
            monitor.record_activity(activity).await.unwrap();
        }

        let session = monitor.stop_session().await.unwrap().unwrap();
        assert_eq!(session.activities.len(), 2); // Limited to 2
    }
}
