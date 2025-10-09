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
