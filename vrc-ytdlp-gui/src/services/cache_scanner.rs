use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub size_bytes: u64,
    #[allow(dead_code)]
    pub created_secs: u64,
    pub last_accessed_secs: u64,
    #[allow(dead_code)]
    pub url: Option<String>,
}

#[derive(Debug, Clone)]
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

pub fn scan_cache(cache_dir: &Path) -> Result<CacheSummary> {
    if !cache_dir.exists() {
        return Ok(CacheSummary {
            total_size_bytes: 0,
            entry_count: 0,
            entries: vec![],
        });
    }

    let read_dir = fs::read_dir(cache_dir)
        .with_context(|| format!("Failed to read cache directory: {}", cache_dir.display()))?;

    let mut entries: Vec<CacheEntry> = Vec::new();

    for dir_entry in read_dir {
        let dir_entry = dir_entry.context("Failed to read directory entry")?;
        let path = dir_entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Skip .meta and .tmp files
        if file_name.ends_with(".meta") || file_name.ends_with(".tmp") {
            continue;
        }

        let metadata = fs::metadata(&path)
            .with_context(|| format!("Failed to read metadata for: {}", path.display()))?;

        let size_bytes = metadata.len();

        // Try to get filesystem timestamps as fallback
        let fs_created = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let fs_accessed = metadata
            .accessed()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Try to parse .meta sidecar
        let meta_path = format!("{}.meta", path.display());
        let (url, created_secs, last_accessed_secs) =
            if let Ok(meta_content) = fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<MetaFile>(&meta_content) {
                    (
                        meta.url,
                        meta.created_at.unwrap_or(fs_created),
                        meta.last_accessed.unwrap_or(fs_accessed),
                    )
                } else {
                    (None, fs_created, fs_accessed)
                }
            } else {
                (None, fs_created, fs_accessed)
            };

        entries.push(CacheEntry {
            path,
            file_name,
            size_bytes,
            created_secs,
            last_accessed_secs,
            url,
        });
    }

    // Sort by last_accessed descending (most recently accessed first)
    entries.sort_by(|a, b| b.last_accessed_secs.cmp(&a.last_accessed_secs));

    let total_size_bytes = entries.iter().map(|e| e.size_bytes).sum();
    let entry_count = entries.len();

    Ok(CacheSummary {
        total_size_bytes,
        entry_count,
        entries,
    })
}

pub fn delete_entry(path: &Path) -> Result<()> {
    fs::remove_file(path)
        .with_context(|| format!("Failed to delete file: {}", path.display()))?;

    let meta_path = format!("{}.meta", path.display());
    let meta_path = Path::new(&meta_path);
    if meta_path.exists() {
        fs::remove_file(meta_path)
            .with_context(|| format!("Failed to delete meta file: {}", meta_path.display()))?;
    }

    Ok(())
}

pub fn clear_cache(cache_dir: &Path) -> Result<()> {
    if !cache_dir.exists() {
        return Ok(());
    }

    let read_dir = fs::read_dir(cache_dir)
        .with_context(|| format!("Failed to read cache directory: {}", cache_dir.display()))?;

    for dir_entry in read_dir {
        let dir_entry = dir_entry.context("Failed to read directory entry")?;
        let path = dir_entry.path();
        if path.is_file() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to delete file: {}", path.display()))?;
        }
    }

    Ok(())
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        let val = bytes as f64 / MB as f64;
        // Use one decimal place for MB
        format!("{:.1} MB", val)
    } else if bytes >= KB {
        let val = bytes as f64 / KB as f64;
        // Use one decimal place for KB
        format!("{:.1} KB", val)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn scan_empty_dir() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let summary = scan_cache(tmp.path()).expect("scan failed");
        assert_eq!(summary.entry_count, 0);
        assert_eq!(summary.total_size_bytes, 0);
        assert!(summary.entries.is_empty());
    }

    #[test]
    fn scan_nonexistent_dir() {
        let summary = scan_cache(Path::new("/nonexistent/path/that/does/not/exist"))
            .expect("scan of nonexistent dir should return empty summary");
        assert_eq!(summary.entry_count, 0);
        assert_eq!(summary.total_size_bytes, 0);
    }

    #[test]
    fn scan_with_files() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");

        // Create video1.mp4 (1000 bytes)
        let video1 = tmp.path().join("video1.mp4");
        let mut f1 = fs::File::create(&video1).expect("create video1");
        f1.write_all(&vec![0u8; 1000]).expect("write video1");
        drop(f1);

        // Create video2.mp4 (2000 bytes)
        let video2 = tmp.path().join("video2.mp4");
        let mut f2 = fs::File::create(&video2).expect("create video2");
        f2.write_all(&vec![0u8; 2000]).expect("write video2");
        drop(f2);

        // Create video1.mp4.meta with url
        let meta1 = tmp.path().join("video1.mp4.meta");
        let mut mf = fs::File::create(&meta1).expect("create meta1");
        mf.write_all(br#"{"url":"https://example.com","created_at":1000,"last_accessed":2000}"#)
            .expect("write meta1");
        drop(mf);

        let summary = scan_cache(tmp.path()).expect("scan failed");

        assert_eq!(summary.entry_count, 2);
        assert_eq!(summary.total_size_bytes, 3000);

        let video1_entry = summary
            .entries
            .iter()
            .find(|e| e.file_name == "video1.mp4")
            .expect("video1 entry not found");

        assert_eq!(
            video1_entry.url.as_deref(),
            Some("https://example.com"),
            "video1 should have url from meta"
        );
    }

    #[test]
    fn format_size_values() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(2_621_440), "2.5 MB");
        assert_eq!(format_size(1_610_612_736), "1.50 GB");
    }

    #[test]
    fn delete_entry_removes_file_and_meta() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");

        let file_path = tmp.path().join("video.mp4");
        fs::write(&file_path, b"data").expect("write file");

        let meta_path = tmp.path().join("video.mp4.meta");
        fs::write(&meta_path, br#"{"url":"https://example.com"}"#).expect("write meta");

        assert!(file_path.exists());
        assert!(meta_path.exists());

        delete_entry(&file_path).expect("delete failed");

        assert!(!file_path.exists(), "file should be deleted");
        assert!(!meta_path.exists(), "meta file should be deleted");
    }
}
