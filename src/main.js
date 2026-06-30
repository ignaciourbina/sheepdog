import { buildExportFilename, downloadTextFile, exportRowsToText } from "./download-utils.js";
import { formatBytes } from "./format-utils.js";

const { invoke } = window.__TAURI__.core;
const { Channel } = window.__TAURI__.core;

// ── State ──────────────────────────────────────────────────────────

let allVenvs = [];
let currentSort = { col: "project", dir: "asc" };
let activeFilter = null; // python version filter
let expandedVenvId = null;
let focusedRowIndex = -1;
let lastSearchQuery = null; // preserve search context for back navigation
let compareSelection = null; // { id, name } of first venv selected for comparison

// ── DOM refs ───────────────────────────────────────────────────────

const scanBtn = document.getElementById("scan-btn");
const searchInput = document.getElementById("search-input");
const scanStatus = document.getElementById("scan-status");
const statsBar = document.getElementById("stats-bar");
const filterBar = document.getElementById("filter-bar");
const breadcrumb = document.getElementById("breadcrumb");
const tableContainer = document.getElementById("table-container");
const progressOverlay = document.getElementById("progress-overlay");
const progressText = document.getElementById("progress-text");
const progressBar = document.getElementById("progress-bar");
const progressDetail = document.getElementById("progress-detail");

// ── Helpers ────────────────────────────────────────────────────────

function shortenPath(p) {
  const home = "/home/ignacio";
  if (p.startsWith(home)) return "~" + p.slice(home.length);
  return p;
}

function extractProjectName(projectPath) {
  const parts = projectPath.replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || projectPath;
}

function escapeHtml(s) {
  const div = document.createElement("div");
  div.textContent = s;
  return div.innerHTML;
}

function relativeTime(dateStr) {
  if (!dateStr) return "";
  const date = new Date(dateStr.replace(" ", "T"));
  if (isNaN(date)) return dateStr;
  const now = new Date();
  const diffMs = now - date;
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);
  const diffMonths = Math.floor(diffDays / 30);
  const diffYears = Math.floor(diffDays / 365);

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 30) return `${diffDays}d ago`;
  if (diffMonths < 12) return `${diffMonths}mo ago`;
  return `${diffYears}y ago`;
}

function pyVersionClass(version) {
  if (!version) return "tag-py-other";
  if (version.startsWith("3.14")) return "tag-py314";
  if (version.startsWith("3.13")) return "tag-py313";
  if (version.startsWith("3.12")) return "tag-py312";
  if (version.startsWith("3.11")) return "tag-py311";
  return "tag-py-other";
}

function configFileShortName(filename) {
  const map = {
    "requirements.txt": "req",
    "requirements-dev.txt": "req-dev",
    "requirements_dev.txt": "req-dev",
    "pyproject.toml": "pyproject",
    "setup.py": "setup.py",
    "setup.cfg": "setup.cfg",
    "Pipfile": "Pipfile",
    "environment.yml": "env.yml",
  };
  return map[filename] || filename;
}

function renderConfigBadges(configFilesStr, projectPath) {
  if (!configFilesStr) return '<span style="color:var(--text-muted)">—</span>';
  const files = configFilesStr.split(",").filter(Boolean);
  if (files.length === 0) return '<span style="color:var(--text-muted)">—</span>';

  return files
    .map(
      (f) =>
        `<span class="config-badge" data-file="${escapeHtml(f)}" data-project="${escapeHtml(projectPath)}" title="${escapeHtml(f)}">${configFileShortName(f)}</span>`
    )
    .join(" ");
}

function pyMinor(version) {
  if (!version) return "other";
  const parts = version.split(".");
  if (parts.length >= 2) return parts[0] + "." + parts[1];
  return version;
}

// ── Sorting ────────────────────────────────────────────────────────

function sortVenvs(venvs, col, dir) {
  const sorted = [...venvs];
  const mult = dir === "asc" ? 1 : -1;

  sorted.sort((a, b) => {
    switch (col) {
      case "project":
        return mult * extractProjectName(a.project_path).localeCompare(extractProjectName(b.project_path));
      case "python":
        return mult * a.python_version.localeCompare(b.python_version);
      case "packages":
        return mult * (a.package_count - b.package_count);
      case "size":
        return mult * (a.size_bytes - b.size_bytes);
      case "modified":
        return mult * (a.last_modified || "").localeCompare(b.last_modified || "");
      default:
        return 0;
    }
  });
  return sorted;
}

// ── Filter ─────────────────────────────────────────────────────────

function getFilteredVenvs() {
  let venvs = allVenvs;
  if (activeFilter) {
    venvs = venvs.filter((v) => pyMinor(v.python_version) === activeFilter);
  }
  return sortVenvs(venvs, currentSort.col, currentSort.dir);
}

function renderFilterBar() {
  const counts = {};
  for (const v of allVenvs) {
    const minor = pyMinor(v.python_version);
    counts[minor] = (counts[minor] || 0) + 1;
  }

  const versions = Object.keys(counts).sort().reverse();
  if (versions.length <= 1) {
    filterBar.classList.add("hidden");
    return;
  }

  filterBar.classList.remove("hidden");
  let html = '<span class="filter-label">Python</span>';

  for (const ver of versions) {
    const isActive = activeFilter === ver;
    html += `<button class="filter-chip ${isActive ? "active" : ""}" data-version="${ver}">
      ${ver} <span class="chip-count">(${counts[ver]})</span>
    </button>`;
  }

  if (activeFilter) {
    html += `<button class="filter-chip-clear" id="clear-filter">Clear</button>`;
  }

  filterBar.innerHTML = html;

  filterBar.querySelectorAll(".filter-chip").forEach((chip) => {
    chip.addEventListener("click", () => {
      const ver = chip.dataset.version;
      activeFilter = activeFilter === ver ? null : ver;
      renderFilterBar();
      renderVenvTable(getFilteredVenvs());
    });
  });

  const clearBtn = document.getElementById("clear-filter");
  if (clearBtn) {
    clearBtn.addEventListener("click", () => {
      activeFilter = null;
      renderFilterBar();
      renderVenvTable(getFilteredVenvs());
    });
  }
}

