import { gameApi } from '../utils/tauriApi'
import '../styles/pages/ControllerGuidePage.css'

// Controller Setup guide. Steam Input based scheme (LB + button chords).
// The launcher already disables the conflicting native CustomControlMap.txt files
// and turns leaning off; the steps below are the manual Steam + in-game parts.

const STEAM_PRESET_URL = 'steam://controllerconfig/377160/3482465058'

const MODE_SHIFT: { combo: string; action: string }[] = [
  { combo: 'LB + A', action: 'Weapon Wheel' },
  { combo: 'LB + B', action: 'Enable / disable the Immersive HUD' },
  { combo: 'LB + X', action: 'Swap semi / full auto on the current weapon' },
  { combo: 'LB + Y', action: 'Replaceable armor plate' },
  { combo: 'LB + RB', action: 'VAFS - slow time' },
  { combo: 'LB + RT', action: 'VAFS - use criticals' },
  { combo: 'LB + D-Pad Down', action: 'Lower / holster weapon (tap RT to raise it again)' },
  { combo: 'LB + D-Pad Up', action: 'Dynamic Helmet - remove facial clothing (gas mask)' },
  { combo: 'LB + D-Pad Right', action: 'Swap shoulder camera in 3rd person' },
  { combo: 'Double-tap A', action: 'Dodge (does not fire in rapid succession)' },
  { combo: 'L3 + R3 (click both sticks)', action: 'Barber / surgery camera' },
]

const MCM_BINDS: { feature: string; key: string }[] = [
  { feature: 'VAFS - Toggle Focus / slow time', key: 'J' },
  { feature: 'VAFS - Use Criticals', key: 'K' },
  { feature: 'Dynamic Helmet - remove facial clothing', key: 'P' },
]

function Section(
  { title, wide, children }: { title: string; wide?: boolean; children: React.ReactNode },
) {
  return (
    <section className={`ctrl-section panel ${wide ? 'ctrl-section--wide' : ''}`}>
      <h3 className="ctrl-section__title">{title}</h3>
      {children}
    </section>
  )
}

export default function ControllerGuidePage() {
  const openPreset = () => gameApi.openExternal(STEAM_PRESET_URL).catch(() => {})

  return (
    <div className="page-content controller-page">
      <div className="ctrl-head">
        <span className="ctrl-head__icon" aria-hidden="true" />
        <div>
          <h2 className="ctrl-head__title">Controller Setup</h2>
          <p className="ctrl-head__sub">
            Full controller support using a Steam Input preset with “LB + button” mode shifts.
            Works with Xbox and PS5 (DualSense) controllers.
          </p>
        </div>
      </div>

      <div className="ctrl-beta panel">
        <h3 className="ctrl-beta__title">⚠ Controller support is in beta</h3>
        <p className="ctrl-text">
          This is still being refined — expect to do a few steps yourself, and don't be surprised
          if something needs tweaking on your end before it feels right.
        </p>
        <p className="ctrl-text" style={{ marginTop: 8 }}>
          Some mods won't respond to the controller at all. That's because most Fallout 4 mod
          authors only add <b>keyboard</b> bindings and never wire up gamepad support out of the
          box. This setup works around that by mapping Steam Input chords (and the MCM keybinds
          below) onto those keyboard actions — but a mod that has no rebindable key, or that
          hardcodes mouse input, may still only work on keyboard &amp; mouse.
        </p>
      </div>

      <div className="ctrl-grid">
        <Section title="Setup steps" wide>
          <div className="ctrl-steps">
            <div className="ctrl-step">
              <h4 className="ctrl-step__title">1. Apply the Steam Input preset</h4>
              <p className="ctrl-text">
                Click the button to open the shared controller layout in Steam, then choose
                “Apply Configuration”. Steam must be running.
              </p>
              <button className="btn btn-primary ctrl-apply" onClick={openPreset}>
                Apply Steam Controller Preset
              </button>
              <p className="ctrl-hint">
                If the button does nothing, paste this into your browser: <code>{STEAM_PRESET_URL}</code>
              </p>
            </div>

            <div className="ctrl-step">
              <h4 className="ctrl-step__title">2. Set controls to default in-game</h4>
              <ol className="ctrl-list">
                <li>Launch the game and load into the menu.</li>
                <li>Enable your controller in the in-game settings.</li>
                <li>Open the “Customise Controls” screen and reset everything to default.</li>
              </ol>
            </div>

            <div className="ctrl-step">
              <h4 className="ctrl-step__title">3. Match the MCM keybinds</h4>
              <p className="ctrl-text">
                Open the MCM and set these so the Steam chords trigger the right features.
                The facial-clothing key must be set to <b>P</b>.
              </p>
              <table className="ctrl-table">
                <thead>
                  <tr>
                    <th>Feature</th>
                    <th>MCM Key</th>
                  </tr>
                </thead>
                <tbody>
                  {MCM_BINDS.map((b) => (
                    <tr key={b.feature}>
                      <td>{b.feature}</td>
                      <td className="ctrl-key">{b.key}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </Section>

        <Section title="Button layout — hold LB for mode shifts" wide>
          <p className="ctrl-text">
            Based on the Fallout 4 default controller layout. All the normal buttons stay the same.
            Hold <b>LB</b> for the extra actions:
          </p>
          <div className="ctrl-modeshift">
            {MODE_SHIFT.map((m) => (
              <div key={m.combo} className="ctrl-chord">
                <span className="ctrl-combo">{m.combo}</span>
                <span className="ctrl-action">{m.action}</span>
              </div>
            ))}
          </div>
        </Section>

        <Section title="Notes" wide>
          <ul className="ctrl-list">
            <li>
              <b>Uneducated Shooter:</b> used for Gun Inertia and Dynamic Height. Leaning is turned
              off (vanilla peeking is used instead), which the launcher has already set for you.
            </li>
            <li>
              <b>Dynamic Helmet:</b> in the MCM, set it to remove facial clothing only. Selecting
              both helmet and facial clothing can sometimes leave one on and remove the other.
            </li>
            <li>Dodge does not trigger if you tap A too quickly in succession.</li>
          </ul>
        </Section>
      </div>

      <p className="ctrl-credit">Controller layout by Salamander and Xan. Thank you.</p>
    </div>
  )
}
