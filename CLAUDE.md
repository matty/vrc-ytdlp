# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A media proxy that bridges VRChat with yt-dlp. Two Rust crates in one repo:

- **`crates/vrc-ytdlp/`** — the backend (`vrc-ytdlp`): a library (`lib.rs`) with a thin binary (`main.rs` → `cli::run`). Runs as a CLI; `--serve` starts an Axum HTTP server that resolves and streams video URLs via a yt-dlp subprocess. Wrapper side: `cli.rs`, `args.rs` (whitelist filtering), `config.rs`, `executor.rs` (spawns yt-dlp), `downloader.rs` (yt-dlp self-update). Server side under `server/`: `mod.rs` (Axum app), `pipeline.rs`, `cache.rs`, `lifecycle.rs` (Windows job objects), `client.rs` (wrapper→server HTTP).
- **`apps/desktop/`** — the desktop manager (Tauri 2, in progress): static UI in `ui/` (vanilla HTML/CSS/JS, no bundler), Rust side in `src-tauri/` depending on the core crate as a library. Design spec: `docs/superpowers/specs/2026-06-10-desktop-manager-design.md`.
- **`vrc-ytdlp-gui/`** — legacy (Iced app + first Tauri attempt), excluded from the workspace; do not build on it. Will be deleted once the manager reaches parity.

## Build & run

```bash
# Workspace (from repo root) — builds backend + manager
cargo build
cargo test

# Backend only
cargo build --release -p vrc-ytdlp
cargo run -p vrc-ytdlp -- --serve     # HTTP server; health at GET /health

# Desktop manager
cargo run -p vrc-ytdlp-manager
```

Unit tests live in `#[cfg(test)]` modules next to the code (`crates/vrc-ytdlp/src/args.rs`, `crates/vrc-ytdlp/src/cli.rs`, `crates/vrc-ytdlp/src/server/cache.rs`, `apps/desktop/src-tauri/src/commands/error.rs`) — run them with `cargo test`.

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
- **Releases** are a manual GitHub Actions dispatch (`.github/workflows/rust_release.yml`): it bumps the core crate version via `cargo set-version -p vrc-ytdlp` (reads `crates/vrc-ytdlp/Cargo.toml`), tags `vX.Y.Z`, and packages a Windows ZIP. Don't hand-edit versions.
