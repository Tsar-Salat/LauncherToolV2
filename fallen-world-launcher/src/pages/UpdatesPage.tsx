import { useState, useRef, useEffect } from 'react'
import { updatesApi, systemApi, gameApi } from '../utils/tauriApi'
import SetupRequired from '../components/SetupRequired'
import '../styles/pages/UpdatesPage.css'

interface ModUpdate {
  mod_id: string
  current_version: string
  available_version: string
  changelog: string
}

export default function UpdatesPage() {
  const [updates, setUpdates] = useState<ModUpdate[]>([])
  const [loading, setLoading] = useState(false)
  const [message, setMessage] = useState('')
  const [selectedMod, setSelectedMod] = useState<string | null>(null)
  const [lastChecked, setLastChecked] = useState<string>('')
  const [launcherUpdate, setLauncherUpdate] = useState<string | null>(null)
  const [updating, setUpdating] = useState<string | null>(null)
  const [pathConfigured, setPathConfigured] = useState<boolean | null>(null)
  const isMountedRef = useRef(true)

  useEffect(() => {
    isMountedRef.current = true
    systemApi.isGamePathConfigured().then((configured) => {
      if (!isMountedRef.current) return
      setPathConfigured(configured)
      if (configured) checkForUpdates()
    }).catch(() => {
      if (isMountedRef.current) setPathConfigured(false)
    })
    return () => {
      isMountedRef.current = false
    }
  }, [])

  const checkForUpdates = async () => {
    setLoading(true)
    try {
      const result = await updatesApi.check()
      if (isMountedRef.current) {
        if (result.success) {
          try {
            const updateList = JSON.parse(result.message)
            setUpdates(updateList || [])
            setLastChecked(new Date().toLocaleString())
            setMessage(
              updateList.length > 0
                ? `Found ${updateList.length} available update(s)`
                : 'All mods are up to date!'
            )
          } catch {
            setUpdates([])
            setLastChecked(new Date().toLocaleString())
            setMessage('All mods are up to date!')
          }
        } else {
          setMessage(`Error checking updates: ${result.message}`)
        }
      }

      // Check for launcher updates
      const launcherResult = await updatesApi.checkLauncher()
      if (isMountedRef.current && launcherResult.success && launcherResult.message) {
        setLauncherUpdate(launcherResult.message)
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

  const handleUpdateMod = async (modId: string) => {
    setUpdating(modId)
    try {
      const result = await updatesApi.update(modId)
      if (isMountedRef.current) {
        if (result.success) {
          setMessage(`Successfully updated: ${modId}`)
          setUpdates(updates.filter((u) => u.mod_id !== modId))
        } else {
          setMessage(`Error updating mod: ${result.message}`)
        }
      }
    } catch (err) {
      if (isMountedRef.current) {
        const msg = err instanceof Error ? err.message : String(err)
        setMessage(`Error: ${msg}`)
      }
    } finally {
      if (isMountedRef.current) {
        setUpdating(null)
      }
    }
  }

  const getSelectedMod = () => updates.find((u) => u.mod_id === selectedMod)

  if (pathConfigured === false) {
    return <SetupRequired feature="Updates Manager" />
  }

  return (
    <div className="page-content updates-page">
      <div className="updates-header">
        <h2>Updates Manager</h2>
        <button className="btn btn-check" onClick={checkForUpdates} disabled={loading}>
          {loading ? 'Checking...' : 'Check for Updates'}
        </button>
      </div>

      {lastChecked && <div className="last-checked">Last checked: {lastChecked}</div>}

      {launcherUpdate && (
        <div className="launcher-update-banner">
          <div className="banner-content">
            <span>New launcher version available: {launcherUpdate}</span>
            <button
              className="btn btn-small"
              onClick={() => gameApi.openExternal('https://github.com/Fallout-Anomaly/changelog').catch(() => {})}
            >
              View Release
            </button>
          </div>
        </div>
      )}

      <div className="updates-container">
        <div className="updates-list-section">
          <h3>Available Updates ({updates.length})</h3>
          {loading && updates.length === 0 ? (
            <p className="loading">Checking for updates...</p>
          ) : updates.length === 0 ? (
            <p className="empty">No updates available. All mods are up to date!</p>
          ) : (
            <ul className="update-items">
              {updates.map((update) => (
                <li
                  key={update.mod_id}
                  className={`update-item ${selectedMod === update.mod_id ? 'selected' : ''}`}
                  onClick={() =>
                    setSelectedMod(selectedMod === update.mod_id ? null : update.mod_id)
                  }
                >
                  <div className="update-name">{update.mod_id}</div>
                  <div className="update-versions">
                    <span className="current">v{update.current_version}</span>
                    <span className="arrow">→</span>
                    <span className="available">v{update.available_version}</span>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="update-details-section">
          {getSelectedMod() ? (
            <>
              <div className="update-info">
                <h4>{getSelectedMod()!.mod_id}</h4>
                <div className="version-comparison">
                  <div className="version-item">
                    <label>Current Version:</label>
                    <span className="version-value current">v{getSelectedMod()!.current_version}</span>
                  </div>
                  <div className="arrow-separator">→</div>
                  <div className="version-item">
                    <label>Available Version:</label>
                    <span className="version-value available">
                      v{getSelectedMod()!.available_version}
                    </span>
                  </div>
                </div>
              </div>

              <div className="changelog-section">
                <h5>Changelog</h5>
                <div className="changelog-content">
                  {getSelectedMod()!.changelog ? (
                    <p>{getSelectedMod()!.changelog}</p>
                  ) : (
                    <p className="no-changelog">No changelog available</p>
                  )}
                </div>
              </div>

              <div className="update-actions">
                <button
                  className="btn btn-small btn-update"
                  onClick={() => handleUpdateMod(getSelectedMod()!.mod_id)}
                  disabled={updating === getSelectedMod()!.mod_id}
                >
                  {updating === getSelectedMod()!.mod_id ? 'Updating...' : 'Update Now'}
                </button>
              </div>
            </>
          ) : (
            <div className="no-selection">Select a mod to view update details and changelog</div>
          )}
        </div>
      </div>

      {message && (
        <div className={`message ${message.includes('Error') ? 'error' : 'success'}`}>
          {message}
        </div>
      )}
    </div>
  )
}
