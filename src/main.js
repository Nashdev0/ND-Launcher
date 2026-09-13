const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
// dialog object removed as we will invoke plugin directly

let accountSelect;
let avatarImg;
let activeUsernameDisplay;
let addAccountBtn;
let deleteAccountBtn;
let currentActiveAccount = null;
let elybyAvatarCache = {};
let elybyAvatarPromises = {};
let elybyProfileCache = {};
let elybyProfilePromise = null;

let customVersionWrapper;
let customVersionDisplay;
let customVersionOptions;
let versionSelectHidden;

let customInstanceWrapper;
let customInstanceDisplay;
let customInstanceOptions;
let instanceSelectHidden;
let currentActiveInstance = null;

let customLoaderWrapper;
let customLoaderDisplay;
let customLoaderOptions;
let loaderSelectHidden;

let customRamDisplay;
let customRamOptions;
let ramSelect;
let javaAutoSelect;
let javaCustomInput;
let statusMsg;
let launchBtn;
let logBox;

async function fetchVersions() {
  try {
    let versions = await invoke("get_minecraft_versions");
    customVersionOptions.innerHTML = "";

    if (versions.length === 0) {
      customVersionDisplay.textContent = "No versions found";
      return;
    }

    let defaultSelected = versions.length > 0 ? versions[0].id : "";
    versionSelectHidden.value = defaultSelected;
    customVersionDisplay.textContent = defaultSelected;

    versions.forEach(v => {
      let div = document.createElement("div");
      div.className = "custom-option";
      div.textContent = `${v.id} (${v.time.split("T")[0]})`;
      div.addEventListener("click", () => {
        versionSelectHidden.value = v.id;
        customVersionDisplay.textContent = v.id;
        customVersionOptions.classList.remove("open");
      });
      customVersionOptions.appendChild(div);
    });
  } catch (e) {
    customVersionDisplay.textContent = "Error loading versions";
  }
}

async function fetchInstances() {
  try {
    let instances = await invoke("get_instances");
    let settings = await invoke("get_settings");

    let instancesGrid = document.querySelector("#instances-grid");
    if (instancesGrid) instancesGrid.innerHTML = "";
    if (customInstanceOptions) customInstanceOptions.innerHTML = "";
    currentActiveInstance = null;

    if (instances.length === 0) {
      if (customInstanceDisplay) customInstanceDisplay.textContent = "Belum ada instance";
      if (instanceSelectHidden) instanceSelectHidden.value = "";
      return;
    }

    let activeId = settings.active_instance_id || instances[0].id;

    instances.forEach(inst => {
      // 1. Render in bottom dock
      let optDiv = document.createElement("div");
      optDiv.className = "custom-option";
      optDiv.textContent = `${inst.name} (${inst.version})`;
      optDiv.addEventListener("click", async () => {
        await invoke("switch_instance", { instanceId: inst.id });
        await fetchInstances(); // Refresh UI
        customInstanceOptions.classList.remove("open");
      });
      if (customInstanceOptions) customInstanceOptions.appendChild(optDiv);

      if (inst.id === activeId) {
        currentActiveInstance = inst;
        if (customInstanceDisplay) customInstanceDisplay.textContent = `${inst.name} (${inst.version} ${inst.loader})`;
        if (instanceSelectHidden) instanceSelectHidden.value = inst.id;

        // Check if Modrinth tab should show warning
        let modWarning = document.querySelector("#modrinth-warning");
        if (modWarning) {
          if (inst.loader.toLowerCase() === "vanilla") {
            modWarning.style.display = "block";
          } else {
            modWarning.style.display = "none";
          }
        }
      }

      // 2. Render in Grid View
      if (instancesGrid) {
        let card = document.createElement("div");
        card.className = "news-card panel";
        card.innerHTML = `
          <div style="display: flex; justify-content: space-between; align-items: start;">
            <span class="tag" style="background: var(--accent); color: white;">${inst.version} ${inst.loader.toUpperCase()}</span>
            <button class="btn btn-danger" data-id="${inst.id}" data-name="${inst.name}" style="padding: 2px 6px; font-size: 10px; border:none; background: #ffebee; color: var(--danger);">HAPUS</button>
          </div>
          <h4 style="margin-top: 8px;">${inst.name}</h4>
          <p>Local Instance</p>
          <div style="display: flex; gap: 8px; margin-top: 12px;">
            <button class="btn btn-primary play-inst-btn" data-id="${inst.id}" style="flex: 1; ${inst.id === activeId ? 'background: var(--success);' : ''}">${inst.id === activeId ? 'SELECTED' : 'SELECT'}</button>
            <button class="btn manage-mod-btn" data-id="${inst.id}" data-name="${inst.name}" style="flex: 1;">Kelola Mod</button>
          </div>
        `;
        instancesGrid.appendChild(card);
      }
    });

    // Grid action listeners
    if (instancesGrid) {
      instancesGrid.querySelectorAll(".play-inst-btn").forEach(btn => {
        btn.addEventListener("click", async (e) => {
          let id = e.target.getAttribute("data-id");
          await invoke("switch_instance", { instanceId: id });
          await fetchInstances();
        });
      });
    }

    // Delete Instance Logic
    document.querySelectorAll(".btn-danger[data-id]").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        let id = e.target.getAttribute("data-id");
        if (confirm("Yakin ingin menghapus instance ini?")) {
          await invoke("delete_instance", { instanceId: id });
          await fetchInstances();
        }
      });
    });

    // Manage Mod Logic
    document.querySelectorAll(".manage-mod-btn").forEach(btn => {
      btn.addEventListener("click", (e) => {
        let id = e.target.getAttribute("data-id");
        let name = e.target.getAttribute("data-name");
        openModManager(id, name);
      });
    });

  } catch (e) {
    console.log("Failed to fetch instances", e);
    if (customInstanceDisplay) customInstanceDisplay.textContent = "Error loading instances";
  }
}

async function getCachedElybyAvatar(username) {
  if (elybyAvatarCache[username]) return elybyAvatarCache[username];
  if (elybyAvatarPromises[username]) return elybyAvatarPromises[username];
  const promise = (async () => {
    try {
      const url = await invoke("get_elyby_head_data_url", { username });
      elybyAvatarCache[username] = url;
      return url;
    } catch (e) {
      return `https://mc-heads.net/head/${username}/64`;
    } finally {
      delete elybyAvatarPromises[username];
    }
  })();
  elybyAvatarPromises[username] = promise;
  return promise;
}

async function getCachedElybyProfile(username) {
  if (elybyProfileCache[username]) return elybyProfileCache[username];
  if (elybyProfilePromise) return elybyProfilePromise;
  const promise = (async () => {
    try {
      const profile = await invoke("get_elyby_profile", { username });
      elybyProfileCache[username] = profile;
      return profile;
    } catch (e) {
      console.log("Ely.by profile not found for:", username, e);
      return null;
    } finally {
      elybyProfilePromise = null;
    }
  })();
  elybyProfilePromise = promise;
  return promise;
}

async function updateElybyProfileDisplay() {
  if (!currentActiveAccount) return;

  const loading = document.querySelector("#elyby-loading");
  const content = document.querySelector("#elyby-content");
  const offline = document.querySelector("#elyby-offline");

  if (!loading || !content || !offline) return;

  loading.style.display = "block";
  content.style.display = "none";
  offline.style.display = "none";

  const profile = await getCachedElybyProfile(currentActiveAccount.username);

  if (profile) {
    document.querySelector("#elyby-username").textContent = profile.name;
    document.querySelector("#elyby-uuid").textContent = profile.uuid.substring(0, 8) + "...";

    if (profile.has_custom_skin) {
      document.querySelector("#elyby-skin-status").textContent = "✅ Skin kustom aktif";
      document.querySelector("#elyby-skin-status").style.color = "var(--success)";
    } else {
      document.querySelector("#elyby-skin-status").textContent = "⚪ Skin default";
      document.querySelector("#elyby-skin-status").style.color = "var(--text-muted)";
    }

    const skinDataUrl = await getCachedElybyAvatar(currentActiveAccount.username);
    document.querySelector("#elyby-avatar").src = skinDataUrl;

    loading.style.display = "none";
    content.style.display = "block";
  } else {
    loading.style.display = "none";
    offline.style.display = "block";
  }
}

async function fetchAccounts() {
  try {
    let accounts = await invoke("get_accounts");
    let settings = await invoke("get_settings");
    accountSelect.innerHTML = "";
    currentActiveAccount = null;

    if (accounts.length === 0) {
      accountSelect.innerHTML = `<option value="">No accounts found</option>`;
      avatarImg.src = "https://mc-heads.net/head/Steve/64";
      activeUsernameDisplay.textContent = "Please add an account";
      return;
    }

    accountSelect.innerHTML = `<option value="">-- Pilih Akun --</option>`;
    accounts.forEach(acc => {
      let opt = document.createElement("option");
      opt.value = acc.id;
      opt.textContent = acc.username;
      if (settings.active_account_id === acc.id) {
        opt.selected = true;
        currentActiveAccount = acc;
        activeUsernameDisplay.textContent = acc.username;
      }
      accountSelect.appendChild(opt);
    });

    if (!currentActiveAccount && accounts.length > 0) {
      currentActiveAccount = accounts[0];
      accountSelect.value = currentActiveAccount.id;
      activeUsernameDisplay.textContent = currentActiveAccount.username;
      await invoke("switch_account", { accountId: currentActiveAccount.id });
    }

    // Render avatar — FIRE AND FORGET, jangan block fetchAccounts()
    if (currentActiveAccount) {
      getCachedElybyAvatar(currentActiveAccount.username).then((skinDataUrl) => {
        setTimeout(() => {
          if (!skinDataUrl || !avatarImg) return;
          if (skinDataUrl.startsWith("data:")) {
            const img = new Image();
            img.onload = () => {
              const c = document.createElement("canvas");
              c.width = 64;
              c.height = 64;
              const ctx = c.getContext("2d");
              ctx.imageSmoothingEnabled = false;
              ctx.drawImage(img, 8, 8, 8, 8, 0, 0, 64, 64);
              avatarImg.src = c.toDataURL();
            };
            img.src = skinDataUrl;
          } else {
            avatarImg.src = skinDataUrl || `https://mc-heads.net/head/${currentActiveAccount?.username}/64`;
          }
        }, 0);
      }).catch(() => {
        if (avatarImg) avatarImg.src = `https://mc-heads.net/head/${currentActiveAccount?.username}/64`;
      });
      // Also update Ely.by profile display
      updateElybyProfileDisplay();
    }
  } catch (e) {
    accountSelect.innerHTML = `<option value="">Error: ${e.message || e}</option>`;
  }
}

