use crate::adblock::ShieldEngine;
use crate::crypto::CryptoEngine;
use crate::database::DbManager;
use crate::dns::DnsResolver;
use crate::extensions::ExtensionEngine;
use shared::{
    AppConfig, BookmarkRecord, DecryptedVaultRecord, DnsTestResult, DownloadRecord, ExtensionItem,
    HistoryRecord, PageContentResponse, ShieldLevel, ShieldStats, ShieldVerdict,
};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use tauri::{
    webview::WebviewBuilder,
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalSize, State, WebviewUrl,
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

    let clean_url = strip_tracking_parameters(&url);
    let parsed_url = url::Url::parse(&clean_url).map_err(|e| e.to_string())?;
    let domain = parsed_url.host_str().unwrap_or("").to_string();
    let shield_enabled = db.get_site_shield_status(&domain).unwrap_or(true);

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
        let init_script = if shield_enabled {
            shield.get_injected_script()
        } else {
            "window.isShieldDisabled = true;".to_string()
        };

        let wv_builder = WebviewBuilder::new(&tab_id, WebviewUrl::External(parsed_url))
            .initialization_script(&init_script);

        let wv = window.add_child(wv_builder, content_pos, content_size)
            .map_err(|e| e.to_string())?;
        let _ = wv.set_focus();
    }

    let _ = db.insert_history(&clean_url, &clean_url);
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
        "Aggressive" => S
