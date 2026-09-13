mod minecraft;
mod launcher;
mod settings;
mod java;
mod server;
mod screenshot;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_minecraft_versions() -> Result<Vec<minecraft::VersionEntry>, String> {
    minecraft::fetch_versions().await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(server::ServerState::default())
        .manage(server::TunnelState::default())
        .invoke_handler(tauri::generate_handler![
            greet, 
            get_minecraft_versions, 
            launcher::launch_game,
            settings::get_settings,
            settings::save_settings,
            settings::add_account,
            settings::switch_account,
            settings::delete_account,
            settings::get_accounts,
            settings::create_instance,
            settings::switch_instance,
            settings::delete_instance,
            settings::get_instances,
            settings::get_app_version,
            settings::download_mod_to_instance,
            settings::download_shader_to_instance,
            settings::copy_local_skin,
            settings::get_local_skin_data,
            settings::get_instance_mods,
            settings::get_latest_crash_log,
            settings::toggle_mod,
            settings::delete_mod,
            java::get_installed_java,
            java::install_java,
            server::start_server,
            server::stop_server,
            server::get_local_ip,
            server::start_tunnel,
            server::stop_tunnel,
            server::download_server_plugin,
            server::get_server_plugins,
            server::delete_server_plugin,
            server::send_console_command,
            server::get_server_properties,
            server::save_server_properties,
            screenshot::get_screenshots,
            screenshot::get_screenshot_base64,
            screenshot::delete_screenshot,
            screenshot::open_screenshot_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
