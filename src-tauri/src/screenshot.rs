use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::command;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotInfo {
    pub filename: String,
    pub size: u64,
    pub modified: u64,
}

fn get_screenshots_dir(instance_id: &str) -> PathBuf {
    // Screenshots are stored per-instance under base dir (same as AGENTS.md note #2)
    crate::settings::get_base_dir().join("instances").join(instance_id).join("screenshots")
}

#[command]
pub async fn get_screenshots(instance_id: String) -> Result<Vec<ScreenshotInfo>, String> {
    let dir = get_screenshots_dir(&instance_id);
    let mut screenshots = Vec::new();

    if dir.exists() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            if ext.to_string_lossy().to_lowercase() == "png" {
                                if let Ok(metadata) = entry.metadata() {
                                    let modified = metadata.modified()
                                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs();
                                        
                                    screenshots.push(ScreenshotInfo {
                                        filename: entry.file_name().to_string_lossy().to_string(),
                                        size: metadata.len(),
                                        modified,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Sort by newest first
    screenshots.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(screenshots)
}

#[command]
pub async fn get_screenshot_base64(instance_id: String, filename: String) -> Result<String, String> {
    let dir = get_screenshots_dir(&instance_id);
    let path = dir.join(&filename);
    
    if path.exists() {
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let b64 = general_purpose::STANDARD.encode(&bytes);
        Ok(format!("data:image/png;base64,{}", b64))
    } else {
        Err("Screenshot not found".into())
    }
}

#[command]
pub async fn delete_screenshot(instance_id: String, filename: String) -> Result<(), String> {
    let dir = get_screenshots_dir(&instance_id);
    let path = dir.join(&filename);
    
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub async fn open_screenshot_folder(instance_id: String) -> Result<(), String> {
    let dir = get_screenshots_dir(&instance_id);
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}
