# Plan: Fix Ely.by API Overload — Avatar Caching

## Masalah
`fetchAccounts()` dipanggil 4+ kali saat startup + setiap account switch/add/delete. Setiap panggilan langsung nembak 2-3 request ke Ely.by API:
1. `POST /api/profiles/minecraft` → ambil UUID
2. `GET /sessionserver/session/minecraft/profile/{uuid}` → ambil texture
3. (optional) download skin image

Total bisa **10+ request** dalam hitungan detik ke Ely.by → server timeout/unresponsive.

## Solusi: Client-Side Cache + Promise Deduplication

Tambahkan cache di frontend (`src/main.js`) agar setiap username hanya fetch sekali.

### Perubahan File: `src/main.js`

#### 1. Tambah variabel global (baris ~11, setelah `currentActiveAccount`):
```javascript
let elybyAvatarCache = {};     // username → data URL
let elybyAvatarPromises = {};  // username → promise aktif (dedup)
```

#### 2. Tambah helper function (sebelum `async function fetchAccounts()`):
```javascript
async function getCachedElybyAvatar(username) {
  // Langsung return kalau sudah di-cache
  if (elybyAvatarCache[username]) {
    return elybyAvatarCache[username];
  }
  // Reuse promise yang sedang berjalan (dedup)
  if (elybyAvatarPromises[username]) {
    return elybyAvatarPromises[username];
  }
  // Fetch pertama kali
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
```

#### 3. Ganti baris 207-209 di `fetchAccounts()`:
```javascript
// OLD:
const skinDataUrl = await invoke("get_elyby_head_data_url", {
  username: currentActiveAccount.username,
});

// NEW:
const skinDataUrl = await getCachedElybyAvatar(currentActiveAccount.username);
```

### Hasil
| Sebelum | Sesudah |
|----------|---------|
| 4 akun × 2-3 request = 8-12 request/API call | 4 akun × 1 request = **4 request total** |
| Setiap switch account = fetch ulang | Switch = langsung pakai cache |
| Server Ely.by overloaded | Server tenang |

### Clear Cache (opsional)
Tambahkan fungsi `clearElybyAvatarCache()` jika user ingin refresh avatar — dipanggil hanya saat user mengganti skin manual.

## File yang Diubah
- `src/main.js` saja (3 perubahan kecil: variabel global, helper function, 1 baris invoke)
