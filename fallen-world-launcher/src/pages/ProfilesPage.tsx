import { useState, useEffect, useRef } from 'react'
import { profilesApi, systemApi } from '../utils/tauriApi'
import type { GameProfile } from '../utils/tauriApi'
import SetupRequired from '../components/SetupRequired'
import '../styles/pages/ProfilesPage.css'

const PROTECTED_PROFILE = 'Fallen World'

const PROFILE_NAME_MAX_LENGTH = 50
const PROFILE_NAME_REGEX = /^[a-zA-Z0-9_ \-]+$/

export default function ProfilesPage() {
  const [profiles, setProfiles] = useState<GameProfile[]>([])
  const [newProfileName, setNewProfileName] = useState('')
  const [loading, setLoading] = useState(false)
  const [message, setMessage] = useState('')
  const [renamingProfile, setRenamingProfile] = useState<string | null>(null)
  const [renameValue, setRenameValue] = useState('')
  const [pathConfigured, setPathConfigured] = useState<boolean | null>(null)
  const isMountedRef = useRef(true)

  useEffect(() => {
    isMountedRef.current = true
    systemApi.isGamePathConfigured().then((configured) => {
      if (!isMountedRef.current) return
      setPathConfigured(configured)
      if (configured) loadProfiles()
    }).catch(() => {
      if (isMountedRef.current) setPathConfigured(false)
    })
    return () => { isMountedRef.current = false }
  }, [])

  const loadProfiles = async () => {
    setLoading(true)
    try {
      const data = await profilesApi.list()
      if (isMountedRef.current) setProfiles(data || [])
    } catch (err) {
      if (isMountedRef.current)
        setMessage(`Error loading profiles: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  const validateProfileName = (name: string): string | null => {
    const trimmed = name.trim()
    if (!trimmed) return 'Profile name cannot be empty'
    if (trimmed.length > PROFILE_NAME_MAX_LENGTH) return `Profile name must be ${PROFILE_NAME_MAX_LENGTH} characters or less`
    if (!PROFILE_NAME_REGEX.test(trimmed)) return 'Profile name can only contain letters, numbers, spaces, dashes, and underscores'
    return null
  }

  const handleCreateProfile = async () => {
    const nameError = validateProfileName(newProfileName)
    if (nameError) { setMessage(nameError); return }

    setLoading(true)
    try {
      const result = await profilesApi.save({
        name: newProfileName.trim(),
        is_active: false,
        enabled_mods: [],
        ini_overrides: {},
        created_date: new Date().toISOString(),
        last_modified: new Date().toISOString(),
      })
      if (isMountedRef.current) {
        setMessage(result.success ? result.message : `Error: ${result.message}`)
        if (result.success) { setNewProfileName(''); await loadProfiles() }
      }
    } catch (err) {
      if (isMountedRef.current)
        setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  const handleActivateProfile = async (name: string) => {
    setLoading(true)
    try {
      const result = await profilesApi.activate(name)
      if (isMountedRef.current) {
        setMessage(result.success ? result.message : `Error: ${result.message}`)
        if (result.success) await loadProfiles()
      }
    } catch (err) {
      if (isMountedRef.current)
        setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  const handleDeleteProfile = async (name: string) => {
    if (!confirm(`Delete profile "${name}"? This cannot be undone.`)) return

    setLoading(true)
    try {
      const result = await profilesApi.delete(name)
      if (isMountedRef.current) {
        setMessage(result.success ? result.message : `Error: ${result.message}`)
        if (result.success) await loadProfiles()
      }
    } catch (err) {
      if (isMountedRef.current)
        setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  const handleStartRename = (name: string) => {
    setRenamingProfile(name)
    setRenameValue(name)
  }

  const handleConfirmRename = async () => {
    if (!renamingProfile) return
    const nameError = validateProfileName(renameValue)
    if (nameError) { setMessage(nameError); return }

    setLoading(true)
    try {
      const result = await profilesApi.rename(renamingProfile, renameValue.trim())
      if (isMountedRef.current) {
        setMessage(result.success ? result.message : `Error: ${result.message}`)
        if (result.success) { setRenamingProfile(null); await loadProfiles() }
      }
    } catch (err) {
      if (isMountedRef.current)
        setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  const handleCancelRename = () => { setRenamingProfile(null); setRenameValue('') }

  const handleOpenFolder = async () => {
    try {
      const result = await profilesApi.openFolder()
      if (!result.success) setMessage(`Error: ${result.message}`)
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    }
  }

  const handleBackupSaves = async () => {
    setLoading(true)
    try {
      const result = await profilesApi.backupSaves()
      if (isMountedRef.current)
        setMessage(result.success ? result.message : `Error: ${result.message}`)
    } catch (err) {
      if (isMountedRef.current)
        setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }

  if (pathConfigured === false) {
    return <SetupRequired feature="Profile Management" />
  }

  return (
    <div className="page-content profiles-page">
      <div className="create-profile-section">
        <h3>Create New Profile</h3>
        <p className="profile-hint">Creates a copy of the currently active profile with a new name.</p>
        <div className="input-group">
          <input
            type="text"
            value={newProfileName}
            onChange={(e) => setNewProfileName(e.target.value)}
            placeholder="Profile name..."
            className="profile-input"
            onKeyPress={(e) => e.key === 'Enter' && handleCreateProfile()}
          />
          <button className="btn btn-create" onClick={handleCreateProfile} disabled={loading}>
            Create
          </button>
        </div>
      </div>

      <div className="profiles-list-section">
        <div className="profiles-list-header">
          <h3>MO2 Profiles</h3>
          <div className="profiles-list-header__actions">
            <button className="btn btn-small" onClick={loadProfiles} disabled={loading} title="Reload profiles from disk">
              ↻ Refresh
            </button>
            <button className="btn btn-small" onClick={handleBackupSaves} disabled={loading} title="Move saves out of MO2 folder">
              ◫ Backup Saves
            </button>
            <button className="btn btn-folder" onClick={handleOpenFolder} title="Open profiles folder in Explorer">
              ◫ Open Folder
            </button>
          </div>
        </div>

        {loading && profiles.length === 0 ? (
          <p className="loading">Loading profiles...</p>
        ) : profiles.length === 0 ? (
          <p className="empty">No MO2 profiles found. Check your MO2 installation.</p>
        ) : (
          <div className="profiles-grid">
            {profiles.map((profile) => (
              <div
                key={profile.name}
                className={`profile-card ${profile.is_active ? 'profile-card--active' : ''}`}
              >
                {renamingProfile === profile.name ? (
                  <div className="rename-section">
                    <input
                      type="text"
                      value={renameValue}
                      onChange={(e) => setRenameValue(e.target.value)}
                      className="profile-input"
                      placeholder="New profile name..."
                      onKeyPress={(e) => e.key === 'Enter' && handleConfirmRename()}
                    />
                    <div className="rename-buttons">
                      <button className="btn btn-small" onClick={handleConfirmRename} disabled={loading}>Confirm</button>
                      <button className="btn btn-small" onClick={handleCancelRename} disabled={loading}>Cancel</button>
                    </div>
                  </div>
                ) : (
                  <>
                    <div className="profile-card-header">
                      <h4>{profile.name}</h4>
                      {profile.is_active && <span className="profile-active-badge">ACTIVE</span>}
                    </div>
                    <div className="profile-stats">
                      <div>Mods enabled: {profile.enabled_mods.length}</div>
                      {profile.last_modified && (
                        <div>Modified: {new Date(profile.last_modified).toLocaleDateString()}</div>
                      )}
                    </div>
                    <div className="profile-actions">
                      {!profile.is_active && (
                        <button
                          className="btn btn-small btn-activate"
                          onClick={() => handleActivateProfile(profile.name)}
                          disabled={loading}
                          title="Set as active MO2 profile"
                        >
                          Activate
                        </button>
                      )}
                      <button
                        className="btn btn-small"
                        onClick={() => handleStartRename(profile.name)}
                        disabled={loading || profile.name === PROTECTED_PROFILE}
                        title={profile.name === PROTECTED_PROFILE ? 'The Fallen World profile cannot be renamed' : 'Rename profile'}
                      >
                        Rename
                      </button>
                      <button
                        className="btn btn-small btn-danger"
                        onClick={() => handleDeleteProfile(profile.name)}
                        disabled={loading || profile.is_active}
                        title={profile.is_active ? 'Cannot delete the active profile' : 'Delete profile'}
                      >
                        Delete
                      </button>
                    </div>
                  </>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {message && (
        <div className={`message ${message.startsWith('Error') ? 'error' : 'success'}`}>
          {message}
          <button className="message-dismiss" onClick={() => setMessage('')}>✕</button>
        </div>
      )}
    </div>
  )
}