// ── Stats ──────────────────────────────────────────────────────────

function renderStats(status) {
  if (!status.has_data) {
    statsBar.classList.add("hidden");
    return;
  }

  statsBar.classList.remove("hidden");
  const pyVersions = new Set(allVenvs.map((v) => pyMinor(v.python_version)));

  statsBar.innerHTML = `
    <div class="stat"><span class="stat-value">${status.venv_count}</span> venvs</div>
    <div class="stat-divider"></div>
    <div class="stat"><span class="stat-value">${status.package_count.toLocaleString()}</span> packages</div>
    <div class="stat-divider"></div>
    <div class="stat"><span class="stat-value">${pyVersions.size}</span> Python versions</div>
    <div class="stat-divider"></div>
    <div class="stat"><span class="stat-value">${formatBytes(status.total_size_bytes)}</span> indexed size</div>
    <div class="stats-spacer"></div>
    <div class="export-controls">
      <select id="export-format" class="export-format" title="Export format">
        <option value="csv">CSV</option>
        <option value="json">JSON</option>
      </select>
      <button id="export-data-btn" class="btn-ghost export-btn" title="Download consolidated table">Export</button>
    </div>
  `;

  document.getElementById("export-data-btn")?.addEventListener("click", exportConsolidatedData);
}

// ── Export ─────────────────────────────────────────────────────────

async function exportConsolidatedData() {
  const btn = document.getElementById("export-data-btn");
  const format = document.getElementById("export-format")?.value || "csv";
  if (!btn) return;

  const originalText = btn.textContent;
  btn.disabled = true;
  btn.textContent = "Exporting...";

  try {
    // The backend owns the data shape; frontend utilities only serialize and
    // trigger the download so CSV/JSON behavior stays independent of Tauri IPC.
    const rows = await invoke("get_export_rows");
    const filename = buildExportFilename(format);
    const { content, mimeType } = exportRowsToText(rows, format);
    downloadTextFile(filename, content, mimeType);

    btn.textContent = "Exported";
    setTimeout(() => {
      btn.textContent = originalText;
    }, 1200);
  } catch (e) {
    console.error("Export failed:", e);
    btn.textContent = "Failed";
    setTimeout(() => {
      btn.textContent = originalText;
    }, 1800);
  } finally {
    btn.disabled = false;
  }
}

// ── Breadcrumb ─────────────────────────────────────────────────────

function setBreadcrumb(items) {
  if (!items || items.length === 0) {
    breadcrumb.classList.add("hidden");
    return;
  }

  breadcrumb.classList.remove("hidden");
  let html = "";

  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (i > 0) html += '<span class="breadcrumb-sep">&#x25B8;</span>';

    if (i === items.length - 1) {
      html += `<span class="breadcrumb-current">${escapeHtml(item.label)}</span>`;
    } else {
      html += `<button class="breadcrumb-item" data-action="${item.action}">${escapeHtml(item.label)}</button>`;
    }
  }

  breadcrumb.innerHTML = html;

  breadcrumb.querySelectorAll("[data-action]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const action = btn.dataset.action;
      if (action === "dashboard") {
        loadDashboard();
      } else if (action.startsWith("search:")) {
        const query = action.slice("search:".length);
        searchInput.value = query;
        doSearch(query);
      } else if (action.startsWith("venv:")) {
        const parts = action.split(":");
        const id = parseInt(parts[1]);
        const searchCtx = parts[2] || null;
        if (searchCtx) {
          showVenvDetail(id, searchCtx);
        } else {
          expandAndShowVenv(id);
        }
      }
    });
  });
}

// ── Scan ───────────────────────────────────────────────────────────

scanBtn.addEventListener("click", async () => {
  scanBtn.disabled = true;
  progressOverlay.classList.remove("hidden");
  progressText.textContent = "Scanning filesystem...";
  progressBar.style.width = "0%";
  progressDetail.textContent = "";

  try {
    const onProgress = new Channel();
    onProgress.onmessage = (msg) => {
      if (msg.phase === "scanning") {
        progressText.textContent = "Scanning filesystem...";
        progressDetail.textContent = shortenPath(msg.current_path);
      } else if (msg.phase === "indexing") {
        const pct = msg.total_found > 0 ? (msg.current / msg.total_found) * 100 : 0;
        progressText.textContent = `Indexing ${msg.current} / ${msg.total_found}`;
        progressBar.style.width = pct + "%";
        progressDetail.textContent = shortenPath(msg.current_path);
      }
    };

    const count = await invoke("scan_venvs", { onProgress });
    progressText.textContent = `Found ${count} virtual environments`;
    progressBar.style.width = "100%";

    setTimeout(() => {
      progressOverlay.classList.add("hidden");
      loadDashboard();
    }, 600);
  } catch (e) {
    progressText.textContent = "Scan failed: " + e;
    setTimeout(() => progressOverlay.classList.add("hidden"), 2000);
  } finally {
    scanBtn.disabled = false;
  }
});

