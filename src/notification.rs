use anyhow::Result;
use notify_rust::{Notification, Timeout};
use std::path::{Path, PathBuf};

/// Notification preferences stored in config
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub show_on_error: bool,
    pub show_on_solution_found: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_on_error: true,
            show_on_solution_found: true,
        }
    }
}

/// Notification manager for OS-native notifications
pub struct NotificationManager {
    config: NotificationConfig,
    config_path: PathBuf,
}

impl NotificationManager {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let config_path = repo_path.join(".fukura").join("notification.toml");
        let config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            toml::from_str(&content).unwrap_or_default()
        } else {
            let default_config = NotificationConfig::default();
            // Create config file with defaults
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, content)?;
            default_config
        };

        Ok(Self {
            config,
            config_path,
        })
    }

    /// Show error notification
    pub fn notify_error(&self, command: &str, error_message: &str) -> Result<()> {
        if !self.config.enabled || !self.config.show_on_error {
            return Ok(());
        }

        let summary = "Fukura: Error Detected";
        let body = format!(
            "Command failed: {}\n\nClick to view details",
            Self::truncate(command, 50)
        );

        #[cfg(target_os = "macos")]
        self.show_notification_macos(summary, &body, Some(error_message))?;

        #[cfg(target_os = "linux")]
        self.show_notification_linux(summary, &body)?;

        #[cfg(target_os = "windows")]
        self.show_notification_windows(summary, &body)?;

        Ok(())
    }

    /// Show error notification with note ID (BEST PRACTICE: Rich notification without auto-opening)
    pub fn notify_error_with_id(
        &self,
        command: &str,
        error_message: &str,
        note_id: &str,
    ) -> Result<()> {
        if !self.config.enabled || !self.config.show_on_error {
            return Ok(());
        }

        let short_id = &note_id[..8.min(note_id.len())];

        let summary = "Fukura: Error Captured";
        let body = format!(
            "Command: {}\n\nError: {}\n\nView details:\n  fuku view {}\n  fuku open {}",
            Self::truncate(command, 40),
            Self::truncate(error_message, 60),
            short_id,
            short_id
        );

        #[cfg(target_os = "macos")]
        self.show_notification_detailed_macos(summary, &body)?;

        #[cfg(target_os = "linux")]
        self.show_notification_detailed_linux(summary, &body, short_id)?;

        #[cfg(target_os = "windows")]
        self.show_notification_detailed_windows(summary, &body)?;

        Ok(())
    }

    /// Show error with known solutions (WORLD-CLASS: Intelligent assistance)
    pub fn notify_error_with_solutions(
        &self,
        command: &str,
        _error_message: &str,
        note_id: &str,
        solutions: &[crate::daemon::SolutionHit],
    ) -> Result<()> {
        if !self.config.enabled || !self.config.show_on_error {
            return Ok(());
        }

        let short_id = &note_id[..8.min(note_id.len())];
        let solution_count = solutions.len();

        let summary = format!(
            "Fukura: Error Captured ({} solution{} found)",
            solution_count,
            if solution_count > 1 { "s" } else { "" }
        );

        let solutions_text = solutions
            .iter()
            .take(2)
            .map(|s| format!("  • {}", Self::truncate(&s.snippet, 80)))
            .collect::<Vec<_>>()
            .join("\n");

        let body = format!(
            "Command: {}\n\nYou've solved this before:\n{}\n\nView:\n  fuku view {}\n  fuku open {}",
            Self::truncate(command, 40),
            solutions_text,
            short_id,
            short_id
        );

        #[cfg(target_os = "macos")]
        self.show_notification_detailed_macos(&summary, &body)?;

        #[cfg(target_os = "linux")]
        self.show_notification_detailed_linux(&summary, &body, short_id)?;

        #[cfg(target_os = "windows")]
        self.show_notification_detailed_windows(&summary, &body)?;

        Ok(())
    }

    /// Show solution found notification
    pub fn notify_solution_found(&self, error_pattern: &str, solution_count: usize) -> Result<()> {
        if !self.config.enabled || !self.config.show_on_solution_found {
            return Ok(());
        }

        let summary = "Fukura: Solutions Found";
        let body = format!(
            "Found {} solution(s) for:\n{}\n\nClick to view",
            solution_count,
            Self::truncate(error_pattern, 60)
        );

        #[cfg(target_os = "macos")]
        self.show_notification_macos(summary, &body, None)?;

        #[cfg(target_os = "linux")]
        self.show_notification_linux(summary, &body)?;

        #[cfg(target_os = "windows")]
        self.show_notification_windows(summary, &body)?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn show_notification_macos(
        &self,
        summary: &str,
        body: &str,
        _details: Option<&str>,
    ) -> Result<()> {
        // macOS: Use osascript for guaranteed notifications
        use std::process::Command;

        let escaped_title = summary.replace('"', r#"\""#);
        let escaped_body = body.replace('"', r#"\""#);

        let script = format!(
            r#"display notification "{}" with title "{}""#,
            escaped_body, escaped_title
        );

        Command::new("osascript").arg("-e").arg(&script).output()?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn show_notification_detailed_macos(&self, summary: &str, body: &str) -> Result<()> {
        // macOS: Use osascript for guaranteed notifications (BEST PRACTICE)
        use std::process::Command;

        let escaped_title = summary.replace('"', r#"\""#);
        let escaped_body = body.replace('"', r#"\""#).replace('\n', " ");

        let script = format!(
            r#"display notification "{}" with title "{}" sound name "Submarine""#,
            escaped_body, escaped_title
        );

        Command::new("osascript").arg("-e").arg(&script).output()?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn show_notification_linux(&self, summary: &str, body: &str) -> Result<()> {
        Notification::new()
            .summary(summary)
            .body(body)
            .appname("Fukura")
            .timeout(Timeout::Milliseconds(5000))
            .show()?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn show_notification_detailed_linux(
        &self,
        summary: &str,
        body: &str,
        _short_id: &str,
    ) -> Result<()> {
        // Linux: Rich notification with action hint (NO auto-open)
        Notification::new()
            .summary(summary)
            .body(body)
            .appname("Fukura")
            .timeout(Timeout::Milliseconds(15000))
            .urgency(notify_rust::Urgency::Normal)
            .show()?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn show_notification_windows(&self, summary: &str, body: &str) -> Result<()> {
        Notification::new()
            .summary(summary)
            .body(body)
            .appname("Fukura")
            .timeout(Timeout::Milliseconds(5000))
            .show()?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn show_notification_detailed_windows(&self, summary: &str, body: &str) -> Result<()> {
        // Windows: Rich notification with command instructions (NO auto-open)
        Notification::new()
            .summary(summary)
            .body(body)
            .appname("Fukura")
            .timeout(Timeout::Milliseconds(15000))
            .show()?;
        Ok(())
    }

    /// Enable notifications
    pub fn enable(&mut self) -> Result<()> {
        self.config.enabled = true;
        self.save_config()
    }

    /// Disable notifications
    pub fn disable(&mut self) -> Result<()> {
        self.config.enabled = false;
        self.save_config()
    }

    /// Check if notifications are enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Send test notification (for debugging)
    pub fn send_test_notification(&self) -> Result<()> {
        let summary = "Fukura: Test Notification";
        let body = "If you can see this, notifications are working correctly!";

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let script = format!(
                r#"display notification "{}" with title "{}" sound name "Submarine""#,
                body, summary
            );
            Command::new("osascript").arg("-e").arg(&script).output()?;
        }

        #[cfg(target_os = "linux")]
        {
            Notification::new()
                .summary(summary)
                .body(body)
                .appname("Fukura")
                .timeout(Timeout::Milliseconds(5000))
                .show()?;
        }

        #[cfg(target_os = "windows")]
        {
            Notification::new()
                .summary(summary)
                .body(body)
                .appname("Fukura")
                .timeout(Timeout::Milliseconds(5000))
                .show()?;
        }

        Ok(())
    }

    /// Save notification config
    fn save_config(&self) -> Result<()> {
        let content = toml::to_string_pretty(&self.config)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Truncate string with ellipsis
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }
}
