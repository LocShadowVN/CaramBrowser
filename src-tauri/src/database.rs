#![allow(dead_code)]
use rusqlite::{params, Connection};
use shared::{AppConfig, BookmarkRecord, DownloadRecord, ExtensionItem, HistoryRecord};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct VaultCipherRow {
    pub id: i64,
    pub website: String,
    pub username: String,
    pub ciphertext: String,
    pub nonce: String,
    pub salt: String,
    pub created_at: String,
}

pub struct DbManager {
    pub conn: Mutex<Connection>,
}

impl DbManager {
    pub fn init() -> Self {
        let mut dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("caram-browser");
        fs::create_dir_all(&dir).expect("Cannot create app storage directory");
        dir.push("caram_system.sqlite");

        let conn = Connection::open(dir).expect("SQLite initialization error");

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS bookmarks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS downloads (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                url TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_size TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS extensions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                description TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                path TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS vault (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                website TEXT NOT NULL,
                username TEXT NOT NULL,
                ciphertext TEXT NOT NULL,
                nonce TEXT NOT NULL,
                salt TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS vault_auth (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                master_hash TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT OR IGNORE INTO settings (key, value) VALUES ('search_engine', 'https://duckduckgo.com/?q=');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('shield_level', 'Standard');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('doh_provider', 'Cloudflare');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('custom_doh_url', 'https://cloudflare-dns.com/dns-query');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('download_path', '/tmp');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('dev_mode_extensions', 'true');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('dark_theme', 'true');
            ",
        ).expect("Schema migration failure");

        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn insert_history(&self, url: &str, title: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO history (url, title) VALUES (?1, ?2)",
            params![url, title],
        )?;
        Ok(())
    }

    pub fn fetch_history(&self) -> rusqlite::Result<Vec<HistoryRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, url, title, timestamp FROM history ORDER BY id DESC LIMIT 100")?;
        let rows = stmt.query_map([], |r| {
            Ok(HistoryRecord {
                id: Some(r.get(0)?),
                url: r.get(1)?,
                title: r.get(2)?,
                timestamp: Some(r.get(3)?),
            })
        })?;
        let mut list = Vec::new();
        for item in rows.flatten() {
            list.push(item);
        }
        Ok(list)
    }

    pub fn wipe_history(&self) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn insert_bookmark(&self, url: &str, title: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO bookmarks (url, title) VALUES (?1, ?2)",
            params![url, title],
        )?;
        Ok(())
    }

    pub fn fetch_bookmarks(&self) -> rusqlite::Result<Vec<BookmarkRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, url, title FROM bookmarks ORDER BY id DESC")?;
        let rows = stmt.query_map([], |r| {
            Ok(BookmarkRecord {
                id: Some(r.get(0)?),
                url: r.get(1)?,
                title: r.get(2)?,
            })
        })?;
        let mut list = Vec::new();
        for item in rows.flatten() {
            list.push(item);
        }
        Ok(list)
    }

    pub fn delete_bookmark(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn fetch_downloads(&self) -> rusqlite::Result<Vec<DownloadRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, filename, url, file_path, file_size, status, created_at FROM downloads ORDER BY id DESC")?;
        let rows = stmt.query_map([], |r| {
            Ok(DownloadRecord {
                id: Some(r.get(0)?),
                filename: r.get(1)?,
                url: r.get(2)?,
                file_path: r.get(3)?,
                file_size: r.get(4)?,
                status: r.get(5)?,
                created_at: Some(r.get(6)?),
            })
        })?;
        let mut list = Vec::new();
        for item in rows.flatten() {
            list.push(item);
        }
        Ok(list)
    }

    pub fn insert_download(
        &self,
        filename: &str,
        url: &str,
        file_path: &str,
        file_size: &str,
        status: &str,
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO downloads (filename, url, file_path, file_size, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![filename, url, file_path, file_size, status],
        )?;
        Ok(())
    }

    pub fn wipe_downloads(&self) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM downloads", [])?;
        Ok(())
    }

    pub fn fetch_extensions(&self) -> rusqlite::Result<Vec<ExtensionItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, version, description, enabled, path FROM extensions")?;
        let rows = stmt.query_map([], |r| {
            Ok(ExtensionItem {
                id: r.get(0)?,
                name: r.get(1)?,
                version: r.get(2)?,
                description: r.get(3)?,
                enabled: r.get::<_, i64>(4)? == 1,
                path: r.get(5)?,
            })
        })?;
        let mut list = Vec::new();
        for item in rows.flatten() {
            list.push(item);
        }
        Ok(list)
    }

    pub fn save_extension(&self, ext: &ExtensionItem) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO extensions (id, name, version, description, enabled, path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![ext.id, ext.name, ext.version, ext.description, if ext.enabled { 1 } else { 0 }, ext.path],
        )?;
        Ok(())
    }

    pub fn set_extension_state(&self, id: &str, enabled: bool) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE extensions SET enabled = ?1 WHERE id = ?2",
            params![if enabled { 1 } else { 0 }, id],
        )?;
        Ok(())
    }

    pub fn remove_extension(&self, id: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM extensions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_master_hash(&self) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT master_hash FROM vault_auth WHERE id = 1").ok()?;
        stmt.query_row([], |r| r.get(0)).ok()
    }

    pub fn set_master_hash(&self, hash: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO vault_auth (id, master_hash) VALUES (1, ?1)",
            params![hash],
        )?;
        Ok(())
    }

    pub fn insert_vault_row(
        &self,
        site: &str,
        user: &str,
        cipher: &str,
        nonce: &str,
        salt: &str,
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO vault (website, username, ciphertext, nonce, salt) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![site, user, cipher, nonce, salt],
        )?;
        Ok(())
    }

    pub fn list_vault_rows(&self) -> rusqlite::Result<Vec<VaultCipherRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, website, username, ciphertext, nonce, salt, created_at FROM vault ORDER BY id DESC")?;
        let rows = stmt.query_map([], |r| {
            Ok(VaultCipherRow {
                id: r.get(0)?,
                website: r.get(1)?,
                username: r.get(2)?,
                ciphertext: r.get(3)?,
                nonce: r.get(4)?,
                salt: r.get(5)?,
                created_at: r.get(6)?,
            })
        })?;
        let mut list = Vec::new();
        for item in rows.flatten() {
            list.push(item);
        }
        Ok(list)
    }

    pub fn delete_vault_row(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM vault WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn load_config(&self) -> AppConfig {
        let conn = self.conn.lock().unwrap();
        let mut cfg = AppConfig {
            search_engine: "https://duckduckgo.com/?q=".into(),
            shield_level: "Standard".into(),
            doh_provider: "Cloudflare".into(),
            custom_doh_url: "https://cloudflare-dns.com/dns-query".into(),
            download_path: "/tmp".into(),
            dev_mode_extensions: true,
            dark_theme: true,
        };

        if let Ok(mut stmt) = conn.prepare("SELECT key, value FROM settings") {
            if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))) {
                for (k, v) in rows.flatten() {
                    match k.as_str() {
                        "search_engine" => cfg.search_engine = v,
                        "shield_level" => cfg.shield_level = v,
                        "doh_provider" => cfg.doh_provider = v,
                        "custom_doh_url" => cfg.custom_doh_url = v,
                        "download_path" => cfg.download_path = v,
                        "dev_mode_extensions" => cfg.dev_mode_extensions = v == "true",
                        "dark_theme" => cfg.dark_theme = v == "true",
                        _ => {}
                    }
                }
            }
        }
        cfg
    }

    pub fn save_config_item(&self, key: &str, val: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, val],
        )?;
        Ok(())
    }
}
