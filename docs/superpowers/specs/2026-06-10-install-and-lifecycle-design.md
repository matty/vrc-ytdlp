# vrc-ytdlp Manager — Install & Lifecycle Design

**Date:** 2026-06-10
**Status:** Draft — awaiting user approval
**Builds on:** `docs/superpowers/specs/2026-06-10-desktop-manager-design.md` (the overall manager design). This spec settles the install/setup and server-lifecycle mechanism that the services-phase implementation plan depends on. Theme/UI direction is Mica Dark (mockup-4), unchanged.

## 1. Goal

Define exactly how the desktop manager installs the vrc-ytdlp backend (as VRChat's `yt-dlp.exe` stand-in) plus the real yt-dlp/ffmpeg/plugins, where those files live, how the manager detects and (only) observes the server VRChat auto-spawns, and how config edits and updates interact with a possibly-running server. This is the foundation the per-domain services (config, components/install, server, cache, cookies, logs) build on.

## 2. Architecture & ownership

**The wrapper owns the server lifecycle. The manager is an observer, configurator, and installer — never the lifecycle owner.**

| Actor | Role |
|---|---|
| **VRChat** | Invokes `Tools\yt-dlp.exe --get-url <url>` when a video plays. Unaware anything was swapped. |
| **The wrapper** (`yt-dlp.exe`, our backend) | On a `--get-url` call, cold-starts its own `--serve` process if `/health` fails, registers the stream, returns a local URL. The server idle-times-out on its own. The only thing that starts/stops the server in normal use. |
| **The manager** (`vrc-ytdlp-manager` desktop app) | Installs/updates components, reads & writes `config.json`, tails logs, manages the cache — filesystem operations that work whether or not a server is running. Observes server state via `/health`; can manually start/stop for testing-without-VRChat, but does not drive the lifecycle day-to-day. |

The manager interacts with the backend through three channels, each independent of server state:

- **Filesystem** — `config.json`, log files, cache dir, installed binaries. Always available.
- **HTTP `/health` poll** (5 s) on `config.server_port` — the single source of truth for "is a server running right now."
- **Process control** — spawn `yt-dlp.exe --serve` to start; stop via the `server.pid` the server writes on startup.

The manager's Rust side reuses the core crate directly (`vrc_ytdlp::config::Config`, `vrc_ytdlp::paths`, cache types) — the reason the repo is a workspace, so config shape cannot drift between backend and manager.

## 3. Install layout

Decision: **subfolder under Tools**, **overwrite VRChat's `yt-dlp.exe` with no backup**.

```
%USERPROFILE%\AppData\LocalLow\VRChat\VRChat\Tools\
├─ yt-dlp.exe          ← our wrapper (overwrites VRChat's; from OUR GitHub releases)
├─ config.json         ← wrapper reads this from its own dir (exe_dir)
└─ vrc-ytdlp\
   ├─ yt-dlp.exe       ← real yt-dlp (extraction backend)
   ├─ ffmpeg.exe, ffprobe.exe
   ├─ yt-dlp-plugins\  ← bgutil-pot PO-token provider
   ├─ cache\
   ├─ logs\
   ├─ server.pid       ← written by the server on startup
   ├─ version.txt      ← real yt-dlp version (existing, written by ensure_ytdlp)
   └─ .install.json    ← manager's install marker (backend semver, ISO install date)
```

Why this layout: the wrapper reads `config.json` from `exe_dir()` (= Tools root), so `config.json` and `yt-dlp.exe` must sit directly in Tools; everything else is namespaced in the subfolder via config-relative paths. Tidy, trivial to uninstall, and `config.json` stays where the wrapper looks.

`config.json` tool paths become subfolder-relative:

```jsonc
{
  "ytdlp_location": "./vrc-ytdlp/yt-dlp.exe",
  "ffmpeg_location": "./vrc-ytdlp/ffmpeg.exe",
  "plugin_dirs": "./vrc-ytdlp/yt-dlp-plugins",
  "cache_dir": "./vrc-ytdlp/cache"
  // ... other fields unchanged
}
```

The self-reference guard in `resolve_ytdlp_path` already prevents `ytdlp_location` from accidentally pointing back at the wrapper.

### Install flow (idempotent — safe to re-run)

1. Resolve/confirm the Tools dir (auto-detect the LocalLow path; overridable).
2. Create `Tools\vrc-ytdlp\`.
3. Download our backend release asset → write `Tools\yt-dlp.exe` (temp + atomic rename, the existing `download_release` pattern).
4. Write/merge `config.json` with the subfolder-relative paths (atomic; re-validated through `Config` deserialization before the rename).
5. Write `.install.json` (backend semver, ISO date).
6. Install the remaining components (real yt-dlp, ffmpeg, plugins) — see §5.

### Uninstall

Delete `Tools\yt-dlp.exe` and `Tools\config.json`; remove `Tools\vrc-ytdlp\`. No backup to restore — VRChat regenerates its own `yt-dlp.exe` on next launch/update.

## 4. Install detection (clobber-aware)

VRChat re-creates its own `yt-dlp.exe` on game updates, silently overwriting ours. A marker file alone cannot detect that, so detection is **active**:

- The backend gains a dedicated identifying flag **`--vrc-ytdlp-version`**, short-circuited in `cli::run` *before* the wrapper flow (the same way `--serve` is), printing `vrc-ytdlp <CARGO_PKG_VERSION>`. A plain `--version` would fall through the wrapper and run the *real* yt-dlp, reporting the wrong thing — hence a dedicated flag.
- The manager runs `Tools\yt-dlp.exe --vrc-ytdlp-version` and branches on the result:

| Probe result | Verdict | Dashboard action |
|---|---|---|
| prints `vrc-ytdlp X.Y.Z` | **ours, vX.Y.Z** (dashboard backend-version source) | normal |
| prints a yt-dlp date / errors / unrecognized | **clobbered by VRChat** | "Backend overwritten — reinstall" (`backend-clobbered`) |
| file missing | **not installed** | offer setup wizard (`tools-dir-missing` / not-installed) |

## 5. Components: source, installer, updater

| Component | Source | Installs | Updates | Version tracking |
|---|---|---|---|---|
| **Backend wrapper** (`Tools\yt-dlp.exe`) | Our GitHub releases (`matty/vrc-ytdlp` Windows ZIP) | Manager | Manager (no self-update) | `--vrc-ytdlp-version` + `.install.json` |
| **Real yt-dlp** (`vrc-ytdlp\yt-dlp.exe`) | yt-dlp GitHub releases (`yt-dlp_x86.exe`) | Manager during setup, **or** the wrapper on first run | Wrapper auto-updates (`ensure_ytdlp`, throttled by `update_check_days`); manager can force | `vrc-ytdlp\version.txt` (existing) |
| **ffmpeg + ffprobe** | gyan.dev release-essentials zip | Manager | Manager (manual check) | bundle date recorded in `.install.json` |
| **Plugins** (bgutil-pot) | `Brainicism/bgutil-ytdlp-pot-provider` releases | Manager | Manager (manual) | presence check (count of `*.py`) |

Notes:

- **Real yt-dlp has two installers, intentionally.** The wrapper self-downloads it on first `--get-url` if absent (`ensure_ytdlp`) and keeps it current on a timer; the manager pre-installs it during setup so the *first* video in VRChat isn't blocked on a cold download. Both converge on the same file + `version.txt`; whoever runs first wins, the other sees "already current."
- **ffmpeg is the only component nothing auto-fetches.** The wrapper assumes ffmpeg exists (falls back to PATH, else errors). The manager must own ffmpeg install — the genuinely new capability versus the legacy GUI, which stubbed `ffmpeg_exists: true` and never downloaded it. The download is large: it reports progress via Tauri `emit` events and writes to `*.tmp` then atomic-renames.
- The **setup wizard** chains these: detect Tools dir → install backend → install yt-dlp + ffmpeg → plugins → cookies opt-in → write config → done. Each step is skippable and shows progress.

## 6. Lifecycle, detection & update-in-place safety

**Detecting server state:** `/health` poll on `config.server_port` every 5 s is the single source of truth. Up = running; down = not running (the normal idle state, not an error). Dashboard server card and status bar both read this one signal. Uptime / active-stream counts come from the health (or a small status) endpoint response, so the manager never guesses.

**Manual start/stop** (testing without VRChat; the Server screen controls):

- **Start** → spawn `Tools\yt-dlp.exe --serve` detached (the same contract the wrapper uses), then poll `/health` until up or timeout, surfacing captured stderr if the spawn fails (`server-spawn-failed`).
- **Stop** → read `vrc-ytdlp\server.pid`, terminate that PID, remove the file. Requires the backend change in §7 (the server writes its own PID). Fallback if the PID file is missing/stale: match the process listening on the port.

**Config save** (settled decision): write `config.json` atomically (re-validated through `Config`), then — if `/health` shows a server running — **stop it via `server.pid` and do not respawn**. The next VRChat `--get-url` cold-starts a fresh server that reads the new config. The save action warns that an in-progress stream will be interrupted.

**Update-in-place safety:** `Tools\yt-dlp.exe` (the wrapper) is the same binary as the running `--serve` server, and Windows will not overwrite a running executable. A **backend update** therefore sequences: poll `/health` → if running, stop via `server.pid` → download new wrapper to `.tmp` → atomic-rename over `Tools\yt-dlp.exe` → done (next request cold-starts the new version). The manager process is a different exe (`vrc-ytdlp-manager`), so it can safely perform the replace once the server is stopped. Updating the real yt-dlp or ffmpeg is lower-risk (held only briefly during an active extraction); the existing temp+rename handles it, and when the manager initiates it we first check `/health` is idle to avoid replacing mid-stream.

## 7. Required backend (core crate) changes

Small, additive, behavior-preserving — shipped as part of the services phase, since they make the subfolder layout real:

1. **Logs/cache/pid into the subfolder.** `run_serve`/`run_wrapper` currently call `setup_logging(&app_dir)` (writes to Tools root). Derive a `data_dir` for runtime files and send logs → `data_dir/logs`; confirm `cache_dir` resolves there (it already does via `app_dir.join(cache_dir)` once config points at `./vrc-ytdlp/cache`). Keep `config.json` resolution in `exe_dir()` (Tools root) — unchanged.
2. **Server writes `server.pid` on startup** (in `run_serve`/lifecycle), in the subfolder, so the manager can stop a server it did not spawn. Remove it on clean shutdown.
3. **`--vrc-ytdlp-version` flag** short-circuited in `cli::run` before the wrapper flow, printing `vrc-ytdlp <CARGO_PKG_VERSION>` — the install-detection and dashboard version source.

## 8. Manager command surface

Services do the work; commands are thin wrappers returning the `CmdError { code, message }` type already built. Grouped per the overall design §4, with install/lifecycle specifics nailed down:

- `setup::{detect_tools_dir, install_backend, wizard_state}`
- `components::{status, install(component), check_updates}` — component ∈ {backend, ytdlp, ffmpeg, plugins}; `status` runs the `--vrc-ytdlp-version` probe and returns the ours/clobbered/missing verdict
- `server::{status, start, stop}` — `status` = the `/health` poll; `stop` = pid-file kill
- `config::{get, save, exists, defaults}` — `save` does the write-then-kill-server sequence
- `cache::{scan, delete_entry, clear}`
- `cookies::{status, extract, browsers}`
- `logs::{read, log_dir}`

## 9. Error handling

Services return `anyhow::Result`; commands map to `CmdError`. Domain codes the UI branches on:

- `tools-dir-missing` → offer wizard
- `backend-clobbered` → offer reinstall
- `network` (release check/download failed) → retry button
- `server-spawn-failed` → show captured stderr
- everything else → `internal`

Long operations (downloads, cookie extraction) stream progress via Tauri `emit` events so the UI shows live progress bars. config.json and binary writes are atomic (temp + rename); config is re-validated through `Config` deserialization before the rename.

## 10. Testing

- **Services:** unit tests beside the code — release-JSON parsing (fixture), install-path resolution, the `--vrc-ytdlp-version` parse/verdict logic, config round-trip through the shared `Config`, pid-file lifecycle.
- **Backend changes:** the `--vrc-ytdlp-version` flag and pid-file write/remove get tests in the core crate next to the existing `#[cfg(test)]` modules.
- **Commands/UI:** too thin to test — logic lives in services by design.
- **Manual gate:** clean-Tools-dir wizard run, server start/stop, config-save-kills-server, then `cargo fmt` + `cargo clippy --workspace` (CLAUDE.md rule).

## 11. Out of scope

- HTTP Range/206 support for cached file serving (a separate, known gap inherited from old `main`; tracked elsewhere).
- Backend internals (pipeline, cache eviction algorithm, extraction logic) — the manager only consumes them.
- Auto-update of the manager itself (Tauri updater).
- Non-Windows packaging. Manager services should compile on non-Windows (per the overall design) but the Tools-dir resolution and process control are Windows-first.
