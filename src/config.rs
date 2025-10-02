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
}

impl FukuraConfig {
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
