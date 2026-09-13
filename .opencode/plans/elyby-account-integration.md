# Plan: Ely.by Account Integration di Launcher

## Research Result

Setelah investigasi mendalam ke Ely.by:

### ✅ Yang Tersedia (Public API)
- `POST /api/profiles/minecraft` → ambil UUID username
- `GET /sessionserver/session/minecraft/profile/{uuid}` → ambil texture/skin data
- **Tidak perlu password** untuk akses read-only

### ❌ Yang Tidak Tersedia
- API untuk upload skin (harus via web UI)
- API untuk manage account (harus via web UI)
- API untuk login dengan credentials (hanya Minecraft auth)

## Solusi: Ely.by Account Panel (Read-Only + Direct Link)

Karena tidak ada API untuk upload skin, solusi terbaik adalah:
1. Tampilkan profil Ely.by user DI DALAM launcher
2. User bisa lihat UUID, skin preview, status akun
3. Link langsung ke halaman manage skin di ely.by
4. Cache hasil query untuk hindari rate limit

## Perubahan File

### 1. `src-tauri/src/settings.rs` — Tambah command baru

**Command:** `get_elyby_profile(username)` → return JSON profile lengkap:
```rust
#[derive(Serialize)]
struct ElybyProfile {
    uuid: String,
    name: String,
    skin_url: Option<String>,
    has_custom_skin: bool,
}
```

Implementasi:
- POST ke `/api/profiles/minecraft` → ambil UUID
- GET session profile → decode base64 textures
- Extract skin URL dari JSON
- Return struct di atas

**Register di** `lib.rs:26-66` dalam `generate_handler![...]`

### 2. `src/index.html` — Tambah Ely.by Account Section

Di Settings page, ganti section Ely.by yang sekarang (baris 248-271) dengan:

```html
<div id="elyby-account-section" style="margin-top: 16px;">
  <h4 style="font-size: 13px; font-weight: 600; margin-bottom: 8px;">🧑‍💻 Akun Ely.by Aktif</h4>
  
  <div id="elyby-profile-loading" style="font-size: 12px; color: var(--text-muted);">
    Memuat profil...
  </div>
  
  <div id="elyby-profile-content" style="display: none;">
    <div style="display: flex; gap: 12px; align-items: center; margin-bottom: 12px;">
      <img id="elyby-avatar" src="" width="64" height="64" 
           style="border-radius: 8px; border: 2px solid var(--accent);" />
      <div>
        <div style="font-size: 14px; font-weight: 600;" id="elyby-username"></div>
        <div style="font-size: 11px; color: var(--text-muted);" id="elyby-uuid"></div>
        <div style="font-size: 11px; margin-top: 4px;" id="elyby-skin-status"></div>
      </div>
    </div>
    
    <div style="font-size: 12px; color: var(--text-muted); margin-bottom: 8px;">
      📋 Skin saat ini:
    </div>
    <img id="elyby-skin-preview" src="" width="64" height="64" 
         style="border-radius: 4px; image-rendering: pixelated; border: 1px solid var(--border-color);" />
    
    <div style="margin-top: 12px; display: flex; gap: 8px;">
      <button class="btn btn-outline" id="btn-manage-elyby" style="flex: 1; font-size: 12px;">
        🔗 Kelola di Ely.by
      </button>
      <button class="btn btn-outline" id="btn-refresh-elyby" style="flex: 1; font-size: 12px;">
        🔄 Refresh
      </button>
    </div>
  </div>
  
  <div id="elyby-profile-error" style="display: none; font-size: 12px; color: #ff5252;">
    Gagal memuat profil Ely.by
  </div>
</div>
```

### 3. `src/main.js` — Update display profile

Tambahkan fungsi baru:

```javascript
// Cache profil Ely.by
let elybyProfileCache = {};
let elybyProfilePromise = null;

async function getElybyProfile(username) {
  // Return cache jika ada
  if (elybyProfileCache[username]) {
    return elybyProfileCache[username];
  }
  // Dedup promise
  if (elybyProfilePromise) return elybyProfilePromise;
  
  const promise = (async () => {
    try {
      const profile = await invoke("get_elyby_profile", { username });
      elybyProfileCache[username] = profile;
      return profile;
    } catch (e) {
      console.error("Failed to get Ely.by profile:", e);
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
  
  const loading = document.querySelector("#elyby-profile-loading");
  const content = document.querySelector("#elyby-profile-content");
  const error = document.querySelector("#elyby-profile-error");
  
  loading.style.display = "block";
  content.style.display = "none";
  error.style.display = "none";
  
  const profile = await getElybyProfile(currentActiveAccount.username);
  
  if (profile) {
    document.querySelector("#elyby-username").textContent = profile.name;
    document.querySelector("#elyby-uuid").textContent = profile.uuid;
    
    if (profile.has_custom_skin) {
      document.querySelector("#elyby-skin-status").textContent = "✅ Skin kustom aktif";
      document.querySelector("#elyby-skin-status").style.color = "var(--success)";
    } else {
      document.querySelector("#elyby-skin-status").textContent = "⚪ Menggunakan skin default";
      document.querySelector("#elyby-skin-status").style.color = "var(--text-muted)";
    }
    
    // Avatar
    const skinDataUrl = await getCachedElybyAvatar(currentActiveAccount.username);
    document.querySelector("#elyby-avatar").src = skinDataUrl;
    
    // Skin preview (full skin image, not cropped)
    if (profile.skin_url) {
      document.querySelector("#elyby-skin-preview").src = profile.skin_url;
    }
    
    loading.style.display = "none";
    content.style.display = "block";
  } else {
    loading.style.display = "none";
    error.style.display = "block";
  }
}

// Event listeners
document.querySelector("#btn-manage-elyby")?.addEventListener("click", () => {
  if (window.__TAURI__) {
    window.__TAURI__.core.invoke("plugin:opener|open", { path: "https://ely.by" });
  }
});

document.querySelector("#btn-refresh-elyby")?.addEventListener("click", async () => {
  elybyProfileCache = {}; // Clear cache
  await updateElybyProfileDisplay();
});
```

Panggil `updateElybyProfileDisplay()` di dalam `fetchAccounts()` setelah render avatar.

### 4. Hapus checkbox `use_elyby` (opsional)

Karena sekarang kita fetch skin secara otomatis berdasarkan username, checkbox `use_elyby` bisa dihapus atau diganti jadi toggle untuk force-refresh cache.

## Flow Setelah Implementasi

1. User buka launcher → login dengan username Ely.by mereka
2. Otomatis muncul profil Ely.by di settings (UUID, avatar, status skin)
3. User klik "Kelola di Ely.by" → browser terbuka ke ely.by (untuk upload skin)
4. Setelah upload skin di website, user klik "Refresh" di launcher
5. Avatar launcher update sesuai skin baru

## Rate Limit Protection

- Cache profil di memory (`elybyProfileCache`)
- Promise deduplication (`elybyProfilePromise`)
- Hanya fetch sekali per username
- Tombol "Refresh" untuk manual refresh jika diperlukan

## Files Changed

| File | Perubahan |
|------|-----------|
| `src-tauri/src/settings.rs` | +~40 baris (command baru) |
| `src-tauri/src/lib.rs` | +1 baris (register command) |
| `src/index.html` | ~30 baris (HTML section baru) |
| `src/main.js` | ~50 baris (fungsi + event listener) |

Total: +~120 baris
