//! ENB Manager.
//!
//! Model:
//! * The **default ENB** ships read-only in `<mods>/Fallen World FOMOD Resources/ENBs`.
//!   Users cannot edit or delete it. Its preview is `Showcase.png` in that folder.
//! * The **live ENB** is deployed into `<mods>/Fallen World Optional Mods` (the same
//!   MO2 mod that holds the other optional files). Only one ENB is live at a time.
//! * When the user installs a custom ENB, the previously-deployed ENB files are
//!   removed first (tracked via a manifest + the known default file list) so no
//!   stale files from the old ENB linger. New files overwrite old ones.
//! * `Root/enblocal.ini` in the deploy folder is **protected**: never overwritten
//!   and never deleted. A user-supplied `enblocal.ini` is silently rejected.

use crate::models::OperationResult;
use crate::services::PathDiscoveryService;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_ENB_SUBDIR: &str = "Fallen World FOMOD Resources/ENBs";
const DEPLOY_SUBDIR: &str = "Fallen World Optional Mods";
/// Our enblocal.ini: never overwritten and never deleted. A user-supplied copy
/// is rejected. Compared case- and separator-insensitively.
const PROTECTED_ENBLOCAL: &str = "root/enblocal.ini";
/// Files that must never be deleted during an ENB swap (the ENB wrapper binaries
/// plus enblocal.ini). They may still be overwritten by a source that provides a
/// newer copy — except enblocal.ini, which is never overwritten either.
const KEEP_ON_REMOVAL: &[&str] = &[
    "root/enblocal.ini",
    "root/d3d11.dll",
    "root/d3dcompiler_46e.dll",
];
const SHOWCASE_NAME: &str = "Showcase.png";
const CUSTOM_SHOWCASE_STEM: &str = "enb_custom_showcase";

#[derive(Serialize, Deserialize, Default)]
struct EnbState {
    /// "default" or "custom".
    active: String,
    custom_name: Option<String>,
    /// Filename of the copied custom preview within the app data dir.
    custom_showcase: Option<String>,
    /// Relative paths the launcher last deployed into the deploy folder.
    manifest: Vec<String>,
}

#[derive(Serialize)]
pub struct EnbInfo {
    pub name: String,
    pub has_showcase: bool,
}

#[derive(Serialize, Default)]
pub struct EnbEffect {
    pub name: String,
    pub enabled: bool,
}

/// A best-effort, CSS-mappable summary of an enbseries.ini for the live preview.
#[derive(Serialize, Default)]
pub struct EnbConfig {
    pub found: bool,
    pub brightness: f32,
    pub gamma: f32,
    /// Bloom amount (0 when bloom disabled).
    pub bloom: f32,
    /// Lens amount (0 when lens disabled).
    pub lens: f32,
    pub enable_bloom: bool,
    pub enable_lens: bool,
    pub enable_dof: bool,
    pub enable_ssao: bool,
    pub effects: Vec<EnbEffect>,
    /// All parsed values keyed by "section|key" (both lowercased) for the editor.
    pub values: std::collections::HashMap<String, String>,
    /// True when these values came from the live (editable) deploy enbseries.ini.
    pub editable: bool,
}

/// A single enbseries.ini edit from the configurator.
#[derive(Deserialize)]
pub struct EnbIniChange {
    pub section: String,
    pub key: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct EnbStatus {
    pub default: EnbInfo,
    pub custom: Option<EnbInfo>,
    /// "default" or "custom".
    pub active: String,
    pub deploy_path: String,
}

pub struct EnbManager;

impl EnbManager {
    // ── paths ────────────────────────────────────────────────────────────
    fn mods_folder() -> Result<PathBuf, String> {
        let paths = PathDiscoveryService::discover()?;
        Ok(PathBuf::from(paths.mods_folder))
    }

    fn default_dir() -> Result<PathBuf, String> {
        Ok(Self::mods_folder()?.join(DEFAULT_ENB_SUBDIR))
    }

    fn deploy_dir() -> Result<PathBuf, String> {
        Ok(Self::mods_folder()?.join(DEPLOY_SUBDIR))
    }

