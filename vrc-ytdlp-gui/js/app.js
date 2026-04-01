// ================================================================
// vrc-ytdlp GUI — Frontend
// ================================================================

const { invoke } = window.__TAURI__.core;

let config = null;
let serverRunning = false;
let logLines = [];
let logFilter = '';

// ================================================================
// Navigation
// ================================================================

document.querySelectorAll('.nav-item').forEach(btn => {
  btn.addEventListener('click', () => {
    const page = btn.dataset.page;
    document.querySelectorAll('.nav-item').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    document.getElementById(`page-${page}`).classList.add('active');

    // Trigger page-specific loads
    if (page === 'cache') refreshCache();
    if (page === 'logs') refreshLogs();
    if (page === 'updates') refreshUpdates();
    if (page === 'cookies') refreshCookies();
  });
});

// ================================================================
// Init
// ================================================================

async function init() {
  try {
    config = await invoke('get_config');
  } catch {
    config = await invoke('get_default_config');
  }
  renderDashboard();
  renderConfigForm();
  renderServerCard();
  pollHealth();
  setInterval(pollHealth, 5000);
}

// ================================================================
// Dashboard
// ================================================================

function renderDashboard() {
  const grid = document.getElementById('dashboard-cards');
  grid.innerHTML = `
    <div class="card card-clickable" onclick="navigateTo('server')">
      <div class="card-header">
        <div class="card-label">Server</div>
        <div class="dot ${serverRunning ? 'dot-green' : 'dot-red'}"></div>
      </div>
      <div class="card-value">${serverRunning ? 'Running' : 'Stopped'}</div>
      <div class="card-detail">Port ${config.server_port}</div>
    </div>
    <div class="card card-clickable" onclick="navigateTo('cache')">
      <div class="card-label">Cache</div>
      <div class="card-value" id="dash-cache-value">—</div>
      <div style="margin-top: 8px;">
        <div class="progress-track"><div class="progress-fill" id="dash-cache-bar" style="width: 0%"></div></div>
        <div class="progress-labels"><span id="dash-cache-pct">—</span><span>${config.cache_max_size_mb} MB limit</span></div>
      </div>
    </div>
    <div class="card card-clickable" onclick="navigateTo('updates')">
      <div class="card-header">
        <div class="card-label">yt-dlp</div>
        <span class="pill pill-accent" id="dash-ytdlp-pill">—</span>
      </div>
      <div class="card-value" id="dash-ytdlp-version">—</div>
      <div class="card-detail" id="dash-ytdlp-detail"></div>
    </div>
    <div class="card card-clickable" onclick="navigateTo('cookies')">
      <div class="card-header">
        <div class="card-label">Cookies</div>
        <div class="dot dot-red" id="dash-cookie-dot"></div>
      </div>
      <div class="card-value" id="dash-cookie-value">—</div>
      <div class="card-detail" id="dash-cookie-detail"></div>
    </div>
  `;
  refreshDashboardData();
}

async function refreshDashboardData() {
  // Version info
  try {
    const info = await invoke('get_version_info', {
      ytdlpLocation: config.ytdlp_location,
      ffmpegLocation: config.ffmpeg_location,
    });
    document.getElementById('dash-ytdlp-version').textContent = info.current || 'Not installed';
    const pill = document.getElementById('dash-ytdlp-pill');
    if (info.current) { pill.textContent = 'Installed'; pill.className = 'pill pill-accent'; }
    else { pill.textContent = 'Missing'; pill.className = 'pill pill-yellow'; }
  } catch {}

  // Cookie status
  try {
    const cs = await invoke('check_cookies');
    const dot = document.getElementById('dash-cookie-dot');
    const val = document.getElementById('dash-cookie-value');
    const det = document.getElementById('dash-cookie-detail');
    if (cs.exists) {
      dot.className = 'dot dot-green';
      val.textContent = 'Available';
      det.textContent = cs.age_description ? `Updated ${cs.age_description}` : '';
    } else {
      dot.className = 'dot dot-red';
      val.textContent = 'Not found';
      det.textContent = '';
    }
  } catch {}

  // Cache
  try {
    const summary = await invoke('scan_cache', { cacheDir: config.cache_dir });
    const mb = (summary.total_size_bytes / 1024 / 1024).toFixed(1);
    const pct = config.cache_max_size_mb > 0
      ? Math.min(100, (summary.total_size_bytes / (config.cache_max_size_mb * 1024 * 1024)) * 100)
      : 0;
    document.getElementById('dash-cache-value').textContent = `${mb} MB`;
    document.getElementById('dash-cache-bar').style.width = `${pct}%`;
    document.getElementById('dash-cache-pct').textContent = `${pct.toFixed(0)}% · ${summary.entry_count} files`;
  } catch {}
}

