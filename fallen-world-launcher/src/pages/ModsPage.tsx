import { useState, useEffect, useRef, useMemo, useCallback } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { modsApi, ModEntry, dashboardApi } from '../utils/tauriApi'
import '../styles/pages/ModsPage.css'

interface ModRowProps {
  m: ModEntry
  busy: string
  mo2Running: boolean
  override: boolean
  onToggle: (m: ModEntry) => void
  onMove: (m: ModEntry, up: boolean) => void
}

function ModRow({ m, busy, mo2Running, override: overrideMode, onToggle, onMove }: ModRowProps) {
  const locked = !m.is_user && !overrideMode
  const isBusy = busy === m.name
  const canEdit = !locked && !mo2Running
  return (
    <div className={`mod-row ${m.enabled ? 'on' : 'off'} ${locked ? 'locked' : ''}`}>
      <button
        className={`mod-toggle ${m.enabled ? 'on' : 'off'}`}
        onClick={() => onToggle(m)}
        disabled={!canEdit || isBusy}
        title={locked ? 'Base modlist mod (enable Override to change)' : (m.enabled ? 'Disable' : 'Enable')}
      >
        {isBusy ? '…' : m.enabled ? '✓' : ''}
      </button>
      <div className="mod-move">
        <button className="mod-move__btn" onClick={() => onMove(m, false)} disabled={!canEdit || isBusy} title="Higher priority">▲</button>
        <button className="mod-move__btn" onClick={() => onMove(m, true)} disabled={!canEdit || isBusy} title="Lower priority">▼</button>
      </div>
      <span className="mod-name">{m.name}</span>
      <span className="mod-tags">
        {m.is_user && <span className="mod-tag user">USER</span>}
        {locked && <span className="mod-tag lock">🔒</span>}
        <span className={`mod-state ${m.enabled ? 'on' : 'off'}`}>{m.enabled ? 'ON' : 'OFF'}</span>
      </span>
    </div>
  )
}

