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
