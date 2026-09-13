use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub skin_url: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version: String,
    pub loader: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModInfo {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LauncherSettings {
    pub accounts: Vec<Account>,
    pub active_account_id: Option<String>,
    pub instances: Vec<Instance>,
    pub active_instance_id: Option<String>,
    pub last_version: String,
    
    // Server & Client RAM
    pub ram_min: u32,
    pub ram_max: u32,
    
    // UI Theme
    pub theme: String, // "light" or "dark"
    
    // Skin Integration
    pub use_elyby: Option<bool>,
    
    // Custom Paths
    pub custom_java_path: Option<String>,
    pub custom_game_dir: Option<String>,
    pub custom_server_dir: Option<String>,
    
    // Client Resolution
    pub res_width: u32,
    pub res_height: u32,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            accounts: vec![],
            active_account_id: None,
            instances: vec![],
            active_instance_id: None,
            last_version: "".to_string(),
            ram_min: 1024,
            ram_max: 4096,
            theme: "light".to_string(),
            use_elyby: Some(false),
            custom_java_path: None,
            custom_game_dir: None,
            custom_server_dir: None,
            res_width: 854,
            res_height: 480,
        }
    }
}

pub fn get_base_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("C:\\ND Launcher")
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join("ND Launcher")
        } else {
            std::env::current_dir().unwrap().join("ND Launcher")
        }
    }
}

fn get_settings_path() -> PathBuf {
    get_base_dir().join("game_data").join("launcher_settings.json")
}

#[tauri::command]
pub async fn get_settings() -> Result<LauncherSettings, String> {
    let path = get_settings_path();
    if !path.exists() {
        return Ok(LauncherSettings::default());
    }
    
    let contents = fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
    let settings: LauncherSettings = serde_json::from_str(&contents).unwrap_or_default();
    Ok(settings)
}

#[tauri::command]
pub async fn save_settings(settings: LauncherSettings) -> Result<(), String> {
    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent).await;
    }
    
    let contents = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, contents).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_global_game_dir() -> PathBuf {
    let settings = get_settings().await.unwrap_or_default();
    settings.custom_game_dir
        .clone()
        .filter(|d| !d.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| get_base_dir().join("game_data"))
}

#[tauri::command]
pub async fn add_account(username: String) -> Result<Account, String> {
    let mut settings = get_settings().await?;
    
    let uuid_val = uuid::Uuid::new_v3(&uuid::Uuid::nil(), format!("OfflinePlayer:{}", username).as_bytes());
    
    // Set all other accounts to inactive
    for acc in &mut settings.accounts {
        acc.is_active = false;
    }
    
    let new_account = Account {
        id: uuid::Uuid::new_v4().to_string(),
        username: username.clone(),
        uuid: uuid_val.to_string(),
        skin_url: Some(format!("https://mc-heads.net/avatar/{}/64", username)),
        is_active: true,
    };
    
    settings.active_account_id = Some(new_account.id.clone());
    settings.accounts.push(new_account.clone());
    save_settings(settings).await?;
    
    Ok(new_account)
}

#[tauri::command]
pub async fn switch_account(accountId: String) -> Result<(), String> {
    let mut settings = get_settings().await?;
    
    let mut found = false;
    for acc in &mut settings.accounts {
        if acc.id == accountId {
            acc.is_active = true;
            found = true;
        } else {
            acc.is_active = false;
        }
    }
    
    if found {
        settings.active_account_id = Some(accountId);
        save_settings(settings).await?;
        Ok(())
    } else {
        Err("Account not found".into())
    }
}

