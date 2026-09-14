use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    #[serde(rename = "time")]
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

/// Cache file holds the filtered release list plus a unix timestamp.
#[derive(Serialize, Deserialize)]
struct VersionCache {
    fetched_at: u64,
    releases: Vec<VersionEntry>,
}

const CACHE_TTL_SECS: u64 = 60 * 60 * 6; // 6 jam — cukup segar, tapi startup instan

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub async fn fetch_versions() -> Result<Vec<VersionEntry>, String> {
    let cache_path = crate::settings::get_base_dir()
        .join("game_data")
        .join("versions_cache.json");

    // 1. Coba pakai cache kalau masih fresh (instan, tanpa internet)
    if let Ok(content) = tokio::fs::read_to_string(&cache_path).await {
        if let Ok(cache) = serde_json::from_str::<VersionCache>(&content) {
            if now_secs().saturating_sub(cache.fetched_at) < CACHE_TTL_SECS
                && !cache.releases.is_empty()
            {
                return Ok(cache.releases);
            }
        }
    }

    // 2. Cache kosong/basi — ambil dari Mojang
    let url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    let manifest: VersionManifest = response.json().await.map_err(|e| e.to_string())?;

    let releases: Vec<VersionEntry> = manifest
        .versions
        .into_iter()
        .filter(|v| v.version_type == "release")
        .collect();

    // 3. Simpan ke cache (best-effort)
    if let Some(parent) = cache_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let cache = VersionCache { fetched_at: now_secs(), releases: releases.clone() };
    if let Ok(json) = serde_json::to_string(&cache) {
        let _ = tokio::fs::write(&cache_path, json).await;
    }

    Ok(releases)
}
