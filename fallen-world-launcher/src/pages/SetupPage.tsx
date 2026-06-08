import { useState, useEffect, useRef } from 'react'
import { open } from '@tauri-apps/api/dialog'
import { onboardingApi, systemApi, gameApi, iniApi, OnboardingConfig, SystemInfo, PagefileInfo, OperationResult } from '../utils/tauriApi'
import { useI18n, LangCode } from '../i18n'
import RobCoTerminal from './RobCoTerminal'
import '../styles/pages/SetupPage.css'

const TERMINAL_KEY = 'fwl.terminal.shown'

type GpuVendor = 'NVIDIA' | 'AMD' | 'Intel'
type Upscaler = 'DLAA' | 'DLSS'

interface ResolutionOption {
  label: string
  value: [number, number]
}

const RESOLUTION_PRESETS: ResolutionOption[] = [
  { label: '1920 x 1080  (16:9 Full HD)', value: [1920, 1080] },
  { label: '2560 x 1440  (16:9 QHD)', value: [2560, 1440] },
  { label: '3840 x 2160  (16:9 4K)', value: [3840, 2160] },
  { label: '2560 x 1080  (21:9 Ultrawide)', value: [2560, 1080] },
  { label: '3440 x 1440  (21:9 Ultrawide)', value: [3440, 1440] },
  { label: '3840 x 1600  (24:10 Ultrawide)', value: [3840, 1600] },
  { label: '5120 x 1440  (32:9 Super Ultrawide)', value: [5120, 1440] },
  { label: '5120 x 2160  (32:9 Super Ultrawide 4K)', value: [5120, 2160] },
]

const GPU_OPTIONS: GpuVendor[] = ['NVIDIA', 'AMD', 'Intel']

const PREREQ_ITEMS = [
  'Visual C++ Redistributable 2015-2022 (x64) installed',
  'Fallout 4 language set to English',
  'All DLCs installed (excluding High-Res Texture Pack)',
  'Steam Overlay disabled for Fallout 4',
  'Anti-virus exclusions added for the modlist folder',
]

const STEP_LABELS = [
  'Game Location',
  'Display Resolution',
  'GPU & Upscaler',
  'System Prerequisites',
  'Pagefile',
  'Review & Save',
] as const

function normaliseGpuVendor(raw: string): GpuVendor | null {
  const lower = raw.toLowerCase()
  if (lower.includes('nvidia')) return 'NVIDIA'
  if (lower.includes('amd') || lower.includes('radeon')) return 'AMD'
  if (lower.includes('intel')) return 'Intel'
  return null
}

function aspectNote(res: [number, number] | null): string {
  if (!res) return ''
  const r = res[0] / res[1]
  if (r > 3.0) return ' — Super Ultrawide patches will be applied'
  if (r > 2.0) return ' — Ultrawide patches will be applied'
  return ''
}

interface SetupPageProps {
  /** Called after setup is saved successfully (App shows the tutorial). */
  onComplete?: () => void
}

