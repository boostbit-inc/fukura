use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Comprehensive activity tracking system
///
/// This module provides a multi-layered activity tracking system that captures
/// not just commands, but file changes, clipboard operations, editor activities,
/// and more to provide complete context for development work.

// ============================================================================
// Activity Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActivityType {
    Command(CommandActivity),
    Error(ErrorActivity),
    FileChange(FileChangeActivity),
    Clipboard(ClipboardActivity),
    Editor(EditorActivity),
    App(AppActivity),
    UserInput(InputActivity),
    Git(GitActivity),
    Browser(BrowserActivity),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub activity_type: ActivityType,
    pub timestamp: SystemTime,
    pub session_id: String,
    pub metadata: HashMap<String, String>,
}

// ============================================================================
// Command Activity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandActivity {
    pub command: String,
    pub exit_code: Option<i32>,
    pub working_directory: String,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub duration_ms: u64,
    pub environment: HashMap<String, String>,
}

impl CommandActivity {
    pub fn new(command: String, working_directory: String) -> Self {
        Self {
            command,
            exit_code: None,
            working_directory,
            stdout: None,
            stderr: None,
            duration_ms: 0,
            environment: HashMap::new(),
        }
    }

    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.exit_code = Some(code);
        self
    }

    pub fn with_output(mut self, stdout: String, stderr: String) -> Self {
        self.stdout = Some(stdout);
        self.stderr = Some(stderr);
        self
    }
}

// ============================================================================
// Error Activity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorActivity {
    pub message: String,
    pub normalized: String,
    pub source: ErrorSource,
    pub stderr_output: Option<String>,
    pub stack_trace: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSource {
    Command,
    Compiler,
    Runtime,
    Linter,
    Test,
    Other(String),
}

// ============================================================================
// File Change Activity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeActivity {
    pub path: PathBuf,
    pub change_type: FileChangeType,
    pub file_type: String,
    pub size_bytes: u64,
    pub diff: Option<String>,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
}

impl FileChangeActivity {
    pub fn new(path: PathBuf, change_type: FileChangeType) -> Self {
        let file_type = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            path,
            change_type,
            file_type,
            size_bytes: 0,
            diff: None,
            lines_added: 0,
            lines_removed: 0,
            language: None,
        }
    }

    pub fn with_diff(mut self, diff: String, added: usize, removed: usize) -> Self {
        self.diff = Some(diff);
        self.lines_added = added;
        self.lines_removed = removed;
        self
    }
}

// ============================================================================
// Clipboard Activity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardActivity {
    pub content: String,
    pub content_type: ClipboardType,
    pub source_app: Option<String>,
    pub redacted: bool,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardType {
    Text,
    Code(String), // Language name
    Url,
    FilePath,
    Image,
    Other,
}

impl ClipboardActivity {
    pub fn new(content: String) -> Self {
        let length = content.len();
        Self {
            content,
            content_type: ClipboardType::Text,
            source_app: None,
            redacted: false,
            length,
        }
    }

    pub fn detect_type(mut self) -> Self {
        self.content_type =
            if self.content.starts_with("http://") || self.content.starts_with("https://") {
                ClipboardType::Url
            } else if self.content.starts_with('/') || self.content.contains(":\\") {
                ClipboardType::FilePath
            } else if self.content.contains("fn ")
                || self.content.contains("def ")
                || self.content.contains("function ")
            {
                ClipboardType::Code("unknown".to_string())
            } else {
                ClipboardType::Text
            };
        self
    }
}

// ============================================================================
// Editor Activity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorActivity {
    pub editor: String,
    pub file_path: PathBuf,
    pub action: EditorAction,
    pub language: Option<String>,
    pub changes: Option<TextChange>,
    pub cursor_position: Option<CursorPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditorAction {
    Open,
    Edit,
    Save,
    Close,
    Format,
    Refactor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChange {
    pub start_line: usize,
    pub end_line: usize,
    pub text_added: String,
    pub text_removed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
}

// ============================================================================
// App Activity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppActivity {
    pub app_name: String,
    pub app_bundle_id: Option<String>,
    pub action: AppAction,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppAction {
    Opened,
    Closed,
    Focused,
    Unfocused,
}

// ============================================================================
// User Input Activity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputActivity {
    pub input_text: String,
    pub context: String,
    pub redacted: bool,
    pub prompt: Option<String>,
}

