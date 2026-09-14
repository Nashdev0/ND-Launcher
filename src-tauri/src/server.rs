use std::fs;
use std::process::{Command, Child, Stdio};
use std::sync::Mutex;
use tauri::State;
use reqwest;
use std::io::Write;

pub struct ServerState(pub Mutex<Option<Child>>);

impl Default for ServerState {
    fn default() -> Self {
        ServerState(Mutex::new(None))
    }
}

/// Lock a mutex, recovering from poisoning instead of panicking.
fn lock_recover<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn get_server_dir(version: &str) -> std::path::PathBuf {
    let default_dir = crate::settings::get_base_dir().join("game_data").join("server").join(version);
    let settings_path = crate::settings::get_base_dir().join("game_data").join("launcher_settings.json");
    
    if let Ok(contents) = std::fs::read_to_string(&settings_path) {
        if let Ok(settings) = serde_json::from_str::<crate::settings::LauncherSettings>(&contents) {
            if let Some(custom) = settings.custom_server_dir {
                if !custom.trim().is_empty() {
                    // custom_server_dir is the server root (analogous to game_data/server)
                    return std::path::PathBuf::from(custom).join(version);
                }
            }
        }
    }
    
    default_dir
}

#[tauri::command]
pub async fn start_server(
    app: tauri::AppHandle,
    _core: String,
    version: String,
    ram: u32,
    state: State<'_, ServerState>,
) -> Result<(), String> {
    use tauri::Emitter;
    use std::io::{BufRead, BufReader};

    // Check if already running
    {
        let mut child_opt = lock_recover(&state.0);
        if let Some(child) = child_opt.as_mut() {
            if let Ok(None) = child.try_wait() {
                return Err("Server is already running!".to_string());
            }
        }
    }

    let server_dir = get_server_dir(&version);
    if !server_dir.exists() {
        fs::create_dir_all(&server_dir).map_err(|e| format!("Failed to create server directory: {}", e))?;
    }

    let eula_path = server_dir.join("eula.txt");
    fs::write(&eula_path, "eula=true\n").map_err(|e| format!("Failed to write eula.txt: {}", e))?;

    let props_path = server_dir.join("server.properties");
    if !props_path.exists() {
        fs::write(&props_path, "online-mode=false\nserver-port=25565\n").map_err(|e| format!("Failed to write server.properties: {}", e))?;
    }

    let server_jar_path = server_dir.join("server.jar");
    if !server_jar_path.exists() {
        let url = format!("https://api.purpurmc.org/v2/purpur/{}/latest/download", version);
        
        let _ = app.emit("server-log", format!("Downloading Purpur {} from {}...", version, url));
        
        let mut response = reqwest::get(&url).await.map_err(|e| format!("Failed to fetch server jar: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("Failed to download Purpur {}: HTTP {} — versi ini mungkin tidak tersedia di Purpur.", version, response.status()));
        }
        
        let mut file = fs::File::create(&server_jar_path).map_err(|e| format!("Failed to create server.jar: {}", e))?;
        while let Some(chunk) = response.chunk().await.map_err(|e| format!("Failed to read chunk: {}", e))? {
            use std::io::Write;
            file.write_all(&chunk).map_err(|e| format!("Failed to write chunk: {}", e))?;
        }
        let _ = app.emit("server-log", "Download complete.".to_string());
    }

    let _ = app.emit("server-log", format!("Starting server with RAM {} GB", ram));
    
    // Resolve Java path — respect custom_java_path from settings
    let java_bin = if let Ok(settings) = crate::settings::get_settings().await {
        if let Some(ref path) = settings.custom_java_path {
            if !path.trim().is_empty() && std::path::Path::new(path).exists() {
                path.clone()
            } else {
                "java".to_string()
            }
        } else {
            if cfg!(windows) { "java.exe".to_string() } else { "java".to_string() }
        }
    } else {
        if cfg!(windows) { "java.exe".to_string() } else { "java".to_string() }
    };

    // Spawn server process
    let mut child = Command::new(&java_bin)
        .current_dir(&server_dir)
        .arg(format!("-Xmx{}G", ram))
        .arg("-Xms1G")
        .arg("-jar")
        .arg("server.jar")
        .arg("nogui")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start java process: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to grab stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to grab stderr")?;
    
    let app_clone1 = app.clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut buf = String::new();
        while let Ok(bytes) = reader.read_line(&mut buf) {
            if bytes == 0 { break; }
            let _ = app_clone1.emit("server-log", buf.clone());
            buf.clear();
        }
    });

    let app_clone2 = app.clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut buf = String::new();
        while let Ok(bytes) = reader.read_line(&mut buf) {
            if bytes == 0 { break; }
            let _ = app_clone2.emit("server-log", buf.clone());
            buf.clear();
        }
    });

    let app_clone3 = app.clone();
    let cache_dir = server_dir.join("cache");
    std::thread::spawn(move || {
        let mut last_size = 0;
        let mut unchanged_count = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3));
            if !cache_dir.exists() {
                unchanged_count += 1;
                if unchanged_count > 10 { break; } // stop if no cache dir after 30s
                continue;
            }
            
            // find jar file in cache
            let mut current_size = 0;
            if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.ends_with(".jar") {
                            if let Ok(meta) = entry.metadata() {
                                current_size = meta.len();
                            }
                            break;
                        }
                    }
                }
            }
            
            if current_size > 0 {
                if current_size == last_size {
                    unchanged_count += 1;
                    if unchanged_count > 3 {
                        let _ = app_clone3.emit("server-log", "[Progress] Selesai mengunduh file dasar.\n".to_string());
                        break; // size stopped growing, probably done downloading
                    }
                } else {
                    unchanged_count = 0;
                    last_size = current_size;
                    let mb = current_size as f64 / 1_048_576.0;
                    let _ = app_clone3.emit("server-log", format!("[Progress] Mendownload file dasar (mojang.jar): {:.1} MB ...\n", mb));
                }
            }
        }
    });

    let mut state_lock = lock_recover(&state.0);
    *state_lock = Some(child);

    Ok(())
}