function navigateTo(page) {
  document.querySelector(`.nav-item[data-page="${page}"]`).click();
}

// ================================================================
// Health polling
// ================================================================

async function pollHealth() {
  try {
    serverRunning = await invoke('check_server_health', { port: config.server_port });
  } catch {
    serverRunning = false;
  }
  updateServerUI();
}

function updateServerUI() {
  // Sidebar
  const dot = document.getElementById('sidebar-dot');
  const txt = document.getElementById('sidebar-status-text');
  dot.className = serverRunning ? 'dot dot-green' : 'dot dot-red';
  txt.textContent = serverRunning ? 'Server running' : 'Server stopped';
  document.getElementById('sidebar-port').textContent = `Port ${config.server_port}`;

  // Dashboard card (if visible)
  const dashDot = document.querySelector('#dashboard-cards .dot');
  if (dashDot) {
    dashDot.className = serverRunning ? 'dot dot-green' : 'dot dot-red';
    const val = document.querySelector('#dashboard-cards .card-value');
    if (val) val.textContent = serverRunning ? 'Running' : 'Stopped';
  }

  renderServerCard();
}

// ================================================================
// Server
// ================================================================

function renderServerCard() {
  const card = document.getElementById('server-card');
  card.innerHTML = `
    <div class="card-header" style="margin-bottom: 12px;">
      <div style="display: flex; align-items: center; gap: 8px;">
        <div class="dot ${serverRunning ? 'dot-green' : 'dot-red'}"></div>
        <span style="font-size: 14px; font-weight: 500;">${serverRunning ? `Running on port ${config.server_port}` : 'Stopped'}</span>
      </div>
    </div>
    <div style="display: flex; gap: 16px; color: var(--text-muted); font-size: 11px; margin-bottom: 16px;">
      <span>Port: ${config.server_port}</span>
      <span>Idle timeout: ${config.server_idle_timeout_secs}s</span>
    </div>
    <button class="btn ${serverRunning ? 'btn-danger' : 'btn-primary'}" id="server-toggle">
      ${serverRunning ? 'Stop Server' : 'Start Server'}
    </button>
  `;
  document.getElementById('server-toggle').addEventListener('click', toggleServer);
}

async function toggleServer() {
  const status = document.getElementById('server-status');
  try {
    if (serverRunning) {
      await invoke('stop_server');
      status.innerHTML = '<span class="msg-success">Server stopped</span>';
    } else {
      await invoke('start_server', { port: config.server_port, idleTimeout: config.server_idle_timeout_secs });
      status.innerHTML = '<span class="msg-success">Server starting...</span>';
    }
    setTimeout(pollHealth, 2000);
  } catch (e) {
    status.innerHTML = `<span class="msg-error">${e}</span>`;
  }
}

// ================================================================
// Config
// ================================================================

let configDraft = null;

