#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adblock;
mod bridge;
mod commands;
mod crypto;
mod database;
mod dns;
mod downloader;
mod extensions;

use adblock::ShieldEngine;
use commands::{VaultSession, ViewportManager};
use database::DbManager;
use tauri::webview::WebviewBuilder;
use tauri::window::WindowBuilder;
use tauri::{LogicalPosition, LogicalSize, PhysicalSize, WebviewUrl};

fn main() {
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    env_logger::init();

    let db = DbManager::init();
    let shield = ShieldEngine::new();
    let vp_manager = ViewportManager::new();
    let vault_session = VaultSession::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(db)
        .manage(shield)
        .manage(vp_manager)
        .manage(vault_session)
        .setup(|app| {
            let window = WindowBuilder::new(app, "main")
                .title("Vibird Browser")
                .inner_size(1400.0, 900.0)
                .min_inner_size(950.0, 650.0)
                .resizable(true)
                .build()?;

            let scale = window.scale_factor().unwrap_or(1.0);
            let phys_size = window.inner_size().unwrap_or(PhysicalSize::new(1400, 900));
            let logical_size = phys_size.to_logical::<f64>(scale);

            let ui_webview = WebviewBuilder::new(
                "ui_chrome",
                WebviewUrl::default(),
            ).auto_resize();

            window.add_child(
                ui_webview,
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(logical_size.width, logical_size.height),
            )?;

            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Resized(phys) = event {
                    let handle = app_handle.clone();
                    let p_size = *phys;
                    tauri::async_runtime::spawn(async move {
                        let _ = crate::commands::handle_window_resize(&handle, p_size).await;
                    });
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_version,
            commands::check_for_updates,
            commands::apply_update,
            commands::restart_browser,
            commands::open_native_tab,
            commands::switch_tab_view,
            commands::close_native_tab,
            commands::snooze_tab,
            commands::expand_ui_for_menu,
            commands::get_site_shield,
            commands::toggle_site_shield,
            commands::start_multithread_download,
            commands::check_vault_credentials_for_domain,
            commands::execute_autofill,
            commands::webview_go_back,
            commands::webview_go_forward,
            commands::webview_reload,
            commands::find_in_page,
            commands::clear_site_data,
            commands::report_tab_title,
            commands::check_shield,
            commands::set_shield_level,
            commands::resolve_url,
            commands::fetch_web_page,
            commands::record_history,
            commands::fetch_history,
            commands::clear_history,
            commands::save_bookmark,
            commands::fetch_bookmarks,
            commands::remove_bookmark,
            commands::fetch_downloads,
            commands::clear_downloads,
            commands::remove_download,
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
            commands::get_shield_stats,
            commands::increment_blocked_stat,
            commands::toggle_devtools
        ])
        .run(tauri::generate_context!())
        .expect("Vibird Browser launch failure");
}
