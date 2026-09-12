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
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn check_shield(
    shield: State<'_, ShieldEngine>,
    target: String,
    host: String,
) -> Result<ShieldVerdict, String> {
    Ok(shield.inspect_url(&target, &host).await)
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
        let blocked_html = format!(r#"
            <!DOCTYPE html>
            <html>
            <head><meta charset="utf-8"><title>Blocked by Caram Shield</title>
            <style>
                body {{ background: #0e1013; color: #e6e8eb; font-family: -apple-system, BlinkMacSystemFont, sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
                .card {{ background: #16181d; border: 1px solid #ef4444; border-radius: 12px; padding: 32px; max-width: 480px; text-align: center; box-shadow: 0 10px 30px rgba(0,0,0,0.5); }}
                h1 {{ color: #ef4444; font-size: 22px; margin-bottom: 12px; }}
                p {{ color: #8c929d; font-size: 14px; line-height: 1.6; margin-bottom: 20px; }}
                code {{ background: #20232a; padding: 4px 8px; border-radius: 4px; color: #f97316; font-size: 12px; word-break: break-all; }}
            </style>
            </head>
            <body>
                <div class="card">
                    <h1>Shield Protection Triggered</h1>
                    <p>Caram Shield blocked this destination because it matches known tracking or advertisement filter rules.</p>
                    <p>Blocked Target: <code>{}</code></p>
                </div>
            </body>
            </html>
        "#, url);

        return Ok(PageContentResponse {
            final_url: url,
            title: "Blocked by Caram Shield".into(),
            html: blocked_html,
            blocked_count: 1,
            status: 403,
        });
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36 Caram/1.0.0")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let final_url = resp.url().to_string();
    let status = resp.status().as_u16();
    let content_type = resp.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    // Stream & save binary downloads
    if !content_type.contains("text/html") && !content_type.contains("text/plain") && !content_type.contains("application/xhtml") {
        let config = db.load_config();
        let download_dir = PathBuf::from(&config.download_path);
        let filename = resp.url().path_segments()
            .and_then(|mut s| s.next_back())
            .filter(|s| !s.is_empty())
            .unwrap_or("download.bin")
            .to_string();

        let target_path = download_dir.join(&filename);
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        let size_str = format!("{:.2} MB", bytes.len() as f64 / (1024.0 * 1024.0));
        let _ = tokio::fs::write(&target_path, &bytes).await;
        let _ = db.insert_download(&filename, &final_url, target_path.to_str().unwrap_or(""), &size_str, "Completed");

        let download_html = format!(r#"
            <!DOCTYPE html>
            <html>
            <head><meta charset="utf-8"><title>Download Completed</title>
            <style>
                body {{ background: #0e1013; color: #e6e8eb; font-family: -apple-system, sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
                .card {{ background: #16181d; border: 1px solid #10b981; border-radius: 12px; padding: 32px; max-width: 480px; text-align: center; }}
                h1 {{ color: #10b981; font-size: 20px; margin-bottom: 12px; }}
                p {{ color: #8c929d; font-size: 13px; line-height: 1.6; }}
                code {{ background: #20232a; padding: 4px 8px; border-radius: 4px; color: #f97316; font-size: 12px; }}
            </style>
            </head>
            <body>
                <div class="card">
                    <h1>Download Completed</h1>
                    <p>File: <strong>{}</strong> ({})</p>
                    <p>Saved in: <code>{}</code></p>
                </div>
            </body>
            </html>
        "#, filename, size_str, target_path.display());

        return Ok(PageContentResponse {
            final_url,
            title: format!("Downloaded: {}", filename),
            html: download_html,
            blocked_count: 0,
            status,
        });
    }

    let raw_html = resp.text().await.map_err(|e| e.to_string())?;

    let title = extract_title(&raw_html).unwrap_or_else(|| {
        url::Url::parse(&final_url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_else(|| final_url.clone())
    });

    let _ = db.insert_history(&final_url, &title);

    let processed_html = process_html(&raw_html, &final_url, &verdict.cosmetic_css);

    Ok(PageContentResponse {
        final_url,
        title,
        html: processed_html,
        blocked_count: 0,
        status,
    })
}

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start_tag = "<title>";
    let end_tag = "</title>";
    let start_idx = lower.find(start_tag)? + start_tag.len();
    let end_idx = lower[start_idx..].find(end_tag)? + start_idx;
    let t = html[start_idx..end_idx].trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

fn process_html(html: &str, base_url: &str, cosmetic_css: &str) -> String {
    let base_tag = format!("<base href=\"{}\">\n", base_url);
    let cosmetic_tag = format!(
        "<style id=\"caram-shield-cosmetic\">\n{}\n</style>\n",
        cosmetic_css
    );
    let bridge_script = r#"
    <script id="caram-bridge-core">
    (function() {
        // 1. WebBridge Polyfill
        window.chrome = {
            runtime: { id: "caram-runtime", getManifest: () => ({ name: "Caram Browser" }) },
            app: { isInstalled: false },
            csi: function() {},
            loadTimes: function() { return { requestTime: performance.now() }; }
        };

        // 2. Report document title & state to top container
        function reportState() {
            try {
                window.parent.postMessage({
                    type: 'CARAM_METADATA',
                    title: document.title || window.location.href,
                    url: window.location.href
                }, '*');
            } catch(e) {}
        }
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', reportState);
        } else {
            reportState();
        }

        new MutationObserver(() => reportState()).observe(
            document.querySelector('title') || document.head || document.documentElement,
            { subtree: true, characterData: true, childList: true }
        );

        // 3. Link Navigation Interception
        document.addEventListener('click', function(e) {
            let target = e.target;
            while (target && target.tagName !== 'A') {
                target = target.parentElement;
            }
            if (target && target.href && !target.href.startsWith('javascript:')) {
                e.preventDefault();
                window.parent.postMessage({
                    type: 'CARAM_NAVIGATE',
                    url: target.href
                }, '*');
            }
        }, true);

        // 4. Runtime In-Page Shield Interception
        const BLOCKED_PATTERNS = [
            'doubleclick.net', 'google-analytics.com', 'googlesyndication.com',
            'adnxs.com', 'facebook.com/tr', 'adroll.com', 'taboola.com',
            'outbrain.com', 'criteo.com', 'scorecardresearch.com',
            'hotjar.com', 'zedo.com', 'moatads.com', 'advertising.com',
            'quantserve.com', 'popads.net', 'amazon-adsystem.com',
            'rubiconproject.com', 'openx.net', 'smartadserver.com'
        ];

        function isTracker(url) {
            if (!url) return false;
            for (let i = 0; i < BLOCKED_PATTERNS.length; i++) {
                if (url.indexOf(BLOCKED_PATTERNS[i]) !== -1) return true;
            }
            return false;
        }

        const obs = new MutationObserver((mutations) => {
            let count = 0;
            for (const m of mutations) {
                for (const node of m.addedNodes) {
                    if (node.nodeType === 1) {
                        const src = node.src || node.getAttribute('src');
                        if (src && isTracker(src)) {
                            node.src = 'about:blank';
                            node.remove();
                            count++;
                        }
                    }
                }
            }
            if (count > 0) {
                window.parent.postMessage({ type: 'CARAM_SHIELD_BLOCK', count: count }, '*');
            }
        });
        if (document.documentElement) {
            obs.observe(document.documentElement, { childList: true, subtree: true });
        }

        const origFetch = window.fetch;
        window.fetch = function(input, init) {
            const url = typeof input === 'string' ? input : (input && input.url ? input.url : '');
            if (isTracker(url)) {
                window.parent.postMessage({ type: 'CARAM_SHIELD_BLOCK', count: 1 }, '*');
                return Promise.resolve(new Response('', { status: 204, statusText: 'Blocked' }));
            }
            return origFetch.apply(this, arguments);
        };
    })();
    </script>
    "#;

    let injection = format!("{}{}{}", base_tag, cosmetic_tag, bridge_script);
    let lower = html.to_lowercase();
    if let Some(pos) = lower.find("<head>") {
        let insert_at = pos + 6;
        format!("{}{}{}", &html[..insert_at], injection, &html[insert_at..])
    } else if let Some(pos) = lower.find("<html>") {
        let insert_at = pos + 6;
        format!("{}{}{}", &html[..insert_at], injection, &html[insert_at..])
    } else {
        format!("{}{}", injection, html)
    }
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
pub fn remove_download(db: State<DbManager>, id: i64) -> Result<(), String> {
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
pub fn get_shield_stats(db: State<DbManager>, shield: State<ShieldEngine>) -> ShieldStats {
    let total = db.get_total_blocked() + shield.get_blocked_count();
    ShieldStats {
        total_blocked: total,
        trackers_blocked: total,
        bandwidth_saved_mb: (total as f64 * 0.08).round(),
        time_saved_secs: (total as f64 * 0.02).round(),
    }
}

#[tauri::command]
pub fn increment_blocked_stat(db: State<DbManager>, shield: State<ShieldEngine>, count: u64) {
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
