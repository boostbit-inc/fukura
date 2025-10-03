use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, ensure, Context, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::config::FukuraConfig;
use crate::index::{SearchHit, SearchIndex, SearchSort};
use crate::models::{Note, NoteEnvelope, NoteRecord};
use crate::pack::{load_object_from_pack, load_pack_indices, pack_objects, PackReport};
use crate::redaction::Redactor;

#[derive(Clone, Debug)]
pub struct FukuraRepo {
    root: PathBuf,
    dot_dir: PathBuf,
}

impl FukuraRepo {
    pub fn init(path: &Path, force: bool) -> Result<Self> {
        let dot_dir = path.join(".fukura");
        if dot_dir.exists() && !force {
            bail!("Repository already initialized at {}", path.display());
        }
        fs::create_dir_all(path)?;
        let repo = Self {
            root: path.to_path_buf(),
            dot_dir,
        };
        repo.ensure_layout()?;
        let cfg = FukuraConfig { version: 1, ..Default::default() };
        cfg.save(&repo.config_path())?;
        Ok(repo)
    }

    pub fn open(path: &Path) -> Result<Self> {
        let dot_dir = path.join(".fukura");
        if !dot_dir.exists() {
            bail!("No .fukura directory at {}", path.display());
        }
        let repo = Self {
            root: path.to_path_buf(),
            dot_dir,
        };
        repo.ensure_layout()?;
        Ok(repo)
    }

    pub fn discover(start: Option<&Path>) -> Result<Self> {
        let mut current = start
            .map(|p| p.to_path_buf())
            .unwrap_or(std::env::current_dir()?);
        loop {
            let candidate = current.join(".fukura");
            if candidate.exists() {
                return Self::open(&current);
            }
            if !current.pop() {
                bail!("No fuku repository found. Run `fuku init` first.");
            }
        }
    }

    fn ensure_layout(&self) -> Result<()> {
        for dir in ["objects", "packs", "refs", "index", "locks"] {
            fs::create_dir_all(self.dot_dir.join(dir))?;
        }
        Ok(())
    }

    pub fn config_path(&self) -> PathBuf {
        self.dot_dir.join("config")
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.dot_dir.join("objects")
    }

    pub fn refs_dir(&self) -> PathBuf {
        self.dot_dir.join("refs")
    }

    pub fn index_dir(&self) -> PathBuf {
        self.dot_dir.join("index")
    }

