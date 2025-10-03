use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::repo::FukuraRepo;

#[derive(Debug, Serialize, Deserialize)]
struct PackIndex {
    pack_file: String,
    created_at: String,
    objects: Vec<PackIndexEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct PackIndexEntry {
    id: String,
    offset: u64,
    length: u32,
}

#[derive(Debug, Serialize)]
pub struct PackReport {
    pub pack_file: PathBuf,
    pub index_file: PathBuf,
    pub object_count: usize,
    pub pruned: usize,
}

pub fn pack_objects(repo: &FukuraRepo, prune: bool) -> Result<PackReport> {
    let objects = collect_loose_objects(repo)?;
    if objects.is_empty() {
        bail!("No loose objects to pack");
    }
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let pack_name = format!("pack-{}.fop", timestamp);
    let pack_path = repo.pack_dir().join(&pack_name);
    let mut pack_file = File::create(&pack_path).with_context(|| "Failed to create pack file")?;
    let mut index = PackIndex {
        pack_file: pack_name.clone(),
        created_at: timestamp.to_string(),
        objects: Vec::with_capacity(objects.len()),
    };

    pack_file.write_all(b"FOP\0")?;
    pack_file.write_all(&1u32.to_le_bytes())?;
    pack_file.write_all(&(objects.len() as u32).to_le_bytes())?;

    for (object_id, path) in &objects {
        let mut data = Vec::new();
        File::open(path)
            .with_context(|| format!("Failed to open {}", path.display()))?
            .read_to_end(&mut data)?;
        let length = data.len();
        if length > u32::MAX as usize {
            bail!("Object {} is too large to pack", object_id);
        }
        let offset = pack_file.stream_position()?;
        pack_file.write_all(object_id.as_bytes())?;
        pack_file.write_all(&(length as u32).to_le_bytes())?;
        pack_file.write_all(&data)?;
        index.objects.push(PackIndexEntry {
            id: object_id.clone(),
            offset: offset + 64 + 4,
            length: length as u32,
        });
    }
    pack_file.flush()?;
    pack_file.sync_all()?;

    let index_path = repo.pack_dir().join(format!("pack-{}.fop.idx", timestamp));
    let index_json = serde_json::to_string_pretty(&index)?;
    fs::write(&index_path, index_json)?;

    let mut pruned = 0usize;
    if prune {
        for (_object_id, path) in &objects {
            if path.exists() {
                fs::remove_file(path)?;
                pruned += 1;
            }
            if let Some(parent) = path.parent() {
                cleanup_empty_dirs(parent, repo.objects_dir())?;
            }
        }
    }

    Ok(PackReport {
        pack_file: pack_path,
        index_file: index_path,
        object_count: objects.len(),
        pruned,
    })
}

pub fn load_object_from_pack(repo: &FukuraRepo, object_id: &str) -> Result<Option<Vec<u8>>> {
    let pack_dir = repo.pack_dir();
    if !pack_dir.exists() {
        return Ok(None);
    }
    for entry in fs::read_dir(&pack_dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("idx") {
            continue;
        }
        let index: PackIndex = serde_json::from_str(&fs::read_to_string(entry.path())?)
            .with_context(|| format!("Failed to parse {}", entry.path().display()))?;
        if let Some(item) = index
            .objects
            .iter()
            .find(|candidate| candidate.id == object_id)
        {
            let pack_path = pack_dir.join(index.pack_file);
            if !pack_path.exists() {
                continue;
            }
            let mut pack = File::open(&pack_path)
                .with_context(|| format!("Failed to open {}", pack_path.display()))?;
            pack.seek(SeekFrom::Start(item.offset))?;
            let mut buf = vec![0u8; item.length as usize];
            pack.read_exact(&mut buf)?;
            return Ok(Some(buf));
        }
    }
    Ok(None)
}

fn collect_loose_objects(repo: &FukuraRepo) -> Result<Vec<(String, PathBuf)>> {
    let mut objects = Vec::new();
    if !repo.objects_dir().exists() {
        return Ok(objects);
    }
    for prefix_entry in fs::read_dir(repo.objects_dir())? {
        let prefix_entry = prefix_entry?;
        if !prefix_entry.file_type()?.is_dir() {
            continue;
        }
        let prefix = prefix_entry.file_name();
        let prefix_str = prefix.to_string_lossy();
        for object_entry in fs::read_dir(prefix_entry.path())? {
            let object_entry = object_entry?;
            if !object_entry.file_type()?.is_file() {
                continue;
            }
            let suffix = object_entry.file_name();
            let suffix_str = suffix.to_string_lossy();
            let object_id = format!("{}{}", prefix_str, suffix_str);
            objects.push((object_id, object_entry.path()));
        }
    }
    Ok(objects)
}

fn cleanup_empty_dirs(path: &Path, root: PathBuf) -> Result<()> {
    if path == root {
        return Ok(());
    }
    if fs::read_dir(path)?.next().is_none() {
        fs::remove_dir(path)?;
        if let Some(parent) = path.parent() {
            cleanup_empty_dirs(parent, root)?;
        }
    }
    Ok(())
}

pub(crate) fn load_pack_indices(
    repo: &FukuraRepo,
) -> Result<BTreeMap<String, (PathBuf, PackIndexEntry)>> {
    let mut map = BTreeMap::new();
    if !repo.pack_dir().exists() {
        return Ok(map);
    }
    for entry in fs::read_dir(repo.pack_dir())? {
        let entry = entry?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("idx") {
            continue;
        }
        let index: PackIndex = serde_json::from_str(&fs::read_to_string(entry.path())?)
            .with_context(|| format!("Failed to parse {}", entry.path().display()))?;
        let pack_file = repo.pack_dir().join(index.pack_file);
        for object in index.objects {
            map.insert(object.id.clone(), (pack_file.clone(), object));
        }
    }
    Ok(map)
}
