# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A media proxy that bridges VRChat with yt-dlp. Two Rust crates in one repo:

- **`src/`** — the backend (`vrc-ytdlp`): a library (`lib.rs`) with a thin binary (`main.rs` → `cli::run`). Runs as a CLI; `--serve` starts an Axum HTTP server that resolves and streams video URLs via a yt-dlp subprocess. Wrapper side: `cli.rs`, `args.rs` (whitelist filtering), `config.rs`, `executor.rs` (spawns yt-dlp), `downloader.rs` (yt-dlp self-update). Server side under `server/`: `mod.rs` (Axum app), `pipeline.rs`, `cache.rs`, `lifecycle.rs` (Windows job objects), `client.rs` (wrapper→server HTTP).
- **`vrc-ytdlp-gui/`** — the desktop app. **The active GUI is the Tauri 2 app** (`src-tauri/` + `index.html` + `js/app.js` + `css/app.css`); Tauri commands live in `src-tauri/src/commands/`. The Iced implementation at `vrc-ytdlp-gui/src/main.rs` is legacy — do not build on it. The Tauri app is slated for rework, so confirm direction before large GUI changes.

## Build & run

```bash
# Backend binary (from repo root)
cargo build                      # debug
cargo build --release            # release (CI uses --profile release)
cargo run -- --serve             # run the HTTP server locally; health at GET /health

# GUI (Tauri)
cd vrc-ytdlp-gui && cargo build  # debug build of the GUI crate
```

Unit tests live in `#[cfg(test)]` modules next to the code (`args.rs`, `cli.rs`, `server/cache.rs`) — run them with `cargo test`.

## Before finishing a Rust change

Run both — CI does **not** enforce either, so it's on us:

```bash
cargo fmt
cargo clippy
```

## Conventions

- **Commits:** Conventional Commits with a scope, e.g. `feat(gui): ...`, `fix(server): ...`, `chore(release): ...`.
- **Pushing:** Commit when asked; **never push or merge without confirming first.** Current dev branch is `test`; `main` is primary.

## Gotchas

- **Windows-first.** System tray, `server/lifecycle.rs` process management (job objects), and GUI path resolution (resolves to the VRChat Tools dir) are Windows-specific and guarded by `cfg(windows)`. Non-Windows builds emit unused-import warnings — expected.
- **`config.json`** (repo root, or the VRChat Tools dir at runtime) drives backend behavior: yt-dlp/ffmpeg paths, `allowed_args` whitelist, `custom_args`, cookie extraction, `execution_timeout_secs`, plugin dirs. Read it before changing executor/pipeline logic — see @config.json.
- **Releases** are a manual GitHub Actions dispatch (`.github/workflows/rust_release.yml`): it bumps the patch version, tags `vX.Y.Z`, and packages a Windows ZIP. Don't hand-edit versions.