async function fetchJavaInstallations() {
  try {
    let installations = await invoke("get_installed_java");
    let settings = await invoke("get_settings");

    // Update javaAutoSelect (in case it still exists on main screen)
    if (javaAutoSelect) {
      javaAutoSelect.innerHTML = `<option value="">System Default</option>`;
      installations.forEach(java => {
        let opt = document.createElement("option");
        opt.value = java.path;
        opt.textContent = `Java ${java.major_version} (${java.version}) - ${java.path}`;
        javaAutoSelect.appendChild(opt);
      });
    }

    // Update Java Manager UI in Settings
    let versionsToCheck = [17, 21, 25];
    versionsToCheck.forEach(ver => {
      let container = document.querySelector(`#java-${ver}-status`);
      if (container) {
        let installedJava = installations.find(j => j.major_version === ver);
        if (installedJava) {
          let isActive = (settings.custom_java_path === installedJava.path);
          if (isActive) {
            container.innerHTML = `<button class="btn btn-primary" disabled style="background: var(--success); opacity: 1; padding: 5px 15px; font-size: 0.85rem; color: white; cursor: default;">✅ Aktif</button>`;
          } else {
            container.innerHTML = `<button class="btn btn-outline btn-set-java" data-path="${installedJava.path}" type="button" style="padding: 5px 15px; font-size: 0.85rem;">Gunakan</button>`;
          }
        } else {
          container.innerHTML = `<button class="btn btn-primary btn-install-java" data-version="${ver}" type="button" style="padding: 5px 15px; font-size: 0.85rem;">⬇️ Install</button>`;
        }
      }
    });

    // Rebind set active java buttons
    document.querySelectorAll(".btn-set-java").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        let path = e.target.getAttribute("data-path");
        let currentSettings = await invoke("get_settings");
        currentSettings.custom_java_path = path;
        await invoke("save_settings", { settings: currentSettings });
        if (typeof globalSettings !== 'undefined') globalSettings = currentSettings; // update global ref if exists
        await fetchJavaInstallations(); // refresh ui
      });
    });

    // Rebind install buttons
    document.querySelectorAll(".btn-install-java").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        let v = parseInt(e.target.getAttribute("data-version"));
        let originalText = e.target.textContent;
        e.target.textContent = "Mengunduh...";
        e.target.disabled = true;
        try {
          await invoke("install_java", { version: v });
          // Automatically set as active if successful
          let newInstalls = await invoke("get_installed_java");
          let newlyInstalled = newInstalls.find(j => j.major_version === v);
          if (newlyInstalled) {
            let currentSettings = await invoke("get_settings");
            currentSettings.custom_java_path = newlyInstalled.path;
            await invoke("save_settings", { settings: currentSettings });
          }
          await fetchJavaInstallations(); // refresh
          alert(`Java ${v} berhasil dipasang dan diaktifkan!`);
        } catch (err) {
          alert(`Gagal memasang Java: ${err}`);
          e.target.textContent = originalText;
          e.target.disabled = false;
        }
      });
    });

  } catch (e) {
    if (javaAutoSelect) javaAutoSelect.innerHTML = `<option value="">Error detecting Java</option>`;
    console.error(e);
  }
}

async function launchGame() {
  if (!currentActiveAccount) {
    alert("Please add and select an account first!");
    return;
  }
  if (!currentActiveInstance) {
    alert("Please create and select an Instance first!");
    return;
  }

  statusMsg.textContent = "Menyiapkan...";
  launchBtn.disabled = true;

  try {
    // Save settings safely
    let currentSettings = await invoke("get_settings");
    currentSettings.active_account_id = currentActiveAccount.id;

    let ramValue = parseInt(document.querySelector("#settings-ram-max").value) || 4096;
    currentSettings.ram_min = ramValue / 4;
    currentSettings.ram_max = ramValue;

    await invoke("save_settings", { settings: currentSettings });

    let result = await invoke("launch_game", {
      username: currentActiveAccount.username,
      uuidStr: currentActiveAccount.uuid,
      version: currentActiveInstance.version,
      ram: 4, // Ignored in Rust (uses global settings instead)
      javaPath: currentSettings.custom_java_path,
      instanceId: currentActiveInstance.id
    });
    statusMsg.textContent = "Membuka Minecraft...";
    launchBtn.textContent = "Sedang Bermain";
    // Biarkan tombol disable saat bermain
  } catch (error) {
    statusMsg.textContent = "Error: " + error;
    launchBtn.disabled = false;
    launchBtn.textContent = "Mainkan";
  }
}

async function fetchPurpurVersions() {
  let sel = document.querySelector("#server-version-select");
  if (!sel) return;
  try {
    let res = await fetch("https://api.purpurmc.org/v2/purpur/");
    let data = await res.json();
    if (data && data.versions && data.versions.length > 0) {
      sel.innerHTML = "";
      // Reverse so newest versions are first
      let versions = [...data.versions].reverse();
      versions.forEach(v => {
        let opt = document.createElement("option");
        opt.value = v;
        opt.textContent = v;
        sel.appendChild(opt);
      });
    }
  } catch (e) {
    sel.innerHTML = `<option value="1.20.4">1.20.4 (offline)</option>`;
  }
}



async function openModManager(instanceId, instanceName) {
  let modal = document.querySelector("#mod-manager-modal");
  let title = document.querySelector("#mod-manager-title");
  let container = document.querySelector("#mod-list-container");

  if (!modal || !container) return;

  title.textContent = `Kelola Mod: ${instanceName}`;
  container.innerHTML = "Memuat mod...";
  modal.style.display = "flex";

  let closeBtn = document.querySelector("#close-mod-manager");
  closeBtn.onclick = () => { modal.style.display = "none"; };

  try {
    let mods = await invoke("get_instance_mods", { instanceId });
    container.innerHTML = "";

    if (mods.length === 0) {
      container.innerHTML = "<p style='color: var(--text-muted);'>Belum ada mod yang terpasang di instance ini.</p>";
      return;
    }

    mods.forEach(mod => {
      let modCard = document.createElement("div");
      modCard.className = "panel";
      modCard.style.cssText = "display: flex; justify-content: space-between; align-items: center; padding: 12px; margin-bottom: 8px;";

      let nameLabel = document.createElement("span");
      nameLabel.textContent = mod.name.replace(".jar.disabled", "").replace(".jar", "");
      nameLabel.style.fontWeight = "500";
      nameLabel.style.fontSize = "14px";
      if (!mod.enabled) {
        nameLabel.style.color = "var(--text-muted)";
        nameLabel.style.textDecoration = "line-through";
      }

      let actionContainer = document.createElement("div");
      actionContainer.style.display = "flex";
      actionContainer.style.gap = "8px";

      let toggleBtn = document.createElement("button");
      toggleBtn.className = "btn";
      toggleBtn.style.cssText = mod.enabled
        ? "background: var(--success); color: white; border: none; font-size: 12px; padding: 6px 12px;"
        : "background: #f1f3f5; color: var(--text-muted); border: none; font-size: 12px; padding: 6px 12px;";
      toggleBtn.textContent = mod.enabled ? "ON" : "OFF";

      toggleBtn.addEventListener("click", async () => {
        try {
          await invoke("toggle_mod", {
            instanceId,
            modName: mod.name,
            enabled: !mod.enabled
          });
          modal.style.display = "none"; // Close modal, user can reopen
        } catch (err) {
          alert("Gagal mengubah status mod: " + err);
        }
      });

      let deleteBtn = document.createElement("button");
      deleteBtn.className = "btn";
      deleteBtn.style.cssText = "background: #ffebee; color: var(--danger); border: none; font-size: 12px; padding: 6px 12px;";
      deleteBtn.innerHTML = "Hapus";

      deleteBtn.addEventListener("click", async () => {
        if (confirm(`Yakin ingin menghapus ${mod.name}?`)) {
          try {
            await invoke("delete_mod", {
              instanceId,
              modName: mod.name
            });
            modal.style.display = "none"; // Close modal, user can reopen
          } catch (err) {
            alert("Gagal menghapus mod: " + err);
          }
        }
      });

      actionContainer.appendChild(toggleBtn);
      actionContainer.appendChild(deleteBtn);

      modCard.appendChild(nameLabel);
      modCard.appendChild(actionContainer);
      container.appendChild(modCard);
    });
  } catch (e) {
    container.innerHTML = "Gagal memuat mod: " + e;
  }
}

