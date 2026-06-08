// Curated Fallout 4 INI settings catalog for the structured editor.
//
// Each entry maps a friendly control to a real INI section/key. The editor only
// renders an entry when that key actually exists in the selected file, so
// unknown/guessed keys never produce bogus writes — extend freely.

export type ControlType = 'toggle' | 'slider' | 'text' | 'dropdown' | 'color'

export interface IniSetting {
  section: string
  key: string
  label: string
  control: ControlType
  tab: string
  group: string
  min?: number
  max?: number
  step?: number
  options?: { label: string; value: string }[]
  help?: string
}

export const INI_TABS = ['Interface', 'Visuals', 'Basic', 'General', 'Setup', 'View Distance'] as const

/** One-click boolean tweaks shown as toggle buttons in "Quick Tweaks".
 *  Rendered only when the key exists in the selected file. */
export interface QuickToggle {
  section: string
  key: string
  label: string
}

export const QUICK_TWEAKS: QuickToggle[] = [
  { section: 'Display', key: 'iPresentInterval', label: 'VSync' },
  { section: 'Display', key: 'bBorderless', label: 'Borderless' },
  { section: 'Display', key: 'bFull Screen', label: 'Fullscreen' },
  { section: 'Display', key: 'bMaximizeWindow', label: 'Maximize Window' },
  { section: 'Display', key: 'bVolumetricLightingEnable', label: 'God Rays' },
  { section: 'Display', key: 'bSAOEnable', label: 'Ambient Occlusion' },
  { section: 'Display', key: 'bDoDepthOfField', label: 'Depth of Field' },
  { section: 'Display', key: 'bScreenSpaceReflectionEnabled', label: 'SSR' },
  { section: 'Display', key: 'bDeferredShadows', label: 'Deferred Shadows' },
  { section: 'Interface', key: 'bDialogueSubtitles', label: 'Dialogue Subtitles' },
  { section: 'Interface', key: 'bGeneralSubtitles', label: 'General Subtitles' },
  { section: 'Controls', key: 'bGamePadRumble', label: 'Controller Vibration' },
  { section: 'Controls', key: 'bInvertYValues', label: 'Invert Y Axis' },
  { section: 'Controls', key: 'bMouseAcceleration', label: 'Mouse Accel' },
  { section: 'Archive', key: 'bInvalidateOlderFiles', label: 'Loose File Loading' },
  { section: 'General', key: 'bAlwaysActive', label: 'Audio While Alt-Tabbed' },
  { section: 'GamePlay', key: 'bSaveOnPause', label: 'Save On Pause' },
  { section: 'GamePlay', key: 'bSaveOnRest', label: 'Save On Rest' },
  { section: 'GamePlay', key: 'bSaveOnTravel', label: 'Save On Travel' },
  { section: 'GamePlay', key: 'bSaveOnWait', label: 'Save On Wait' },
]

