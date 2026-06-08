import '../styles/SetupRequired.css'

interface SetupRequiredProps {
  feature: string
}

export default function SetupRequired({ feature }: SetupRequiredProps) {
  return (
    <div className="setup-required">
      <div className="setup-required-icon">⚠</div>
      <h3>Setup Required</h3>
      <p>
        The <strong>{feature}</strong> feature needs your Fallen World installation path.
      </p>
      <p className="setup-required-action">
        Go to <strong>Settings</strong> → <strong>Game Paths</strong> and click <strong>Set Path</strong>.
      </p>
    </div>
  )
}
