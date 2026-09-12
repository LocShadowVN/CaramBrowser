use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum ShieldLevel {
    Off,
    #[default]
    Standard,
    Aggressive,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ShieldVerdict {
    pub blocked: bool,
    pub rule: Option<String>,
    pub level: ShieldLevel,
    pub cosmetic_css: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PageContentResponse {
    pub final_url: String,
    pub title: String,
    pub html: String,
    pub blocked_count: u32,
    pub status: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HistoryRecord {
    pub id: Option<i64>,
    pub url: String,
    pub title: String,
    pub timestamp: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BookmarkRecord {
    pub id: Option<i64>,
    pub url: String,
    pub title: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DownloadRecord {
    pub id: Option<i64>,
    pub filename: String,
    pub url: String,
    pub file_path: String,
    pub file_size: String,
    pub status: String,
    pub created_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExtensionItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DecryptedVaultRecord {
    pub id: i64,
    pub website: String,
    pub username: String,
    pub secret: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DnsTestResult {
    pub latency_ms: u128,
    pub resolved_ip: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ShieldStats {
    pub total_blocked: u64,
    pub trackers_blocked: u64,
    pub bandwidth_saved_mb: f64,
    pub time_saved_secs: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppConfig {
    pub search_engine: String,
    pub shield_level: String,
    pub doh_provider: String,
    pub custom_doh_url: String,
    pub download_path: String,
    pub dev_mode_extensions: bool,
    pub dark_theme: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            search_engine: "https://search.brave.com/search?q=".into(),
            shield_level: "Standard".into(),
            doh_provider: "Cloudflare".into(),
            custom_doh_url: "https://cloudflare-dns.com/dns-query".into(),
            download_path: "/tmp".into(),
            dev_mode_extensions: true,
            dark_theme: true,
        }
    }
}