// ── Search ─────────────────────────────────────────────────────────

let searchTimeout = null;
searchInput.addEventListener("input", () => {
  clearTimeout(searchTimeout);
  const q = searchInput.value.trim();
  if (q.length === 0) {
    loadDashboard();
    return;
  }
  searchTimeout = setTimeout(() => doSearch(q), 200);
});

// Focus search on "/" key
document.addEventListener("keydown", (e) => {
  if (e.key === "/" && document.activeElement !== searchInput) {
    e.preventDefault();
    searchInput.focus();
  }
  if (e.key === "Escape") {
    const fileViewer = document.getElementById("file-viewer-overlay");
    if (!fileViewer.classList.contains("hidden")) {
      closeFileViewer();
    } else if (document.activeElement === searchInput) {
      searchInput.value = "";
      searchInput.blur();
      loadDashboard();
    }
  }
});

async function doSearch(query) {
  try {
    const results = await invoke("search_packages", { query });
    lastSearchQuery = query;

    setBreadcrumb([
      { label: "All Venvs", action: "dashboard" },
      { label: `Search: "${query}"` },
    ]);
    filterBar.classList.add("hidden");

    renderSearchResults(results, query);
  } catch (e) {
    console.error("Search failed:", e);
  }
}

function highlightMatch(text, query) {
  const idx = text.toLowerCase().indexOf(query.toLowerCase());
  if (idx === -1) return escapeHtml(text);
  const before = escapeHtml(text.slice(0, idx));
  const match = escapeHtml(text.slice(idx, idx + query.length));
  const after = escapeHtml(text.slice(idx + query.length));
  return `${before}<mark>${match}</mark>${after}`;
}

function renderSearchResults(results, query) {
  if (results.length === 0) {
    tableContainer.innerHTML = `
      <div class="empty-state">
        <div class="empty-state-title">No packages matching "${escapeHtml(query)}"</div>
        <div class="empty-state-hint">Try a different search term</div>
      </div>`;
    return;
  }

  // Group by package name for cleaner display
  const grouped = {};
  for (const r of results) {
    if (!grouped[r.package_name]) grouped[r.package_name] = [];
    grouped[r.package_name].push(r);
  }

  let html = `<div class="section-header">
    <span class="section-title">${results.length} results across ${Object.keys(grouped).length} packages</span>
  </div>`;

  html += '<div class="table-scroll"><table><thead><tr>';
  html += "<th>Package</th><th>Version</th><th>Project</th><th>Python</th>";
  html += "</tr></thead><tbody>";

  for (const r of results) {
    const venvLabel = r.venv_name ? `<span class="venv-label">${escapeHtml(r.venv_name)}</span>` : "";
    html += `<tr data-venv-id="${r.venv_id}">
      <td class="mono">${highlightMatch(r.package_name, query)}</td>
      <td><span class="tag tag-version">${escapeHtml(r.package_version)}</span></td>
      <td>
        <div class="project-info">
          <div class="project-name">${escapeHtml(extractProjectName(r.project_path))} ${venvLabel}</div>
          <div class="project-path">${escapeHtml(shortenPath(r.project_path))}</div>
        </div>
      </td>
      <td><span class="tag ${pyVersionClass(r.python_version)}">${escapeHtml(r.python_version)}</span></td>
    </tr>`;
  }

  html += "</tbody></table></div>";
  tableContainer.innerHTML = html;

  tableContainer.querySelectorAll("tr[data-venv-id]").forEach((row) => {
    row.addEventListener("click", () => {
      const venvId = parseInt(row.dataset.venvId);
      showVenvDetail(venvId, query);
    });
  });

  attachContextMenu();
}

// ── Dashboard ──────────────────────────────────────────────────────

async function loadDashboard() {
  expandedVenvId = null;
  activeFilter = null;
  setBreadcrumb(null);

  try {
    const status = await invoke("get_scan_status");

    if (status.last_scan) {
      scanStatus.textContent = relativeTime(status.last_scan);
    } else {
      scanStatus.textContent = "";
    }

    if (!status.has_data) {
      statsBar.classList.add("hidden");
      filterBar.classList.add("hidden");
      tableContainer.innerHTML = `
        <div class="empty-state">
          <div class="empty-state-icon">&#x1F43E;</div>
          <div class="empty-state-title">No virtual environments indexed yet</div>
          <div class="empty-state-hint">Click <strong>Scan</strong> to sniff out your Python venvs</div>
        </div>`;
      return;
    }

    allVenvs = await invoke("get_all_venvs");
    renderStats(status);
    renderFilterBar();
    renderVenvTable(getFilteredVenvs());
  } catch (e) {
    console.error("Failed to load dashboard:", e);
  }
}

