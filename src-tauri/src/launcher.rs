use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone, Serialize)]
struct ProgressPayload {
    stage: String,
    message: String,
    current: usize,
    total: usize,
}

#[derive(Debug, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct VersionJson {
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndex,
    pub downloads: Downloads,
    pub libraries: Vec<Library>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
}

#[derive(Debug, Deserialize)]
pub struct Downloads {
    pub client: DownloadFile,
}

#[derive(Debug, Deserialize)]
pub struct DownloadFile {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Library {
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
}

#[derive(Debug, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Deserialize)]
pub struct OsRule {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AssetIndexData {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
}

fn check_rules(rules: &Option<Vec<Rule>>) -> bool {
    if let Some(rules) = rules {
        let mut allow = false;
        for rule in rules {
            if rule.action == "allow" {
                if let Some(os) = &rule.os {
                    let os_name = std::env::consts::OS;
                    let match_os = match os.name.as_str() {
                        "windows" => os_name == "windows",
                        "osx" => os_name == "macos",
                        "linux" => os_name == "linux",
                        _ => false,
                    };
                    if match_os {
                        allow = true;
                    }
                } else {
                    allow = true;
                }
            } else if rule.action == "disallow" {
                if let Some(os) = &rule.os {
                    let os_name = std::env::consts::OS;
                    let match_os = match os.name.as_str() {
                        "windows" => os_name == "windows",
                        "osx" => os_name == "macos",
                        "linux" => os_name == "linux",
                        _ => false,
                    };
                    if match_os {
                        allow = false;
                    }
                }
            }
        }
        return allow;
    }
    true
}

#[derive(Debug, Deserialize)]
struct FabricLoaderInfo {
    pub version: String,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderMeta {
    pub loader: FabricLoaderInfo,
}

#[derive(Debug, Deserialize)]
struct FabricProfileLibrary {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct FabricProfile {
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub libraries: Vec<FabricProfileLibrary>,
}

async fn download_file(client: &reqwest::Client, url: &str, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
    }
        
    let max_retries = 3;
    let mut last_error = String::new();
    let tmp_dest = dest.with_extension("tmp");
    
    for attempt in 1..=max_retries {
        match client.get(url).send().await {
            Ok(mut response) => {
                if !response.status().is_success() {
                    last_error = format!("HTTP {}", response.status());
                    continue;
                }
                
                match fs::File::create(&tmp_dest).await {
                    Ok(mut file) => {
                        let mut success = true;
                        while let Some(chunk_res) = response.chunk().await.transpose() {
                            match chunk_res {
                                Ok(chunk) => {
                                    if let Err(e) = file.write_all(&chunk).await {
                                        last_error = format!("File write error: {}", e);
                                        success = false;
                                        break;
                                    }
                                },
                                Err(e) => {
                                    last_error = format!("Chunk error: {}", e);
                                    success = false;
                                    break;
                                }
                            }
                        }
                        if success {
                            let _ = fs::rename(&tmp_dest, dest).await;
                            return Ok(());
                        }
                    },
                    Err(e) => {
                        last_error = format!("File create error: {}", e);
                    }
                }
            },
            Err(e) => {
                last_error = format!("Request error: {}", e);
            }
        }
        
        if attempt < max_retries {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    
    Err(format!("Failed after {} attempts. Last error: {}", max_retries, last_error))
}

#[tauri::command]
pub async fn launch_game(app: AppHandle, username: String, uuid_str: String, version: String, _ram: u32, java_path: Option<String>, instance_id: String) -> Result<String, String> {
    let settings = crate::settings::get_settings().await.unwrap_or_default();
    
    // Create a single HTTP client for all downloads in this launch sequence to reuse connection pools
    let dl_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30)) // Only limit connection phase, not download body
        .build()
        .map_err(|e| e.to_string())?;
        
    let global_game_dir = settings.custom_game_dir
        .clone()
        .filter(|d| !d.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::settings::get_base_dir().join("game_data"));
        
    let instance_dir = global_game_dir.join("instances").join(&instance_id);
    
    if !instance_dir.exists() {
        fs::create_dir_all(&instance_dir).await.map_err(|e| e.to_string())?;
    }
    
    let _ = app.emit("progress", ProgressPayload { stage: "init".into(), message: "Fetching manifest...".into(), current: 0, total: 100 });
    
    // 1. Fetch main manifest to get version URL
    let manifest_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let manifest_resp = reqwest::Client::new().get(manifest_url).send().await.map_err(|e| e.to_string())?;
    
