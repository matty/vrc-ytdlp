use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use axum::body::Body;
use axum::http::{Response, StatusCode};
use tokio::sync::{broadcast, oneshot, Mutex};
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

/// Result of resolving a URL against the cache.
pub enum CacheOutcome {
    /// A valid cached file exists at this path.
    Hit(PathBuf),
    /// The caller owns the download and must run the pipeline, then call
    /// `finish_download` or `fail_download`.
    Owner,
}

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
    async fn start_download(
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

    /// Resolve a URL against the cache. Returns a cached file path, or makes
    /// the caller the owner of a new download. If another task is already
    /// downloading the same URL, waits for it and re-checks — so when an
    /// owner fails, exactly one waiter is promoted to owner.
    pub async fn acquire(&self, video_url: &str) -> CacheOutcome {
        loop {
            if let Some(path) = self.get(video_url).await {
                return CacheOutcome::Hit(path);
            }
            match self.start_download(video_url).await {
                None => return CacheOutcome::Owner,
                Some(mut waiter) => {
                    tracing::info!(url = %video_url, "waiting for in-progress download of same URL");
                    // Success or failure, loop back: re-check the cache and
                    // re-contend for ownership.
                    let _ = waiter.recv().await;
                }
            }
        }
    }

    /// Called when download+remux completes successfully.
    /// Renames .tmp → .mp4, writes .meta, inserts into index, notifies waiters.
    /// On failure, notifies waiters and clears the inflight slot so future
    /// requests don't wait forever.
    pub async fn finish_download(&self, video_url: &str, tmp_path: &Path) -> Result<PathBuf> {
        match self.try_finish(video_url, tmp_path).await {
            Ok(path) => Ok(path),
            Err(e) => {
                self.fail_download(video_url, &e.to_string()).await;
                Err(e)
            }
        }
    }

    async fn try_finish(&self, video_url: &str, tmp_path: &Path) -> Result<PathBuf> {
        let key = cache_key(video_url);
        let final_path = self.cache_dir.join(format!("{key}.mp4"));

        // Rename tmp → final (atomic on most filesystems)
        std::fs::rename(tmp_path, &final_path).context("renaming cache temp file to final")?;

        // Write sidecar meta file with the URL
        let meta = meta_path(&final_path);
        let _ = std::fs::write(&meta, video_url);

        let size = std::fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);

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
/// cache entry only if `pipeline_status` reports the pipeline processes exited
/// successfully — otherwise the partial file is discarded. On drop without
/// completion, cleans up the temp file.
pub fn tee_stream(
    stdout: tokio::process::ChildStdout,
    cache_file: std::fs::File,
    cache: Arc<VideoCache>,
    video_url: String,
    pipeline_status: oneshot::Receiver<bool>,
) -> TeeStream {
    TeeStream {
        inner: ReaderStream::new(stdout),
        writer: Some(std::io::BufWriter::with_capacity(256 * 1024, cache_file)),
        cache,
        video_url,
        pipeline_status: Some(pipeline_status),
        completed: false,
    }
}

pin_project_lite::pin_project! {
    /// Stream that tees ffmpeg output to a cache file while yielding to HTTP response.
    pub struct TeeStream {
        #[pin]
        inner: ReaderStream<tokio::process::ChildStdout>,
        writer: Option<std::io::BufWriter<std::fs::File>>,
        cache: Arc<VideoCache>,
        video_url: String,
        pipeline_status: Option<oneshot::Receiver<bool>>,
        completed: bool,
    }

    impl PinnedDrop for TeeStream {
        fn drop(this: Pin<&mut Self>) {
            let this = this.project();
            if !*this.completed {
                let cache = this.cache.clone();
                let url = this.video_url.clone();
                // Drop can run outside a runtime during shutdown — skip
                // cleanup there rather than panic.
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        cache
                            .fail_download(&url, "stream dropped before completion")
                            .await;
                    });
                }
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
                if let Some(w) = this.writer.as_mut() {
                    if w.write_all(&bytes).is_err() {
                        // Caching is best-effort: stop writing but keep
                        // streaming. Dropping the status receiver makes the
                        // completion path discard the truncated file instead
                        // of caching it.
                        *this.writer = None;
                        *this.pipeline_status = None;
                    }
                }
                std::task::Poll::Ready(Some(Ok(bytes)))
            }
            std::task::Poll::Ready(Some(Err(e))) => std::task::Poll::Ready(Some(Err(e))),
            std::task::Poll::Ready(None) => {
                if let Some(mut w) = this.writer.take() {
                    let _ = w.flush();
                }
                *this.completed = true;

                let cache = this.cache.clone();
                let url = this.video_url.clone();
                let status = this.pipeline_status.take();
                tokio::spawn(async move {
                    // EOF only says the output pipe closed — wait for the
                    // actual exit status so partial output from a failed
                    // pipeline is never cached.
                    let success = match status {
                        Some(rx) => tokio::time::timeout(Duration::from_secs(30), rx)
                            .await
                            .map(|r| r.unwrap_or(false))
                            .unwrap_or(false),
                        None => false,
                    };

                    if success {
                        let tmp = cache.tmp_path(&url);
                        if let Err(e) = cache.finish_download(&url, &tmp).await {
                            tracing::warn!(error = %e, "failed to finalize cache entry");
                        }
                    } else {
                        tracing::warn!(url = %url, "pipeline exited with error, discarding partial cache file");
                        cache
                            .fail_download(&url, "pipeline exited with error")
                            .await;
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
            let size = file.metadata().await.map(|m| m.len()).ok();
            let stream = ReaderStream::new(file);
            let mut builder = Response::builder().header("content-type", "video/mp4");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(tag: &str) -> PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("vrc-ytdlp-test-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn cache_key_is_deterministic_hex() {
        let a = cache_key("https://example.com/v");
        let b = cache_key("https://example.com/v");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn finish_download_failure_notifies_waiters() {
        let cache = VideoCache::new(test_dir("finish-fail"), 10, 3600)
            .await
            .unwrap();
        let url = "https://example.com/video";

        assert!(
            cache.start_download(url).await.is_none(),
            "first caller should become the download owner"
        );
        let mut waiter = cache
            .start_download(url)
            .await
            .expect("second caller should get a waiter");

        // Finalize with a temp file that doesn't exist — the rename fails.
        let missing = cache.tmp_path(url);
        assert!(cache.finish_download(url, &missing).await.is_err());

        let notified = tokio::time::timeout(std::time::Duration::from_secs(2), waiter.recv()).await;
        match notified {
            Ok(Ok(Err(_))) => {} // waiter told the download failed — correct
            other => panic!("waiter was not notified of the failure: {other:?}"),
        }

        // The inflight slot must be free again so a retry can become owner.
        assert!(
            cache.start_download(url).await.is_none(),
            "inflight entry should be cleared after a failed finalize"
        );
    }

    #[tokio::test]
    async fn acquire_promotes_waiter_after_owner_failure() {
        let cache = Arc::new(
            VideoCache::new(test_dir("acquire-fail"), 10, 3600)
                .await
                .unwrap(),
        );
        let url = "https://example.com/promote";

        assert!(matches!(cache.acquire(url).await, CacheOutcome::Owner));

        let cache2 = cache.clone();
        let waiter = tokio::spawn(async move { cache2.acquire(url).await });

        tokio::time::sleep(Duration::from_millis(100)).await;
        cache.fail_download(url, "boom").await;

        let outcome = tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("waiter should resolve after owner failure")
            .unwrap();
        assert!(
            matches!(outcome, CacheOutcome::Owner),
            "waiter should be promoted to download owner"
        );
    }

    #[tokio::test]
    async fn acquire_returns_hit_after_owner_finishes() {
        let cache = Arc::new(
            VideoCache::new(test_dir("acquire-hit"), 10, 3600)
                .await
                .unwrap(),
        );
        let url = "https://example.com/hit";

        assert!(matches!(cache.acquire(url).await, CacheOutcome::Owner));
        let tmp = cache.tmp_path(url);
        std::fs::write(&tmp, b"video data").unwrap();

        let cache2 = cache.clone();
        let waiter = tokio::spawn(async move { cache2.acquire(url).await });

        tokio::time::sleep(Duration::from_millis(100)).await;
        cache.finish_download(url, &tmp).await.unwrap();

        let outcome = tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("waiter should resolve after owner success")
            .unwrap();
        match outcome {
            CacheOutcome::Hit(path) => {
                assert_eq!(std::fs::read(path).unwrap(), b"video data");
            }
            CacheOutcome::Owner => panic!("waiter should get a cache hit, not ownership"),
        }
    }

    // --- TeeStream: only a successful pipeline may populate the cache ---

    #[cfg(windows)]
    async fn run_tee(
        cache: &Arc<VideoCache>,
        url: &str,
        pipeline_success: bool,
    ) -> tokio::process::Child {
        use futures_core::Stream;

        let tmp = cache.tmp_path(url);
        let file = std::fs::File::create(&tmp).unwrap();

        let mut child = tokio::process::Command::new("cmd")
            .args(["/C", "echo hello"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();

        let (tx, rx) = oneshot::channel();
        tx.send(pipeline_success).unwrap();

        let stream = tee_stream(stdout, file, cache.clone(), url.to_string(), rx);
        let mut stream = Box::pin(stream);
        while let Some(item) = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
            item.expect("stream chunk");
        }
        child
    }

    #[cfg(windows)]
    async fn wait_for<F: Fn() -> bool>(cond: F, what: &str) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !cond() {
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for: {what}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn tee_stream_caches_output_when_pipeline_succeeds() {
        let cache = Arc::new(VideoCache::new(test_dir("tee-ok"), 10, 3600).await.unwrap());
        let url = "https://example.com/tee-ok";
        assert!(cache.start_download(url).await.is_none());

        let _child = run_tee(&cache, url, true).await;

        let cache2 = cache.clone();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let path = loop {
            if let Some(p) = cache2.get(url).await {
                break p;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "output was never cached"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("hello"));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn tee_stream_discards_partial_file_when_pipeline_fails() {
        let cache = Arc::new(
            VideoCache::new(test_dir("tee-fail"), 10, 3600)
                .await
                .unwrap(),
        );
        let url = "https://example.com/tee-fail";
        assert!(cache.start_download(url).await.is_none());
        let mut waiter = cache.start_download(url).await.expect("waiter");

        let tmp = cache.tmp_path(url);
        let _child = run_tee(&cache, url, false).await;

        // Waiters must learn about the failure...
        let notified = tokio::time::timeout(Duration::from_secs(5), waiter.recv()).await;
        assert!(
            matches!(notified, Ok(Ok(Err(_)))),
            "waiter should be notified of pipeline failure: {notified:?}"
        );

        // ...and neither the partial temp file nor a cache entry may survive.
        wait_for(|| !tmp.exists(), "partial temp file removal").await;
        assert!(
            cache.get(url).await.is_none(),
            "failed pipeline output must not be cached"
        );
    }
}
