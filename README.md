# 🎮 ND Launcher

**Minecraft Offline Launcher** — buat kamu yang mau main tanpa ribet. Multi-instance, mod pack manager, server Purpur lokal, dan integrasi skin Ely.by langsung di dalam app.

Dibangun dengan **Tauri v2** (Rust backend + vanilla JS frontend) — ringan, cepat, dan aman.

---

## ✨ Kenapa ND Launcher?

| | Launcher Biasa | ND Launcher |
|---|---|---|
| **Multi-Instance** | ❌ Tidak ada | ✅ Hingga 5 instance terpisah |
| **Mod Download** | Manual satu-satu | ✅ Cari & install langsung dari UI |
| **Server Lokal** | Harus command line | ✅ Start/stop + SSH tunneling |
| **Crash Report** | Baca manual | ✅ Analisis otomatis |
| **Skin System** | Upload ke website | ✅ Otomatis dari akun Ely.by |

---

## 🚀 Fitur Utama

### 📦 Multi-Instance Manager
Buat beberapa dunia Minecraft dengan setup berbeda — masing-masing punya mod, shader, dan config sendiri. Maksimum 5 instance aktif.

### 🔧 Fabric Loader Auto-Inject
Pilih versi Minecraft → launcher otomatis download Fabric loader, injeksi library, dan siap main. Tanpa konfigurasi manual.

### 🟢 Modrinth Integration
Cari ribuan mod langsung dari dalam app. Pilih versi, install, dan aktifkan — semua tanpa buka browser.

### 🖥️ Purpur Server Hub
Jalankan server Minecraft sendiri di komputer lokal. Dengan **SSH tunneling** via pinggy.io, server-mu bisa diakses teman dari internet.

### 🧑‍💻 Ely.by Skin System
Login pakai username Ely.by → avatar skin muncul otomatis di launcher. UUID deterministik, texture langsung diterapkan saat main.

### 🛡️ Crash Analyzer
Game crash? Launcher otomatis baca crash report terbaru dan beri kamu ringkasan masalahnya — bukan error code kosong.

### ⚡ Java Auto-Detect
Launcher pilihkan JDK yang tepat untuk version Minecraft kamu:
- 1.17 – 1.20.4 → Java 17
- 1.20.5 – 1.21.1 → Java 21
- 1.21.2+ / 26.x → Java 25

Auto-download dari Adoptium jika belum terinstall.

---

## 📸 Tampilan

```
┌──────────────┬──────────────────────────────┬────────────────────┐
│  LOGO        │  MAIN WORKSPACE              │  SERVER HUB      │
│  ND Launcher │                              │                    │
│              │  ┌────────────────────────┐  │  Purpur Server   │
│  📰 News     │  │  [Multi-Instance]  │  │  Status: RUNNING │
│  📦 Instansi │  │  Instance A (ON)   │  │  [Stop]        │
│  🟢 Modrinth │  │  Instance B (OFF)  │  │  [Console]     │
│  ✨ Shaders  │  └────────────────────────┘  │  [Tunnel]      │
│  📸 Screenshot│                               │                    │
│  ⚙️ Settings │  Avt: [skin]  Minecraft 1.21│  IP: tcp://...   │
│              │         RAM: 4GB  ▶ PLAY   │                    │
└──────────────┴──────────────────────────────┴────────────────────┘
```

---

## 📥 Download

### Windows (Release)
[**Unduh Versi Terbaru**](https://github.com/Nashdev0/ND-Launcher/releases/latest)
- Format: `.exe` (NSIS Installer) atau `.msi`
- Ukuran: ~5MB

---

## 🗂️ Data Penyimpanan

Semua data ada di folder `ND Launcher/`:

```
C:\ND Launcher\                Windows
~/ND Launcher/                 Linux / macOS
```

| Folder | Isi |
|--------|-----|
| `game_data/instances/` | Mod, saves, shaderpack per instance |
| `game_data/java/` | JDK otomatis (17, 21, 25) |
| `game_data/skins/` | Avatar skin cache |
| `server/` | File server Purpur per versi |

---

## 🔄 Changelog

### v0.0.6 — Bug Hunt Menyeluruh
- Fix crash Modrinth saat deskripsi mod kosong
- Fix kebocoran profil Ely.by antar akun
- Fix alokasi RAM yang tertimpa saat launch
- Backend anti-panic (mutex recovery, error handling)
- Folder server kustom jadi konsisten
- Changelog GitHub disanitasi (anti-injeksi)
- Runtime data tidak lagi ikut ter-commit

### v0.0.5 — Ely.by Integration
- Panel profil Ely.by otomatis (avatar, UUID, status skin)
- Avatar skin tampil langsung di launcher
- Cache API + deduplication prevent spam
- Server console placeholder centered
- 8 bug fixes & optimasi performa

### v0.0.4 — Skin System
- Akun offline bisa pakai skin Ely.by
- Skin otomatis apply saat launch game

### v0.0.3 — Modrinth Downloader Pro
- Pilih versi spesifik mod
- Re-install otomatis hapus mod lama

---

## 💡 Cara Pakai

1. **Download** launcher dari link di atas
2. **Install** dan buka aplikasinya
3. **Tambah akun** — masukkan username offline kamu
4. **Pilih instance** dan versi Minecraft yang diinginkan
5. **Klik "Launch"** — launcher akan download semua yang dibutuhkan
6. **Nikmati!**

Untuk skin kustom, login dengan akun **Ely.by** di menu Settings.

---

## 📄 Lisensi

Private project — **ND Launcher Demo**. Dibuat oleh Nashdev0.

---

<p align="center">
  Made with ❤️ using <a href="https://tauri.app">Tauri</a> & <a href="https://www.rust-lang.org">Rust</a>
</p>
