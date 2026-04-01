use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::Result;

// ---------------------------------------------------------------------------
// LogLevel
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel
{
    Info,
    Warn,
    Error,
    Debug,
    Other,
}

impl LogLevel
{
    pub fn from_line(line: &str) -> Self
    {
        if line.contains(" ERROR ") {
            LogLevel::Error
        } else if line.contains(" WARN ") {
            LogLevel::Warn
        } else if line.contains(" INFO ") {
            LogLevel::Info
        } else if line.contains(" DEBUG ") {
            LogLevel::Debug
        } else {
            LogLevel::Other
        }
    }
}

// ---------------------------------------------------------------------------
// LogLine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LogLine
{
    pub text: String,
    pub level: LogLevel,
}

impl LogLine
{
    fn from_str(line: &str) -> Self
    {
        Self {
            level: LogLevel::from_line(line),
            text: line.to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// find_latest_log
// ---------------------------------------------------------------------------

/// Return the newest `vrc-ytdlp.log*` file in `app_dir`, sorted
/// alphabetically (the rolling-log suffix `YYYY-MM-DD` sorts correctly that
/// way).  Returns `None` if no matching file exists.
pub fn find_latest_log(app_dir: &Path) -> Option<PathBuf>
{
    let mut matches: Vec<PathBuf> = std::fs::read_dir(app_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("vrc-ytdlp.log"))
                    .unwrap_or(false)
        })
        .collect();

    matches.sort();
    matches.into_iter().last()
}

// ---------------------------------------------------------------------------
// read_log_file
// ---------------------------------------------------------------------------

/// Read an entire log file and return all lines parsed into `LogLine`s.
pub fn read_log_file(path: &Path) -> Result<Vec<LogLine>>
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let lines = reader
        .lines()
        .filter_map(|l| l.ok())
        .map(|l| LogLine::from_str(&l))
        .collect();
    Ok(lines)
}

// ---------------------------------------------------------------------------
// LogTailer
// ---------------------------------------------------------------------------

pub struct LogTailer
{
    path: PathBuf,
    position: u64,
}

impl LogTailer
{
    /// Create a tailer starting at the **end** of the file (only future lines
    /// will be returned).
    pub fn new(path: PathBuf) -> Self
    {
        let position = File::open(&path)
            .and_then(|mut f| f.seek(SeekFrom::End(0)))
            .unwrap_or(0);
        Self { path, position }
    }

    /// Create a tailer starting at the **beginning** of the file.
    pub fn from_start(path: PathBuf) -> Self
    {
        Self { path, position: 0 }
    }

    /// Seek to the last known position, read any new complete lines, and
    /// advance the stored position.  Handles file truncation by resetting to
    /// the beginning when the file is shorter than our last position.
    pub fn read_new_lines(&mut self) -> Vec<LogLine>
    {
        let mut file = match File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        // Detect truncation.
        let file_len = file.seek(SeekFrom::End(0)).unwrap_or(0);
        if file_len < self.position {
            self.position = 0;
        }

        if file.seek(SeekFrom::Start(self.position)).is_err() {
            return Vec::new();
        }

        let mut buf = String::new();
        if file.read_to_string(&mut buf).is_err() {
            return Vec::new();
        }

        // Only consume complete lines (those ending with '\n').
        // If the last write is partial we'll pick it up on the next poll.
        let ends_with_newline = buf.ends_with('\n');
        let mut lines: Vec<&str> = buf.lines().collect();
        if !ends_with_newline && !lines.is_empty() {
            lines.pop(); // incomplete last line — leave it for next call
        }

        let parsed: Vec<LogLine> = lines.iter().map(|l| LogLine::from_str(l)).collect();

        // Advance position by the bytes we actually consumed.
        let consumed: usize = if ends_with_newline || buf.is_empty() {
            buf.len()
        } else {
            // Drop the partial line we popped above.
            buf.rfind('\n').map(|i| i + 1).unwrap_or(0)
        };
        self.position += consumed as u64;

        parsed
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests
{
    use super::*;
    use std::io::Write;

    #[test]
    fn log_level_detection()
    {
        assert_eq!(LogLevel::from_line("2026-04-01 INFO starting"), LogLevel::Info);
        assert_eq!(LogLevel::from_line("2026-04-01 WARN timeout"), LogLevel::Warn);
        assert_eq!(LogLevel::from_line("2026-04-01 ERROR crash"), LogLevel::Error);
        assert_eq!(LogLevel::from_line("something else"), LogLevel::Other);
    }

    #[test]
    fn tailer_reads_incrementally()
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let log_path = dir.path().join("test.log");

        // Write two initial lines.
        {
            let mut f = File::create(&log_path).expect("create");
            writeln!(f, "2026-04-01 INFO first line").unwrap();
            writeln!(f, "2026-04-01 INFO second line").unwrap();
        }

        let mut tailer = LogTailer::from_start(log_path.clone());

        // First read should return both lines.
        let lines = tailer.read_new_lines();
        assert_eq!(lines.len(), 2, "expected 2 lines on first read");
        assert_eq!(lines[0].level, LogLevel::Info);
        assert_eq!(lines[1].level, LogLevel::Info);

        // Second read with no new content should return nothing.
        let lines = tailer.read_new_lines();
        assert_eq!(lines.len(), 0, "expected 0 lines on second read");

        // Append a new error line.
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&log_path)
                .expect("open for append");
            writeln!(f, "2026-04-01 ERROR something crashed").unwrap();
        }

        // Third read should return only the new line.
        let lines = tailer.read_new_lines();
        assert_eq!(lines.len(), 1, "expected 1 new line after append");
        assert_eq!(lines[0].level, LogLevel::Error);
    }
}
