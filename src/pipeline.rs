use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// Configuration for the pipeline tools.
#[derive(Clone, Debug)]
pub struct PipelineConfig {
    pub ytdlp_path: PathBuf,
    pub ffmpeg_path: PathBuf,
    pub ytdlp_args: Vec<String>,
    /// Directory containing yt-dlp plugins (e.g., PO token provider)
    pub plugin_dirs: Option<PathBuf>,
    /// Extra extractor args (e.g., "youtube:player-client=mweb")
    pub extractor_args: Vec<String>,
}

/// The running pipeline — holds child processes and provides the ffmpeg stdout.
pub struct PipelineHandle {
    pub ffmpeg_stdout: tokio::process::ChildStdout,
    ytdlp_child: Option<Child>,
    ffmpeg_child: Child,
    temp_file: Option<PathBuf>,
}

impl PipelineHandle {
    /// Spawn a background task that waits for processes to finish and cleans up.
    /// Returns the ffmpeg stdout for streaming to the client.
    pub fn into_monitored(mut self, stream_id: String) -> tokio::process::ChildStdout {
        let stdout = self.ffmpeg_stdout;

        tokio::spawn(async move {
            match self.ffmpeg_child.wait().await {
                Ok(status) if status.success() => {
                    tracing::info!(id = %stream_id, "ffmpeg completed successfully");
                }
                Ok(status) => {
                    tracing::warn!(id = %stream_id, status = %status, "ffmpeg exited with error");
                }
                Err(e) => tracing::error!(id = %stream_id, error = %e, "ffmpeg wait failed"),
            }

            if let Some(mut ytdlp) = self.ytdlp_child.take() {
                match ytdlp.wait().await {
                    Ok(status) if !status.success() => {
                        tracing::warn!(id = %stream_id, status = %status, "yt-dlp exited with error");
                    }
                    Err(e) => tracing::error!(id = %stream_id, error = %e, "yt-dlp wait failed"),
                    _ => {}
                }
            }

            if let Some(path) = self.temp_file.take() {
                if let Err(e) = std::fs::remove_file(&path) {
                    tracing::debug!(id = %stream_id, error = %e, path = %path.display(), "failed to remove temp file");
                } else {
                    tracing::debug!(id = %stream_id, path = %path.display(), "cleaned up temp file");
                }
            }
        });

        stdout
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build and start the full media pipeline for a video URL.
///
/// Tries the pipe strategy first (instant streaming), falls back to temp file
/// (reliable but delayed start) if the pipe fails.
pub async fn start_pipeline(
    config: &PipelineConfig,
    video_url: &str,
    stream_id: &str,
) -> Result<PipelineHandle> {
    // Try pipe strategy first — streams data to client immediately
    match spawn_pipe_pipeline(config, video_url, stream_id).await {
        Ok(handle) => {
            tracing::info!(id = %stream_id, "using pipe strategy (instant streaming)");
            return Ok(handle);
        }
        Err(e) => {
            tracing::warn!(id = %stream_id, error = %e, "pipe strategy failed, falling back to temp file");
        }
    }

    // Fallback: download to temp file, then remux (reliable but slow start)
    tracing::info!(id = %stream_id, "using temp file strategy");
    spawn_tempfile_pipeline(config, video_url, stream_id).await
}

// ---------------------------------------------------------------------------
// Strategy 1: Pipe (yt-dlp stdout → ffmpeg → fragmented MP4 stdout)
// ---------------------------------------------------------------------------

async fn spawn_pipe_pipeline(
    config: &PipelineConfig,
    video_url: &str,
    stream_id: &str,
) -> Result<PipelineHandle> {
    let work_dir = config.ytdlp_path.parent().unwrap_or(Path::new("."));
    let tmp_dir = work_dir.join("tmp");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let path_env = augmented_path(work_dir);

    let ytdlp_args = build_pipe_args(config, video_url);
    tracing::debug!(id = %stream_id, args = ?ytdlp_args, "spawning yt-dlp pipe");

    let mut cmd = Command::new(&config.ytdlp_path);
    cmd.args(&ytdlp_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_ytdlp_env(&mut cmd, work_dir, &path_env);

    let mut ytdlp_child = cmd.spawn().context("spawning yt-dlp (pipe mode)")?;

    if let Some(stderr) = ytdlp_child.stderr.take() {
        let id = stream_id.to_string();
        tokio::spawn(log_stderr("yt-dlp", id, stderr));
    }

    let ytdlp_stdout = ytdlp_child
        .stdout
        .take()
        .context("no stdout from yt-dlp")?;

    // Convert tokio ChildStdout → Stdio for ffmpeg stdin
    let stdin: Stdio = ytdlp_stdout
        .try_into()
        .context("converting yt-dlp stdout to ffmpeg stdin")?;

    let mut ffmpeg_child = spawn_ffmpeg_pipe(&config.ffmpeg_path, stdin, stream_id)?;

    let ffmpeg_stdout = ffmpeg_child
        .stdout
        .take()
        .context("no stdout from ffmpeg")?;

    Ok(PipelineHandle {
        ffmpeg_stdout,
        ytdlp_child: Some(ytdlp_child),
        ffmpeg_child,
        temp_file: None,
    })
}

// ---------------------------------------------------------------------------
// Strategy 2: Temp file (yt-dlp → file → ffmpeg → fragmented MP4 stdout)
// ---------------------------------------------------------------------------

async fn spawn_tempfile_pipeline(
    config: &PipelineConfig,
    video_url: &str,
    stream_id: &str,
) -> Result<PipelineHandle> {
    let work_dir = config.ytdlp_path.parent().unwrap_or(Path::new("."));
    let tmp_dir = work_dir.join("tmp");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let path_env = augmented_path(work_dir);

    let temp_file = tmp_dir.join(format!("stream_{stream_id}.mp4"));
    let ytdlp_args = build_file_args(config, video_url, &temp_file);

    let max_retries = 3;
    let mut last_error = String::new();

    for attempt in 1..=max_retries {
        if attempt > 1 {
            let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32 - 1));
            tracing::info!(id = %stream_id, attempt, delay_secs = delay.as_secs(), "retrying download");
            tokio::time::sleep(delay).await;
        }

        tracing::info!(
            id = %stream_id, attempt, output = %temp_file.display(),
            "downloading via yt-dlp to temp file"
        );

        let _ = std::fs::remove_file(&temp_file);

        let mut cmd = Command::new(&config.ytdlp_path);
        cmd.args(&ytdlp_args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        apply_ytdlp_env(&mut cmd, work_dir, &path_env);

        let output = cmd.output().await.context("running yt-dlp download")?;

        let stderr_text = String::from_utf8_lossy(&output.stderr);
        log_ytdlp_stderr(stream_id, &stderr_text);

        if output.status.success() && temp_file.exists() {
            let file_size = std::fs::metadata(&temp_file)
                .map(|m| m.len())
                .unwrap_or(0);

            if file_size > 0 {
                tracing::info!(
                    id = %stream_id, size = file_size, attempt,
                    "download complete, starting ffmpeg remux"
                );
                last_error.clear();
                break;
            }
        }

        last_error = stderr_text
            .lines()
            .filter(|l| l.contains("ERROR"))
            .last()
            .unwrap_or("unknown error")
            .to_string();

        let is_transient = stderr_text.contains("403")
            || stderr_text.contains("timed out")
            || stderr_text.contains("Connection reset");

        if !is_transient {
            tracing::warn!(id = %stream_id, "non-transient error, skipping retries");
            break;
        }

        tracing::warn!(id = %stream_id, attempt, "download failed (transient), will retry");
    }

    if !last_error.is_empty() || !temp_file.exists() {
        let _ = std::fs::remove_file(&temp_file);
        bail!("yt-dlp download failed after {max_retries} attempts: {last_error}");
    }

    let file_size = std::fs::metadata(&temp_file)
        .map(|m| m.len())
        .unwrap_or(0);
    tracing::info!(id = %stream_id, size = file_size, "download complete, starting ffmpeg remux");

    let (needs_transcode_video, needs_transcode_audio) =
        probe_codecs(&config.ffmpeg_path, &temp_file).await;

    let mut ffmpeg_child = spawn_ffmpeg_file(
        &config.ffmpeg_path,
        &temp_file,
        needs_transcode_video,
        needs_transcode_audio,
        stream_id,
    )?;

    let ffmpeg_stdout = ffmpeg_child
        .stdout
        .take()
        .context("no stdout from ffmpeg")?;

    Ok(PipelineHandle {
        ffmpeg_stdout,
        ytdlp_child: None,
        ffmpeg_child,
        temp_file: Some(temp_file),
    })
}

// ---------------------------------------------------------------------------
// ffmpeg spawning
// ---------------------------------------------------------------------------

/// Spawn ffmpeg reading from a pipe — for the streaming strategy.
fn spawn_ffmpeg_pipe(
    ffmpeg_path: &Path,
    stdin: Stdio,
    stream_id: &str,
) -> Result<Child> {
    let mut cmd = Command::new(ffmpeg_path);
    cmd.args(["-hide_banner", "-loglevel", "warning"]);
    cmd.args(["-probesize", "10M", "-analyzeduration", "10M"]);
    cmd.args(["-i", "pipe:0"]);
    cmd.args(["-c", "copy"]);
    cmd.args(["-bsf:a", "aac_adtstoasc"]);
    cmd.args([
        "-movflags",
        "+frag_keyframe+empty_moov+default_base_moof",
    ]);
    cmd.args(["-f", "mp4", "pipe:1"]);
    cmd.stdin(stdin).stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(windows)]
    apply_no_window(&mut cmd);

    let mut child = cmd.spawn().context("spawning ffmpeg (pipe mode)")?;
    if let Some(stderr) = child.stderr.take() {
        let id = stream_id.to_string();
        tokio::spawn(log_stderr("ffmpeg", id, stderr));
    }
    Ok(child)
}

/// Spawn ffmpeg reading from a file — for the temp file strategy.
fn spawn_ffmpeg_file(
    ffmpeg_path: &Path,
    input_file: &Path,
    needs_transcode_video: bool,
    needs_transcode_audio: bool,
    stream_id: &str,
) -> Result<Child> {
    let mut cmd = Command::new(ffmpeg_path);
    cmd.args(["-hide_banner", "-loglevel", "warning"]);
    cmd.args(["-i", &input_file.to_string_lossy()]);

    if needs_transcode_video {
        tracing::info!(id = %stream_id, "transcoding video to h264");
        cmd.args(["-c:v", "libx264", "-preset", "fast", "-crf", "23"]);
    } else {
        cmd.args(["-c:v", "copy"]);
    }

    if needs_transcode_audio {
        tracing::info!(id = %stream_id, "transcoding audio to aac");
        cmd.args(["-c:a", "aac", "-b:a", "192k"]);
    } else {
        cmd.args(["-c:a", "copy"]);
    }

    cmd.args(["-bsf:a", "aac_adtstoasc"]);
    cmd.args([
        "-movflags",
        "+frag_keyframe+empty_moov+default_base_moof",
    ]);
    cmd.args(["-f", "mp4", "pipe:1"]);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    apply_no_window(&mut cmd);

    let mut child = cmd.spawn().context("spawning ffmpeg (file mode)")?;
    if let Some(stderr) = child.stderr.take() {
        let id = stream_id.to_string();
        tokio::spawn(log_stderr("ffmpeg", id, stderr));
    }
    Ok(child)
}

// ---------------------------------------------------------------------------
// yt-dlp argument builders
// ---------------------------------------------------------------------------

/// Build the common yt-dlp arguments shared by both strategies.
fn build_common_args(config: &PipelineConfig) -> Vec<String> {
    let mut args = Vec::new();

    if let Some(ref plugin_dir) = config.plugin_dirs {
        args.push("--plugin-dirs".into());
        args.push(plugin_dir.to_string_lossy().to_string());
    }

    for ea in &config.extractor_args {
        args.push("--extractor-args".into());
        args.push(ea.clone());
    }

    let mut skip_next = false;
    for arg in &config.ytdlp_args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--get-url" {
            continue;
        }
        if arg == "-f" || arg == "--format" {
            skip_next = true;
            continue;
        }
        if arg.starts_with("http://") || arg.starts_with("https://") {
            continue;
        }
        args.push(arg.clone());
    }

    // Tell yt-dlp where ffmpeg is (for internal merging of split formats)
    let ffmpeg_dir = config
        .ffmpeg_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_string_lossy()
        .to_string();
    args.push("--ffmpeg-location".into());
    args.push(ffmpeg_dir);

    args
}

