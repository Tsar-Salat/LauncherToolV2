import { invoke } from '@tauri-apps/api/tauri'

export interface Mod {
  id: string
  name: string
  version: string
  author: string
  description: string
  enabled: boolean
  load_order: number
  dependencies: string[]
  conflicts: string[]
  last_updated: string
}

export interface Preset {
  name: string
  preset_type: string
  preview_image: number[] | null
  installed: boolean
}

export interface GameProfile {
  name: string
  is_active: boolean
  enabled_mods: string[]
  ini_overrides: Record<string, string>
  mcm_preset?: string
  created_date: string
  last_modified: string
}

export interface IniConfig {
  sections: Record<string, Record<string, string>>
}

export interface OperationResult {
  success: boolean
  message: string
}

// Game commands — path discovery happens on the backend
export const gameApi = {
  launch: () => invoke<OperationResult>('launch_game'),
  launchMo2: () => invoke<OperationResult>('launch_mo2'),
  getInfo: () => invoke<string | null>('get_game_info'),
  openExternal: (target: string) => invoke<void>('open_external', { target }),
  quit: () => invoke<void>('quit_app'),
}

/** Whether to close the launcher when the game launches (Settings). Defaults to
 *  true — only off when the user has explicitly unchecked it. */
export function shouldCloseOnLaunch(): boolean {
  try {
    const s = JSON.parse(localStorage.getItem('appSettings') || '{}')
    return s.closeOnLaunch !== false
  } catch {
    return true
  }
}

// Hardcore-loot scarcity. Values are ChanceNone (inverted): 0 = always drops, 95 = almost
// never. The backend rewrites the RobCo loot INI from these on Play.
export interface LootChances {
  guns: number
  meds: number
  food: number
  drink: number
  ammo: number
  junk: number
}

export const DEFAULT_SCARCITY: LootChances = {
  guns: 90, meds: 78, food: 75, drink: 75, ammo: 55, junk: 40,
}

const SCARCITY_KEY = 'fwc_loot_scarcity'

export function loadScarcity(): LootChances {
  try {
    const raw = localStorage.getItem(SCARCITY_KEY)
    return raw ? { ...DEFAULT_SCARCITY, ...JSON.parse(raw) } : { ...DEFAULT_SCARCITY }
  } catch {
    return { ...DEFAULT_SCARCITY }
  }
}

export function saveScarcity(c: LootChances): void {
  try { localStorage.setItem(SCARCITY_KEY, JSON.stringify(c)) } catch { /* ignore */ }
}

export const lootApi = {
  applyScarcity: (chances: LootChances) =>
    invoke<OperationResult>('apply_loot_scarcity', { chances }),
}

// ── Dashboard: live system monitor, process status, changelog/news ──────────
export interface LiveStats {
  cpu_name: string
  cpu_usage: number
  gpu_name: string
  gpu_usage: number | null
  ram_used_gb: number
  ram_total_gb: number
  ram_usage: number
}

export interface ProcessStatus {
  fallout4_running: boolean
  mo2_running: boolean
  f4se_running: boolean
}

export interface ChangelogLink {
  label: string
  url: string
}
export interface ChangelogEntry {
  title: string
  notes: string[]
  links: ChangelogLink[]
}
export interface ChangelogSection {
  category: string
  entries: ChangelogEntry[]
}
export interface ChangelogData {
  version: string
  released: string
  sections: ChangelogSection[]
}

export interface YoutubeVideo {
  id: string
  title: string
  published: string
  url: string
  embed_url: string
  thumbnail: string
}

export interface AnomalyUpdate {
  version: string
  description: string
  raw: string
  is_new: boolean
}

export const dashboardApi = {
  liveStats: () => invoke<LiveStats>('get_live_stats'),
  processStatus: () => invoke<ProcessStatus>('get_process_status'),
  changelog: () => invoke<ChangelogData>('get_changelog_data'),
  youtubeVideos: (limit?: number) => invoke<YoutubeVideo[]>('get_youtube_videos', { limit }),
  checkUpdate: () => invoke<AnomalyUpdate>('check_anomaly_update'),
  markUpdateSeen: (version: string) => invoke<void>('mark_update_seen', { version }),
  newsBanner: () => invoke<string>('get_news_banner'),
}

