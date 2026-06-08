import { useState, useEffect, useRef, useMemo } from 'react'
import { iniApi, systemApi, IniConfig, IniFileInfo, IniChange } from '../utils/tauriApi'
import { INI_CATALOG, INI_TABS, IniSetting, QUICK_TWEAKS } from '../data/iniCatalog'
import '../styles/pages/IniEditorPage.css'

type Mode = 'structured' | 'quick' | 'raw'
const PRESETS = ['Performance', 'Graphics', 'Ultra']

// ── Raw mode sub-tabs ────────────────────────────────────────────────────
const RAW_SUBTABS = ['Display', 'Gameplay', 'General', 'Interface', 'Audio', 'Other'] as const
type RawSubTab = typeof RAW_SUBTABS[number]

function categorizeSections(section: string): RawSubTab {
  const s = section.toLowerCase()
  if (
    s === 'display' || s === 'lod' || s === 'water' || s === 'waterdepthblur' ||
    s === 'grashopper' || s === 'grassland' || s === 'landscape' || s === 'terrain' ||
    s === 'terrainmanager' || s === 'decals' || s === 'firedecals' || s === 'particles' ||
    s === 'lighting' || s === 'sky' || s === 'lens' || s === 'speedtreeleaf' ||
    s === 'trees' || s === 'weather' || s === 'facegen' || s === 'geometry' ||
    s === 'motionblur' || s === 'texturemapping' || s === 'generatedmeshes' ||
    s.startsWith('blur') || s.startsWith('imagespace') || s.startsWith('lightingshader')
  ) return 'Display'
  if (
    s === 'gameplay' || s === 'combat' || s === 'animation' || s === 'chargen' ||
    s === 'npc destination' || s === 'detectionevent' ||
    s.startsWith('actor') || s.startsWith('character')
  ) return 'Gameplay'
  if (
    s === 'general' || s === 'main' || s === 'archive' || s === 'papyrus' ||
    s === 'backgroundload' || s === 'saves' || s === 'strings'
  ) return 'General'
  if (
    s === 'interface' || s === 'controls' || s === 'launcher' || s === 'menu' ||
    s === 'mail' || s === 'projection' || s === 'pipboy' || s === 'pipboycolorpreset'
  ) return 'Interface'
  if (
    s === 'audio' || s === 'voice' || s === 'music' || s === 'sound' ||
    s === 'footstep' || s === 'audiomenu'
  ) return 'Audio'
  return 'Other'
}

/** Case-insensitive view over an IniConfig for lookups by section+key. */
function buildIndex(config: IniConfig | null) {
  const map = new Map<string, { section: string; key: string; value: string }>()
  if (!config) return map
  for (const [section, entries] of Object.entries(config.sections)) {
    for (const [key, value] of Object.entries(entries)) {
      map.set(`${section.toLowerCase()}|${key.toLowerCase()}`, { section, key, value })
    }
  }
  return map
}