    let manifest_json: serde_json::Value = manifest_resp.json().await.map_err(|e| e.to_string())?;
    let versions = manifest_json["versions"].as_array().ok_or("Invalid manifest")?;
    
    let mut version_json_url = String::new();
    for v in versions {
        if v["id"].as_str() == Some(version.as_str()) {
            if let Some(url) = v["url"].as_str() {
                version_json_url = url.to_string();
                break;
            }
        }
    }
    
    if version_json_url.is_empty() {
        return Err(format!("Version {} not found in manifest", version));
    }

    let response = reqwest::Client::new().get(&version_json_url).send().await.map_err(|e| e.to_string())?;
    let version_data: VersionJson = response.json().await.map_err(|e| format!("Error decoding response body: {}", e))?;

    let _ = app.emit("progress", ProgressPayload { stage: "libraries".into(), message: "Downloading client.jar...".into(), current: 1, total: version_data.libraries.len() + 1 });
    let client_jar_path = global_game_dir.join(format!("versions/{0}/{0}.jar", version));
    download_file(&dl_client, &version_data.downloads.client.url, &client_jar_path).await?;

    let mut classpath_entries = vec![client_jar_path.to_string_lossy().to_string()];
    
    // --- FABRIC LOADER INJECTION ---
    let instances = crate::settings::get_instances().await?;
    let instance = instances.iter().find(|i| i.id == instance_id).ok_or("Instance not found")?;
    let loader_type = instance.loader.to_lowercase();
    
    let mut main_class_to_run = version_data.main_class.clone();

    if loader_type == "fabric" {
        let _ = app.emit("progress", ProgressPayload { stage: "fabric".into(), message: "Fetching Fabric Meta...".into(), current: 0, total: 100 });
        let meta_url = format!("https://meta.fabricmc.net/v2/versions/loader/{}", version);
        let meta_resp = reqwest::Client::new().get(&meta_url).send().await.map_err(|e| e.to_string())?;
        let fabric_versions: Vec<FabricLoaderMeta> = meta_resp.json().await.map_err(|e| e.to_string())?;
        
        if let Some(latest_fabric) = fabric_versions.first() {
            let profile_url = format!("https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json", version, latest_fabric.loader.version);
            let profile_resp = reqwest::Client::new().get(&profile_url).send().await.map_err(|e| e.to_string())?;
            let fabric_profile: FabricProfile = profile_resp.json().await.map_err(|e| format!("Fabric JSON Error: {}", e))?;
            
            main_class_to_run = fabric_profile.main_class;
            
            for f_lib in fabric_profile.libraries {
                // name is like "net.fabricmc:fabric-loader:0.15.7"
                // split by :
                let parts: Vec<&str> = f_lib.name.split(':').collect();
                if parts.len() == 3 {
                    let domain = parts[0].replace('.', "/");
                    let name = parts[1];
                    let ver = parts[2];
                    let jar_name = format!("{}-{}.jar", name, ver);
                    let lib_path = global_game_dir.join("libraries").join(&domain).join(name).join(ver).join(&jar_name);
                    
                    let download_url = format!("{}{}/{}/{}/{}", f_lib.url, domain, name, ver, jar_name);
                    
                    if !lib_path.exists() {
                        let _ = download_file(&dl_client, &download_url, &lib_path).await;
                    }
                    
                    classpath_entries.push(lib_path.to_string_lossy().to_string());
                }
            }
        }
    }
    // --- END FABRIC INJECTION ---

    let total_libs = version_data.libraries.len();
    
    let sem_libs = Arc::new(Semaphore::new(20));
    let comp_libs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles_libs = vec![];
    
    for lib in version_data.libraries.into_iter() {
        if check_rules(&lib.rules) {
            if let Some(downloads) = lib.downloads {
                if let Some(artifact) = downloads.artifact {
                    let lib_path = global_game_dir.join("libraries").join(&artifact.path);
                    classpath_entries.push(lib_path.to_string_lossy().to_string());
                    
                    let sem = sem_libs.clone();
                    let app_c = app.clone();
                    let comp = comp_libs.clone();
                    
                    let dl_client_clone = dl_client.clone();
                    handles_libs.push(tokio::spawn(async move {
                        let _permit = sem.acquire().await.unwrap();
                        let _ = download_file(&dl_client_clone, &artifact.url, &lib_path).await;
                        let c = comp.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        if c % 10 == 0 || c == total_libs {
                            let _ = app_c.emit("progress", ProgressPayload { 
                                stage: "libraries".into(),
                                message: format!("Downloading libraries ({}/{})", c, total_libs),
                                current: c, 
                                total: total_libs 
                            });
                        }
                    }));
                }
            }
        }
    }
    
