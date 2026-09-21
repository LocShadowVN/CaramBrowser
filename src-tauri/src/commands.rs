use crate::adblock::ShieldEngine;
use crate::crypto::CryptoEngine;
use crate::database::DbManager;
use crate::dns::DnsResolver;
use crate::downloader::DownloadEngine;
use crate::extensions::ExtensionEngine;
use serde::Serialize;
use shared::{
    AppConfig, BookmarkRecord, DecryptedVaultRecord, DnsTestResult, DownloadRecord, ExtensionItem,
    HistoryRecord, PageContentResponse, ShieldLevel, ShieldStats, ShieldVerdict, SiteCredential,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tauri::{
    webview::{DownloadEvent, PageLoadEvent, WebviewBuilder},
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalSize, State, WebviewUrl,
};

pub const NAV_BAR_HEIGHT: f64 = 92.0;

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

pub struct VaultSession {
    pub derived_key: Mutex<Option<[u8; 32]>>,
}

impl VaultSession {
    pub fn new() -> Self {
        Self {
            derived_key: Mutex::new(None),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct PageNavigationState {
    pub tab_id: String,
    pub url: String,
    pub title: Option<String>,
    pub is_loading: bool,
}

pub fn de_amp_url(url_str: &str) -> String {
    if let Ok(u) = url::Url::parse(url_str) {
        if u.host_str() == Some("www.google.com") && u.path().starts_with("/amp/s/") {
            let real_url = &u.path()["/amp/s/".len()..];
            let scheme = if real_url.starts_with("http") { "" } else { "https://" };
            return format!("{}{}", scheme, real_url);
        }
        if let Some(host) = u.host_str() {
            if host.ends_with(".cdn.ampproject.org") {
                if let Some(pos) = u.path().find("/s/") {
                    let real_url = &u.path()[pos + 3..];
                    return format!("https://{}", real_url);
                }
            }
        }
    }
    url_str.to_string()
}

pub fn strip_tracking_parameters(url_str: &str) -> String {
    let de_amped = de_amp_url(url_str);
    let Ok(mut parsed_url) = url::Url::parse(&de_amped) else {
        return de_amped;
    };

    if parsed_url.query().is_none() {
        return parsed_url.to_string();
    }

    const TRACKING_KEYS: &[&str] = &[
        "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content",
        "utm_id", "utm_source_platform", "utm_creative",
        "fbclid", "gclid", "gbraid", "wbraid", "msclkid",
        "mc_eid", "_ga", "_gl", "yclid", "igshid", "si", "ref_src", "ref_url",
        "dclid", "twclid", "spm", "_hsenc", "_hsmi", "mkt_tok"
    ];

    let clean_pairs: Vec<(String, String)> = parsed_url
        .query_pairs()
        .filter(|(k, _)| !TRACKING_KEYS.contains(&k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    parsed_url.set_query(None);
    if !clean_pairs.is_empty() {
        let mut serializer = parsed_url.query_pairs_mut();
        for (k, v) in clean_pairs {
            serializer.append_pair(&k, &v);
        }
    }

    parsed_url.to_string()
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
pub async fn start_multithread_download(
    app: AppHandle,
    db: State<'_, DbManager>,
    url: String,
    connections: Option<usize>,
) -> Result<String, String> {
    let config = db.load_config();
    let save_dir = PathBuf::from(&config.download_path);
    let _ = tokio::fs::create_dir_all(&save_dir).await;

    DownloadEngine::start_download(
        app,
        url,
        save_dir,
        None,
        connections.unwrap_or(8),
    ).await
}

#[tauri::command]
pub fn check_vault_credentials_for_domain(
    db: State<'_, DbManager>,
    session: State<'_, VaultSession>,
    domain: String,
) -> Result<Vec<SiteCredential>, String> {
    let key_guard = session.derived_key.lock().unwrap();
    let Some(key) = *key_guard else {
        return Ok(Vec::new());
    };

    let rows = db.list_vault_rows().map_err(|e| e.to_string())?;
    let mut matches = Vec::new();

    for r in rows {
        if r.website.to_lowercase().contains(&domain.to_lowercase()) {
            if let Ok(secret) = CryptoEngine::decrypt_with_derived_key(&key, &r.ciphertext, &r.nonce) {
                matches.push(SiteCredential {
                    username: r.username,
                    secret,
                });
            }
        }
    }

    Ok(matches)
}

#[tauri::command]
pub async fn execute_autofill(
    app: AppHandle,
    vp: State<'_, ViewportManager>,
    username: String,
    secret: String,
) -> Result<(), String> {
    let active_id = vp.active_tab.lock().unwrap().clone();
    if active_id.is_empty() {
        return Err("No active tab".into());
    }

    let Some(wv) = app.get_webview(&active_id) else {
        return Err("Webview not found".into());
    };

    let user_json = serde_json::to_string(&username).map_err(|e| e.to_string())?;
    let secret_json = serde_json::to_string(&secret).map_err(|e| e.to_string())?;

    let eval_script = format!(
        r#"if (window.__CARAM_AUTOFILL) {{ window.__CARAM_AUTOFILL({}, {}); }}"#,
        user_json, secret_json
    );

    wv.eval(&eval_script).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn webview_go_back(app: AppHandle, vp: State<'_, ViewportManager>) -> Result<(), String> {
    let active_id = vp.active_tab.lock().unwrap().clone();
    if let Some(wv) = app.get_webview(&active_id) {
        wv.eval("window.history.back()").map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn webview_go_forward(app: AppHandle, vp: State<'_, ViewportManager>) -> Result<(), String> {
    let active_id = vp.active_tab.lock().unwrap().clone();
    if let Some(wv) = app.get_webview(&active_id) {
        wv.eval("window.history.forward()").map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn webview_reload(app: AppHandle, vp: State<'_, ViewportManager>, hard: bool) -> Result<(), String> {
    let active_id = vp.active_tab.lock().unwrap().clone();
    if let Some(wv) = app.get_webview(&active_id) {
        if hard {
            wv.eval("window.location.reload(true)").map_err(|e| e.to_string())?;
        } else {
            wv.eval("window.location.reload()").map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Tìm kiếm trong trang an toàn (Find in page - Ctrl + F)
#[tauri::command]
pub async fn find_in_page(
    app: AppHandle,
    vp: State<'_, ViewportManager>,
    query: String,
    forward: bool,
) -> Result<bool, String> {
    let active_id = vp.active_tab.lock().unwrap().clone();
    let Some(wv) = app.get_webview(&active_id) else {
        return Ok(false);
    };

    let safe_query = serde_json::to_string(&query).map_err(|e| e.to_string())?;
    let backwards = !forward;

    let eval_script = format!(
        r#"window.find({}, false, {}, true, false, true, false);"#,
        safe_query, backwards
    );

    wv.eval(&eval_script).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Xoá sạch Cookie và Bộ nhớ đệm của tên miền hiện tại
#[tauri::command]
pub async fn clear_site_data(
    app: AppHandle,
    vp: State<'_, ViewportManager>,
) -> Result<(), String> {
    let active_id = vp.active_tab.lock().unwrap().clone();
    let Some(wv) = app.get_webview(&active_id) else {
        return Err("No active webview".into());
    };

    let script = r#"
        try {
            localStorage.clear();
            sessionStorage.clear();
            document.cookie.split(";").forEach(function(c) {
                document.cookie = c.replace(/^ +/, "").replace(/=.*/, "=;expires=" + new Date().toUTCString() + ";path=/");
            });
            window.location.reload();
        } catch(e) {}
    "#;

    wv.eval(script).map_err(|e| e.to_string())?;
    Ok(())
}

/// Báo cáo tiêu đề tài liệu từ Webview con về giao diện chính
#[tauri::command]
pub fn report_tab_title(app: AppHandle, tab_id: String, title: String, url: String) {
    let _ = app.emit("tab-navigation-state", PageNavigationState {
        tab_id,
        url,
        title: Some(title),
        is_loading: false,
    });
}

#[tauri::command]
pub async fn open_native_tab(
    app: AppHandle,
    shield: State<'_, ShieldEngine>,
    vp: State<'_, ViewportManager>,
    db: State<'_, DbManager>,
    tab_id: String,
    url: String,
    is_incognito: Option<bool>,
) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let phys_size = window.inner_size().unwrap_or(PhysicalSize::new(1400, 900));
    let logical = phys_size.to_logical::<f64>(scale);

    let clean_url = strip_tracking_parameters(&url);
    let parsed_url = url::Url::parse(&clean_url).map_err(|e| e.to_string())?;

    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err("Blocked insecure URL protocol".into());
    }

    let domain = parsed_url.host_str().unwrap_or("").to_string();
    let shield_enabled = db.get_site_shield_status(&domain).unwrap_or(true);
    let incognito = is_incognito.unwrap_or(false);

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
        let base_script = if shield_enabled {
            shield.get_injected_script()
        } else {
            crate::bridge::get_webbridge_script().to_string()
        };

        // Inject script đồng bộ tiêu đề và phím tắt từ trang web con về UI
        let init_script = format!(
            r#"
            {}
            (function() {{
                const TAB_ID = "{}";
                function reportTitle() {{
                    if (window.__TAURI__ && window.__TAURI__.core) {{
                        window.__TAURI__.core.invoke('report_tab_title', {{
                            tabId: TAB_ID,
                            title: document.title || window.location.hostname,
                            url: window.location.href
                        }}).catch(() => {{}});
                    }}
                }}
                if (document.readyState === 'loading') {{
                    document.addEventListener('DOMContentLoaded', reportTitle);
                }} else {{
                    reportTitle();
                }}
                window.addEventListener('load', reportTitle);
                new MutationObserver(() => reportTitle()).observe(document.querySelector('title') || document.head, {{
                    subtree: true, characterData: true, childList: true
                }});
            }})();
            "#,
            base_script, tab_id
        );

        let app_handle_for_events = app.clone();
        let tab_id_for_events = tab_id.clone();
        let app_handle_for_dl = app.clone();

        let wv_builder = WebviewBuilder::new(&tab_id, WebviewUrl::External(parsed_url))
            .user_agent(crate::bridge::CHROME_USER_AGENT)
            .initialization_script(&init_script)
            // 1. Đồng bộ trạng thái tải trang (Loading bar)
            .on_page_load(move |_wv, payload| {
                let current_url = payload.url().to_string();
                let is_loading = payload.event() == PageLoadEvent::Started;
                let _ = app_handle_for_events.emit("tab-navigation-state", PageNavigationState {
                    tab_id: tab_id_for_events.clone(),
                    url: current_url,
                    title: None,
                    is_loading,
                });
            })
            // 2. Download Sniffer: Bắt link tự động chuyển sang luồng tải đa luồng IDM
            .on_download(move |_wv, event| {
                match event {
                    DownloadEvent::Requested { url, .. } => {
                        let dl_url = url.to_string();
                        let app_c = app_handle_for_dl.clone();
                        tauri::async_runtime::spawn(async move {
                            let db_c = app_c.state::<DbManager>();
                            let cfg = db_c.load_config();
                            let s_dir = PathBuf::from(&cfg.download_path);
                            let _ = DownloadEngine::start_download(app_c, dl_url, s_dir, None, 8).await;
                        });
                        false // Huỷ trình tải đơn luồng chậm của WebKit
                    }
                    _ => true,
                }
            });

        let wv = window.add_child(wv_builder, content_pos, content_size)
            .map_err(|e| e.to_string())?;
        let _ = wv.set_focus();
    }

    // Nếu là tab ẩn danh: Tuyệt đối KHÔNG lưu lịch sử
    if !incognito {
        let _ = db.insert_history(&clean_url, &clean_url);
    }

    Ok(())
}

#[tauri::command]
pub fn get_site_shield(db: State<'_, DbManager>, domain: String) -> Result<bool, String> {
    db.get_site_shield_status(&domain).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_site_shield(
    app: AppHandle,
    db: State<'_, DbManager>,
    vp: State<'_, ViewportManager>,
    domain: String,
    enabled: bool,
) -> Result<(), String> {
    db.set_site_shield_status(&domain, enabled).map_err(|e| e.to_string())?;

    let active_id = vp.active_tab.lock().unwrap().clone();
    if !active_id.is_empty() {
        if let Some(wv) = app.get_webview(&active_id) {
            if let Ok(cur_url) = wv.url() {
                if cur_url.host_str() == Some(&domain) {
                    let _ = wv.navigate(cur_url);
                }
            }
        }
    }

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
pub async fn snooze_tab(
    app: AppHandle,
    vp: State<'_, ViewportManager>,
    tab_id: String,
) -> Result<(), String> {
    let active_id = vp.active_tab.lock().unwrap().clone();
    if active_id == tab_id {
        return Err("Cannot snooze the active tab".into());
    }

    if let Some(wv) = app.get_webview(&tab_id) {
        let _ = wv.close();
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
        return strip_tracking_parameters(input);
    }
    if input.starts_with("localhost") || input.starts_with("127.0.0.1") {
        return format!("http://{}", input);
    }
    if input.contains('.') && !input.contains(' ') {
        return strip_tracking_parameters(&format!("https://{}", input));
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
        .user_agent(crate::bridge::CHROME_USER_AGENT)
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
    let canonical = p.canonicalize().map_err(|e| e.to_string())?;

    let target_dir = if canonical.is_file() {
        canonical.parent().unwrap_or(&canonical)
    } else {
        &canonical
    };

    Command::new("xdg-open")
        .arg(target_dir)
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
    session: State<'_, VaultSession>,
    master_pass: String,
    website: String,
    username: String,
    secret: String,
) -> Result<(), String> {
    let hash = db.get_master_hash().ok_or("Vault not initialized")?;
    if !CryptoEngine::verify_master_password(&master_pass, &hash) {
        return Err("Authentication failed: Wrong password".into());
    }

    let salt_bytes = b"caram_vault_global_salt_v1";
    let key = CryptoEngine::derive_key(&master_pass, salt_bytes)?;
    let (cipher, nonce) = CryptoEngine::encrypt_with_derived_key(&key, &secret)?;

    db.insert_vault_row(&website, &username, &cipher, &nonce, "v1")
        .map_err(|e| e.to_string())?;

    let mut s = session.derived_key.lock().unwrap();
    *s = Some(key);
    Ok(())
}

#[tauri::command]
pub fn vault_read_all(
    db: State<'_, DbManager>,
    session: State<'_, VaultSession>,
    master_pass: String,
) -> Result<Vec<DecryptedVaultRecord>, String> {
    let hash = db.get_master_hash().ok_or("Vault not initialized")?;
    if !CryptoEngine::verify_master_password(&master_pass, &hash) {
        return Err("Authentication failed: Wrong password".into());
    }

    let salt_bytes = b"caram_vault_global_salt_v1";
    let key = CryptoEngine::derive_key(&master_pass, salt_bytes)?;

    {
        let mut s = session.derived_key.lock().unwrap();
        *s = Some(key);
    }

    let rows = db.list_vault_rows().map_err(|e| e.to_string())?;
    let mut list = Vec::new();

    for r in rows {
        if let Ok(secret) = CryptoEngine::decrypt_with_derived_key(&key, &r.ciphertext, &r.nonce) {
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
