use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)] // <-- Thêm Default vào đây
pub enum ShieldLevel {
    Off,
    #[default] // <-- Thêm dòng này trước Standard
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
pub struct AppConfig {
    pub search_engine: String,
    pub shield_level: String,
    pub doh_provider: String,
    pub custom_doh_url: String,
    pub download_path: String,
    pub dev_mode_extensions: bool,
    pub dark_theme: bool,
}
