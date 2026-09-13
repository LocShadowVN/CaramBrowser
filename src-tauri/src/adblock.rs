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

impl ShieldEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<ShieldJob>();

        thread::spawn(move || {
            let mut rules: Vec<String> = vec![
                // --- Core Ad Networks & Exchanges ---
                "||doubleclick.net^$third-party".into(),
                "||googleadservices.com^".into(),
                "||pagead2.googlesyndication.com^".into(),
                "||adservice.google.com^".into(),
                "||adnxs.com^".into(),
                "||adroll.com^".into(),
                "||taboola.com^".into(),
                "||outbrain.com^".into(),
                "||criteo.com^".into(),
                "||criteo.net^".into(),
                "||moatads.com^".into(),
                "||advertising.com^".into(),
                "||quantserve.com^".into(),
                "||popads.net^".into(),
                "||popcash.net^".into(),
                "||amazon-adsystem.com^".into(),
                "||rubiconproject.com^".into(),
                "||pubmatic.com^".into(),
                "||openx.net^".into(),
                "||smartadserver.com^".into(),
                "||bidswitch.net^".into(),
                "||yieldmo.com^".into(),
                "||revcontent.com^".into(),
                "||media.net^".into(),
                "||mgid.com^".into(),
                "||infolinks.com^".into(),
                "||chitika.net^".into(),
                "||zedo.com^".into(),
                "||propellerads.com^".into(),
                "||exoclick.com^".into(),
                "||juicyads.com^".into(),
                "||trafficjunky.com^".into(),
                "||adsterra.com^".into(),
                "||clickadu.com^".into(),
                "/ads/*".into(),
                "/adbanner/*".into(),
                "/ad-service/*".into(),

                // --- Trackers, Telemetry & Analytics ---
                "||google-analytics.com^".into(),
                "||analytics.google.com^".into(),
                "||googletagmanager.com/gtm.js*".into(),
                "||googletagservices.com^".into(),
                "||facebook.com/tr/*".into(),
                "||connect.facebook.net/*/fbevents.js".into(),
                "||scorecardresearch.com^".into(),
                "||hotjar.com^".into(),
                "||mouseflow.com^".into(),
                "||fullstory.com^".into(),
                "||segment.io^".into(),
                "||segment.com^".into(),
                "||mixpanel.com^".into(),
                "||newrelic.com^".into(),
                "||yandex.ru/metrika/*".into(),
                "||mc.yandex.ru/*".into(),
                "||clarity.ms^".into(),
                "||bat.bing.com^".into(),
                "||tiktok.com/api/v1/pixel/*".into(),
                "||analytics.tiktok.com^".into(),
                "||byteoversea.com^".into(),
                "||ads.twitter.com^".into(),
                "||ads-twitter.com^".into(),
                "||static.ads-twitter.com^".into(),
                "/telemetry/*".into(),
                "/beacon/*".into(),
                "*-analytics.*".into(),
                "*-tracker.*".into(),
                "*-telemetry.*".into(),

                // --- Cookie Consent, GDPR & Annoyance Networks ---
                "||onetrust.com^".into(),
                "||cookielaw.org^".into(),
                "||cookiebot.com^".into(),
                "||trustarc.com^".into(),
                "||usercentrics.eu^".into(),
                "||didomi.io^".into(),
                "||iubenda.com^".into(),
                "||quantcast.mgr.consensu.org^".into(),
                "||complianz.io^".into(),
                "||fundingchoicesmessages.google.com^".into(),
                "||consentmanager.net^".into(),
                "||cookieinformation.com^".into(),
                "||axeptio.eu^".into(),
                "||sirdata.io^".into(),
                "||osano.com^".into(),
                "||ketch.com^".into(),
                "||cookie-script.com^".into(),
                "||cookieyes.com^".into(),
                "||uniconsent.com^".into(),
                "||termsfeed.com^".into(),
            ];

            // Tự động load thêm rules ngoại vi nếu user bỏ file vào ~/.local/share/caram-browser/custom_rules.txt
            let mut custom_path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
            custom_path.push("caram-browser");
            custom_path.push("custom_rules.txt");
            if custom_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&custom_path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && !trimmed.starts_with('!') && !trimmed.starts_with('#') {
                            rules.push(trimmed.to_string());
                        }
                    }
                }
            }

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
            /* Triệt tiêu phần tử quảng cáo và video ads */
            .ad-banner, .adsbygoogle, [id^='google_ads_'], [id^='div-gpt-ad'],
            .ad-container, .ad-wrapper, .ad-slot, .ad_box, .advertisement,
            .sponsored-post, .taboola-ad, .outbrain-ad, [class*='sponsored'],
            [data-ad-client], [data-google-query-id], iframe[src*='doubleclick'],
            iframe[src*='adnxs'], .video-ads, .ytp-ad-module, .ytp-ad-overlay-container,
            
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

            /* Mở khóa cuộn trang nếu website ép body overflow: hidden để bắt bấm đồng ý */
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
                // ============================================================
                // 1. BRAVE FARBLING: CHỐNG LẤY DẤU VÂN TAY (ANTI-FINGERPRINTING)
                // ============================================================
                try {{
                    // Canvas Farbling: Thêm nhiễu ngẫu nhiên vi mô vào dữ liệu Canvas
                    const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
                    HTMLCanvasElement.prototype.toDataURL = function() {{
                        const ctx = this.getContext('2d');
                        if (ctx && this.width > 16 && this.height > 16) {{
                            try {{
                                const imgData = ctx.getImageData(0, 0, 2, 2);
                                imgData.data[0] = (imgData.data[0] ^ 1);
                                ctx.putImageData(imgData, 0, 0);
                            }} catch(e) {{}}
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

                    // AudioContext Farbling: Chống fingerprinting qua âm thanh
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

                    // Chống phát hiện tự động hóa
                    Object.defineProperty(navigator, 'webdriver', {{ get: () => false }});
                    if (navigator.getBattery) {{
                        navigator.getBattery = () => Promise.reject();
                    }}
                }} catch(e) {{}}

                // ============================================================
                // 2. SCRIPTLET DEFUSERS & MOCK TRACKERS (CHUẨN uBLOCK ORIGIN)
                // ============================================================
                window.chrome = {{
                    runtime: {{ id: "caram-runtime", getManifest: () => ({{ name: "Caram Browser" }}) }},
                    app: {{ isInstalled: false }},
                    csi: function() {{}},
                    loadTimes: function() {{ return {{ requestTime: performance.now() }}; }}
                }};
                window.canRunAds = true;
                window.isAdBlockActive = false;
                window.ga = function() {{}};
                window.ga.q = [];
                window.gtag = function() {{}};
                window.fbq = function() {{}};

                // ============================================================
                // 3. AUTO-DEFUSE BANNER COOKIE / GDPR (TCF & CMP STUBS)
                // ============================================================
                const stubCmp = function(cmd, ver, cb) {{
                    if (typeof cb === 'function') {{
                        cb({{ eventStatus: 'tcloaded', gdprApplies: false, tcString: '' }}, true);
                    }}
                }};
                window.__tcfapi = stubCmp;
                window.__cmp = stubCmp;
                window.OneTrust = {{ IsAlertBoxClosed: () => true, Close: () => {{}} }};
                window.Cookiebot = {{ consented: true, declined: false, hide: () => {{}} }};

                // Triệt tiêu Beacon API
                if (navigator.sendBeacon) {{
                    navigator.sendBeacon = () => true;
                }}

                // ============================================================
                // 4. SUBRESOURCE NETWORK HOOK (CHẶN REQUEST NGẦM TRONG TRANG)
                // ============================================================
                const BLOCKED_DOMAINS = [
                    'doubleclick.net', 'google-analytics.com', 'googlesyndication.com',
                    'googleadservices.com', 'adnxs.com', 'facebook.com/tr',
                    'adroll.com', 'taboola.com', 'outbrain.com', 'criteo.com',
                    'scorecardresearch.com', 'hotjar.com', 'moatads.com',
                    'advertising.com', 'popads.net', 'amazon-adsystem.com',
                    'rubiconproject.com', 'openx.net', 'smartadserver.com',
                    'onetrust.com', 'cookielaw.org', 'cookiebot.com', 'clarity.ms'
                ];

                function isTrackingUrl(url) {{
                    if (!url) return false;
                    for (let i = 0; i < BLOCKED_DOMAINS.length; i++) {{
                        if (url.indexOf(BLOCKED_DOMAINS[i]) !== -1) return true;
                    }}
                    return false;
                }}

                const origFetch = window.fetch;
                window.fetch = function(input, init) {{
                    const url = typeof input === 'string' ? input : (input && input.url ? input.url : '');
                    if (isTrackingUrl(url)) {{
                        return Promise.resolve(new Response('', {{ status: 204, statusText: 'Blocked by Caram Shield' }}));
                    }}
                    return origFetch.apply(this, arguments);
                }};

                const origOpen = XMLHttpRequest.prototype.open;
                XMLHttpRequest.prototype.open = function(method, url) {{
                    if (isTrackingUrl(url)) {{
                        this.abort();
                        return;
                    }}
                    return origOpen.apply(this, arguments);
                }};

                // ============================================================
                // 5. INJECT COSMETIC STYLESHEET
                // ============================================================
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
