import '../styles/ComingSoon.css'

interface ComingSoonProps {
  title: string
  icon?: string
  note?: string
}

/** Placeholder panel for features that are planned but not yet built. */
export default function ComingSoon({ title, icon = '✦', note }: ComingSoonProps) {
  return (
    <div className="page-content coming-soon">
      <div className="cs-card">
        <div className="cs-stamp">WIP</div>
        <div className="cs-icon">{icon}</div>
        <h2 className="cs-title">{title}</h2>
        <p className="cs-sub">Coming Soon</p>
        {note && <p className="cs-note">{note}</p>}
        <div className="cs-bar"><span /></div>
      </div>
    </div>
  )
}
