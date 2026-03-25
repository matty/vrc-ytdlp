use std::collections::HashMap;
use std::net::SocketAddr;
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
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_util::io::ReaderStream;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// --- State ---

struct StreamEntry {
    /// Original video URL (e.g. YouTube URL)
    video_url: String,
    /// Path to yt-dlp executable
    ytdlp_path: String,
    /// Args to pass to yt-dlp (format, cookies, etc.) — no --get-url, no URL
    ytdlp_args: Vec<String>,
}

struct AppState {
    streams: Mutex<HashMap<String, StreamEntry>>,
    next_id: AtomicU64,
    last_activity: AtomicU64,
}

impl AppState {
    fn new() -> Self {
        Self {
            streams: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            last_activity: AtomicU64::new(now_secs()),
        }
    }

    fn touch(&self) {
        self.last_activity.store(now_secs(), Ordering::Relaxed);
    }

    fn idle_secs(&self) -> u64 {
        now_secs().saturating_sub(self.last_activity.load(Ordering::Relaxed))
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// --- Routes ---

#[derive(Deserialize)]
struct RegisterRequest {
    video_url: String,
    ytdlp_path: String,
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
    tracing::debug!(id = %id, ytdlp_path = %req.ytdlp_path, ytdlp_args = ?req.ytdlp_args, "stream details");

    state.streams.lock().await.insert(
        id.clone(),
        StreamEntry {
            video_url: req.video_url,
            ytdlp_path: req.ytdlp_path,
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
        streams.get(&id).map(|e| (e.video_url.clone(), e.ytdlp_path.clone(), e.ytdlp_args.clone()))
    };

    let (video_url, ytdlp_path, ytdlp_args) = match entry {
        Some(e) => e,
        None => {
            tracing::warn!(id = %id, "stream not found");
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("stream not found"))
                .unwrap();
        }
    };

    tracing::info!(id = %id, video_url = %video_url, "starting yt-dlp → ffmpeg pipeline");

    // Build yt-dlp args: download to stdout, piped to ffmpeg
    // - Strip --get-url (we're downloading, not resolving)
    // - Strip URLs (we add the video_url ourselves)
    // - Replace -f with best quality (yt-dlp merges video+audio internally)
    // - Add --logtostderr so logs don't corrupt video data on stdout
    // - Add -o - to output video to stdout
    let mut ytdlp_full_args: Vec<String> = Vec::new();
    let mut skip_next = false;
    for arg in &ytdlp_args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--get-url" {
            continue;
        }
        if arg == "-f" {
            skip_next = true; // skip the original format value, we'll add our own
            continue;
        }
        // Strip any URLs (we add video_url at the end)
        if arg.starts_with("http://") || arg.starts_with("https://") {
            continue;
        }
        ytdlp_full_args.push(arg.clone());
    }
    ytdlp_full_args.push("-f".to_string());
    ytdlp_full_args.push("bv*[height<=1080]+ba/b[height<=1080]/b".to_string());
    ytdlp_full_args.push("--logtostderr".to_string());
    ytdlp_full_args.push("-o".to_string());
    ytdlp_full_args.push("-".to_string());
    ytdlp_full_args.push(video_url.clone());

    tracing::debug!(id = %id, ytdlp_args = ?ytdlp_full_args, "spawning yt-dlp");

    // Set up tmp dir next to yt-dlp
    let ytdlp_dir = std::path::Path::new(&ytdlp_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let tmp_dir = ytdlp_dir.join("tmp");
    let _ = std::fs::create_dir_all(&tmp_dir);

    // Spawn yt-dlp (std::process) — downloads video to stdout
    let mut ytdlp_child = match std::process::Command::new(&ytdlp_path)
        .args(&ytdlp_full_args)
        .current_dir(ytdlp_dir)
        .env("TEMP", &tmp_dir)
        .env("TMP", &tmp_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => {
            tracing::debug!(id = %id, "yt-dlp process spawned");
            c
        }
        Err(e) => {
            tracing::error!(id = %id, error = %e, "failed to spawn yt-dlp");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("failed to spawn yt-dlp: {e}")))
                .unwrap();
        }
    };

    let ytdlp_stdout = ytdlp_child.stdout.take().unwrap();

    // Spawn ffmpeg (tokio) — reads from yt-dlp stdout via OS pipe, remuxes to fragmented mp4
    tracing::debug!(id = %id, "spawning ffmpeg");

    let mut ffmpeg_child = match Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "warning",
            "-i",
            "pipe:0",
            "-c",
            "copy",
            "-movflags",
            "frag_mp4+empty_moov+default_base_moof",
            "-f",
            "mp4",
            "pipe:1",
        ])
        .stdin(ytdlp_stdout)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => {
            tracing::debug!(id = %id, "ffmpeg process spawned");
            c
        }
        Err(e) => {
            tracing::error!(id = %id, error = %e, "failed to spawn ffmpeg");
            let _ = ytdlp_child.kill();
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("failed to spawn ffmpeg: {e}")))
                .unwrap();
        }
    };

    let ffmpeg_stdout = match ffmpeg_child.stdout.take() {
        Some(s) => s,
        None => {
            tracing::error!(id = %id, "no stdout from ffmpeg");
            let _ = ytdlp_child.kill();
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("no stdout from ffmpeg"))
                .unwrap();
        }
    };

    // Background task: wait for both processes to finish, clean up
    let state_clone = state.clone();
    let id_clone = id.clone();
    tokio::spawn(async move {
        // Wait for ffmpeg (which waits for yt-dlp via pipe EOF)
        let ffmpeg_status = ffmpeg_child.wait().await;
        match &ffmpeg_status {
            Ok(s) => tracing::info!(id = %id_clone, status = %s, "ffmpeg finished"),
            Err(e) => tracing::error!(id = %id_clone, error = %e, "ffmpeg wait failed"),
        }

        // yt-dlp should already be done, but reap it
        let _ = ytdlp_child.wait();

        state_clone.touch();
        state_clone.streams.lock().await.remove(&id_clone);
        tracing::debug!(id = %id_clone, "stream cleaned up");
    });

    let stream = ReaderStream::new(ffmpeg_stdout);
    let body = Body::from_stream(stream);

    tracing::debug!(id = %id, "streaming response started");

    Response::builder()
        .header("content-type", "video/mp4")
        .header("cache-control", "no-cache")
        .body(body)
        .unwrap()
}

// --- Server Entry Point ---

pub async fn run_server(port: u16, idle_timeout_secs: u64) -> Result<()> {
    let state = Arc::new(AppState::new());

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

    // Idle timeout watcher — only shuts down when idle AND no active streams
    let state_for_timeout = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let idle = state_for_timeout.idle_secs();
            let active = state_for_timeout.streams.lock().await.len();
            if idle >= idle_timeout_secs && active == 0 {
                tracing::info!(idle_secs = idle, "server idle timeout reached, shutting down");
                std::process::exit(0);
            }
        }
    });

    axum::serve(listener, app)
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

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const DETACHED_PROCESS: u32 = 0x00000008;

        std::process::Command::new(exe)
            .args([
                "--serve",
                "--port",
                &port.to_string(),
                "--idle-timeout",
                &idle_timeout_secs.to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
            .spawn()
            .context("spawning server process")?;
    }

    #[cfg(not(windows))]
    {
        std::process::Command::new(exe)
            .args([
                "--serve",
                "--port",
                &port.to_string(),
                "--idle-timeout",
                &idle_timeout_secs.to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("spawning server process")?;
    }

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