async function searchModrinth(query) {
  let grid = document.querySelector("#modrinth-grid");
  if (!grid) return;
  grid.innerHTML = "Memuat data dari Modrinth...";

  if (!currentActiveInstance) {
    grid.innerHTML = "Pilih instance terlebih dahulu!";
    return;
  }

  try {
    let loader = currentActiveInstance.loader.toLowerCase();
    let mc_version = currentActiveInstance.version;
    let facets = `[["versions:${mc_version}"],["categories:${loader}"],["project_type:mod"]]`;
    let url = `https://api.modrinth.com/v2/search?query=${encodeURIComponent(query)}&facets=${encodeURIComponent(facets)}&limit=10`;

    let res = await fetch(url);
    let data = await res.json();

    grid.innerHTML = "";
    if (data.hits.length === 0) {
      grid.innerHTML = "Mod tidak ditemukan.";
      return;
    }

    // Fetch installed mods to check which are already installed
    let installedMods = [];
    try {
      installedMods = await invoke("get_instance_mods", { instanceId: currentActiveInstance.id });
    } catch (e) { /* ignore */ }
    let installedNames = installedMods.map(m => m.name.toLowerCase());

    data.hits.forEach(mod => {
      let card = document.createElement("div");
      card.className = "news-card panel";

      // Try to find the exact target version
      let targetVersionLabel = mc_version;
      if (mod.versions && mod.versions.includes(mc_version)) {
        targetVersionLabel = mc_version;
      }

      let downloads = mod.downloads >= 1000000 ? (mod.downloads / 1000000).toFixed(1) + "M" :
        mod.downloads >= 1000 ? (mod.downloads / 1000).toFixed(1) + "K" : mod.downloads;

      // Check if mod is already installed (match slug against filenames)
      let isInstalled = installedNames.some(name => name.includes(mod.slug.toLowerCase()));

      card.innerHTML = `
        <div style="display: flex; gap: 12px; align-items: start; margin-bottom: 12px;">
          <img src="${mod.icon_url || 'https://mc-heads.net/avatar/Steve/64'}" width="48" height="48" style="border-radius: 8px;" />
          <div>
            <h4 style="margin-bottom: 4px; font-size: 15px;">${mod.title}</h4>
            <p style="font-size: 11px; color: var(--accent); margin-bottom: 4px;">by ${mod.author}</p>
            <p style="font-size: 12px;">${mod.description.substring(0, 50)}...</p>
          </div>
        </div>
        <div style="display: flex; justify-content: space-between; font-size: 11px; color: var(--text-muted); margin-bottom: 12px; font-weight: 600;">
          <span style="background: #eef2ff; color: var(--accent); padding: 2px 6px; border-radius: 4px;">🎯 ${targetVersionLabel} ${loader.toUpperCase()}</span>
          <span>📥 ${downloads}</span>
        </div>
        <div style="display: flex; gap: 8px; margin-bottom: 12px; align-items: center;">
          <select id="ver-${mod.slug}" class="input-box" style="flex: 1; font-size: 11px; padding: 4px; background: rgba(0,0,0,0.05); border-color: rgba(0,0,0,0.1);">
            <option value="">Memuat versi...</option>
          </select>
        </div>
        <button class="btn ${isInstalled ? '' : 'btn-primary'} mod-install-btn" data-slug="${mod.slug}" data-is-installed="${isInstalled}" style="width: 100%; ${isInstalled ? 'background: var(--accent); color: white; border: none;' : ''}">${isInstalled ? 'Ganti Versi / Re-install' : 'Install'}</button>
      `;
      grid.appendChild(card);

      // Async fetch versions for this mod
      fetch(`https://api.modrinth.com/v2/project/${mod.slug}/version?loaders=["${loader}"]&game_versions=["${mc_version}"]`)
        .then(res => res.json())
        .then(verData => {
          let select = document.getElementById(`ver-${mod.slug}`);
          if (select) {
            select.innerHTML = "";
            if (verData.length === 0) {
              select.innerHTML = `<option value="">Tidak ada versi cocok</option>`;
            } else {
              verData.forEach(v => {
                let opt = document.createElement("option");
                opt.value = v.id;
                opt.textContent = v.name || v.version_number;
                select.appendChild(opt);
              });
            }
          }
        })
        .catch(err => {
          let select = document.getElementById(`ver-${mod.slug}`);
          if (select) select.innerHTML = `<option value="">Error memuat versi</option>`;
        });
    });

    grid.querySelectorAll(".mod-install-btn").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        let slug = e.target.getAttribute("data-slug");
        e.target.textContent = "Menganalisa...";
        e.target.disabled = true;

        try {
          let isInstalled = e.target.getAttribute("data-is-installed") === "true";
          let specificVersionId = document.getElementById(`ver-${slug}`).value;
          if (!specificVersionId) {
            alert("Silakan tunggu versi dimuat atau tidak ada versi yang cocok.");
            e.target.textContent = isInstalled ? "Ganti Versi / Re-install" : "Install";
            e.target.disabled = false;
            return;
          }

          if (isInstalled) {
            let existingMod = installedMods.find(m => m.name.toLowerCase().includes(slug.toLowerCase()));
            if (existingMod) {
              try {
                await invoke("delete_mod", { instanceId: currentActiveInstance.id, modName: existingMod.name });
              } catch (err) {
                console.log("Failed to delete old mod", err);
              }
            }
          }

          // Recursive Dependency Resolver
          let downloadQueue = [];
          let resolvedSet = new Set();

          async function resolveDeps(projectId, specificVerId = null) {
            let verUrl = specificVerId ?
              `https://api.modrinth.com/v2/version/${specificVerId}` :
              `https://api.modrinth.com/v2/project/${projectId}/version?loaders=["${loader}"]&game_versions=["${mc_version}"]`;

            let verRes = await fetch(verUrl);
            if (!verRes.ok) return;

            let version;
            if (specificVerId) {
              version = await verRes.json();
            } else {
              let verData = await verRes.json();
              if (verData.length > 0) {
                version = verData[0];
              }
            }

            if (version) {
              let file = version.files.find(f => f.primary) || version.files[0];

              if (!resolvedSet.has(file.url)) {
                resolvedSet.add(file.url);
                downloadQueue.push(file);
                e.target.textContent = `Menganalisa... (${downloadQueue.length} files)`;
              }

              if (version.dependencies && version.dependencies.length > 0) {
                for (let dep of version.dependencies) {
                  // Skip Sodium (AANobbMI) if we are installing Iris, as newer Iris embeds it or user prefers standalone
                  if (dep.project_id === "AANobbMI" && slug === "iris") {
                    continue;
                  }
                  if (dep.dependency_type === "required" && dep.project_id) {
                    await resolveDeps(dep.project_id, null);
                  }
                }
              }
            }
          }

          await resolveDeps(slug, specificVersionId);

          if (downloadQueue.length === 0) {
            alert("Versi file yang cocok tidak ditemukan.");
            e.target.textContent = "Install";
            e.target.disabled = false;
            return;
          }

          e.target.textContent = `Mengunduh... (0/${downloadQueue.length})`;

          let successCount = 0;
          let promises = downloadQueue.map(async (file) => {
            try {
              await invoke("download_mod_to_instance", {
                instanceId: currentActiveInstance.id,
                downloadUrl: file.url,
                fileName: file.filename
              });
              successCount++;
              e.target.textContent = `Mengunduh... (${successCount}/${downloadQueue.length})`;
            } catch (err) {
              if (String(err).toLowerCase().includes("already exists")) {
                successCount++; // count as success if already exists
              } else {
                throw err;
              }
            }
          });

          await Promise.all(promises);

          e.target.textContent = "✓ Terpasang";
          e.target.style.background = "var(--success)";
          e.target.style.color = "white";
          e.target.style.border = "none";
          e.target.setAttribute("data-is-installed", "true");

          // Refresh installed mods in memory so if they click again it can find the new file
          try {
            installedMods = await invoke("get_instance_mods", { instanceId: currentActiveInstance.id });
          } catch (e) { }

        } catch (err) {
          alert("Gagal: " + err);
          let isInstalled = e.target.getAttribute("data-is-installed") === "true";
          e.target.textContent = isInstalled ? "Ganti Versi / Re-install" : "Install";
          e.target.disabled = false;
        }
      });
    });

  } catch (e) {
    grid.innerHTML = "Gagal mengambil data dari Modrinth.";
  }
}

