use crate::services::platform::CreationFlagsNoop;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A clickable mod/source reference shown under a changelog entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogLink {
    pub label: String,
    pub url: String,
}

/// One titled bullet block within a changelog category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub title: String,
    pub notes: Vec<String>,
    pub links: Vec<ChangelogLink>,
}

/// A category heading (e.g. "Added") grouping a set of entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogSection {
    pub category: String,
    pub entries: Vec<ChangelogEntry>,
}

/// Parsed changelog for the dashboard "Latest Changes" feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogData {
    pub version: String,
    pub released: String,
    pub sections: Vec<ChangelogSection>,
}

/// Remote changelog published by the modlist authors.
pub const CHANGELOG_MD_URL: &str =
    "https://raw.githubusercontent.com/Fallout-Anomaly/changelog/main/changelog.md";

/// Bundled fallback so the dashboard always has content if the network is down
/// and no local override exists.
const DEFAULT_CHANGELOG_MD: &str = include_str!("default_changelog.md");

pub struct ChangelogService;

impl ChangelogService {
    /// Load changelog data. Resolution order:
    /// 1. `%APPDATA%\FallenWorldLauncher\changelog.md` (local override)
    /// 2. remote `changelog.md` (the live, authoritative source)
    /// 3. embedded default `.md`
    pub fn load() -> Result<ChangelogData, String> {
        if let Some(path) = Self::override_path() {
            if path.is_file() {
                if let Ok(content) = fs::read_to_string(&path) {
                    return Ok(Self::parse_markdown(&content));
                }
            }
        }
        if let Ok(remote) = Self::fetch_remote() {
            if !remote.trim().is_empty() {
                return Ok(Self::parse_markdown(&remote));
            }
        }
        Ok(Self::parse_markdown(DEFAULT_CHANGELOG_MD))
    }

    fn fetch_remote() -> Result<String, String> {
        let out = std::process::Command::new("curl")
            .args(["-fsSL", "--max-time", "12", "-A", "FallenWorldLauncher/1.0", CHANGELOG_MD_URL])
            .creation_flags_noop()
            .output()
            .map_err(|e| format!("curl unavailable: {}", e))?;
        if !out.status.success() {
            return Err("changelog.md fetch failed".to_string());
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    fn override_path() -> Option<PathBuf> {
        let appdata = std::env::var("APPDATA").ok()?;
        Some(PathBuf::from(appdata).join("FallenWorldLauncher").join("changelog.md"))
    }

    /// Dependency-free Markdown parser supporting two changelog shapes:
    ///   1. Structured: `## Category` · `### Entry` · `- note` · `[label](url)`.
    ///   2. Flat (the live remote file): one entry per line, e.g.
    ///      `CHANGELOG 5/15/26 CLICK [HERE](https://…)` — links are extracted
    ///      and the remaining text becomes the entry title.
    pub fn parse_markdown(md: &str) -> ChangelogData {
        let mut version = String::new();
        let mut released = String::new();
        let mut sections: Vec<ChangelogSection> = Vec::new();
        // Did the current entry come from an explicit `###` (structured)? If so,
        // following content lines are its notes/links; otherwise each line is
        // its own flat entry.
        let mut explicit_entry = false;

        let ensure_section = |sections: &mut Vec<ChangelogSection>| {
            if sections.is_empty() {
                sections.push(ChangelogSection { category: "Changelog".into(), entries: Vec::new() });
            }
        };

        for raw in md.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }

            if sections.is_empty() {
                if let Some(v) = line.strip_prefix("version:") { version = v.trim().into(); continue; }
                if let Some(r) = line.strip_prefix("released:") { released = r.trim().into(); continue; }
            }

            if let Some(cat) = line.strip_prefix("## ") {
                sections.push(ChangelogSection { category: cat.trim().into(), entries: Vec::new() });
                explicit_entry = false;
                continue;
            }
            if let Some(title) = line.strip_prefix("### ") {
                ensure_section(&mut sections);
                sections.last_mut().unwrap().entries.push(ChangelogEntry {
                    title: title.trim().into(), notes: Vec::new(), links: Vec::new(),
                });
                explicit_entry = true;
                continue;
            }
            if line.starts_with('#') {
                continue; // ignore single-`#` titles
            }

            let (text, links) = Self::extract_links(line);

            if explicit_entry {
                // Structured: attach to the current ### entry.
                let entry = sections.last_mut().unwrap().entries.last_mut().unwrap();
                entry.links.extend(links);
                if !text.is_empty() {
                    let note = text.strip_prefix("- ").or_else(|| text.strip_prefix("* ")).unwrap_or(&text);
                    entry.notes.push(note.trim().to_string());
                }
            } else {
                // Flat: one entry per line.
                ensure_section(&mut sections);
                let title = text.trim().trim_end_matches("CLICK").trim().to_string();
                sections.last_mut().unwrap().entries.push(ChangelogEntry {
                    title: if title.is_empty() { "Changelog".into() } else { title },
                    notes: Vec::new(),
                    links,
                });
            }
        }

        ChangelogData {
            version: if version.is_empty() { "1.0".into() } else { version },
            released,
            sections,
        }
    }

    /// Extract all `[label](url)` links from a line, returning the line with the
    /// link markdown removed plus the parsed links.
    fn extract_links(line: &str) -> (String, Vec<ChangelogLink>) {
        let mut links = Vec::new();
        let mut text = String::new();
        let mut rest = line;
        while let Some(open) = rest.find('[') {
            if let Some(close) = rest[open..].find("](") {
                let close = open + close;
                if let Some(end_rel) = rest[close + 2..].find(')') {
                    let end = close + 2 + end_rel;
                    text.push_str(&rest[..open]);
                    let label = rest[open + 1..close].trim().to_string();
                    let url = rest[close + 2..end].trim().to_string();
                    if !label.is_empty() {
                        links.push(ChangelogLink { label, url });
                    }
                    rest = &rest[end + 1..];
                    continue;
                }
            }
            // No complete link from here — keep the rest verbatim.
            break;
        }
        text.push_str(rest);
        (text.trim().to_string(), links)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_default_parses() {
        let data = ChangelogService::parse_markdown(DEFAULT_CHANGELOG_MD);
        assert!(!data.sections.is_empty());
        assert_eq!(data.sections[0].category, "Added");
        // First entry has a title; Gunners entry has notes + a link.
        let gunners = data.sections[0]
            .entries
            .iter()
            .find(|e| e.title == "Gunners")
            .expect("Gunners entry");
        assert!(!gunners.notes.is_empty());
        assert_eq!(gunners.links[0].label, "Gunners Overhaul");
    }

    #[test]
    fn flat_link_list_parses() {
        let md = "CHANGELOG 5/15/26 CLICK [HERE](https://example.com/23)\nCHANGELOG 5/10/26 CLICK [HERE](https://example.com/21)";
        let data = ChangelogService::parse_markdown(md);
        assert_eq!(data.sections.len(), 1);
        assert_eq!(data.sections[0].category, "Changelog");
        assert_eq!(data.sections[0].entries.len(), 2);
        assert_eq!(data.sections[0].entries[0].title, "CHANGELOG 5/15/26");
        assert_eq!(data.sections[0].entries[0].links[0].url, "https://example.com/23");
    }
}