#[tauri::command]
pub fn stop_server(state: State<'_, ServerState>) -> Result<(), String> {
    let mut child_opt = lock_recover(&state.0);
    
    if let Some(mut child) = child_opt.take() {
        // Try to gracefully stop via stdin if possible
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"stop\n");
            let _ = stdin.flush();
        }
        
        // Wait a bit or just kill
        let _ = child.kill();
        let _ = child.wait();
        Ok(())
    } else {
        Err("Server is not running.".to_string())
    }
}

#[tauri::command]
pub fn send_console_command(command: String, state: State<'_, ServerState>) -> Result<(), String> {
    let mut child_opt = lock_recover(&state.0);
    if let Some(child) = child_opt.as_mut() {
        if let Some(stdin) = child.stdin.as_mut() {
            let cmd = format!("{}\n", command);
            stdin.write_all(cmd.as_bytes()).map_err(|e| e.to_string())?;
            stdin.flush().map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err("Server is not running.".to_string())
}

pub struct TunnelState(pub Mutex<Option<Child>>);

impl Default for TunnelState {
    fn default() -> Self {
        TunnelState(Mutex::new(None))
    }
}

#[tauri::command]
pub async fn start_tunnel(port: u16, state: State<'_, TunnelState>) -> Result<String, String> {
    // Check if already running
    {
        let mut child_opt = lock_recover(&state.0);
        if let Some(child) = child_opt.as_mut() {
            if let Ok(None) = child.try_wait() {
                // If running, we could return the existing one, but for simplicity let's kill and restart
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    use std::io::{BufRead, BufReader};

    let mut child = Command::new("ssh")
        .args([
            "-p", "443",
            &format!("-R0:localhost:{}", port),
            "-L4300:localhost:4300",
            "tcp@a.pinggy.io",
            "-o", "StrictHostKeyChecking=no",
            "-T"
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start ssh process: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to grab stdout")?;
    let mut reader = BufReader::new(stdout);
    
    // We also need to read stderr because ssh sometimes prints banners to stderr
    let stderr = child.stderr.take().ok_or("Failed to grab stderr")?;
    
    // Let's spawn a thread to read stdout and stderr and find the URL. 
    // Wait, let's just do it sequentially with a timeout or read one by one.
    // Actually, Pinggy prints the URL to stdout usually.
    let (tx, rx) = std::sync::mpsc::channel();
    let tx_err = tx.clone();
    
    std::thread::spawn(move || {
        let mut buf = String::new();
        while let Ok(bytes) = reader.read_line(&mut buf) {
            if bytes == 0 { break; }
            if buf.contains("tcp://") {
                if let Some(start) = buf.find("tcp://") {
                    let url = buf[start..].trim().to_string();
                    let _ = tx.send(url);
                    break;
                }
            }
            buf.clear();
        }
    });
    
    let mut err_reader = BufReader::new(stderr);
    std::thread::spawn(move || {
        let mut buf = String::new();
        while let Ok(bytes) = err_reader.read_line(&mut buf) {
            if bytes == 0 { break; }
            if buf.contains("tcp://") {
                if let Some(start) = buf.find("tcp://") {
                    let url = buf[start..].trim().to_string();
                    let _ = tx_err.send(url);
                    break;
                }
            }
            buf.clear();
        }
    });

    let mut state_lock = lock_recover(&state.0);
    *state_lock = Some(child);

    // Wait up to 10 seconds for the URL
    match rx.recv_timeout(std::time::Duration::from_secs(10)) {
        Ok(url) => Ok(url),
        Err(_) => Err("Timeout waiting for tunnel URL. Please try again.".to_string()),
    }
}

#[tauri::command]
pub fn stop_tunnel(state: State<'_, TunnelState>) -> Result<(), String> {
    let mut child_opt = lock_recover(&state.0);
    if let Some(mut child) = child_opt.take() {
        let _ = child.kill();
        let _ = child.wait();
        Ok(())
    } else {
        Err("Tunnel is not running.".to_string())
    }
}

#[tauri::command]
pub fn get_local_ip() -> Result<String, String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    // Connect to a public DNS (doesn't actually send data)
    socket.connect("8.8.8.8:80").map_err(|e| e.to_string())?;
    let addr = socket.local_addr().map_err(|e| e.to_string())?;
    Ok(format!("{}:25565", addr.ip()))
}

#[tauri::command]
pub async fn download_server_plugin(version: String, download_url: String, file_name: String) -> Result<(), String> {
    let plugins_dir = get_server_dir(&version).join("plugins");
    
    if !plugins_dir.exists() {
        let _ = fs::create_dir_all(&plugins_dir);
    }
    
    let dest_path = plugins_dir.join(&file_name);
    if dest_path.exists() {
        return Err("Plugin already exists!".to_string());
    }
    
    let mut response = reqwest::get(&download_url).await.map_err(|e| format!("Failed to download: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Download HTTP error: {}", response.status()));
    }
    
    let mut file = fs::File::create(&dest_path).map_err(|e| format!("Failed to create plugin file: {}", e))?;
    while let Some(chunk) = response.chunk().await.map_err(|e| format!("Failed to read chunk: {}", e))? {
        use std::io::Write;
        file.write_all(&chunk).map_err(|e| format!("Failed to save plugin: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
pub fn get_server_plugins(version: String) -> Result<Vec<String>, String> {
    let plugins_dir = get_server_dir(&version).join("plugins");
    let mut plugins = Vec::new();
    
    if plugins_dir.exists() {
        if let Ok(entries) = fs::read_dir(plugins_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.ends_with(".jar") || name.ends_with(".jar.disabled") {
                        plugins.push(name);
                    }
                }
            }
        }
    }
    Ok(plugins)
}

#[tauri::command]
pub fn delete_server_plugin(version: String, file_name: String) -> Result<(), String> {
    let plugins_dir = get_server_dir(&version).join("plugins");
    let dest_path = plugins_dir.join(&file_name);
    
    if dest_path.exists() {
        fs::remove_file(dest_path).map_err(|e| format!("Failed to delete plugin: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_server_properties(version: String) -> Result<Vec<(String, String)>, String> {
    let props_path = get_server_dir(&version).join("server.properties");
    if !props_path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(&props_path).map_err(|e| e.to_string())?;
    let mut props = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            props.push((k.to_string(), v.to_string()));
        }
    }
    
    Ok(props)
}

#[tauri::command]
pub fn save_server_properties(version: String, properties: Vec<(String, String)>) -> Result<(), String> {
    use std::collections::{HashMap, HashSet};
    
    let props_path = get_server_dir(&version).join("server.properties");
    
    let content = if props_path.exists() {
        fs::read_to_string(&props_path).unwrap_or_default()
    } else {
        String::new()
    };
    
    let mut props_map: HashMap<String, String> = properties.into_iter().collect();
    let mut new_lines = Vec::new();
    let mut seen_keys = HashSet::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            new_lines.push(line.to_string());
            continue;
        }
        if let Some((k, _)) = trimmed.split_once('=') {
            if let Some(new_val) = props_map.remove(k) {
                new_lines.push(format!("{}={}", k, new_val));
                seen_keys.insert(k.to_string());
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }
    
    // Add any remaining new properties
    for (k, v) in props_map {
        new_lines.push(format!("{}={}", k, v));
    }
    
    fs::write(&props_path, new_lines.join("\n")).map_err(|e| e.to_string())?;
    Ok(())
}
