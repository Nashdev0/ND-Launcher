# ND Launcher Design System: 3-Zone Soft Neobrutalism

Arsitektur antarmuka modern dengan layout 3-Zone: Mod Hub (Kiri), Dynamic Workspace (Tengah), Server Tool (Kanan), dan Control Dock (Bawah).

---

## 1. Tata Letak Global (Layout Grid)

* **Top Bar (Opsional / Mini):** Window controls + Toggle button untuk Server Panel kanan.
* **Left Sidebar (Width: ~220px):** Navigasi platform & modding.
* **Main Canvas (Flexible):** Area jelajah mod, konfigurasi modpack, atau galeri instance.
* **Right Panel (Width: ~260px - Collapsible):** Local Server Host & Tunneling Manager.
* **Bottom Bar (Height: ~80px - Persistent):** Game launcher dock.

---

## 2. Struktur Komponen Tiap Zona

### A. Sidebar Kiri (Modding & Library Hub)
* **Warna Aksen:** Soft Yellow (`#FDFFB6`) & White.
* **Elemen:**
  * Logo launcher (Top).
  * Menu Navigasi (Pill shape buttons dengan border hitam 2.5px):
    - 📦 **My Instances** (Daftar game yang terpasang).
    - 🟢 **Modrinth** (Browser mod/modpack API).
    - 🟠 **CurseForge** (Browser mod/modpack API).
    - ⚙️ **Settings** (Java, RAM, Direktori).

### B. Workspace Tengah (Dynamic Content)
* **Kondisi Instances:** Grid kartu instance/profil game dengan thumbnail dan versi.
* **Kondisi Modrinth/CurseForge:**
  * Search bar neobrutalist dengan filter tags (Fabric, Forge, Versi).
  * Grid Card Mod: Banner mod, judul tebal, deskripsi singkat, tombol "Install" (Soft Pink).

### C. Sidebar Kanan (Self-Host Local Server Panel)
* **Warna Aksen:** Soft Pink (`#FFC6FF`) & Soft Blue (`#A0C4FF`).
* **Karakteristik:** Bisa di-*minimize* agar tidak memakan ruang browsing mod.
* **Elemen:**
  * **Server Quick Toggle:** Tombol Start/Stop server lokal.
  * **Engine Selector:** Dropdown Paper / Purpur / Vanilla jar.
  * **RAM Slider:** Slider alokasi RAM khusus server.
  * **Connection Sharing:** Menampilkan IP lokal atau integrasi tunneling (ngrok / playit.gg) agar pemain bisa menyalin link untuk dibagikan ke teman.
  * **Console Logs Mini:** Jendela collapsible untuk memantau log server.

### D. Bottom Dock (Game Launch Bar)
* **Warna Aksen:** Soft Blue (`#A0C4FF`) untuk trigger utama.
* **Elemen:**
  * **Kiri:** Profile badge (Avatar kepala player melengkung + nama user offline).
  * **Tengah:** Instance & version selector chip (misal: "Fabric 1.20.4 • 4GB RAM").
  * **Kanan:** Progress bar ramping + Tombol **"LAUNCH GAME"** (Soft Blue pill-button besar).

---

## 3. Wireframe Visual

```text
+-------------------------------------------------------------------------------------------------------+
| [ND LAUNCHER]                                                      [ Host Server: OFF (Pink Toggle) ] |
+---------------+-----------------------------------------------------------------------+---------------+
| SIDEBAR       | MAIN WORKSPACE: DEFAULT (NEWS & CHANGELOG FEED)                       | SERVER HUB    |
| (KIRI)        |                                                                       | (KANAN)       |
|               | +-------------------------------------------------------------------+ | [Status: OFF] |
| [•] Beranda   | | FEATURED BANNER: MINECRAFT UPDATE (Soft Blue Card)                | |               |
|     (News)    | | Patch Notes & New Mechanics Overview                              | | Core:         |
|               | | [ Baca Selengkapnya ]                                             | | [ Purpur v ]  |
| [ ] Instances | +-------------------------------------------------------------------+ |               |
|               |                                                                       | RAM Server:   |
| [ ] Modrinth  | TERBARU & CHANGELOG:                                                  | [ 3 GB     ]  |
|               | +----------------------------------+ +------------------------------+ |               |
| [ ] Curse-    | | [RELEASE] Update 1.20.4          | | [SNAPSHOT] 26w...            | | Tunnel Link:  |
|     Forge     | | - Fix network connection bugs    | | - Testing new mob behaviors  | | [playit.gg ]  |
|               | | Tag: Soft Yellow                 | | Tag: Soft Pink               | | [ Copy Link ] |
| [⚙] Settings  | +----------------------------------+ +------------------------------+ |               |
+---------------+-----------------------------------------------------------------------+---------------+
| BOTTOM DOCK (LAUNCH BAR):                                                                             |
| [Avatar] Steve | Versi: [ 1.20.4 (Fabric + Iris Shaders) v ] | RAM: [ 4 GB ] | [  LAUNCH GAME (Blue) ] |
+-------------------------------------------------------------------------------------------------------+