export const INI_CATALOG: IniSetting[] = [
  // ── Interface ──────────────────────────────────────────────────────────
  { tab: 'Interface', group: 'Subtitles & HUD', section: 'Interface', key: 'bDialogueSubtitles', label: 'Dialogue Subtitles', control: 'toggle' },
  { tab: 'Interface', group: 'Subtitles & HUD', section: 'Interface', key: 'bGeneralSubtitles', label: 'General Subtitles', control: 'toggle' },
  { tab: 'Interface', group: 'Subtitles & HUD', section: 'Interface', key: 'fHUDOpacity', label: 'HUD Opacity', control: 'slider', min: 0, max: 1, step: 0.05 },

  { tab: 'Interface', group: 'Mouse', section: 'Controls', key: 'fMouseHeadingSensitivity', label: 'Look Sensitivity', control: 'slider', min: 0, max: 0.1, step: 0.001 },
  { tab: 'Interface', group: 'Mouse', section: 'Controls', key: 'bInvertYValues', label: 'Invert Y Axis', control: 'toggle' },
  { tab: 'Interface', group: 'Mouse', section: 'Controls', key: 'bMouseAcceleration', label: 'Mouse Acceleration', control: 'toggle' },

  { tab: 'Interface', group: 'Controller', section: 'Controls', key: 'bGamePadRumble', label: 'Vibration', control: 'toggle' },
  { tab: 'Interface', group: 'Controller', section: 'Controls', key: 'fGamepadHeadingSensitivity', label: 'Controller Sensitivity', control: 'slider', min: 0, max: 2, step: 0.05 },

  // ── Visuals ────────────────────────────────────────────────────────────
  { tab: 'Visuals', group: 'Quality', section: 'Display', key: 'iMaxAnisotropy', label: 'Anisotropic Filtering', control: 'dropdown',
    options: [{ label: 'Off', value: '0' }, { label: '2x', value: '2' }, { label: '4x', value: '4' }, { label: '8x', value: '8' }, { label: '16x', value: '16' }] },
  { tab: 'Visuals', group: 'Quality', section: 'Display', key: 'iShadowMapResolution', label: 'Shadow Quality', control: 'dropdown',
    options: [{ label: 'Low (1024)', value: '1024' }, { label: 'Medium (2048)', value: '2048' }, { label: 'High (4096)', value: '4096' }] },
  { tab: 'Visuals', group: 'Quality', section: 'Display', key: 'fShadowDistance', label: 'Shadow Distance', control: 'slider', min: 0, max: 20000, step: 500 },
  { tab: 'Visuals', group: 'Quality', section: 'Display', key: 'iPresentInterval', label: 'VSync', control: 'toggle' },
  { tab: 'Visuals', group: 'Quality', section: 'Display', key: 'fGamma', label: 'Gamma', control: 'slider', min: 0.2, max: 2, step: 0.05 },

  { tab: 'Visuals', group: 'Camera (FOV)', section: 'Camera', key: 'fDefaultWorldFOV', label: 'World FOV', control: 'slider', min: 60, max: 120, step: 1 },
  { tab: 'Visuals', group: 'Camera (FOV)', section: 'Camera', key: 'fDefault1stPersonFOV', label: '1st-Person FOV', control: 'slider', min: 60, max: 120, step: 1 },

  // ── Basic ──────────────────────────────────────────────────────────────
  { tab: 'Basic', group: 'Display', section: 'Display', key: 'iSize W', label: 'Resolution Width', control: 'text' },
  { tab: 'Basic', group: 'Display', section: 'Display', key: 'iSize H', label: 'Resolution Height', control: 'text' },
  { tab: 'Basic', group: 'Display', section: 'Display', key: 'bFull Screen', label: 'Fullscreen', control: 'toggle' },
  { tab: 'Basic', group: 'Display', section: 'Display', key: 'bBorderless', label: 'Borderless Window', control: 'toggle' },
  { tab: 'Basic', group: 'Display', section: 'Display', key: 'bMaximizeWindow', label: 'Maximize Window', control: 'toggle' },

  // ── General ────────────────────────────────────────────────────────────
  { tab: 'General', group: 'Application', section: 'General', key: 'bAlwaysActive', label: 'Audio While Alt-Tabbed', control: 'toggle' },
  { tab: 'General', group: 'Saving', section: 'GamePlay', key: 'bSaveOnPause', label: 'Save On Pause', control: 'toggle' },
  { tab: 'General', group: 'Saving', section: 'GamePlay', key: 'bSaveOnRest', label: 'Save On Rest', control: 'toggle' },
  { tab: 'General', group: 'Saving', section: 'GamePlay', key: 'bSaveOnTravel', label: 'Save On Travel', control: 'toggle' },
  { tab: 'General', group: 'Saving', section: 'GamePlay', key: 'bSaveOnWait', label: 'Save On Wait', control: 'toggle' },

  // ── Setup ──────────────────────────────────────────────────────────────
  { tab: 'Setup', group: 'Modding', section: 'Archive', key: 'bInvalidateOlderFiles', label: 'Loose File Loading', control: 'toggle', help: 'Required for many texture/mesh mods' },
  { tab: 'Setup', group: 'Modding', section: 'General', key: 'sLanguage', label: 'Game Language', control: 'text' },

  // ── View Distance ──────────────────────────────────────────────────────
  { tab: 'View Distance', group: 'Object LOD', section: 'Display', key: 'fLODFadeOutMultObjects', label: 'Object Detail', control: 'slider', min: 1, max: 15, step: 0.5 },
  { tab: 'View Distance', group: 'Object LOD', section: 'Display', key: 'fLODFadeOutMultItems', label: 'Item Detail', control: 'slider', min: 1, max: 15, step: 0.5 },
  { tab: 'View Distance', group: 'Object LOD', section: 'Display', key: 'fLODFadeOutMultActors', label: 'Actor Detail', control: 'slider', min: 1, max: 15, step: 0.5 },
  { tab: 'View Distance', group: 'Terrain & Grass', section: 'Grass', key: 'fGrassStartFadeDistance', label: 'Grass Distance', control: 'slider', min: 0, max: 15000, step: 500 },
  { tab: 'View Distance', group: 'Terrain & Grass', section: 'TerrainManager', key: 'fBlockLevel0Distance', label: 'Terrain Detail', control: 'slider', min: 5000, max: 60000, step: 1000 },
]