function renderConfigForm() {
  configDraft = JSON.parse(JSON.stringify(config));
  const form = document.getElementById('config-form');

  form.innerHTML = `
    <div class="section-header">Paths</div>
    <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 24px;">
      ${formInput('ytdlp_location', 'yt-dlp Location', configDraft.ytdlp_location)}
      ${formInput('ffmpeg_location', 'ffmpeg Location', configDraft.ffmpeg_location)}
      ${formInput('plugin_dirs', 'Plugin Directory', configDraft.plugin_dirs || '', '(optional)')}
    </div>

    <div class="section-header section-gap">Server</div>
    <div class="form-row form-row-3" style="margin-bottom: 24px;">
      ${formInput('server_port', 'Port', configDraft.server_port)}
      ${formInput('server_idle_timeout_secs', 'Idle Timeout (secs)', configDraft.server_idle_timeout_secs)}
      ${formInput('bgutil_pot_port', 'bgutil-pot Port', configDraft.bgutil_pot_port)}
    </div>

    <div class="section-header section-gap">Downloads</div>
    <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 24px;">
      ${formInput('execution_timeout_secs', 'Execution Timeout (secs)', configDraft.execution_timeout_secs)}
    </div>

    <div class="section-header section-gap">Cookies</div>
    <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 24px;">
      <div class="form-toggle-row">
        <div class="form-toggle-label">
          <div class="form-toggle-title">Enable Cookies</div>
          <div class="form-toggle-desc">Authenticate with browser cookies for restricted content</div>
        </div>
        <label class="toggle">
          <input type="checkbox" id="cfg-cookies" ${configDraft.cookies ? 'checked' : ''}>
          <div class="toggle-track"></div>
          <div class="toggle-thumb"></div>
        </label>
      </div>
      <div class="form-group">
        <label class="form-label">Browser</label>
        <select class="form-select" id="cfg-cookies_browser"></select>
      </div>
    </div>

    <div class="section-header section-gap">Cache</div>
    <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 24px;">
      ${formInput('cache_dir', 'Cache Directory', configDraft.cache_dir)}
      <div class="form-row form-row-2">
        ${formInput('cache_max_size_mb', 'Max Size (MB)', configDraft.cache_max_size_mb)}
        ${formInput('cache_ttl_secs', 'TTL (secs)', configDraft.cache_ttl_secs)}
      </div>
    </div>

    <div class="section-header section-gap">Updates</div>
    <div style="margin-bottom: 24px;">
      ${formInput('update_check_days', 'Check Interval (days)', configDraft.update_check_days)}
    </div>
  `;

  // Populate browser dropdown
  invoke('get_browsers').then(browsers => {
    const sel = document.getElementById('cfg-cookies_browser');
    browsers.forEach(b => {
      const opt = document.createElement('option');
      opt.value = b; opt.textContent = b;
      if (b === configDraft.cookies_browser) opt.selected = true;
      sel.appendChild(opt);
    });
    sel.addEventListener('change', () => { configDraft.cookies_browser = sel.value; checkDirty(); });
  });

  // Wire up inputs
  form.querySelectorAll('.form-input[data-field]').forEach(input => {
    input.addEventListener('input', () => {
      const field = input.dataset.field;
      const val = input.value;
      // Numbers
      if (['server_port', 'server_idle_timeout_secs', 'bgutil_pot_port', 'execution_timeout_secs',
           'cache_max_size_mb', 'cache_ttl_secs', 'update_check_days'].includes(field)) {
        configDraft[field] = parseInt(val) || 0;
      } else if (field === 'plugin_dirs') {
        configDraft[field] = val || null;
      } else {
        configDraft[field] = val;
      }
      checkDirty();
    });
  });

  document.getElementById('cfg-cookies').addEventListener('change', e => {
    configDraft.cookies = e.target.checked;
    checkDirty();
  });

  // Save/reset
  document.getElementById('config-save').addEventListener('click', saveConfig);
  document.getElementById('config-reset').addEventListener('click', async () => {
    configDraft = await invoke('get_default_config');
    renderConfigForm();
  });
}

function formInput(field, label, value, placeholder) {
  return `<div class="form-group">
    <label class="form-label">${label}</label>
    <input class="form-input" data-field="${field}" value="${value ?? ''}" placeholder="${placeholder || ''}">
  </div>`;
}

function checkDirty() {
  const dirty = JSON.stringify(configDraft) !== JSON.stringify(config);
  document.getElementById('config-save').disabled = !dirty;
}

