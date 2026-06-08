import { useState, useEffect, useRef } from 'react'
import {
  gameApi, systemApi, dashboardApi, lootApi, loadScarcity, shouldCloseOnLaunch,
  ChangelogData, ProcessStatus, AnomalyUpdate, YoutubeVideo,
} from '../utils/tauriApi'
import { useI18n } from '../i18n'
import LootScarcityPanel from '../components/LootScarcityPanel'
import '../styles/pages/HomePage.css'

// Apply the current loot-scarcity sliders, then launch. Loot apply failure never blocks play.
const launchWithLoot = async () => {
  try { await lootApi.applyScarcity(loadScarcity()) } catch { /* loot not updated */ }
  return gameApi.launch()
}

const CHANGELOG_REPO_URL = 'https://github.com/Fallout-Anomaly/changelog/issues'

function StatusRow({ label, running, t }: { label: string; running: boolean; t: (k: string) => string }) {
  return (
    <div className="status-row">
      <span className="status-row__label">{label}</span>
      <span className={`status-chip ${running ? 'is-on' : 'is-off'}`}>
        {running ? t('dash.running') : t('dash.stopped')}
      </span>
    </div>
  )
}

function fmtDate(iso: string): string {
  if (!iso) return ''
  const d = new Date(iso)
  return isNaN(d.getTime()) ? '' : d.toLocaleDateString()
}

