use std::collections::BTreeMap;

use anyhow::Result;

use crate::repo::FukuraRepo;

#[derive(Debug, Default)]
pub struct RedactionUpdateReport {
    pub set: BTreeMap<String, String>,
    pub removed: Vec<String>,
}

pub fn update_remote(repo: &FukuraRepo, remote: Option<&str>) -> Result<Option<String>> {
    let mut cfg = repo.config()?;
    cfg.set_default_remote(remote.map(|s| s.to_string()));
    cfg.save(&repo.config_path())?;
    Ok(cfg.default_remote.clone())
}

pub fn update_redaction(
    repo: &FukuraRepo,
    additions: Vec<(String, String)>,
    removals: Vec<String>,
) -> Result<RedactionUpdateReport> {
    let mut cfg = repo.config()?;
    let mut report = RedactionUpdateReport::default();
    for (key, pattern) in additions {
        cfg.set_redaction_override(&key, &pattern);
        report.set.insert(key, pattern);
    }
    for key in removals {
        if cfg.remove_redaction_override(&key) {
            report.removed.push(key);
        }
    }
    cfg.save(&repo.config_path())?;
    Ok(report)
}