async function saveConfig() {
  const status = document.getElementById('config-status');
  try {
    await invoke('save_config', { cfg: configDraft });
    config = JSON.parse(JSON.stringify(configDraft));
    status.innerHTML = '<span class="msg-success">Configuration saved.</span>';
    checkDirty();
    renderDashboard();
    renderServerCard();
  } catch (e) {
    status.innerHTML = `<span class="msg-error">Save failed: ${e}</span>`;
  }
}

// ================================================================
// Cache
// ================================================================

async function refreshCache() {
  const summary = document.getElementById('cache-summary');
  const list = document.getElementById('cache-list');
  const clearBtn = document.getElementById('cache-clear');

  try {
    const data = await invoke('scan_cache', { cacheDir: config.cache_dir });
    const mb = (data.total_size_bytes / 1024 / 1024).toFixed(1);
    const pct = config.cache_max_size_mb > 0
      ? (data.total_size_bytes / (config.cache_max_size_mb * 1024 * 1024) * 100).toFixed(0)
      : 0;

    summary.innerHTML = `
      <div style="font-size: 14px; font-weight: 500; margin-bottom: 8px;">${mb} MB used</div>
      <div class="progress-track"><div class="progress-fill" style="width: ${pct}%"></div></div>
      <div class="progress-labels"><span>${pct}% of ${config.cache_max_size_mb} MB</span><span>${data.entry_count} files</span></div>
    `;

    clearBtn.style.display = data.entry_count > 0 ? '' : 'none';

    list.innerHTML = data.entries.map(e => `
      <div class="file-item">
        <span class="file-name">${e.file_name}</span>
        <span class="file-size">${formatSize(e.size_bytes)}</span>
        <button class="btn btn-danger btn-sm" onclick="deleteCacheEntry('${e.path.replace(/'/g, "\\'")}')">Delete</button>
      </div>
    `).join('');
  } catch (e) {
    summary.innerHTML = `<span class="msg-error">${e}</span>`;
  }
}

async function deleteCacheEntry(path) {
  try {
    await invoke('delete_cache_entry', { path });
    refreshCache();
  } catch (e) {
    alert(e);
  }
}

document.getElementById('cache-refresh').addEventListener('click', refreshCache);
document.getElementById('cache-clear').addEventListener('click', async () => {
  if (!confirm('Clear all cached files?')) return;
  try {
    await invoke('clear_cache', { cacheDir: config.cache_dir });
    refreshCache();
  } catch (e) {
    alert(e);
  }
});

// ================================================================
// Logs
// ================================================================

async function refreshLogs() {
  try {
    logLines = await invoke('read_logs', { maxLines: 1000 });
    renderLogs();
  } catch {}
}

function renderLogs() {
  const container = document.getElementById('log-container');
  const filter = logFilter.toLowerCase();
  const filtered = filter
    ? logLines.filter(l => l.text.toLowerCase().includes(filter))
    : logLines;

  container.innerHTML = filtered.map(l =>
    `<div class="log-line log-${l.level}">${escapeHtml(l.text)}</div>`
  ).join('');
  container.scrollTop = container.scrollHeight;
}

document.getElementById('log-filter').addEventListener('input', e => {
  logFilter = e.target.value;
  renderLogs();
});
document.getElementById('log-reload').addEventListener('click', refreshLogs);

// ================================================================
// Updates
// ================================================================

async function refreshUpdates() {
  try {
    const info = await invoke('get_version_info', {
      ytdlpLocation: config.ytdlp_location,
      ffmpegLocation: config.ffmpeg_location,
    });
    renderYtdlpCard(info);
    renderFfmpegCard(info);
  } catch {}
}

