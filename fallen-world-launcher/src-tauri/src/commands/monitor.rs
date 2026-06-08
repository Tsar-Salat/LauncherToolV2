use crate::services::{
    AnomalyUpdate, AnomalyUpdateService, ChangelogData, ChangelogService, LiveStats, ProcessStatus,
    SystemInfoService, SystemMonitor, YouTube, YoutubeVideo,
};

/// Live CPU/GPU/RAM sample for the dashboard Status widget. Poll ~every 2s.
#[tauri::command]
pub fn get_live_stats() -> LiveStats {
    SystemMonitor::sample()
}

/// Running/Stopped state for Fallout 4 + Mod Organizer 2.
#[tauri::command]
pub fn get_process_status() -> ProcessStatus {
    SystemInfoService::process_status()
}

/// Parsed `changelog.md` for the dashboard "Latest Changes" feed.
#[tauri::command]
pub fn get_changelog_data() -> Result<ChangelogData, String> {
    ChangelogService::load()
}

/// Recent videos from the Fallout Anomaly YouTube channel (newest first).
#[tauri::command]
pub async fn get_youtube_videos(limit: Option<usize>) -> Result<Vec<YoutubeVideo>, String> {
    let n = limit.unwrap_or(8);
    tauri::async_runtime::spawn_blocking(move || YouTube::recent_videos(n))
        .await
        .map_err(|e| format!("youtube task panicked: {}", e))?
}

/// Check version.md for a new modlist update (drives the launch banner).
#[tauri::command]
pub async fn check_anomaly_update() -> Result<AnomalyUpdate, String> {
    tauri::async_runtime::spawn_blocking(AnomalyUpdateService::check)
        .await
        .map_err(|e| format!("update task panicked: {}", e))?
}

/// Record a version as seen so it stops being flagged as new.
#[tauri::command]
pub fn mark_update_seen(version: String) -> Result<(), String> {
    AnomalyUpdateService::mark_seen(&version)
}

/// Fetch the fallback news-banner text (shown when no update is pending).
#[tauri::command]
pub async fn get_news_banner() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(AnomalyUpdateService::news_banner)
        .await
        .map_err(|e| format!("news task panicked: {}", e))?
}
