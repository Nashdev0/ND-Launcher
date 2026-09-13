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

#[derive(Debug, Serialize, Deserialize)]
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

pub async fn fetch_versions() -> Result<Vec<VersionEntry>, String> {
    let url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;

    let manifest: VersionManifest = response.json().await.map_err(|e| e.to_string())?;

    let releases: Vec<VersionEntry> = manifest
        .versions
        .into_iter()
        .filter(|v| v.version_type == "release")
        .collect();

    Ok(releases)
}
