use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cache::{self, VideoCache};
use crate::pipeline::{self, PipelineConfig};

// --- State ---

struct StreamEntry {
    /// Original video URL (e.g. YouTube URL)
    video_url: String,
    /// yt-dlp args (format, cookies, etc.) — no --get-url, no video URL
    ytdlp_args: Vec<String>,
}

pub struct ServerConfig {
    pub ytdlp_path: PathBuf,
    pub ffmpeg_path: PathBuf,
    pub plugin_dirs: Option<PathBuf>,
    pub extractor_args: Vec<String>,
    pub cache_dir: PathBuf,
    pub cache_max_size_mb: u64,
    pub cache_ttl_secs: u64,
}

struct AppState {
    streams: tokio::sync::Mutex<HashMap<String, StreamEntry>>,
    next_id: AtomicU64,
    last_activity: AtomicU64,
    active_pipelines: AtomicU64,
    server_config: ServerConfig,
    cache: Arc<VideoCache>,
}

impl AppState {
    async fn new(config: ServerConfig) -> Result<Self> {
        let cache = VideoCache::new(
            config.cache_dir.clone(),
            config.cache_max_size_mb,
            config.cache_ttl_secs,
        )
        .await?;

        Ok(Self {
            streams: tokio::sync::Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            last_activity: AtomicU64::new(now_secs()),
            active_pipelines: AtomicU64::new(0),
            server_config: config,
            cache: Arc::new(cache),
        })
    }

    fn touch(&self) {
        self.last_activity.store(now_secs(), Ordering::Relaxed);
    }

    fn idle_secs(&self) -> u64 {
        now_secs().saturating_sub(self.last_activity.load(Ordering::Relaxed))
    }
}

use crate::util::now_secs;

// --- Routes ---

#[derive(Deserialize)]
struct RegisterRequest {
    video_url: String,
    #[serde(default)]
    #[allow(dead_code)]
    ytdlp_path: String, // kept for backwards compat, ignored in favor of server config
    ytdlp_args: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct RegisterResponse {
    id: String,
}

async fn health() -> &'static str {
    "ok"
}

async fn register_stream(
    State(state): State<Arc<AppState>>,
    axum::Json(req): axum::Json<RegisterRequest>,
) -> impl IntoResponse {
    state.touch();
    let id = state
        .next_id
        .fetch_add(1, Ordering::Relaxed)
        .to_string();

    tracing::info!(id = %id, video_url = %req.video_url, "registered stream");

    state.streams.lock().await.insert(
        id.clone(),
        StreamEntry {
            video_url: req.video_url,
            ytdlp_args: req.ytdlp_args,
        },
    );

    axum::Json(RegisterResponse { id })
}

async fn stream_video(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    state.touch();

    let entry = {
        let streams = state.streams.lock().await;
        streams
            .get(&id)
            .map(|e| (e.video_url.clone(), e.ytdlp_args.clone()))
    };

    let (video_url, ytdlp_args) = match entry {
        Some(e) => e,
        None => {
            tracing::warn!(id = %id, "stream not found");
            return error_response(StatusCode::NOT_FOUND, "stream not found");
        }
    };

    // --- Check cache ---
    if let Some(cached_path) = state.cache.get(&video_url).await {
        tracing::info!(id = %id, video_url = %video_url, "serving from cache");
        state.streams.lock().await.remove(&id);
        return cache::serve_cached_file(&cached_path).await;
    }

    // --- Check if another request is already downloading this URL ---
    if let Some(mut waiter) = state.cache.start_download(&video_url).await {
        tracing::info!(id = %id, "waiting for in-progress download of same URL");
        match waiter.recv().await {
            Ok(Ok(())) => {
                if let Some(cached_path) = state.cache.get(&video_url).await {
                    tracing::info!(id = %id, "serving from cache after wait");
                    state.streams.lock().await.remove(&id);
                    return cache::serve_cached_file(&cached_path).await;
                }
            }
            _ => {
                tracing::warn!(id = %id, "waited download failed, starting own pipeline");
            }
        }
    }

    // --- Cache miss: run the pipeline ---
    tracing::info!(id = %id, video_url = %video_url, "cache miss, starting pipeline");

    let pipeline_config = PipelineConfig {
        ytdlp_path: state.server_config.ytdlp_path.clone(),
        ffmpeg_path: state.server_config.ffmpeg_path.clone(),
        ytdlp_args,
        plugin_dirs: state.server_config.plugin_dirs.clone(),
        extractor_args: state.server_config.extractor_args.clone(),
    };

    state.active_pipelines.fetch_add(1, Ordering::Relaxed);

    let handle = match pipeline::start_pipeline(&pipeline_config, &video_url, &id).await {
        Ok(h) => h,
        Err(e) => {
            state.active_pipelines.fetch_sub(1, Ordering::Relaxed);
            state.cache.fail_download(&video_url, &e.to_string()).await;
            tracing::error!(id = %id, error = %e, "pipeline failed");
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("pipeline failed: {e}"),
            );
        }
    };

    let ffmpeg_stdout = handle.into_monitored(id.clone());

    // Decrement active count and clean up stream entry
    let state_clone = state.clone();
    let id_clone = id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        state_clone.active_pipelines.fetch_sub(1, Ordering::Relaxed);
        state_clone.streams.lock().await.remove(&id_clone);
        state_clone.touch();
        tracing::debug!(id = %id_clone, "stream cleaned up");
    });

    // Tee ffmpeg output to cache file while streaming to client
    let cache_tmp = state.cache.tmp_path(&video_url);
    let body = match std::fs::File::create(&cache_tmp) {
        Ok(file) => {
            let stream = cache::tee_stream(
                ffmpeg_stdout,
                file,
                state.cache.clone(),
                video_url,
            );
            Body::from_stream(stream)
        }
        Err(e) => {
            tracing::warn!(error = %e, "couldn't create cache file, streaming without caching");
            state.cache.fail_download(&video_url, "cache file creation failed").await;
            Body::from_stream(ReaderStream::new(ffmpeg_stdout))
        }
    };

    tracing::info!(id = %id, "streaming response started");

    Response::builder()
        .header("content-type", "video/mp4")
        .header("cache-control", "no-cache")
        .body(body)
        .unwrap()
}