export default function ModsPage() {
  const [mods, setMods] = useState<ModEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState('') // name currently mutating
  const [message, setMessage] = useState('')
  const [filter, setFilter] = useState('')
  const [override, setOverride] = useState(false)
  const [mo2Running, setMo2Running] = useState(false)
  const mounted = useRef(true)

  useEffect(() => {
    mounted.current = true
    load()
    checkMo2()
    return () => { mounted.current = false }
  }, [])

  const checkMo2 = async () => {
    try {
      const s = await dashboardApi.processStatus()
      if (mounted.current) setMo2Running(s.mo2_running)
    } catch { /* non-fatal */ }
  }

  const load = async () => {
    setLoading(true)
    try {
      const list = await modsApi.list()
      if (mounted.current) setMods(list)
    } catch (err) {
      if (mounted.current) setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (mounted.current) setLoading(false)
    }
  }

  const toggle = useCallback(async (m: ModEntry) => {
    setBusy(m.name)
    try {
      const r = await modsApi.toggle(m.name, !m.enabled)
      if (!r.success) setMessage(r.message)
      await load()
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (mounted.current) setBusy('')
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const move = useCallback(async (m: ModEntry, up: boolean) => {
    setBusy(m.name)
    try {
      const r = await modsApi.move(m.name, up)
      if (!r.success) setMessage(r.message)
      await load()
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (mounted.current) setBusy('')
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const installFrom = async (source: string) => {
    setLoading(true)
    try {
      const r = await modsApi.addUserMod(source)
      setMessage(r.message)
      await load()
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (mounted.current) setLoading(false)
    }
  }

  const addZip = async () => {
    const picked = await open({
      multiple: false,
      title: 'Select a mod .zip archive',
      filters: [{ name: 'Mod archive', extensions: ['zip'] }],
    })
    if (picked && typeof picked === 'string') await installFrom(picked)
  }

  const addFolder = async () => {
    const picked = await open({ directory: true, multiple: false, title: 'Select a mod folder to add' })
    if (picked && typeof picked === 'string') await installFrom(picked)
  }

  const toggleOverride = () => {
    if (!override) {
      const ok = confirm(
        'Override mode lets you enable/disable ANY mod in the list, including the curated ' +
        'Fallen World base mods.\n\nModified load orders are NOT supported — issues caused by ' +
        'changes here will not be covered by support. Continue?'
      )
      if (!ok) return
    }
    setOverride(!override)
  }

  const f = filter.toLowerCase()
  const { userMods, baseMods } = useMemo(() => {
    const visible = mods.filter((m) => !m.is_separator && (!f || m.name.toLowerCase().includes(f)))
    // MO2 stores modlist.txt with lowest-priority mods first; reverse so the
    // display matches MO2's UI (highest priority at the top).
    return {
      userMods: visible.filter((m) => m.is_user).reverse(),
      baseMods: visible.filter((m) => !m.is_user).reverse(),
    }
  }, [mods, f])

  const enabledCount = mods.filter((m) => m.enabled && !m.is_separator).length
  const totalCount = mods.filter((m) => !m.is_separator).length

  return (
    <div className="page-content mods-page">
      <div className="mods-toolbar panel">
        <div className="mods-toolbar__left">
          <h3 className="panel-title-inline">MOD LIST</h3>
          <span className="mods-count">{enabledCount} / {totalCount} enabled</span>
        </div>
        <div className="mods-toolbar__right">
          <input className="mods-filter" placeholder="Filter mods…" value={filter} onChange={(e) => setFilter(e.target.value)} />
          <button className="btn btn-primary" onClick={addZip} disabled={loading || mo2Running} title={mo2Running ? 'Close Mod Organizer 2 first' : 'Install a mod from a .zip archive'}>+ Add ZIP</button>
          <button className="btn btn-secondary" onClick={addFolder} disabled={loading || mo2Running} title={mo2Running ? 'Close Mod Organizer 2 first' : 'Install a mod from a folder'}>+ Folder</button>
          <button className="btn btn-secondary" onClick={() => { load(); checkMo2() }} disabled={loading}>↻ Refresh</button>
          <label className={`override-switch ${override ? 'on' : ''}`} title="Unlock all mods (unsupported)">
            <input type="checkbox" checked={override} onChange={toggleOverride} />
            <span>Override</span>
          </label>
        </div>
      </div>

      {mo2Running && (
        <div className="message error">
          ⚠ Mod Organizer 2 is running. Changes are locked — MO2 rewrites the mod list when it
          closes, which would undo anything changed here. Close MO2, then press Refresh.
        </div>
      )}

      {override && (
        <div className="message error">
          ⚠ Override is ON — base modlist mods are unlocked. Modified load orders are not supported.
        </div>
      )}

      {loading && mods.length === 0 ? (
        <p className="mods-loading">Reading MO2 modlist…</p>
      ) : mods.length === 0 ? (
        <p className="mods-loading">No mods found. Is Mod Organizer 2 detected? Set your game path in Settings.</p>
      ) : (
        <>
          <section className="mods-section panel">
            <div className="mods-section__head">
              <h4>Your Added Mods</h4>
              <span>{userMods.length}</span>
            </div>
            {userMods.length === 0 ? (
              <p className="mods-empty">Mods you add appear here, forced to the top of the load order. Use “+ Add Mod”.</p>
            ) : (
              <div className="mods-list">{userMods.map((m) => <ModRow key={m.name} m={m} busy={busy} mo2Running={mo2Running} override={override} onToggle={toggle} onMove={move} />)}</div>
            )}
          </section>

          <section className="mods-section mods-section--base panel">
            <div className="mods-section__head">
              <h4>Fallen World Base Mods</h4>
              <span>{baseMods.length}</span>
            </div>
            <p className="mods-empty">Curated and locked for stability. Enable Override to change (unsupported).</p>
            <div className="mods-list mods-list--base">{baseMods.map((m) => <ModRow key={m.name} m={m} busy={busy} mo2Running={mo2Running} override={override} onToggle={toggle} onMove={move} />)}</div>
          </section>
        </>
      )}

      {message && (
        <div className={`message ${message.toLowerCase().includes('error') ? 'error' : 'success'}`}>{message}</div>
      )}
    </div>
  )
}
