use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Serialize)]
pub struct CacheEntry {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub created_secs: u64,
    pub last_accessed_secs: u64,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheSummary {
    pub total_size_bytes: u64,
    pub entry_count: usize,
    pub entries: Vec<CacheEntry>,
}

#[derive(Deserialize)]
struct MetaFile {
    url: Option<String>,
    created_at: Option<u64>,
    last_accessed: Option<u64>,
}

#[tauri::command]
pub fn scan_cache(cache_dir: String) -> Result<CacheSummary, String> {
    let dir = paths::resolve_path(&cache_dir).map_err(|e| e.to_string())?;
    scan_cache_dir(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_cache_entry(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if p.exists() {
        fs::remove_file(p).map_err(|e| format!("Delete failed: {e}"))?;
    }
    let meta = PathBuf::from(format!("{}.meta", path));
    let _ = fs::remove_file(&meta);
    Ok(())
}

#[tauri::command]
pub fn clear_cache(cache_dir: String) -> Result<(), String> {
    let dir = paths::resolve_path(&cache_dir).map_err(|e| e.to_string())?;
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        if entry.path().is_file() {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}

fn scan_cache_dir(dir: &Path) -> anyhow::Result<CacheSummary> {
    if !dir.exists() {
        return Ok(CacheSummary { total_size_bytes: 0, entry_count: 0, entries: vec![] });
    }

    let mut entries = Vec::new();
    let mut total: u64 = 0;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "meta" || ext == "tmp" { continue; }
        }
        if !path.is_file() { continue; }

        let metadata = match fs::metadata(&path) { Ok(m) => m, Err(_) => continue };
        let size = metadata.len();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        let meta_path = PathBuf::from(format!("{}.meta", path.display()));
        let meta = fs::read_to_string(&meta_path)
            .ok()
            .and_then(|s| serde_json::from_str::<MetaFile>(&s).ok());

        let created = meta.as_ref().and_then(|m| m.created_at).unwrap_or(now);
        let accessed = meta.as_ref().and_then(|m| m.last_accessed).unwrap_or(created);
        let url = meta.and_then(|m| m.url);

        total += size;
        entries.push(CacheEntry {
            path: path.to_string_lossy().to_string(),
            file_name,
            size_bytes: size,
            created_secs: created,
            last_accessed_secs: accessed,
            url,
        });
    }

    entries.sort_by(|a, b| b.last_accessed_secs.cmp(&a.last_accessed_secs));
    Ok(CacheSummary { total_size_bytes: total, entry_count: entries.len(), entries })
}