function renderVenvTable(venvs) {
  const maxPkgs = Math.max(...venvs.map((v) => v.package_count), 1);

  let html = '<div class="table-scroll"><table><thead><tr>';

  const cols = [
    { key: "project", label: "Project" },
    { key: "python", label: "Python" },
    { key: "packages", label: "Packages" },
    { key: "size", label: "Size" },
    { key: "config", label: "Config" },
    { key: "modified", label: "Modified" },
  ];

  for (const col of cols) {
    const isSorted = currentSort.col === col.key;
    const arrow = isSorted ? (currentSort.dir === "asc" ? "&#x25B2;" : "&#x25BC;") : "&#x25B2;";
    html += `<th class="${isSorted ? "sorted" : ""}" data-sort="${col.key}">
      ${col.label} <span class="sort-arrow">${arrow}</span>
    </th>`;
  }

  html += "</tr></thead><tbody>";

  // Detect projects with multiple venvs
  const projectVenvCounts = {};
  for (const v of venvs) {
    projectVenvCounts[v.project_path] = (projectVenvCounts[v.project_path] || 0) + 1;
  }

  for (const v of venvs) {
    const isExpanded = expandedVenvId === v.id;
    const pct = Math.max(2, (v.package_count / maxPkgs) * 100);
    const projName = extractProjectName(v.project_path);
    const showVenvName = projectVenvCounts[v.project_path] > 1 || v.venv_name !== "venv";
    const venvLabel = showVenvName ? `<span class="venv-label">${escapeHtml(v.venv_name)}</span>` : "";

    html += `<tr data-venv-id="${v.id}" class="${isExpanded ? "expanded" : ""}" tabindex="0">
      <td>
        <div class="cell-project">
          <span class="expand-chevron">&#x25B8;</span>
          <div class="project-info">
            <div class="project-name">${escapeHtml(projName)} ${venvLabel}</div>
            <div class="project-path">${escapeHtml(shortenPath(v.project_path))}</div>
          </div>
        </div>
      </td>
      <td><span class="tag ${pyVersionClass(v.python_version)}">${escapeHtml(v.python_version)}</span></td>
      <td>
        <div class="pkg-count-cell">
          <div class="pkg-count-bar"><div class="pkg-count-fill" style="width:${pct}%"></div></div>
          <span class="pkg-count-num">${v.package_count}</span>
        </div>
      </td>
      <td><span class="mono">${formatBytes(v.size_bytes)}</span></td>
      <td>${renderConfigBadges(v.config_files, v.project_path)}</td>
      <td><span class="time-relative" title="${escapeHtml(v.last_modified)}">${relativeTime(v.last_modified)}</span></td>
    </tr>`;

    if (isExpanded) {
      html += `<tr class="expanded-content" data-expanded-for="${v.id}"><td colspan="6">
        <div class="expanded-inner" id="expanded-${v.id}">
          <div style="color:var(--text-muted);font-size:11px">Loading packages...</div>
        </div>
      </td></tr>`;
    }
  }

  html += "</tbody></table></div>";
  tableContainer.innerHTML = html;

  // Sort click handlers
  tableContainer.querySelectorAll("th[data-sort]").forEach((th) => {
    th.addEventListener("click", () => {
      const col = th.dataset.sort;
      if (currentSort.col === col) {
        currentSort.dir = currentSort.dir === "asc" ? "desc" : "asc";
      } else {
        currentSort = { col, dir: "asc" };
      }
      renderVenvTable(getFilteredVenvs());
    });
  });

  // Row click handlers (expand inline)
  tableContainer.querySelectorAll("tr[data-venv-id]").forEach((row) => {
    row.addEventListener("click", () => {
      const venvId = parseInt(row.dataset.venvId);
      toggleExpand(venvId);
    });
  });

  // Load expanded content if any
  if (expandedVenvId !== null) {
    loadExpandedPackages(expandedVenvId);
  }

  // Keyboard navigation
  setupKeyboardNav();

  // Context menu & compare highlight
  attachContextMenu();
  highlightCompareRow();

  // Config file click handlers
  attachConfigBadgeClicks();
}

// ── Inline expansion ───────────────────────────────────────────────

async function toggleExpand(venvId) {
  if (expandedVenvId === venvId) {
    expandedVenvId = null;
  } else {
    expandedVenvId = venvId;
  }
  renderVenvTable(getFilteredVenvs());
}

async function expandAndShowVenv(venvId) {
  searchInput.value = "";
  expandedVenvId = venvId;
  setBreadcrumb(null);
  filterBar.classList.remove("hidden");
  await loadDashboard();
  expandedVenvId = venvId;
  renderVenvTable(getFilteredVenvs());

  // Scroll to expanded row
  setTimeout(() => {
    const row = tableContainer.querySelector(`tr[data-venv-id="${venvId}"]`);
    if (row) row.scrollIntoView({ behavior: "smooth", block: "start" });
  }, 100);
}

async function loadExpandedPackages(venvId) {
  const container = document.getElementById(`expanded-${venvId}`);
  if (!container) return;

  try {
    const packages = await invoke("get_venv_packages", { venvId });

    let html = `<div class="expanded-toolbar">
      <span style="font-size:11px;color:var(--text-dim)">${packages.length} packages</span>
      <button class="btn-ghost btn-orange" id="check-outdated-${venvId}">Check Outdated</button>
      <button class="btn-ghost" id="view-detail-${venvId}">Full View</button>
    </div>`;

    html += '<div class="expanded-packages">';
    for (const p of packages) {
      html += `<div class="pkg-item" data-pkg-id="${p.id}" data-pkg-name="${escapeHtml(p.name)}" data-venv-id="${venvId}">
        <span class="pkg-item-name">${escapeHtml(p.name)}</span>
        <span class="pkg-item-version">${escapeHtml(p.version)}</span>
      </div>`;
    }
    html += "</div>";

    container.innerHTML = html;

    // Package click → show deps
    container.querySelectorAll(".pkg-item").forEach((item) => {
      item.addEventListener("click", (e) => {
        e.stopPropagation();
        const pkgId = parseInt(item.dataset.pkgId);
        const pkgName = item.dataset.pkgName;
        showDependencies(pkgId, pkgName, venvId);
      });
    });

    // Check outdated
    const outdatedBtn = document.getElementById(`check-outdated-${venvId}`);
    if (outdatedBtn) {
      outdatedBtn.addEventListener("click", async (e) => {
        e.stopPropagation();
        outdatedBtn.disabled = true;
        outdatedBtn.textContent = "Checking...";

        try {
          const results = await invoke("check_outdated", { venvId });
          for (const r of results) {
            const items = container.querySelectorAll(`.pkg-item[data-pkg-name="${r.package_name}"]`);
            items.forEach((item) => {
              const verSpan = item.querySelector(".pkg-item-version");
              if (!verSpan) return;
              if (r.error) {
                // leave as-is
              } else if (r.is_outdated) {
                verSpan.innerHTML = `${escapeHtml(r.installed_version)} <span class="tag tag-outdated" style="margin-left:4px">${escapeHtml(r.latest_version)}</span>`;
              } else {
                verSpan.innerHTML = `${escapeHtml(r.installed_version)} <span class="tag tag-current" style="margin-left:4px">OK</span>`;
              }
            });
          }
          outdatedBtn.textContent = "Done";
        } catch (err) {
          outdatedBtn.textContent = "Failed";
        }
      });
    }

    // Full view
    const fullBtn = document.getElementById(`view-detail-${venvId}`);
    if (fullBtn) {
      fullBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        showVenvDetail(venvId);
      });
    }
  } catch (e) {
    container.innerHTML = `<div style="color:var(--red);font-size:11px">Failed to load packages</div>`;
  }
}