#[tauri::command]
pub async fn delete_account(accountId: String) -> Result<(), String> {
    let mut settings = get_settings().await?;
    
    settings.accounts.retain(|acc| acc.id != accountId);
    if settings.active_account_id == Some(accountId) {
        settings.active_account_id = settings.accounts.first().map(|a| a.id.clone());
        if let Some(first_id) = &settings.active_account_id {
            for acc in &mut settings.accounts {
                if acc.id == *first_id {
                    acc.is_active = true;
                }
            }
        }
    }
    
    save_settings(settings).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_accounts() -> Result<Vec<Account>, String> {
    let settings = get_settings().await?;
    Ok(settings.accounts)
}

#[tauri::command]
pub async fn create_instance(name: String, version: String, loader: String) -> Result<Instance, String> {
    let mut settings = get_settings().await?;
    
    if settings.instances.len() >= 5 {
        return Err("Batas maksimal pembuatan Instance adalah 5! Hapus instance lama untuk membuat yang baru.".to_string());
    }
    
    // Create a URL-safe slug from the instance name
    let mut base_id = name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "-");
    while base_id.contains("--") {
        base_id = base_id.replace("--", "-");
    }
    base_id = base_id.trim_matches('-').to_string();
    if base_id.is_empty() {
        base_id = "instance".to_string();
    }
    
    let mut instance_id = base_id.clone();
    let mut counter = 1;
    while settings.instances.iter().any(|i| i.id == instance_id) {
        instance_id = format!("{}-{}", base_id, counter);
        counter += 1;
    }
    
    let new_instance = Instance {
        id: instance_id.clone(),
        name,
        version,
        loader,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string(),
    };
    
    // Create Folder Skeletons
    let instance_dir = get_global_game_dir().await.join("instances").join(&instance_id);
    let _ = fs::create_dir_all(instance_dir.join("mods")).await;
    let _ = fs::create_dir_all(instance_dir.join("config")).await;
    let _ = fs::create_dir_all(instance_dir.join("saves")).await;
    
    // Write instance.json metadata
    let metadata_json = serde_json::to_string_pretty(&new_instance).unwrap_or_default();
    let _ = fs::write(instance_dir.join("instance.json"), metadata_json).await;

    settings.active_instance_id = Some(new_instance.id.clone());
    settings.instances.push(new_instance.clone());
    save_settings(settings).await?;
    
    Ok(new_instance)
}

#[tauri::command]
pub async fn switch_instance(instanceId: String) -> Result<(), String> {
    let mut settings = get_settings().await?;
    
    if settings.instances.iter().any(|inst| inst.id == instanceId) {
        settings.active_instance_id = Some(instanceId);
        save_settings(settings).await?;
        Ok(())
    } else {
        Err("Instance not found".into())
    }
}

#[tauri::command]
pub async fn delete_instance(instanceId: String) -> Result<(), String> {
    let mut settings = get_settings().await?;
    
    let instance_dir = get_global_game_dir().await.join("instances").join(&instanceId);
    settings.instances.retain(|inst| inst.id != instanceId);
    if settings.active_instance_id == Some(instanceId.clone()) {
        settings.active_instance_id = settings.instances.first().map(|i| i.id.clone());
    }
    
    if instance_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&instance_dir).await {
            println!("Failed to delete instance folder: {}", e);
        }
    }
    
    save_settings(settings).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_instances() -> Result<Vec<Instance>, String> {
    let settings = get_settings().await?;
    Ok(settings.instances)
}