    pub fn pack_dir(&self) -> PathBuf {
        self.dot_dir.join("packs")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn store_note(&self, mut note: Note) -> Result<NoteRecord> {
        let cfg = FukuraConfig::load(&self.config_path())?;
        let redactor = Redactor::default_with_overrides(&cfg.redaction_overrides);
        note.body = redactor.redact(&note.body);
        
        // Also redact meta fields
        let mut redacted_meta = std::collections::BTreeMap::new();
        for (key, value) in note.meta {
            redacted_meta.insert(key, redactor.redact(&value));
        }
        note.meta = redacted_meta;
        
        let object_id = self.persist_object("note", &note.canonical_bytes()?)?;
        let record = NoteRecord {
            object_id: object_id.clone(),
            note: note.clone(),
        };
        let index = SearchIndex::open_or_create(self)?;
        index.add_note(&record)?;
        self.update_latest_ref(&object_id)?;
        Ok(record)
    }

    pub fn load_note(&self, object_id: &str) -> Result<NoteRecord> {
        let object_bytes = self.load_object_bytes(object_id)?;
        let mut decoder = ZlibDecoder::new(std::io::Cursor::new(object_bytes));
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        let mut parts = buf.splitn(2, |b| *b == 0);
        let header = parts.next().context("Missing object header")?;
        let payload = parts.next().context("Missing object payload")?;
        let header_str = std::str::from_utf8(header)?;
        let mut header_parts = header_str.split_whitespace();
        let object_type = header_parts.next().context("Invalid header")?;
        if object_type != "note" {
            bail!("Object {} is not a note", object_id);
        }
        let envelope: NoteEnvelope = ciborium::de::from_reader(std::io::Cursor::new(payload))?;
        let record = NoteRecord {
            object_id: object_id.to_string(),
            note: envelope.note,
        };
        Ok(record)
    }

    pub fn load_object_bytes(&self, object_id: &str) -> Result<Vec<u8>> {
        let path = self.object_path(object_id);
        if path.exists() {
            return fs::read(&path).with_context(|| format!("Failed to read {}", path.display()));
        }
        if let Some(bytes) = load_object_from_pack(self, object_id)? {
            return Ok(bytes);
        }
        bail!("Object {} not found", object_id)
    }

    pub fn search(&self, query: &str, limit: usize, sort: SearchSort) -> Result<Vec<SearchHit>> {
        let index = SearchIndex::open_or_create(self)?;
        index.search(query, limit, sort)
    }

    pub fn pack_loose_objects(&self, prune: bool) -> Result<PackReport> {
        pack_objects(self, prune)
    }

    fn persist_object(&self, object_type: &str, payload: &[u8]) -> Result<String> {
        let mut header = format!("{} {}\0", object_type, payload.len()).into_bytes();
        header.extend_from_slice(payload);
        let mut hasher = Sha256::new();
        hasher.update(&header);
        let digest = hasher.finalize();
        let object_id = hex::encode(digest);
        let (prefix, rest) = object_id.split_at(2);
        let dir_path = self.objects_dir().join(prefix);
        fs::create_dir_all(&dir_path)?;
        let object_path = dir_path.join(rest);
        if object_path.exists() {
            return Ok(object_id);
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&header)?;
        let compressed = encoder.finish()?;
        let mut temp = NamedTempFile::new_in(&dir_path)?;
        temp.write_all(&compressed)?;
        temp.flush()?;
        temp.as_file().sync_all()?;
        let persisted = temp.persist(&object_path)?;
        let _ = persisted.sync_all();
        if let Ok(dir_file) = File::open(&dir_path) {
            let _ = dir_file.sync_all();
        }
        if let Ok(objects_dir) = File::open(self.objects_dir()) {
            let _ = objects_dir.sync_all();
        }
        Ok(object_id)
    }

    fn update_latest_ref(&self, object_id: &str) -> Result<()> {
        let latest_path = self.refs_dir().join("latest");
        fs::write(&latest_path, object_id.as_bytes())?;
        if let Ok(file) = File::open(&latest_path) {
            let _ = file.sync_all();
        }
        if let Ok(dir_file) = File::open(self.refs_dir()) {
            let _ = dir_file.sync_all();
        }
        Ok(())
    }

    fn object_path(&self, object_id: &str) -> PathBuf {
        let (prefix, rest) = object_id.split_at(2);
        self.objects_dir().join(prefix).join(rest)
    }

    pub fn latest(&self) -> Result<Option<String>> {
        let latest_path = self.refs_dir().join("latest");
        if !latest_path.exists() {
            return Ok(None);
        }
        let mut buf = String::new();
        File::open(&latest_path)?.read_to_string(&mut buf)?;
        if buf.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(buf.trim().to_string()))
        }
    }

    pub fn config(&self) -> Result<FukuraConfig> {
        FukuraConfig::load(&self.config_path())
    }

    pub fn collect_tags(&self) -> Result<Vec<String>> {
        let index = SearchIndex::open_or_create(self)?;
        index.collect_tags()
    }

    pub fn resolve_object_id(&self, input: &str) -> Result<String> {
        let candidate = input.trim();
        if candidate.len() >= 64 {
            let object_id = candidate[..64].to_lowercase();
            if self.object_path(&object_id).exists() {
                return Ok(object_id);
            }
            if let Ok(map) = load_pack_indices(self) {
                if map.contains_key(&object_id) {
                    return Ok(object_id);
                }
            }
        }
        let mut matches = Vec::new();
        if candidate.len() >= 2 {
            let (dir_prefix, rest) = candidate.split_at(2);
            let dir_path = self.objects_dir().join(dir_prefix);
            if dir_path.exists() {
                for entry in fs::read_dir(dir_path)? {
                    let entry = entry?;
                    if !entry.file_type()?.is_file() {
                        continue;
                    }
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    if name.starts_with(rest) {
                        matches.push(format!("{}{}", dir_prefix, name));
                    }
                }
            }
        } else {
            let objects_dir = self.objects_dir();
            if objects_dir.exists() {
                for dir in fs::read_dir(objects_dir)? {
                    let dir = dir?;
                    if !dir.file_type()?.is_dir() {
                        continue;
                    }
                    let dir_name = dir.file_name().to_string_lossy().to_string();
                    if !dir_name.starts_with(candidate) {
                        continue;
                    }
                    for entry in fs::read_dir(dir.path())? {
                        let entry = entry?;
                        if !entry.file_type()?.is_file() {
                            continue;
                        }
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        matches.push(format!("{}{}", dir_name, file_name));
                    }
                }
            }
        }
        matches.sort();
        matches.dedup();
        if matches.is_empty() {
            if let Ok(map) = load_pack_indices(self) {
                matches.extend(
                    map.keys()
                        .filter(|id| id.starts_with(candidate))
                        .cloned()
                        .collect::<Vec<_>>(),
                );
                matches.sort();
                matches.dedup();
            }
        }
        ensure!(!matches.is_empty(), "No object matching '{}'", candidate);
        ensure!(
            matches.len() == 1,
            "Ambiguous id '{}': choose from {:?}",
            candidate,
            matches
        );
        Ok(matches.remove(0))
    }
}
