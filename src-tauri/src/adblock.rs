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
                // Core tracker & ad networks
                "||doubleclick.net^$third-party",
                "||google-analytics.com^",
                "||googlesyndication.com^",
                "||googleadservices.com^",
                "||adservice.google.com^",
                "||pagead2.googlesyndication.com^",
                "||adnxs.com^",
                "||facebook.com/tr/*",
                "||connect.facebook.net/*/fbevents.js",
                "||adroll.com^",
                "||taboola.com^",
                "||outbrain.com^",
                "||criteo.com^",
                "||criteo.net^",
                "||scorecardresearch.com^",
                "||hotjar.com^",
                "||zedo.com^",
                "||moatads.com^",
                "||advertising.com^",
                "||quantserve.com^",
                "||popads.net^",
                "||popcash.net^",
                "||propellerads.com^",
                "||amazon-adsystem.com^",
                "||rubiconproject.com^",
                "||pubmatic.com^",
                "||casalemedia.com^",
                "||openx.net^",
                "||smartadserver.com^",
                "||bidswitch.net^",
                "||yieldmo.com^",
                "||revcontent.com^",
                "||infolinks.com^",
                "||media.net^",
                "||sovrn.com^",
                "||mgid.com^",
                "||exponential.com^",
                "||adcolony.com^",
                "||chartbeat.com^",
                "||crazyegg.com^",
                "||yandex.ru/metrika/*",
                "||mc.yandex.ru/*",
                "/ads/*",
                "/adbanner/*",
                "/ad-service/*",
                "/telemetry/*",
                "/beacon/*",
                "*-analytics.*",
                "*-tracker.*",
                // Cosmetic selectors
                "##.ad-banner",
                "##.adsbygoogle",
                "##[id^='google_ads_']",
                "##[id^='div-gpt-ad']",
                "##.ad-container",
                "##.ad-wrapper",
                "##.ad-slot",
                "##.ad_box",
                "##.advertisement",
                "##.sponsored-post",
                "##.taboola-ad",
                "##.outbrain-ad",
                "##[class*='sponsored']",
                "##[data-ad-client]",
                "##[data-google-query-id]",
                "##iframe[src*='doubleclick']",
                "##iframe[src*='adnxs']",
                "##.cookie-banner",
                "##.consent-banner",
                "##[id*='cookie-notice']",
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

    pub async fn inspect_url(&self, target_url: &str, host_url: &str) -> ShieldVerdict {
        let level = self.get_level();
        let cosmetic_css = r#"
            .ad-banner, .adsbygoogle, [id^='google_ads_'], [id^='div-gpt-ad'],
            .ad-container, .ad-wrapper, .ad-slot, .ad_box, .advertisement,
            .sponsored-post, .taboola-ad, .outbrain-ad, [class*='sponsored'],
            [data-ad-client], [data-google-query-id], iframe[src*='doubleclick'],
            iframe[src*='adnxs'], .cookie-banner, .consent-banner, [id*='cookie-notice'] {
                display: none !important;
                visibility: hidden !important;
                height: 0 !important;
                max-height: 0 !important;
                opacity: 0 !important;
                pointer-events: none !important;
            }
        "#;

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
            rule: if is_blocked { Some("Brave Engine Filter Match".into()) } else { None },
            level,
            cosmetic_css: cosmetic_css.into(),
        }
    }
}
