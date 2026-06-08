import { useEffect, useRef, useState } from 'react'
import '../styles/Sidebar.css'
import { useI18n } from '../i18n'
import { gameApi, dashboardApi, shouldCloseOnLaunch, LiveStats } from '../utils/tauriApi'
import ThemeBadge from './ThemeBadge'

export type PageName =
  | 'home' | 'setup' | 'presets' | 'mods' | 'profiles' | 'ini'
  | 'settings' | 'fomod' | 'debug' | 'controller'

interface SidebarProps {
  currentPage: PageName
  onPageChange: (page: PageName) => void
}

interface NavItem { key: PageName; labelKey: string; icon: string }

const NAV: NavItem[] = [
  { key: 'home', labelKey: 'nav.dashboard', icon: '◉' },
  { key: 'mods', labelKey: 'nav.mods', icon: '▤' },
  { key: 'fomod', labelKey: 'nav.fomod', icon: '⬡' },
  { key: 'presets', labelKey: 'nav.enb', icon: '✦' },
  { key: 'profiles', labelKey: 'nav.profiles', icon: '◈' },
  { key: 'ini', labelKey: 'nav.ini', icon: '✎' },
  { key: 'controller', labelKey: 'nav.controller', icon: '⊞' },
  { key: 'settings', labelKey: 'nav.settings', icon: '⛭' },
  { key: 'debug', labelKey: 'nav.debug', icon: '⚒' },
]

function StatBar({ label, name, pct }: { label: string; name?: string; pct: number | null }) {
  const value = pct ?? 0
  const tone = value > 85 ? 'crit' : value > 60 ? 'warn' : 'ok'
  return (
    <div className="stat">
      <div className="stat__head">
        <span className="stat__label">{label}</span>
        <span className="stat__name" title={name}>{name || '—'}</span>
        <span className={`stat__val stat__val--${tone}`}>{pct === null ? '—' : `${Math.round(value)}%`}</span>
      </div>
      <div className="stat__track">
        <div className={`stat__fill stat__fill--${tone}`} style={{ width: `${value}%` }} />
      </div>
    </div>
  )
}

export default function Sidebar({ currentPage, onPageChange }: SidebarProps) {
  const { t } = useI18n()
  const [stats, setStats] = useState<LiveStats | null>(null)
  const mounted = useRef(true)

  useEffect(() => {
    mounted.current = true
    const poll = async () => {
      try {
        const s = await dashboardApi.liveStats()
        if (mounted.current) setStats(s)
      } catch {
        /* monitor unavailable; bars stay at last known/empty */
      }
    }
    poll()
    const id = setInterval(poll, 2000)
    return () => {
      mounted.current = false
      clearInterval(id)
    }
  }, [])

  const open = (url: string) => gameApi.openExternal(url).catch(() => {})

  const quickActions = [
    { label: t('inv.launchF4se'), icon: '▶', run: () => gameApi.launch()
        .then((r) => { if (r.success && shouldCloseOnLaunch()) setTimeout(() => gameApi.quit().catch(() => {}), 1800) })
        .catch(() => {}) },
    { label: t('inv.launchMo2'), icon: '◈', run: () => gameApi.launchMo2().catch(() => {}) },
    { label: t('inv.backupSaves'), icon: '◫', run: () => onPageChange('profiles') },
    { label: t('inv.refreshProfiles'), icon: '↻', run: () => onPageChange('profiles') },
    { label: t('inv.optimizeEnb'), icon: '✦', run: () => onPageChange('presets') },
    { label: t('inv.settings'), icon: '⛭', run: () => onPageChange('settings') },
  ]

  const cpuLoad = stats?.cpu_usage ?? 0

  return (
    <aside className="sidebar">
      <div className="sidebar__brand">
        <div className="sidebar__logo">☢</div>
        <div className="sidebar__brandtext">
          <strong>Fallen</strong>
          <span>World</span>
        </div>
      </div>

      <nav className="sidebar__nav">
        {NAV.map((item) => (
          <button
            key={item.key}
            className={`nav-btn ${currentPage === item.key ? 'active' : ''}`}
            onClick={() => onPageChange(item.key)}
          >
            <span className={`nav-btn__icon${item.key === 'controller' ? ' nav-btn__icon--img' : ''}`}>
              {item.key === 'controller' ? '' : item.icon}
            </span>
            <span className="nav-btn__label">{t(item.labelKey)}</span>
          </button>
        ))}
      </nav>

      <section className="sidebar__section">
        <h4 className="sidebar__heading">◈ {t('inv.title')}</h4>
        <div className="quick-grid">
          {quickActions.map((qa) => (
            <button key={qa.label} className="quick-btn" onClick={qa.run}>
              <span className="quick-btn__icon">{qa.icon}</span>
              {qa.label}
            </button>
          ))}
        </div>
      </section>

      <section className="sidebar__section">
        <h4 className="sidebar__heading">☢ {t('status.title')}</h4>
        <StatBar label="CPU" name={stats?.cpu_name} pct={stats ? stats.cpu_usage : null} />
        <StatBar
          label="GPU"
          name={stats?.gpu_name}
          pct={stats ? stats.gpu_usage : null}
        />
        <StatBar
          label="RAM"
          name={stats ? `${stats.ram_used_gb.toFixed(1)} / ${stats.ram_total_gb.toFixed(1)} GB` : undefined}
          pct={stats ? stats.ram_usage : null}
        />
        <div className="geiger">
          <span
            className="geiger__dot"
            style={{ animationDuration: `${Math.max(0.25, 1.6 - cpuLoad / 100).toFixed(2)}s` }}
          />
          <span className="geiger__label">{t('status.geiger')}</span>
        </div>
      </section>

      <section className="sidebar__section sidebar__data">
        <h4 className="sidebar__heading">⛓ {t('data.title')}</h4>
        <button className="data-btn" onClick={() => open('https://ko-fi.com/falloutanomaly')}>☕ {t('data.support')}</button>
        <button className="data-btn" onClick={() => open('https://www.nexusmods.com/fallout4/mods/104700')}>✚ {t('data.nexus')}</button>
        <button className="data-btn" onClick={() => open('https://discord.com/invite/TAueAV8Utk')}>💬 {t('data.discord')}</button>
        <button className="data-btn" onClick={() => open('https://fallenworld.nexus/docs/intro/')}>📖 {t('data.troubleshooting')}</button>
      </section>

      <div className="sidebar__footer">
        <ThemeBadge />
        <span className="sidebar__version">v0.1.0</span>
      </div>
    </aside>
  )
}
