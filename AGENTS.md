# AGENTS.md - ND Launcher Demo v0.0.5

Tauri v2 app: Rust backend + vanilla HTML/CSS/JS frontend. Minecraft launcher offline dengan Fabric loader, Purpur server hub, Modrinth mod downloader, crash analyzer.

Frontend: `src/main.js` (1967 baris, tanpa framework). Backend: `src-tauri/src/` - modules: `launcher.rs`, `settings.rs`, `java.rs`, `server.rs`, `screenshot.rs`, `minecraft.rs`.

## Perintah

```bash
npm install                                      # install deps
npm run tauri dev                                # development mode (Rust watch + frontend)
npm run tauri build                              # release build (LTO, strip, lambat tapi binary kecil)
```

CI: Windows-only (`build-windows.yml`). Output NSIS `.exe` + MSI di `src-tauri/target/release/bundle/`. Linux/macOS build manual.

Tidak ada linter, formatter, typechecker, atau test suite. Tidak ada build tool frontend. Satu-satunya script npm: `tauri`.

## Data Layout

Base dir (`get_base_dir()` di `settings.rs:78-91`):
- Windows: `C:\ND Launcher\`
- Linux/macOS: `$HOME/ND Launcher\`

```
ND Launcher/
  game_data/
    launcher_settings.json       ← semua settings (accounts, instances, RAM, dll)
    instances/{id}/mods/         ← mod (*.jar aktif, *.jar.disabled nonaktif)
    java/17/ 21/ 25/             ← JRE otomatis dari Adoptium
    skins/{username}.png         ← skin lokal
    assets/, versions/, libraries/
    server/{version}/            ← Purpur server files
  authlib-injector.jar           ← Ely.by (download on-demand saat launch)
```

**PENTING**: Screenshot path beda - simpan di `base_dir/instances/{id}/screenshots/` (TIDAK pakai `game_data/`). Lihat `screenshot.rs:14-16`.

## Quirks Kritis

| # | Quirk | Detail | Source |
|---|-------|--------|--------|
| 1 | **Max 5 instance** | Harus hapus instance lama dulu sebelum buat baru | `settings.rs:209-211` |
| 2 | **Auth offline** | UUID = `v3(nil, "OfflinePlayer:{username}")`, token `"0"`, userType `"legacy"` | `settings.rs:134`, `launcher.rs:594-613` |
| 3 | **Java → MC mapping** | 1.21.2+/26.x→Java 25; 1.20.5–1.21.1→Java 21; 1.17–1.20.4→Java 17; lain→Java 8 | `java.rs:170-187` |
| 4 | **Fabric injection** | Fetch meta dari `meta.fabricmc.net/v2/versions/loader/{ver}`, download ke `game_data/libraries/`, ganti main class | `launcher.rs:273-325` |
| 5 | **Server pakai `java` PATH** | BUKAN custom Java path dari settings - ini bug/quirk penting | `server.rs:89` |
| 6 | **Crash log** | Cek `crash-reports/` (file < 1 jam), fallback ke `logs/latest.log` | `settings.rs:451-495` |
| 7 | **Mod toggle** | Rename file `.jar` ↔ `.jar.disabled` - bukan edit konten | `settings.rs:411-429` |
| 8 | **Iris skip Sodium** | Dep resolver frontend skip Sodium (`AANobbMI`) kalau install Iris | `main.js:634` |
| 9 | **Crate type wajib tetap** | Jangan ubah `["staticlib", "cdylib", "rlib"]` - Tauri v2 requirement | `Cargo.toml:14-15` |
| 10 | **Frontend invoke pattern** | Semua panggil command Rust via `window.__TAURI__.core.invoke()`. Frontend hanya fetch langsung untuk: Modrinth API, GitHub API, Purpur API, Ely.by API | `main.js:1-5` |

## Cara Tambah Command Baru

Wajib 2 tempat (lupa salah satu = command tidak jalan):

1. Buat function `#[tauri::command]` di module terkait
2. Daftarkan di `src-tauri/src/lib.rs:26-66` dalam `generate_handler![...]`

