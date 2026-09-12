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
pub async fn switch_account(account_id: String) -> Result<(), String> {
    let mut settings = get_settings().await?;
    
    let mut found = false;
    for acc in &mut settings.accounts {
        if acc.id == account_id {
            acc.is_active = true;
            found = true;
        } else {
            acc.is_active = false;
        }
    }
    
    if found {
        settings.active_account_id = Some(account_id);
        save_settings(settings).await?;
        Ok(())
    } else {
        Err("Account not found".into())
    }
}

#[tauri::command]
pub async fn delete_account(account_id: String) -> Result<(), String> {
    let mut settings = get_settings().await?;
    
    settings.accounts.retain(|acc| acc.id != account_id);
    if settings.active_account_id == Some(account_id) {
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
pub async fn switch_instance(instance_id: String) -> Result<(), String> {
    let mut settings = get_settings().await?;
    
    if settings.instances.iter().any(|inst| inst.id == instance_id) {
        settings.active_instance_id = Some(instance_id);
        save_settings(settings).await?;
        Ok(())
    } else {
        Err("Instance not found".into())
    }
}

#[tauri::command]
pub async fn delete_instance(instance_id: String) -> Result<(), String> {
    let mut settings = get_settings().await?;
    
    let instance_dir = get_global_game_dir().await.join("instances").join(&instance_id);
    settings.instances.retain(|inst| inst.id != instance_id);
    if settings.active_instance_id == Some(instance_id.clone()) {
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
pub async fn download_mod_to_instance(instance_id: String, download_url: String, file_name: String) -> Result<(), String> {
    let instance_dir = get_global_game_dir().await.join("instances").join(&instance_id);
    let mods_dir = instance_dir.join("mods");
    
    if !mods_dir.exists() {
        let _ = fs::create_dir_all(&mods_dir).await;
    }
    
    let dest_path = mods_dir.join(&file_name);
    
    if dest_path.exists() {
        return Err("Mod already exists in this instance!".to_string());
    }
    
    let mut response = reqwest::get(&download_url).await.map_err(|e| format!("Failed to download: {}", e))?;
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
pub async fn download_shader_to_instance(instance_id: String, download_url: String, file_name: String) -> Result<(), String> {
    let instance_dir = get_global_game_dir().await.join("instances").join(&instance_id);
    let shader_dir = instance_dir.join("shaderpacks");
    
    if !shader_dir.exists() {
        let _ = std::fs::create_dir_all(&shader_dir);
    }
    
    let dest_path = shader_dir.join(&file_name);
    
    if dest_path.exists() {
        return Err("Shader already exists in this instance!".to_string());
    }
    
    let mut response = reqwest::get(&download_url).await.map_err(|e| format!("Failed to download: {}", e))?;
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
pub async fn get_instance_mods(instance_id: String) -> Result<Vec<ModInfo>, String> {
    let mods_dir = get_global_game_dir().await.join("instances").join(&instance_id).join("mods");
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
pub async fn toggle_mod(instance_id: String, mod_name: String, enabled: bool) -> Result<(), String> {
    let mods_dir = get_global_game_dir().await.join("instances").join(&instance_id).join("mods");
    let target_path = mods_dir.join(&mod_name);
    if !target_path.exists() {
        return Err("Mod file not found".to_string());
    }
    
    if enabled && mod_name.ends_with(".jar.disabled") {
        let new_name = mod_name.replace(".jar.disabled", ".jar");
        let new_path = mods_dir.join(&new_name);
        fs::rename(target_path, new_path).await.map_err(|e| e.to_string())?;
    } else if !enabled && mod_name.ends_with(".jar") {
        let new_name = format!("{}.disabled", mod_name);
        let new_path = mods_dir.join(&new_name);
        fs::rename(target_path, new_path).await.map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn delete_mod(instance_id: String, mod_name: String) -> Result<(), String> {
    let mods_dir = get_global_game_dir().await.join("instances").join(&instance_id).join("mods");
    let target_path = mods_dir.join(&mod_name);
    
    if target_path.exists() {
        fs::remove_file(target_path).await.map_err(|e| e.to_string())?;
    } else {
        return Err("Mod file not found".to_string());
    }
    
    Ok(())
}
