use std::fs;
use std::path::PathBuf;

use serde::Serialize;

use crate::paths;

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub text: String,
    pub level: String,
}

#[tauri::command]
pub fn read_logs(max_lines: Option<usize>) -> Result<Vec<LogLine>, String> {
    let app_dir = paths::app_dir().map_err(|e| e.to_string())?;
    let log_path = match find_latest_log(&app_dir) {
        Some(p) => p,
        None => return Ok(vec![]),
    };

    let content = fs::read_to_string(&log_path).map_err(|e| e.to_string())?;
    let max = max_lines.unwrap_or(1000);

    let lines: Vec<LogLine> = content
        .lines()
        .rev()
        .take(max)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|line| LogLine {
            level: detect_level(line).to_string(),
            text: line.to_string(),
        })
        .collect();

    Ok(lines)
}

fn find_latest_log(app_dir: &PathBuf) -> Option<PathBuf> {
    let entries = fs::read_dir(app_dir).ok()?;
    let mut logs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("vrc-ytdlp.log"))
                .unwrap_or(false)
        })
        .collect();
    logs.sort();
    logs.last().cloned()
}

fn detect_level(line: &str) -> &str {
    if line.contains(" ERROR ") { "error" }
    else if line.contains(" WARN ") { "warn" }
    else if line.contains(" DEBUG ") { "debug" }
    else if line.contains(" INFO ") { "info" }
    else { "other" }
}
