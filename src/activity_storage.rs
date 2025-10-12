use anyhow::Result;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::activity::{Activity, ActivitySession};

/// Storage for activity data
pub struct ActivityStorage {
    storage_path: PathBuf,
}

impl ActivityStorage {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let storage_path = repo_path.join(".fukura").join("activities");
        fs::create_dir_all(&storage_path)?;

        Ok(Self { storage_path })
    }

    /// Store an activity session
    pub fn store_session(&self, session: &ActivitySession) -> Result<()> {
        let session_file = self.storage_path.join(format!("{}.json", session.id));
        let file = File::create(session_file)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, session)?;
        Ok(())
    }

    /// Load an activity session by ID
    pub fn load_session(&self, session_id: &str) -> Result<ActivitySession> {
        let session_file = self.storage_path.join(format!("{}.json", session_id));
        let file = File::open(session_file)?;
        let reader = BufReader::new(file);
        let session = serde_json::from_reader(reader)?;
        Ok(session)
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let mut sessions = Vec::new();
        
        for entry in fs::read_dir(&self.storage_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(file_stem) = path.file_stem() {
                    if let Some(session_id) = file_stem.to_str() {
                        sessions.push(session_id.to_string());
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Get sessions since a specific time
    pub fn get_sessions_since(&self, since: SystemTime) -> Result<Vec<ActivitySession>> {
        let mut sessions = Vec::new();

        for session_id in self.list_sessions()? {
            if let Ok(session) = self.load_session(&session_id) {
                if session.start_time >= since {
                    sessions.push(session);
                }
            }
        }

        // Sort by start time
        sessions.sort_by_key(|s| s.start_time);
        Ok(sessions)
    }

    /// Get activities from a session
    pub fn get_activities(&self, session_id: &str) -> Result<Vec<Activity>> {
        let session = self.load_session(session_id)?;
        Ok(session.activities)
    }

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let session_file = self.storage_path.join(format!("{}.json", session_id));
        fs::remove_file(session_file)?;
        Ok(())
    }

    /// Compact storage by removing old sessions
    pub fn compact(&self, retention_days: u32) -> Result<usize> {
        let cutoff = SystemTime::now()
            - std::time::Duration::from_secs(retention_days as u64 * 24 * 3600);

        let mut removed = 0;

        for session_id in self.list_sessions()? {
            if let Ok(session) = self.load_session(&session_id) {
                if session.start_time < cutoff {
                    self.delete_session(&session_id)?;
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::ActivitySession;
    use tempfile::TempDir;

    #[test]
    fn test_activity_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ActivityStorage::new(temp_dir.path()).unwrap();
        assert!(storage.storage_path.exists());
    }

    #[test]
    fn test_store_and_load_session() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ActivityStorage::new(temp_dir.path()).unwrap();

        let session = ActivitySession::new("Test session".to_string());
        let session_id = session.id.clone();

        // Store session
        storage.store_session(&session).unwrap();

        // Load session
        let loaded = storage.load_session(&session_id).unwrap();
        assert_eq!(loaded.id, session_id);
        assert_eq!(loaded.title, "Test session");
    }

    #[test]
    fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ActivityStorage::new(temp_dir.path()).unwrap();

        // Store multiple sessions
        for i in 0..3 {
            let session = ActivitySession::new(format!("Session {}", i));
            storage.store_session(&session).unwrap();
        }

        let sessions = storage.list_sessions().unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[test]
    fn test_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ActivityStorage::new(temp_dir.path()).unwrap();

        let session = ActivitySession::new("Test".to_string());
        let session_id = session.id.clone();

        storage.store_session(&session).unwrap();
        assert!(storage.load_session(&session_id).is_ok());

        storage.delete_session(&session_id).unwrap();
        assert!(storage.load_session(&session_id).is_err());
    }

    #[test]
    fn test_compact_old_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ActivityStorage::new(temp_dir.path()).unwrap();

        // Create sessions
        let mut old_session = ActivitySession::new("Old".to_string());
        old_session.start_time = SystemTime::now()
            - std::time::Duration::from_secs(31 * 24 * 3600); // 31 days ago
        
        let new_session = ActivitySession::new("New".to_string());

        storage.store_session(&old_session).unwrap();
        storage.store_session(&new_session).unwrap();

        // Compact with 30 day retention
        let removed = storage.compact(30).unwrap();
        assert_eq!(removed, 1);

        // Only new session should remain
        assert!(storage.load_session(&new_session.id).is_ok());
        assert!(storage.load_session(&old_session.id).is_err());
    }
}

