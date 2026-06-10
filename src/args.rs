//! Sanitizes untrusted yt-dlp arguments through the `allowed_args` whitelist
//! and appends configured/auto-detected extras.

use crate::config::Config;
use crate::paths::{exe_dir, which};

/// Filter raw caller args through the whitelist, then append `custom_args`,
/// cookie flags, and the detected JS runtime.
pub fn filter_args(input: &[String], config: &Config) -> Vec<String> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < input.len() {
        let arg = &input[i];

        // Pass through positional arguments (URLs, etc.)
        if !arg.starts_with('-') {
            result.push(arg.clone());
            i += 1;
            continue;
        }

        if let Some(entry) = find_allowed(arg, &config.allowed_args) {
            if entry.ends_with('=') {
                // Value-taking arg
                if arg.contains('=') {
                    // Inline form: --flag=value
                    result.push(arg.clone());
                    i += 1;
                } else if i + 1 < input.len() {
                    // Two-arg form: --flag value
                    result.push(arg.clone());
                    result.push(input[i + 1].clone());
                    i += 2;
                } else {
                    // Flag with no value following — drop it
                    tracing::debug!(arg, "dropping value-taking arg with no value");
                    i += 1;
                }
            } else {
                // Standalone flag
                result.push(arg.clone());
                i += 1;
            }
        } else {
            // When dropping unknown flags, also skip the next arg if it looks
            // like a value — but never swallow a URL: that's the video
            // positional arg, not a flag value.
            if !arg.contains('=')
                && i + 1 < input.len()
                && !input[i + 1].starts_with('-')
                && !is_http_url(&input[i + 1])
            {
                tracing::debug!(
                    arg,
                    value = &input[i + 1],
                    "dropping disallowed arg with value"
                );
                i += 2;
            } else {
                tracing::debug!(arg, "dropping disallowed arg");
                i += 1;
            }
        }
    }

    result.extend(config.custom_args.iter().cloned());

    if config.cookies {
        // Check for a cookies.txt file first (works on headless servers)
        let app_dir = exe_dir().unwrap_or_default();
        let cookie_file = app_dir.join("cookies.txt");
        if cookie_file.exists() {
            result.push("--cookies".into());
            result.push(cookie_file.to_string_lossy().to_string());
        } else {
            result.push(format!("--cookies-from-browser={}", config.cookies_browser));
        }
    }

    if let Some(js_runtime) = detect_js_runtime() {
        result.push("--js-runtimes".into());
        result.push(js_runtime);
    }

    result
}

pub fn is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn find_allowed<'a>(arg: &str, allowed: &'a [String]) -> Option<&'a str> {
    for entry in allowed {
        if entry.ends_with('=') {
            let prefix = &entry[..entry.len() - 1];
            if arg == prefix || arg.starts_with(entry.as_str()) {
                return Some(entry);
            }
        } else if arg == entry.as_str() {
            return Some(entry);
        }
    }
    None
}

fn detect_js_runtime() -> Option<String> {
    // yt-dlp preference order: deno, node, bun, quickjs
    const CANDIDATES: &[(&str, &[&str])] = &[
        ("deno", &["deno", "deno.exe"]),
        ("node", &["node", "node.exe"]),
        ("bun", &["bun", "bun.exe"]),
        ("quickjs", &["qjs", "qjs.exe"]),
    ];

    for (name, binaries) in CANDIDATES {
        for bin in *binaries {
            if let Some(path) = which(bin) {
                let runtime = format!("{}:{}", name, path.display());
                tracing::info!(runtime = %runtime, "detected JS runtime for yt-dlp");
                return Some(runtime);
            }
        }
    }

    tracing::warn!("no JS runtime found — yt-dlp may fail to solve YouTube challenges");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(allowed: &[&str]) -> Config {
        Config {
            allowed_args: allowed.iter().map(|s| s.to_string()).collect(),
            custom_args: Vec::new(),
            cookies: false,
            ..Config::default()
        }
    }

    fn to_args(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn filter_args_keeps_url_after_dropped_unknown_flag() {
        let config = test_config(&["--get-url"]);
        let result = filter_args(
            &to_args(&["--no-playlist", "https://example.com/watch?v=1"]),
            &config,
        );
        assert!(
            result.contains(&"https://example.com/watch?v=1".to_string()),
            "URL must survive filtering of an unknown flag, got: {result:?}"
        );
    }

    #[test]
    fn filter_args_passes_allowed_flag_and_url() {
        let config = test_config(&["--get-url"]);
        let result = filter_args(&to_args(&["--get-url", "https://example.com/v"]), &config);
        assert!(result.contains(&"--get-url".to_string()));
        assert!(result.contains(&"https://example.com/v".to_string()));
    }

    #[test]
    fn filter_args_drops_disallowed_flag_with_value() {
        let config = test_config(&["--get-url"]);
        let result = filter_args(&to_args(&["--output", "file.mp4"]), &config);
        assert!(!result.contains(&"--output".to_string()));
        assert!(!result.contains(&"file.mp4".to_string()));
    }

    #[test]
    fn filter_args_accepts_value_taking_arg_in_both_forms() {
        let config = test_config(&["--format="]);
        let inline = filter_args(&to_args(&["--format=best"]), &config);
        assert!(inline.contains(&"--format=best".to_string()));

        let two_arg = filter_args(&to_args(&["--format", "best"]), &config);
        assert!(two_arg.contains(&"--format".to_string()));
        assert!(two_arg.contains(&"best".to_string()));
    }
}
