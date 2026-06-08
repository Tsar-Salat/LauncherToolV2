import RadioPlayer from './RadioPlayer'
import { useI18n } from '../i18n'
import { gameApi } from '../utils/tauriApi'

interface TopBarProps {
  /** Localized title for the current page (e.g. "Dashboard"). */
  pageTitle: string
}

export default function TopBar({ pageTitle }: TopBarProps) {
  const { t } = useI18n()

  const openHelp = () => {
    gameApi.openExternal('https://fallenworld.nexus/docs/intro/').catch(() => {})
  }

  return (
    <header className="topbar">
      <div className="topbar__title">
        <h1>{pageTitle}</h1>
        <span>{t('app.subtitle')}</span>
      </div>
      <div className="topbar__right">
        <RadioPlayer />
        <button className="help-link" onClick={openHelp}>
          ☢ {t('topbar.help')}
        </button>
      </div>
    </header>
  )
}
