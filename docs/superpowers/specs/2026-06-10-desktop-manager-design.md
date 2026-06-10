# vrc-ytdlp Desktop Manager — Design

**Date:** 2026-06-10
**Status:** Draft — awaiting user approval
**Replaces:** `vrc-ytdlp-gui/` (legacy Iced app + first-pass Tauri app), which is ignored and eventually deleted.

## 1. Goal

Reorganize the repo into a clean two-app structure — the **core backend** (`vrc-ytdlp`, the yt-dlp proxy VRChat invokes) and a new **desktop manager** (Tauri 2) built from scratch — where the manager handles everything a user needs: installing the backend into the VRChat Tools dir, keeping yt-dlp/ffmpeg/backend up to date, managing the cache, editing config, extracting cookies, controlling the media server, and viewing logs.

### Assumptions (user delegated decisions)

- Windows-first, matching the backend. Non-Windows builds may warn but should compile.
- No Node/bundler toolchain: the frontend stays vanilla HTML/CSS/JS served from a static dir, like the previous Tauri attempt. Build remains `cargo`-only.
- The manager **downloads** the backend from GitHub releases rather than bundling it, mirroring how yt-dlp is fetched. This keeps the manager slim and lets the backend update independently.
- The legacy `vrc-ytdlp-gui/` directory is left untouched (excluded from the workspace) until the new app reaches parity, then deleted in a follow-up.

## 2. Repo organization

### Considered approaches

**A. Cargo workspace; core as a library dependency of the manager (chosen).**
The manager's Rust side imports `vrc_ytdlp::config::Config`, `paths`, and the cache types directly. One `cargo build` builds both; types can't drift.

**B. Two independent crates; manager talks to backend only via CLI/HTTP.**
Loose coupling, but the old GUI did exactly this and had to duplicate `config.rs`, `paths.rs`, and cache scanning — they had already drifted (e.g. GUI config had `port`/`cache_max_size_mb` fields the backend spells differently). Rejected.

**C. Tauri sidecar (bundle backend inside the manager).**
Wrong fit: VRChat invokes the backend from its Tools dir as a yt-dlp stand-in, so the binary must be *installed there*, not embedded in the manager bundle. Rejected.

### Target layout

```
Cargo.toml                     # [workspace] members = ["crates/vrc-ytdlp", "apps/desktop/src-tauri"]
crates/
  vrc-ytdlp/                   # core backend, moved verbatim from /src + /Cargo.toml
    src/{lib,main,cli,args,config,executor,downloader,logging,paths,util}.rs
    src/server/{mod,pipeline,cache,lifecycle,client}.rs
apps/
  desktop/                     # NEW: "vrc-ytdlp Manager"
    ui/                        # static frontend (index.html, css/, js/) — no bundler
    src-tauri/
      Cargo.toml               # depends on vrc-ytdlp = { path = "../../../crates/vrc-ytdlp" }
      tauri.conf.json          # frontendDist: ../ui
      src/
        main.rs / lib.rs
        state.rs               # shared AppState (paths, config handle)
        commands/              # one module per domain (see §4)
        services/              # install.rs, releases.rs, process.rs, tray.rs
vrc-ytdlp-gui/                 # legacy — excluded from workspace; delete after parity
docs/
  superpowers/specs/           # this doc
  design/                      # HTML theme mockups
```

### Workspace details

- Root `Cargo.toml` becomes a pure `[workspace]` (resolver = "2") with shared `[workspace.dependencies]` for tokio/serde/reqwest/anyhow/tracing so versions stay aligned.
- `vrc-ytdlp-gui/` keeps its own `Cargo.toml`/lock and is **not** a workspace member (`exclude = ["vrc-ytdlp-gui"]`).
- Core crate is unchanged in content; only its path moves. `git mv` preserves history.

### CI impact (`.github/workflows/rust_release.yml`)

- `cargo set-version --bump patch` → `cargo set-version -p vrc-ytdlp --bump patch`.
- Version-read step parses `crates/vrc-ytdlp/Cargo.toml` instead of root.
- `cargo build --release` → `cargo build --release -p vrc-ytdlp` (artifact path `target/release/vrc-ytdlp.exe` is unchanged — workspace target dir is shared).
- A second, separate manual workflow `desktop_release.yml` builds the Tauri bundle (`cargo tauri build`) and uploads the installer ZIP/NSIS. Added later in the implementation plan; backend releases are not blocked on it.

## 3. Desktop app architecture

Three layers, each independently testable:

1. **UI (`ui/`)** — vanilla JS views calling `window.__TAURI__.core.invoke`. One JS module per screen; a tiny hash-router for the sidebar; CSS custom properties carry the theme.
2. **Tauri commands (`src-tauri/src/commands/`)** — thin `#[tauri::command]` wrappers: deserialize args, call a service, map errors to strings. No business logic.
3. **Services (`src-tauri/src/services/`)** + the **core crate** — all real logic. Reuses `vrc_ytdlp::config::Config` (single source of truth for config.json shape), `vrc_ytdlp::paths` for resolution, and the cache metadata types.

**Backend interaction model:** the manager never embeds server logic. It

- *installs* `vrc-ytdlp.exe` into the VRChat Tools dir (`%USERPROFILE%\AppData\LocalLow\VRChat\VRChat\Tools`) by downloading the GitHub release asset;
- *spawns* it with `--serve` for the media server (PID file + `/health` polling, same contract as today);
- *reads/writes* `config.json` in that dir via the shared `Config` type.

**Events over polling where it matters:** long operations (downloads, cookie extraction) report progress via Tauri events (`emit`) so the UI shows live progress bars instead of spinners. Server health stays a 5s poll (cheap, simple).

## 4. Feature set & screens

Feature parity with the old GUI, plus the missing piece: **backend installation/self-update** and a **first-run wizard**.

