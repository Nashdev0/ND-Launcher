# Plan: Skin Upload UI di Launcher

## Goal
User bisa upload skin langsung dari launcher tanpa buka Ely.by. Avatar otomatis update ke skin yang diupload.

## Backend (SUDAH ADA — tidak perlu ubah)
- `copy_local_skin(username, source_path)` — simpan PNG ke `game_data/skins/{username}.png` ✅
- `get_local_skin_data(username)` — baca bytes skin lokal ✅
- Tauri dialog plugin sudah tersedia (`plugin:dialog|open`) ✅

## Perubahan File

### 1. `src/index.html` — Tambah tombol Upload Skin

Di area account manager, setelah tombol "Hapus Akun", tambahkan:
```html
<button class="btn btn-outline" id="btn-upload-skin"
    style="width: 100%; font-size: 13px; margin-top: 4px;">👕 Upload Skin</button>
```

Posisi: antara `delete-account-btn` dan `show-add-account-btn`.

### 2. `src/main.js` — Event listener + preview avatar

**a. Tambah variable** (baris ~12):
```javascript
let uploadSkinBtn;
```

**b. Tambah fungsi updateAvatarFromLocalSkin(username)** — fire-and-forget, ambil bytes lokal, crop head pakai canvas:
```javascript
async function updateAvatarFromLocalSkin(username) {
  try {
    const skinBytes = await invoke("get_local_skin_data", { username });
    if (!skinBytes || skinBytes.length === 0) {
      avatarImg.src = `https://mc-heads.net/head/${username}/64`;
      return;
    }
    const base64 = btoa(String.fromCharCode(...new Uint8Array(skinBytes)));
    const dataUrl = `data:image/png;base64,${base64}`;
    const img = new Image();
    img.onload = () => {
      const c = document.createElement("canvas");
      c.width = 64; c.height = 64;
      const ctx = c.getContext("2d");
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(img, 8, 8, 8, 8, 0, 0, 64, 64);
      avatarImg.src = c.toDataURL();
    };
    img.src = dataUrl;
  } catch (e) {
    avatarImg.src = `https://mc-heads.net/head/${username}/64`;
  }
}
```

**c. Tambah event listener** (di DOMContentLoaded, setelah deleteAccountBtn setup):
```javascript
uploadSkinBtn = document.querySelector("#btn-upload-skin");
if (uploadSkinBtn) {
  uploadSkinBtn.addEventListener("click", async () => {
    if (!currentActiveAccount) {
      alert("Pilih akun dulu sebelum upload skin!");
      return;
    }
    try {
      const selected = await invoke('plugin:dialog|open', {
        options: { filters: [{ name: 'PNG', extensions: ['png'] }] }
      });
      if (selected) {
        await invoke("copy_local_skin", {
          username: currentActiveAccount.username,
          source_path: selected
        });
        alert(`Skin berhasil disimpan untuk ${currentActiveAccount.username}!`);
        updateAvatarFromLocalSkin(currentActiveAccount.username);
      }
    } catch (e) {
      console.error("Gagal upload skin:", e);
      alert("Gagal upload skin: " + (e.message || e));
    }
  });
}
```

## Flow
1. User klik "Upload Skin"
2. File dialog terbuka (hanya PNG)
3. User pilih file → `copy_local_skin` simpan ke `game_data/skins/{username}.png`
4. `get_local_skin_data` baca bytes
5. Canvas crop area kepala (8x8 offset 8,8) → resize ke 64x64
6. Avatar img.src diupdate
7. Avatar muncul skin user, bukan Steve

## Hasil Akhir
- User tidak perlu buka Ely.by sama sekali
- Avatar launcher sesuai skin lokal
- Skin tetap dipakai game saat launch (launcher.rs sudah copy skin ke player textures)
- API Ely.by sama sekali tidak dipanggil

## Files Changed
- `src/index.html`: +1 tombol
- `src/main.js`: +1 variable, +1 function, +1 event listener (~30 baris)
