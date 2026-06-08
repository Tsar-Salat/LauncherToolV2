import { useState, useEffect } from 'react'
import './styles/App.css'
import Sidebar, { PageName } from './components/Sidebar'
import TopBar from './components/TopBar'
import LauncherTutorial from './components/LauncherTutorial'
import { useI18n } from './i18n'
import { systemApi, onboardingApi } from './utils/tauriApi'
import HomePage from './pages/HomePage'
import PresetsPage from './pages/PresetsPage'
import ModsPage from './pages/ModsPage'
import ProfilesPage from './pages/ProfilesPage'
import IniEditorPage from './pages/IniEditorPage'
import SettingsPage from './pages/SettingsPage'
import FomodPage from './pages/FomodPage'
import SetupPage from './pages/SetupPage'
import DebugPage from './pages/DebugPage'
import ControllerGuidePage from './pages/ControllerGuidePage'

const TUTORIAL_SEEN_KEY = 'fwl.tutorial.seen'
const TUTORIAL_PENDING_KEY = 'fwl.tutorial.pending'

const PAGE_TITLE_KEY: Record<PageName, string> = {
  home: 'nav.dashboard',
  setup: 'nav.setup',
  presets: 'nav.enb',
  mods: 'nav.mods',
  profiles: 'nav.profiles',
  ini: 'nav.ini',
  controller: 'nav.controller',
  settings: 'nav.settings',
  fomod: 'nav.fomod',
  debug: 'nav.debug',
}

function App() {
  const { t } = useI18n()
  const [currentPage, setCurrentPage] = useState<PageName>('home')
  const [mo2Killed, setMo2Killed] = useState(false)
  const [mo2Vfs, setMo2Vfs] = useState(false)
  const [showTutorial, setShowTutorial] = useState(false)
  const [firstRun, setFirstRun] = useState(false)

  // On launch: if we were started *through* MO2's VFS, keep the launcher open and
  // prompt the user to close MO2 with a button (we don't auto-kill it because the
  // user explicitly launched it that way). Otherwise close MO2 if running and warn.
  useEffect(() => {
    systemApi.mo2Startup()
      .then(async res => {
        if (res.under_mo2) setMo2Vfs(true)
        else if (res.killed_mo2) setMo2Killed(true)

        let complete = true
        try { complete = await onboardingApi.isComplete() } catch { complete = true }

        if (!complete) {
          // First run: go into the setup phase and do NOT show the tutorial yet
          // (it fires after setup completes). If the MO2 warning is up, the jump
          // to setup is deferred until it's dismissed (see dismissMo2).
          setFirstRun(true)
          if (!res.killed_mo2) setCurrentPage('setup')
          return
        }

        // Returning user: show the tutorial if a prior setup save queued it.
        if (localStorage.getItem(TUTORIAL_PENDING_KEY) === '1') {
          localStorage.removeItem(TUTORIAL_PENDING_KEY)
          setShowTutorial(true)
        }
      })
      .catch(() => {})
  }, [])

  const dismissTutorial = () => {
    localStorage.setItem(TUTORIAL_SEEN_KEY, '1')
    setShowTutorial(false)
  }

  // Dismiss the MO2 warning; on a first run, continue into the startup phase.
  const dismissMo2 = () => {
    setMo2Killed(false)
    if (firstRun) setCurrentPage('setup')
  }

  const renderPage = () => {
    switch (currentPage) {
      case 'home':     return <HomePage />
      case 'setup':    return <SetupPage onComplete={() => { localStorage.removeItem(TUTORIAL_PENDING_KEY); setShowTutorial(true) }} />
      case 'presets':  return <PresetsPage />
      case 'mods':     return <ModsPage />
      case 'profiles': return <ProfilesPage />
      case 'ini':      return <IniEditorPage />
      case 'controller': return <ControllerGuidePage />
      case 'settings': return <SettingsPage onNavigate={setCurrentPage} onShowTutorial={() => setShowTutorial(true)} />
      case 'fomod':    return <FomodPage />
      case 'debug':    return <DebugPage />
      default:         return <HomePage />
    }
  }

  return (
    <div className="app-container">
      <Sidebar currentPage={currentPage} onPageChange={setCurrentPage} />
      <main className="main-content">
        <TopBar pageTitle={t(PAGE_TITLE_KEY[currentPage])} />
        {renderPage()}
      </main>

      {showTutorial && <LauncherTutorial onDone={dismissTutorial} />}

      {mo2Vfs && (
        <div className="mo2-modal-overlay">
          <div className="mo2-modal" onClick={e => e.stopPropagation()}>
            <div className="mo2-modal__icon">!</div>
            <h2 className="mo2-modal__title">LAUNCHED VIA MOD ORGANIZER</h2>
            <p className="mo2-modal__lead">
              You started the launcher through Mod Organizer 2. 
              Because MO2 uses a Virtual File System, the launcher cannot properly manage your mods or settings from here.
            </p>
            <p className="mo2-modal__reason-label">WHAT YOU NEED TO DO:</p>
            <ul className="mo2-modal__reasons">
              <li>
                <strong>Close Mod Organizer 2 completely.</strong>
              </li>
              <li>
                <strong>Go to your game folder.</strong>
              </li>
              <li>
                <strong>Double-click <code>Fallen World Launcher.exe</code> directly.</strong>
              </li>
            </ul>
            <p className="mo2-modal__footer">
              Do not run this launcher from the MO2 dropdown menu!
            </p>
            <button className="mo2-modal__btn" onClick={() => {
              import('@tauri-apps/api/process').then(m => m.exit(0));
            }}>
              EXIT LAUNCHER
            </button>
          </div>
        </div>
      )}

      {mo2Killed && (
        <div className="mo2-modal-overlay" onClick={dismissMo2}>
          <div className="mo2-modal" onClick={e => e.stopPropagation()}>
            <div className="mo2-modal__icon">⚠</div>
            <h2 className="mo2-modal__title">MOD ORGANIZER 2 CLOSED</h2>
            <p className="mo2-modal__lead">
              Mod Organizer 2 was detected and has been force-closed.
            </p>
            <p className="mo2-modal__reason-label">THIS IS REQUIRED BECAUSE:</p>
            <ul className="mo2-modal__reasons">
              <li>
                MO2 keeps the mod list in memory and{' '}
                <strong>rewrites modlist.txt on exit</strong>, silently
                overwriting every change made in this launcher.
              </li>
              <li>
                Running both tools at the same time creates{' '}
                <strong>conflicting load-order edits</strong> that corrupt
                the intended mod priority.
              </li>
              <li>
                Changing active mods while a session is in-flight risks{' '}
                <strong>save-file incompatibilities</strong> and mid-session
                crashes.
              </li>
            </ul>
            <p className="mo2-modal__footer">
              Use this launcher as your single control point.
              Launch the game from here when you are ready.
            </p>
            <button className="mo2-modal__btn" onClick={dismissMo2}>
              UNDERSTOOD — CONTINUE
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

export default App
