import { useState, useEffect, useCallback } from 'react'
import { open } from '@tauri-apps/api/dialog'
import { enbApi, EnbStatus, EnbConfig, EnbIniChange } from '../utils/tauriApi'
import '../styles/pages/PresetsPage.css'

type Which = 'default' | 'custom' | 'none'

interface InstallForm {
  path: string
  name: string
  showcase: string | null
}

interface Field {
  section: string
  key: string
  label: string
  type: 'toggle' | 'slider'
  min?: number
  max?: number
  step?: number
}
interface Group {
  title: string
  fields: Field[]
}

/** Editable enbseries.ini fields. Only those present in the loaded ini are shown. */
const SCHEMA: Group[] = [
  {
    title: 'Color Grade',
    fields: [
      { section: 'COLORCORRECTION', key: 'Brightness', label: 'Brightness', type: 'slider', min: 0.3, max: 2.5, step: 0.01 },
      { section: 'COLORCORRECTION', key: 'GammaCurve', label: 'Gamma', type: 'slider', min: 0.3, max: 2.5, step: 0.01 },
    ],
  },
  {
    title: 'Bloom',
    fields: [
      { section: 'BLOOM', key: 'AmountDay', label: 'Bloom · Day', type: 'slider', min: 0, max: 5, step: 0.05 },
      { section: 'BLOOM', key: 'AmountNight', label: 'Bloom · Night', type: 'slider', min: 0, max: 5, step: 0.05 },
      { section: 'BLOOM', key: 'AmountInteriorDay', label: 'Bloom · Interior', type: 'slider', min: 0, max: 5, step: 0.05 },
    ],
  },
  {
    title: 'Lens',
    fields: [
      { section: 'LENS', key: 'AmountDay', label: 'Lens · Day', type: 'slider', min: 0, max: 5, step: 0.05 },
      { section: 'LENS', key: 'AmountNight', label: 'Lens · Night', type: 'slider', min: 0, max: 5, step: 0.05 },
    ],
  },
  {
    title: 'Effects',
    fields: [
      { section: 'EFFECT', key: 'EnableBloom', label: 'Bloom', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableLens', label: 'Lens', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableDepthOfField', label: 'Depth of Field', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableSSAO', label: 'Ambient Occlusion', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableAdaptation', label: 'Adaptation', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableReflections', label: 'Reflections', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableWater', label: 'Water', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableSubSurfaceScattering', label: 'Subsurface Scattering', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableProceduralSun', label: 'Procedural Sun', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableCloudShadows', label: 'Cloud Shadows', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableDetailedShadow', label: 'Detailed Shadow', type: 'toggle' },
      { section: 'EFFECT', key: 'EnableSkylighting', label: 'Skylighting', type: 'toggle' },
    ],
  },
]

const lk = (section: string, key: string) => `${section.toLowerCase()}|${key.toLowerCase()}`
const num = (s: string | undefined, d: number) => {
  const n = parseFloat(s ?? '')
  return Number.isNaN(n) ? d : n
}
const isOn = (s: string | undefined) => (s ?? '').trim().toLowerCase() === 'true'
const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v))

// Configurator (preview + ini editor) is hidden for now. Flip to true to re-enable
// the "Configure / Preview" buttons and the modal.
const SHOW_CONFIG = false