export default function IniEditorPage() {
  const [files, setFiles] = useState<IniFileInfo[]>([])
  const [file, setFile] = useState('')
  const [config, setConfig] = useState<IniConfig | null>(null)
  const [working, setWorking] = useState<Record<string, string>>({}) // "section|key" -> value
  const [mode, setMode] = useState<Mode>('structured')
  const [activeTab, setActiveTab] = useState<string>(INI_TABS[0])
  const [filter, setFilter] = useState('')
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [rawSubTab, setRawSubTab] = useState<RawSubTab>('Display')
  const [loading, setLoading] = useState(false)
  const [message, setMessage] = useState('')
  const [activeProfile, setActiveProfile] = useState<string | null>(null)
  const mounted = useRef(true)

  const index = useMemo(() => buildIndex(config), [config])
  const dirtyCount = Object.keys(working).length

  useEffect(() => {
    mounted.current = true
    ;(async () => {
      try {
        // Fetch active profile name alongside INI files so the toolbar can show
        // which profile's INIs are being edited.
        const [list, paths] = await Promise.all([
          iniApi.listFiles(),
          systemApi.discoverGamePaths().catch(() => null),
        ])
        if (!mounted.current) return
        if (paths?.mo2_profile) setActiveProfile(paths.mo2_profile)
        setFiles(list)
        const preferred = list.find((f) => f.name === 'Fallout4Prefs.ini') ?? list[0]
        if (preferred) {
          setFile(preferred.name)
          await loadFile(preferred.name)
        } else {
          setMessage('No Fallout 4 INI files found. Set your game path in Settings.')
        }
      } catch (err) {
        if (mounted.current) setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
      }
    })()
    return () => { mounted.current = false }
  }, [])

  const loadFile = async (name: string) => {
    setLoading(true)
    try {
      const cfg = await iniApi.readFile(name)
      if (mounted.current) {
        setConfig(cfg)
        setWorking({})
        setExpanded(new Set())
      }
    } catch (err) {
      if (mounted.current) setMessage(`Error reading ${name}: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (mounted.current) setLoading(false)
    }
  }

  const changeFile = async (name: string) => {
    setFile(name)
    await loadFile(name)
  }

  // ── value access ──────────────────────────────────────────────────────
  const current = (section: string, key: string): string | undefined => {
    const wk = `${section}|${key}`
    if (wk in working) return working[wk]
    return index.get(`${section.toLowerCase()}|${key.toLowerCase()}`)?.value
  }
  const exists = (section: string, key: string) =>
    index.has(`${section.toLowerCase()}|${key.toLowerCase()}`)
  const setValue = (section: string, key: string, value: string) =>
    setWorking((prev) => ({ ...prev, [`${section}|${key}`]: value }))

  const save = async () => {
    const changes: IniChange[] = Object.entries(working).map(([k, value]) => {
      const [section, key] = k.split('|')
      return { section, key, value }
    })
    if (changes.length === 0) { setMessage('No changes to save'); return }
    setLoading(true)
    try {
      const res = await iniApi.saveChanges(file, changes)
      if (mounted.current) {
        setMessage(res.message)
        if (res.success) await loadFile(file)
      }
    } catch (err) {
      if (mounted.current) setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (mounted.current) setLoading(false)
    }
  }

  const applyPreset = async (preset: string) => {
    setLoading(true)
    try {
      const r = await iniApi.applyPreset(preset)
      if (!mounted.current) return
      setMessage(r.message)
      if (r.success) await loadFile(file)
    } catch (e) {
      if (mounted.current) setMessage(String(e))
    } finally {
      if (mounted.current) setLoading(false)
    }
  }

  const backup = async () => {
    try { const r = await iniApi.backup(); setMessage(r.message) } catch (e) { setMessage(String(e)) }
  }
  const restore = async () => {
    if (!confirm('Restore Fallout4.ini from backup?')) return
    try { const r = await iniApi.restore(); setMessage(r.message); if (r.success) await loadFile(file) }
    catch (e) { setMessage(String(e)) }
  }

  // ── structured: which settings are present in this file ──────────────────
  const presentByTab = useMemo(() => {
    const map: Record<string, IniSetting[]> = {}
    for (const s of INI_CATALOG) {
      if (exists(s.section, s.key)) (map[s.tab] ??= []).push(s)
    }
    return map
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [index])

  const availableTabs: string[] = INI_TABS.filter((t) => (presentByTab[t]?.length ?? 0) > 0)
  const effectiveTab = availableTabs.includes(activeTab) ? activeTab : availableTabs[0]

  const groupsFor = (tab: string): Record<string, IniSetting[]> => {
    const groups: Record<string, IniSetting[]> = {}
    for (const s of presentByTab[tab] ?? []) (groups[s.group] ??= []).push(s)
    return groups
  }

  // ── render a single control ──────────────────────────────────────────
  const renderControl = (s: IniSetting) => {
    const raw = current(s.section, s.key) ?? ''
    const dirty = `${s.section}|${s.key}` in working
    const cls = `ini-setting ${dirty ? 'is-dirty' : ''}`
    if (s.control === 'toggle') {
      const on = raw === '1' || raw.toLowerCase() === 'true'
      return (
        <label key={s.key} className={`${cls} ini-setting--toggle`}>
          <input type="checkbox" checked={on} onChange={(e) => setValue(s.section, s.key, e.target.checked ? '1' : '0')} />
          <span className="ini-setting__label">{s.label}</span>
        </label>
      )
    }
    return (
      <div key={s.key} className={cls}>
        <span className="ini-setting__label" title={s.help}>{s.label}</span>
        {s.control === 'slider' && (
          <span className="ini-setting__slider">
            <input
              type="range" min={s.min} max={s.max} step={s.step}
              value={parseFloat(raw) || s.min || 0}
              onChange={(e) => setValue(s.section, s.key, e.target.value)}
            />
            <span className="ini-setting__num">{raw}</span>
          </span>
        )}
        {s.control === 'dropdown' && (
          <select value={raw} onChange={(e) => setValue(s.section, s.key, e.target.value)}>
            {s.options?.some((o) => o.value === raw) ? null : <option value={raw}>{raw || '—'}</option>}
            {s.options?.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
          </select>
        )}
        {(s.control === 'text' || s.control === 'color') && (
          <input type="text" value={raw} onChange={(e) => setValue(s.section, s.key, e.target.value)} />
        )}
      </div>
    )
  }

  // ── quick (raw) viewer ───────────────────────────────────────────────
  const filterLower = filter.toLowerCase()
  const rawSections = config?.sections
    ? Object.entries(config.sections)
        .map(([section, values]) => {
          if (!filterLower) return { section, values }
          if (section.toLowerCase().includes(filterLower)) return { section, values }
          const fv = Object.fromEntries(
            Object.entries(values).filter(([k, v]) =>
              k.toLowerCase().includes(filterLower) || v.toLowerCase().includes(filterLower))
          )
          return Object.keys(fv).length ? { section, values: fv } : null
        })
        .filter(Boolean) as { section: string; values: Record<string, string> }[]
    : []

  const toggleSection = (s: string) =>
    setExpanded((prev) => { const n = new Set(prev); n.has(s) ? n.delete(s) : n.add(s); return n })

  // Count sections per sub-tab so we can disable empty ones and auto-select the
  // first populated tab (no "All" tab — it rendered every section at once and
  // made the panel churn/squish).
  const subTabCounts = RAW_SUBTABS.reduce(
    (acc, t) => ({ ...acc, [t]: 0 }),
    {} as Record<RawSubTab, number>,
  )
  for (const { section } of rawSections) subTabCounts[categorizeSections(section)]++
  const availableSubTabs = RAW_SUBTABS.filter((t) => subTabCounts[t] > 0)
  const effectiveSubTab = availableSubTabs.includes(rawSubTab)
    ? rawSubTab
    : (availableSubTabs[0] ?? rawSubTab)
  const displayedSections = rawSections.filter(
    ({ section }) => categorizeSections(section) === effectiveSubTab,
  )

  return (
    <div className="page-content ini-editor-page">
      {/* Toolbar */}
      <div className="ini-toolbar panel">
        {activeProfile && (
          <div className="ini-profile-badge">
            ◈ Profile: <strong>{activeProfile}</strong>
          </div>
        )}
        <div className="ini-toolbar__row">
          <label className="ini-file-select">
            <span>INI File</span>
            <select value={file} onChange={(e) => changeFile(e.target.value)} disabled={loading || !files.length}>
              {files.map((f) => (
                <option key={f.name} value={f.name}>{f.name} ({(f.size / 1024).toFixed(1)} KB)</option>
              ))}
            </select>
          </label>
          <div className="ini-toolbar__actions">
            <button className="btn btn-secondary" onClick={() => loadFile(file)} disabled={loading || !file}>↻ Reload</button>
            <button className="btn btn-primary" onClick={save} disabled={loading || dirtyCount === 0}>
              ⬇ Save{dirtyCount ? ` (${dirtyCount})` : ''}
            </button>
            <button className="btn btn-secondary" onClick={() => setWorking({})} disabled={dirtyCount === 0}>↺ Reset</button>
            <button className="btn btn-secondary" onClick={backup} disabled={loading}>Backup</button>
            <button className="btn btn-secondary" onClick={restore} disabled={loading}>Restore</button>
          </div>
        </div>
        <div className="ini-mode">
          <button className={`ini-mode__btn ${mode === 'structured' ? 'active' : ''}`} onClick={() => setMode('structured')}>Structured</button>
          <button className={`ini-mode__btn ${mode === 'quick' ? 'active' : ''}`} onClick={() => setMode('quick')}>Quick Tweaks</button>
          <button className={`ini-mode__btn ${mode === 'raw' ? 'active' : ''}`} onClick={() => setMode('raw')}>Raw</button>
        </div>
      </div>

      {mode === 'structured' && (
        <div className="ini-structured panel">
          {availableTabs.length === 0 ? (
            <p className="loading">No catalog settings found in {file || 'this file'}. Try “Quick Tweaks” to browse all keys.</p>
          ) : (
            <>
              <div className="ini-tabs">
                {availableTabs.map((tab) => (
                  <button key={tab} className={`ini-tab ${effectiveTab === tab ? 'active' : ''}`} onClick={() => setActiveTab(tab)}>
                    {tab}
                  </button>
                ))}
              </div>
              <div className="ini-groups">
                {Object.entries(groupsFor(effectiveTab)).map(([group, settings]) => (
                  <div key={group} className="ini-group">
                    <h4 className="ini-group__title">{group}</h4>
                    <div className="ini-group__body">
                      {settings.map(renderControl)}
                    </div>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      )}

      {mode === 'quick' && (
        <div className="ini-quick panel">
          <div className="ini-quick__section">
            <h4 className="ini-group__title">Quick Toggles</h4>
            <div className="quick-tweaks">
              {QUICK_TWEAKS.filter((q) => exists(q.section, q.key)).map((q) => {
                const raw = current(q.section, q.key) ?? '0'
                const on = raw === '1' || raw.toLowerCase() === 'true'
                const dirty = `${q.section}|${q.key}` in working
                return (
                  <button
                    key={`${q.section}-${q.key}`}
                    className={`tweak-btn ${on ? 'on' : 'off'} ${dirty ? 'is-dirty' : ''}`}
                    onClick={() => setValue(q.section, q.key, on ? '0' : '1')}
                  >
                    <span className="tweak-btn__state">{on ? 'ON' : 'OFF'}</span>
                    <span className="tweak-btn__label">{q.label}</span>
                  </button>
                )
              })}
              {QUICK_TWEAKS.filter((q) => exists(q.section, q.key)).length === 0 && (
                <p className="loading">No quick toggles available in {file}.</p>
              )}
            </div>
          </div>
          <div className="ini-quick__section">
            <h4 className="ini-group__title">Optimization Presets</h4>
            <div className="quick-presets">
              {PRESETS.map((p) => (
                <button key={p} className="btn btn-secondary" onClick={() => applyPreset(p)} disabled={loading}>
                  {p}
                </button>
              ))}
            </div>
            <p className="ini-quick__hint">Presets apply tuned values immediately and reload the file.</p>
          </div>
        </div>
      )}

      {mode === 'raw' && (
        <div className="ini-viewer panel">
          <div className="ini-viewer-header">
            <h3>RAW · {file}{config && <span className="ini-stats">{Object.keys(config.sections).length} sections</span>}</h3>
            <input className="ini-filter" placeholder="Filter sections / keys / values…" value={filter} onChange={(e) => setFilter(e.target.value)} />
          </div>

          <div className="raw-subtabs">
            {RAW_SUBTABS.map((tab) => {
              const count = subTabCounts[tab]
              return (
                <button
                  key={tab}
                  className={`raw-subtab ${effectiveSubTab === tab ? 'active' : ''}`}
                  onClick={() => setRawSubTab(tab)}
                  disabled={count === 0}
                >
                  {tab}
                  {count > 0 && <span className="raw-subtab__count">{count}</span>}
                </button>
              )
            })}
          </div>

          <div className="sections">
            {displayedSections.map(({ section, values }) => {
              const open = expanded.has(section) || filterLower !== ''
              return (
                <div key={section} className={`section ${open ? 'open' : ''}`}>
                  <button className="section-header" onClick={() => toggleSection(section)}>
                    <span className="section-chevron">{open ? '▾' : '▸'}</span>
                    <span className="section-name">[{section}]</span>
                    <span className="section-count">{Object.keys(values).length}</span>
                  </button>
                  {open && (
                    <div className="section-values">
                      {Object.entries(values).map(([key]) => {
                        const dirty = `${section}|${key}` in working
                        return (
                          <div key={`${section}-${key}`} className={`value-item ${dirty ? 'is-dirty' : ''}`}>
                            <span className="value-key" title={key}>{key}</span>
                            <span className="value-sep">=</span>
                            <input
                              className="value-input"
                              spellCheck={false}
                              value={current(section, key) ?? ''}
                              onChange={(e) => setValue(section, key, e.target.value)}
                            />
                          </div>
                        )
                      })}
                    </div>
                  )}
                </div>
              )
            })}
          </div>
        </div>
      )}

      {message && (
        <div className={`message ${message.toLowerCase().includes('error') ? 'error' : 'success'}`}>{message}</div>
      )}
    </div>
  )
}
