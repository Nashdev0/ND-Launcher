# 🎮 ND Launcher v0.0.4

> Launcher Minecraft offline — Tauri v2 · Rust backend · Vanilla JS frontend

[![Build Status](https://github.com/Nashdev0/ND-Launcher/actions/workflows/build-windows.yml/badge.svg)](https://github.com/Nashdev0/ND-Launcher/actions)
[![Version](https://img.shields.io/badge/version-0.0.4-blue)](#)
[![Platform](https://img.shields.io/badge/platform-Windows-blue?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgZmlsbD0iI2ZmZiI+PHBhdGggZD0iTTAsMS41UTAsMCwxLjUsMEgxNC41USoxNiwxLjUgMTYsMS41VjE0LjVxMCwxLjUtMS41LDEuNUgxLjVRMCwxNiwwLDE0LjVaTTMsM1gxMyBWMTNIM1oiLz48L3N2Zz4=)](#)
[![Rust](https://img.shields.io/badge/rust-%23000?logo=rust&logoColor=orange)](#)
[![Tauri](https://img.shields.io/badge/tauri-2.x-24c8d8?logo=tauri)](#)
[![License](https://img.shields.io/badge/license-MIT-green)](#)

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| **Multi-Instance** | Up to 5 instances with isolated mods & saves |
| **Fabric Loader** | Auto-download + inject Fabric into game |
| **Purpur Server** | Download & run Purpur server locally |
| **SSH Tunnel** | Expose server to the internet via `pinggy.io` |
| **Modrinth Integration** | Search & install mods directly from UI |
| **Shader Manager** | Download & toggle shaderpacks per instance |
| **Screenshot Capture** | Take screenshots while playing |
| **Crash Analyzer** | Auto-read crash reports & latest.log |
| **Java Auto-Detect** | Smart JVM selection based on MC version |
| **Offline Auth** | Deterministic UUID (Ely.by optional) |
| **Dark / Light Theme** | Toggle in settings |

---

## 📸 Preview

<!-- Add a screenshot GIF or image here -->
<!-- ![ND Launcher Preview](docs/screenshot.png) -->

---

## 🚀 Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI](https://tauri.app/start/prerequisites/)

```bash
# Install dependencies
npm install

# Development mode (hot reload)
npm run tauri dev

# Production build
npm run tauri build
```

Windows release binaries: `src-tauri/target/release/bundle/` (`.exe` NSIS + `.msi`)

---

## 🏗 Project Structure

```
ND-LAUNCHER-PROJEK/
├── src/
│   └── main.js              # Frontend (~1967 lines, vanilla JS)
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs           # @tauri::command registration
│   │   ├── main.rs          # Entry point
│   │   ├── settings.rs      # Account & instance management
│   │   ├── java.rs          # Java version auto-detection
│   │   ├── server.rs        # Purpur server runner
│   │   ├── launcher.rs      # Game launch + Fabric injection
│   │   └── screenshot.rs    # Screenshot capture
│   └── tauri.conf.json
└── .github/workflows/
    └── build-windows.yml    # CI pipeline
```

---

## 💾 Data Storage

All data lives in **`ND Launcher/`**:

```
C:\ND Launcher\               Windows
~/ND Launcher/                Linux / macOS
```

```
ND Launcher/
├── game_data/
│   ├── launcher_settings.json   ← accounts, instances, RAM, theme
│   ├── instances/{id}/
│   │   ├── mods/                ← .jar = active, .jar.disabled = inactive
│   │   ├── shaderpacks/
│   │   └── saves/
│   ├── java/{17,21,25}/         ← Adoptium JRE (auto-downloaded)
│   ├── skins/{username}.png     ← cached skin images
│   └── libraries/               ← Fabric library cache
└── server/{version}/            ← Purpur server files
```

---

## ⚡ Key Technical Details

### Java → Minecraft Version Mapping

| Minecraft Version | Java Version |
|-------------------|-------------|
| 1.21.2+ / 26.x | Java 25 |
| 1.20.5 – 1.21.1 | Java 21 |
| 1.17 – 1.20.4 | Java 17 |
| Older | Java 8 |

### JVM Flags (Aikar's Optimization)

Applied automatically at launch via `launcher.rs`:

```
-Xmx{ram_max}M -Xms{ram_min}M
-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200
-XX:+AlwaysPreTouch -XX:InitiatingHeapOccupancyPercent=15 ...
```

### External APIs Used

| Source | Purpose |
|--------|---------|
| `modrinth.com/api` | Mod & plugin search/download |
| `purpurmc.org/api` | Purpur version list & download |
| `fabricmc.net/meta` | Fabric loader metadata |
| `adoptium.net/api` | Automatic JRE download |
| `ely.by` | Authlib injector & skin textures |
| `mojang.com` | Official Minecraft version manifest |

---

## ⚠️ Known Quirks

| # | Issue | Detail |
|---|-------|--------|
| 1 | Max 5 instances | Must delete old instances before creating new ones |
| 2 | Server bug | Always uses `java` from PATH, ignores custom Java path in settings |
| 3 | Auth offline only | UUID is deterministic (`v3(nil, "OfflinePlayer:{name}")`), not real Mojang auth |
| 4 | Mod toggle | Enables/disables by renaming `.jar` ↔ `.jar.disabled` — no content editing |
| 5 | Iris skips Sodium | Mod resolver auto-excludes Sodium when installing Iris |

---

## 🛠 Adding a New Command

Two places must be updated — missing either one breaks the command:

```rust
// 1. Define the command
#[tauri::command]
async fn my_new_command() -> Result<String, String> { ... }

// 2. Register in src-tauri/src/lib.rs generate_handler![]
```

---

## 📡 Event System

Three custom Tauri events used in the frontend:

| Event | Payload | Use Case |
|-------|---------|----------|
| `progress` | `{stage, message, current, total}` | Progress bar updates |
| `game-log` | `string` | Stream game output; `[SYSTEM] Game exited` triggers crash analyzer |
| `server-log` | `string` | Stream server console output |

---

## 🐧 Building for Linux/macOS

CI only targets Windows. For other platforms, build manually:

```bash
# macOS
cargo build --release --target x86_64-apple-darwin

# Linux
cargo build --release --target x86_64-unknown-linux-gnu
```

---

## 📄 License

Private project — **ND Launcher Demo**.
