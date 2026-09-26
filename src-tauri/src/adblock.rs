use adblock::lists::{FilterFormat, ParseOptions};
use adblock::request::Request;
use adblock::Engine;
use shared::{ShieldLevel, ShieldVerdict};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, RwLock};
use std::thread;

enum ShieldJob {
    Check {
        url: String,
        host: String,
        reply_to: tokio::sync::oneshot::Sender<bool>,
    },
}

pub struct ShieldEngine {
    tx: Mutex<Sender<ShieldJob>>,
    level: RwLock<ShieldLevel>,
    blocked_count: AtomicU64,
}

fn resolve_bundled_rules_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(data_dir) = dirs::data_local_dir() {
        candidates.push(data_dir.join("vibird-browser").join("custom_rules.txt"));
        candidates.push(data_dir.join("caram-browser").join("custom_rules.txt"));
    }

    if let Ok(appdir) = std::env::var("APPDIR") {
        let root = PathBuf::from(&appdir);
        candidates.push(root.join("usr/lib/vibird-browser/resources/rules.txt"));
        candidates.push(root.join("usr/lib/caram-browser/resources/rules.txt"));
        candidates.push(root.join("usr/lib/caram_browser/resources/rules.txt"));
        candidates.push(root.join("usr/bin/resources/rules.txt"));
        candidates.push(root.join("resources/rules.txt"));
    }

    candidates.push(PathBuf::from("/usr/lib/vibird-browser/resources/rules.txt"));
    candidates.push(PathBuf::from("/usr/lib/caram-browser/resources/rules.txt"));
    candidates.push(PathBuf::from("/usr/lib/caram_browser/resources/rules.txt"));
    candidates.push(PathBuf::from("/usr/share/vibird-browser/resources/rules.txt"));
    candidates.push(PathBuf::from("/usr/share/caram-browser/resources/rules.txt"));

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            candidates.push(parent.join("resources/rules.txt"));
            candidates.push(parent.join("../lib/vibird-browser/resources/rules.txt"));
            candidates.push(parent.join("../lib/caram-browser/resources/rules.txt"));
            candidates.push(parent.join("../lib/caram_browser/resources/rules.txt"));
        }
    }

    candidates.push(PathBuf::from("resources/rules.txt"));
    candidates.push(PathBuf::from("src-tauri/resources/rules.txt"));

    candidates.into_iter().find(|p| p.exists())
}