/// Build args for the pipe strategy (yt-dlp stdout → ffmpeg).
/// Favours HLS combined formats, then split formats (yt-dlp merges internally).
fn build_pipe_args(config: &PipelineConfig, video_url: &str) -> Vec<String> {
    let mut args = build_common_args(config);

    // Format selector: HLS combined first (single stream, direct pipe),
    // then split (yt-dlp merges internally via ffmpeg before piping),
    // then any fallback.
    args.push("-f".into());
    args.push(
        "b[height<=1080][protocol^=m3u8][vcodec^=avc][acodec^=mp4a]\
         /b[height<=1080][protocol^=m3u8]\
         /bv[vcodec^=avc][height<=1080]+ba[acodec^=mp4a]\
         /bv[height<=1080]+ba\
         /b[height<=1080]\
         /b"
        .into(),
    );

    // Merge split formats into MP4 container
    args.push("--merge-output-format".into());
    args.push("mp4".into());

    // Output to stdout — yt-dlp sends progress/logs to stderr by default with -o -
    args.push("-o".into());
    args.push("-".into());
    args.push("--newline".into());

    args.push(video_url.into());
    args
}

/// Build args for the temp file strategy (yt-dlp → file → ffmpeg).
fn build_file_args(config: &PipelineConfig, video_url: &str, output_path: &Path) -> Vec<String> {
    let mut args = build_common_args(config);

    // Same format selector as pipe but also allows direct HTTPS as last resort
    args.push("-f".into());
    args.push(
        "b[height<=1080][protocol^=m3u8][vcodec^=avc]\
         /b[height<=1080][protocol^=m3u8]\
         /b[height<=1080][vcodec^=avc][acodec^=mp4a]\
         /bv[vcodec^=avc][height<=1080]+ba[acodec^=mp4a]\
         /bv[height<=1080]+ba\
         /b[height<=1080]\
         /b"
        .into(),
    );

    args.push("--merge-output-format".into());
    args.push("mp4".into());

    args.push("-o".into());
    args.push(output_path.to_string_lossy().to_string());

    args.push(video_url.into());
    args
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_mp4_video_codec(codec: &str) -> bool {
    let c = codec.to_lowercase();
    c.is_empty()
        || c == "none"
        || c == "unknown"
        || c.starts_with("avc")
        || c.starts_with("h264")
        || c.starts_with("h.264")
        || c.starts_with("hevc")
        || c.starts_with("h265")
        || c.starts_with("h.265")
        || c.starts_with("mp4v")
}

fn is_mp4_audio_codec(codec: &str) -> bool {
    let c = codec.to_lowercase();
    c.is_empty()
        || c == "none"
        || c == "unknown"
        || c.starts_with("mp4a")
        || c.starts_with("aac")
        || c == "mp3"
}

async fn probe_codecs(ffmpeg_path: &Path, file: &Path) -> (bool, bool) {
    let ffprobe_path = ffmpeg_path.with_file_name(if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    });

    let mut cmd = Command::new(&ffprobe_path);
    cmd.args([
            "-v",
            "quiet",
            "-show_entries",
            "stream=codec_name,codec_type",
            "-of",
            "csv=p=0",
        ])
        .arg(file)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    #[cfg(windows)]
    apply_no_window(&mut cmd);

    let output = cmd.output().await;

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => {
            tracing::warn!("ffprobe failed, assuming transcoding needed");
            return (true, true);
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut vcodec = String::new();
    let mut acodec = String::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 2 {
            match parts[1] {
                "video" => vcodec = parts[0].to_string(),
                "audio" => acodec = parts[0].to_string(),
                _ => {}
            }
        }
    }

    let needs_v = !is_mp4_video_codec(&vcodec);
    let needs_a = !is_mp4_audio_codec(&acodec);

    if needs_v {
        tracing::info!(vcodec = %vcodec, "video needs transcoding to h264");
    }
    if needs_a {
        tracing::info!(acodec = %acodec, "audio needs transcoding to aac");
    }

    (needs_v, needs_a)
}