// ============================================================================
// Git Activity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitActivity {
    pub operation: GitOperation,
    pub branch: Option<String>,
    pub commit_message: Option<String>,
    pub files_changed: Vec<String>,
    pub commit_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitOperation {
    Commit,
    Push,
    Pull,
    Checkout,
    Branch,
    Merge,
    Rebase,
    Stash,
    Other(String),
}

// ============================================================================
// Browser Activity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserActivity {
    pub url: String,
    pub title: String,
    pub action: BrowserAction,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserAction {
    Opened,
    Closed,
    Error,
    ConsoleError,
}

// ============================================================================
// Activity Factory
// ============================================================================

impl Activity {
    pub fn command(session_id: String, activity: CommandActivity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            activity_type: ActivityType::Command(activity),
            timestamp: SystemTime::now(),
            session_id,
            metadata: HashMap::new(),
        }
    }

    pub fn file_change(session_id: String, activity: FileChangeActivity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            activity_type: ActivityType::FileChange(activity),
            timestamp: SystemTime::now(),
            session_id,
            metadata: HashMap::new(),
        }
    }

    pub fn clipboard(session_id: String, activity: ClipboardActivity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            activity_type: ActivityType::Clipboard(activity),
            timestamp: SystemTime::now(),
            session_id,
            metadata: HashMap::new(),
        }
    }

    pub fn editor(session_id: String, activity: EditorActivity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            activity_type: ActivityType::Editor(activity),
            timestamp: SystemTime::now(),
            session_id,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

// ============================================================================
// Activity Session
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySession {
    pub id: String,
    pub title: String,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub activities: Vec<Activity>,
    pub tags: Vec<String>,
}

impl ActivitySession {
    pub fn new(title: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            start_time: SystemTime::now(),
            end_time: None,
            activities: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn add_activity(&mut self, activity: Activity) {
        self.activities.push(activity);
    }

    pub fn finish(&mut self) {
        self.end_time = Some(SystemTime::now());
    }

    pub fn duration(&self) -> Option<std::time::Duration> {
        self.end_time
            .or(Some(SystemTime::now()))
            .and_then(|end| end.duration_since(self.start_time).ok())
    }
}

// ============================================================================
// Activity Filters
// ============================================================================

pub trait ActivityFilter: Send + Sync {
    fn should_include(&self, activity: &Activity) -> bool;
    fn filter(&self, activity: Activity) -> Option<Activity> {
        if self.should_include(&activity) {
            Some(activity)
        } else {
            None
        }
    }
}

pub struct PrivacyFilter {
    redaction_patterns: Vec<regex::Regex>,
}

impl PrivacyFilter {
    pub fn new(patterns: Vec<String>) -> Result<Self> {
        let redaction_patterns = patterns
            .iter()
            .map(|p| regex::Regex::new(p))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { redaction_patterns })
    }

    pub fn redact_text(&self, text: &str) -> String {
        let mut result = text.to_string();
        for pattern in &self.redaction_patterns {
            result = pattern.replace_all(&result, "[REDACTED]").to_string();
        }
        result
    }
}

impl ActivityFilter for PrivacyFilter {
    fn should_include(&self, _activity: &Activity) -> bool {
        true // Don't exclude, just redact
    }

    fn filter(&self, mut activity: Activity) -> Option<Activity> {
        // Apply redaction based on activity type
        match &mut activity.activity_type {
            ActivityType::Command(cmd) => {
                cmd.command = self.redact_text(&cmd.command);
                if let Some(stdout) = &cmd.stdout {
                    cmd.stdout = Some(self.redact_text(stdout));
                }
                if let Some(stderr) = &cmd.stderr {
                    cmd.stderr = Some(self.redact_text(stderr));
                }
            }
            ActivityType::Clipboard(clip) => {
                clip.content = self.redact_text(&clip.content);
                clip.redacted = true;
            }
            ActivityType::UserInput(input) => {
                input.input_text = self.redact_text(&input.input_text);
                input.redacted = true;
            }
            _ => {}
        }
        Some(activity)
    }
}

pub struct SizeFilter {
    max_content_size: usize,
}

impl SizeFilter {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_content_size: max_size,
        }
    }
}