| Screen | Contents |
|---|---|
| **Setup wizard** (first run / re-runnable) | Steps: locate Tools dir (auto-detected, overridable) → install backend → install yt-dlp + ffmpeg → cookies opt-in (browser pick) → write config.json → done. Each step shows progress and can be skipped. |
| **Dashboard** | Status cards: backend (installed? version, update badge), server (running/stopped, port), yt-dlp (version, update badge), ffmpeg (present), cache (used / max, bar), cookies (age). Cards deep-link to their screens. Quick actions: start/stop server, update all. |
| **Components** (replaces "Updates") | One card per managed binary — backend, yt-dlp, ffmpeg: installed version, latest version, Check / Install / Update buttons with progress events. |
| **Server** | Start/stop toggle, health dot, port + idle-timeout (read from config), PID, uptime, last-activity. |
| **Cache** | Usage bar (used vs `cache_max_size_mb`), entry list from `.meta` files (name, source URL, size, last access), per-entry delete, clear-all with confirm. |
| **Config** | Form mirroring `Config`: paths, allowed/custom args (editable lists), cookies toggle + browser, timeouts, cache limits, plugin dirs, extractor args. Dirty-tracking, validation, Save / Reset-to-defaults. |
| **Cookies** | cookies.txt status + age, browser dropdown, Extract button (spawns yt-dlp `--cookies-from-browser`), progress + result. |
| **Logs** | Tail of newest `vrc-ytdlp.log*`, level coloring, text filter, follow-mode toggle, open-log-folder button. |
| **Tray** (Windows) | Minimize-to-tray; menu: Open, Start/Stop server, Quit. Closing the window hides to tray when the server is running (with a one-time notice). |

Tauri command surface (≈ old GUI's, grouped): `config::{get,save,exists,defaults}`, `server::{start,stop,status}`, `cache::{scan,delete_entry,clear}`, `components::{status,check_updates,install(component)}`, `cookies::{status,extract,browsers}`, `logs::{read,log_dir}`, `setup::{detect_dirs,wizard_state}`.

## 5. UX & theming

- **Layout:** fixed icon+label sidebar (~200px), content area with per-screen header (title + primary action). 980×660 default window, 800×560 min.
- **States first:** every screen designs its empty/loading/error states (e.g. Cache when server never ran; Components before first install). The wizard exists so no screen is ever a dead end.
- **Theming:** all colors/radii/spacing as CSS custom properties on `:root` with a `data-theme` attribute switch — making user-selectable themes nearly free. Three candidate directions are mocked as standalone HTML files in `docs/design/`:
  1. **`mockup-1-slate-teal.html`** — refinement of the current identity: near-black slate, teal `#7dcfb6` accent, soft glows.
  2. **`mockup-2-aurora.html`** — darker glassmorphism: violet/cyan aurora gradients, translucent cards, higher contrast type.
  3. **`mockup-3-instrument.html`** — utilitarian "instrument panel": warm dark gray, amber/green status LEDs, monospace numerals, denser layout.
  4. **`mockup-4-mica-dark.html`** — native Windows 11 Mica dark: layered neutral grays, faint wallpaper-tinted wash, system-blue `#60cdff` accent, restrained (no heavy glows).
  5. **`mockup-5-nord-frost.html`** — the Nord palette: Polar Night background, Frost `#88c0d0`/`#81a1c1` accents, Aurora status colors; flat, calm, professional.
- Mockups 4–5 add **desktop-app chrome** the first three omit — a custom 32px title bar with Windows caption buttons (— ☐ ✕) and a bottom status bar (server state, port, version) — to evaluate the more "native application" framing.
- The chosen mockup's tokens become `ui/css/theme.css`; the others can ship as alternate `data-theme` palettes later if desired.

## 6. Error handling

- Services return `anyhow::Result`; commands map to a serializable `{ code, message }` so the UI can branch (e.g. `tools-dir-missing` → offer wizard) instead of regex-matching strings.
- Network operations (release checks, downloads) have explicit timeouts and surface retry buttons in the UI; downloads write to `*.tmp` then atomic-rename (same pattern as `downloader.rs`).
- Spawning the server validates the exe exists first and reports stderr from a failed start.
- config.json writes are atomic (temp + rename) and re-validated through `Config` deserialization before save.

## 7. Testing

- **Core crate:** existing `#[cfg(test)]` tests move with their files; unchanged.
- **Manager services:** unit tests beside the code — release-JSON parsing (fixture), install path resolution, PID file lifecycle, config round-trip through the shared `Config`.
- **Commands/UI:** smoke-level only; logic lives in services precisely so the Tauri layer stays too thin to need tests.
- **Manual gate:** `cargo fmt` + `cargo clippy` across the workspace before finishing any change (CLAUDE.md rule), plus a scripted dev checklist: wizard on a clean Tools dir, server start/stop, cache clear.

## 8. Migration order (high level — implementation plan to follow)

1. Workspace conversion: move core to `crates/vrc-ytdlp`, root manifest → workspace, fix CI paths. *(Backend behavior unchanged — verifiable by `cargo test` + a release dry-run.)*
2. Scaffold `apps/desktop` Tauri 2 app with chosen theme tokens, sidebar shell, empty screens.
3. Services + commands per domain, in dependency order: config → components/install → server → cache → cookies → logs.
4. Setup wizard + dashboard (consume everything above).
5. Tray + polish (progress events, empty states).
6. `desktop_release.yml` workflow; delete `vrc-ytdlp-gui/`.

## 9. Out of scope

- Reworking backend internals (pipeline, cache eviction, lifecycle) — the manager only consumes them.
- Auto-update of the *manager itself* (Tauri updater) — possible later; not in v1.
- Non-Windows packaging.