#[tauri::command]
pub async fn download_mod_to_instance(instanceId: String, downloadUrl: String, fileName: String) -> Result<(), String> {
    let instance_dir = get_global_game_dir().await.join("instances").join(&instanceId);
    let mods_dir = instance_dir.join("mods");

    if !mods_dir.exists() {
        let _ = fs::create_dir_all(&mods_dir).await;
    }

    let dest_path = mods_dir.join(&fileName);

    if dest_path.exists() {
        return Err("Mod already exists in this instance!".to_string());
    }

    let mut response = reqwest::get(&downloadUrl).await.map_err(|e| format!("Failed to download: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Download HTTP error: {}", response.status()));
    }

    let mut file = std::fs::File::create(&dest_path).map_err(|e| format!("Failed to create mod file: {}", e))?;
    while let Some(chunk) = response.chunk().await.map_err(|e| format!("Failed to read chunk: {}", e))? {
        use std::io::Write;
        file.write_all(&chunk).map_err(|e| format!("Failed to save mod file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn download_shader_to_instance(instanceId: String, downloadUrl: String, fileName: String) -> Result<(), String> {
    let instance_dir = get_global_game_dir().await.join("instances").join(&instanceId);
    let shader_dir = instance_dir.join("shaderpacks");
    
    if !shader_dir.exists() {
        let _ = std::fs::create_dir_all(&shader_dir);
    }
    
    let dest_path = shader_dir.join(&fileName);
    
    if dest_path.exists() {
        return Err("Shader already exists in this instance!".to_string());
    }
    
    let mut response = reqwest::get(&downloadUrl).await.map_err(|e| format!("Failed to download: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Download HTTP error: {}", response.status()));
    }
    
    let mut file = std::fs::File::create(&dest_path).map_err(|e| format!("Failed to create shader file: {}", e))?;
    while let Some(chunk) = response.chunk().await.map_err(|e| format!("Failed to read chunk: {}", e))? {
        use std::io::Write;
        file.write_all(&chunk).map_err(|e| format!("Failed to save shader file: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_local_skin_data(username: String) -> Result<Option<Vec<u8>>, String> {
    let skins_dir = get_global_game_dir().await.join("skins");
    let user_skin = skins_dir.join(format!("{}.png", username));
    if user_skin.exists() {
        let bytes = std::fs::read(&user_skin).map_err(|e| e.to_string())?;
        Ok(Some(bytes))
    } else {
        let settings = get_settings().await?;
        if settings.use_elyby.unwrap_or(false) {
            if let Ok(Some(bytes)) = get_elyby_skin_bytes(&username).await {
                // Save it locally so we don't spam Ely.by API every UI refresh
                let _ = std::fs::create_dir_all(&skins_dir);
                let _ = std::fs::write(&user_skin, &bytes);
                return Ok(Some(bytes));
            }
        }
        Ok(None)
    }
}

#[tauri::command]
pub async fn copy_local_skin(username: String, source_path: String) -> Result<(), String> {
    let skins_dir = get_global_game_dir().await.join("skins");
    if !skins_dir.exists() {
        let _ = std::fs::create_dir_all(&skins_dir);
    }
    let dest_path = skins_dir.join(format!("{}.png", username));
    std::fs::copy(source_path, dest_path).map_err(|e| format!("Gagal menyimpan skin: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn get_instance_mods(instanceId: String) -> Result<Vec<ModInfo>, String> {
    let mods_dir = get_global_game_dir().await.join("instances").join(&instanceId).join("mods");
    let mut mods = vec![];
    if mods_dir.exists() {
        let mut entries = fs::read_dir(mods_dir).await.map_err(|e| e.to_string())?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.ends_with(".jar") {
                        mods.push(ModInfo { name: file_name, enabled: true });
                    } else if file_name.ends_with(".jar.disabled") {
                        mods.push(ModInfo { name: file_name, enabled: false });
                    }
                }
            }
        }
    }
    Ok(mods)
}

#[tauri::command]
pub async fn toggle_mod(instanceId: String, modName: String, enabled: bool) -> Result<(), String> {
    let mods_dir = get_global_game_dir().await.join("instances").join(&instanceId).join("mods");
    let target_path = mods_dir.join(&modName);
    if !target_path.exists() {
        return Err("Mod file not found".to_string());
    }
    
    if enabled && modName.ends_with(".jar.disabled") {
        let new_name = modName.replace(".jar.disabled", ".jar");
        let new_path = mods_dir.join(&new_name);
        fs::rename(target_path, new_path).await.map_err(|e| e.to_string())?;
    } else if !enabled && modName.ends_with(".jar") {
        let new_name = format!("{}.disabled", modName);
        let new_path = mods_dir.join(&new_name);
        fs::rename(target_path, new_path).await.map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn delete_mod(instanceId: String, modName: String) -> Result<(), String> {
    let mods_dir = get_global_game_dir().await.join("instances").join(&instanceId).join("mods");
    let target_path = mods_dir.join(&modName);
    
    if target_path.exists() {
        fs::remove_file(target_path).await.map_err(|e| e.to_string())?;
    } else {
        return Err("Mod file not found".to_string());
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_app_version(app: tauri::AppHandle) -> Result<String, String> {
    Ok(app.package_info().version.to_string())
}

#[tauri::command]
pub async fn get_latest_crash_log(instanceId: String) -> Result<String, String> {
    let base_dir = get_base_dir();
    let instance_dir = base_dir.join("instances").join(&instanceId);
    let crash_reports_dir = instance_dir.join("crash-reports");
    let latest_log = instance_dir.join("logs").join("latest.log");
    
    // Check crash-reports first
    if crash_reports_dir.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(&crash_reports_dir).await {
            let mut latest_file = None;
            let mut latest_time = std::time::UNIX_EPOCH;
            
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        if modified > latest_time {
                            latest_time = modified;
                            latest_file = Some(entry.path());
                        }
                    }
                }
            }
            
            if let Some(path) = latest_file {
                // If it's a new crash report within the last hour
                if let Ok(elapsed) = latest_time.elapsed() {
                    if elapsed.as_secs() < 3600 {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            return Ok(content);
                        }
                    }
                }
            }
        }
    }
    
    // Fallback to logs/latest.log
    if latest_log.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&latest_log).await {
            return Ok(content);
        }
    }
    
    Err("No crash logs found.".to_string())
}
use reqwest::Client;