// ── Linux / CLF3 / Fluorine bridge ──────────────────────────────────────────
export interface Clf3Status {
  binary: string | null
  version: string | null
  has_api_key: boolean
  fluorine_binary: string | null
}
export interface ModlistMetadata {
  title: string
  version: string
  download_url: string
  machine_url: string | null
  archives_size: number | null
  installed_size: number | null
}

export const linuxApi = {
  clf3Status: () => invoke<Clf3Status>('clf3_status'),
  clf3HasApiKey: () => invoke<boolean>('clf3_has_api_key'),
  clf3SetApiKey: (key: string) => invoke<void>('clf3_set_api_key', { key }),
  clf3ListGpus: () => invoke<unknown[]>('clf3_list_gpus'),
  clf3CheckUpdates: (name?: string) => invoke<unknown>('clf3_check_updates', { name }),
  clf3Install: (args: {
    wabbajackUrl: string
    downloads: string
    output: string
    nexusKey: string
    autoFluorine: boolean
  }) =>
    invoke<unknown>('clf3_install_modlist', {
      wabbajackUrl: args.wabbajackUrl,
      downloads: args.downloads,
      output: args.output,
      nexusKey: args.nexusKey,
      autoFluorine: args.autoFluorine,
    }),
  fetchModlistMetadata: (url?: string) =>
    invoke<ModlistMetadata>('fetch_modlist_metadata', { url }),
  bootstrapInstall: (args: {
    nexusKey: string
    downloads: string
    output: string
    pinnedWabbajackUrl?: string
    modlistJson?: string
  }) =>
    invoke<unknown>('bootstrap_install', {
      nexusKey: args.nexusKey,
      downloads: args.downloads,
      output: args.output,
      pinnedWabbajackUrl: args.pinnedWabbajackUrl,
      modlistJson: args.modlistJson,
    }),
  fluorineOpen: (installDir: string) => invoke<number>('fluorine_open', { installDir }),
}

// Mod commands — read/write the active MO2 profile's modlist.txt
export interface ModEntry {
  name: string
  enabled: boolean
  is_user: boolean
  is_separator: boolean
}

export const modsApi = {
  list: () => invoke<ModEntry[]>('list_mods'),
  toggle: (name: string, enabled: boolean) =>
    invoke<OperationResult>('toggle_mod', { name, enabled }),
  /** source can be a mod folder or a .zip archive */
  addUserMod: (source: string) =>
    invoke<OperationResult>('add_user_mod', { source }),
  move: (name: string, up: boolean) =>
    invoke<OperationResult>('move_mod', { name, up }),
}

// Preset commands
export const presetsApi = {
  list: (presetType: string) =>
    invoke<OperationResult>('list_presets', { preset_type: presetType }),
  install: (name: string, presetType: string) =>
    invoke<OperationResult>('install_preset', {
      name,
      preset_type: presetType,
    }),
  remove: (name: string, presetType: string) =>
    invoke<OperationResult>('remove_preset', { name, preset_type: presetType }),
  getActive: (presetType: string) =>
    invoke<OperationResult>('get_active_preset', { preset_type: presetType }),
  getPreview: (name: string, presetType: string) =>
    invoke<OperationResult>('get_preset_preview', {
      name,
      preset_type: presetType,
    }),
}

