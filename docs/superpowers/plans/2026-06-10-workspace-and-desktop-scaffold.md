# Workspace Conversion + Desktop Manager Scaffold — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the repo to a Cargo workspace (core backend → `crates/vrc-ytdlp`) and scaffold the new Tauri 2 desktop manager (`apps/desktop`) with the Mica Dark theme shell — migration phases 1–2 of `docs/superpowers/specs/2026-06-10-desktop-manager-design.md`.

**Architecture:** Root `Cargo.toml` becomes a pure `[workspace]` with two members: the core backend (moved verbatim via `git mv`, behavior unchanged) and the new manager crate, which depends on the core as a library (`vrc-ytdlp = { path = ... }`). The manager UI is vanilla HTML/CSS/JS (no bundler) served from `apps/desktop/ui`; theme tokens come from `docs/design/mockup-4-mica-dark.html` (the chosen direction). The legacy `vrc-ytdlp-gui/` is excluded from the workspace and left untouched.

**Tech Stack:** Rust (stable), Cargo workspaces, Tauri 2 (`withGlobalTauri`, static frontendDist, no Node toolchain), GitHub Actions (release workflow).

**Out of scope (follow-up plans):** services/commands per domain (config, components/install, server, cache, cookies, logs), setup wizard, dashboard content, tray, `desktop_release.yml`, deleting `vrc-ytdlp-gui/`. The screens scaffolded here are intentionally empty states.

**Branch / push policy:** Work on branch `test`. Commit per task. **Never push without the user's confirmation** (CLAUDE.md rule).

**Known repo facts (verified 2026-06-10):**
- The backend modularization (`src/lib.rs`, `cli.rs`, `args.rs`, `config.rs`, `paths.rs`, `logging.rs`, `util.rs`, `server/` submodule) exists **uncommitted** in the working tree. Task 1 commits it.
- `.gitignore` line 2 is corrupted: `.ideavrc-ytdlp-gui/target/` (two entries fused). `.idea/` is consequently not ignored.
- `.github/workflows/rust_release.yml` still packages `install.bat`/`uninstall.bat`, which were deleted in commit `6509622` — the release ZIP step would fail today. Task 3 fixes this alongside the workspace paths.
- The `M vrc-ytdlp-gui/...` entries in `git status` are CRLF line-ending noise (empty content diff) plus a generated schema. Leave them unstaged; do not commit anything under `vrc-ytdlp-gui/`.
- `vrc_ytdlp::config::Config` derives `Serialize + Deserialize` and implements `Default` (`src/config.rs:64,106`) — used by the scaffold's proof-of-linkage command.
- A PostToolUse hook (`.claude/hooks/rustfmt_on_edit.py`) runs rustfmt on edited Rust files automatically.

---

### Task 1: Fix `.gitignore` and commit the in-flight backend modularization

The working tree contains the finished library-split refactor plus new docs/config. Nothing else can proceed safely (Task 2 is a mass `git mv`) until this is committed.

**Files:**
- Modify: `.gitignore`
- Commit (already-edited): `src/**` (modularization), `CLAUDE.md`, `docs/**`, `.claude/**` (minus `settings.local.json`)

- [ ] **Step 1: Fix `.gitignore`**

Replace the entire contents of `.gitignore` with:

```gitignore
/target
.idea/
.superpowers/
.claude/settings.local.json
vrc-ytdlp-gui/target/
```

- [ ] **Step 2: Verify `.idea/` is now ignored**

Run: `git check-ignore -v .idea/ .claude/settings.local.json`
Expected: both paths print a matching rule; exit code 0. `git status --short` no longer lists `?? .idea/`.

- [ ] **Step 3: Verify the refactor is green before committing**

Run from repo root:

```powershell
cargo fmt --check
cargo clippy
cargo test
```

Expected: fmt exits 0 (no diff); clippy compiles with no errors (warnings about `cfg(windows)` unused imports only appear on non-Windows — on Windows expect a clean run); `cargo test` passes all tests in `args.rs`, `cli.rs`, `server/cache.rs` (`0 failed`).

If fmt or clippy report issues, fix them (`cargo fmt`; address clippy findings) before committing.

- [ ] **Step 4: Commit the backend refactor (code only)**

