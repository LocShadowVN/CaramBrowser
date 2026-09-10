use adblock::lists::{FilterFormat, ParseOptions};
use adblock::request::Request;
use adblock::Engine;
use shared::{ShieldLevel, ShieldVerdict};
use std::sync::RwLock;

pub struct ShieldEngine {
    engine: RwLock<Engine>,
    level: RwLock<ShieldLevel>,
}

impl ShieldEngine {
    pub fn new() -> Self {
        let rules = vec![
            "||doubleclick.net^$third-party",
            "||google-analytics.com^",
            "||googlesyndication.com^",
            "||adnxs.com^",
            "||facebook.com/tr/*",
            "/ads.js",
            "||adroll.com^",
            "||outbrain.com^",
            "||taboola.com^",
            "||hotjar.com^",
            "##.ad-banner",
            "##.adsbygoogle",
            "##[id^='google_ads_']",
            "##.cookie-banner",
            "##.consent-modal",
        ];

        let engine = Engine::from_rules(
            rules.into_iter(),
            ParseOptions {
                format: FilterFormat::Standard,
                ..Default::default()
            },
        );

        Self {
            engine: RwLock::new(engine),
            level: RwLock::new(ShieldLevel::Standard),
        }
    }

    pub fn set_level(&self, level: ShieldLevel) {
        if let Ok(mut l) = self.level.write() {
            *l = level;
        }
    }

    pub fn get_level(&self) -> ShieldLevel {
        self.level.read().unwrap().clone()
    }

    pub fn inspect_url(&self, target_url: &str, host_url: &str) -> ShieldVerdict {
        let level = self.get_level();
        let base_css = r#"
            .ad-banner, .adsbygoogle, [id^='google_ads_'], 
            .cookie-banner, .consent-modal, div[data-ad-unit] {
                display: none !important;
                visibility: hidden !important;
                height: 0 !important;
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

        if level == ShieldLevel::Aggressive {
            let lower = target_url.to_lowercase();
            if lower.contains("telemetry") || lower.contains("fingerprint") || lower.contains("track") {
                return ShieldVerdict {
                    blocked: true,
                    rule: Some("Aggressive Anti-Fingerprinting Heuristic Match".into()),
                    level,
                    cosmetic_css: base_css.into(),
                };
            }
        }

        let req = match Request::new(target_url, host_url, "script") {
            Ok(r) => r,
            Err(_) => {
                return ShieldVerdict {
                    blocked: false,
                    rule: None,
                    level,
                    cosmetic_css: base_css.into(),
                }
            }
        };

        let engine = self.engine.read().unwrap();
        let check = engine.check(&req);

        ShieldVerdict {
            blocked: check.matched,
            rule: check.filter.map(|f| f.to_string()),
            level,
            cosmetic_css: base_css.into(),
        }
    }
}