export default function SetupPage({ onComplete }: SetupPageProps = {}) {
  const { t, lang, setLang, languages } = useI18n()
  // Show the RobCo terminal once per browser session, then never again.
  const [terminalDone, setTerminalDone] = useState<boolean>(
    () => sessionStorage.getItem(TERMINAL_KEY) === '1'
  )

  const [stepIdx, setStepIdx] = useState(0)
  const [loading, setLoading] = useState(false)
  const [message, setMessage] = useState('')

  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null)
  const [gameDir, setGameDir] = useState<string | null>(null)
  const [pathMsg, setPathMsg] = useState('')
  const [resolution, setResolution] = useState<[number, number] | null>(null)
  const [gpu, setGpu] = useState<GpuVendor | null>(null)
  const [upscaler, setUpscaler] = useState<Upscaler | null>('DLAA')
  const [prereqAcks, setPrereqAcks] = useState<boolean[]>(() => PREREQ_ITEMS.map(() => false))
  const [pagefileAcked, setPagefileAcked] = useState(false)
  const [alreadyComplete, setAlreadyComplete] = useState(false)

  const [pagefile, setPagefile] = useState<PagefileInfo | null>(null)
  const [pfDrive, setPfDrive] = useState('')
  const [pfApplying, setPfApplying] = useState(false)
  const [pfMessage, setPfMessage] = useState('')
  const [msvcInstalled, setMsvcInstalled] = useState<boolean | null>(null)
  const [avResult, setAvResult] = useState<OperationResult | null>(null)
  const [avLoading, setAvLoading] = useState(false)

  const isMountedRef = useRef(true)

  useEffect(() => {
    isMountedRef.current = true
    bootstrap()
    return () => {
      isMountedRef.current = false
    }
  }, [])

  const bootstrap = async () => {
    setLoading(true)
    try {
      const [info, saved] = await Promise.all([
        systemApi.getInfo(),
        onboardingApi.get(),
      ])
      if (!isMountedRef.current) return

      setSystemInfo(info)
      setAlreadyComplete(saved.complete)

      // Prefill the game folder if it's already configured or auto-detectable.
      systemApi.getConfiguredGamePath().then((p) => {
        if (isMountedRef.current && p) setGameDir(p)
      }).catch(() => {})

      // Prefer saved values, fall back to detection. This lets the user
      // re-open setup and edit prior selections instead of starting fresh.
      setResolution(saved.resolution ?? info.resolution)
      setGpu(
        (saved.gpu_vendor as GpuVendor | null) ?? normaliseGpuVendor(info.gpu_vendor)
      )
      if (saved.upscaler === 'DLAA' || saved.upscaler === 'DLSS') {
        setUpscaler(saved.upscaler)
      }
      if (saved.prereqs_acked.length === PREREQ_ITEMS.length) {
        setPrereqAcks(saved.prereqs_acked)
      }
      setPagefileAcked(saved.pagefile_acked)

      // Best-effort: load pagefile info for the pagefile step. Re-loaded after the
      // game folder is set, since the recommended drive is derived from it.
      loadPagefile()

      // Detect MSVC redistributable for the prereqs step.
      systemApi.checkMsvcInstalled().then((installed) => {
        if (!isMountedRef.current) return
        setMsvcInstalled(installed)
        // Auto-check the MSVC prereq item (index 0) if it's already installed.
        if (installed) {
          setPrereqAcks((prev) => {
            const next = [...prev]
            next[0] = true
            return next
          })
        }
      }).catch(() => {})
    } catch (err) {
      if (isMountedRef.current) {
        const msg = err instanceof Error ? err.message : String(err)
        setMessage(`Error loading setup: ${msg}`)
      }
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  // Pagefile info — its recommended drive is derived from the configured game
  // path, so this must run after the game folder is set (not just at bootstrap).
  const loadPagefile = () => {
    systemApi.getPagefileInfo().then((pf) => {
      if (!isMountedRef.current) return
      setPagefile(pf)
      setPfDrive(pf.install_drive || pf.drives[0] || 'C:')
    }).catch(() => {})
  }

  const pickGameDir = async () => {
    try {
      const picked = await open({
        directory: true,
        multiple: false,
        title: 'Select your Fallen World installation folder',
      })
      if (typeof picked !== 'string') return
      try {
        await systemApi.setGamePath(picked)
        if (!isMountedRef.current) return
        setGameDir(picked)
        setPathMsg('✓ Fallen World folder set.')
        // Recommended pagefile drive comes from this path — refresh it now.
        loadPagefile()
      } catch (err) {
        if (isMountedRef.current) {
          setPathMsg(`Invalid folder: ${err instanceof Error ? err.message : String(err)}`)
        }
      }
    } catch (err) {
      setPathMsg(`Error: ${err instanceof Error ? err.message : String(err)}`)
    }
  }

  const canAdvance = (): boolean => {
    switch (stepIdx) {
      case 0:
        return !!gameDir
      case 1:
        return resolution !== null
      case 2:
        return gpu !== null
      case 3:
        return prereqAcks.every(Boolean)
      case 4:
        return pagefileAcked
      case 5:
        return true
      default:
        return false
    }
  }

  const handleSave = async () => {
    setLoading(true)
    try {
      const config: OnboardingConfig = {
        resolution,
        gpu_vendor: gpu,
        upscaler,
        prereqs_acked: prereqAcks,
        pagefile_acked: pagefileAcked,
        complete: true,
      }
      const result = await onboardingApi.save(config)
      if (!isMountedRef.current) return
      if (result.success) {
        // Queue the tutorial on every save (first-run and re-run) so the
        // user always sees it after completing or updating their setup.
        localStorage.setItem('fwl.tutorial.pending', '1')
        setAlreadyComplete(true)
        // Push the chosen resolution into the profile INIs so the game and the
        // INI editor's Basic tab reflect it (talks back to MO2).
        let extra = ''
        if (resolution) {
          try {
            const r = await iniApi.applyResolution(resolution[0], resolution[1], upscaler ?? undefined)
            if (!r.success) extra = ` (resolution not applied: ${r.message})`
          } catch {
            extra = ' (resolution not applied)'
          }
        }
        setMessage(`✓ Setup saved${extra}`)
        // Setup is done — let the app fire the tutorial now.
        onComplete?.()
      } else {
        setMessage(`Error: ${result.message}`)
      }
    } catch (err) {
      if (isMountedRef.current) {
        const msg = err instanceof Error ? err.message : String(err)
        setMessage(`Error saving setup: ${msg}`)
      }
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  const handleAddAvExclusion = async () => {
    setAvLoading(true)
    setAvResult(null)
    try {
      const result = await systemApi.addAntivirusExclusion()
      if (isMountedRef.current) setAvResult(result)
    } catch (err) {
      if (isMountedRef.current)
        setAvResult({ success: false, message: err instanceof Error ? err.message : String(err) })
    } finally {
      if (isMountedRef.current) setAvLoading(false)
    }
  }

  const applyPagefile = async () => {
    if (!pagefile || !pfDrive) return
    setPfApplying(true)
    setPfMessage('')
    try {
      await systemApi.configurePagefile(pfDrive, pagefile.recommended_mb)
      if (!isMountedRef.current) return
      setPfMessage(`✓ Pagefile set to ${pagefile.recommended_mb} MB on ${pfDrive}. Restart Windows to apply.`)
      setPagefileAcked(true)
    } catch (err) {
      if (isMountedRef.current) {
        setPfMessage(`Could not set pagefile: ${err instanceof Error ? err.message : String(err)}`)
      }
    } finally {
      if (isMountedRef.current) setPfApplying(false)
    }
  }

  const isLastStep = stepIdx === STEP_LABELS.length - 1

  if (!terminalDone) {
    return (
      <RobCoTerminal
        onComplete={() => {
          sessionStorage.setItem(TERMINAL_KEY, '1')
          setTerminalDone(true)
        }}
      />
    )
  }

  return (
    <div className="page-content setup-page">
      <div className="setup-container">
        <div className="setup-header">
          <h2>{alreadyComplete ? 'Re-run Setup' : 'First-Time Setup'}</h2>
          <label className="setup-lang">
            <span>{t('settings.language')}</span>
            <select value={lang} onChange={(e) => setLang(e.target.value as LangCode)}>
              {Object.entries(languages).map(([code, name]) => (
                <option key={code} value={code}>{name}</option>
              ))}
            </select>
          </label>
        </div>

        <div className="setup-progress">
          <div className="setup-progress-text">
            Step {stepIdx + 1} of {STEP_LABELS.length} — {STEP_LABELS[stepIdx]}
          </div>
          <div className="setup-progress-bar">
            <div
              className="setup-progress-fill"
              style={{ width: `${((stepIdx + 1) / STEP_LABELS.length) * 100}%` }}
            />
          </div>
        </div>

        <div className="setup-step">
          {stepIdx === 0 && (
            <>
              <h3>Game Location</h3>
              <p>
                Point the launcher at your <strong>Fallen World</strong> install — the folder
                containing <code>Fallout4.exe</code>. This is required so resolution, INI edits,
                and mod installs target the right files.
              </p>
              <div className="setup-options">
                <div className="radio-option" style={{ cursor: 'default' }}>
                  <span className="radio-label">
                    <strong>{gameDir ? 'Configured path' : 'Not set'}</strong>
                    <p className="plugin-desc" style={{ wordBreak: 'break-all' }}>
                      {gameDir || 'No Fallen World folder selected yet.'}
                    </p>
                  </span>
                </div>
              </div>
              <div style={{ display: 'flex', gap: 10, marginTop: 12, flexWrap: 'wrap' }}>
                <button className="btn btn-primary" onClick={pickGameDir} disabled={loading}>
                  {gameDir ? 'Change Folder…' : 'Select Fallen World Folder…'}
                </button>
              </div>
              {pathMsg && (
                <div
                  className={`message ${/error|invalid|cannot/i.test(pathMsg) ? 'error' : 'success'}`}
                  style={{ marginTop: 10 }}
                >
                  {pathMsg}
                </div>
              )}
            </>
          )}

          {stepIdx === 1 && (
            <>
              <h3>Display Resolution</h3>
              <p>
                Detected:{' '}
                <strong>
                  {systemInfo
                    ? `${systemInfo.resolution[0]} x ${systemInfo.resolution[1]}`
                    : 'detecting…'}
                </strong>
              </p>
              <p>Confirm or override the resolution Fallen World should target.</p>
              <div className="setup-options">
                {RESOLUTION_PRESETS.map((opt) => {
                  const checked =
                    resolution !== null &&
                    resolution[0] === opt.value[0] &&
                    resolution[1] === opt.value[1]
                  return (
                    <label key={opt.label} className="radio-option">
                      <input
                        type="radio"
                        name="resolution"
                        checked={checked}
                        onChange={() => setResolution(opt.value)}
                      />
                      <span className="radio-label">
                        <strong>{opt.label}</strong>
                      </span>
                    </label>
                  )
                })}
              </div>
              {resolution && (
                <p className="text-muted">
                  Selected: {resolution[0]} x {resolution[1]}
                  {aspectNote(resolution)}
                </p>
              )}
            </>
          )}

          {stepIdx === 2 && (
            <>
              <h3>GPU Vendor & Upscaler</h3>
              <p>
                Detected: <strong>{systemInfo?.gpu_vendor || 'detecting…'}</strong>
              </p>
              <p>Confirm your GPU vendor so the correct upscaler files are installed.</p>
              <div className="setup-options">
                {GPU_OPTIONS.map((v) => (
                  <label key={v} className="radio-option">
                    <input
                      type="radio"
                      name="gpu"
                      checked={gpu === v}
                      onChange={() => setGpu(v)}
                    />
                    <span className="radio-label">
                      <strong>{v}</strong>
                    </span>
                  </label>
                ))}
              </div>

              {gpu && (
                <div className="setup-subgroup">
                  <h4>Upscaler</h4>
                  <p className="text-muted">
                    Choose one, or click the selected option again to clear it. DLAA keeps ENB
                    support; DLSS is higher performance but is incompatible with ENB and requires
                    an NVIDIA GPU.
                  </p>
                  <div className="setup-options">
                    {(['DLAA', 'DLSS'] as const).map((u) => (
                      <label key={u} className="radio-option">
                        <input
                          type="radio"
                          name="upscaler"
                          checked={upscaler === u}
                          onClick={() => setUpscaler(upscaler === u ? null : u)}
                          onChange={() => {}}
                          disabled={u === 'DLSS' && gpu !== 'NVIDIA'}
                        />
                        <span className="radio-label">
                          <strong>{u}</strong>
                          <p className="plugin-desc">
                            {u === 'DLAA'
                              ? 'Quality — anti-aliasing only, ENB-compatible'
                              : gpu !== 'NVIDIA'
                                ? 'Requires an NVIDIA GPU'
                                : 'Performance — no ENB'}
                          </p>
                        </span>
                      </label>
                    ))}
                  </div>
                </div>
              )}
            </>
          )}

          {stepIdx === 3 && (
            <>
              <h3>System Prerequisites</h3>
              <p>Tick each item to confirm before proceeding.</p>
              <div className="setup-options">
                {PREREQ_ITEMS.map((item, i) => {
                  const isMsvc = i === 0
                  const msvcMissing = isMsvc && msvcInstalled === false
                  return (
                    <label
                      key={item}
                      className={`checkbox-option ${msvcMissing ? 'prereq-blocked' : ''}`}
                    >
                      <input
                        type="checkbox"
                        checked={prereqAcks[i]}
                        disabled={msvcMissing}
                        onChange={(e) => {
                          const next = [...prereqAcks]
                          next[i] = e.target.checked
                          setPrereqAcks(next)
                        }}
                      />
                      <span className="checkbox-label">
                        <strong>{item}</strong>
                        {isMsvc && msvcInstalled === true && (
                          <span className="prereq-ok"> ✓ Detected</span>
                        )}
                        {msvcMissing && (
                          <span className="prereq-missing">
                            {' '}⚠ Not detected —{' '}
                            <button
                              type="button"
                              className="prereq-link"
                              onClick={() =>
                                gameApi.openExternal(
                                  'https://aka.ms/vs/17/release/vc_redist.x64.exe'
                                ).catch(() => {})
                              }
                            >
                              Download VC++ Redist x64
                            </button>
                            {' '}then re-open Setup.
                          </span>
                        )}
                      </span>
                    </label>
                  )
                })}
              </div>

              {/* Antivirus exclusion panel */}
              <div className="av-exclusion-panel">
                <div className="av-exclusion-panel__header">
                  ⚠ Anti-Virus Exclusion Required
                </div>
                <p className="av-exclusion-panel__desc">
                  Windows Defender can quarantine mod files and cause crashes. Add an exclusion
                  for your modlist folder to prevent this.
                </p>
                <div className="av-exclusion-panel__steps">
                  <span>Manual steps (Windows Defender):</span>
                  <ol>
                    <li>Scroll down to <strong>Exclusions</strong></li>
                    <li>Click <strong>Add or remove exclusions</strong></li>
                    <li>Click <strong>+ Add an exclusion → Folder</strong> and paste your modlist path</li>
                    <li>Add process exclusions for <code>ModOrganizer.exe</code> and <code>f4se_loader.exe</code></li>
                  </ol>
                </div>
                <div className="av-exclusion-panel__actions">
                  <button
                    className="btn btn-primary"
                    onClick={handleAddAvExclusion}
                    disabled={avLoading}
                  >
                    {avLoading ? 'Adding…' : '🛡 Auto-Add Exclusion (Admin)'}
                  </button>
                  <button
                    className="btn btn-secondary"
                    onClick={() => gameApi.openExternal('windowsdefender://threatsettings/').catch(() => {})}
                  >
                    Open Windows Defender
                  </button>
                </div>
                {avResult && (
                  <div className={`message ${avResult.success ? 'success' : 'error'}`} style={{ marginTop: 10 }}>
                    {avResult.message}
                  </div>
                )}
              </div>
            </>
          )}

          {stepIdx === 4 && (
            <>
              <h3>Pagefile Configuration</h3>
              <p>
                Fallout 4 needs a large Windows pagefile (virtual memory) to stay stable and
                avoid engine crashes during big cell loads. The launcher can set this for you.
              </p>

              {pagefile ? (
                <>
                  <div className="setup-review" style={{ marginBottom: 12 }}>
                    <div className="setup-review-item">
                      <span className="label">System RAM:</span>
                      <span className="value">{(pagefile.ram_mb / 1024).toFixed(1)} GB</span>
                    </div>
                    <div className="setup-review-item">
                      <span className="label">Recommended:</span>
                      <span className="value">{pagefile.recommended_mb} MB (1.5× RAM)</span>
                    </div>
                  </div>

                  <h4>Target Drive</h4>
                  <div className="setup-options">
                    {pagefile.drives.map((d) => (
                      <label key={d} className="radio-option">
                        <input type="radio" name="pfdrive" checked={pfDrive === d} onChange={() => setPfDrive(d)} />
                        <span className="radio-label">
                          <strong>Drive {d}</strong>
                          {d === pagefile.install_drive && <p className="plugin-desc">Recommended — modlist install drive</p>}
                        </span>
                      </label>
                    ))}
                  </div>

                  <div style={{ display: 'flex', gap: 10, alignItems: 'center', marginTop: 14, flexWrap: 'wrap' }}>
                    <button className="btn btn-primary" onClick={applyPagefile} disabled={pfApplying || !pfDrive}>
                      {pfApplying ? 'Applying…' : `Apply ${pagefile.recommended_mb} MB to ${pfDrive}`}
                    </button>
                    <span className="text-muted">Requires Administrator (a UAC prompt will appear).</span>
                  </div>
                  {pfMessage && (
                    <div className={`message ${pfMessage.startsWith('✓') ? 'success' : 'error'}`} style={{ marginTop: 12 }}>
                      {pfMessage}
                    </div>
                  )}

                  {pfMessage && !pfMessage.startsWith('✓') && (
                    <div className="pagefile-manual-guide">
                      <div className="pagefile-manual-guide__title">▸ Manual Method</div>
                      <p>If the automatic fix is blocked (UAC denied, policy restriction), set the pagefile manually:</p>
                      <ol>
                        <li>Press <strong>Win + R</strong>, type <code>sysdm.cpl</code>, press Enter</li>
                        <li>Click the <strong>Advanced</strong> tab → under Performance click <strong>Settings</strong></li>
                        <li>In Performance Options click the <strong>Advanced</strong> tab → under Virtual Memory click <strong>Change</strong></li>
                        <li>Uncheck <strong>Automatically manage paging file size for all drives</strong></li>
                        <li>Select your game drive (<strong>{pfDrive || 'C:'}</strong>), choose <strong>Custom size</strong></li>
                        <li>Set Initial size and Maximum size both to <strong>{pagefile?.recommended_mb ?? 16384} MB</strong></li>
                        <li>Click <strong>Set</strong> → <strong>OK</strong> → restart Windows to apply</li>
                      </ol>
                      <p style={{ marginTop: 10, marginBottom: 0 }}>
                        Once done, tick the acknowledgement below to continue.
                      </p>
                    </div>
                  )}
                </>
              ) : (
                <p className="text-muted">Reading system memory…</p>
              )}

              <label className="checkbox-option" style={{ marginTop: 16 }}>
                <input
                  type="checkbox"
                  checked={pagefileAcked}
                  onChange={(e) => setPagefileAcked(e.target.checked)}
                />
                <span className="checkbox-label">
                  <strong>Pagefile applied or already configured to my liking.</strong>
                </span>
              </label>
            </>
          )}

          {stepIdx === 5 && (
            <>
              <h3>Review & Save</h3>
              <p>Confirm your selections — they feed downstream features like the FOMOD installer.</p>
              <div className="setup-review">
                <div className="setup-review-item">
                  <span className="label">Resolution:</span>
                  <span className="value">
                    {resolution ? `${resolution[0]} x ${resolution[1]}` : '—'}
                  </span>
                </div>
                <div className="setup-review-item">
                  <span className="label">GPU vendor:</span>
                  <span className="value">{gpu || '—'}</span>
                </div>
                <div className="setup-review-item">
                  <span className="label">Upscaler:</span>
                  <span className="value">{upscaler ?? 'None'}</span>
                </div>
                <div className="setup-review-item">
                  <span className="label">Prerequisites:</span>
                  <span className="value">
                    {prereqAcks.filter(Boolean).length} / {PREREQ_ITEMS.length} confirmed
                  </span>
                </div>
                <div className="setup-review-item">
                  <span className="label">Pagefile reviewed:</span>
                  <span className="value">{pagefileAcked ? 'Yes' : 'No'}</span>
                </div>
              </div>
            </>
          )}
        </div>

        <div className="setup-nav">
          <button
            className="btn btn-secondary"
            onClick={() => setStepIdx(Math.max(0, stepIdx - 1))}
            disabled={stepIdx === 0 || loading}
          >
            Previous
          </button>

          {isLastStep ? (
            <button
              className="btn btn-primary"
              onClick={handleSave}
              disabled={loading}
              title="Persist these selections to disk"
            >
              {loading ? 'Saving…' : 'Save Setup'}
            </button>
          ) : (
            <button
              className="btn btn-primary"
              onClick={() => setStepIdx(stepIdx + 1)}
              disabled={loading || !canAdvance()}
              title={!canAdvance() ? 'Complete this step to continue' : ''}
            >
              Next
            </button>
          )}
        </div>

        {message && (
          <div className={`message ${message.includes('Error') ? 'error' : 'success'}`}>
            {message}
          </div>
        )}
      </div>
    </div>
  )
}
