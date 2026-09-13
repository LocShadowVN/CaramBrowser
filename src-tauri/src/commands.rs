use crate::adblock::ShieldEngine;
use crate::crypto::CryptoEngine;
use crate::database::DbManager;
use crate::dns::DnsResolver;
use crate::extensions::ExtensionEngine;
use shared::{
    AppConfig, BookmarkRecord, DecryptedVaultRecord, DnsTestResult, DownloadRecord, ExtensionItem,
    HistoryRecord, PageContentResponse, ShieldLevel, ShieldStats, ShieldVerdict,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tauri::{
    webview::{WebviewBuilder, WebviewUrl},
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalSize, State,
};

pub const NAV_BAR_HEIGHT: f64 = 104.0;

pub struct ViewportManager {
    pub active_tab: Mutex<String>,
    pub is_internal: Mutex<bool>,
    pub menu_expanded: Mutex<bool>,
}

impl ViewportManager {
    pub fn new() -> Self {
        Self {
            active_tab: Mutex::new(String::new()),
            is_internal: Mutex::new(true),
            menu_expanded: Mutex::new(false),
        }
    }
}

pub async fn handle_window_resize(app: &AppHandle, phys_size: PhysicalSize<u32>) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let logical = phys_size.to_logical::<f64>(scale);

    let vp_state = app.state::<ViewportManager>();
    let is_internal = *vp_state.is_internal.lock().unwrap();
    let menu_expanded = *vp_state.menu_expanded.lock().unwrap();
    let active_id = vp_state.active_tab.lock().unwrap().clone();

    if let Some(ui_wv) = app.get_webview("ui_chrome") {
        let ui_height = if is_internal || menu_expanded {
            logical.height
        } else {
            NAV_BAR_HEIGHT
        };
        let _ = ui_wv.set_size(LogicalSize::new(logical.width, ui_height));
    }

    if !is_internal && !active_id.is_empty() {
        if let Some(content_wv) = app.get_webview(&active_id) {
            let content_height = (logical.height - NAV_BAR_HEIGHT).max(100.0);
            let _ = content_wv.set_position(LogicalPosition::new(0.0, NAV_BAR_HEIGHT));
            let _ = content_wv.set_size(LogicalSize::new(logical.width, content_height));
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn open_native_tab(
    app: AppHandle,
    shield: State<'_, ShieldEngine>,
    vp: State<'_, ViewportManager>,
    db: State<'_, DbManager>,
    tab_id: String,
    url: String,
) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let phys_size = window.inner_size().unwrap_or(PhysicalSize::new(1400, 900));
    let logical = phys_size.to_logical::<f64>(scale);

    let parsed_url = url::Url::parse(&url).map_err(|e| e.to_string())?;

    {
        let mut act = vp.active_tab.lock().unwrap();
        *act = tab_id.clone();
        let mut internal = vp.is_internal.lock().unwrap();
        *internal = false;
        let mut menu = vp.menu_expanded.lock().unwrap();
        *menu = false;
    }

    if let Some(ui_wv) = app.get_webview("ui_chrome") {
        let _ = ui_wv.set_size(LogicalSize::new(logical.width, NAV_BAR_HEIGHT));
    }

    let content_height = (logical.height - NAV_BAR_HEIGHT).max(100.0);
    let content_pos = LogicalPosition::new(0.0, NAV_BAR_HEIGHT);
    let content_size = LogicalSize::new(logical.width, content_height);

    if let Some(wv) = app.get_webview(&tab_id) {
        let _ = wv.set_position(content_pos);
        let _ = wv.set_size(content_size);
        let _ = wv.show();
        let _ = wv.set_focus();
        wv.navigate(parsed_url).map_err(|e| e.to_string())?;
    } else {
        let init_script = shield.get_injected_script();
        let wv_builder = WebviewBuilder::new(&tab_id, WebviewUrl::External(parsed_url))
            .initialization_script(&init_script);

        let wv = window.add_child(wv_builder, content_pos, content_size)
            .map_err(|e| e.to_string())?;
        let _ = wv.set_focus();
    }

    let _ = db.insert_history(&url, &url);
    Ok(())
}

#[tauri::command]
pub async fn switch_tab_view(
    app: AppHandle,
    vp: State<'_, ViewportManager>,
    active_tab_id: String,
    is_internal: bool,
    all_tab_ids: Vec<String>,
) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let phys_size = window.inner_size().unwrap_or(PhysicalSize::new(1400, 900));
    let logical = phys_size.to_logical::<f64>(scale);

    {
        let mut act = vp.active_tab.lock().unwrap();
        *act = active_tab_id.clone();
        let mut internal = vp.is_internal.lock().unwrap();
        *internal = is_internal;
        let mut menu = vp.menu_expanded.lock().unwrap();
        *menu = false;
    }

    if let Some(ui_wv) = app.get_webview("ui_chrome") {
        let ui_height = if is_internal { logical.height } else { NAV_BAR_HEIGHT };
        let _ = ui_wv.set_size(LogicalSize::new(logical.width, ui_height));
    }

    for id in all_tab_ids {
        if let Some(wv) = app.get_webview(&id) {
            if !is_internal && id == active_tab_id {
                let content_height = (logical.height - NAV_BAR_HEIGHT).max(100.0);
                let _ = wv.set_position(LogicalPosition::new(0.0, NAV_BAR_HEIGHT));
                let _ = wv.set_size(LogicalSize::new(logical.width, content_height));
                let _ = wv.show();
                let _ = wv.set_focus();
            } else {
                let _ = wv.hide();
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn close_native_tab(
    app: AppHandle,
    vp: State<'_, ViewportManager>,
    tab_id: String,
) -> Result<(), String> {
    if let Some(wv) = app.get_webview(&tab_id) {
        let _ = wv.close();
    }
    let mut act = vp.active_tab.lock().unwrap();
    if *act == tab_id {
        act.clear();
    }
    Ok(())
}

#[tauri::command]
pub async fn expand_ui_for_menu(
    app: AppHandle,
    vp: State<'_, ViewportManager>,
    expanded: bool,
) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let phys_size = window.inner_size().unwrap_or(PhysicalSize::new(1400, 900));
    let logical = phys_size.to_logical::<f64>(scale);

    let is_internal = *vp.is_internal.lock().unwrap();
    {
        let mut menu = vp.menu_expanded.lock().unwrap();
        *menu = expanded;
    }

    if let Some(ui_wv) = app.get_webview("ui_chrome") {
        let ui_height = if is_internal || expanded {
            logical.height
        } else {
            NAV_BAR_HEIGHT
        };
        let _ = ui_wv.set_size(LogicalSize::new(logical.width, ui_height));
    }

    Ok(())
}

#[tauri::command]
pub async fn check_shield(
    shield: State<'_, ShieldEngine>,
    target: String,
    host: String,
) -> Result<ShieldVerdict, String> {
    Ok(shield.inspect_url(&target, &host).await)
}

#[tauri::command]
pub fn set_shield_level(shield: State<'_, ShieldEngine>, level: String) -> Result<(), String> {
    let mode = match level.as_str() {
        "Off" => ShieldLevel::Off,
        "Aggressive" => ShieldLevel::Aggressive,
        _ => ShieldLevel::Standard,
    };
    shield.set_level(mode);
    Ok(())
}

#[tauri::command]
pub fn resolve_url(raw: String, engine: String) -> String {
    let input = raw.trim();
    if input.is_empty() {
        return "caram://newtab".to_string();
    }
    if input.starts_with("caram://") || input.starts_with("about:") {
        return input.to_string();
    }
    if input.starts_with("http://") || input.starts_with("https://") {
        return input.to_string();
    }
    if input.starts_with("localhost") || input.starts_with("127.0.0.1") {
        return format!("http://{}", input);
    }
    if input.contains('.') && !input.contains(' ') {
        return format!("https://{}", input);
    }
    let encoded = url::form_urlencoded::byte_serialize(input.as_bytes()).collect::<String>();
    if engine.contains("%s") {
        engine.replace("%s", &encoded)
    } else {
        format!("{}{}", engine, encoded)
    }
}

#[tauri::command]
pub async fn fetch_web_page(
    shield: State<'_, ShieldEngine>,
    db: State<'_, DbManager>,
    url: String,
) -> Result<PageContentResponse, String> {
    let verdict = shield.inspect_url(&url, &url).await;
    if verdict.blocked {
        return Ok(PageContentResponse {
            final_url: url,
            title: "Blocked by Caram Shield".into(),
            html: "<h1>Blocked</h1>".into(),
            blocked_count: 1,
            status: 403,
        });
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36 Caram/1.0.0")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let final_url = resp.url().to_string();
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let _ = db.insert_history(&final_url, &final_url);

    Ok(PageContentResponse {
        final_url,
        title: "Web Resource".into(),
        html: text,
        blocked_count: 0,
        status,
    })
}

#[tauri::command]
pub fn record_history(db: State<'_, DbManager>, url: String, title: String) -> Result<(), String> {
    db.insert_history(&url, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_history(db: State<'_, DbManager>) -> Result<Vec<HistoryRecord>, String> {
    db.fetch_history().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_history(db: State<'_, DbManager>) -> Result<(), String> {
    db.wipe_history().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_bookmark(db: State<'_, DbManager>, url: String, title: String) -> Result<(), String> {
    db.insert_bookmark(&url, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_bookmarks(db: State<'_, DbManager>) -> Result<Vec<BookmarkRecord>, String> {
    db.fetch_bookmarks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_bookmark(db: State<'_, DbManager>, id: i64) -> Result<(), String> {
    db.delete_bookmark(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_downloads(db: State<'_, DbManager>) -> Result<Vec<DownloadRecord>, String> {
    db.fetch_downloads().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_downloads(db: State<'_, DbManager>) -> Result<(), String> {
    db.wipe_downloads().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_download(db: State<'_, DbManager>, id: i64) -> Result<(), String> {
    db.delete_download(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_file_manager(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    let target = if p.is_file() {
        p.parent().unwrap_or(p)
    } else {
        p
    };

    Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn fetch_extensions(db: State<'_, DbManager>) -> Result<Vec<ExtensionItem>, String> {
    db.fetch_extensions().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_unpacked_extension(db: State<'_, DbManager>, folder_path: String) -> Result<ExtensionItem, String> {
    let item = ExtensionEngine::parse_manifest(&folder_path)?;
    db.save_extension(&item).map_err(|e| e.to_string())?;
    Ok(item)
}

#[tauri::command]
pub fn toggle_extension(db: State<'_, DbManager>, id: String, enabled: bool) -> Result<(), String> {
    db.set_extension_state(&id, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_extension(db: State<'_, DbManager>, id: String) -> Result<(), String> {
    db.remove_extension(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_doh(url: String) -> DnsTestResult {
    DnsResolver::ping_test(&url).await
}

#[tauri::command]
pub fn vault_is_configured(db: State<'_, DbManager>) -> bool {
    db.get_master_hash().is_some()
}

#[tauri::command]
pub fn vault_setup(db: State<'_, DbManager>, master_pass: String) -> Result<(), String> {
    if master_pass.len() < 8 {
        return Err("Password must be at least 8 characters".into());
    }
    let hash = CryptoEngine::hash_master_password(&master_pass)?;
    db.set_master_hash(&hash).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn vault_save_credential(
    db: State<'_, DbManager>,
    master_pass: String,
    website: String,
    username: String,
    secret: String,
) -> Result<(), String> {
    let hash = db.get_master_hash().ok_or("Vault not initialized")?;
    if !CryptoEngine::verify_master_password(&master_pass, &hash) {
        return Err("Authentication failed: Wrong password".into());
    }
    let (cipher, nonce, salt) = CryptoEngine::encrypt_secret(&master_pass, &secret)?;
    db.insert_vault_row(&website, &username, &cipher, &nonce, &salt)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn vault_read_all(
    db: State<'_, DbManager>,
    master_pass: String,
) -> Result<Vec<DecryptedVaultRecord>, String> {
    let hash = db.get_master_hash().ok_or("Vault not initialized")?;
    if !CryptoEngine::verify_master_password(&master_pass, &hash) {
        return Err("Authentication failed: Wrong password".into());
    }

    let rows = db.list_vault_rows().map_err(|e| e.to_string())?;
    let mut list = Vec::new();

    for r in rows {
        if let Ok(secret) = CryptoEngine::decrypt_secret(&master_pass, &r.ciphertext, &r.nonce, &r.salt) {
            list.push(DecryptedVaultRecord {
                id: r.id,
                website: r.website,
                username: r.username,
                secret,
                created_at: r.created_at,
            });
        }
    }
    Ok(list)
}

#[tauri::command]
pub fn vault_delete(db: State<'_, DbManager>, id: i64) -> Result<(), String> {
    db.delete_vault_row(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn generate_password(length: usize) -> String {
    CryptoEngine::generate_strong_password(length)
}

#[tauri::command]
pub fn get_settings(db: State<'_, DbManager>) -> AppConfig {
    db.load_config()
}

#[tauri::command]
pub fn update_setting(db: State<'_, DbManager>, key: String, value: String) -> Result<(), String> {
    db.save_config_item(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_shield_stats(db: State<'_, DbManager>, shield: State<'_, ShieldEngine>) -> ShieldStats {
    let total = db.get_total_blocked() + shield.get_blocked_count();
    ShieldStats {
        total_blocked: total,
        trackers_blocked: total,
        bandwidth_saved_mb: (total as f64 * 0.08).round(),
        time_saved_secs: (total as f64 * 0.02).round(),
    }
}

#[tauri::command]
pub fn increment_blocked_stat(db: State<'_, DbManager>, shield: State<'_, ShieldEngine>, count: u64) {
    shield.increment_blocked(count);
    db.increment_blocked_stat(count);
}

#[tauri::command]
pub fn toggle_devtools(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_devtools_open() {
            w.close_devtools();
        } else {
            w.open_devtools();
        }
    }
}