// ── Venv Detail (full page) ────────────────────────────────────────

async function showVenvDetail(venvId, fromSearch) {
  filterBar.classList.add("hidden");

  try {
    const packages = await invoke("get_venv_packages", { venvId });
    const venv = allVenvs.find((v) => v.id === venvId);
    const projName = venv ? extractProjectName(venv.project_path) : `Venv #${venvId}`;

    const crumbs = [{ label: "All Venvs", action: "dashboard" }];
    if (fromSearch) {
      crumbs.push({ label: `Search: "${fromSearch}"`, action: `search:${fromSearch}` });
    }
    crumbs.push({ label: projName });
    setBreadcrumb(crumbs);

    let html = `<div class="section-header">
      <span class="section-title">${packages.length} packages</span>
      <button class="btn-ghost btn-orange" id="check-outdated-full">Check Outdated</button>
    </div>`;

    html += '<div class="table-scroll"><table><thead><tr>';
    html += "<th>Package</th><th>Version</th><th>Summary</th><th>Status</th>";
    html += "</tr></thead><tbody>";

    for (const p of packages) {
      html += `<tr class="clickable" data-pkg-id="${p.id}" data-pkg-name="${escapeHtml(p.name)}" data-venv-id="${venvId}">
        <td class="mono">${escapeHtml(p.name)}</td>
        <td><span class="tag tag-version">${escapeHtml(p.version)}</span></td>
        <td style="color:var(--text-muted);max-width:350px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:11px">${escapeHtml(p.summary || "")}</td>
        <td class="outdated-cell" data-pkg-name="${escapeHtml(p.name)}"><span style="color:var(--text-muted)">—</span></td>
      </tr>`;
    }

    html += "</tbody></table></div>";
    tableContainer.innerHTML = html;

    // Click package → deps
    tableContainer.querySelectorAll("tr[data-pkg-id]").forEach((row) => {
      row.addEventListener("click", () => {
        const pkgId = parseInt(row.dataset.pkgId);
        const pkgName = row.dataset.pkgName;
        showDependencies(pkgId, pkgName, venvId, fromSearch);
      });
    });

    // Check outdated
    document.getElementById("check-outdated-full").addEventListener("click", async (e) => {
      const btn = e.target;
      btn.disabled = true;
      btn.textContent = "Checking...";

      try {
        const results = await invoke("check_outdated", { venvId });
        for (const r of results) {
          const cell = tableContainer.querySelector(`td.outdated-cell[data-pkg-name="${r.package_name}"]`);
          if (!cell) continue;
          if (r.error) {
            cell.innerHTML = `<span class="tag tag-error">?</span>`;
          } else if (r.is_outdated) {
            cell.innerHTML = `<span class="tag tag-outdated">${escapeHtml(r.latest_version)}</span>`;
          } else {
            cell.innerHTML = `<span class="tag tag-current">OK</span>`;
          }
        }
        btn.textContent = "Done";
      } catch (err) {
        btn.textContent = "Failed";
      }
    });
  } catch (e) {
    console.error("Failed to load venv detail:", e);
  }
}

// ── Dependencies ───────────────────────────────────────────────────

