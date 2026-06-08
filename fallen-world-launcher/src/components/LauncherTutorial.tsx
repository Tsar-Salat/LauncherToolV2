import { useState } from 'react'
import '../styles/LauncherTutorial.css'

interface TutorialStep {
  icon: string
  title: string
  body: string
  hint?: string
}

const STEPS: TutorialStep[] = [
  {
    icon: '◉',
    title: 'Dashboard',
    body: 'Your home base. Launch the game or Mod Organizer 2 from here, watch the Commonwealth News ticker for modlist updates, check whether the game is running, and browse the latest changelog entries and community YouTube videos.',
    hint: 'The Launch Game button uses your saved settings and scarcity sliders automatically.',
  },
  {
    icon: '▤',
    title: 'Mod Manager',
    body: 'See every mod in your Fallen World install. Toggle mods on or off, reorder them by dragging, and add your own custom mods from a folder or zip archive. Changes are written directly to MO2\'s modlist.txt.',
    hint: 'Do not run MO2 at the same time — the launcher will close it automatically on startup.',
  },
  {
    icon: '⬡',
    title: 'Optional Mods',
    body: 'Run the FOMOD installer to customize your experience. Pick your GPU, select upscaler (DLAA keeps ENB; DLSS is faster but incompatible with ENB), choose a map style, enable creature grabs, add Unlimited Survival Mode, and more.',
    hint: 'Complete First-Time Setup before running the installer — it needs your resolution and GPU info.',
  },
  {
    icon: '✦',
    title: 'ENB Presets',
    body: 'Browse and apply community ENB preset packages to change the visual style of the game — from realistic lighting to cinematic looks. Presets are swapped in one click without manually copying files.',
  },
  {
    icon: '◈',
    title: 'Profiles',
    body: 'Manage your Mod Organizer 2 profiles. Create new profiles (copies of the active one), switch between them, rename them, and move your save files out of the MO2 folder to a safe location using the Backup Saves button.',
    hint: 'The "Fallen World" profile is protected — it is the source of truth and cannot be renamed or deleted.',
  },
  {
    icon: '✎',
    title: 'INI Editor',
    body: 'Edit Fallout 4\'s INI files directly. The Basic tab exposes common settings like resolution, shadow distance, and VSync. The Advanced tab gives you a full per-file structured editor with search.',
    hint: 'Always use Backup INI before making major changes — you can restore it with one click.',
  },
  {
    icon: '⛭',
    title: 'Settings',
    body: 'Configure the launcher itself: set your game path, change the display language, switch between colour themes, and control launcher-level preferences.',
  },
  {
    icon: '⚒',
    title: 'Debug',
    body: 'If something goes wrong, come here first. View launcher logs, run diagnostics, and check for configuration problems. Share the log output when asking for support on the Fallen World Discord.',
  },
  {
    icon: '☢',
    title: 'Quick Actions & Status',
    body: 'The left sidebar has quick-launch buttons for common tasks (launch game, launch MO2, backup saves, change ENB). Below that, live CPU, GPU and RAM usage bars update every 2 seconds — useful for monitoring performance before launching.',
    hint: 'You can return to this tutorial at any time from the Settings page.',
  },
]

interface LauncherTutorialProps {
  onDone: () => void
}

export default function LauncherTutorial({ onDone }: LauncherTutorialProps) {
  const [step, setStep] = useState(0)

  const isLast = step === STEPS.length - 1
  const current = STEPS[step]

  const advance = () => {
    if (isLast) { onDone(); return }
    setStep(step + 1)
  }

  return (
    <div className="tutorial-overlay" role="dialog" aria-modal="true" aria-label="Launcher tutorial">
      {/* Scanline backdrop */}
      <div className="tutorial-backdrop" onClick={onDone} />

      <div className="tutorial-card">
        {/* Header */}
        <div className="tutorial-card__header">
          <div className="tutorial-card__step-counter">
            {STEPS.map((_, i) => (
              <button
                key={i}
                className={`tutorial-dot ${i === step ? 'active' : ''}`}
                onClick={() => setStep(i)}
                aria-label={`Step ${i + 1}: ${STEPS[i].title}`}
              />
            ))}
          </div>
          <button className="tutorial-skip" onClick={onDone} aria-label="Skip tutorial">
            ✕ Skip
          </button>
        </div>

        {/* Body */}
        <div className="tutorial-card__body">
          <div className="tutorial-icon">{current.icon}</div>
          <div className="tutorial-section-label">Step {step + 1} of {STEPS.length}</div>
          <h2 className="tutorial-title">{current.title}</h2>
          <p className="tutorial-body">{current.body}</p>
          {current.hint && (
            <div className="tutorial-hint">
              <span className="tutorial-hint__icon">☢</span>
              {current.hint}
            </div>
          )}
        </div>

        {/* Navigation */}
        <div className="tutorial-card__footer">
          <button
            className="btn btn-secondary tutorial-prev"
            onClick={() => setStep(Math.max(0, step - 1))}
            disabled={step === 0}
          >
            ← Prev
          </button>
          <button className="btn btn-primary tutorial-next" onClick={advance}>
            {isLast ? '✓ Done' : 'Next →'}
          </button>
        </div>
      </div>
    </div>
  )
}