// ENB manager commands
export interface EnbInfo {
  name: string
  has_showcase: boolean
}
export interface EnbStatus {
  default: EnbInfo
  custom: EnbInfo | null
  active: 'default' | 'custom' | 'none'
  deploy_path: string
}
export interface EnbEffect {
  name: string
  enabled: boolean
}
export interface EnbConfig {
  found: boolean
  brightness: number
  gamma: number
  bloom: number
  lens: number
  enable_bloom: boolean
  enable_lens: boolean
  enable_dof: boolean
  enable_ssao: boolean
  effects: EnbEffect[]
  /** All raw ini values keyed by "section|key" (both lowercased). */
  values: Record<string, string>
  /** True when these came from the editable live deploy enbseries.ini. */
  editable: boolean
}
export interface EnbIniChange {
  section: string
  key: string
  value: string
}
export const enbApi = {
  status: () => invoke<EnbStatus>('get_enb_status'),
  applyDefault: () => invoke<OperationResult>('apply_default_enb'),
  installCustom: (sourceDir: string, name: string, showcasePath?: string | null) =>
    invoke<OperationResult>('install_custom_enb', { sourceDir, name, showcasePath: showcasePath ?? null }),
  removeCustom: () => invoke<OperationResult>('remove_custom_enb'),
  disableEnb: () => invoke<OperationResult>('disable_enb'),
  /** Returns a data: URL (or empty string). `which` is 'default' | 'custom'. */
  showcase: (which: 'default' | 'custom') => invoke<string>('get_enb_showcase', { which }),
  /** Parsed enbseries.ini for the preview/editor. `target` is 'live' (editable
   *  deploy copy) or 'source' (read-only bundled default). */
  config: (target: 'live' | 'source') => invoke<EnbConfig>('get_enb_config', { target }),
  /** Write edits back into the live enbseries.ini. */
  saveConfig: (changes: EnbIniChange[]) => invoke<OperationResult>('save_enb_config', { changes }),
}

// Profile commands
export const profilesApi = {
  list: async (): Promise<GameProfile[]> => {
    const result = await invoke<OperationResult>('list_profiles')
    if (result.success) {
      try {
        return JSON.parse(result.message)
      } catch {
        return []
      }
    }
    throw new Error(result.message)
  },
  save: (profile: GameProfile) => invoke<OperationResult>('save_profile', { profile }),
  load: async (name: string): Promise<GameProfile | null> => {
    const result = await invoke<OperationResult>('load_profile', { name })
    if (result.success) {
      try {
        return JSON.parse(result.message)
      } catch {
        return null
      }
    }
    throw new Error(result.message)
  },
  delete: (name: string) => invoke<OperationResult>('delete_profile', { name }),
  activate: (name: string) => invoke<OperationResult>('activate_profile', { name }),
  openFolder: () => invoke<OperationResult>('open_profiles_folder'),
  exists: async (name: string): Promise<boolean> => {
    const result = await invoke<OperationResult>('profile_exists', { name })
    if (result.success) {
      return result.message === 'true'
    }
    throw new Error(result.message)
  },
  rename: (oldName: string, newName: string) =>
    invoke<OperationResult>('rename_profile', { old_name: oldName, new_name: newName }),
  getMetadata: async (
    name: string
  ): Promise<{ created_date: string; last_modified: string; name: string }> => {
    const result = await invoke<OperationResult>('get_profile_metadata', { name })
    if (result.success) {
      try {
        return JSON.parse(result.message)
      } catch {
        throw new Error('Failed to parse metadata')
      }
    }
    throw new Error(result.message)
  },
  backupSaves: () => invoke<OperationResult>('backup_saves'),
}

// INI commands — all commands use path discovery on the backend, no path arg needed
export interface IniFileInfo {
  name: string
  size: number
}
export interface IniChange {
  section: string
  key: string
  value: string
}

export const iniApi = {
  read: () => invoke<IniConfig | null>('get_ini_config'),
  listFiles: () => invoke<IniFileInfo[]>('list_ini_files'),
  readFile: (file: string) => invoke<IniConfig>('read_ini_file', { file }),
  saveChanges: (file: string, changes: IniChange[]) =>
    invoke<OperationResult>('save_ini_changes', { file, changes }),
  applyResolution: (width: number, height: number, upscaler?: string) =>
    invoke<OperationResult>('apply_resolution', { width, height, upscaler }),
  updateValue: (section: string, key: string, value: string) =>
    invoke<OperationResult>('update_ini_value', { section, key, value }),
  applyPreset: (preset: string) => invoke<OperationResult>('apply_preset', { preset }),
  backup: () => invoke<OperationResult>('backup_ini'),
  restore: () => invoke<OperationResult>('restore_ini'),
}

export const debugApi = {
  revealLogFile: (path: string) => invoke<OperationResult>('reveal_log_file', { path }),
}

