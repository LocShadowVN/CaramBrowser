use adblock::lists::{FilterFormat, ParseOptions};
use adblock::request::Request;
use adblock::Engine;
use shared::{ShieldLevel, ShieldVerdict};
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

impl ShieldEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<ShieldJob>();

        thread::spawn(move || {
            let brave_filters = vec![
                // --- Quảng cáo & Mạng đấu giá ---
                "||doubleclick.net^$third-party",
                "||google-analytics.com^",
                "||googlesyndication.com^",
                "||googleadservices.com^",
                "||adservice.google.com^",
                "||pagead2.googlesyndication.com^",
                "||adnxs.com^",
                "||adroll.com^",
                "||taboola.com^",
                "||outbrain.com^",
                "||criteo.com^",
                "||criteo.net^",
                "||moatads.com^",
                "||advertising.com^",
                "||quantserve.com^",
                "||popads.net^",
                "||amazon-adsystem.com^",
                "||rubiconproject.com^",
                "||pubmatic.com^",
                "||openx.net^",
                "||smartadserver.com^",
                "||bidswitch.net^",
                "||yieldmo.com^",
                "||revcontent.com^",
                "||media.net^",
                "||mgid.com^",
                "/ads/*",
                "/adbanner/*",
                "/ad-service/*",
                // --- Tracker & Telemetry ---
                "||facebook.com/tr/*",
                "||connect.facebook.net/*/fbevents.js",
                "||scorecardresearch.com^",
                "||hotjar.com^",
                "||mouseflow.com^",
                "||segment.io^",
                "||segment.com^",
                "||yandex.ru/metrika/*",
                "||mc.yandex.ru/*",
                "/telemetry/*",
                "/beacon/*",
                "*-analytics.*",
                "*-tracker.*",
                // --- Cookie Consent & GDPR Overlays ---
                "||onetrust.com^",
                "||cookielaw.org^",
                "||cookiebot.com^",
                "||trustarc.com^",
                "||usercentrics.eu^",
                "||didomi.io^",
                "||iubenda.com^",
                "||quantcast.mgr.consensu.org^",
                "||complianz.io^",
                "||fundingchoicesmessages.google.com^",
                "||consentmanager.net^",
                "||cookieinformation.com^",
                "||axeptio.eu^",
                "||sirdata.io^",
            ];

            let engine = Engine::from_rules(
                brave_filters.iter().copied(),
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
            /* Triệt tiêu phần tử quảng cáo */
            .ad-banner, .adsbygoogle, [id^='google_ads_'], [id^='div-gpt-ad'],
            .ad-container, .ad-wrapper, .ad-slot, .ad_box, .advertisement,
            .sponsored-post, .taboola-ad, .outbrain-ad, [class*='sponsored'],
            [data-ad-client], [data-google-query-id], iframe[src*='doubleclick'],
            iframe[src*='adnxs'],
            /* Triệt tiêu banner Cookie, thông báo GDPR và hộp thoại xác nhận */
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
            /* Mở khóa cuộn trang nếu website ép body overflow: hidden */
            html, body {
                overflow: auto !important;
                position: static !important;
            }
        "#
    }

    pub fn get_injected_script(&self) -> String {
        let css = self.get_cosmetic_css();
        format!(r#"
            (function() {{
                // 1. WebBridge Polyfills
                window.chrome = {{
                    runtime: {{ id: "caram-runtime", getManifest: () => ({{ name: "Caram Browser" }}) }},
                    app: {{ isInstalled: false }},
                    csi: function() {{}},
                    loadTimes: function() {{ return {{ requestTime: performance.now() }}; }}
                }};
                window.canRunAds = true;
                window.isAdBlockActive = false;

                // 2. Mock TCF/CMP APIs để qua mặt các trang web EU
                const stubCmp = function(cmd, ver, cb) {{
                    if (typeof cb === 'function') {{
                        cb({{ eventStatus: 'tcloaded', gdprApplies: false, tcString: '' }}, true);
                    }}
                }};
                window.__tcfapi = stubCmp;
                window.__cmp = stubCmp;

                // 3. Inject CSS tự động
                const injectCss = () => {{
                    if (document.getElementById('caram-shield-cosmetics')) return;
                    const style = document.createElement('style');
                    style.id = 'caram-shield-cosmetics';
                    style.textContent = `{}`;
                    (document.head || document.documentElement).appendChild(style);
                }};
                if (document.readyState === 'loading') {{
                    document.addEventListener('DOMContentLoaded', injectCss);
                }} else {{
                    injectCss();
                }}
            }})();
        "#, css)
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
            }).is_ok()
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
