// main.rs — Tauri v2 application entry point for MemonBook ERP

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_android_device_info,
            commands::get_ios_device_info,
            commands::get_desktop_device_hash,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MemonBook ERP");
}
