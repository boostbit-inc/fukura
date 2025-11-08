use std::{collections::BTreeMap, fs, io::Read, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FukuraConfig {
    pub version: u32,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub redaction_overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub default_remote: Option<String>,
    #[serde(default)]
    pub auto_sync: Option<bool>,
    #[serde(default)]
    pub daemon_enabled: Option<bool>,
    #[serde(default)]
    pub recording: RecordingConfig,
    #[serde(default)]
    pub activity_tracking: ActivityTrackingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    /// Maximum time to look back for time-based recording (in hours)
    #[serde(default = "RecordingConfig::default_max_lookback_hours")]
    pub max_lookback_hours: u32,
    /// Minimum time to look back for time-based recording (in minutes)  
    #[serde(default = "RecordingConfig::default_min_lookback_minutes")]
    pub min_lookback_minutes: u32,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            max_lookback_hours: 3,
            min_lookback_minutes: 1,
        }
    }
}

impl RecordingConfig {
    fn default_max_lookback_hours() -> u32 {
        3
    }

    fn default_min_lookback_minutes() -> u32 {
        1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityTrackingConfig {
    /// Enable comprehensive activity tracking
    #[serde(default = "ActivityTrackingConfig::default_enabled")]
    pub enabled: bool,

    /// Track file changes
    #[serde(default = "ActivityTrackingConfig::default_file_tracking")]
    pub file_tracking: bool,

    /// Track clipboard (privacy-sensitive, off by default)
    #[serde(default)]
    pub clipboard_tracking: bool,

    /// Track application switches
    #[serde(default = "ActivityTrackingConfig::default_app_tracking")]
    pub app_tracking: bool,

    /// Track editor activities (requires editor integration)
    #[serde(default = "ActivityTrackingConfig::default_editor_tracking")]
    pub editor_tracking: bool,

    /// Maximum clipboard content length to store
    #[serde(default = "ActivityTrackingConfig::default_max_clipboard_length")]
    pub max_clipboard_length: usize,

    /// Maximum file size (KB) for diff calculation
    #[serde(default = "ActivityTrackingConfig::default_max_file_size_kb")]
    pub max_file_size_kb: u64,

    /// Maximum activities per session
    #[serde(default = "ActivityTrackingConfig::default_max_activities_per_session")]
    pub max_activities_per_session: usize,

    /// Data retention period (days)
    #[serde(default = "ActivityTrackingConfig::default_retention_days")]
    pub retention_days: u32,

    /// Excluded file patterns
    #[serde(default = "ActivityTrackingConfig::default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

impl Default for ActivityTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            file_tracking: true,
            clipboard_tracking: false,
            app_tracking: true,
            editor_tracking: true,
            max_clipboard_length: 1000,
            max_file_size_kb: 100,
            max_activities_per_session: 10000,
            retention_days: 30,
            exclude_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "__pycache__".to_string(),
                ".env".to_string(),
                "*.key".to_string(),
                "*.pem".to_string(),
            ],
        }
    }
}

impl ActivityTrackingConfig {
    fn default_enabled() -> bool {
        true
    }

    fn default_file_tracking() -> bool {
        true
    }

    fn default_app_tracking() -> bool {
        true
    }

    fn default_editor_tracking() -> bool {
        true
    }

    fn default_max_clipboard_length() -> usize {
        1000
    }

    fn default_max_file_size_kb() -> u64 {
        100
    }

    fn default_max_activities_per_session() -> usize {
        10000
    }

    fn default_retention_days() -> u32 {
        30
    }

    fn default_exclude_patterns() -> Vec<String> {
        vec![
            "node_modules".to_string(),
            ".git".to_string(),
            "target".to_string(),
            "__pycache__".to_string(),
        ]
    }
}

impl FukuraConfig {
    /// Get global config directory path
    pub fn global_config_dir() -> Result<std::path::PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(std::path::PathBuf::from(home).join(".fukura"))
    }

    /// Get global config file path
    pub fn global_config_path() -> Result<std::path::PathBuf> {
        Ok(Self::global_config_dir()?.join("config.toml"))
    }

    /// Load global config
    pub fn load_global() -> Result<Self> {
        let path = Self::global_config_path()?;
        Self::load(&path)
    }

    /// Load config with global fallback
    pub fn load_with_global_fallback(path: &Path) -> Result<Self> {
        // Try local config first
        let mut config = Self::load(path)?;

        // Load global config for defaults
        if let Ok(global) = Self::load_global() {
            // Use global values if local ones are not set
            if config.default_remote.is_none() && global.default_remote.is_some() {
                config.default_remote = global.default_remote;
            }
            if config.auto_sync.is_none() && global.auto_sync.is_some() {
                config.auto_sync = global.auto_sync;
            }
        }

        Ok(config)
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let mut file = fs::File::open(path)
            .with_context(|| format!("Failed to open config at {}", path.display()))?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        let cfg = toml::from_str::<Self>(&buf).with_context(|| "Failed to parse config")?;
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let payload = toml::to_string_pretty(self)?;
        fs::write(path, payload)?;
        Ok(())
    }

    pub fn set_default_remote(&mut self, remote: Option<String>) {
        self.default_remote = remote
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty());
    }

    pub fn set_redaction_override(&mut self, key: &str, pattern: &str) {
        self.redaction_overrides
            .insert(key.trim().to_string(), pattern.to_string());
    }

    pub fn remove_redaction_override(&mut self, key: &str) -> bool {
        self.redaction_overrides.remove(key.trim()).is_some()
    }
}