function renderYtdlpCard(info) {
  document.getElementById('ytdlp-card').innerHTML = `
    <div class="card-header">
      <div class="card-label">yt-dlp</div>
      <div class="dot ${info.ytdlp_exists ? 'dot-green' : 'dot-red'}"></div>
    </div>
    <div class="card-value">${info.current || 'Not installed'}</div>
    ${info.latest ? `<div class="card-detail">Latest: ${info.latest}</div>` : ''}
    <div style="margin-top: 12px; display: flex; gap: 8px;">
      <button class="btn btn-primary" id="ytdlp-check">Check for Update</button>
      <button class="btn btn-primary" id="ytdlp-download" style="display:none;">Download Update</button>
    </div>
    <div id="ytdlp-msg" style="margin-top: 8px;"></div>
  `;

  document.getElementById('ytdlp-check').addEventListener('click', async () => {
    const msg = document.getElementById('ytdlp-msg');
    msg.innerHTML = '<span style="color: var(--text-secondary);">Checking...</span>';
    try {
      const result = await invoke('check_for_update', { ytdlpLocation: config.ytdlp_location });
      renderYtdlpCard(result);
      if (result.update_available) {
        document.getElementById('ytdlp-download').style.display = '';
        msg.innerHTML = `<span class="msg-success">Update available: ${result.latest}</span>`;
      } else {
        msg.innerHTML = '<span class="msg-success">Already up to date</span>';
      }
    } catch (e) {
      msg.innerHTML = `<span class="msg-error">${e}</span>`;
    }
  });

  const dlBtn = document.getElementById('ytdlp-download');
  if (dlBtn) {
    dlBtn.addEventListener('click', async () => {
      const msg = document.getElementById('ytdlp-msg');
      msg.innerHTML = '<span style="color: var(--text-secondary);">Downloading...</span>';
      try {
        const version = await invoke('download_ytdlp', { ytdlpLocation: config.ytdlp_location });
        msg.innerHTML = `<span class="msg-success">Updated to ${version}</span>`;
        refreshUpdates();
      } catch (e) {
        msg.innerHTML = `<span class="msg-error">${e}</span>`;
      }
    });
  }
}

function renderFfmpegCard(info) {
  document.getElementById('ffmpeg-card').innerHTML = `
    <div class="card-header">
      <div class="card-label">ffmpeg</div>
      <div class="dot ${info.ffmpeg_exists ? 'dot-green' : 'dot-red'}"></div>
    </div>
    <div class="card-value">${info.ffmpeg_exists ? 'Installed' : 'Not found'}</div>
    <div class="card-detail">${config.ffmpeg_location}</div>
  `;
}

// ================================================================
// Cookies
// ================================================================

async function refreshCookies() {
  try {
    const status = await invoke('check_cookies');
    document.getElementById('cookies-status-card').innerHTML = `
      <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 4px;">
        <div class="dot ${status.exists ? 'dot-green' : 'dot-yellow'}"></div>
        <span style="font-size: 14px; font-weight: 500;">${status.exists ? 'cookies.txt found' : 'cookies.txt not found'}</span>
      </div>
      ${status.age_description ? `<div class="card-detail">Last updated: ${status.age_description}</div>` : ''}
    `;
  } catch {}

  try {
    const browsers = await invoke('get_browsers');
    document.getElementById('cookies-extract-card').innerHTML = `
      <div style="margin-bottom: 12px; font-size: 12px; color: var(--text-label);">Extract cookies from browser</div>
      <div style="display: flex; gap: 8px; align-items: center;">
        <select class="form-select" id="cookie-browser" style="width: 160px;">
          ${browsers.map(b => `<option value="${b}" ${b === config.cookies_browser ? 'selected' : ''}>${b}</option>`).join('')}
        </select>
        <button class="btn btn-primary" id="cookie-extract">Extract Cookies</button>
        <button class="btn btn-secondary" id="cookie-refresh">Refresh Status</button>
      </div>
    `;

    document.getElementById('cookie-extract').addEventListener('click', async () => {
      const msg = document.getElementById('cookies-msg');
      const browser = document.getElementById('cookie-browser').value;
      msg.innerHTML = '<span style="color: var(--text-secondary);">Extracting...</span>';
      try {
        const result = await invoke('extract_cookies', { ytdlpLocation: config.ytdlp_location, browser });
        msg.innerHTML = `<span class="msg-success">${result}</span>`;
        refreshCookies();
      } catch (e) {
        msg.innerHTML = `<span class="msg-error">${e}</span>`;
      }
    });

    document.getElementById('cookie-refresh').addEventListener('click', refreshCookies);
  } catch {}
}

// ================================================================
// Helpers
// ================================================================

function formatSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// ================================================================
// Boot
// ================================================================

init();
