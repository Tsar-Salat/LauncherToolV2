import { useState, useEffect, useRef } from 'react'
import {
  fomodApi,
  systemApi,
  onboardingApi,
  FomodStep,
  GamePaths,
  OnboardingConfig,
} from '../utils/tauriApi'
import SetupRequired from '../components/SetupRequired'
import '../styles/pages/FomodPage.css'

interface Selection {
  step_index: number
  group_index: number
  plugin_indices: number[]
}

interface PendingUsmSelection {
  stepIdx: number
  groupIdx: number
  pluginIdx: number
}

// Options intentionally hidden from the Optional Mods wizard.
const HIDDEN_PLUGINS = new Set(
  ['e for activate', 'ps5 controller support', 'unleveled world', 'upscaling support']
)

const USM_PLUGIN_NAME = 'unlimited survival mode'

export default function FomodPage() {
  const [steps, setSteps] = useState<FomodStep[]>([])
  const [currentStep, setCurrentStep] = useState(0)
  const [selections, setSelections] = useState<Map<string, number[]>>(new Map())
  const [loading, setLoading] = useState(false)
  const [message, setMessage] = useState('')
  const [paths, setPaths] = useState<GamePaths | null>(null)
  const [pathConfigured, setPathConfigured] = useState<boolean | null>(null)
  const [onboarding, setOnboarding] = useState<OnboardingConfig | null>(null)
  const [images, setImages] = useState<Record<string, string>>({})
  const [installResult, setInstallResult] = useState<{ ok: boolean; text: string } | null>(null)
  const [showUsmWarning, setShowUsmWarning] = useState(false)
  const [pendingUsm, setPendingUsm] = useState<PendingUsmSelection | null>(null)
  const isMountedRef = useRef(true)
  const resultRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    isMountedRef.current = true
    loadFomod()
    return () => {
      isMountedRef.current = false
    }
  }, [])

  const loadFomod = async () => {
    setLoading(true)
    try {
      const configured = await systemApi.isGamePathConfigured()
      if (!isMountedRef.current) return
      setPathConfigured(configured)
      if (!configured) {
        setLoading(false)
        return
      }

      const [gamePaths, savedOnboarding] = await Promise.all([
        systemApi.discoverGamePaths(),
        onboardingApi.get(),
      ])
      if (!isMountedRef.current) return

      setPaths(gamePaths)
      setOnboarding(savedOnboarding)

      const fomodResources = await fomodApi.getResources(gamePaths.mods_folder)
      if (!isMountedRef.current) return

      setSteps(fomodResources.steps)

      // Lazily fetch plugin preview images (base64 data URLs).
      const wanted = new Set<string>()
      for (const step of fomodResources.steps)
        for (const group of step.groups)
          for (const plugin of group.plugins)
            if (plugin.image && !HIDDEN_PLUGINS.has(plugin.name.toLowerCase())) wanted.add(plugin.image)
      wanted.forEach((imgPath) => {
        fomodApi.getImage(gamePaths.mods_folder, imgPath)
          .then((data) => { if (isMountedRef.current) setImages((prev) => ({ ...prev, [imgPath]: data })) })
          .catch(() => {})
      })

      if (fomodResources.steps.length === 0) {
        setMessage(
          `No FOMOD configuration found at ${gamePaths.mods_folder}\\Fallen World FOMOD Resources`
        )
      } else {
        setMessage('')
      }
    } catch (err) {
      if (isMountedRef.current) {
        const msg = err instanceof Error ? err.message : String(err)
        setMessage(`Error loading FOMOD: ${msg}`)
      }
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  const handlePluginSelect = (
    stepIdx: number,
    groupIdx: number,
    pluginIdx: number,
    isChecked: boolean
  ) => {
    const key = `${stepIdx},${groupIdx}`
    const group = steps[stepIdx]?.groups[groupIdx]

    if (group?.group_type === 'SelectExactlyOne') {
      setSelections(new Map(selections).set(key, [pluginIdx]))
    } else {
      const current = selections.get(key) || []
      const updated = isChecked
        ? [...current, pluginIdx].sort((a, b) => a - b)
        : current.filter((i) => i !== pluginIdx)
      const next = new Map(selections)
      if (updated.length === 0) next.delete(key)
      else next.set(key, updated)
      setSelections(next)
    }
  }

  /** Every required SelectExactlyOne group on the current step must be answered. */
  const canAdvance = (): boolean => {
    const step = steps[currentStep]
    if (!step) return false
    return (step.groups || []).every((group, gIdx) => {
      if (group.group_type !== 'SelectExactlyOne') return true
      const picks = selections.get(`${currentStep},${gIdx}`) || []
      return picks.length > 0
    })
  }

  const handleInstall = async () => {
    if (steps.length === 0) {
      setInstallResult({ ok: false, text: 'No FOMOD configuration available.' })
      return
    }
    if (!paths) {
      setInstallResult({ ok: false, text: 'Game paths not loaded.' })
      return
    }
    if (!onboarding || !onboarding.complete) {
      setInstallResult({ ok: false, text: 'First-Time Setup is not complete. Open the SETUP page and finish all steps before installing.' })
      return
    }
    if (!onboarding.resolution || !onboarding.gpu_vendor) {
      setInstallResult({ ok: false, text: 'Setup is missing resolution or GPU. Re-run setup to populate these values.' })
      return
    }

    setInstallResult(null)
    setLoading(true)
    try {
      const selectionList: Selection[] = Array.from(selections.entries()).map(([key, indices]) => {
        const [stepIdx, groupIdx] = key.split(',').map(Number)
        return { step_index: stepIdx, group_index: groupIdx, plugin_indices: indices }
      })

      const sep = paths.mods_folder.includes('\\') ? '\\' : '/'
      const sourceFolder = `${paths.mods_folder}${sep}Fallen World FOMOD Resources`
      const outputFolder = `${paths.mods_folder}${sep}Fallen World Optional Mods`

      console.log('[FOMOD] Installing:', { sourceFolder, outputFolder, selectionCount: selectionList.length })

      const result = await fomodApi.install({
        source_folder: sourceFolder,
        output_folder: outputFolder,
        selections: selectionList,
        confirmed_resolution: onboarding.resolution,
        confirmed_gpu: onboarding.gpu_vendor,
        upscaler_mode: onboarding.upscaler || undefined,
      })

      console.log('[FOMOD] Result:', result)

      if (isMountedRef.current) {
        setInstallResult({
          ok: result.success,
          text: result.success
            ? '✓ Installation completed successfully!'
            : result.message || 'Installation failed with no error message.',
        })
        setTimeout(() => resultRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' }), 50)
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      console.error('[FOMOD] Install error:', err)
      if (isMountedRef.current) {
        setInstallResult({ ok: false, text: `Error: ${msg}` })
        setTimeout(() => resultRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' }), 50)
      }
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  const currentStepData = steps[currentStep]
  const stepName = currentStepData?.name || 'Loading...'
  const isLastStep = currentStep >= steps.length - 1

  if (pathConfigured === false) {
    return <SetupRequired feature="Optional Mods Installer" />
  }

  const setupIncomplete = onboarding !== null && !onboarding.complete

  return (
    <div className="page-content fomod-page">
      <div className="fomod-container">
        <h2>Optional Mods Installer</h2>

        {setupIncomplete && (
          <div
            className="message error"
            style={{ marginBottom: 16 }}
            role="alert"
          >
            First-Time Setup hasn't been completed. Open the SETUP page first so the installer can
            use your confirmed resolution, GPU and upscaler.
          </div>
        )}

        {steps.length === 0 ? (
          <div className="fomod-empty">
            <p>No FOMOD configuration found.</p>
            <p>Ensure "Fallen World FOMOD Resources" is installed in your mods folder.</p>
            <button className="btn btn-primary" onClick={loadFomod} disabled={loading}>
              {loading ? 'Checking...' : 'Check Again'}
            </button>
          </div>
        ) : (
          <>
            <div className="fomod-progress">
              <div className="progress-text">
                Step {currentStep + 1} of {steps.length} — {stepName}
              </div>
              <div className="progress-bar">
                <div
                  className="progress-fill"
                  style={{ width: `${((currentStep + 1) / steps.length) * 100}%` }}
                />
              </div>
            </div>

            <div className="fomod-step">
              <h3>{stepName}</h3>
              {currentStepData?.groups && (
                <div className="fomod-groups">
                  {currentStepData.groups.map((group, groupIdx) => (
                    <div key={`${currentStep}-${groupIdx}`} className="fomod-group">
                      <h4>{group.name}</h4>
                      <p className="group-type">
                        {group.group_type === 'SelectExactlyOne' ? 'Choose one:' : 'Select any:'}
                      </p>

                      <div className="fomod-options">
                        {group.plugins.map((plugin, pluginIdx) => {
                          // Hidden options keep their original index for selection
                          // integrity — we just don't render them.
                          if (HIDDEN_PLUGINS.has(plugin.name.toLowerCase())) return null
                          const thumb = plugin.image ? images[plugin.image] : undefined
                          return (
                          <div
                            key={`${currentStep}-${groupIdx}-${pluginIdx}`}
                            className="fomod-option"
                          >
                            {group.group_type === 'SelectExactlyOne' ? (
                              <label className="radio-option">
                                <input
                                  type="radio"
                                  name={`group-${currentStep}-${groupIdx}`}
                                  checked={
                                    (selections.get(`${currentStep},${groupIdx}`) || [])[0] ===
                                    pluginIdx
                                  }
                                  onChange={() =>
                                    handlePluginSelect(currentStep, groupIdx, pluginIdx, true)
                                  }
                                  disabled={loading}
                                />
                                <span className="radio-label">
                                  {thumb && <img className="fomod-thumb" src={thumb} alt="" />}
                                  <strong>{plugin.name}</strong>
                                  {plugin.description && (
                                    <p className="plugin-desc">{plugin.description}</p>
                                  )}
                                </span>
                              </label>
                            ) : (
                              <label className="checkbox-option">
                                <input
                                  type="checkbox"
                                  checked={(selections.get(`${currentStep},${groupIdx}`) || []).includes(
                                    pluginIdx
                                  )}
                                  onChange={(e) => {
                                    const isUsm = plugin.name.toLowerCase() === USM_PLUGIN_NAME
                                    if (isUsm && e.target.checked) {
                                      setPendingUsm({ stepIdx: currentStep, groupIdx, pluginIdx })
                                      setShowUsmWarning(true)
                                    } else {
                                      handlePluginSelect(currentStep, groupIdx, pluginIdx, e.target.checked)
                                    }
                                  }}
                                  disabled={loading}
                                />
                                <span className="checkbox-label">
                                  {thumb && <img className="fomod-thumb" src={thumb} alt="" />}
                                  <strong>
                                    {plugin.name}
                                    {plugin.name.toLowerCase() === USM_PLUGIN_NAME && (
                                      <span className="plugin-cheat-badge">⚠ CHEAT MOD</span>
                                    )}
                                  </strong>
                                  {plugin.description && (
                                    <p className="plugin-desc">{plugin.description}</p>
                                  )}
                                </span>
                              </label>
                            )}
                          </div>
                          )
                        })}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div className="fomod-nav">
              <button
                className="btn btn-secondary"
                onClick={() => setCurrentStep(Math.max(0, currentStep - 1))}
                disabled={currentStep === 0 || loading}
              >
                Previous
              </button>

              {isLastStep ? (
                <button
                  className="btn btn-primary"
                  onClick={handleInstall}
                  disabled={loading || !canAdvance() || setupIncomplete}
                  title={
                    setupIncomplete
                      ? 'Complete First-Time Setup before installing'
                      : !canAdvance()
                        ? 'Answer the required choices on this step'
                        : ''
                  }
                >
                  {loading ? 'Installing...' : 'Install'}
                </button>
              ) : (
                <button
                  className="btn btn-primary"
                  onClick={() => setCurrentStep(currentStep + 1)}
                  disabled={loading || !canAdvance()}
                  title={!canAdvance() ? 'Answer the required choices on this step' : ''}
                >
                  Next
                </button>
              )}
            </div>
          </>
        )}

        {message && (
          <div className="message error">{message}</div>
        )}

        {installResult && (
          <div
            ref={resultRef}
            className={`message ${installResult.ok ? 'success' : 'error'}`}
            style={{ whiteSpace: 'pre-wrap', marginTop: 8 }}
          >
            {installResult.text}
          </div>
        )}
      </div>

      {/* ── USM cheat-mod confirmation modal ─────────────────────────── */}
      {showUsmWarning && (
        <div className="usm-modal-overlay" onClick={() => { setShowUsmWarning(false); setPendingUsm(null) }}>
          <div className="usm-modal" onClick={(e) => e.stopPropagation()}>
            <div className="usm-modal__icon">⚠</div>
            <h3 className="usm-modal__title">Cheat Mod — Proceed?</h3>
            <p className="usm-modal__body">
              <strong>Unlimited Survival Mode</strong> restores features the game explicitly disables
              in Survival difficulty. Installing it changes how Fallen World plays in ways that are
              not intended by the modlist design.
            </p>
            <ul className="usm-modal__bullets">
              <li>Console (~) — unlocks the debug console</li>
              <li>God Mode (TGM) — invincibility and unlimited ammo</li>
              <li>Fast Travel — removes the core survival restriction</li>
              <li>Vanilla compass — restores all map markers</li>
              <li>Quick Save / Auto Save — removes the permadeath pressure</li>
            </ul>
            <div className="usm-modal__warning">
              This is NOT how Fallen World is designed to be played. Only install if you want
              a relaxed, casual playthrough without survival penalties.
            </div>
            <div className="usm-modal__actions">
              <button
                className="usm-modal__confirm"
                onClick={() => {
                  if (pendingUsm) {
                    handlePluginSelect(pendingUsm.stepIdx, pendingUsm.groupIdx, pendingUsm.pluginIdx, true)
                  }
                  setShowUsmWarning(false)
                  setPendingUsm(null)
                }}
              >
                I Understand — Install Anyway
              </button>
              <button
                className="btn btn-secondary usm-modal__cancel"
                onClick={() => { setShowUsmWarning(false); setPendingUsm(null) }}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