```powershell
git add .gitignore src
git commit -m "refactor(core): split backend into library modules under src/ and src/server/"
```

Note: `src/cache.rs → src/server/cache.rs` etc. are already staged-as-renames; `git add src` picks up the remaining new/modified files.

- [ ] **Step 5: Commit docs and project config**

```powershell
git add CLAUDE.md docs .claude
git commit -m "docs: add CLAUDE.md, desktop manager design spec, and theme mockups"
```

- [ ] **Step 6: Verify a clean-ish tree**

Run: `git status --short`
Expected: only the CRLF-noise entries under `vrc-ytdlp-gui/` remain (leave them).

---

### Task 2: Convert the repo to a Cargo workspace

Move the core crate verbatim to `crates/vrc-ytdlp`; root manifest becomes a pure workspace. **The diff must be a pure move** — no content changes to any `.rs` file, no dependency changes. (Shared `[workspace.dependencies]` from the spec is deferred to the services-phase plan, when the two members actually start duplicating version pins — YAGNI until then.)

**Files:**
- Create: `Cargo.toml` (new root workspace manifest)
- Move: `Cargo.toml` → `crates/vrc-ytdlp/Cargo.toml` (verbatim)
- Move: `src/**` → `crates/vrc-ytdlp/src/**` (verbatim)

- [ ] **Step 1: Move the crate with git mv (preserves history)**

```powershell
New-Item -ItemType Directory -Force crates/vrc-ytdlp
git mv Cargo.toml crates/vrc-ytdlp/Cargo.toml
git mv src crates/vrc-ytdlp/src
```

`Cargo.lock`, `config.json`, `LICENSE` etc. stay at the repo root. The `[[bin]] path = "src/main.rs"` inside the moved manifest is crate-relative and stays correct.

- [ ] **Step 2: Write the new root workspace manifest**

Create `Cargo.toml` at the repo root:

```toml
[workspace]
resolver = "2"
members = ["crates/vrc-ytdlp"]
exclude = ["vrc-ytdlp-gui"]
```

`exclude` keeps both legacy crates (`vrc-ytdlp-gui/` and `vrc-ytdlp-gui/src-tauri/`) out of the workspace so they neither build nor break the build.

- [ ] **Step 3: Build and test from the workspace root**

```powershell
cargo build
cargo test
```

Expected: compiles; same test pass count as Task 1 Step 3. `Cargo.lock` may gain a trivial diff (workspace membership) — that's expected.

- [ ] **Step 4: Verify history survived the move**

