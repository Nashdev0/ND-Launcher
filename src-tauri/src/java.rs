use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct JavaInstallation {
    pub path: String,
    pub version: String,
    pub major_version: u32,
}

#[tauri::command]
pub async fn get_installed_java() -> Result<Vec<JavaInstallation>, String> {
    let mut installations = Vec::new();

    let mut search_paths = vec![];

    if cfg!(windows) {
        search_paths.push(PathBuf::from("C:\\Program Files\\Java"));
        search_paths.push(PathBuf::from("C:\\Program Files\\Eclipse Adoptium"));
        search_paths.push(PathBuf::from("C:\\Program Files (x86)\\Java"));
    } else {
        search_paths.push(PathBuf::from("/usr/lib/jvm"));
        if let Ok(home) = std::env::var("HOME") {
            search_paths.push(PathBuf::from(home).join(".sdkman/candidates/java"));
        }
    }

    // Add our custom Java paths
    let base_java = crate::settings::get_base_dir().join("java");
    if base_java.exists() {
        search_paths.push(base_java.clone());
        search_paths.push(base_java.join("17"));
        search_paths.push(base_java.join("21"));
        search_paths.push(base_java.join("25"));
    }

    // Add default system java
    if cfg!(windows) {
        if let Ok(path) = which::which("java.exe") {
            check_and_add_java(path, &mut installations).await;
        }
    } else {
        if let Ok(path) = which::which("java") {
            check_and_add_java(path, &mut installations).await;
        }
    }

    for base_dir in search_paths {
        if base_dir.exists() && base_dir.is_dir() {
            if let Ok(mut entries) = tokio::fs::read_dir(base_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let mut java_bin = if cfg!(windows) {
                        entry.path().join("bin").join("java.exe")
                    } else {
                        entry.path().join("bin").join("java")
                    };

                    // Sometimes it's inside a nested folder (like jdk-17.0.2/bin/java)
                    if !java_bin.exists() && entry.path().is_dir() {
                        if let Ok(mut sub_entries) = tokio::fs::read_dir(entry.path()).await {
                            while let Ok(Some(sub)) = sub_entries.next_entry().await {
                                let sub_bin = if cfg!(windows) {
                                    sub.path().join("bin").join("java.exe")
                                } else {
                                    sub.path().join("bin").join("java")
                                };
                                if sub_bin.exists() {
                                    java_bin = sub_bin;
                                    break;
                                }
                            }
                        }
                    }

                    if java_bin.exists() {
                        // Fast path: infer major version from the folder name (no process spawn).
                        // Only spawn `java -version` when we cannot guess from the path.
                        if let Some(major) = infer_major_from_path(&java_bin) {
                            let path_str = java_bin.to_string_lossy().to_string();
                            installations.push(JavaInstallation {
                                path: path_str,
                                version: format!("{}", major),
                                major_version: major,
                            });
                        } else {
                            check_and_add_java(java_bin, &mut installations).await;
                        }
                    }
                }
            }
        }
    }

    installations.dedup_by(|a, b| a.path == b.path);
    Ok(installations)
}

/// Guess the Java major version from the install folder name without spawning a process.
/// Handles names like `jdk-17.0.2`, `jdk-21`, `temurin-25.0.1+8`, `jre1.8.0_381`, `jdk-8u401`.
fn infer_major_from_path(java_bin: &std::path::Path) -> Option<u32> {
    // java_bin = .../<install-dir>/bin/java(.exe)
    let dir = java_bin.parent()?.parent()?;
    let name = dir.file_name()?.to_string_lossy().to_lowercase();

    // Find the first run of digits that is a plausible version start.
    let bytes = name.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            // Collect the whole dotted token, e.g. "1.8.0" or "17.0.2" or "8".
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let token = name[start..i].trim_end_matches('.');
            let mut parts = token.split('.');
            let first = parts.next()?;
            let n = first.parse::<u32>().ok()?;
            if n == 1 {
                // Legacy style: 1.8 → 8
                if let Some(m) = parts.next().and_then(|s| s.parse::<u32>().ok()) {
                    if m >= 6 {
                        return Some(m);
                    }
                }
            } else if (6..=30).contains(&n) {
                return Some(n);
            }
            break;
        }
        i += 1;
    }
    None
}

async fn check_and_add_java(path: PathBuf, list: &mut Vec<JavaInstallation>) {
    let path_str = path.to_string_lossy().to_string();
    if let Ok(output) = tokio::process::Command::new(&path_str)
        .arg("-version")
        .output()
        .await
    {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(version_line) = stderr.lines().find(|l| l.contains("version")) {
            let parts: Vec<&str> = version_line.split('"').collect();
            if parts.len() >= 2 {
                let version_str = parts[1];
                let major = parse_major_version(version_str);
                list.push(JavaInstallation {
                    path: path_str,
                    version: version_str.to_string(),
                    major_version: major,
                });
            }
        }
    }
}

fn parse_major_version(version: &str) -> u32 {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        if parts[0] == "1" {
            parts[1].parse().unwrap_or(0)
        } else {
            parts[0].parse().unwrap_or(0)
        }
    } else {
        0
    }
}

#[tauri::command]
pub async fn install_java(version: u32) -> Result<(), String> {
    let os = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        "aarch64"
    };

    // Download JRE (not JDK to save space) from Adoptium
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jre/hotspot/normal/eclipse",
        version, os, arch
    );

    let res = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let bytes = res.bytes().await.map_err(|e| e.to_string())?;

    let target_dir = crate::settings::get_base_dir()
        .join("java")
        .join(version.to_string());
    if target_dir.exists() {
        let _ = std::fs::remove_dir_all(&target_dir);
    }
    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    if cfg!(windows) {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        archive.extract(&target_dir).map_err(|e| e.to_string())?;
    } else {
        use flate2::read::GzDecoder;
        use tar::Archive;
        let tar = GzDecoder::new(Cursor::new(bytes));
        let mut archive = Archive::new(tar);
        archive.unpack(&target_dir).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn get_best_java_for_version(mc_version: &str) -> Option<String> {
    if let Ok(installations) = get_installed_java().await {
        let required_major = if mc_version.starts_with("26.") || mc_version.starts_with("1.21.2") {
            25
        } else if mc_version.starts_with("1.21")
            || mc_version.starts_with("1.20.5")
            || mc_version.starts_with("1.20.6")
        {
            21
        } else if mc_version.starts_with("1.17")
            || mc_version.starts_with("1.18")
            || mc_version.starts_with("1.19")
            || mc_version.starts_with("1.20")
        {
            17
        } else {
            8
        };

        // Find exact match first
        if let Some(java) = installations
            .iter()
            .find(|j| j.major_version == required_major)
        {
            return Some(java.path.clone());
        }

        // Fallback to any Java that is >= required
        if let Some(java) = installations
            .iter()
            .filter(|j| j.major_version >= required_major)
            .min_by_key(|j| j.major_version)
        {
            return Some(java.path.clone());
        }

        // Fallback to anything
        if let Some(java) = installations.first() {
            return Some(java.path.clone());
        }
    }
    None
}