#[derive(Deserialize)]
struct ElybyProfile {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct ElybySessionProfile {
    properties: Vec<ElybyProperty>,
}

#[derive(Deserialize)]
struct ElybyProperty {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct ElybyTexturesValue {
    textures: ElybyTextures,
}

#[derive(Deserialize)]
struct ElybyTextures {
    #[serde(rename = "SKIN")]
    skin: Option<ElybySkin>,
}

#[derive(Deserialize)]
struct ElybySkin {
    url: String,
}

#[derive(Serialize, Deserialize)]
pub struct ElybyAccountInfo {
    pub uuid: String,
    pub name: String,
    pub skin_url: Option<String>,
    pub has_custom_skin: bool,
}

pub async fn get_elyby_uuid(username: &str) -> Result<Option<String>, String> {
    let client = Client::new();
    let profiles_res = client
        .post("https://authserver.ely.by/api/profiles/minecraft")
        .json(&vec![username])
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let profiles: Vec<ElybyProfile> = profiles_res.json().await.map_err(|e| e.to_string())?;
    Ok(profiles.first().map(|p| p.id.clone()))
}

#[tauri::command]
pub async fn get_elyby_profile(username: String) -> Result<ElybyAccountInfo, String> {
    let client = Client::new();
    // 1. Get UUID
    let profiles_res = client
        .post("https://authserver.ely.by/api/profiles/minecraft")
        .json(&vec![username.clone()])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let profiles: Vec<ElybyProfile> = profiles_res.json().await.map_err(|e| e.to_string())?;
    let profile = match profiles.first() {
        Some(p) => p,
        None => return Err(format!("Username '{}' not found on Ely.by", username)),
    };

    // 2. Get session with textures
    let session_res = client
        .get(format!(
            "https://authserver.ely.by/api/authlib-injector/sessionserver/session/minecraft/profile/{}",
            profile.id
        ))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let session_profile: ElybySessionProfile = session_res.json().await.map_err(|e| e.to_string())?;

    // 3. Extract skin info
    let textures_prop = match session_profile.properties.iter().find(|p| p.name == "textures") {
        Some(p) => p,
        None => {
            return Ok(ElybyAccountInfo {
                uuid: profile.id.clone(),
                name: profile.name.clone(),
                skin_url: None,
                has_custom_skin: false,
            })
        }
    };

    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&textures_prop.value)
        .map_err(|e| e.to_string())?;
    let decoded_str = String::from_utf8_lossy(&decoded);
    let textures_val: ElybyTexturesValue =
        serde_json::from_str(&decoded_str).map_err(|e| e.to_string())?;