async function showDependencies(packageId, packageName, venvId, fromSearch) {
  const venv = allVenvs.find((v) => v.id === venvId);
  const projName = venv ? extractProjectName(venv.project_path) : "Venv";

  const crumbs = [{ label: "All Venvs", action: "dashboard" }];
  if (fromSearch) {
    crumbs.push({ label: `Search: "${fromSearch}"`, action: `search:${fromSearch}` });
  }
  crumbs.push({ label: projName, action: `venv:${venvId}:${fromSearch || ""}` });
  crumbs.push({ label: packageName });
  setBreadcrumb(crumbs);

  try {
    const deps = await invoke("get_package_dependencies", { packageId });

    if (deps.length === 0) {
      tableContainer.innerHTML = `
        <div class="deps-container">
          <div class="empty-state" style="padding:40px">
            <div class="empty-state-title">${escapeHtml(packageName)} has no dependencies</div>
          </div>
        </div>`;
      return;
    }

    const core = deps.filter((d) => !d.extra);
    const extras = deps.filter((d) => d.extra);

    let html = `<div class="deps-container">
      <div class="section-header" style="padding-left:0">
        <span class="section-title">${deps.length} dependencies</span>
      </div>`;

    if (core.length > 0) {
      html += '<div class="dep-grid">';
      for (const d of core) {
        html += `<div class="dep-item">
          <span class="dep-name">${escapeHtml(d.dep_name)}</span>
          ${d.version_spec ? `<span class="dep-spec">${escapeHtml(d.version_spec)}</span>` : ""}
        </div>`;
      }
      html += "</div>";
    }

    if (extras.length > 0) {
      const groups = {};
      for (const d of extras) {
        const key = d.extra || "other";
        if (!groups[key]) groups[key] = [];
        groups[key].push(d);
      }

      for (const [extra, groupDeps] of Object.entries(groups)) {
        html += `<div class="dep-extra-group">
          <div class="dep-extra-label">[${escapeHtml(extra)}]</div>
          <div class="dep-grid">`;
        for (const d of groupDeps) {
          html += `<div class="dep-item">
            <span class="dep-name">${escapeHtml(d.dep_name)}</span>
            ${d.version_spec ? `<span class="dep-spec">${escapeHtml(d.version_spec)}</span>` : ""}
          </div>`;
        }
        html += "</div></div>";
      }
    }

    html += "</div>";
    tableContainer.innerHTML = html;
  } catch (e) {
    console.error("Failed to load deps:", e);
  }
}

// ── Keyboard navigation ────────────────────────────────────────────

function setupKeyboardNav() {
  const rows = tableContainer.querySelectorAll("tbody tr[data-venv-id]");
  if (rows.length === 0) return;

  document.addEventListener("keydown", handleTableKeyboard);
}

function handleTableKeyboard(e) {
  const rows = Array.from(tableContainer.querySelectorAll("tbody tr[data-venv-id]"));
  if (rows.length === 0) return;
  if (document.activeElement === searchInput) return;

  if (e.key === "ArrowDown" || e.key === "j") {
    e.preventDefault();
    focusedRowIndex = Math.min(focusedRowIndex + 1, rows.length - 1);
    rows[focusedRowIndex]?.focus();
  } else if (e.key === "ArrowUp" || e.key === "k") {
    e.preventDefault();
    focusedRowIndex = Math.max(focusedRowIndex - 1, 0);
    rows[focusedRowIndex]?.focus();
  } else if (e.key === "Enter" || e.key === " ") {
    if (focusedRowIndex >= 0 && focusedRowIndex < rows.length) {
      e.preventDefault();
      const venvId = parseInt(rows[focusedRowIndex].dataset.venvId);
      toggleExpand(venvId);
    }
  }
}

// ── Config file viewer ─────────────────────────────────────────────

function attachConfigBadgeClicks() {
  tableContainer.querySelectorAll(".config-badge").forEach((badge) => {
    badge.addEventListener("click", (e) => {
      e.stopPropagation();
      const filename = badge.dataset.file;
      const projectPath = badge.dataset.project;
      showFileViewer(projectPath, filename);
    });
  });
}

async function showFileViewer(projectPath, filename) {
  try {
    const content = await invoke("read_project_file", { projectPath, filename });
    const overlay = document.getElementById("file-viewer-overlay");
    const title = document.getElementById("file-viewer-title");
    const body = document.getElementById("file-viewer-body");
    const pathEl = document.getElementById("file-viewer-path");

    title.textContent = filename;
    pathEl.textContent = shortenPath(projectPath + "/" + filename);
    body.textContent = content;
    overlay.classList.remove("hidden");
  } catch (e) {
    console.error("Failed to read file:", e);
  }
}

function closeFileViewer() {
  document.getElementById("file-viewer-overlay").classList.add("hidden");
}
window.closeFileViewer = closeFileViewer;

// ── Context menu & Compare ──────────────────────────────────────────

const contextMenu = document.getElementById("context-menu");
const ctxSelectCompare = document.getElementById("ctx-select-compare");
const ctxCompareWith = document.getElementById("ctx-compare-with");
const ctxClearCompare = document.getElementById("ctx-clear-compare");
const compareBadge = document.getElementById("compare-badge");
let contextVenvId = null;

// Close context menu on any click or Escape
document.addEventListener("click", () => contextMenu.classList.add("hidden"));
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !contextMenu.classList.contains("hidden")) {
    contextMenu.classList.add("hidden");
  }
});

// Attach right-click handler (called after each table render)
function attachContextMenu() {
  tableContainer.querySelectorAll("tr[data-venv-id]").forEach((row) => {
    row.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      contextVenvId = parseInt(row.dataset.venvId);
      const venv = allVenvs.find((v) => v.id === contextVenvId);
      const name = venv ? extractProjectName(venv.project_path) : `#${contextVenvId}`;

      // Configure menu items
      if (compareSelection && compareSelection.id !== contextVenvId) {
        ctxSelectCompare.textContent = `Select "${name}"`;
        ctxCompareWith.textContent = `Compare with "${compareSelection.name}"`;
        ctxCompareWith.disabled = false;
        ctxCompareWith.classList.remove("hidden");
      } else {
        ctxSelectCompare.textContent = `Select for Compare`;
        ctxCompareWith.classList.add("hidden");
      }

      ctxClearCompare.style.display = compareSelection ? "" : "none";
      document.getElementById("ctx-divider-clear").style.display = compareSelection ? "" : "none";

      // Position menu
      const x = Math.min(e.clientX, window.innerWidth - 220);
      const y = Math.min(e.clientY, window.innerHeight - 120);
      contextMenu.style.left = x + "px";
      contextMenu.style.top = y + "px";
      contextMenu.classList.remove("hidden");
    });
  });
}