Run: `git log --oneline --follow -3 -- crates/vrc-ytdlp/src/cli.rs`
Expected: commits from before the move appear (e.g. Task 1's refactor commit).

- [ ] **Step 5: Verify the release binary path is unchanged**

```powershell
cargo build --release -p vrc-ytdlp
Test-Path target\release\vrc-ytdlp.exe
```

Expected: `True` — the workspace shares one root `target/`, so CI's artifact path still works.

- [ ] **Step 6: Commit**

```powershell
git add -A
git commit -m "refactor: convert repo to cargo workspace; move core to crates/vrc-ytdlp"
```

(`git status --short` before committing: expect only the moves, the new root `Cargo.toml`, and possibly `Cargo.lock`. The `vrc-ytdlp-gui` CRLF noise will be swept in by `-A` — restore it first with `git checkout -- vrc-ytdlp-gui` if it shows as modified.)

---

### Task 3: Fix the release workflow for the workspace layout

Update `.github/workflows/rust_release.yml`: version ops target the member crate, and drop the `install.bat`/`uninstall.bat` references (those files were deleted from the repo long ago — the workflow is broken today regardless of the workspace change).

**Files:**
- Modify: `.github/workflows/rust_release.yml`

- [ ] **Step 1: Point the version bump at the member crate**

Change the *Bump version (patch)* step (line ~45):

```yaml
      - name: Bump version (patch)
        run: cargo set-version -p vrc-ytdlp --bump patch
```

- [ ] **Step 2: Read the version from the moved manifest**

In the *Read current version from Cargo.toml* step, change only the path line:

```powershell
          $content = Get-Content -Raw -Path "crates/vrc-ytdlp/Cargo.toml"
```

and the two error messages may keep saying `Cargo.toml` (cosmetic). The `[package]`-section regex works unchanged on the moved file.

- [ ] **Step 3: Stage the right files in the bump commit**

In *Commit version bump and tag*, change:

```powershell
          git add crates/vrc-ytdlp/Cargo.toml Cargo.lock
```

- [ ] **Step 4: Build the member explicitly**

```yaml
      - name: Build Windows release binary
        run: cargo build --release -p vrc-ytdlp
```

(Artifact path `target\release\vrc-ytdlp.exe` is unchanged — verified in Task 2 Step 5.)

- [ ] **Step 5: Drop the deleted installer scripts from packaging**

In *Create release ZIP*, the `$paths` array becomes:

```powershell
          $paths = @(
            "target\release\vrc-ytdlp.exe"
          )
```

In *Verify packaged contents*, the `$allowed` array becomes:

```powershell
          $allowed = @(
            "vrc-ytdlp.exe"
          )
```

- [ ] **Step 6: Sanity-check the version-read logic locally**

Run from repo root (mimics the CI step against the real file):

```powershell
$content = Get-Content -Raw -Path "crates/vrc-ytdlp/Cargo.toml"
$packageSection = ($content -split "\r?\n\r?\n") | Where-Object { $_ -match "^\[package\]" }
[regex]::Match($packageSection, 'version\s*=\s*"([^"]+)"').Groups[1].Value
```

Expected output: `0.1.0`

- [ ] **Step 7: Commit**

```powershell
git add .github/workflows/rust_release.yml
git commit -m "ci(release): target workspace member crate; drop removed installer scripts"
```

---

### Task 4: Mica Dark UI shell (static frontend)

Pure static files — no Rust yet. Tokens come straight from `docs/design/mockup-4-mica-dark.html`; layout is the app chrome (caption bar, sidebar, status bar) plus an empty-state placeholder per screen. The JS guards on `window.__TAURI__` so the shell also opens in a plain browser for a quick visual check.

**Files:**
- Create: `apps/desktop/ui/index.html`
- Create: `apps/desktop/ui/css/theme.css`
- Create: `apps/desktop/ui/css/app.css`
- Create: `apps/desktop/ui/js/app.js`

- [ ] **Step 1: Create `apps/desktop/ui/css/theme.css`** (design tokens only — alternate palettes later become other `data-theme` blocks)

```css
/* Mica Dark — Win11 layered neutral grays, system-blue accent.
   Source of truth: docs/design/mockup-4-mica-dark.html */
:root {
  --bg: #1c1c1c;
  --bg-window: #202020;
  --bg-sidebar: #1d1d1d;
  --bg-card: #2b2b2b;
  --bg-card-hover: #323232;
  --bg-inset: #191919;
  --border: rgba(255, 255, 255, 0.07);
  --border-strong: rgba(255, 255, 255, 0.12);
  --accent: #60cdff;
  --accent-dim: #4ca8d4;
  --accent-glow: rgba(96, 205, 255, 0.14);
  --text: #ffffff;
  --text-2: #c8c8c8;
  --text-3: #8a8a8a;
  --ok: #6ccb5f;
  --warn: #fce100;
  --err: #ff99a4;
  --radius: 8px;
  --radius-sm: 5px;
  --font: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
  --mono: "Cascadia Code", Consolas, monospace;
  --caption-h: 32px;
  --statusbar-h: 26px;
}
```

- [ ] **Step 2: Create `apps/desktop/ui/css/app.css`** (chrome + shell layout + empty states; card/dashboard styles arrive with their screens in later plans)

```css
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { height: 100%; }
body {
  font-family: var(--font);
  /* Mica: faint wallpaper-tinted wash bleeding through the window */
  background:
    radial-gradient(1100px 500px at 12% -8%, rgba(96, 205, 255, 0.10), transparent 60%),
    radial-gradient(900px 600px at 105% 115%, rgba(140, 120, 220, 0.10), transparent 55%),
    var(--bg);
  color: var(--text);
  font-size: 13px;
  overflow: hidden;
  -webkit-font-smoothing: antialiased;
}
.window {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: rgba(32, 32, 32, 0.72);
}

/* ---------- title bar (caption) ---------- */
.caption {
  height: var(--caption-h);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  user-select: none;
}
/* children must not swallow drag events — buttons re-enable below */
.caption-title { display: flex; align-items: center; gap: 9px; padding-left: 13px; pointer-events: none; }
.caption-title .ico {
  width: 16px; height: 16px; border-radius: 4px;
  background: linear-gradient(135deg, var(--accent), #3f8fd0);
  display: grid; place-items: center;
}
.caption-title .ico svg { width: 8px; height: 8px; fill: #06121b; }
.caption-title span { font-size: 12px; color: var(--text-2); }
.caption-buttons { display: flex; height: 100%; }
.cap-btn {
  width: 46px; height: 100%;
  display: grid; place-items: center;
  transition: background 0.12s ease;
}
.cap-btn svg { width: 10px; height: 10px; stroke: var(--text-2); stroke-width: 1; fill: none; }
.cap-btn:hover { background: rgba(255, 255, 255, 0.06); }
.cap-btn.close:hover { background: #c42b1c; }
.cap-btn.close:hover svg { stroke: #fff; }

/* ---------- body row: sidebar + main ---------- */
.body { flex: 1; display: flex; overflow: hidden; }
.sidebar {
  width: 208px;
  flex-shrink: 0;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 8px 8px 12px;
}
.brand { display: flex; align-items: center; gap: 10px; padding: 8px 10px 14px; }
.brand-mark {
  width: 30px; height: 30px; border-radius: 7px;
  background: linear-gradient(135deg, var(--accent), #3f8fd0);
  display: grid; place-items: center;
}
.brand-mark svg { width: 13px; height: 13px; fill: #06121b; }
.brand-name { font-weight: 600; font-size: 13.5px; letter-spacing: -0.005em; }
.brand-name span {
  display: block; font-weight: 400; font-size: 10px; color: var(--text-3);
  letter-spacing: 0.06em; text-transform: uppercase; margin-top: 1px;
}
.nav { display: flex; flex-direction: column; gap: 2px; margin-top: 2px; }
.nav-label { font-size: 11px; color: var(--text-3); padding: 14px 12px 6px; }
.nav a {
  display: flex; align-items: center; gap: 11px;
  padding: 8px 11px; border-radius: var(--radius-sm);
  color: var(--text-2); text-decoration: none;
  position: relative;
  transition: background 0.12s ease;
}
.nav a svg {
  width: 16px; height: 16px;
  stroke: currentColor; fill: none; stroke-width: 1.6;
  stroke-linecap: round; stroke-linejoin: round;
}
.nav a:hover { color: var(--text); background: rgba(255, 255, 255, 0.045); }
.nav a.active { color: var(--text); background: rgba(255, 255, 255, 0.06); }
.nav a.active::before {
  content: "";
  position: absolute; left: 2px; top: 9px; bottom: 9px; width: 3px;
  border-radius: 3px;
  background: var(--accent);
}

/* ---------- main ---------- */
.main { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.topbar { display: flex; align-items: center; justify-content: space-between; padding: 16px 26px 0; }
h1 { font-size: 20px; font-weight: 600; letter-spacing: -0.01em; }
.subtitle { color: var(--text-2); margin-top: 3px; font-size: 12.5px; }
.content { padding: 20px 26px 26px; overflow-y: auto; flex: 1; }

/* ---------- empty states ---------- */
.empty {
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius);
  padding: 56px 24px;
  text-align: center;
  color: var(--text-3);
}
.empty h2 { font-size: 14px; font-weight: 600; color: var(--text-2); margin-bottom: 6px; }
.empty p { font-size: 12.5px; }

/* ---------- status bar ---------- */
.statusbar {
  height: var(--statusbar-h);
  flex-shrink: 0;
  border-top: 1px solid var(--border);
  display: flex; align-items: center; gap: 18px;
  padding: 0 14px;
  font-size: 11.5px; color: var(--text-3);
  background: rgba(0, 0, 0, 0.18);
}
.statusbar .seg { display: flex; align-items: center; gap: 6px; }
.statusbar .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--ok); box-shadow: 0 0 7px rgba(108, 203, 95, 0.7); }
.statusbar .dot.off { background: var(--text-3); box-shadow: none; }
.statusbar code { font-family: var(--mono); color: var(--text-2); }
.statusbar .right { margin-left: auto; font-family: var(--mono); }
```

- [ ] **Step 3: Create `apps/desktop/ui/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>vrc-ytdlp Manager</title>
<link rel="stylesheet" href="css/theme.css">
<link rel="stylesheet" href="css/app.css">
</head>
<body>
<div class="window">

  <div class="caption" data-tauri-drag-region>
    <div class="caption-title">
      <div class="ico"><svg viewBox="0 0 24 24"><path d="M6 4l14 8-14 8z"/></svg></div>
      <span>vrc-ytdlp Manager</span>
    </div>
    <div class="caption-buttons">
      <div class="cap-btn" id="cap-min"><svg viewBox="0 0 12 12"><line x1="2" y1="6" x2="10" y2="6"/></svg></div>
      <div class="cap-btn" id="cap-max"><svg viewBox="0 0 12 12"><rect x="2.5" y="2.5" width="7" height="7"/></svg></div>
      <div class="cap-btn close" id="cap-close"><svg viewBox="0 0 12 12"><line x1="3" y1="3" x2="9" y2="9"/><line x1="9" y1="3" x2="3" y2="9"/></svg></div>
    </div>
  </div>

  <div class="body">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark"><svg viewBox="0 0 24 24"><path d="M6 4l14 8-14 8z"/></svg></div>
        <div class="brand-name">vrc-ytdlp<span>Manager</span></div>
      </div>
      <nav class="nav" id="nav">
        <a href="#/dashboard" data-screen="dashboard"><svg viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="9" rx="1"/><rect x="14" y="3" width="7" height="5" rx="1"/><rect x="14" y="12" width="7" height="9" rx="1"/><rect x="3" y="16" width="7" height="5" rx="1"/></svg>Dashboard</a>
        <a href="#/components" data-screen="components"><svg viewBox="0 0 24 24"><path d="M21 16V8l-9-5-9 5v8l9 5 9-5z"/><path d="M3.3 7.3L12 12l8.7-4.7M12 22V12"/></svg>Components</a>
        <a href="#/server" data-screen="server"><svg viewBox="0 0 24 24"><rect x="2" y="5" width="20" height="6" rx="2"/><rect x="2" y="13" width="20" height="6" rx="2"/><path d="M6 8h.01M6 16h.01"/></svg>Server</a>
        <a href="#/cache" data-screen="cache"><svg viewBox="0 0 24 24"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14c0 1.7 4 3 9 3s9-1.3 9-3V5"/><path d="M3 12c0 1.7 4 3 9 3s9-1.3 9-3"/></svg>Cache</a>
        <div class="nav-label">Settings</div>
        <a href="#/config" data-screen="config"><svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 00.3 1.9l.1.1a2 2 0 11-2.8 2.8l-.1-.1a1.7 1.7 0 00-1.9-.3 1.7 1.7 0 00-1 1.5V21a2 2 0 11-4 0v-.1a1.7 1.7 0 00-1-1.6 1.7 1.7 0 00-1.9.3l-.1.1a2 2 0 11-2.8-2.8l.1-.1a1.7 1.7 0 00.3-1.9 1.7 1.7 0 00-1.5-1H3a2 2 0 110-4h.1a1.7 1.7 0 001.6-1 1.7 1.7 0 00-.3-1.9l-.1-.1a2 2 0 112.8-2.8l.1.1a1.7 1.7 0 001.9.3h0a1.7 1.7 0 001-1.5V3a2 2 0 114 0v.1a1.7 1.7 0 001 1.6h0a1.7 1.7 0 001.9-.3l.1-.1a2 2 0 112.8 2.8l-.1.1a1.7 1.7 0 00-.3 1.9v0a1.7 1.7 0 001.5 1H21a2 2 0 110 4h-.1a1.7 1.7 0 00-1.5 1z"/></svg>Configuration</a>
        <a href="#/cookies" data-screen="cookies"><svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><circle cx="9" cy="9" r="1" fill="currentColor"/><circle cx="15" cy="10" r="1" fill="currentColor"/><circle cx="10" cy="15" r="1" fill="currentColor"/><circle cx="15" cy="15" r="1" fill="currentColor"/></svg>Cookies</a>
        <a href="#/logs" data-screen="logs"><svg viewBox="0 0 24 24"><path d="M4 4h16v16H4z"/><path d="M8 9h8M8 13h8M8 17h5"/></svg>Logs</a>
      </nav>
    </aside>

    <div class="main">
      <div class="topbar">
        <div>
          <h1 id="screen-title"></h1>
          <div class="subtitle" id="screen-subtitle"></div>
        </div>
      </div>
      <div class="content" id="content"></div>
    </div>
  </div>

  <div class="statusbar">
    <div class="seg"><span class="dot off"></span> <span id="status-server">Server not running</span></div>
    <div class="right" id="status-version"></div>
  </div>

</div>
<script src="js/app.js"></script>
</body>
</html>
```

- [ ] **Step 4: Create `apps/desktop/ui/js/app.js`**

```js
// Shell: hash router + window chrome. Screens are placeholders until their
// service phases land (see docs/superpowers/specs/2026-06-10-desktop-manager-design.md §4).
const TAURI = window.__TAURI__ ?? null;

const SCREENS = {
  dashboard: { title: "Dashboard", subtitle: "Status overview" },
  components: { title: "Components", subtitle: "Backend, yt-dlp and ffmpeg versions" },
  server: { title: "Server", subtitle: "Media server control" },
  cache: { title: "Cache", subtitle: "Cached streams on disk" },
  config: { title: "Configuration", subtitle: "config.json in the VRChat Tools directory" },
  cookies: { title: "Cookies", subtitle: "Browser cookie extraction" },
  logs: { title: "Logs", subtitle: "Backend log files" },
};

function render(id) {
  const screen = SCREENS[id];
  document.getElementById("screen-title").textContent = screen.title;
  document.getElementById("screen-subtitle").textContent = screen.subtitle;
  document.getElementById("content").innerHTML = `
    <div class="empty">
      <h2>${screen.title} isn't wired up yet</h2>
      <p>This screen arrives in a later phase of the desktop manager plan.</p>
    </div>`;
  for (const a of document.querySelectorAll("#nav a")) {
    a.classList.toggle("active", a.dataset.screen === id);
  }
}

function route() {
  const id = location.hash.replace("#/", "");
  render(SCREENS[id] ? id : "dashboard");
}
window.addEventListener("hashchange", route);
route();

if (TAURI) {
  const appWindow = TAURI.window.getCurrentWindow();
  document.getElementById("cap-min").addEventListener("click", () => appWindow.minimize());
  document.getElementById("cap-max").addEventListener("click", () => appWindow.toggleMaximize());
  document.getElementById("cap-close").addEventListener("click", () => appWindow.close());

  TAURI.core.invoke("version").then((v) => {
    document.getElementById("status-version").textContent = `v${v}`;
  });
}
```

- [ ] **Step 5: Visual smoke check in a browser**

Run: `Start-Process apps/desktop/ui/index.html`
Expected: Mica Dark shell renders; clicking sidebar items switches the title/empty state; caption buttons and version are inert (no `__TAURI__` in a plain browser). Close the browser tab.

- [ ] **Step 6: Commit**

```powershell
git add apps/desktop/ui
git commit -m "feat(desktop): Mica Dark static UI shell with hash router and empty screens"
```

---

### Task 5: Scaffold the Tauri manager crate

The Rust side: workspace member `apps/desktop/src-tauri`, depending on the core crate. Includes the serializable command-error type from spec §6 (TDD — it's the one piece of real logic) and two commands: `version` (status bar) and `default_config` (proves the shared-`Config` linkage that motivated the workspace).

**Files:**
- Modify: `Cargo.toml` (root — add member)
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/build.rs`
- Create: `apps/desktop/src-tauri/tauri.conf.json`
- Create: `apps/desktop/src-tauri/capabilities/default.json`
- Create: `apps/desktop/src-tauri/icons/icon.ico`, `icons/icon.png` (copied from legacy)
- Create: `apps/desktop/src-tauri/src/main.rs`, `src/lib.rs`
- Create: `apps/desktop/src-tauri/src/commands/mod.rs`, `commands/error.rs`, `commands/app.rs`

- [ ] **Step 1: Create `apps/desktop/src-tauri/Cargo.toml`**

```toml
[package]
name = "vrc-ytdlp-manager"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
vrc-ytdlp = { path = "../../../crates/vrc-ytdlp" }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

(The legacy GUI manifest wrongly listed `tauri-build` under `[dependencies]` too — don't replicate that.)

- [ ] **Step 2: Create `apps/desktop/src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 3: Create `apps/desktop/src-tauri/tauri.conf.json`**

Window geometry per spec §5 (980×660 default, 800×560 min); `decorations: false` because the shell draws its own caption bar; no `devUrl`/`beforeDevCommand` so plain `cargo run` serves `../ui` from disk — no Node toolchain.

```json
{
  "productName": "vrc-ytdlp-manager",
  "version": "0.1.0",
  "identifier": "com.vrc-ytdlp.manager",
  "build": {
    "frontendDist": "../ui",
    "beforeBuildCommand": "",
    "beforeDevCommand": ""
  },
  "app": {
    "withGlobalTauri": true,
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'"
    },
    "windows": [
      {
        "title": "vrc-ytdlp Manager",
        "width": 980,
        "height": 660,
        "minWidth": 800,
        "minHeight": 560,
        "resizable": true,
        "decorations": false,
        "center": true
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": ["icons/icon.ico", "icons/icon.png"]
  }
}
```

- [ ] **Step 4: Create `apps/desktop/src-tauri/capabilities/default.json`**

The custom caption bar needs the window-control permissions (`core:default` alone doesn't grant minimize/maximize/close).

```json
{
  "identifier": "default",
  "description": "Main window: core APIs plus custom-caption window controls",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-minimize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-close",
    "core:window:allow-start-dragging"
  ]
}
```

- [ ] **Step 5: Copy icons from the legacy app**

```powershell
New-Item -ItemType Directory -Force apps/desktop/src-tauri/icons
Copy-Item vrc-ytdlp-gui/src-tauri/icons/icon.ico apps/desktop/src-tauri/icons/
Copy-Item vrc-ytdlp-gui/src-tauri/icons/icon.png apps/desktop/src-tauri/icons/
```

(`tauri::generate_context!` fails at compile time if the icons referenced in `tauri.conf.json` are missing.)

- [ ] **Step 6: Write the failing test — `commands/error.rs`**

Create `apps/desktop/src-tauri/src/commands/error.rs` with the test only (the type doesn't exist yet, so this won't compile — that's the failing state):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn anyhow_chain_flattens_into_code_and_message() {
        let err = anyhow::anyhow!("root cause").context("outer context");
        let cmd: CmdError = err.into();
        assert_eq!(cmd.code, "internal");
        assert_eq!(cmd.message, "outer context: root cause");
        let json = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["code"], "internal");
        assert_eq!(json["message"], "outer context: root cause");
    }
}
```

Also create the minimal module wiring so the crate parses — `apps/desktop/src-tauri/src/commands/mod.rs`:

```rust
mod error;

pub use error::CmdError;
```

`apps/desktop/src-tauri/src/lib.rs` (commands wired in Step 9):

```rust
mod commands;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`apps/desktop/src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    vrc_ytdlp_manager::run()
}
```

Add the member to the root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/vrc-ytdlp", "apps/desktop/src-tauri"]
exclude = ["vrc-ytdlp-gui"]
```

- [ ] **Step 7: Run the test to verify it fails**

Run: `cargo test -p vrc-ytdlp-manager`
Expected: **compile error** — `cannot find type CmdError in this scope`.

- [ ] **Step 8: Implement `CmdError` (prepend to `commands/error.rs`, above the tests)**

```rust
use serde::Serialize;

/// Serializable command error (spec §6): the UI branches on `code` and
/// displays `message`. Domain commands map known failures to specific
/// codes (e.g. "tools-dir-missing"); everything else is "internal".
#[derive(Debug, Serialize)]
pub struct CmdError {
    pub code: String,
    pub message: String,
}

impl CmdError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for CmdError {
    fn from(err: anyhow::Error) -> Self {
        // "{:#}" renders the full context chain: "outer: inner".
        Self::new("internal", format!("{err:#}"))
    }
}
```

- [ ] **Step 9: Add the commands and wire the handler**

Create `apps/desktop/src-tauri/src/commands/app.rs`:

```rust
use vrc_ytdlp::config::Config;

use super::CmdError;

/// Manager version for the status bar.
#[tauri::command]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Default backend config, straight from the shared core type — the
/// Config screen's reset-to-defaults source.
#[tauri::command]
pub fn default_config() -> Result<Config, CmdError> {
    Ok(Config::default())
}
```

Update `commands/mod.rs`:

```rust
pub mod app;
mod error;

pub use error::CmdError;
```

Update `lib.rs`:

```rust
mod commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::app::version,
            commands::app::default_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 10: Run tests to verify they pass**

Run: `cargo test -p vrc-ytdlp-manager`
Expected: `anyhow_chain_flattens_into_code_and_message ... ok`, 0 failed. (First build pulls the full Tauri dependency tree — expect several minutes.)

Then run the whole workspace: `cargo test`
Expected: core tests + manager test all pass.

- [ ] **Step 11: Commit**

```powershell
git add Cargo.toml Cargo.lock apps/desktop/src-tauri
git commit -m "feat(desktop): scaffold Tauri manager crate as workspace member wired to core"
```

---

### Task 6: Workspace verification, manual launch, CLAUDE.md refresh

**Files:**
- Modify: `CLAUDE.md` (layout + build commands changed)

- [ ] **Step 1: Format and lint the whole workspace**

```powershell
cargo fmt
cargo clippy --workspace
```

Expected: no diff from fmt; clippy clean (fix anything it reports before continuing).

- [ ] **Step 2: Manual launch check**

Run: `cargo run -p vrc-ytdlp-manager`
Expected: a 980×660 undecorated window with the Mica Dark shell. Verify: sidebar navigation switches screens; the status bar shows `v0.1.0` (proves `invoke("version")` round-trip); caption minimize/maximize/close work (proves capabilities); dragging the caption moves the window. Close the app.

If the window opens blank, check the dev console (right-click → Inspect) for CSP or script errors before changing any Rust.

- [ ] **Step 3: Update CLAUDE.md for the new layout**

In the **What this is** section, change the backend bullet's location reference from `src/` to `crates/vrc-ytdlp/src/`, and add a bullet for the manager:

```markdown
- **`crates/vrc-ytdlp/`** — the backend (`vrc-ytdlp`): a library (`lib.rs`) with a thin binary (`main.rs` → `cli::run`). Runs as a CLI; `--serve` starts an Axum HTTP server that resolves and streams video URLs via a yt-dlp subprocess. Wrapper side: `cli.rs`, `args.rs` (whitelist filtering), `config.rs`, `executor.rs` (spawns yt-dlp), `downloader.rs` (yt-dlp self-update). Server side under `server/`: `mod.rs` (Axum app), `pipeline.rs`, `cache.rs`, `lifecycle.rs` (Windows job objects), `client.rs` (wrapper→server HTTP).
- **`apps/desktop/`** — the desktop manager (Tauri 2, in progress): static UI in `ui/` (no bundler), Rust side in `src-tauri/` depending on the core crate. Design: `docs/superpowers/specs/2026-06-10-desktop-manager-design.md`.
- **`vrc-ytdlp-gui/`** — legacy (Iced + first Tauri attempt), excluded from the workspace; do not build on it. Deleted once the manager reaches parity.
```

In **Build & run**, replace the commands block:

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

Update the unit-test note to mention `crates/vrc-ytdlp/src/{args,cli}.rs`, `crates/vrc-ytdlp/src/server/cache.rs`, and `apps/desktop/src-tauri/src/commands/error.rs`.

In **Gotchas**, update the release-workflow sentence: the workflow bumps `crates/vrc-ytdlp/Cargo.toml` via `cargo set-version -p vrc-ytdlp`.

- [ ] **Step 4: Check the run-server skill still works**

Read `.claude/skills/run-server/SKILL.md`; if it references `cargo run -- --serve` or root-crate paths, update it to `cargo run -p vrc-ytdlp -- --serve`.

- [ ] **Step 5: Final commit**

```powershell
git add CLAUDE.md .claude/skills
git commit -m "docs: update CLAUDE.md and skills for workspace layout"
```

- [ ] **Step 6: Report**

Summarize to the user: workspace converted, CI fixed (including the pre-existing broken installer packaging), manager scaffold launches with the Mica Dark shell. **Ask before pushing.** Next plans: services/commands per domain (config → components → server → cache → cookies → logs), then wizard + dashboard, then tray/polish, then `desktop_release.yml` + legacy deletion.