fn error_response(status: StatusCode, msg: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(msg.to_string()))
        .unwrap()
}

// --- Server Entry Point ---

pub async fn run_server(
    port: u16,
    idle_timeout_secs: u64,
    server_config: ServerConfig,
    shutdown: CancellationToken,
) -> Result<()> {
    let state = Arc::new(
        AppState::new(server_config)
            .await
            .context("initializing server state")?,
    );

    // Start background cache eviction
    cache::spawn_eviction_task(state.cache.clone(), shutdown.clone());

    let app = Router::new()
        .route("/health", get(health))
        .route("/stream", post(register_stream))
        .route("/stream/{id}", get(stream_video))
        .with_state(state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context(format!("binding to {addr}"))?;

    tracing::info!(addr = %addr, "media server started");

    // Idle timeout watcher — cancels the shutdown token instead of exit(0)
    let state_for_timeout = state.clone();
    let idle_shutdown = shutdown.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            if idle_shutdown.is_cancelled() {
                return;
            }
            let idle = state_for_timeout.idle_secs();
            let active = state_for_timeout.active_pipelines.load(Ordering::Relaxed);
            if idle >= idle_timeout_secs && active == 0 {
                tracing::info!(idle_secs = idle, "server idle timeout reached, shutting down");
                idle_shutdown.cancel();
                return;
            }
        }
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await
        .context("running media server")
}

// --- Client helpers (used by the wrapper to talk to the server) ---

pub async fn check_server_health(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/health");
    reqwest::get(&url)
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub fn spawn_server_process(port: u16, idle_timeout_secs: u64) -> Result<()> {
    let exe = std::env::current_exe().context("getting current exe path")?;

    tracing::debug!(exe = %exe.display(), port, idle_timeout_secs, "spawning detached server process");

    let mut cmd = std::process::Command::new(exe);
    cmd.args([
        "--serve",
        "--port",
        &port.to_string(),
        "--idle-timeout",
        &idle_timeout_secs.to_string(),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    // Ensure the detached server inherits a valid temp directory
    .env("TEMP", std::env::temp_dir())
    .env("TMP", std::env::temp_dir());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }

    cmd.spawn().context("spawning server process")?;

    tracing::debug!("server process spawned");
    Ok(())
}

pub async fn register_stream_with_server(
    port: u16,
    video_url: &str,
    ytdlp_path: &str,
    ytdlp_args: &[String],
) -> Result<String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "video_url": video_url,
        "ytdlp_path": ytdlp_path,
        "ytdlp_args": ytdlp_args,
    });

    tracing::debug!(port, video_url, "registering stream with server");

    let resp: RegisterResponse = client
        .post(format!("http://127.0.0.1:{port}/stream"))
        .json(&body)
        .send()
        .await
        .context("posting stream to server")?
        .error_for_status()
        .context("server returned error")?
        .json()
        .await
        .context("parsing server response")?;

    tracing::debug!(id = %resp.id, "stream registered");
    Ok(resp.id)
}

pub fn stream_url(port: u16, id: &str) -> String {
    format!("http://127.0.0.1:{port}/stream/{id}")
}