ctxSelectCompare.addEventListener("click", (e) => {
  e.stopPropagation();
  contextMenu.classList.add("hidden");
  if (contextVenvId === null) return;

  const venv = allVenvs.find((v) => v.id === contextVenvId);
  const name = venv ? extractProjectName(venv.project_path) : `#${contextVenvId}`;
  const venvName = venv?.venv_name || "";

  compareSelection = { id: contextVenvId, name, venvName };
  updateCompareBadge();
  highlightCompareRow();
});

ctxCompareWith.addEventListener("click", (e) => {
  e.stopPropagation();
  contextMenu.classList.add("hidden");
  if (!compareSelection || contextVenvId === null) return;

  const leftId = compareSelection.id;
  const rightId = contextVenvId;
  compareSelection = null;
  updateCompareBadge();
  showComparison(leftId, rightId);
});

ctxClearCompare.addEventListener("click", (e) => {
  e.stopPropagation();
  contextMenu.classList.add("hidden");
  compareSelection = null;
  updateCompareBadge();
  highlightCompareRow();
});

document.getElementById("ctx-copy-path").addEventListener("click", (e) => {
  e.stopPropagation();
  contextMenu.classList.add("hidden");
  if (contextVenvId === null) return;
  const venv = allVenvs.find((v) => v.id === contextVenvId);
  if (venv) navigator.clipboard.writeText(venv.project_path);
});

document.getElementById("ctx-open-vscode").addEventListener("click", (e) => {
  e.stopPropagation();
  contextMenu.classList.add("hidden");
  if (contextVenvId === null) return;
  const venv = allVenvs.find((v) => v.id === contextVenvId);
  if (venv) invoke("open_in_vscode", { path: venv.project_path });
});

function updateCompareBadge() {
  if (!compareSelection) {
    compareBadge.classList.add("hidden");
    return;
  }

  const label = compareSelection.venvName && compareSelection.venvName !== "venv"
    ? `${compareSelection.name} (${compareSelection.venvName})`
    : compareSelection.name;

  compareBadge.classList.remove("hidden");
  compareBadge.innerHTML = `
    <span class="compare-badge-hint">Comparing:</span>
    <span class="compare-badge-name">${escapeHtml(label)}</span>
    <span class="compare-badge-hint">— right-click another to compare</span>
    <button class="compare-badge-clear" title="Clear selection">&times;</button>
  `;

  compareBadge.querySelector(".compare-badge-clear").addEventListener("click", () => {
    compareSelection = null;
    updateCompareBadge();
    highlightCompareRow();
  });
}

function highlightCompareRow() {
  tableContainer.querySelectorAll("tr.compare-selected").forEach((r) => {
    r.classList.remove("compare-selected");
  });

  if (compareSelection) {
    const row = tableContainer.querySelector(`tr[data-venv-id="${compareSelection.id}"]`);
    if (row) row.classList.add("compare-selected");
  }
}

// ── Comparison view ────────────────────────────────────────────────

async function showComparison(leftId, rightId) {
  filterBar.classList.add("hidden");

  try {
    const [leftPkgs, rightPkgs] = await Promise.all([
      invoke("get_venv_packages", { venvId: leftId }),
      invoke("get_venv_packages", { venvId: rightId }),
    ]);

    const leftVenv = allVenvs.find((v) => v.id === leftId);
    const rightVenv = allVenvs.find((v) => v.id === rightId);
    const leftName = leftVenv ? extractProjectName(leftVenv.project_path) : `#${leftId}`;
    const rightName = rightVenv ? extractProjectName(rightVenv.project_path) : `#${rightId}`;
    const leftLabel = leftVenv?.venv_name && leftVenv.venv_name !== "venv" ? `${leftName} (${leftVenv.venv_name})` : leftName;
    const rightLabel = rightVenv?.venv_name && rightVenv.venv_name !== "venv" ? `${rightName} (${rightVenv.venv_name})` : rightName;

    setBreadcrumb([
      { label: "All Venvs", action: "dashboard" },
      { label: "Compare" },
    ]);

    // Build package maps
    const leftMap = {};
    for (const p of leftPkgs) leftMap[p.name.toLowerCase()] = p;
    const rightMap = {};
    for (const p of rightPkgs) rightMap[p.name.toLowerCase()] = p;

    // Union of all package names
    const allNames = new Set([...Object.keys(leftMap), ...Object.keys(rightMap)]);
    const sorted = [...allNames].sort();

    let same = 0, diff = 0, onlyLeft = 0, onlyRight = 0;

    let rows = "";
    for (const name of sorted) {
      const l = leftMap[name];
      const r = rightMap[name];

      let status, statusClass;
      if (l && r) {
        if (l.version === r.version) {
          status = "same"; statusClass = "compare-status-same"; same++;
        } else {
          status = "diff"; statusClass = "compare-status-diff"; diff++;
        }
      } else if (l) {
        status = "only-left"; statusClass = "compare-status-only-left"; onlyLeft++;
      } else {
        status = "only-right"; statusClass = "compare-status-only-right"; onlyRight++;
      }

      rows += `<div class="compare-row">
        <div class="compare-cell${!l ? " compare-cell-right" : ""}">
          ${l ? `<span class="compare-pkg-name">${escapeHtml(l.name)}</span>
                 <span class="compare-pkg-ver">${escapeHtml(l.version)}</span>` : ""}
        </div>
        <div class="compare-center"><span class="compare-status ${statusClass}"></span></div>
        <div class="compare-cell${!r ? "" : ""}">
          ${r ? `<span class="compare-pkg-name">${escapeHtml(r.name)}</span>
                 <span class="compare-pkg-ver">${escapeHtml(r.version)}</span>` : ""}
        </div>
      </div>`;
    }

    tableContainer.innerHTML = `<div class="compare-container">
      <div class="compare-summary">
        <span class="compare-summary-stat"><strong>${allNames.size}</strong> total packages</span>
        <span class="compare-summary-stat"><strong>${same}</strong> identical</span>
        <span class="compare-summary-stat"><strong>${diff}</strong> different versions</span>
        <span class="compare-summary-stat"><strong>${onlyLeft}</strong> only left</span>
        <span class="compare-summary-stat"><strong>${onlyRight}</strong> only right</span>
      </div>
      <div class="compare-legend">
        <div class="compare-legend-item"><span class="compare-status compare-status-same"></span> Same version</div>
        <div class="compare-legend-item"><span class="compare-status compare-status-diff"></span> Different version</div>
        <div class="compare-legend-item"><span class="compare-status compare-status-only-left"></span> Only in left</div>
        <div class="compare-legend-item"><span class="compare-status compare-status-only-right"></span> Only in right</div>
      </div>
      <div class="compare-header">
        <div class="compare-col-title">${escapeHtml(leftLabel)}<span class="compare-py">${escapeHtml(leftVenv?.python_version || "")}</span></div>
        <div class="compare-col-center">Status</div>
        <div class="compare-col-title">${escapeHtml(rightLabel)}<span class="compare-py">${escapeHtml(rightVenv?.python_version || "")}</span></div>
      </div>
      ${rows}
    </div>`;
  } catch (e) {
    console.error("Comparison failed:", e);
  }
}