// Update commands
export const updatesApi = {
  check: () => invoke<OperationResult>('check_updates'),
  update: (modId: string) => invoke<OperationResult>('update_mod', { mod_id: modId }),
  getChangelog: (modId: string) =>
    invoke<OperationResult>('get_changelog', { mod_id: modId }),
  checkLauncher: () => invoke<OperationResult>('check_launcher_update'),
}

// FOMOD commands
export interface FomodStep {
  name: string
  groups: FomodGroup[]
  special_type?: string
}

export interface FomodGroup {
  name: string
  group_type: 'SelectExactlyOne' | 'SelectAny'
  plugins: FomodPlugin[]
}

export interface FomodPlugin {
  name: string
  description: string
  files: any[]
  image?: string | null
}

export interface FomodSelection {
  step_index: number
  group_index: number
  plugin_indices: number[]
}

export const fomodApi = {
  loadConfig: (configPath: string) =>
    invoke<{ steps: FomodStep[] }>('load_fomod_config', { configPath }),
  getResources: (modsFolder: string) =>
    invoke<{ steps: FomodStep[] }>('get_fomod_resources', { modsFolder }),
  checkAvailable: (modsFolder: string) =>
    invoke<boolean>('check_fomod_available', { modsFolder }),
  getImage: (modsFolder: string, imagePath: string) =>
    invoke<string>('get_fomod_image', { modsFolder, imagePath }),
  install: (req: {
    source_folder: string
    output_folder: string
    selections: FomodSelection[]
    confirmed_resolution?: [number, number]
    confirmed_gpu?: string
    upscaler_mode?: string
  }) =>
    // Tauri wraps the JS args object in the Rust parameter name. The Rust
    // command takes `req: InstallFomodRequest`, so we must send { req: ... }
    // here, not the bare request body.
    invoke<OperationResult>('install_fomod_options', { req }),
}

// System info
export interface SystemInfo {
  resolution: [number, number]
  gpu_vendor: string
  display_scale: number
}

export interface GamePaths {
  game_root: string
  mods_folder: string
  mo2_root: string | null
  mo2_profile: string | null
  ini_folder: string
}

export interface PagefileInfo {
  ram_mb: number
  recommended_mb: number
  drives: string[]
  install_drive: string
}

export interface Mo2StartupResult {
  killed_mo2: boolean
  under_mo2: boolean
}

export const systemApi = {
  getInfo: () => invoke<SystemInfo>('get_system_info'),
  getPagefileInfo: () => invoke<PagefileInfo>('get_pagefile_info'),
  configurePagefile: (drive: string, targetMb: number) =>
    invoke<void>('configure_pagefile', { drive, targetMb }),
  checkMsvcInstalled: () => invoke<boolean>('check_msvc_installed'),
  addAntivirusExclusion: () => invoke<OperationResult>('add_antivirus_exclusion'),
  getResolution: () => invoke<[number, number]>('detect_screen_resolution'),
  getGpuVendor: () => invoke<string>('detect_gpu_vendor'),
  checkBlockingProcesses: () => invoke<string[]>('check_blocking_processes'),
  setGamePath: (path: string) => invoke<string>('set_game_path', { path }),
  isGamePathConfigured: () => invoke<boolean>('is_game_path_configured'),
  getConfiguredGamePath: () => invoke<string | null>('get_configured_game_path'),
  discoverGamePaths: () => invoke<GamePaths>('discover_game_paths'),
  checkAndKillMo2: (): Promise<boolean> => invoke('check_and_kill_mo2'),
  mo2Startup: () => invoke<Mo2StartupResult>('mo2_startup'),
}

// First-time setup / onboarding
export interface OnboardingConfig {
  resolution: [number, number] | null
  gpu_vendor: string | null
  upscaler: string | null
  prereqs_acked: boolean[]
  pagefile_acked: boolean
  complete: boolean
}

export const onboardingApi = {
  get: () => invoke<OnboardingConfig>('get_onboarding_config'),
  save: (config: OnboardingConfig) =>
    invoke<OperationResult>('save_onboarding_config', { config }),
  isComplete: () => invoke<boolean>('is_onboarding_complete'),
}

