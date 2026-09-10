#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adblock;
mod commands;
mod crypto;
mod database;
mod dns;
mod extensions;

use adblock::ShieldEngine;
use database::DbManager;

fn main() {
    env_logger::init();

    let db = DbManager::init();
    let shield = ShieldEngine::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(db)
        .manage(shield)
        .invoke_handler(tauri::generate_handler![
            commands::check_shield,
            commands::set_shield_level,
            commands::resolve_url,
            commands::record_history,
            commands::fetch_history,
            commands::clear_history,
            commands::save_bookmark,
            commands::fetch_bookmarks,
            commands::remove_bookmark,
            commands::fetch_downloads,
            commands::clear_downloads,
            commands::open_file_manager,
            commands::fetch_extensions,
            commands::load_unpacked_extension,
            commands::toggle_extension,
            commands::remove_extension,
            commands::test_doh,
            commands::vault_is_configured,
            commands::vault_setup,
            commands::vault_save_credential,
            commands::vault_read_all,
            commands::vault_delete,
            commands::generate_password,
            commands::get_settings,
            commands::update_setting,
            commands::toggle_devtools
        ])
        .run(tauri::generate_context!())
        .expect("Caram Browser launch failure");
}