export default function PresetsPage() {
  const [status, setStatus] = useState<EnbStatus | null>(null)
  const [defaultImg, setDefaultImg] = useState('')
  const [customImg, setCustomImg] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const [install, setInstall] = useState<InstallForm | null>(null)

  // Config editor state
  const [configFor, setConfigFor] = useState<Which | null>(null)
  const [config, setConfig] = useState<EnbConfig | null>(null)
  const [vals, setVals] = useState<Record<string, string>>({})
  const [orig, setOrig] = useState<Record<string, string>>({})
  const [saving, setSaving] = useState(false)

  const refresh = useCallback(async () => {
    try {
      const s = await enbApi.status()
      setStatus(s)
      if (s.default.has_showcase) enbApi.showcase('default').then(setDefaultImg).catch(() => {})
      if (s.custom?.has_showcase) enbApi.showcase('custom').then(setCustomImg).catch(() => setCustomImg(''))
      else setCustomImg('')
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    }
  }, [])

  useEffect(() => { refresh() }, [refresh])

  const run = async (fn: () => Promise<{ success: boolean; message: string }>) => {
    setBusy(true)
    setMessage('')
    try {
      const res = await fn()
      setMessage(res.message)
      await refresh()
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      setBusy(false)
    }
  }

  const startInstall = async () => {
    try {
      const picked = await open({
        multiple: false,
        title: 'Select an ENB .zip archive',
        filters: [{ name: 'ENB archive', extensions: ['zip'] }],
      })
      if (typeof picked === 'string') {
        const base = (picked.split(/[\\/]/).pop() || 'Custom ENB').replace(/\.zip$/i, '')
        setInstall({ path: picked, name: base, showcase: null })
        setMessage('')
      }
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    }
  }

  const pickShowcase = async () => {
    if (!install) return
    try {
      const picked = await open({
        multiple: false,
        title: 'Choose preview image',
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      })
      if (typeof picked === 'string') setInstall({ ...install, showcase: picked })
    } catch { /* cancelled */ }
  }

  const confirmInstall = () => {
    if (!install) return
    const form = install
    setInstall(null)
    run(() => enbApi.installCustom(form.path, form.name.trim() || 'Custom ENB', form.showcase))
  }

  const loadConfig = useCallback(async (which: Which) => {
    const target = which === (status?.active ?? 'default') ? 'live' : 'source'
    const c = await enbApi.config(target)
    setConfig(c)
    const next: Record<string, string> = {}
    if (c.values) {
      for (const g of SCHEMA) for (const f of g.fields) {
        const k = lk(f.section, f.key)
        if (k in c.values) next[k] = c.values[k]
      }
    }
    setVals(next)
    setOrig(next)
  }, [status])

  const openConfig = async (which: Which) => {
    setConfigFor(which)
    setConfig(null)
    setVals({})
    setOrig({})
    try { await loadConfig(which) } catch { setConfig({ found: false } as EnbConfig) }
  }

  const setField = (section: string, key: string, value: string) =>
    setVals((p) => ({ ...p, [lk(section, key)]: value }))

  const dirtyChanges = (): EnbIniChange[] => {
    const out: EnbIniChange[] = []
    for (const g of SCHEMA) for (const f of g.fields) {
      const k = lk(f.section, f.key)
      if (k in vals && vals[k] !== orig[k]) out.push({ section: f.section, key: f.key, value: vals[k] })
    }
    return out
  }

  const saveConfig = async () => {
    const changes = dirtyChanges()
    if (changes.length === 0) return
    setSaving(true)
    setMessage('')
    try {
      const res = await enbApi.saveConfig(changes)
      setMessage(res.message)
      if (res.success) setOrig({ ...vals })
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      setSaving(false)
    }
  }

  const active = status?.active ?? 'default'
  const configImg = configFor === 'custom' ? customImg : defaultImg
  const editable = !!config?.editable
  const dirty = dirtyChanges().length

  // Live preview values derived from edited fields.
  const brightness = num(vals[lk('COLORCORRECTION', 'Brightness')], 1)
  const gamma = num(vals[lk('COLORCORRECTION', 'GammaCurve')], 1)
  const bloomOn = isOn(vals[lk('EFFECT', 'EnableBloom')])
  const bloomAmt = bloomOn ? num(vals[lk('BLOOM', 'AmountDay')], 1) : 0
  const ssaoOn = isOn(vals[lk('EFFECT', 'EnableSSAO')])
  const lensOn = isOn(vals[lk('EFFECT', 'EnableLens')])
  const previewFilter =
    `brightness(${(brightness * (1 + bloomAmt * 0.05)).toFixed(3)}) ` +
    `contrast(${(1 + (gamma - 1) * 0.5).toFixed(3)}) saturate(1)`
  const bloomOpacity = clamp(bloomAmt * 0.16, 0, 0.65)

  return (
    <div className="page-content enb-page">
      <div className="enb-head panel">
        <div>
          <h2 className="enb-head__title">ENB Manager</h2>
          <p className="enb-head__sub">
            Swap your visual preset. The default ENB is protected — it can't be edited or removed.
          </p>
        </div>
        <button className="btn btn-secondary" onClick={refresh} disabled={busy}>↻ Refresh</button>
      </div>

      {/* ── Disabled banner ── */}
      {active === 'none' && (
        <div className="message" style={{ marginBottom: 12, background: 'rgba(255,180,0,0.12)', borderColor: 'rgba(255,180,0,0.4)', color: 'var(--text-muted)' }}>
          ⚠ ENB is currently <strong>disabled</strong>. ENB files have been removed from the Optional Mods folder.
          Activate the Default ENB or install a Custom ENB to re-enable visuals.
        </div>
      )}

      <div className="enb-grid">
        {/* ── Default ENB ── */}
        <div className={`enb-card panel ${active === 'default' ? 'is-active' : ''}`}>
          <div className="enb-card__shot">
            {defaultImg
              ? <img src={defaultImg} alt={status?.default.name ?? 'Default ENB'} />
              : <div className="enb-card__noshot">No preview</div>}
            <span className="enb-tag enb-tag--lock">🔒 Protected</span>
            {active === 'default' && <span className="enb-tag enb-tag--active">● Active</span>}
          </div>
          <div className="enb-card__body">
            <div className="enb-card__row">
              <h3 className="enb-card__name">{status?.default.name ?? 'Default ENB'}</h3>
              <span className="enb-card__kind">Default</span>
            </div>
            <p className="enb-card__note">The bundled Fallen World ENB. Cannot be edited or deleted.</p>
            <div className="enb-card__actions">
              {SHOW_CONFIG && (
                <button className="btn btn-secondary" onClick={() => openConfig('default')} disabled={busy}>
                  Configure / Preview
                </button>
              )}
              {active !== 'none' && (
                <button
                  className="btn btn-secondary"
                  onClick={() => run(() => enbApi.disableEnb())}
                  disabled={busy}
                  title="Remove ENB files from the Optional Mods folder"
                >
                  Disable ENB
                </button>
              )}
              <button
                className="btn btn-primary"
                onClick={() => run(() => enbApi.removeCustom())}
                disabled={busy || active === 'default'}
              >
                {active === 'default' ? 'Currently Active' : 'Enable Default ENB'}
              </button>
            </div>
          </div>
        </div>

        {/* ── Custom ENB (only present while active) ── */}
        {status?.custom && active === 'custom' ? (
          <div className="enb-card panel is-active">
            <div className="enb-card__shot">
              {customImg
                ? <img src={customImg} alt={status.custom.name} />
                : <div className="enb-card__noshot">No preview</div>}
              <span className="enb-tag enb-tag--active">● Active</span>
            </div>
            <div className="enb-card__body">
              <div className="enb-card__row">
                <h3 className="enb-card__name">{status.custom.name}</h3>
                <span className="enb-card__kind enb-card__kind--custom">Custom</span>
              </div>
              <p className="enb-card__note">Your installed ENB. The protected enblocal.ini is untouched.</p>
              <div className="enb-card__actions">
                {SHOW_CONFIG && (
                  <button className="btn btn-secondary" onClick={() => openConfig('custom')} disabled={busy}>
                    Configure / Preview
                  </button>
                )}
                <button className="btn btn-secondary" onClick={startInstall} disabled={busy}>Replace…</button>
                <button className="btn btn-secondary" onClick={() => run(() => enbApi.removeCustom())} disabled={busy}>
                  Remove
                </button>
              </div>
            </div>
          </div>
        ) : (
          <div className="enb-card enb-card--empty panel">
            <div className="enb-empty">
              <div className="enb-empty__icon">✦</div>
              <h3>Install Your Custom ENB</h3>
              <p>Pick the ENB <code>.zip</code> archive. Add a preview picture if you like.</p>
              <button className="btn btn-primary" onClick={startInstall} disabled={busy}>Install ENB</button>
            </div>
          </div>
        )}
      </div>

      {/* ── Install modal ── */}
      {install && (
        <div className="enb-modal-overlay" onClick={() => setInstall(null)}>
          <div className="enb-modal panel" onClick={(e) => e.stopPropagation()}>
            <h3>Install Custom ENB</h3>
            <p className="enb-modal__path">{install.path}</p>
            <label className="enb-modal__field">
              <span>Display name</span>
              <input value={install.name} onChange={(e) => setInstall({ ...install, name: e.target.value })} autoFocus />
            </label>
            <div className="enb-modal__field">
              <span>Preview image (optional)</span>
              <div className="enb-modal__pic">
                <button className="btn btn-secondary" onClick={pickShowcase}>
                  {install.showcase ? 'Change Image…' : 'Add Image…'}
                </button>
                {install.showcase && (
                  <span className="enb-modal__picname" title={install.showcase}>
                    {install.showcase.split(/[\\/]/).pop()}
                    <button className="enb-modal__picclear" onClick={() => setInstall({ ...install, showcase: null })}>✕</button>
                  </span>
                )}
              </div>
            </div>
            <p className="enb-modal__warn">
              ⚠ This clears the previous ENB's files. Your <code>enblocal.ini</code> and ENB binaries
              (<code>d3d11.dll</code>, <code>d3dcompiler_46e.dll</code>) are preserved; a bundled
              <code> enblocal.ini</code> is ignored.
            </p>
            <div className="enb-modal__actions">
              <button className="btn btn-secondary" onClick={() => setInstall(null)}>Cancel</button>
              <button className="btn btn-primary" onClick={confirmInstall}>Install ENB</button>
            </div>
          </div>
        </div>
      )}

      {/* ── Config / editor modal (hidden for now via SHOW_CONFIG) ── */}
      {SHOW_CONFIG && configFor && (
        <div className="enb-modal-overlay" onClick={() => setConfigFor(null)}>
          <div className="enb-modal enb-cfg panel" onClick={(e) => e.stopPropagation()}>
            <div className="enb-cfg__head">
              <h3>{configFor === 'custom' ? status?.custom?.name : status?.default.name} · Configurator</h3>
              <button className="enb-cfg__close" onClick={() => setConfigFor(null)}>✕</button>
            </div>

            <div className="enb-cfg__body">
              {/* preview */}
              <div className="enb-cfg__previewcol">
                <div className="enb-cfg__stage">
                  {configImg ? (
                    <>
                      <img src={configImg} alt="preview" style={{ filter: previewFilter }} />
                      {ssaoOn && <div className="enb-fx enb-fx--ssao" />}
                      {bloomAmt > 0 && <div className="enb-fx enb-fx--bloom" style={{ opacity: bloomOpacity }} />}
                      {lensOn && <div className="enb-fx enb-fx--lens" />}
                    </>
                  ) : (
                    <div className="enb-card__noshot">No preview image for this ENB</div>
                  )}
                </div>
                <p className="enb-cfg__disclaimer">
                  ⚠ This configurator approximates the in-game look as closely as we can, but actual
                  results vary by monitor, calibration, HDR and in-game lighting. Use it as a guide,
                  then fine-tune in-game.
                </p>
              </div>

              {/* editor */}
              <div className="enb-cfg__editcol">
                {!config && <p className="enb-cfg__hint">Loading…</p>}
                {config && !config.found && <p className="enb-cfg__missing">No enbseries.ini found for this ENB.</p>}

                {config?.found && !editable && (
                  <p className="enb-cfg__readonly">Read-only preview. Activate this ENB to edit and save its settings.</p>
                )}

                {config?.found && SCHEMA.map((g) => {
                  const present = g.fields.filter((f) => lk(f.section, f.key) in vals)
                  if (present.length === 0) return null
                  return (
                    <div key={g.title} className="enb-cfg__group">
                      <h4 className="enb-cfg__grouptitle">{g.title}</h4>
                      {g.title === 'Effects' ? (
                        <div className="enb-cfg__toggles">
                          {present.map((f) => {
                            const k = lk(f.section, f.key)
                            const on = isOn(vals[k])
                            return (
                              <button
                                key={k}
                                className={`enb-toggle ${on ? 'on' : 'off'}`}
                                disabled={!editable}
                                onClick={() => setField(f.section, f.key, on ? 'false' : 'true')}
                              >
                                {on ? '●' : '○'} {f.label}
                              </button>
                            )
                          })}
                        </div>
                      ) : (
                        present.map((f) => {
                          const k = lk(f.section, f.key)
                          const v = num(vals[k], f.min ?? 0)
                          return (
                            <label key={k} className="enb-slider">
                              <span className="enb-slider__label">{f.label}</span>
                              <input
                                type="range" min={f.min} max={f.max} step={f.step}
                                value={v}
                                disabled={!editable}
                                onChange={(e) => setField(f.section, f.key, e.target.value)}
                              />
                              <span className="enb-slider__val">{v.toFixed(2)}</span>
                            </label>
                          )
                        })
                      )}
                    </div>
                  )
                })}
              </div>
            </div>

            {config?.found && editable && (
              <div className="enb-cfg__savebar">
                <span className="enb-cfg__dirty">{dirty > 0 ? `${dirty} unsaved change${dirty > 1 ? 's' : ''}` : 'No changes'}</span>
                <div className="enb-cfg__saveactions">
                  <button className="btn btn-secondary" onClick={() => setVals({ ...orig })} disabled={dirty === 0 || saving}>Revert</button>
                  <button className="btn btn-primary" onClick={saveConfig} disabled={dirty === 0 || saving}>
                    {saving ? 'Saving…' : 'Save to enbseries.ini'}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {busy && <div className="enb-busy">Working…</div>}
      {message && (
        <div className={`message ${message.toLowerCase().includes('error') || message.toLowerCase().includes('fail') ? 'error' : 'success'}`}>
          {message}
        </div>
      )}
    </div>
  )
}