Jika perlu shared state antar command, gunakan `.manage(MyState::default())` sebelum `.invoke_handler()` di builder.

## Event System (3 events)

| Event | Payload | Listener | Fungsi |
|-------|---------|----------|--------|
| `progress` | `{stage, message, current, total}` | `main.js:1329` | Update progress bar & status text |
| `game-log` | String | `main.js:1342` | Stream log game; deteksi `[SYSTEM] Game exited` → trigger crash analyzer |
| `server-log` | String | `main.js:1416` | Stream console output server |

## External APIs

| API | Tujuan | Dari |
|-----|--------|------|
| `piston-meta.mojang.com/mc/game/version_manifest_v2.json` | Minecraft versions | Rust |
| `meta.fabricmc.net/v2/versions/loader/{ver}` | Fabric loader info | Rust |
| `resources.download.minecraft.net/{hash}/{hash}` | Minecraft assets | Rust |
| `api.modrinth.com/v2/search` + `/project/{slug}/version` | Cari & install mod/plugin | JS frontend |
| `api.purpurmc.org/v2/purpur/` | Daftar versi Purpur | JS frontend |
| `api.purpurmc.org/v2/purpur/{ver}/latest/download` | Download Purpur jar | Rust |
| `authserver.ely.by/api/profiles/minecraft` | Ely.by UUID lookup | Rust |
| `authserver.ely.by/api/authlib-injector/sessionserver/...` | Ely.by texture lookup | Rust |
| `mc-heads.net/avatar/{user}/64` | Avatar fallback | JS frontend |
| `api.github.com/repos/Nashdev0/ND-Launcher/releases/latest` | Update checker | JS frontend |
| `github.com/yushijinhun/authlib-injector/releases/...` | Authlib-injector download | Rust |
| `api.adoptium.net/v3/binary/latest/{ver}/ga/{os}/{arch}/jre/hotspot/normal/eclipse` | Java download | Rust |
| SSH `tcp@a.pinggy.io:443` | Tunneling server ke internet | Rust |

## JVM Flags (Aikar's Optimization)

Diterapkan saat launch di `launcher.rs:563-585`:
```
-Xmx{ram_max}M -Xms{ram_min}M
-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200
-XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch
-XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1HeapRegionSize=8M
-XX:G1ReservePercent=20 -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4
-XX:InitiatingHeapOccupancyPercent=15 -XX:G1MixedGCLiveThresholdPercent=90
-XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32
-XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1
```

Windows: gunakan `javaw.exe` (tidak muncul window CMD). Linux/macOS: gunakan `java`.

## UI Structure

3 kolom + bottom dock:
- **Left sidebar** (220px): logo, nav 6 tabs (news, instances, modrinth, shaders, screenshots, settings), account manager
- **Main workspace** (flex): 6 views, tab switching via `switchTab(nav, view)` di `main.js:1025-1031`
- **Right sidebar** (240px): Server Hub (start/stop server, tunnel, console, plugin manager)
- **Bottom dock** (min-height 72px): avatar, instance selector, RAM, launch button

Dark mode: tambahkan class `theme-dark` ke `<body>` (`styles.css:42-73`).

## Launcher Settings Schema

Data tersimpan di `game_data/launcher_settings.json` (`settings.rs:29-55`):
```
{ accounts, active_account_id, instances, active_instance_id, last_version,
  ram_min, ram_max, theme, use_elyby, custom_java_path, custom_game_dir,
  custom_server_dir, res_width, res_height }
```

Default RAM: min=1024 MB, max=4096 MB. Default resolusi: 854×480.

## Yang Tidak Ada

Tidak ada config opencode, CLAUDE.md, .cursor/rules/, atau copilot-instructions.md. Tidak ada `.gitignore` untuk `src-tauri/target/` atau `game_data/`.
