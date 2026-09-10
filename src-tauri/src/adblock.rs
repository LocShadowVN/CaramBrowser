use adblock::lists::{FilterFormat, ParseOptions};
use adblock::request::Request;
use adblock::Engine;
use shared::{ShieldLevel, ShieldVerdict};
use std::sync::mpsc::{channel, Sender};
use std::sync::RwLock;
use std::thread;

enum ShieldJob {
    Check {
        url: String,
        host: String,
        reply_to: tokio::sync::oneshot::Sender<bool>,
    },
}

pub struct ShieldEngine {
    tx: Sender<ShieldJob>,
    level: RwLock<ShieldLevel>,
}

impl ShieldEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<ShieldJob>();

        // Khởi chạy một Worker Thread độc lập dành riêng cho Brave Engine
        thread::spawn(move || {
            let default_rules = vec![
                "||doubleclick.net^$third-party",
                "||google-analytics.com^",
                "||googlesyndication.com^",
                "||adnxs.com^",
                "||facebook.com/tr/*",
                "/ads/*",
                "/adbanner/*",
                "||adroll.com^",
                "||taboola.com^",
                "||outbrain.com^",
                "##.ad-banner",
                "##.adsbygoogle",
                "##[id^='google_ads_']",
            ];

            let engine = Engine::from_rules(
                default_rules.iter().map(|s| *s),
                ParseOptions {
                    format: FilterFormat::Standard,
                    ..Default::default()
                },
            );

            // Lắng nghe các yêu cầu kiểm tra URL
            while let Ok(job) = rx.recv() {
                match job {
                    ShieldJob::Check { url, host, reply_to } => {
                        let blocked = match Request::new(&url, &host, "script") {
                            Ok(req) => engine.check_network_urls(&req).matched,
                            Err(_) => false,
                        };
                        let _ = reply_to.send(blocked);
                    }
                }
            }
        });

        Self {
            tx,
            level: RwLock::new(ShieldLevel::Standard),
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

    pub async fn inspect_url(&self, target_url: &str, host_url: &str) -> ShieldVerdict {
        let level = self.get_level();
        let cosmetic_css = r#"
            .ad-banner, .adsbygoogle, [id^='google_ads_'], .cookie-banner, div[data-ad] {
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

        // Kiểm tra qua Brave Adblock Engine trên luồng Worker
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let _ = self.tx.send(ShieldJob::Check {
            url: target_url.to_string(),
            host: host_url.to_string(),
            reply_to: reply_tx,
        });

        let is_blocked = reply_rx.await.unwrap_or(false);

        ShieldVerdict {
            blocked: is_blocked,
            rule: if is_blocked { Some("Brave Engine Match".into()) } else { None },
            level,
            cosmetic_css: cosmetic_css.into(),
        }
    }
}
