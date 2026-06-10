//! HTTP helpers the wrapper CLI uses to talk to (and spawn) the media server.

use std::process::Stdio;

use anyhow::{Context, Result};

use super::RegisterResponse;

pub async fn check_server_health(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/health");
    reqwest::get(&url)
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Spawn a detached `--serve` instance of this executable.
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