    fn app_dir() -> Result<PathBuf, String> {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .or_else(|_| dirs::data_dir().ok_or_else(|| "No data dir".to_string()))?;
        let dir = base.join("FallenWorldLauncher");
        fs::create_dir_all(&dir).map_err(|e| format!("Cannot create app dir: {}", e))?;
        Ok(dir)
    }

    fn state_path() -> Result<PathBuf, String> {
        Ok(Self::app_dir()?.join("enb_state.json"))
    }

    fn load_state() -> EnbState {
        Self::state_path()
            .ok()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| EnbState { active: "default".to_string(), ..Default::default() })
    }

    fn save_state(state: &EnbState) -> Result<(), String> {
        let path = Self::state_path()?;
        let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| format!("Cannot save ENB state: {}", e))
    }

    // ── helpers ──────────────────────────────────────────────────────────
    fn norm(rel: &str) -> String {
        rel.replace('\\', "/").to_ascii_lowercase()
    }
    /// enblocal.ini — never copied from a source, never deleted.
    fn is_enblocal(rel: &str) -> bool {
        Self::norm(rel) == PROTECTED_ENBLOCAL
    }
    /// File must survive a swap (never deleted): enblocal.ini + ENB binaries.
    fn is_kept(rel: &str) -> bool {
        let n = Self::norm(rel);
        KEEP_ON_REMOVAL.iter().any(|k| *k == n)
    }

    /// Recursively collect (absolute file, forward-slash relpath) under `base`.
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(PathBuf, String)>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                Self::walk(base, &p, out);
            } else if p.is_file() {
                if let Ok(rel) = p.strip_prefix(base) {
                    out.push((p.clone(), rel.to_string_lossy().replace('\\', "/")));
                }
            }
        }
    }

    /// Files that make up the default ENB, as (source, relpath) pairs. Excludes
    /// launcher metadata (meta.ini, Showcase.png) and the protected enblocal.ini.
    fn collect_default_files() -> Result<Vec<(PathBuf, String)>, String> {
        let dir = Self::default_dir()?;
        if !dir.is_dir() {
            return Err(format!("Default ENB folder not found: {}", dir.display()));
        }
        let mut all = Vec::new();
        Self::walk(&dir, &dir, &mut all);
        Ok(all
            .into_iter()
            .filter(|(_, rel)| {
                let base = rel.rsplit('/').next().unwrap_or(rel);
                !base.eq_ignore_ascii_case("meta.ini")
                    && !base.eq_ignore_ascii_case(SHOWCASE_NAME)
                    && !Self::is_enblocal(rel)
            })
            .collect())
    }

    /// Files for a user-selected ENB folder, as (source, dst-relpath) pairs.
    /// Routing: if the folder already has a `Root/` dir it is treated as MO2
    /// layout and copied as-is. Otherwise, if it contains ENB binaries at the top
    /// (d3d11.dll / enbseries.ini / enbseries/), the whole thing is wrapped under
    /// `Root/` (so game-root files virtualise correctly). The protected
    /// enblocal.ini is dropped wherever it would land.
    fn collect_custom_files(source: &Path) -> Result<Vec<(PathBuf, String)>, String> {
        if !source.is_dir() {
            return Err("Selected ENB path is not a folder.".to_string());
        }
        let has_root = source.join("Root").is_dir();
        let has_enb_top = source.join("d3d11.dll").exists()
            || source.join("enbseries.ini").exists()
            || source.join("enbseries").is_dir();
        let wrap_under_root = !has_root && has_enb_top;

        let mut all = Vec::new();
        Self::walk(source, source, &mut all);

        let mut files = Vec::new();
        for (src, rel0) in all {
            let rel = if wrap_under_root { format!("Root/{}", rel0) } else { rel0 };
            if Self::is_enblocal(&rel) {
                continue; // silently reject user-supplied enblocal.ini
            }
            files.push((src, rel));
        }
        if files.is_empty() {
            return Err("No installable ENB files found in the selected folder.".to_string());
        }
        Ok(files)
    }

    /// Remove `removal` relpaths from the deploy folder (never the protected one),
    /// then copy `files` in, returning the new manifest. Empty directories left by
    /// removals are pruned.
    fn deploy(
        deploy: &Path,
        removal: &[String],
        files: &[(PathBuf, String)],
    ) -> Result<Vec<String>, String> {
        fs::create_dir_all(deploy).map_err(|e| format!("Cannot create deploy folder: {}", e))?;

        for rel in removal {
            if Self::is_kept(rel) {
                continue; // never delete enblocal.ini or the ENB wrapper binaries
            }
            let target = deploy.join(rel);
            if target.is_file() {
                let _ = fs::remove_file(&target);
                Self::prune_empty_parents(deploy, &target);
            }
        }

        let mut manifest = Vec::with_capacity(files.len());
        for (src, rel) in files {
            let dst = deploy.join(rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create {}: {}", parent.display(), e))?;
            }
            fs::copy(src, &dst)
                .map_err(|e| format!("Failed to copy {} -> {}: {}", src.display(), dst.display(), e))?;
            manifest.push(rel.clone());
        }
        Ok(manifest)
    }

    /// Remove empty directories from `leaf`'s parent up toward (but not including)
    /// `stop`.
    fn prune_empty_parents(stop: &Path, leaf: &Path) {
        let mut cur = leaf.parent();
        while let Some(dir) = cur {
            if dir == stop || !dir.starts_with(stop) {
                break;
            }
            let is_empty = fs::read_dir(dir).map(|mut it| it.next().is_none()).unwrap_or(false);
            if !is_empty || fs::remove_dir(dir).is_err() {
                break;
            }
            cur = dir.parent();
        }
    }

    /// Best-effort preview discovery: the largest top-level image in the folder.
    fn detect_showcase(source: &Path) -> Option<PathBuf> {
        let mut best: Option<(u64, PathBuf)> = None;
        for entry in fs::read_dir(source).ok()?.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
                    best = Some((size, p));
                }
            }
        }
        best.map(|(_, p)| p)
    }

    /// Copy an image into the app dir as the custom showcase; returns its filename.
    fn copy_showcase(img: &Path) -> Option<String> {
        let ext = img.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
        let fname = format!("{}.{}", CUSTOM_SHOWCASE_STEM, ext);
        let dst = Self::app_dir().ok()?.join(&fname);
        fs::copy(img, &dst).ok().map(|_| fname)
    }

    fn is_zip(path: &Path) -> bool {
        path.is_file()
            && path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("zip"))
                .unwrap_or(false)
    }

    /// Extract a zip into a fresh temp dir (same approach as Add Mod). Returns
    /// `(temp_dir_to_clean, enb_root)`. A single top-level folder becomes the
    /// root; otherwise the temp dir itself is the root.
    fn extract_zip(zip_path: &Path) -> Result<(PathBuf, PathBuf), String> {
        let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Not a valid zip archive: {}", e))?;

        let temp = std::env::temp_dir().join(format!("fwl_enb_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp).map_err(|e| format!("Cannot create temp dir: {}", e))?;
        archive.extract(&temp).map_err(|e| format!("Failed to extract zip: {}", e))?;

        let entries: Vec<_> = fs::read_dir(&temp)
            .map_err(|e| format!("Cannot read extracted files: {}", e))?
            .flatten()
            .collect();
        if entries.len() == 1 && entries[0].path().is_dir() {
            let root = entries[0].path();
            Ok((temp, root))
        } else {
            Ok((temp.clone(), temp))
        }
    }

    /// Minimal INI parser -> { section(lower) : { key(lower) : value } }.
    fn parse_ini(path: &Path) -> Option<std::collections::HashMap<String, std::collections::HashMap<String, String>>> {
        use std::collections::HashMap;
        let content = fs::read_to_string(path).ok()?;
        let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut cur = String::new();
        for line in content.lines() {
            let t = line.trim();
            if t.starts_with('[') && t.ends_with(']') {
                cur = t[1..t.len() - 1].to_ascii_lowercase();
            } else if let Some((k, v)) = t.split_once('=') {
                map.entry(cur.clone())
                    .or_default()
                    .insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
            }
        }
        Some(map)
    }

    fn base64(data: &[u8]) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push(if chunk.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
            out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
        }
        out
    }

    fn image_data_url(path: &Path) -> Result<String, String> {
        let bytes = fs::read(path).map_err(|e| format!("Cannot read image: {}", e))?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
        let mime = match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            _ => "image/png",
        };
        Ok(format!("data:{};base64,{}", mime, Self::base64(&bytes)))
    }

    fn default_name() -> String {
        // Derive a friendly name from the bundled meta.ini (installationFile), else
        // fall back to the folder name.
        if let Ok(dir) = Self::default_dir() {
            if let Ok(meta) = fs::read_to_string(dir.join("meta.ini")) {
                for line in meta.lines() {
                    if let Some(v) = line.trim().strip_prefix("installationFile=") {
                        let v = v.trim();
                        if !v.is_empty() {
                            // "Hope ENB-97411-1-0-...zip" -> "Hope ENB"
                            let name = v.split('-').next().unwrap_or(v).trim();
                            if !name.is_empty() {
                                return name.to_string();
                            }
                        }
                    }
                }
            }
        }
        "Default ENB".to_string()
    }

    // ── public API ───────────────────────────────────────────────────────
    pub fn get_status() -> Result<EnbStatus, String> {
        let state = Self::load_state();
        let default_dir = Self::default_dir()?;
        let deploy = Self::deploy_dir()?;

        let default = EnbInfo {
            name: Self::default_name(),
            has_showcase: default_dir.join(SHOWCASE_NAME).is_file(),
        };

        let custom = state.custom_name.as_ref().map(|name| {
            let has_showcase = state
                .custom_showcase
                .as_ref()
                .map(|f| Self::app_dir().map(|d| d.join(f).is_file()).unwrap_or(false))
                .unwrap_or(false);
            EnbInfo { name: name.clone(), has_showcase }
        });

        let active = match state.active.as_str() {
            "none" | "custom" => state.active.clone(),
            _ => "default".to_string(),
        };
        Ok(EnbStatus {
            default,
            custom,
            active,
            deploy_path: deploy.to_string_lossy().to_string(),
        })
    }

    pub fn apply_default() -> OperationResult {
        match Self::apply_default_inner() {
            Ok(n) => OperationResult { success: true, message: format!("Default ENB applied ({} files).", n) },
            Err(e) => OperationResult { success: false, message: e },
        }
    }

    fn apply_default_inner() -> Result<usize, String> {
        let deploy = Self::deploy_dir()?;
        let defaults = Self::collect_default_files()?;
        let default_rels: Vec<String> = defaults.iter().map(|(_, r)| r.clone()).collect();

        let mut state = Self::load_state();
        // Remove whatever we deployed last, plus the known default set, then lay
        // the default down fresh.
        let mut removal = state.manifest.clone();
        removal.extend(default_rels.iter().cloned());

        let manifest = Self::deploy(&deploy, &removal, &defaults)?;
        let count = manifest.len();
        state.active = "default".to_string();
        state.manifest = manifest;
        Self::save_state(&state)?;
        Ok(count)
    }

    pub fn install_custom(source_dir: &str, name: &str, showcase: Option<&str>) -> OperationResult {
        match Self::install_custom_inner(source_dir, name, showcase) {
            Ok(n) => OperationResult { success: true, message: format!("Installed custom ENB '{}' ({} files).", name, n) },
            Err(e) => OperationResult { success: false, message: e },
        }
    }

    fn install_custom_inner(source_dir: &str, name: &str, showcase: Option<&str>) -> Result<usize, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Please provide a name for the ENB.".to_string());
        }

        // Accept a .zip archive (extracted to a temp dir, same as Add Mod) or a
        // folder. Cleanup of the temp dir runs regardless of success.
        let src = PathBuf::from(source_dir);
        let mut temp_cleanup: Option<PathBuf> = None;
        let source = if Self::is_zip(&src) {
            let (temp, root) = Self::extract_zip(&src)?;
            temp_cleanup = Some(temp);
            root
        } else if src.is_dir() {
            src.clone()
        } else {
            return Err("Select an ENB .zip archive or folder.".to_string());
        };

        let result = (|| -> Result<usize, String> {
            let files = Self::collect_custom_files(&source)?;
            let deploy = Self::deploy_dir()?;

            let mut state = Self::load_state();
            // Remove the previously-deployed set AND the known default ENB files
            // (which may have been placed by the FOMOD installer without a
            // manifest), so no stale ENB files survive.
            let defaults = Self::collect_default_files().unwrap_or_default();
            let mut removal = state.manifest.clone();
            removal.extend(defaults.iter().map(|(_, r)| r.clone()));

            let manifest = Self::deploy(&deploy, &removal, &files)?;
            let count = manifest.len();

            // Preview: prefer the user-supplied image, else auto-detect one in the
            // selected ENB (best-effort).
            let custom_showcase = showcase
                .map(PathBuf::from)
                .filter(|p| p.is_file())
                .and_then(|p| Self::copy_showcase(&p))
                .or_else(|| Self::detect_showcase(&source).and_then(|p| Self::copy_showcase(&p)));

            state.active = "custom".to_string();
            state.custom_name = Some(name.to_string());
            state.custom_showcase = custom_showcase;
            state.manifest = manifest;
            Self::save_state(&state)?;
            Ok(count)
        })();

        if let Some(tmp) = temp_cleanup {
            let _ = fs::remove_dir_all(&tmp);
        }
        result
    }

    pub fn disable_enb() -> OperationResult {
        match Self::disable_enb_inner() {
            Ok(n) => OperationResult { success: true, message: format!("ENB disabled ({} files removed from Optional Mods).", n) },
            Err(e) => OperationResult { success: false, message: e },
        }
    }

    fn disable_enb_inner() -> Result<usize, String> {
        let deploy = Self::deploy_dir()?;
        let mut state = Self::load_state();

        // Build removal list: current manifest + default ENB files (which may have
        // been placed by the FOMOD installer without going through the launcher).
        let defaults = Self::collect_default_files().unwrap_or_default();
        let mut removal = state.manifest.clone();
        removal.extend(defaults.iter().map(|(_, r)| r.clone()));
        removal.sort();
        removal.dedup();

        let mut removed = 0usize;
        for rel in &removal {
            if Self::is_kept(rel) { continue; }
            let target = deploy.join(rel);
            if target.is_file() {
                let _ = fs::remove_file(&target);
                Self::prune_empty_parents(&deploy, &target);
                removed += 1;
            }
        }

        state.active = "none".to_string();
        state.manifest = Vec::new();
        Self::save_state(&state)?;
        Ok(removed)
    }

    pub fn remove_custom() -> OperationResult {
        // Revert to default, then forget the custom entry + its preview.
        match Self::apply_default_inner() {
            Ok(n) => {
                let mut state = Self::load_state();
                if let Some(f) = state.custom_showcase.take() {
                    if let Ok(dir) = Self::app_dir() {
                        let _ = fs::remove_file(dir.join(f));
                    }
                }
                state.custom_name = None;
                let _ = Self::save_state(&state);
                OperationResult { success: true, message: format!("Custom ENB removed; default restored ({} files).", n) }
            }
            Err(e) => OperationResult { success: false, message: e },
        }
    }

    /// Returns a `data:` URL for the requested showcase, or an empty string if
    /// none is available.
    pub fn showcase(which: &str) -> String {
        let path = if which == "custom" {
            let state = Self::load_state();
            match (Self::app_dir().ok(), state.custom_showcase) {
                (Some(dir), Some(f)) => dir.join(f),
                _ => return String::new(),
            }
        } else {
            match Self::default_dir() {
                Ok(d) => d.join(SHOWCASE_NAME),
                Err(_) => return String::new(),
            }
        };
        if !path.is_file() {
            return String::new();
        }
        Self::image_data_url(&path).unwrap_or_default()
    }

    fn live_enbseries() -> Result<PathBuf, String> {
        Ok(Self::deploy_dir()?.join("Root").join("enbseries.ini"))
    }

    /// Parse the relevant enbseries.ini into a CSS-mappable summary + raw values.
    /// `target` is "source" (bundled read-only default) or "live" (the editable
    /// deploy copy).
    pub fn config(target: &str) -> EnbConfig {
        let editable = target != "source";
        let path = if editable {
            Self::live_enbseries()
        } else {
            Self::default_dir().map(|d| d.join("Root").join("enbseries.ini"))
        };
        let Ok(path) = path else { return EnbConfig::default() };
        let Some(ini) = Self::parse_ini(&path) else { return EnbConfig::default() };

        let mut values = std::collections::HashMap::new();
        for (sec, m) in &ini {
            for (k, v) in m {
                values.insert(format!("{}|{}", sec, k), v.clone());
            }
        }

        let getf = |sec: &str, key: &str, def: f32| {
            ini.get(sec).and_then(|m| m.get(key)).and_then(|v| v.parse::<f32>().ok()).unwrap_or(def)
        };
        let getb = |sec: &str, key: &str| {
            ini.get(sec).and_then(|m| m.get(key)).map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(false)
        };

        let enable_bloom = getb("effect", "enablebloom");
        let enable_lens = getb("effect", "enablelens");
        let enable_dof = getb("effect", "enabledepthoffield");
        let enable_ssao = getb("effect", "enablessao");

        let effects = vec![
            EnbEffect { name: "Bloom".into(), enabled: enable_bloom },
            EnbEffect { name: "Lens".into(), enabled: enable_lens },
            EnbEffect { name: "Depth of Field".into(), enabled: enable_dof },
            EnbEffect { name: "Ambient Occlusion".into(), enabled: enable_ssao },
            EnbEffect { name: "Adaptation".into(), enabled: getb("effect", "enableadaptation") },
            EnbEffect { name: "Reflections".into(), enabled: getb("effect", "enablereflections") },
            EnbEffect { name: "Water".into(), enabled: getb("effect", "enablewater") },
            EnbEffect { name: "Subsurface Scattering".into(), enabled: getb("effect", "enablesubsurfacescattering") },
        ];

        EnbConfig {
            found: true,
            brightness: getf("colorcorrection", "brightness", 1.0),
            gamma: getf("colorcorrection", "gammacurve", 1.0),
            bloom: if enable_bloom { getf("bloom", "amountday", 1.0) } else { 0.0 },
            lens: if enable_lens { getf("lens", "amountday", 1.0) } else { 0.0 },
            enable_bloom,
            enable_lens,
            enable_dof,
            enable_ssao,
            effects,
            values,
            editable,
        }
    }

    /// Write edits back into the live deploy enbseries.ini, updating existing keys
    /// in place (preserving comments, ordering, and line endings). Keys not found
    /// are skipped.
    pub fn save_config(changes: &[EnbIniChange]) -> OperationResult {
        match Self::save_config_inner(changes) {
            Ok(n) => OperationResult { success: true, message: format!("Saved {} setting(s) to enbseries.ini.", n) },
            Err(e) => OperationResult { success: false, message: e },
        }
    }

    fn save_config_inner(changes: &[EnbIniChange]) -> Result<usize, String> {
        let path = Self::live_enbseries()?;
        if !path.is_file() {
            return Err("enbseries.ini not found for the active ENB.".to_string());
        }
        let content = fs::read_to_string(&path).map_err(|e| format!("Cannot read enbseries.ini: {}", e))?;
        let crlf = content.contains("\r\n");

        let mut applied = 0usize;
        let mut out: Vec<String> = Vec::new();
        let mut cur = String::new();

        for raw in content.split('\n') {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            let t = line.trim();

            if t.starts_with('[') && t.ends_with(']') {
                cur = t[1..t.len() - 1].to_ascii_lowercase();
                out.push(line.to_string());
                continue;
            }

            if let Some((k, _)) = t.split_once('=') {
                let key = k.trim();
                if let Some(ch) = changes.iter().find(|c| {
                    c.section.eq_ignore_ascii_case(&cur) && c.key.eq_ignore_ascii_case(key)
                }) {
                    let indent = &line[..line.len() - line.trim_start().len()];
                    out.push(format!("{}{}={}", indent, key, ch.value));
                    applied += 1;
                    continue;
                }
            }
            out.push(line.to_string());
        }

        let joined = out.join(if crlf { "\r\n" } else { "\n" });
        fs::write(&path, joined).map_err(|e| format!("Cannot write enbseries.ini: {}", e))?;
        Ok(applied)
    }
}