impl ShieldEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<ShieldJob>();

        thread::spawn(move || {
            let mut rules: Vec<String> = vec![
                "||doubleclick.net^$third-party".into(),
                "||googleadservices.com^".into(),
                "||pagead2.googlesyndication.com^".into(),
                "||google-analytics.com^".into(),
                "||analytics.google.com^".into(),
                "||googletagmanager.com/gtm.js*".into(),
                "||adnxs.com^".into(),
                "||adroll.com^".into(),
                "||taboola.com^".into(),
                "||outbrain.com^".into(),
                "||criteo.com^".into(),
                "||facebook.com/tr/*".into(),
                "||hotjar.com^".into(),
                "||onetrust.com^".into(),
                "||cookielaw.org^".into(),
                "||cookiebot.com^".into(),
                "/ads/*".into(),
                "/adbanner/*".into(),
                "/telemetry/*".into(),
            ];

            let resolved_path = resolve_bundled_rules_path();
            let mut external_count = 0usize;

            if let Some(ref rules_path) = resolved_path {
                match std::fs::read_to_string(rules_path) {
                    Ok(content) => {
                        for line in content.lines() {
                            let trimmed = line.trim().trim_start_matches('\u{feff}');
                            if !trimmed.is_empty()
                                && !trimmed.starts_with('!')
                                && !trimmed.starts_with('#')
                            {
                                rules.push(trimmed.to_string());
                                external_count += 1;
                            }
                        }
                        log::info!(
                            "Vibird Shield: loaded {} external rules from {:?}",
                            external_count,
                            rules_path
                        );
                    }
                    Err(e) => {
                        log::warn!(
                            "Vibird Shield: cannot read {:?}: {} (using baseline rules only)",
                            rules_path,
                            e
                        );
                    }
                }
            } else {
                log::warn!("Vibird Shield: no bundled rules.txt found — using baseline rules only");
            }

            log::info!("Vibird Shield: engine initialized with {} total rules", rules.len());

            let engine = Engine::from_rules(
                rules.iter().map(|s| s.as_str()),
                ParseOptions {
                    format: FilterFormat::Standard,
                    ..Default::default()
                },
            );

            while let Ok(job) = rx.recv() {
                match job {
                    ShieldJob::Check { url, host, reply_to } => {
                        let blocked = match Request::new(&url, &host, "script") {
                            Ok(req) => engine.check_network_request(&req).matched,
                            Err(_) => false,
                        };
                        let _ = reply_to.send(blocked);
                    }
                }
            }
        });

        Self {
            tx: Mutex::new(tx),
            level: RwLock::new(ShieldLevel::Standard),
            blocked_count: AtomicU64::new(0),
        }
    }

    pub fn set_level(&self, level: ShieldLevel) {
        if let Ok(mut l) = self.level.write() {
            *l = level;
        }
    }

    pub fn get_level(&self) -> ShieldLevel {
        self.level.read().map(|l| l.clone()).unwrap_or(ShieldLevel::Standard)
    }

    pub fn get_blocked_count(&self) -> u64 {
        self.blocked_count.load(Ordering::Relaxed)
    }

    pub fn increment_blocked(&self, delta: u64) {
        self.blocked_count.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn get_cosmetic_css(&self) -> &'static str {
        r#"
            .ad-banner, .adsbygoogle, [id^='google_ads_'], [id^='div-gpt-ad'],
            .ad-container, .ad-wrapper, .ad-slot, .ad_box, .advertisement,
            .sponsored-post, .taboola-ad, .outbrain-ad, [class*='sponsored'],
            [data-ad-client], [data-google-query-id], iframe[src*='doubleclick'],
            iframe[src*='adnxs'], .video-ads, .ytp-ad-module, .ytp-ad-overlay-container,
            #onetrust-consent-sdk, #onetrust-banner-sdk, .onetrust-pc-dark,
            #CybotCookiebotDialog, #CybotCookiebotDialogBody,
            .cc-window, .cc-banner, .cc-floating, .cc-dialog,
            #qc-cmp2-container, #qc-cmp2-ui,
            .cookie-banner, .cookie-notice, .cookie-consent, .cookie-popup,
            .cookie-policy-banner, [id*='cookie-notice'], [id*='cookiebanner'],
            [id*='cookie-law-info'], [id*='cookieConsent'], [class*='cookie-consent'],
            [class*='cookie-banner'], [class*='cookie-notice'], [class*='cookiebar'],
            [aria-label*='cookie' i], [aria-label*='consent' i],
            .fc-consent-root, .fc-dialog-overlay, .fc-dialog-container,
            #iubenda-cs-banner, .iubenda-cs-content,
            .cmp-container, #cmpbox, #cmpbox2,
            .sp_veil, .message-container, [id^='sp_message_container_'] {
                display: none !important;
                visibility: hidden !important;
                opacity: 0 !important;
                pointer-events: none !important;
                height: 0 !important;
                max-height: 0 !important;
                z-index: -99999 !important;
            }
            html, body {
                overflow: auto !important;
                position: static !important;
            }
        "#
    }

    pub fn get_injected_script(&self) -> String {
        let css = self.get_cosmetic_css();

        format!(
            r#"
            (function() {{
                'use strict';

                const TAB_ID = window.__VIBIRD_TAB_ID || '';

                function reportBlock() {{
                    try {{
                        if (window.__TAURI__ && window.__TAURI__.event && window.__TAURI__.event.emit) {{
                            window.__TAURI__.event.emit('shield-blocked', {{ tab_id: TAB_ID, count: 1 }});
                        }}
                    }} catch (e) {{}}
                }}

                const BLOCKED_PATTERNS = [
                    'doubleclick.net', 'google-analytics.com', 'googlesyndication.com',
                    'googleadservices.com', 'adnxs.com', 'facebook.com/tr',
                    'adroll.com', 'taboola.com', 'outbrain.com', 'criteo.com',
                    'scorecardresearch.com', 'hotjar.com', 'moatads.com',
                    'advertising.com', 'popads.net', 'amazon-adsystem.com',
                    'rubiconproject.com', 'openx.net', 'smartadserver.com',
                    'onetrust.com', 'cookielaw.org', 'cookiebot.com', 'clarity.ms',
                    'tiktok.com/api/v1/pixel', 'bat.bing.com'
                ];

                function isTrackingUrl(url) {{
                    if (!url || typeof url !== 'string') return false;
                    for (let i = 0; i < BLOCKED_PATTERNS.length; i++) {{
                        if (url.indexOf(BLOCKED_PATTERNS[i]) !== -1) return true;
                    }}
                    return false;
                }}

                const origScriptSrcDesc = Object.getOwnPropertyDescriptor(HTMLScriptElement.prototype, 'src');
                if (origScriptSrcDesc) {{
                    Object.defineProperty(HTMLScriptElement.prototype, 'src', {{
                        set: function(val) {{
                            if (isTrackingUrl(val)) {{
                                reportBlock();
                                return origScriptSrcDesc.set.call(this, 'data:text/javascript,/*blocked-by-vibird-shield*/');
                            }}
                            return origScriptSrcDesc.set.call(this, val);
                        }},
                        get: function() {{
                            return origScriptSrcDesc.get.call(this);
                        }}
                    }});
                }}

                const origIframeSrcDesc = Object.getOwnPropertyDescriptor(HTMLIFrameElement.prototype, 'src');
                if (origIframeSrcDesc) {{
                    Object.defineProperty(HTMLIFrameElement.prototype, 'src', {{
                        set: function(val) {{
                            if (isTrackingUrl(val)) {{
                                reportBlock();
                                return origIframeSrcDesc.set.call(this, 'about:blank');
                            }}
                            return origIframeSrcDesc.set.call(this, val);
                        }},
                        get: function() {{
                            return origIframeSrcDesc.get.call(this);
                        }}
                    }});
                }}

                const OrigWS = window.WebSocket;
                window.WebSocket = function(url, protocols) {{
                    if (isTrackingUrl(url)) {{
                        reportBlock();
                        throw new Error('Blocked by Vibird Shield Deep Network Guard');
                    }}
                    return new OrigWS(url, protocols);
                }};

                try {{
                    const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
                    HTMLCanvasElement.prototype.toDataURL = function() {{
                        const ctx = this.getContext('2d');
                        if (ctx && this.width > 16 && this.height > 16) {{
                            try {{
                                const imgData = ctx.getImageData(0, 0, 2, 2);
                                imgData.data[0] = (imgData.data[0] ^ 1);
                                ctx.putImageData(imgData, 0, 0);
                            }} catch (e) {{}}
                        }}
                        return origToDataURL.apply(this, arguments);
                    }};

                    const origGetImageData = CanvasRenderingContext2D.prototype.getImageData;
                    CanvasRenderingContext2D.prototype.getImageData = function() {{
                        const res = origGetImageData.apply(this, arguments);
                        if (res && res.data && res.data.length > 4) {{
                            res.data[0] = (res.data[0] ^ 1);
                        }}
                        return res;
                    }};

                    if (window.AudioBuffer) {{
                        const origGetChannelData = AudioBuffer.prototype.getChannelData;
                        AudioBuffer.prototype.getChannelData = function() {{
                            const data = origGetChannelData.apply(this, arguments);
                            if (data && data.length > 0) {{
                                data[0] = data[0] + 0.00000001;
                            }}
                            return data;
                        }};
                    }}

                    Object.defineProperty(navigator, 'webdriver', {{ get: () => false }});
                    if (navigator.getBattery) {{
                        navigator.getBattery = () => Promise.reject();
                    }}
                }} catch (e) {{}}

                window.canRunAds = true;
                window.isAdBlockActive = false;
                window.ga = function() {{}};
                window.ga.q = [];
                window.gtag = function() {{}};
                window.fbq = function() {{}};

                const stubCmp = function(cmd, ver, cb) {{
                    if (typeof cb === 'function') {{
                        cb({{ eventStatus: 'tcloaded', gdprApplies: false, tcString: '' }}, true);
                    }}
                }};
                window.__tcfapi = stubCmp;
                window.__cmp = stubCmp;
                window.OneTrust = {{ IsAlertBoxClosed: () => true, Close: () => {{}} }};
                window.Cookiebot = {{ consented: true, declined: false, hide: () => {{}} }};

                if (navigator.sendBeacon) {{
                    navigator.sendBeacon = () => true;
                }}

                const origFetch = window.fetch;
                window.fetch = function(input, init) {{
                    const url = typeof input === 'string' ? input : (input && input.url ? input.url : '');
                    if (isTrackingUrl(url)) {{
                        reportBlock();
                        return Promise.resolve(new Response('', {{ status: 204, statusText: 'Blocked' }}));
                    }}
                    return origFetch.apply(this, arguments);
                }};

                const origOpen = XMLHttpRequest.prototype.open;
                XMLHttpRequest.prototype.open = function(method, url) {{
                    if (isTrackingUrl(url)) {{
                        reportBlock();
                        this.abort();
                        return;
                    }}
                    return origOpen.apply(this, arguments);
                }};

                const injectCss = () => {{
                    if (document.getElementById('vibird-shield-cosmetics')) return;
                    const style = document.createElement('style');
                    style.id = 'vibird-shield-cosmetics';
                    style.textContent = `{}`;
                    (document.head || document.documentElement).appendChild(style);
                }};
                if (document.readyState === 'loading') {{
                    document.addEventListener('DOMContentLoaded', injectCss);
                }} else {{
                    injectCss();
                }}
            }})();
            "#,
            css
        )
    }

    pub async fn inspect_url(&self, target_url: &str, host_url: &str) -> ShieldVerdict {
        let level = self.get_level();
        if level == ShieldLevel::Off {
            return ShieldVerdict {
                blocked: false,
                rule: None,
                level,
                cosmetic_css: String::new(),
            };
        }

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let sent = if let Ok(tx) = self.tx.lock() {
            tx.send(ShieldJob::Check {
                url: target_url.to_string(),
                host: host_url.to_string(),
                reply_to: reply_tx,
            })
            .is_ok()
        } else {
            false
        };

        let is_blocked = if sent {
            reply_rx.await.unwrap_or(false)
        } else {
            false
        };

        if is_blocked {
            self.blocked_count.fetch_add(1, Ordering::Relaxed);
        }

        ShieldVerdict {
            blocked: is_blocked,
            rule: if is_blocked { Some("Brave Engine Match".into()) } else { None },
            level,
            cosmetic_css: self.get_cosmetic_css().to_string(),
        }
    }
}