async function searchModrinthShaders(query) {
  let grid = document.querySelector("#shaders-grid");
  if (!grid) return;
  grid.innerHTML = "Memuat data dari Modrinth...";

  if (!currentActiveInstance) {
    grid.innerHTML = "Pilih instance terlebih dahulu!";
    return;
  }

  try {
    let mc_version = currentActiveInstance.version;
    // We only filter by version and project_type:shader since shaders work for both Iris/Optifine usually.
    let facets = `[["versions:${mc_version}"],["project_type:shader"]]`;
    let url = `https://api.modrinth.com/v2/search?query=${encodeURIComponent(query)}&facets=${encodeURIComponent(facets)}&limit=10`;

    let res = await fetch(url);
    let data = await res.json();

    grid.innerHTML = "";
    if (data.hits.length === 0) {
      grid.innerHTML = "Shader tidak ditemukan.";
      return;
    }

    data.hits.forEach(shader => {
      let card = document.createElement("div");
      card.className = "news-card panel";

      let downloads = shader.downloads >= 1000000 ? (shader.downloads / 1000000).toFixed(1) + "M" :
        shader.downloads >= 1000 ? (shader.downloads / 1000).toFixed(1) + "K" : shader.downloads;

      card.innerHTML = `
        <div style="display: flex; gap: 12px; align-items: start; margin-bottom: 12px;">
          <img src="${shader.icon_url || 'https://mc-heads.net/avatar/Steve/64'}" width="48" height="48" style="border-radius: 8px;" />
          <div>
            <h4 style="margin-bottom: 4px; font-size: 15px;">${shader.title}</h4>
            <p style="font-size: 11px; color: var(--accent); margin-bottom: 4px;">by ${shader.author}</p>
            <p style="font-size: 12px;">${shader.description.substring(0, 50)}...</p>
          </div>
        </div>
        <div style="display: flex; justify-content: space-between; font-size: 11px; color: var(--text-muted); margin-bottom: 12px; font-weight: 600;">
          <span style="background: #eef2ff; color: var(--accent); padding: 2px 6px; border-radius: 4px;">✨ Shaderpack</span>
          <span>📥 ${downloads}</span>
        </div>
        <button class="btn btn-primary shader-install-btn" data-slug="${shader.slug}" style="width: 100%;">Install</button>
      `;
      grid.appendChild(card);
    });

    grid.querySelectorAll(".shader-install-btn").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        let slug = e.target.getAttribute("data-slug");
        e.target.textContent = "Mengunduh...";
        e.target.disabled = true;
        try {
          // Fetch versions matching mc_version
          let verUrl = `https://api.modrinth.com/v2/project/${slug}/version?game_versions=["${mc_version}"]`;
          let verRes = await fetch(verUrl);
          let verData = await verRes.json();
          if (verData.length > 0) {
            let file = verData[0].files.find(f => f.primary) || verData[0].files[0];
            await invoke("download_shader_to_instance", {
              instanceId: currentActiveInstance.id,
              downloadUrl: file.url,
              fileName: file.filename
            });
            e.target.textContent = "Terpasang";
            e.target.style.background = "var(--success)";
            e.target.style.color = "white";
            e.target.style.border = "none";
          } else {
            alert("Versi file yang cocok tidak ditemukan.");
            e.target.textContent = "Install";
            e.target.disabled = false;
          }
        } catch (err) {
          let errStr = String(err);
          if (errStr.toLowerCase().includes("already exists")) {
            e.target.textContent = "✓ Terpasang";
            e.target.style.background = "var(--success)";
            e.target.style.color = "white";
            e.target.style.border = "none";
          } else {
            alert("Gagal: " + err);
            e.target.textContent = "Install";
            e.target.disabled = false;
          }
        }
      });
    });

  } catch (e) {
    grid.innerHTML = "Gagal mengambil data dari Modrinth.";
  }
}