fn log_ytdlp_stderr(stream_id: &str, stderr_text: &str) {
    for line in stderr_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.contains("ERROR") || line.contains("error") {
            tracing::error!(id = %stream_id, process = "yt-dlp", "{}", line);
        } else if line.contains("WARNING") || line.contains("warning") {
            tracing::warn!(id = %stream_id, process = "yt-dlp", "{}", line);
        } else {
            tracing::debug!(id = %stream_id, process = "yt-dlp", "{}", line);
        }
    }
}

fn augmented_path(tool_dir: &Path) -> std::ffi::OsString {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut result = tool_dir.as_os_str().to_owned();
    if !current.is_empty() {
        let sep = if cfg!(windows) { ";" } else { ":" };
        result.push(sep);
        result.push(current);
    }
    result
}

/// Apply common environment setup for spawning yt-dlp.
/// Clears PyInstaller variables that can cause crashes when yt-dlp.exe
/// (a PyInstaller bundle) is spawned as a child process.
fn apply_ytdlp_env(cmd: &mut Command, work_dir: &Path, path_env: &std::ffi::OsStr) {
    // Ensure TEMP/TMP point to a valid, writable system temp directory.
    // The detached server process may inherit a broken or empty TEMP.
    // PyInstaller (yt-dlp.exe) needs this to extract its bundled files.
    let sys_temp = std::env::temp_dir();

    cmd.current_dir(work_dir)
        .env("PATH", path_env)
        .env("TEMP", &sys_temp)
        .env("TMP", &sys_temp)
        .env_remove("_MEIPASS2")
        .env_remove("_PYI_ARCHIVE_FILE")
        .env_remove("_PYI_SPLASH_IPC");
}

/// Apply CREATE_NO_WINDOW on Windows for ffmpeg/ffprobe processes.
#[cfg(windows)]
fn apply_no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

async fn log_stderr(
    process_name: &'static str,
    stream_id: String,
    stderr: tokio::process::ChildStderr,
) {
    let reader = BufReader::new(stderr);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        if line.contains("ERROR") || line.contains("error") {
            tracing::error!(id = %stream_id, process = process_name, "{}", line);
        } else if line.contains("WARNING") || line.contains("warning") {
            tracing::warn!(id = %stream_id, process = process_name, "{}", line);
        } else {
            tracing::debug!(id = %stream_id, process = process_name, "{}", line);
        }
    }
}