    for handle in handles_libs {
        let _ = handle.await;
    }

    // --- ASSETS DOWNLOADER ---
    let _ = app.emit("progress", ProgressPayload { stage: "assets".into(), message: "Fetching asset index...".into(), current: 0, total: 1 });
    let asset_index_id = &version_data.asset_index.id;
    let asset_index_url = &version_data.asset_index.url;
    let index_dest = global_game_dir.join("assets").join("indexes").join(format!("{}.json", asset_index_id));
    download_file(&dl_client, asset_index_url, &index_dest).await?;

    let index_bytes = fs::read(&index_dest).await.map_err(|e| e.to_string())?;
    let asset_data: AssetIndexData = serde_json::from_slice(&index_bytes).map_err(|e| e.to_string())?;
    
    let objects: Vec<(String, String)> = asset_data.objects.into_iter()
        .map(|(_, obj)| {
            let hash = obj.hash.clone();
            let subhash = &hash[0..2];
            let url = format!("https://resources.download.minecraft.net/{}/{}", subhash, hash);
            (hash, url)
        }).collect();

    let total_assets = objects.len();
    let semaphore = Arc::new(Semaphore::new(20)); // Max 20 concurrent downloads
    let mut handles = vec![];
    let app_clone = app.clone();
    
    let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    for (hash, url) in objects {
        let sem = semaphore.clone();
        let app_c = app_clone.clone();
        let comp = completed.clone();
        let subhash = hash[0..2].to_string();
        let dest = global_game_dir.join("assets").join("objects").join(&subhash).join(&hash);
        let dl_client_clone = dl_client.clone();
        
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let _ = download_file(&dl_client_clone, &url, &dest).await;
            let c = comp.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if c % 50 == 0 || c == total_assets {
                let _ = app_c.emit("progress", ProgressPayload { 
                    stage: "assets".into(),
                    message: format!("Downloading assets ({}/{})", c, total_assets),
                    current: c, 
                    total: total_assets 
                });
            }
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }
    // --- END ASSETS DOWNLOADER ---

    let separator = if cfg!(windows) { ";" } else { ":" };
    let classpath = classpath_entries.join(separator);

