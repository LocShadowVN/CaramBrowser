use crate::adblock::ShieldEngine;
use crate::crypto::CryptoEngine;
use crate::database::DbManager;
use crate::dns::DnsResolver;
use crate::extensions::ExtensionEngine;
use shared::{
    AppConfig, BookmarkRecord, DecryptedVaultRecord, DnsTestResult, DownloadRecord, ExtensionItem,
    HistoryRecord, ShieldLevel, ShieldVerdict,
};
use std::process::Command;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn check_shield(shield: State<'_, ShieldEngine>, target: String, host: String) -> ShieldVerdict {
    shield.inspect_url(&target, &host).await
}

#[tauri::command]
pub fn set_shield_level(shield: State<ShieldEngine>, level: String) -> Result<(), String> {
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
    if input.starts_with("caram://") || input.starts_with("about:") {
        return input.to_string();
    }
    if input.starts_with("http://") || input.starts_with("https://") {
        return input.to_string();
    }
    if input.contains('.') && !input.contains(' ') {
        return format!("https://{}", input);
    }
    let encoded = url::form_urlencoded::byte_serialize(input.as_bytes()).collect::<String>();
    format!("{}{}", engine, encoded)
}

#[tauri::command]
pub fn record_history(db: State<DbManager>, url: String, title: String) -> Result<(), String> {
    db.insert_history(&url, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_history(db: State<DbManager>) -> Result<Vec<HistoryRecord>, String> {
    db.fetch_history().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_history(db: State<DbManager>) -> Result<(), String> {
    db.wipe_history().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_bookmark(db: State<DbManager>, url: String, title: String) -> Result<(), String> {
    db.insert_bookmark(&url, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_bookmarks(db: State<DbManager>) -> Result<Vec<BookmarkRecord>, String> {
    db.fetch_bookmarks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_bookmark(db: State<DbManager>, id: i64) -> Result<(), String> {
    db.delete_bookmark(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_downloads(db: State<DbManager>) -> Result<Vec<DownloadRecord>, String> {
    db.fetch_downloads().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_downloads(db: State<DbManager>) -> Result<(), String> {
    db.wipe_downloads().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_file_manager(path: String) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn fetch_extensions(db: State<DbManager>) -> Result<Vec<ExtensionItem>, String> {
    db.fetch_extensions().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_unpacked_extension(db: State<DbManager>, folder_path: String) -> Result<ExtensionItem, String> {
    let item = ExtensionEngine::parse_manifest(&folder_path)?;
    db.save_extension(&item).map_err(|e| e.to_string())?;
    Ok(item)
}

#[tauri::command]
pub fn toggle_extension(db: State<DbManager>, id: String, enabled: bool) -> Result<(), String> {
    db.set_extension_state(&id, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_extension(db: State<DbManager>, id: String) -> Result<(), String> {
    db.remove_extension(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_doh(url: String) -> DnsTestResult {
    DnsResolver::ping_test(&url).await
}

#[tauri::command]
pub fn vault_is_configured(db: State<DbManager>) -> bool {
    db.get_master_hash().is_some()
}

#[tauri::command]
pub fn vault_setup(db: State<DbManager>, master_pass: String) -> Result<(), String> {
    if master_pass.len() < 8 {
        return Err("Password must be at least 8 characters".into());
    }
    let hash = CryptoEngine::hash_master_password(&master_pass)?;
    db.set_master_hash(&hash).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn vault_save_credential(
    db: State<DbManager>,
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
    db: State<DbManager>,
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
pub fn vault_delete(db: State<DbManager>, id: i64) -> Result<(), String> {
    db.delete_vault_row(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn generate_password(length: usize) -> String {
    CryptoEngine::generate_strong_password(length)
}

#[tauri::command]
pub fn get_settings(db: State<DbManager>) -> AppConfig {
    db.load_config()
}

#[tauri::command]
pub fn update_setting(db: State<DbManager>, key: String, value: String) -> Result<(), String> {
    db.save_config_item(&key, &value).map_err(|e| e.to_string())
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
