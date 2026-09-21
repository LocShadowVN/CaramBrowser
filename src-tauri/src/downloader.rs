use crate::database::DbManager;
use shared::DownloadProgressPayload;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct DownloadEngine;

impl DownloadEngine {
    /// Lọc bỏ các ký tự nguy hiểm tránh Directory Traversal
    fn sanitize_filename(name: &str) -> String {
        let clean = name
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
            .trim_matches(['.', ' '])
            .to_string();

        if clean.is_empty() {
            format!("download_{}.bin", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
        } else {
            clean
        }
    }

    pub async fn start_download(
        app: AppHandle,
        url: String,
        save_dir: PathBuf,
        custom_name: Option<String>,
        connections: usize,
    ) -> Result<String, String> {
        let task_id = format!("dl_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let head_resp = client.head(&url).send().await.map_err(|e| format!("HEAD error: {}", e))?;

        let total_size = head_resp
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let supports_ranges = head_resp
            .headers()
            .get(reqwest::header::ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase().contains("bytes"))
            .unwrap_or(false);

        let raw_filename = custom_name.unwrap_or_else(|| {
            head_resp
                .url()
                .path_segments()
                .and_then(|mut s| s.next_back())
                .filter(|s| !s.is_empty())
                .unwrap_or("download.bin")
                .to_string()
        });

        let final_filename = Self::sanitize_filename(&raw_filename);
        let target_path = save_dir.join(&final_filename);

        // Đảm bảo an toàn đường dẫn nằm trọn trong thư mục download
        let canonical_dir = tokio::fs::canonicalize(&save_dir).await.map_err(|e| e.to_string())?;
        if !target_path.starts_with(&canonical_dir) && !target_path.starts_with(&save_dir) {
            return Err("Invalid download target path".into());
        }

        let active_connections = if supports_ranges && total_size > 1_048_576 { connections.clamp(2, 16) } else { 1 };
        let progress_downloaded = Arc::new(AtomicU64::new(0));
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let task_id_clone = task_id.clone();
        let final_filename_clone = final_filename.clone();
        let target_path_clone = target_path.clone();
        let url_clone = url.clone();
        let app_clone = app.clone();
        let prog_clone = progress_downloaded.clone();
        let cancel_clone = cancel_flag.clone();

        tokio::spawn(async move {
            let target_path_bg = target_path_clone.clone();

            let result = if active_connections > 1 {
                Self::download_multi_threaded(
                    client,
                    url_clone.clone(),
                    target_path_clone.clone(),
                    total_size,
                    active_connections,
                    prog_clone.clone(),
                    cancel_clone.clone(),
                ).await
            } else {
                Self::download_single_stream(
                    client,
                    url_clone.clone(),
                    target_path_clone.clone(),
                    prog_clone.clone(),
                    cancel_clone.clone(),
                ).await
            };

            match result {
                Ok(_) => {
                    let db = app_clone.state::<DbManager>();
                    let size_str = format!("{:.2} MB", total_size as f64 / (1024.0 * 1024.0));
                    let _ = db.insert_download(
                        &final_filename_clone,
                        &url_clone,
                        target_path_bg.to_str().unwrap_or(""),
                        &size_str,
                        "Completed",
                    );
                    let _ = app_clone.emit("download-progress", DownloadProgressPayload {
                        id: task_id_clone,
                        filename: final_filename_clone,
                        downloaded_bytes: total_size,
                        total_bytes: total_size,
                        speed_mbps: 0.0,
                        progress_percent: 100.0,
                        status: "Completed".into(),
                        threads: active_connections,
                    });
                }
                Err(err) => {
                    let _ = app_clone.emit("download-progress", DownloadProgressPayload {
                        id: task_id_clone,
                        filename: final_filename_clone,
                        downloaded_bytes: prog_clone.load(Ordering::Relaxed),
                        total_bytes: total_size,
                        speed_mbps: 0.0,
                        progress_percent: 0.0,
                        status: format!("Failed: {}", err),
                        threads: active_connections,
                    });
                }
            }
        });

        let app_ticker = app.clone();
        let task_id_ticker = task_id.clone();
        let filename_ticker = final_filename.clone();
        let cancel_ticker = cancel_flag.clone();
        let prog_ticker = progress_downloaded.clone();

        tokio::spawn(async move {
            let mut last_bytes = 0u64;
            let mut last_time = Instant::now();

            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                if cancel_ticker.load(Ordering::Relaxed) {
                    break;
                }

                let current_bytes = prog_ticker.load(Ordering::Relaxed);
                let elapsed = last_time.elapsed().as_secs_f64();
                let bytes_diff = current_bytes.saturating_sub(last_bytes);
                let speed_mbps = if elapsed > 0.0 { (bytes_diff as f64 * 8.0) / (elapsed * 1_000_000.0) } else { 0.0 };

                last_bytes = current_bytes;
                last_time = Instant::now();

                let percent = if total_size > 0 {
                    ((current_bytes as f64 / total_size as f64) * 100.0) as f32
                } else {
                    0.0
                };

                let _ = app_ticker.emit("download-progress", DownloadProgressPayload {
                    id: task_id_ticker.clone(),
                    filename: filename_ticker.clone(),
                    downloaded_bytes: current_bytes,
                    total_bytes: total_size,
                    speed_mbps: (speed_mbps * 10.0).round() / 10.0,
                    progress_percent: (percent * 10.0).round() / 10.0,
                    status: "Downloading".into(),
                    threads: active_connections,
                });

                if total_size > 0 && current_bytes >= total_size {
                    break;
                }
            }
        });

        Ok(task_id)
    }

    async fn download_multi_threaded(
        client: reqwest::Client,
        url: String,
        target_path: PathBuf,
        total_size: u64,
        connections: usize,
        progress: Arc<AtomicU64>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(), String> {
        let chunk_size = total_size / connections as u64;
        let mut handles = Vec::new();
        let mut part_files = Vec::new();

        for i in 0..connections {
            let start = i as u64 * chunk_size;
            let end = if i == connections - 1 { total_size - 1 } else { (i as u64 + 1) * chunk_size - 1 };
            let part_path = target_path.with_extension(format!("part{}", i));
            part_files.push(part_path.clone());

            let client_c = client.clone();
            let url_c = url.clone();
            let prog_c = progress.clone();
            let cancel_c = cancelled.clone();

            let handle = tokio::spawn(async move {
                let resp = client_c
                    .get(&url_c)
                    .header("Range", format!("bytes={}-{}", start, end))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;

                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&part_path)
                    .await
                    .map_err(|e| e.to_string())?;

                let mut stream = resp.bytes_stream();
                use futures_util::StreamExt;
                while let Some(chunk_res) = stream.next().await {
                    if cancel_c.load(Ordering::Relaxed) {
                        return Err("Download cancelled".into());
                    }
                    let chunk = chunk_res.map_err(|e| e.to_string())?;
                    file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                    prog_c.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                }
                file.flush().await.map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            });
            handles.push(handle);
        }

        for h in handles {
            h.await.map_err(|e| e.to_string())??;
        }

        let mut final_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&target_path)
            .await
            .map_err(|e| e.to_string())?;

        for p in &part_files {
            let mut part_f = File::open(p).await.map_err(|e| e.to_string())?;
            let mut buf = vec![0u8; 1024 * 1024];
            loop {
                let n = part_f.read(&mut buf).await.map_err(|e| e.to_string())?;
                if n == 0 { break; }
                final_file.write_all(&buf[..n]).await.map_err(|e| e.to_string())?;
            }
            let _ = tokio::fs::remove_file(p).await;
        }
        final_file.flush().await.map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn download_single_stream(
        client: reqwest::Client,
        url: String,
        target_path: PathBuf,
        progress: Arc<AtomicU64>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(), String> {
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&target_path)
            .await
            .map_err(|e| e.to_string())?;

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk_res) = stream.next().await {
            if cancelled.load(Ordering::Relaxed) {
                return Err("Download cancelled".into());
            }
            let chunk = chunk_res.map_err(|e| e.to_string())?;
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            progress.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        }
        file.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
