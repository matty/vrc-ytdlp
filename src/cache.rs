use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use axum::body::Body;
use axum::http::{Response, StatusCode};
use tokio::sync::{broadcast, Mutex};
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;

use crate::util::now_secs;

// ---------------------------------------------------------------------------
// Cache entry
// ---------------------------------------------------------------------------

struct CacheEntry {
    url: String,
    path: PathBuf,
    size: u64,
    created_at: u64,
    last_accessed: u64,
}

// ---------------------------------------------------------------------------
// VideoCache
// ---------------------------------------------------------------------------

pub struct VideoCache {
    cache_dir: PathBuf,
    max_size_bytes: u64,
    ttl_secs: u64,
    index: Mutex<HashMap<String, CacheEntry>>,
    inflight: Mutex<HashMap<String, broadcast::Sender<Result<(), String>>>>,
}

impl VideoCache {
    /// Create the cache, ensure directory exists, rebuild index from disk.
    pub async fn new(cache_dir: PathBuf, max_size_mb: u64, ttl_secs: u64) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir).context("creating cache directory")?;

        let cache = Self {
            cache_dir,
            max_size_bytes: max_size_mb * 1024 * 1024,
            ttl_secs,
            index: Mutex::new(HashMap::new()),
            inflight: Mutex::new(HashMap::new()),
        };

        cache.rebuild_index().await;
        Ok(cache)
    }

    /// Check for a valid (non-expired) cached entry. Updates last_accessed on hit.
    pub async fn get(&self, video_url: &str) -> Option<PathBuf> {
        let key = cache_key(video_url);
        let mut index = self.index.lock().await;

        let entry = index.get_mut(&key)?;

        // Check TTL
        let now = now_secs();
        if now.saturating_sub(entry.created_at) > self.ttl_secs {
            // Expired — remove it
            let entry = index.remove(&key).unwrap();
            let _ = std::fs::remove_file(&entry.path);
            let _ = std::fs::remove_file(meta_path(&entry.path));
            tracing::debug!(url = %video_url, "cache entry expired");
            return None;
        }

        // Verify file still exists and is non-empty
        if !entry.path.exists() || entry.size == 0 {
            let entry = index.remove(&key).unwrap();
            let _ = std::fs::remove_file(&entry.path);
            let _ = std::fs::remove_file(meta_path(&entry.path));
            return None;
        }

        entry.last_accessed = now;
        tracing::info!(url = %video_url, size = entry.size, "cache hit");
        Some(entry.path.clone())
    }

    /// Register that a download is starting.
    /// Returns None if no other download is in progress (caller should proceed).
    /// Returns Some(receiver) if another task is downloading (caller should wait).
    pub async fn start_download(
        &self,
        video_url: &str,
    ) -> Option<broadcast::Receiver<Result<(), String>>> {
        let key = cache_key(video_url);
        let mut inflight = self.inflight.lock().await;

        if let Some(sender) = inflight.get(&key) {
            return Some(sender.subscribe());
        }

        // No inflight — register ourselves
        let (tx, _) = broadcast::channel(16);
        inflight.insert(key, tx);
        None
    }

    /// Called when download+remux completes successfully.
    /// Renames .tmp → .mp4, writes .meta, inserts into index, notifies waiters.
    pub async fn finish_download(&self, video_url: &str, tmp_path: &Path) -> Result<PathBuf> {
        let key = cache_key(video_url);
        let final_path = self.cache_dir.join(format!("{key}.mp4"));

        // Rename tmp → final (atomic on most filesystems)
        std::fs::rename(tmp_path, &final_path)
            .context("renaming cache temp file to final")?;

        // Write sidecar meta file with the URL
        let meta = meta_path(&final_path);
        let _ = std::fs::write(&meta, video_url);

        let size = std::fs::metadata(&final_path)
            .map(|m| m.len())
            .unwrap_or(0);

        if size == 0 {
            let _ = std::fs::remove_file(&final_path);
            let _ = std::fs::remove_file(&meta);
            bail!("refusing to cache empty file");
        }

        let now = now_secs();

        // Insert into index
        {
            let mut index = self.index.lock().await;
            index.insert(
                key.clone(),
                CacheEntry {
                    url: video_url.to_string(),
                    path: final_path.clone(),
                    size,
                    created_at: now,
                    last_accessed: now,
                },
            );
        }

        tracing::info!(url = %video_url, size, "cached video");

        // Notify waiters
        {
            let mut inflight = self.inflight.lock().await;
            if let Some(tx) = inflight.remove(&key) {
                let _ = tx.send(Ok(()));
            }
        }

        // Evict if needed
        self.evict().await;

        Ok(final_path)
    }

    /// Called when a download fails. Cleans up and notifies waiters.
    pub async fn fail_download(&self, video_url: &str, error: &str) {
        let key = cache_key(video_url);

        // Clean up temp file
        let tmp_path = self.cache_dir.join(format!("{key}.tmp"));
        let _ = std::fs::remove_file(&tmp_path);

        // Notify waiters of failure
        let mut inflight = self.inflight.lock().await;
        if let Some(tx) = inflight.remove(&key) {
            let _ = tx.send(Err(error.to_string()));
        }
    }

    /// Get the temp file path for an in-progress download.
    pub fn tmp_path(&self, video_url: &str) -> PathBuf {
        let key = cache_key(video_url);
        self.cache_dir.join(format!("{key}.tmp"))
    }

    /// Evict expired entries (TTL) then over-limit entries (LRU).
    pub async fn evict(&self) {
        // Collect paths to delete under lock, then release lock before doing I/O
        let to_delete: Vec<PathBuf>;
        {
            let mut index = self.index.lock().await;
            let now = now_secs();
            let mut remove_keys = Vec::new();

            // Phase 1: TTL expiry
            for (key, entry) in index.iter() {
                if now.saturating_sub(entry.created_at) > self.ttl_secs {
                    remove_keys.push(key.clone());
                }
            }

            // Phase 2: LRU if over size limit
            let mut total: u64 = index.values().map(|e| e.size).sum();
            if total > self.max_size_bytes {
                let mut by_access: Vec<(String, u64, u64)> = index
                    .iter()
                    .filter(|(k, _)| !remove_keys.contains(k))
                    .map(|(k, e)| (k.clone(), e.last_accessed, e.size))
                    .collect();
                by_access.sort_by_key(|(_, ts, _)| *ts);

                for (key, _, size) in by_access {
                    if total <= self.max_size_bytes {
                        break;
                    }
                    total -= size;
                    remove_keys.push(key);
                }
            }

            // Remove from index and collect paths
            to_delete = remove_keys
                .iter()
                .filter_map(|key| index.remove(key))
                .flat_map(|entry| {
                    tracing::debug!(url = %entry.url, "evicting cache entry");
                    [entry.path.clone(), meta_path(&entry.path)]
                })
                .collect();
        }
        // Lock released — do blocking I/O outside the lock
        for path in &to_delete {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Scan cache directory and rebuild the in-memory index.
    async fn rebuild_index(&self) {
        let entries = match std::fs::read_dir(&self.cache_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut index = self.index.lock().await;
        let mut recovered = 0u32;

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Clean up orphaned .tmp files
            if name.ends_with(".tmp") {
                let _ = std::fs::remove_file(&path);
                continue;
            }

            // Only process .mp4 files
            if !name.ends_with(".mp4") {
                continue;
            }

            let key = name.trim_end_matches(".mp4").to_string();

            // Read URL from sidecar .meta file
            let meta = meta_path(&path);
            let url = std::fs::read_to_string(&meta).unwrap_or_else(|_| "<unknown>".into());
            let url = url.trim().to_string();

            let metadata = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let mtime = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            index.insert(
                key,
                CacheEntry {
                    url,
                    path,
                    size: metadata.len(),
                    created_at: mtime,
                    last_accessed: mtime,
                },
            );
            recovered += 1;
        }

        if recovered > 0 {
            tracing::info!(entries = recovered, "rebuilt cache index from disk");
        }
    }
}

// ---------------------------------------------------------------------------
// Tee stream — writes to cache file while yielding to HTTP response
// ---------------------------------------------------------------------------

/// Create a stream that reads from ffmpeg stdout, writes each chunk to a cache
/// file, and yields chunks to the HTTP response. On completion, finalizes the
/// cache entry. On drop without completion, cleans up the temp file.
pub fn tee_stream(
    stdout: tokio::process::ChildStdout,
    cache_file: std::fs::File,
    cache: Arc<VideoCache>,
    video_url: String,
) -> TeeStream {
    let writer = std::sync::Mutex::new(Some(std::io::BufWriter::with_capacity(
        256 * 1024,
        cache_file,
    )));

    TeeStream {
        inner: ReaderStream::new(stdout),
        writer,
        cache,
        video_url,
        completed: std::sync::atomic::AtomicBool::new(false),
    }
}

pin_project_lite::pin_project! {
    /// Stream that tees ffmpeg output to a cache file while yielding to HTTP response.
    pub struct TeeStream {
        #[pin]
        inner: ReaderStream<tokio::process::ChildStdout>,
        writer: std::sync::Mutex<Option<std::io::BufWriter<std::fs::File>>>,
        cache: Arc<VideoCache>,
        video_url: String,
        completed: std::sync::atomic::AtomicBool,
    }

    impl PinnedDrop for TeeStream {
        fn drop(this: Pin<&mut Self>) {
            let this = this.project();
            if !this.completed.load(std::sync::atomic::Ordering::Relaxed) {
                let cache = this.cache.clone();
                let url = this.video_url.clone();
                tokio::spawn(async move {
                    cache
                        .fail_download(&url, "stream dropped before completion")
                        .await;
                });
            }
        }
    }
}

impl futures_core::Stream for TeeStream {
    type Item = Result<tokio_util::bytes::Bytes, std::io::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(bytes))) => {
                if let Ok(mut guard) = this.writer.lock() {
                    if let Some(ref mut w) = *guard {
                        if w.write_all(&bytes).is_err() {
                            let _ = guard.take();
                        }
                    }
                }
                std::task::Poll::Ready(Some(Ok(bytes)))
            }
            std::task::Poll::Ready(Some(Err(e))) => std::task::Poll::Ready(Some(Err(e))),
            std::task::Poll::Ready(None) => {
                if let Ok(mut guard) = this.writer.lock() {
                    if let Some(mut w) = guard.take() {
                        let _ = w.flush();
                    }
                }
                this.completed
                    .store(true, std::sync::atomic::Ordering::Relaxed);

                let cache = this.cache.clone();
                let url = this.video_url.clone();
                tokio::spawn(async move {
                    let tmp = cache.tmp_path(&url);
                    if let Err(e) = cache.finish_download(&url, &tmp).await {
                        tracing::warn!(error = %e, "failed to finalize cache entry");
                    }
                });

                std::task::Poll::Ready(None)
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}


// ---------------------------------------------------------------------------
// Serve from cache
// ---------------------------------------------------------------------------

pub async fn serve_cached_file(path: &Path) -> Response<Body> {
    match tokio::fs::File::open(path).await {
        Ok(file) => {
            let size = file
                .metadata()
                .await
                .map(|m| m.len())
                .ok();
            let stream = ReaderStream::new(file);
            let mut builder = Response::builder()
                .header("content-type", "video/mp4");
            if let Some(len) = size {
                builder = builder.header("content-length", len);
            }
            builder.body(Body::from_stream(stream)).unwrap()
        }
        Err(e) => {
            tracing::error!(error = %e, path = %path.display(), "failed to open cached file");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("cache read error"))
                .unwrap()
        }
    }
}

// ---------------------------------------------------------------------------
// Background eviction task
// ---------------------------------------------------------------------------

pub fn spawn_eviction_task(cache: Arc<VideoCache>, shutdown: CancellationToken) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    cache.evict().await;
                }
                _ = shutdown.cancelled() => return,
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Stable FNV-1a hash — deterministic across Rust versions and restarts.
fn cache_key(video_url: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in video_url.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn meta_path(mp4_path: &Path) -> PathBuf {
    mp4_path.with_extension("meta")
}
