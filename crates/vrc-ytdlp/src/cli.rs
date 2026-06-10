//! Top-level run modes: the `--serve` media server and the yt-dlp wrapper
//! that VRChat invokes.

use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::args::{filter_args, is_http_url};
use crate::config::{self, Config};
use crate::downloader::ensure_ytdlp;
use crate::executor::run_ytdlp;
use crate::logging::setup_logging;
use crate::paths::{exe_dir, resolve_ffmpeg_path, resolve_ytdlp_path};
use crate::server::{client, lifecycle, ServerConfig};

/// Entry point: dispatch to `--serve` mode or the wrapper flow.
pub async fn run() -> Result<()> {
    let raw_args: Vec<String> = env::args().skip(1).collect();

    if raw_args.iter().any(|a| a == "--serve") {
        run_serve(&raw_args).await
    } else {
        run_wrapper(&raw_args).await
    }
}

/// Run as the background media server.
async fn run_serve(raw_args: &[String]) -> Result<()> {
    let port = parse_flag_value(raw_args, "--port")
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(config::default_server_port);
    let idle_timeout = parse_flag_value(raw_args, "--idle-timeout")
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(config::default_server_idle_timeout_secs);

    let app_dir = exe_dir()?;
    let _guard = setup_logging(&app_dir);
    let config = config::load_config(&app_dir.join("config.json"))?;
    let ytdlp_path = resolve_ytdlp_path(&config, &app_dir)?;
    let ffmpeg_path = resolve_ffmpeg_path(&config, &app_dir)?;

    tracing::info!(
        ytdlp = %ytdlp_path.display(),
        ffmpeg = %ffmpeg_path.display(),
        "server tool paths resolved"
    );

    let plugin_dirs = config.plugin_dirs.as_ref().map(|p| {
        if Path::new(p).is_absolute() {
            PathBuf::from(p)
        } else {
            app_dir.join(p)
        }
    });

    // Auto-detect bgutil-pot binary next to the executable
    let bgutil_pot_path = {
        let name = if cfg!(windows) {
            "bgutil-pot.exe"
        } else {
            "bgutil-pot"
        };
        let candidate = app_dir.join(name);
        candidate.exists().then_some(candidate)
    };

    let cache_dir = if Path::new(&config.cache_dir).is_absolute() {
        PathBuf::from(&config.cache_dir)
    } else {
        app_dir.join(&config.cache_dir)
    };

    let server_config = ServerConfig {
        ytdlp_path,
        ffmpeg_path,
        plugin_dirs,
        extractor_args: config.extractor_args.clone(),
        cache_dir,
        cache_max_size_mb: config.cache_max_size_mb,
        cache_ttl_secs: config.cache_ttl_secs,
    };

    lifecycle::run_managed_server(
        port,
        idle_timeout,
        server_config,
        bgutil_pot_path,
        config.bgutil_pot_port,
    )
    .await
}

/// Run as the yt-dlp wrapper: sanitize args, then either route `--get-url`
/// through the media server or execute yt-dlp directly.
async fn run_wrapper(raw_args: &[String]) -> Result<()> {
    let app_dir = exe_dir()?;
    let _guard = setup_logging(&app_dir);
    let config = config::load_config(&app_dir.join("config.json"))?;
    let ytdlp_path = resolve_ytdlp_path(&config, &app_dir)?;
    ensure_ytdlp(&ytdlp_path, &config).await?;
    let args = filter_args(raw_args, &config);

    // Check if this is a --get-url request — route through media server
    let is_get_url = args.iter().any(|a| a == "--get-url");

    if is_get_url {
        // Extract the video URL from filtered args (positional arg)
        let video_url = args
            .iter()
            .find(|a| is_http_url(a))
            .context("no video URL found in args")?
            .clone();

        tracing::info!(video_url = %video_url, "routing --get-url through media server");

        let url =
            ensure_server_and_stream(&config, &video_url, &ytdlp_path.to_string_lossy(), &args)
                .await?;
        println!("{url}");
    } else {
        tracing::info!(args = ?args, "executing yt-dlp");
        run_ytdlp(
            &ytdlp_path,
            &args,
            Duration::from_secs(config.execution_timeout_secs),
        )?;
    }

    Ok(())
}

/// Make sure the media server is running, register the stream, and return
/// the local URL to hand back to VRChat.
async fn ensure_server_and_stream(
    config: &Config,
    video_url: &str,
    ytdlp_path: &str,
    ytdlp_args: &[String],
) -> Result<String> {
    let port = config.server_port;
    let idle_timeout = config.server_idle_timeout_secs;

    // Check if server is already running
    if !client::check_server_health(port).await {
        tracing::info!("media server not running, starting...");
        client::spawn_server_process(port, idle_timeout)?;

        // Wait for server to come up
        let mut attempts = 0;
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if client::check_server_health(port).await {
                tracing::info!("media server is ready");
                break;
            }
            attempts += 1;
            if attempts > 50 {
                bail!("media server failed to start within 5 seconds");
            }
        }
    }

    let stream_id =
        client::register_stream_with_server(port, video_url, ytdlp_path, ytdlp_args).await?;
    let url = client::stream_url(port, &stream_id);
    tracing::info!(url = %url, "stream registered");
    Ok(url)
}

fn parse_flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().map(|s| s.as_str());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flag_value_finds_value() {
        let args: Vec<String> = ["--serve", "--port", "9000"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(parse_flag_value(&args, "--port"), Some("9000"));
        assert_eq!(parse_flag_value(&args, "--missing"), None);
    }
}
