use adblock::lists::{FilterFormat, ParseOptions};
use adblock::request::Request;
use adblock::Engine;
use shared::{ShieldLevel, ShieldVerdict};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

pub struct ShieldEngine {
    engine: RwLock<Engine>,
    level: RwLock<ShieldLevel>,
    blocked_count: AtomicU64,
}

impl ShieldEngine {
    pub fn new() -> Self {
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

        Self {
            engine: RwLock::new(engine),
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

        let blocked = if let Ok(guard) = self.engine.read() {
            match Request::new(target_url, host_url, "script") {
                Ok(req) => guard.check_network_request(&req).matched,
                Err(_) => false,
            }
        } else {
            false
        };

        if blocked {
            self.blocked_count.fetch_add(1, Ordering::Relaxed);
        }

        ShieldVerdict {
            blocked,
            rule: if blocked { Some("Brave Engine Filter Match".into()) } else { None },
            level,
            cosmetic_css: cosmetic_css.into(),
        }
    }
}
