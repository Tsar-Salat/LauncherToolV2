import { useState, useEffect, useRef } from 'react'
import { systemApi } from '../utils/tauriApi'
import { useI18n, LangCode } from '../i18n'
import { useTheme } from '../theme'
import type { PageName } from '../components/Sidebar'
import '../styles/pages/SettingsPage.css'

interface SettingsPageProps {
  onNavigate?: (page: PageName) => void
  onShowTutorial?: () => void
}

const DEFAULT_SETTINGS = {
  gameFolder: '',
  modsFolder: '',
  autoBackup: true,
  checkUpdatesOnStart: true,
  closeOnLaunch: true,
}

export default function SettingsPage({ onNavigate, onShowTutorial }: SettingsPageProps) {
  const { t, lang, setLang, languages } = useI18n()
  const { theme, setTheme, themes } = useTheme()
  const [settings, setSettings] = useState(DEFAULT_SETTINGS)
  const [message, setMessage] = useState('')
  const [loading, setLoading] = useState(false)
  const [configuredPath, setConfiguredPath] = useState<string | null>(null)
  const isMountedRef = useRef(true)

  useEffect(() => {
    isMountedRef.current = true
    loadSettings()
    // Load the currently configured backend path
    systemApi.getConfiguredGamePath().then((path) => {
      if (!isMountedRef.current) return
      setConfiguredPath(path)
      // If backend has a path but localStorage doesn't, sync it back
      if (path) {
        setSettings((prev) => prev.gameFolder ? prev : { ...prev, gameFolder: path })
      }
    }).catch(() => {})
    return () => {
      isMountedRef.current = false
    }
  }, [])

  const loadSettings = () => {
    try {
      const saved = localStorage.getItem('appSettings')
      if (saved) {
        const parsed = JSON.parse(saved)
        if (isMountedRef.current) {
          setSettings({ ...DEFAULT_SETTINGS, ...parsed })
        }
      }
    } catch (err) {
      if (isMountedRef.current) {
        const msg = err instanceof Error ? err.message : String(err)
        setMessage(`Error loading settings: ${msg}`)
        setSettings(DEFAULT_SETTINGS)
      }
    }
  }

  const handleChange = (key: string, value: any) => {
    const updated = { ...settings, [key]: value }
    setSettings(updated)
  }

  const handleSave = () => {
    try {
      localStorage.setItem('appSettings', JSON.stringify(settings))
      if (isMountedRef.current) {
        setMessage('Settings saved successfully')
        setTimeout(() => {
          if (isMountedRef.current) {
            setMessage('')
          }
        }, 3000)
      }
    } catch (err) {
      if (isMountedRef.current) {
        const msg = err instanceof Error ? err.message : String(err)
        setMessage(`Error saving settings: ${msg}`)
      }
    }
  }

  const handleReset = () => {
    if (confirm('Reset all settings to defaults?')) {
      try {
        setSettings(DEFAULT_SETTINGS)
        localStorage.setItem('appSettings', JSON.stringify(DEFAULT_SETTINGS))
        if (isMountedRef.current) {
          setMessage('Settings reset to defaults')
          setTimeout(() => {
            if (isMountedRef.current) {
              setMessage('')
            }
          }, 3000)
        }
      } catch (err) {
        if (isMountedRef.current) {
          const msg = err instanceof Error ? err.message : String(err)
          setMessage(`Error resetting settings: ${msg}`)
        }
      }
    }
  }

  const handleSetGamePath = async () => {
    if (!settings.gameFolder.trim()) {
      setMessage('Please enter a game folder path')
      return
    }

    setLoading(true)
    try {
      const result = await systemApi.setGamePath(settings.gameFolder.trim())
      if (isMountedRef.current) {
        setMessage(result)
        setConfiguredPath(settings.gameFolder.trim())
        // Also persist to localStorage so it stays after a reload
        try {
          const updated = { ...settings, gameFolder: settings.gameFolder.trim() }
          localStorage.setItem('appSettings', JSON.stringify(updated))
        } catch {}
        setTimeout(() => {
          if (isMountedRef.current) {
            setMessage('')
          }
        }, 3000)
      }
    } catch (err) {
      if (isMountedRef.current) {
        const msg = err instanceof Error ? err.message : String(err)
        setMessage(`Error: ${msg}`)
      }
    } finally {
      if (isMountedRef.current) {
        setLoading(false)
      }
    }
  }

  return (
    <div className="page-content settings-page">
      <div className="settings-grid">

        {/* ── Left column ── */}
        <div className="settings-col">
          <section className="settings-section">
            <h3>{t('settings.gamePaths')}</h3>
            <div className="setting-item">
              <label>{t('settings.gameFolder')}</label>
              <div className="setting-row">
                <input
                  type="text"
                  value={settings.gameFolder}
                  onChange={(e) => handleChange('gameFolder', e.target.value)}
                  placeholder="e.g., E:\Games\Fallen World  (any parent folder works)"
                  className="setting-input"
                />
                <button
                  className="btn btn-small"
                  onClick={handleSetGamePath}
                  disabled={loading}
                >
                  {loading ? '…' : t('settings.set')}
                </button>
              </div>
              <p className="setting-hint">
                Pick any folder that contains or is a parent of your Fallout4.exe — the launcher
                will locate it automatically (searches up to 3 levels deep).
              </p>
              <div className={`setting-status ${configuredPath ? 'ok' : 'err'}`}>
                {configuredPath ? `✓ Fallout4.exe found in: ${configuredPath}` : '✗ Not configured'}
              </div>
            </div>
            <div className="setting-item">
              <label>{t('settings.modsFolder')}</label>
              <input
                type="text"
                value={settings.modsFolder}
                onChange={(e) => handleChange('modsFolder', e.target.value)}
                placeholder={t('settings.autoDetected')}
                className="setting-input"
                disabled
              />
              <small className="setting-hint">{t('settings.autoDetectedFrom')}</small>
            </div>
          </section>

          <section className="settings-section">
            <h3>{t('settings.preferences')}</h3>
            <div className="setting-item checkbox">
              <input
                type="checkbox"
                id="autoBackup"
                checked={settings.autoBackup}
                onChange={(e) => handleChange('autoBackup', e.target.checked)}
              />
              <label htmlFor="autoBackup">{t('settings.backupIni')}</label>
            </div>
            <div className="setting-item checkbox">
              <input
                type="checkbox"
                id="checkUpdates"
                checked={settings.checkUpdatesOnStart}
                onChange={(e) => handleChange('checkUpdatesOnStart', e.target.checked)}
              />
              <label htmlFor="checkUpdates">{t('settings.checkUpdates')}</label>
            </div>
            <div className="setting-item checkbox">
              <input
                type="checkbox"
                id="closeOnLaunch"
                checked={settings.closeOnLaunch}
                onChange={(e) => handleChange('closeOnLaunch', e.target.checked)}
              />
              <label htmlFor="closeOnLaunch">{t('settings.closeOnLaunch')}</label>
            </div>
          </section>

          <section className="settings-section">
            <h3>{t('settings.setup')}</h3>
            <div className="setting-item">
              <button className="btn btn-small" onClick={() => onNavigate?.('setup')}>
                ↻ {t('settings.rerunSetup')}
              </button>
              <small className="setting-hint">{t('settings.rerunSetupHint')}</small>
            </div>
            {onShowTutorial && (
              <div className="setting-item">
                <button className="btn btn-small" onClick={onShowTutorial}>
                  ◉ {t('settings.showTutorial')}
                </button>
                <small className="setting-hint">{t('settings.showTutorialHint')}</small>
              </div>
            )}
          </section>
        </div>

        {/* ── Right column ── */}
        <div className="settings-col">
          <section className="settings-section">
            <h3>{t('settings.language')}</h3>
            <div className="setting-item">
              <label htmlFor="language">{t('settings.language')}</label>
              <select
                id="language"
                value={lang}
                onChange={(e) => setLang(e.target.value as LangCode)}
                className="setting-select"
              >
                {Object.entries(languages).map(([code, name]) => (
                  <option key={code} value={code}>{name}</option>
                ))}
              </select>
            </div>
          </section>

          <section className="settings-section">
            <h3>{t('settings.appearance')}</h3>
            <div className="theme-swatches">
              {themes.map((th) => (
                <button
                  key={th.id}
                  type="button"
                  className={`theme-swatch ${theme === th.id ? 'active' : ''}`}
                  onClick={() => setTheme(th.id)}
                  title={t(`theme.${th.id}`)}
                >
                  <span className="theme-swatch__chip" style={{ background: th.swatch[1] }}>
                    <span style={{ background: th.swatch[0] }} />
                  </span>
                  <span className="theme-swatch__name">{t(`theme.${th.id}`)}</span>
                </button>
              ))}
            </div>
          </section>

          <section className="settings-section info">
            <h3>{t('settings.about')}</h3>
            <div className="info-item">
              <span>Fallen World Launcher</span>
              <span className="version">v0.1.0</span>
            </div>
            <div className="info-item">
              <span>{t('settings.build')}</span>
              <span>Rust + Tauri + React</span>
            </div>
          </section>
        </div>
      </div>

      <div className="settings-footer">
        <div className="settings-actions">
          <button className="btn btn-save" onClick={handleSave}>{t('settings.save')}</button>
          <button className="btn btn-reset" onClick={handleReset}>{t('settings.reset')}</button>
        </div>
        {message && (
          <div className={`message ${message.includes('Error') || message.includes('error') ? 'error' : 'success'}`}>
            {message}
          </div>
        )}
      </div>
    </div>
  )
}