// ── Settings / Themes ──────────────────────────────────────────────

const settingsBtn = document.getElementById("settings-btn");
const settingsPanel = document.getElementById("settings-panel");

settingsBtn.addEventListener("click", (e) => {
  e.stopPropagation();
  settingsPanel.classList.toggle("hidden");
  settingsBtn.classList.toggle("active");
});

document.addEventListener("click", (e) => {
  if (!settingsPanel.contains(e.target) && e.target !== settingsBtn) {
    settingsPanel.classList.add("hidden");
    settingsBtn.classList.remove("active");
  }
});

function setTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("sheepdog-theme", theme);
  document.querySelectorAll(".theme-option").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.theme === theme);
  });
}

document.querySelectorAll(".theme-option").forEach((btn) => {
  btn.addEventListener("click", () => setTheme(btn.dataset.theme));
});

function initTheme() {
  const saved = localStorage.getItem("sheepdog-theme") || "dark";
  setTheme(saved);
}

// ── Font settings ──────────────────────────────────────────────────

const UI_FONTS = {
  default: "",
  inter: '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif',
  roboto: '"Roboto", -apple-system, BlinkMacSystemFont, sans-serif',
  helvetica: '"Helvetica Neue", Helvetica, Arial, sans-serif',
  mono: '"JetBrains Mono", "Fira Code", "SF Mono", monospace',
};

const CODE_FONTS = {
  default: "",
  jetbrains: '"JetBrains Mono", monospace',
  firacode: '"Fira Code", monospace',
  cascadia: '"Cascadia Code", monospace',
  ubuntu: '"Ubuntu Mono", monospace',
};

const uiFontSelect = document.getElementById("settings-ui-font");
const codeFontSelect = document.getElementById("settings-code-font");
const fontSizeDisplay = document.getElementById("font-size-display");
const fontSizeDown = document.getElementById("font-size-down");
const fontSizeUp = document.getElementById("font-size-up");

function setUIFont(key) {
  const font = UI_FONTS[key];
  if (font) {
    document.documentElement.style.setProperty("--ui-font", font);
  } else {
    document.documentElement.style.removeProperty("--ui-font");
  }
  localStorage.setItem("sheepdog-ui-font", key);
  uiFontSelect.value = key;
}

function setCodeFont(key) {
  const font = CODE_FONTS[key];
  if (font) {
    document.documentElement.style.setProperty("--code-font", font);
  } else {
    document.documentElement.style.removeProperty("--code-font");
  }
  localStorage.setItem("sheepdog-code-font", key);
  codeFontSelect.value = key;
}

let currentFontSize = 13;

function setFontSize(size) {
  size = Math.max(10, Math.min(18, size));
  currentFontSize = size;
  document.documentElement.style.setProperty("--ui-font-size", size + "px");
  localStorage.setItem("sheepdog-font-size", size);
  fontSizeDisplay.textContent = size + "px";
}

uiFontSelect.addEventListener("change", () => setUIFont(uiFontSelect.value));
codeFontSelect.addEventListener("change", () => setCodeFont(codeFontSelect.value));
fontSizeDown.addEventListener("click", () => setFontSize(currentFontSize - 1));
fontSizeUp.addEventListener("click", () => setFontSize(currentFontSize + 1));

function initSettings() {
  initTheme();
  setUIFont(localStorage.getItem("sheepdog-ui-font") || "default");
  setCodeFont(localStorage.getItem("sheepdog-code-font") || "default");
  const savedSize = parseInt(localStorage.getItem("sheepdog-font-size")) || 13;
  setFontSize(savedSize);
}

// ── Init ───────────────────────────────────────────────────────────

window.addEventListener("DOMContentLoaded", () => {
  initSettings();
  loadDashboard();
});