impl ActivityFilter for SizeFilter {
    fn should_include(&self, activity: &Activity) -> bool {
        match &activity.activity_type {
            ActivityType::Clipboard(clip) => clip.length <= self.max_content_size,
            ActivityType::FileChange(file) => file.size_bytes <= self.max_content_size as u64,
            _ => true,
        }
    }
}

pub struct ExclusionFilter {
    exclude_paths: Vec<PathBuf>,
    exclude_patterns: Vec<regex::Regex>,
}

impl ExclusionFilter {
    pub fn new(paths: Vec<PathBuf>, patterns: Vec<String>) -> Result<Self> {
        let exclude_patterns = patterns
            .iter()
            .map(|p| regex::Regex::new(p))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            exclude_paths: paths,
            exclude_patterns,
        })
    }

    fn should_exclude_path(&self, path: &Path) -> bool {
        // Check exact paths
        if self.exclude_paths.iter().any(|p| path.starts_with(p)) {
            return true;
        }

        // Check patterns
        if let Some(path_str) = path.to_str() {
            for pattern in &self.exclude_patterns {
                if pattern.is_match(path_str) {
                    return true;
                }
            }
        }

        false
    }
}

impl ActivityFilter for ExclusionFilter {
    fn should_include(&self, activity: &Activity) -> bool {
        match &activity.activity_type {
            ActivityType::FileChange(file) => !self.should_exclude_path(&file.path),
            ActivityType::Editor(editor) => !self.should_exclude_path(&editor.file_path),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_activity_creation() {
        let cmd = CommandActivity::new("cargo build".to_string(), "/home/user/project".to_string());
        assert_eq!(cmd.command, "cargo build");
        assert_eq!(cmd.working_directory, "/home/user/project");
    }

    #[test]
    fn test_file_change_activity() {
        let file_change =
            FileChangeActivity::new(PathBuf::from("src/main.rs"), FileChangeType::Modified);
        assert_eq!(file_change.file_type, "rs");
    }

    #[test]
    fn test_clipboard_type_detection() {
        let clip = ClipboardActivity::new("https://example.com".to_string()).detect_type();
        matches!(clip.content_type, ClipboardType::Url);
    }

    #[test]
    fn test_activity_session() {
        let mut session = ActivitySession::new("Test session".to_string());
        assert_eq!(session.activities.len(), 0);

        let activity = Activity::command(
            session.id.clone(),
            CommandActivity::new("test".to_string(), "/tmp".to_string()),
        );
        session.add_activity(activity);

        assert_eq!(session.activities.len(), 1);
    }

    #[test]
    fn test_privacy_filter() {
        let filter = PrivacyFilter::new(vec!["password.*=.*".to_string()]).unwrap();

        let text = "command --password=secret123";
        let redacted = filter.redact_text(text);
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_size_filter() {
        let filter = SizeFilter::new(100);

        let small_clip = Activity::clipboard(
            "session1".to_string(),
            ClipboardActivity::new("small".to_string()),
        );
        assert!(filter.should_include(&small_clip));

        let large_content = "x".repeat(200);
        let large_clip = Activity::clipboard(
            "session1".to_string(),
            ClipboardActivity::new(large_content),
        );
        assert!(!filter.should_include(&large_clip));
    }
}