    // --- OFFLINE SKIN RESOURCE PACK INJECTION ---
    let skins_dir = crate::settings::get_global_game_dir().await.join("skins");
    let user_skin = skins_dir.join(format!("{}.png", username));
    if user_skin.exists() {
        let rp_dir = instance_dir.join("resourcepacks").join("ND_OfflineSkin");
        let wide_dir = rp_dir.join("assets").join("minecraft").join("textures").join("entity").join("player").join("wide");
        let slim_dir = rp_dir.join("assets").join("minecraft").join("textures").join("entity").join("player").join("slim");
        
        let _ = fs::create_dir_all(&wide_dir).await;
        let _ = fs::create_dir_all(&slim_dir).await;
        
        let mut pack_format = 15;
        if version.starts_with("1.21") { pack_format = 34; }
        else if version.starts_with("1.20.5") || version.starts_with("1.20.6") { pack_format = 32; }
        else if version.starts_with("1.20.3") || version.starts_with("1.20.4") { pack_format = 22; }
        else if version.starts_with("1.20.2") { pack_format = 18; }
        else if version.starts_with("1.20") { pack_format = 15; }
        else if version.starts_with("1.19.4") { pack_format = 13; }
        else if version.starts_with("1.19.3") { pack_format = 12; }
        else if version.starts_with("1.19") { pack_format = 9; }
        else if version.starts_with("1.18") { pack_format = 8; }
        else if version.starts_with("1.17") { pack_format = 7; }
        else if version.starts_with("1.16") { pack_format = 6; }
        else if version.starts_with("1.15") { pack_format = 5; }
        else if version.starts_with("1.14") || version.starts_with("1.13") { pack_format = 4; }
        
        let mcmeta = format!(r#"{{"pack":{{"pack_format":{},"description":"ND Launcher Offline Skin"}}}}"#, pack_format);
        let _ = fs::write(rp_dir.join("pack.mcmeta"), mcmeta).await;
        
        // Copy to root player textures (older versions)
        let player_dir = rp_dir.join("assets").join("minecraft").join("textures").join("entity").join("player");
        let _ = fs::copy(&user_skin, player_dir.join("steve.png")).await;
        let _ = fs::copy(&user_skin, player_dir.join("alex.png")).await;
        
        // Copy to wide/slim (newer versions like 1.20+)
        let default_names = vec![
            "steve.png", "ari.png", "efe.png", "kai.png", 
            "makena.png", "noor.png", "sunny.png", "zuri.png"
        ];
        
        for name in &default_names {
            let _ = fs::copy(&user_skin, wide_dir.join(name)).await;
            let _ = fs::copy(&user_skin, slim_dir.join(name)).await;
        }
        
        // Auto-enable resource pack in options.txt
        let options_txt = instance_dir.join("options.txt");
        if let Ok(content) = fs::read_to_string(&options_txt).await {
            if !content.contains("ND_OfflineSkin") {
                let mut new_lines = Vec::new();
                let mut found = false;
                for line in content.lines() {
                    if line.starts_with("resourcePacks:[") {
                        found = true;
                        if line == "resourcePacks:[]" {
                            new_lines.push("resourcePacks:[\"vanilla\",\"file/ND_OfflineSkin\"]".to_string());
                        } else if line.ends_with("]") {
                            let inner = &line[15..line.len()-1];
                            if inner.is_empty() {
                                new_lines.push("resourcePacks:[\"vanilla\",\"file/ND_OfflineSkin\"]".to_string());
                            } else {
                                new_lines.push(format!("resourcePacks:[{},\"file/ND_OfflineSkin\"]", inner));
                            }
                        } else {
                            new_lines.push(line.to_string());
                        }
                    } else {
                        new_lines.push(line.to_string());
                    }
                }
                if !found {
                    new_lines.push("resourcePacks:[\"vanilla\",\"file/ND_OfflineSkin\"]".to_string());
                }
                let _ = fs::write(&options_txt, new_lines.join("\n") + "\n").await;
            }
        } else {
            let _ = fs::write(&options_txt, "resourcePacks:[\"vanilla\",\"file/ND_OfflineSkin\"]\n").await;
        }
    }
    // --- END OFFLINE SKIN ---

    let mut java_bin = if cfg!(windows) { "java.exe".to_string() } else { "java".to_string() };
    if let Some(path) = &settings.custom_java_path {
        if !path.trim().is_empty() {
            java_bin = path.clone();
        }
    } else if let Some(path) = java_path {
        if !path.trim().is_empty() {
            java_bin = path;
        }
    }
    
    // Auto-resolve absolute path if it's just the default command
    if java_bin == "java.exe" || java_bin == "java" {
        if let Some(best) = crate::java::get_best_java_for_version(&version).await {
            java_bin = best;
        }
    }

    let mut child = tokio::process::Command::new(java_bin)
        .current_dir(&instance_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .arg(format!("-Xmx{}M", settings.ram_max))
        .arg(format!("-Xms{}M", settings.ram_min))
        .arg("-cp")
        .arg(classpath)
        .arg(&main_class_to_run)
        .arg("--username")
        .arg(&username)
        .arg("--version")
        .arg(&version)
        .arg("--gameDir")
        .arg(instance_dir.to_string_lossy().to_string())
        .arg("--assetsDir")
        .arg(global_game_dir.join("assets").to_string_lossy().to_string())
        .arg("--assetIndex")
        .arg(&asset_index_id)
        .arg("--uuid")
        .arg(&uuid_str)
        .arg("--accessToken")
        .arg("0")
        .arg("--userType")
        .arg("legacy")
        .arg("--width")
        .arg(settings.res_width.to_string())
        .arg("--height")
        .arg(settings.res_height.to_string())
        .spawn()
        .map_err(|e| e.to_string())?;

    let child_id = child.id().unwrap_or(0);

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let app_out = app.clone();
    let app_err = app.clone();
    let app_exit = app.clone();

    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = app_out.emit("game-log", line);
        }
    });

    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = app_err.emit("game-log", format!("[ERROR] {}", line));
        }
    });

    tokio::spawn(async move {
        let _ = child.wait().await;
        let _ = app_exit.emit("game-log", "[SYSTEM] Game exited.".to_string());
    });

    Ok(format!("Minecraft {} launched with PID {}", version, child_id))
}