window.addEventListener("DOMContentLoaded", async () => {

  accountSelect = document.querySelector("#account-select");
  avatarImg = document.querySelector("#avatar-img");
  activeUsernameDisplay = document.querySelector("#active-username-display");
  addAccountBtn = document.querySelector("#add-account-btn");
  deleteAccountBtn = document.querySelector("#delete-account-btn");

  customVersionWrapper = document.querySelector("#custom-version-wrapper");
  customVersionDisplay = document.querySelector("#custom-version-display");
  customVersionOptions = document.querySelector("#custom-version-options");
  versionSelectHidden = document.querySelector("#version-select");

  customInstanceWrapper = document.querySelector("#custom-instance-wrapper");
  customInstanceDisplay = document.querySelector("#custom-instance-display");
  customInstanceOptions = document.querySelector("#custom-instance-options");
  instanceSelectHidden = document.querySelector("#instance-select");

  customLoaderWrapper = document.querySelector("#custom-loader-wrapper");
  customLoaderDisplay = document.querySelector("#custom-loader-display");
  customLoaderOptions = document.querySelector("#custom-loader-options");
  loaderSelectHidden = document.querySelector("#loader-select");

  // Custom Dropdown for Create Instance Version
  if (customVersionDisplay && customVersionOptions) {
    customVersionDisplay.addEventListener("click", (e) => {
      e.stopPropagation();
      customVersionOptions.classList.toggle("open");
      if (customRamOptions) customRamOptions.classList.remove("open");
      if (customInstanceOptions) customInstanceOptions.classList.remove("open");
      if (customLoaderOptions) customLoaderOptions.classList.remove("open");
    });
  }

  // Custom Dropdown for Instances (Dock)
  if (customInstanceDisplay && customInstanceOptions) {
    customInstanceDisplay.addEventListener("click", (e) => {
      e.stopPropagation();
      customInstanceOptions.classList.toggle("open");
      if (customRamOptions) customRamOptions.classList.remove("open");
      if (customVersionOptions) customVersionOptions.classList.remove("open");
      if (customLoaderOptions) customLoaderOptions.classList.remove("open");
    });
  }

  // Custom Dropdown for Loader (Create Instance)
  if (customLoaderDisplay && customLoaderOptions) {
    let loaderOpts = customLoaderOptions.querySelectorAll(".custom-option");
    loaderOpts.forEach(opt => {
      opt.addEventListener("click", () => {
        loaderSelectHidden.value = opt.textContent.toLowerCase();
        customLoaderDisplay.textContent = opt.textContent;
        customLoaderOptions.classList.remove("open");
      });
    });
    customLoaderDisplay.addEventListener("click", (e) => {
      e.stopPropagation();
      customLoaderOptions.classList.toggle("open");
      if (customRamOptions) customRamOptions.classList.remove("open");
      if (customVersionOptions) customVersionOptions.classList.remove("open");
      if (customInstanceOptions) customInstanceOptions.classList.remove("open");
    });
  }

  // Setup Custom Dropdown for RAM
  customRamDisplay = document.querySelector("#custom-ram-display");
  customRamOptions = document.querySelector("#custom-ram-options");

  if (customRamDisplay && customRamOptions) {
    // Populate RAM 1GB to 8GB
    for (let i = 1; i <= 8; i++) {
      let div = document.createElement("div");
      div.className = "custom-option";
      div.textContent = `${i} GB`;
      div.addEventListener("click", () => {
        ramSelect.value = i;
        customRamDisplay.textContent = `${i} GB`;
        customRamOptions.classList.remove("open");
      });
      customRamOptions.appendChild(div);
    }

    customRamDisplay.addEventListener("click", (e) => {
      e.stopPropagation();
      customRamOptions.classList.toggle("open");
      if (customVersionOptions) customVersionOptions.classList.remove("open");
      if (customInstanceOptions) customInstanceOptions.classList.remove("open");
      if (customLoaderOptions) customLoaderOptions.classList.remove("open");
    });
  }

  // Close all custom dropdowns when clicking outside
  document.addEventListener("click", () => {
    if (customVersionOptions) customVersionOptions.classList.remove("open");
    if (customRamOptions) customRamOptions.classList.remove("open");
    if (customInstanceOptions) customInstanceOptions.classList.remove("open");
    if (customLoaderOptions) customLoaderOptions.classList.remove("open");
  });

  // Add Account Toggle logic
  let showAddAccountBtn = document.querySelector("#show-add-account-btn");
  let addAccountForm = document.querySelector("#add-account-form");
  if (showAddAccountBtn && addAccountForm) {
    showAddAccountBtn.addEventListener("click", () => {
      showAddAccountBtn.style.display = "none";
      addAccountForm.style.display = "flex";
    });
  }
  ramSelect = document.querySelector("#ram-select");
  javaAutoSelect = document.querySelector("#java-auto-select");
  javaCustomInput = document.querySelector("#java-custom-input");
  statusMsg = document.querySelector("#status-msg");
  launchBtn = document.querySelector("#launch-btn");
  logBox = document.querySelector("#log-box");

  // Removed sidebar toggle since it is just an indicator

  // Load settings
  let settings = await invoke("get_settings");
  if (ramSelect && settings) {
    let ramGb = Math.round((settings.ram_max || 4096) / 1024);
    ramSelect.value = ramGb.toString();
    if (customRamDisplay) {
      customRamDisplay.textContent = `${ramGb} GB`;
    }
  }
  if (settings.custom_java_path) {
    javaCustomInput.value = settings.custom_java_path;
  }

  // Parallel fetch
  await Promise.all([
    fetchAccounts(),
    fetchVersions(),
    fetchInstances(),
    fetchJavaInstallations(),
    fetchPurpurVersions()
  ]);

  accountSelect.addEventListener("change", async (e) => {
    if (e.target.value) {
      await invoke("switch_account", { accountId: e.target.value });
      await fetchAccounts();
    }
  });

  addAccountBtn.addEventListener("click", async () => {
    let inputEl = document.querySelector("#new-account-input");
    let username = inputEl ? inputEl.value : null;
    if (username && username.trim() !== "") {
      await invoke("add_account", { username: username.trim() });
      if (inputEl) inputEl.value = "";

      // Hide form, show button again
      if (showAddAccountBtn && addAccountForm) {
        showAddAccountBtn.style.display = "block";
        addAccountForm.style.display = "none";
      }

      await fetchAccounts();
    }
  });

  let cancelAccountBtn = document.querySelector("#cancel-account-btn");
  if (cancelAccountBtn) {
    cancelAccountBtn.addEventListener("click", () => {
      let inputEl = document.querySelector("#new-account-input");
      if (inputEl) inputEl.value = "";

      if (showAddAccountBtn && addAccountForm) {
        showAddAccountBtn.style.display = "block";
        addAccountForm.style.display = "none";
      }
    });
  }

  deleteAccountBtn.addEventListener("click", async () => {
    if (currentActiveAccount && confirm(`Delete account ${currentActiveAccount.username}?`)) {
      await invoke("delete_account", { accountId: currentActiveAccount.id });
      await fetchAccounts();
    }
  });

  // Ely.by profile buttons
  const manageElybyBtn = document.querySelector("#btn-manage-elyby");
  if (manageElybyBtn) {
    manageElybyBtn.addEventListener("click", async () => {
      try {
        if (window.__TAURI__?.shell) {
          await window.__TAURI__.shell.open("https://ely.by");
        } else if (window.__TAURI__?.core) {
          await window.__TAURI__.core.invoke("plugin:opener|open", { path: "https://ely.by" });
        } else {
          window.open("https://ely.by", "_blank");
        }
      } catch (e) {
        console.error("Failed to open Ely.by:", e);
        window.open("https://ely.by", "_blank");
      }
    });
  }

  const refreshElybyBtn = document.querySelector("#btn-refresh-elyby");
  if (refreshElybyBtn) {
    refreshElybyBtn.addEventListener("click", async () => {
      elybyProfileCache = {};
      elybyAvatarCache = {};
      await updateElybyProfileDisplay();
    });
  }


  // TABS LOGIC
  let navInstances = document.querySelector("#nav-instances");
  let navNews = document.querySelector("#nav-news");
  let navModrinth = document.querySelector("#nav-modrinth");
  let navShaders = document.querySelector("#nav-shaders");
  let navSettings = document.querySelector("#nav-settings");
  let navScreenshots = document.querySelector("#nav-screenshots");

  let viewInstances = document.querySelector("#view-instances");
  let viewNews = document.querySelector("#view-news");
  let viewModrinth = document.querySelector("#view-modrinth");
  let viewShaders = document.querySelector("#view-shaders");
  let viewSettings = document.querySelector("#view-settings");
  let viewScreenshots = document.querySelector("#view-screenshots");

  function switchTab(activeNav, activeView) {
    [navInstances, navNews, navModrinth, navShaders, navSettings, navScreenshots].forEach(n => n && n.classList.remove("active"));
    [viewInstances, viewNews, viewModrinth, viewShaders, viewSettings, viewScreenshots].forEach(v => v && (v.style.display = "none"));

    if (activeNav) activeNav.classList.add("active");
    if (activeView) activeView.style.display = "block";
  }

  if (navInstances) navInstances.addEventListener("click", () => switchTab(navInstances, viewInstances));
  if (navNews) navNews.addEventListener("click", () => switchTab(navNews, viewNews));
  if (navModrinth) navModrinth.addEventListener("click", () => switchTab(navModrinth, viewModrinth));
  if (navShaders) navShaders.addEventListener("click", () => switchTab(navShaders, viewShaders));
  if (navSettings) navSettings.addEventListener("click", () => {
    switchTab(navSettings, viewSettings);
    loadSettings(); // Refresh form when opened
  });
  if (navScreenshots) navScreenshots.addEventListener("click", () => {
    switchTab(navScreenshots, viewScreenshots);
    initScreenshotsView();
  });

  // MODRINTH SEARCH
  let modrinthSearchBtn = document.querySelector("#modrinth-search-btn");
  let modrinthSearchInput = document.querySelector("#modrinth-search-input");
  if (modrinthSearchBtn && modrinthSearchInput) {
    modrinthSearchBtn.addEventListener("click", () => {
      searchModrinth(modrinthSearchInput.value);
    });
    modrinthSearchInput.addEventListener("keydown", (e) => {
      if (e.key === "Enter") searchModrinth(modrinthSearchInput.value);
    });
  }

  // SHADERS SEARCH
  let shadersSearchBtn = document.querySelector("#shaders-search-btn");
  let shadersSearchInput = document.querySelector("#shaders-search-input");
  if (shadersSearchBtn && shadersSearchInput) {
    shadersSearchBtn.addEventListener("click", () => {
      searchModrinthShaders(shadersSearchInput.value);
    });
    shadersSearchInput.addEventListener("keydown", (e) => {
      if (e.key === "Enter") searchModrinthShaders(shadersSearchInput.value);
    });

    // Initial fetch for shaders
    setTimeout(() => {
      searchModrinthShaders("");
    }, 1000);
  }

  // CREATE INSTANCE LOGIC
  let showCreateBtn = document.querySelector("#show-create-instance-btn");
  let createForm = document.querySelector("#create-instance-form");
  let confirmCreateBtn = document.querySelector("#confirm-create-instance-btn");
  let cancelCreateBtn = document.querySelector("#cancel-create-instance-btn");
  let newInstanceName = document.querySelector("#new-instance-name");

  if (showCreateBtn && createForm) {
    showCreateBtn.addEventListener("click", () => {
      showCreateBtn.style.display = "none";
      createForm.style.display = "block";
    });
    cancelCreateBtn.addEventListener("click", () => {
      showCreateBtn.style.display = "block";
      createForm.style.display = "none";
      newInstanceName.value = "";
    });
    confirmCreateBtn.addEventListener("click", async () => {
      let name = newInstanceName.value.trim();
      let ver = versionSelectHidden.value;
      let loader = loaderSelectHidden.value;
      if (name !== "" && ver !== "") {
        // Auto Java 25 Check
        if (ver.startsWith("26.") || ver.startsWith("1.21.2") || ver.startsWith("1.21.3") || ver.startsWith("1.21.4")) {
          let installations = await invoke("get_installed_java");
          let hasJava25 = installations.some(j => j.major_version === 25);
          if (!hasJava25) {
            let confirmInstall = confirm(`Versi ${ver} mewajibkan Java 25, tetapi Anda belum menginstallnya.\n\nApakah Anda ingin mengunduh dan menginstall Java 25 secara otomatis sekarang?`);
            if (confirmInstall) {
              let originalBtnText = confirmCreateBtn.textContent;
              confirmCreateBtn.disabled = true;
              confirmCreateBtn.textContent = "Mengunduh Java 25...";
              try {
                await invoke("install_java", { version: 25 });
                let newInstalls = await invoke("get_installed_java");
                let j25 = newInstalls.find(j => j.major_version === 25);
                if (j25) {
                  let currentSettings = await invoke("get_settings");
                  currentSettings.custom_java_path = j25.path;
                  await invoke("save_settings", { settings: currentSettings });
                  if (typeof globalSettings !== 'undefined') globalSettings = currentSettings;
                  if (typeof fetchJavaInstallations !== 'undefined') await fetchJavaInstallations();
                  alert("Java 25 berhasil diinstall dan diaktifkan!");
                }
              } catch (e) {
                alert("Gagal menginstall Java 25: " + e);
              }
              confirmCreateBtn.disabled = false;
              confirmCreateBtn.textContent = originalBtnText;
            }
          }
        }

        let newInst = await invoke("create_instance", { name, version: ver, loader });

        let potatoCheck = document.getElementById("auto-optimize-mods");
        if (potatoCheck && potatoCheck.checked && loader === "fabric") {
          let originalBtnText = confirmCreateBtn.textContent;
          confirmCreateBtn.disabled = true;
          confirmCreateBtn.textContent = "Memasang Mod Optimasi...";
          await installOptimizationMods(newInst.id, ver, loader);
          confirmCreateBtn.textContent = originalBtnText;
          confirmCreateBtn.disabled = false;
        }

        newInstanceName.value = "";
        showCreateBtn.style.display = "block";
        createForm.style.display = "none";
        await fetchInstances();
      } else {
        alert("Nama dan Versi tidak boleh kosong!");
      }
    });
  }

  async function installOptimizationMods(instanceId, mc_version, loader) {
    let mods = ["sodium", "lithium", "ferrite-core", "modmenu"];
    let downloadQueue = [];
    let resolvedSet = new Set();

    async function resolveDeps(projectId) {
      let verUrl = `https://api.modrinth.com/v2/project/${projectId}/version?loaders=["${loader}"]&game_versions=["${mc_version}"]`;
      try {
        let verRes = await fetch(verUrl);
        if (!verRes.ok) return;
        let verData = await verRes.json();
        if (verData.length > 0) {
          let version = verData[0];
          let file = version.files.find(f => f.primary) || version.files[0];
          if (!resolvedSet.has(file.url)) {
            resolvedSet.add(file.url);
            downloadQueue.push(file);
          }
          if (version.dependencies && version.dependencies.length > 0) {
            for (let dep of version.dependencies) {
              if (dep.dependency_type === "required" && dep.project_id) {
                await resolveDeps(dep.project_id);
              }
            }
          }
        }
      } catch (e) { console.error(e); }
    }

    for (let slug of mods) {
      await resolveDeps(slug);
    }

    if (downloadQueue.length === 0) {
      alert("Gagal mengunduh mod optimasi otomatis. Pastikan versi Minecraft (" + mc_version + ") penulisan versinya benar (contoh: 1.21.1) dan didukung oleh Fabric.");
      return;
    }

    let promises = downloadQueue.map(file => {
      return invoke("download_mod_to_instance", {
        instanceId: instanceId,
        downloadUrl: file.url,
        fileName: file.filename
      });
    });

    try {
      await Promise.all(promises);
    } catch (e) {
      console.error("Gagal install optimasi", e);
    }
  }

  // Server Hub Wiring
  let startServerBtn = document.querySelector("#start-server-btn");
  let serverStatusBadge = document.querySelector("#server-status-badge");
  let serverVersionSelect = document.querySelector("#server-version-select");
  let serverRamSelect = document.querySelector("#server-ram-select");
  let serverConsole = document.querySelector("#server-console");

  let isServerRunning = false;

  if (startServerBtn) {
    startServerBtn.addEventListener("click", async () => {
      let v = serverVersionSelect.value;
      let r = serverRamSelect.value;
      if (!v) {
        alert("Pilih versi terlebih dahulu");
        return;
      }

      if (!isServerRunning) {
        startServerBtn.textContent = "MEMULAI...";
        startServerBtn.style.background = "var(--text-muted)";
        if (serverConsole) serverConsole.textContent = "Menghubungkan...\n";
        try {
          let core = "purpur";
          let version = serverVersionSelect.value;
          let ram = parseInt(serverRamSelect.value);

          await invoke("start_server", { core, version, ram });

          isServerRunning = true;
          startServerBtn.textContent = "◼ STOP SERVER";
          startServerBtn.style.background = "var(--danger)";
          startServerBtn.style.color = "white";
          serverStatusBadge.textContent = "ON";
          serverStatusBadge.style.background = "var(--success)";
          serverStatusBadge.style.color = "white";
        } catch (e) {
          alert("Failed to start server:\n" + e);
          startServerBtn.textContent = "▶ START SERVER";
          startServerBtn.style.background = "var(--success)";
        }
      } else {
        startServerBtn.textContent = "STOPPING...";
        startServerBtn.style.background = "var(--text-muted)";
        try {
          await invoke("stop_server");
          await invoke("stop_tunnel");
        } catch (e) {
          console.error("Error stopping:", e);
        } finally {
          isServerRunning = false;
          startServerBtn.textContent = "▶ START SERVER";
          startServerBtn.style.background = "var(--success)";
          serverStatusBadge.textContent = "OFF";
          serverStatusBadge.style.background = "";
          serverStatusBadge.style.color = "";

          // Reset tunnel UI
          if (startTunnelBtn) startTunnelBtn.style.display = "block";
          if (tunnelInfoContainer) tunnelInfoContainer.style.display = "none";
          if (tunnelIpDisplay) tunnelIpDisplay.value = "Membuat tunnel...";
        }
      }
    });
  }

  // IP Detection & Copy
  let localIpDisplay = document.querySelector("#local-ip-display");
  let copyIpBtn = document.querySelector("#copy-ip-btn");
  let openPlayitBtn = document.querySelector("#open-playit-btn");

  // Auto-detect local IP
  if (localIpDisplay) {
    try {
      let ip = await invoke("get_local_ip");
      localIpDisplay.value = ip;
    } catch (e) {
      localIpDisplay.value = "Tidak terdeteksi";
    }
  }

  if (copyIpBtn && localIpDisplay) {
    copyIpBtn.addEventListener("click", () => {
      navigator.clipboard.writeText(localIpDisplay.value).then(() => {
        let old = copyIpBtn.textContent;
        copyIpBtn.textContent = "✓";
        setTimeout(() => copyIpBtn.textContent = old, 1500);
      });
    });
  }

  let startTunnelBtn = document.querySelector("#start-tunnel-btn");
  let tunnelInfoContainer = document.querySelector("#tunnel-info-container");
  let tunnelIpDisplay = document.querySelector("#tunnel-ip-display");
  let copyTunnelBtn = document.querySelector("#copy-tunnel-btn");

  if (startTunnelBtn) {
    startTunnelBtn.addEventListener("click", async () => {
      startTunnelBtn.style.display = "none";
      tunnelInfoContainer.style.display = "flex";
      tunnelIpDisplay.value = "Membuat tunnel...";

      try {
        // By default Minecraft runs on 25565. We pass 25565.
        let url = await invoke("start_tunnel", { port: 25565 });
        // Clean up the URL format (Pinggy returns something like "tcp://xyz...:1234")
        if (url.startsWith("tcp://")) url = url.substring(6);
        tunnelIpDisplay.value = url;
      } catch (e) {
        tunnelIpDisplay.value = "Gagal membuat tunnel";
        startTunnelBtn.style.display = "block";
        startTunnelBtn.textContent = "Coba Lagi";
      }
    });
  }

  if (copyTunnelBtn && tunnelIpDisplay) {
    copyTunnelBtn.addEventListener("click", () => {
      navigator.clipboard.writeText(tunnelIpDisplay.value).then(() => {
        let old = copyTunnelBtn.textContent;
        copyTunnelBtn.textContent = "✓";
        setTimeout(() => copyTunnelBtn.textContent = old, 1500);
      });
    });
  }

  await listen("progress", (event) => {
    let payload = event.payload;
    let percentage = payload.total > 0 ? Math.round((payload.current / payload.total) * 100) : 0;
    statusMsg.textContent = `[${payload.stage.toUpperCase()}] ${payload.message} (${percentage}%)`;

    let progressContainer = document.querySelector("#progress-container");
    let progressBar = document.querySelector("#progress-fill");
    if (progressContainer && progressBar) {
      progressContainer.style.display = "block";
      progressBar.style.width = percentage + "%";
    }
  });

  await listen("game-log", async (event) => {
    let payload = event.payload;
    if (typeof payload === 'string' && payload.startsWith("[SYSTEM] Game exited")) {
      let launchBtn = document.querySelector("#launch-btn");
      let statusMsg = document.querySelector("#status-msg");
      let progressContainer = document.querySelector("#progress-container");

      if (launchBtn) {
        launchBtn.disabled = false;
        launchBtn.textContent = "Mainkan";
      }
      if (statusMsg) statusMsg.textContent = "Ready to play.";
      if (progressContainer) progressContainer.style.display = "none";

      let codeMatch = payload.match(/code: (-?\d+)/);
      if (codeMatch && codeMatch[1] !== "0") {
        // CRASH DETECTED!
        try {
          let crashLog = await invoke("get_latest_crash_log", { instanceId: currentActiveInstance.id });
          analyzeCrashLog(crashLog);
        } catch (err) {
          console.error("Failed to get crash log:", err);
        }
      }
    }

    // Hidden from UI, but still logged to console for debugging
    console.log(payload);
  });

  function analyzeCrashLog(log) {
    let modal = document.getElementById("crash-analyzer-modal");
    let diagText = document.getElementById("crash-diagnosis-text");
    let solText = document.getElementById("crash-solution-text");
    let rawLog = document.getElementById("crash-raw-log");

    if (!modal) return;

    rawLog.textContent = log.length > 5000 ? log.substring(log.length - 5000) : log;

    // Analysis Logic
    let logLower = log.toLowerCase();

    if (logLower.includes("outofmemoryerror")) {
      diagText.textContent = "Game kehabisan memory (RAM) saat berjalan.";
      solText.textContent = "Buka menu Settings, lalu tambahkan alokasi RAM (Maximum RAM). Direkomendasikan minimal 4096 MB (4 GB).";
    }
    else if (logLower.includes("unsupportedclassversionerror")) {
      diagText.textContent = "Versi Java yang Anda gunakan tidak didukung oleh versi Minecraft ini.";
      solText.textContent = "Buka menu Settings, pastikan Anda menggunakan Java 17 untuk versi 1.17 - 1.20.4, Java 21 untuk 1.20.5 - 1.21.1, atau Java 25 untuk 1.21.2 ke atas.";
    }
    else if (logLower.includes("multipleversionexception") || (logLower.includes("sodium") && logLower.includes("iris") && logLower.includes("incompatible"))) {
      diagText.textContent = "Terdapat konflik mod yang parah (biasanya Iris dan Sodium, atau ada versi mod ganda).";
      solText.textContent = "Gunakan Modrinth Downloader untuk menghapus atau mengatur ulang (Re-install) versi Sodium/Iris Anda. Jangan pasang dua versi mod yang sama secara bersamaan.";
    }
    else if (logLower.includes("mixinapplyerror") || logLower.includes("mixintransformer") || logLower.includes("modresolutionexception")) {
      diagText.textContent = "Terjadi kegagalan saat memuat mod. Beberapa mod tidak kompatibel dengan loader atau game versi ini.";
      solText.textContent = "Coba perbarui semua mod Anda (Re-install versi terbaru) atau hapus mod yang baru saja Anda pasang sebelum game crash.";
    }
    else {
      diagText.textContent = "Penyebab spesifik tidak dapat dideteksi secara otomatis, namun terjadi kegagalan fatal saat memuat game.";
      solText.textContent = "Cek log mentah (Raw Log) di bawah, atau coba hapus mod terbaru yang Anda pasang.";
    }

    modal.style.display = "flex";

    document.getElementById("close-crash-modal-btn").onclick = () => modal.style.display = "none";
    document.getElementById("crash-understood-btn").onclick = () => modal.style.display = "none";
  }

   // Server Console Logic
   serverConsole = document.querySelector("#server-console");
   if (serverConsole) {
     let currentProgressLine = null;
     listen("server-log", (event) => {
       // Hide placeholder on first log
       const placeholder = document.querySelector("#server-console-placeholder");
       if (placeholder) placeholder.style.display = "none";

       let payload = event.payload;

      if (payload.startsWith("[Progress]")) {
        if (currentProgressLine) {
          currentProgressLine.textContent = payload;
        } else {
          currentProgressLine = document.createElement("div");
          currentProgressLine.style.color = "var(--accent)";
          currentProgressLine.textContent = payload;
          serverConsole.appendChild(currentProgressLine);
        }
      } else {
        currentProgressLine = null;
        let div = document.createElement("div");
        div.textContent = payload;
        serverConsole.appendChild(div);
      }
      serverConsole.scrollTop = serverConsole.scrollHeight;
    });
  }

  // Plugin Manager Logic
  let openPluginBtn = document.querySelector("#open-plugin-manager-btn");
  let closePluginBtn = document.querySelector("#close-plugin-modal-btn");
  let pluginModal = document.querySelector("#plugin-manager-modal");
  let pluginSearchInput = document.querySelector("#plugin-search-input");
  let pluginSearchBtn = document.querySelector("#plugin-search-btn");
  let pluginResults = document.querySelector("#plugin-search-results");

  if (openPluginBtn && pluginModal) {
    openPluginBtn.addEventListener("click", () => {
      if (!serverVersionSelect.value) {
        alert("Pilih versi server terlebih dahulu.");
        return;
      }
      pluginModal.style.display = "flex";
      pluginResults.innerHTML = `<div style="text-align: center; color: var(--text-muted); margin-top: 32px;">Ketik nama plugin untuk mencari.</div>`;
      pluginSearchInput.value = "";
    });

    closePluginBtn.addEventListener("click", () => {
      pluginModal.style.display = "none";
    });

    pluginSearchBtn.addEventListener("click", async () => {
      let query = pluginSearchInput.value.trim();
      let version = serverVersionSelect.value;
      if (!query || !version) return;

      pluginResults.innerHTML = `<div style="text-align: center; color: var(--text-muted); margin-top: 32px;">Mencari...</div>`;

      try {
        let res = await fetch(`https://api.modrinth.com/v2/search?query=${encodeURIComponent(query)}&facets=[["project_type:plugin"],["versions:${version}"]]&limit=15`);
        let data = await res.json();

        let installedPlugins = await invoke("get_server_plugins", { version: version });

        if (data.hits.length === 0) {
          pluginResults.innerHTML = `<div style="text-align: center; color: var(--text-muted); margin-top: 32px;">Plugin tidak ditemukan.</div>`;
          return;
        }

        pluginResults.innerHTML = "";

        // Use grid-2 for plugin manager as well
        let grid = document.createElement("div");
        grid.className = "grid-2";
        pluginResults.appendChild(grid);

        for (let hit of data.hits) {
          let card = document.createElement("div");
          card.className = "news-card panel";

          let downloads = hit.downloads >= 1000000 ? (hit.downloads / 1000000).toFixed(1) + "M" :
            hit.downloads >= 1000 ? (hit.downloads / 1000).toFixed(1) + "K" : hit.downloads;

          // Basic checking if installed (Modrinth slug or title as filename)
          let isInstalled = installedPlugins.some(p => p.toLowerCase().includes(hit.slug.toLowerCase()) || p.toLowerCase().includes(hit.title.toLowerCase().replace(/ /g, "_")));

          card.innerHTML = `
            <div style="display: flex; gap: 12px; align-items: start; margin-bottom: 12px;">
              <img src="${hit.icon_url || 'https://mc-heads.net/avatar/Steve/64'}" width="48" height="48" style="border-radius: 8px;" />
              <div>
                <h4 style="margin-bottom: 4px; font-size: 15px;">${hit.title}</h4>
                <p style="font-size: 11px; color: var(--accent); margin-bottom: 4px;">by ${hit.author}</p>
                <p style="font-size: 12px;">${hit.description.substring(0, 50)}...</p>
              </div>
            </div>
            <div style="display: flex; justify-content: space-between; font-size: 11px; color: var(--text-muted); margin-bottom: 12px; font-weight: 600;">
              <span style="background: #eef2ff; color: var(--accent); padding: 2px 6px; border-radius: 4px;">🎯 Server Plugin</span>
              <span>📥 ${downloads}</span>
            </div>
            <button class="btn ${isInstalled ? '' : 'btn-primary'} plugin-install-btn" style="width: 100%; ${isInstalled ? 'background: var(--success); color: white; border: none;' : ''}" ${isInstalled ? 'disabled' : ''}>${isInstalled ? '✓ Terpasang' : 'Install'}</button>
          `;

          let btn = card.querySelector(".plugin-install-btn");
          if (!isInstalled) {
            btn.onclick = async () => {
              btn.textContent = "Loading...";
              btn.disabled = true;
              try {
                let verRes = await fetch(`https://api.modrinth.com/v2/project/${hit.project_id}/version?game_versions=["${version}"]`);
                let verData = await verRes.json();
                if (verData.length > 0) {
                  let file = verData[0].files[0];
                  await invoke("download_server_plugin", {
                    version: version,
                    downloadUrl: file.url,
                    fileName: file.filename
                  });
                  btn.textContent = "✓ Terpasang";
                  btn.style.background = "var(--success)";
                  btn.className = "btn"; // remove btn-primary
                  btn.style.color = "white";
                  btn.style.border = "none";
                } else {
                  btn.textContent = "Error";
                  alert("Tidak ada file untuk versi ini.");
                  btn.disabled = false;
                }
              } catch (e) {
                btn.textContent = "Error";
                alert("Gagal menginstall: " + e);
                btn.disabled = false;
              }
            };
          }

          grid.appendChild(card);
        }
      } catch (e) {
        pluginResults.innerHTML = `<div style="text-align: center; color: var(--danger); margin-top: 32px;">Gagal mencari: ${e.message}</div>`;
      }
    });
  }

  // Console Input Logic
  let serverConsoleInput = document.querySelector("#server-console-input");
  let sendConsoleBtn = document.querySelector("#send-console-btn");

  const sendConsoleCmd = async () => {
    let cmd = serverConsoleInput.value.trim();
    if (!cmd) return;
    try {
      await invoke("send_console_command", { command: cmd });
      serverConsoleInput.value = "";
    } catch (e) {
      alert("Gagal mengirim command: " + e);
    }
  };

  if (sendConsoleBtn && serverConsoleInput) {
    sendConsoleBtn.addEventListener("click", sendConsoleCmd);
    serverConsoleInput.addEventListener("keydown", (e) => {
      if (e.key === "Enter") sendConsoleCmd();
    });
  }

  // Server Properties Logic
  let openPropsBtn = document.querySelector("#open-server-props-btn");
  let closePropsBtn = document.querySelector("#close-props-modal-btn");
  let propsModal = document.querySelector("#server-props-modal");
  let propsList = document.querySelector("#server-props-list");
  let savePropsBtn = document.querySelector("#save-props-btn");
  let currentProps = [];

  if (openPropsBtn && propsModal) {
    openPropsBtn.addEventListener("click", async () => {
      let version = serverVersionSelect.value;
      if (!version) {
        alert("Pilih versi server terlebih dahulu.");
        return;
      }
      propsModal.style.display = "flex";
      propsList.innerHTML = `<div style="text-align: center; color: var(--text-muted); margin-top: 32px;">Memuat konfigurasi...</div>`;

      try {
        currentProps = await invoke("get_server_properties", { version });
        if (currentProps.length === 0) {
          propsList.innerHTML = `<div style="text-align: center; color: var(--text-muted); margin-top: 32px;">File server.properties belum ada. Harap nyalakan server sekali.</div>`;
          return;
        }

        propsList.innerHTML = "";

        let grid = document.createElement("div");
        grid.className = "grid-2";
        grid.style.gap = "12px";

        currentProps.forEach(([key, val], index) => {
          let card = document.createElement("div");
          card.style.cssText = "display: flex; flex-direction: column; gap: 8px; padding: 12px; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; transition: all 0.2s;";

          // hover effect simulation by adding a class or just inline
          card.onmouseenter = () => card.style.background = "rgba(255, 255, 255, 0.06)";
          card.onmouseleave = () => card.style.background = "rgba(255, 255, 255, 0.03)";

          let label = document.createElement("label");
          label.textContent = key;
          label.style.fontSize = "11px";
          label.style.fontWeight = "700";
          label.style.color = "#a5b4fc"; // accent color
          label.style.textTransform = "uppercase";
          label.style.letterSpacing = "0.5px";
          label.style.cursor = "default";

          let input = document.createElement("input");
          input.type = "text";
          input.className = "input-box";
          input.value = val;
          input.style.fontSize = "13px";
          input.style.padding = "8px 10px";
          input.style.background = "rgba(0, 0, 0, 0.3)";
          input.style.border = "1px solid rgba(255, 255, 255, 0.1)";
          input.style.color = "#fff";
          input.style.width = "100%";
          input.dataset.key = key;
          input.dataset.index = index;

          // Input focus effects
          input.onfocus = () => {
            input.style.borderColor = "var(--primary)";
            input.style.background = "rgba(0, 0, 0, 0.5)";
          };
          input.onblur = () => {
            input.style.borderColor = "rgba(255, 255, 255, 0.1)";
            input.style.background = "rgba(0, 0, 0, 0.3)";
          };

          card.appendChild(label);
          card.appendChild(input);
          grid.appendChild(card);
        });

        propsList.appendChild(grid);
      } catch (e) {
        propsList.innerHTML = `<div style="text-align: center; color: var(--danger); margin-top: 32px;">Gagal memuat properti: ${e}</div>`;
      }
    });

    closePropsBtn.addEventListener("click", () => {
      propsModal.style.display = "none";
    });

    savePropsBtn.addEventListener("click", async () => {
      let version = serverVersionSelect.value;
      if (!version) return;

      let inputs = propsList.querySelectorAll("input");
      let updatedProps = [];
      inputs.forEach(input => {
        updatedProps.push([input.dataset.key, input.value]);
      });

      let oldText = savePropsBtn.textContent;
      savePropsBtn.textContent = "Menyimpan...";
      savePropsBtn.disabled = true;
      try {
        await invoke("save_server_properties", { version, properties: updatedProps });
        savePropsBtn.textContent = "Tersimpan ✓";
        savePropsBtn.style.background = "var(--success)";
      } catch (e) {
        alert("Gagal menyimpan: " + e);
        savePropsBtn.textContent = "Error";
      }

      setTimeout(() => {
        savePropsBtn.textContent = oldText;
        savePropsBtn.disabled = false;
        savePropsBtn.style.background = "var(--primary)";
      }, 2000);
    });
  }

  launchBtn.addEventListener("click", async (e) => {
    e.preventDefault();
    launchGame();
  });
  // --- SETTINGS LOGIC ---
  let globalSettings = null;

  async function loadSettings() {
    try {
      globalSettings = await invoke("get_settings");

      document.querySelector("#settings-theme").value = globalSettings.theme || "light";
      if (document.querySelector("#settings-elyby")) {
        document.querySelector("#settings-elyby").checked = globalSettings.use_elyby !== false; // Default true if null
      }
      document.querySelector("#settings-ram-max").value = globalSettings.ram_max || 4096;
      document.querySelector("#settings-res-width").value = globalSettings.res_width || 854;
      document.querySelector("#settings-res-height").value = globalSettings.res_height || 480;

      document.querySelector("#settings-game-dir").value = globalSettings.custom_game_dir || "";
      document.querySelector("#settings-server-dir").value = globalSettings.custom_server_dir || "";
      document.querySelector("#settings-java-path").value = globalSettings.custom_java_path || "";

      applyTheme(globalSettings.theme || "light");
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  }

  function applyTheme(theme) {
    if (theme === "dark") {
      document.body.classList.add("theme-dark");
    } else {
      document.body.classList.remove("theme-dark");
    }
  }

  let saveSettingsBtn = document.querySelector("#save-settings-btn");
  if (saveSettingsBtn) {
    saveSettingsBtn.addEventListener("click", async () => {
      if (!globalSettings) return;

      let latestSettings = await invoke("get_settings");

      latestSettings.theme = document.querySelector("#settings-theme").value;
      if (document.querySelector("#settings-elyby")) {
        latestSettings.use_elyby = document.querySelector("#settings-elyby").checked;
      }

      let ramValue = parseInt(document.querySelector("#settings-ram-max").value) || 4096;
      latestSettings.ram_min = ramValue;
      latestSettings.ram_max = ramValue;
      latestSettings.res_width = parseInt(document.querySelector("#settings-res-width").value) || 854;
      latestSettings.res_height = parseInt(document.querySelector("#settings-res-height").value) || 480;

      latestSettings.custom_game_dir = document.querySelector("#settings-game-dir").value.trim() || null;
      latestSettings.custom_server_dir = document.querySelector("#settings-server-dir").value.trim() || null;

      try {
        await invoke("save_settings", { settings: latestSettings });
        globalSettings = latestSettings;
        applyTheme(globalSettings.theme);
        alert("Pengaturan berhasil disimpan!");
      } catch (e) {
        alert("Gagal menyimpan pengaturan: " + e);
      }
    });
  }

  // Folder Pickers
  let pickGameDirBtn = document.querySelector("#btn-pick-game-dir");
  if (pickGameDirBtn) {
    pickGameDirBtn.addEventListener("click", async () => {
      try {
        const selected = await invoke('plugin:dialog|open', { options: { directory: true, multiple: false } });
        if (selected) {
          document.querySelector("#settings-game-dir").value = selected;
        }
      } catch (e) {
        console.error("Folder picker error:", e);
      }
    });
  }

  let pickServerDirBtn = document.querySelector("#btn-pick-server-dir");
  if (pickServerDirBtn) {
    pickServerDirBtn.addEventListener("click", async () => {
      try {
        const selected = await invoke('plugin:dialog|open', { options: { directory: true, multiple: false } });
        if (selected) {
          document.querySelector("#settings-server-dir").value = selected;
        }
      } catch (e) {
        console.error("Folder picker error:", e);
      }
    });
  }

  // Initial theme apply
  invoke("get_settings").then(s => {
    if (s && s.theme) applyTheme(s.theme);
  });

  // Make loadSettings global if needed by nav logic
  window.loadSettings = loadSettings;

  // --- SCREENSHOT MANAGER ---
  async function initScreenshotsView() {
    let select = document.getElementById("screenshot-instance-select");
    if (!select) return;

    // Populate dropdown
    try {
      let instances = await invoke("get_instances");
      let activeId = select.value || currentActiveInstance?.id || (instances.length > 0 ? instances[0].id : "");

      select.innerHTML = '<option value="">-- Pilih Instance --</option>';
      instances.forEach(inst => {
        let opt = document.createElement("option");
        opt.value = inst.id;
        opt.textContent = `${inst.name} (${inst.version})`;
        if (inst.id === activeId) opt.selected = true;
        select.appendChild(opt);
      });

      select.onchange = () => loadScreenshots(select.value);

      let btnFolder = document.getElementById("btn-open-screenshot-folder");
      btnFolder.onclick = () => {
        let id = select.value;
        if (id) {
          invoke("open_screenshot_folder", { instanceId: id }).catch(e => alert("Gagal membuka folder: " + e));
        }
      };

      if (activeId) {
        select.value = activeId;
        loadScreenshots(activeId);
      }
    } catch (e) {
      console.error("Failed to load instances for screenshots", e);
    }
  }

  async function loadScreenshots(instanceId) {
    let grid = document.getElementById("screenshots-grid");
    let empty = document.getElementById("screenshots-empty");
    if (!grid || !empty) return;

    if (!instanceId) {
      grid.innerHTML = "";
      empty.style.display = "block";
      return;
    }

    try {
      let screenshots = await invoke("get_screenshots", { instanceId });

      if (screenshots.length === 0) {
        grid.innerHTML = "";
        empty.style.display = "block";
        return;
      }

      empty.style.display = "none";
      grid.innerHTML = "";

      for (let sc of screenshots) {
        let card = document.createElement("div");
        card.className = "panel";
        card.style.cssText = "padding: 12px; display: flex; flex-direction: column; gap: 8px;";

        let imgContainer = document.createElement("div");
        imgContainer.style.cssText = "width: 100%; aspect-ratio: 16/9; background: #000; border-radius: 8px; overflow: hidden; display: flex; align-items: center; justify-content: center; position: relative;";

        let img = document.createElement("img");
        img.style.cssText = "width: 100%; height: 100%; object-fit: contain; opacity: 0.5; transition: opacity 0.3s;";

        let spinner = document.createElement("div");
        spinner.textContent = "⏳ Memuat...";
        spinner.style.cssText = "position: absolute; color: white; font-size: 12px;";

        imgContainer.appendChild(spinner);
        imgContainer.appendChild(img);

        let info = document.createElement("div");
        info.style.cssText = "display: flex; justify-content: space-between; align-items: center;";

        let name = document.createElement("div");
        name.style.cssText = "font-size: 12px; font-weight: 600; text-overflow: ellipsis; overflow: hidden; white-space: nowrap; max-width: 180px;";
        name.textContent = sc.filename;

        let delBtn = document.createElement("button");
        delBtn.className = "btn";
        delBtn.style.cssText = "background: #ffebee; color: var(--danger); border: none; padding: 4px 8px; font-size: 11px;";
        delBtn.innerHTML = "🗑️";
        delBtn.title = "Hapus Screenshot";

        delBtn.onclick = async () => {
          if (confirm(`Yakin ingin menghapus ${sc.filename}?`)) {
            try {
              await invoke("delete_screenshot", { instanceId, filename: sc.filename });
              loadScreenshots(instanceId); // reload
            } catch (e) {
              alert("Gagal menghapus: " + e);
            }
          }
        };

        info.appendChild(name);
        info.appendChild(delBtn);

        card.appendChild(imgContainer);
        card.appendChild(info);
        grid.appendChild(card);

        // Lazy load Base64
        invoke("get_screenshot_base64", { instanceId, filename: sc.filename })
          .then(b64 => {
            img.src = b64;
            img.onload = () => {
              spinner.style.display = "none";
              img.style.opacity = "1";
            };
            img.onclick = () => {
              // Open full size in new window/tab or simple modal
              let w = window.open("");
              w.document.write(`<body style="margin:0;background:#111;display:flex;align-items:center;justify-content:center;height:100vh;"><img src="${b64}" style="max-width:100%;max-height:100%;object-fit:contain;"></body>`);
            };
            img.style.cursor = "pointer";
          })
          .catch(e => {
            spinner.textContent = "❌ Gagal memuat";
          });
      }
    } catch (e) {
      grid.innerHTML = `<div style="color:var(--danger)">Gagal memuat screenshot: ${e}</div>`;
      empty.style.display = "none";
    }
  }

  // --- UPDATE CHECKER ---
  async function checkForUpdates() {
    try {
      const currentVersion = await invoke("get_app_version");
      const res = await fetch("https://api.github.com/repos/Nashdev0/ND-Launcher/releases/latest");
      if (!res.ok) {
        console.warn(`[Update Checker] GitHub API returned ${res.status}. Pastikan ada minimal satu Release di repo.`);
        return;
      }
      const data = await res.json();

      let latestVersion = data.tag_name;
      if (latestVersion.startsWith("v")) latestVersion = latestVersion.substring(1);

      // Helper to compare semver (e.g., 0.0.4 > 0.0.3)
      const isNewer = (latest, current) => {
        let l = latest.split('.').map(Number);
        let c = current.split('.').map(Number);
        for (let i = 0; i < Math.max(l.length, c.length); i++) {
          let valL = l[i] || 0;
          let valC = c[i] || 0;
          if (valL > valC) return true;
          if (valL < valC) return false;
        }
        return false;
      };

      if (isNewer(latestVersion, currentVersion)) {
        document.getElementById("update-current-version").textContent = currentVersion;
        document.getElementById("update-new-version").textContent = latestVersion;

        let bodyHtml = (data.body || "Tidak ada changelog yang disediakan.").replace(/\r\n/g, "<br>").replace(/\n/g, "<br>");
        document.getElementById("update-changelog").innerHTML = bodyHtml;

        document.getElementById("update-notification").style.display = "block";

        if (data.html_url) {
          document.getElementById("btn-download-update").href = data.html_url;
        }

        document.getElementById("close-update-btn").onclick = () => {
          document.getElementById("update-notification").style.display = "none";
        };
      }
    } catch (e) {
      console.error("Update check failed:", e);
    }
  }

  // Call it a few seconds after startup
  setTimeout(checkForUpdates, 3000);
});