export default function HomePage() {
  const { t } = useI18n()
  const [changelog, setChangelog] = useState<ChangelogData | null>(null)
  const [procStatus, setProcStatus] = useState<ProcessStatus | null>(null)
  const [update, setUpdate] = useState<AnomalyUpdate | null>(null)
  const [showUpdateBanner, setShowUpdateBanner] = useState(false)
  const [newsText, setNewsText] = useState('')
  const [videos, setVideos] = useState<YoutubeVideo[]>([])
  const [selected, setSelected] = useState<YoutubeVideo | null>(null)
  const [gamePath, setGamePath] = useState('')
  const [loading, setLoading] = useState(false)
  const [message, setMessage] = useState('')
  const mounted = useRef(true)

  useEffect(() => {
    mounted.current = true
    ;(async () => {
      try {
        const [cl, path] = await Promise.all([
          dashboardApi.changelog(),
          systemApi.getConfiguredGamePath(),
        ])
        if (mounted.current) {
          setChangelog(cl)
          setGamePath(path || '')
        }
      } catch (err) {
        if (mounted.current) setMessage(`Error loading dashboard: ${err instanceof Error ? err.message : String(err)}`)
      }
    })()

    // News banner: an update takes priority; otherwise show newsbanner.md.
    dashboardApi.checkUpdate().then((u) => {
      if (!mounted.current) return
      setUpdate(u)
      setShowUpdateBanner(u.is_new)
      if (u.is_new) {
        setNewsText(u.raw)
      } else {
        dashboardApi.newsBanner().then((n) => { if (mounted.current && n) setNewsText(n) }).catch(() => {})
      }
    }).catch(() => {
      // No update info — still try the news banner.
      dashboardApi.newsBanner().then((n) => { if (mounted.current && n) setNewsText(n) }).catch(() => {})
    })

    // YouTube channel feed — non-blocking.
    dashboardApi.youtubeVideos(8).then((vids) => {
      if (!mounted.current) return
      setVideos(vids)
      setSelected(vids[0] ?? null)
    }).catch(() => { /* offline: panel shows fallback */ })

    const pollStatus = async () => {
      try {
        const s = await dashboardApi.processStatus()
        if (mounted.current) setProcStatus(s)
      } catch { /* ignore */ }
    }
    pollStatus()
    const id = setInterval(pollStatus, 3000)
    return () => {
      mounted.current = false
      clearInterval(id)
    }
  }, [])

  const dismissUpdate = () => {
    if (update) dashboardApi.markUpdateSeen(update.version).catch(() => {})
    setShowUpdateBanner(false)
  }

  const launch = async (
    fn: () => Promise<{ success: boolean; message: string }>,
    isGameLaunch = false,
  ) => {
    setLoading(true)
    try {
      const result = await fn()
      if (mounted.current) setMessage(result.message)
      // Optionally close the launcher once the game is on its way.
      if (isGameLaunch && result.success && shouldCloseOnLaunch()) {
        if (mounted.current) setMessage('Game launching — closing launcher…')
        setTimeout(() => { gameApi.quit().catch(() => {}) }, 1800)
      }
    } catch (err) {
      if (mounted.current) setMessage(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      if (mounted.current) setLoading(false)
    }
  }

  const canLaunch = !!gamePath && !loading
  const bannerText = newsText || 'Welcome to the Fallen World Launcher'
  const openChangelog = () => gameApi.openExternal(CHANGELOG_REPO_URL).catch(() => {})

  return (
    <div className="page-content dashboard">
      {/* Launch update banner (version.md changed since last seen) */}
      {showUpdateBanner && update && (
        <div className="update-banner">
          <span className="update-banner__icon">☢</span>
          <div className="update-banner__body">
            <strong>Update available — {update.version}</strong>
            {update.description && <span>{update.description}</span>}
          </div>
          <div className="update-banner__actions">
            <button className="btn btn-secondary" onClick={openChangelog}>View</button>
            <button className="btn btn-secondary" onClick={dismissUpdate}>Dismiss</button>
          </div>
        </div>
      )}

      {/* Commonwealth News banner */}
      <div className="news-banner">
        <div className="news-banner__tag">📡 {t('dash.news')}</div>
        <div className="news-banner__track">
          {/* Duplicate text for seamless infinite scroll */}
          <span className="news-banner__text">☢ {bannerText} &nbsp;&nbsp;|&nbsp;&nbsp; ☢ {bannerText}</span>
        </div>
      </div>

      <div className="dash-grid">
        {/* Launch options */}
        <section className="panel dash-launch">
          <div className="panel__title">▶ {t('dash.launchOptions')}</div>
          <div className="panel__body">
            <div className="launch-row">
              <button className="btn btn-primary" disabled={!canLaunch} onClick={() => launch(launchWithLoot, true)}>
                ▶ {t('dash.launchGame')}
              </button>
              <button className="btn btn-secondary" disabled={loading} onClick={() => launch(gameApi.launchMo2)}>
                ⬡ {t('dash.launchMo2')}
              </button>
            </div>
            {!gamePath && <p className="hint">⚠ Set your game path in {t('nav.settings')}.</p>}
            <LootScarcityPanel />
          </div>
        </section>

        {/* Game status */}
        <section className="panel dash-status">
          <div className="panel__title">◉ {t('dash.gameStatus')}</div>
          <div className="panel__body">
            <StatusRow label={t('dash.fallout4')} running={!!procStatus?.fallout4_running} t={t} />
            <StatusRow label={t('dash.mo2')} running={!!procStatus?.mo2_running} t={t} />
          </div>
        </section>

        {/* Latest changes */}
        <section className="panel dash-changes">
          <div className="panel__title">
            📌 {t('dash.latestChanges')}
            <button className="ghost-btn" onClick={openChangelog}>{t('dash.viewFullChangelog')}</button>
          </div>
          <div className="panel__body changelog-scroll">
            {changelog?.sections.map((section) => (
              <div key={section.category} className="cl-section">
                <div className="cl-category">{section.category}</div>
                {section.entries.map((entry) => (
                  <div key={entry.title} className="cl-entry">
                    <h4 className="cl-entry__title">{entry.title}</h4>
                    <ul className="cl-notes">{entry.notes.map((n, i) => <li key={i}>{n}</li>)}</ul>
                    {entry.links.length > 0 && (
                      <div className="cl-links">
                        {entry.links.map((l) => (
                          <button key={l.label} className="cl-link" onClick={() => l.url && gameApi.openExternal(l.url).catch(() => {})}>
                            ⬡ {l.label}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            ))}
          </div>
        </section>

        {/* YouTube channel */}
        <section className="panel dash-channel">
          <div className="panel__title">▶ {t('dash.channel')}</div>
          <div className="panel__body channel-body">
            {selected ? (
              <>
                <iframe
                  key={selected.id}
                  className="yt-embed"
                  src={selected.embed_url}
                  title={selected.title}
                  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
                  allowFullScreen
                />
                <div className="yt-list">
                  {videos.map((v) => (
                    <button
                      key={v.id}
                      className={`yt-item ${selected.id === v.id ? 'active' : ''}`}
                      onClick={() => setSelected(v)}
                    >
                      <img className="yt-thumb" src={v.thumbnail} alt="" loading="lazy" />
                      <span className="yt-meta">
                        <span className="yt-title">{v.title}</span>
                        <span className="yt-date">{fmtDate(v.published)}</span>
                      </span>
                    </button>
                  ))}
                </div>
              </>
            ) : (
              <div className="channel-placeholder">Channel feed unavailable (offline?)</div>
            )}
          </div>
        </section>
      </div>

      {changelog && (
        <div className="dash-meta">v{changelog.version}{changelog.released && ` · ${t('dash.released')} ${changelog.released}`}</div>
      )}

      {message && (
        <div className={`message ${message.toLowerCase().includes('error') || message.toLowerCase().includes('cannot') ? 'error' : 'success'}`}>
          {message}
        </div>
      )}
    </div>
  )
}
