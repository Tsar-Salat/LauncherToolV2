//! Fetch recent videos from the Fallout Anomaly YouTube channel.
//!
//! Uses YouTube's public RSS feed (no API key). Channel id is resolved from the
//! @handle page once and cached; the feed is fetched with `curl` (no extra
//! crates / TLS stack). All network access is best-effort — a failure returns a
//! clear error the UI can swallow.

use crate::services::platform::CreationFlagsNoop;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Mutex;

const HANDLE_URL: &str = "https://www.youtube.com/@FalloutAnomaly";
/// Known channel id for @FalloutAnomaly — used as a fast default; if YouTube
/// ever changes it, `resolve_channel_id` refreshes from the handle page.
const DEFAULT_CHANNEL_ID: &str = "UCowoMPzQU_WfQcNp6bMj1zg";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeVideo {
    pub id: String,
    pub title: String,
    pub published: String,
    pub url: String,
    pub embed_url: String,
    pub thumbnail: String,
}

lazy_static! {
    static ref CHANNEL_ID: Mutex<Option<String>> = Mutex::new(None);
}

pub struct YouTube;

impl YouTube {
    /// Fetch up to `limit` most-recent videos (newest first).
    pub fn recent_videos(limit: usize) -> Result<Vec<YoutubeVideo>, String> {
        let channel_id = Self::resolve_channel_id();
        let feed_url = format!(
            "https://www.youtube.com/feeds/videos.xml?channel_id={}",
            channel_id
        );
        let xml = Self::curl(&feed_url)?;
        let mut videos = Self::parse_feed(&xml);
        videos.truncate(limit);
        if videos.is_empty() {
            return Err("No videos found in channel feed".to_string());
        }
        Ok(videos)
    }

    fn resolve_channel_id() -> String {
        if let Some(id) = CHANNEL_ID.lock().unwrap().clone() {
            return id;
        }
        // Try to scrape a fresh id from the handle page; fall back to the known
        // default so the feature still works offline-ish / on scrape changes.
        let id = Self::scrape_channel_id().unwrap_or_else(|| DEFAULT_CHANNEL_ID.to_string());
        *CHANNEL_ID.lock().unwrap() = Some(id.clone());
        id
    }

    fn scrape_channel_id() -> Option<String> {
        let html = Self::curl_with_consent(HANDLE_URL).ok()?;
        // Look for `"channelId":"UC..."` or `channel_id=UC...`
        for marker in ["\"channelId\":\"", "channel_id="] {
            if let Some(start) = html.find(marker) {
                let rest = &html[start + marker.len()..];
                let id: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .collect();
                if id.starts_with("UC") && id.len() == 24 {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Parse `<entry>` blocks out of the Atom feed.
    fn parse_feed(xml: &str) -> Vec<YoutubeVideo> {
        let mut videos = Vec::new();
        for entry in xml.split("<entry>").skip(1) {
            let id = between(entry, "<yt:videoId>", "</yt:videoId>");
            let title = between(entry, "<title>", "</title>");
            let published = between(entry, "<published>", "</published>");
            if let Some(id) = id {
                videos.push(YoutubeVideo {
                    url: format!("https://www.youtube.com/watch?v={}", id),
                    embed_url: format!("https://www.youtube.com/embed/{}", id),
                    thumbnail: format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", id),
                    title: title.map(decode_xml).unwrap_or_default(),
                    published: published.unwrap_or_default(),
                    id,
                });
            }
        }
        videos
    }

    fn curl(url: &str) -> Result<String, String> {
        Self::run_curl(&["-fsSL", "--max-time", "15", "-A", "Mozilla/5.0", url])
    }

    fn curl_with_consent(url: &str) -> Result<String, String> {
        Self::run_curl(&[
            "-fsSL",
            "--max-time",
            "15",
            "-A",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
            "-H",
            "Cookie: CONSENT=YES+1",
            url,
        ])
    }

    fn run_curl(args: &[&str]) -> Result<String, String> {
        let out = Command::new("curl")
            .args(args)
            .creation_flags_noop()
            .output()
            .map_err(|e| format!("curl unavailable: {}", e))?;
        if !out.status.success() {
            return Err(format!("curl failed ({})", out.status.code().unwrap_or(-1)));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}

fn between(haystack: &str, open: &str, close: &str) -> Option<String> {
    let start = haystack.find(open)? + open.len();
    let end = haystack[start..].find(close)? + start;
    Some(haystack[start..end].to_string())
}

fn decode_xml(s: String) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}
