---
name: run-server
description: Build and launch the vrc-ytdlp backend in --serve mode locally and confirm it's healthy, so you can test HTTP endpoints. Use when asked to run the server, test an endpoint, or verify a backend change.
---

# Run the backend server

This launches the `vrc-ytdlp` backend binary (`crates/vrc-ytdlp/`) in HTTP server mode.

## Steps

1. From the repo root, build first so errors surface clearly:
   ```bash
   cargo build -p vrc-ytdlp
   ```
2. Confirm `config.json` exists at the repo root (the server reads it for yt-dlp/ffmpeg paths, `allowed_args`, timeouts). If a `--serve` flag for port/timeout is needed, check `crates/vrc-ytdlp/src/main.rs` for the current flags — don't assume.
3. Launch the server in the background so the shell stays free:
   ```bash
   cargo run -p vrc-ytdlp -- --serve
   ```
4. Verify it's up by hitting the health endpoint:
   ```bash
   curl http://localhost:<port>/health
   ```
   The default port comes from `crates/vrc-ytdlp/src/server/mod.rs` — read it rather than guessing.
5. Report the resolved port, health response, and any startup log lines. To test a specific endpoint, inspect the route definitions in `crates/vrc-ytdlp/src/server/mod.rs` for the exact path and expected payload.

## Notes

- yt-dlp and ffmpeg paths come from `config.json`; if the binaries are missing the server starts but URL resolution fails — say so explicitly rather than treating it as a crash.
- Stop the background server when done.