    let skin_url = match textures_val.textures.skin {
        Some(s) => s.url,
        None => {
            return Ok(ElybyAccountInfo {
                uuid: profile.id.clone(),
                name: profile.name.clone(),
                skin_url: None,
                has_custom_skin: false,
            })
        }
    };

    let has_custom_skin = !skin_url.contains("minecraft.net/texture/");

    Ok(ElybyAccountInfo {
        uuid: profile.id.clone(),
        name: profile.name.clone(),
        skin_url: Some(skin_url),
        has_custom_skin,
    })
}

#[tauri::command]
pub async fn get_elyby_face_url(username: &str) -> Result<String, String> {
    let client = Client::new();
    // Get UUID from Ely.by
    let profiles_res = client
        .post("https://authserver.ely.by/api/profiles/minecraft")
        .json(&vec![username])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let profiles: Vec<ElybyProfile> = profiles_res.json().await.map_err(|e| e.to_string())?;
    let profile = match profiles.first() {
        Some(p) => p,
        None => return Ok(format!("https://mc-heads.net/head/{}/64", username)),
    };

    // Get session with textures
    let session_res = client
        .get(format!("https://authserver.ely.by/api/authlib-injector/sessionserver/session/minecraft/profile/{}", profile.id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let session_profile: ElybySessionProfile = session_res.json().await.map_err(|e| e.to_string())?;
    let textures_prop = match session_profile.properties.iter().find(|p| p.name == "textures") {
        Some(p) => p,
        None => return Ok(format!("https://mc-heads.net/head/{}/64", username)),
    };

    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD.decode(&textures_prop.value).map_err(|e| e.to_string())?;
    let decoded_str = String::from_utf8_lossy(&decoded);
    let textures_val: ElybyTexturesValue = serde_json::from_str(&decoded_str).map_err(|e| e.to_string())?;
    let skin_url = match textures_val.textures.skin {
        Some(s) => s.url,
        None => return Ok(format!("https://mc-heads.net/head/{}/64", username)),
    };

    // mc-heads.net supports custom skin via ?skin= parameter for head render
    Ok(format!("https://mc-heads.net/face/{}/64?skin={}", username, skin_url))
}

#[tauri::command]
pub async fn get_elyby_head_data_url(username: &str) -> Result<String, String> {
    use base64::Engine;
    let bytes = match get_elyby_skin_bytes(username).await? {
        Some(b) => b,
        None => return Ok(format!("https://mc-heads.net/head/{}/64", username)),
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{}", encoded))
}

pub async fn get_elyby_skin_bytes(username: &str) -> Result<Option<Vec<u8>>, String> {
    let client = Client::new();
    // 1. Get UUID
    let profiles_res = client
        .post("https://authserver.ely.by/api/profiles/minecraft")
        .json(&vec![username])
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let profiles: Vec<ElybyProfile> = profiles_res.json().await.map_err(|e| e.to_string())?;
    let profile = match profiles.first() {
        Some(p) => p,
        None => return Ok(None),
    };
    
    // 2. Get Session Profile
    let session_res = client
        .get(format!("https://authserver.ely.by/api/authlib-injector/sessionserver/session/minecraft/profile/{}", profile.id))
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let session_profile: ElybySessionProfile = session_res.json().await.map_err(|e| e.to_string())?;
    
    // 3. Extract Textures URL
    let textures_prop = match session_profile.properties.iter().find(|p| p.name == "textures") {
        Some(p) => p,
        None => return Ok(None),
    };
    
    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD.decode(&textures_prop.value).map_err(|e| e.to_string())?;
    let decoded_str = String::from_utf8_lossy(&decoded);
    let textures_val: ElybyTexturesValue = serde_json::from_str(&decoded_str).map_err(|e| e.to_string())?;
    
    let skin_url = match textures_val.textures.skin {
        Some(s) => s.url,
        None => return Ok(None),
    };
    
    // 4. Download Skin
    let skin_res = client.get(&skin_url).send().await.map_err(|e| e.to_string())?;
    let bytes = skin_res.bytes().await.map_err(|e| e.to_string())?.to_vec();
    
    Ok(Some(bytes))
}
