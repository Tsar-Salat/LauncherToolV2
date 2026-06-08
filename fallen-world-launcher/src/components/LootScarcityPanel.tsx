import { useState } from 'react'
import { LootChances, loadScarcity, saveScarcity, DEFAULT_SCARCITY } from '../utils/tauriApi'
import { useI18n } from '../i18n'

const FIELDS: { key: keyof LootChances; i18n: string }[] = [
  { key: 'guns', i18n: 'loot.guns' },
  { key: 'meds', i18n: 'loot.meds' },
  { key: 'food', i18n: 'loot.food' },
  { key: 'drink', i18n: 'loot.drinks' },
  { key: 'ammo', i18n: 'loot.ammo' },
  { key: 'junk', i18n: 'loot.junk' },
]

// Per-category world-loot scarcity sliders. Values persist to localStorage and are written
// into the RobCo loot INI on Play (see HomePage launch handler).
export default function LootScarcityPanel() {
  const { t } = useI18n()
  const [c, setC] = useState<LootChances>(loadScarcity)

  const update = (k: keyof LootChances, v: number) => {
    const next = { ...c, [k]: v }
    setC(next)
    saveScarcity(next)
  }

  const reset = () => {
    const next = { ...DEFAULT_SCARCITY }
    setC(next)
    saveScarcity(next)
  }

  const isDefault = (Object.keys(DEFAULT_SCARCITY) as (keyof LootChances)[])
    .every((k) => c[k] === DEFAULT_SCARCITY[k])

  return (
    <div className="loot-scarcity">
      <div className="loot-scarcity__head">
        <div className="panel__title">☢ {t('loot.title')}</div>
        <button className="btn btn-secondary loot-reset" onClick={reset} disabled={isDefault}>
          ↺ {t('loot.default')}
        </button>
      </div>
      {FIELDS.map((f) => (
        <label key={f.key} className="loot-row">
          <span className="loot-row__label">{t(f.i18n)}</span>
          <input
            type="range"
            min={5}
            max={100}
            value={100 - c[f.key]}
            onChange={(e) => update(f.key, 100 - Number(e.target.value))}
          />
          <span className="loot-row__val">{100 - c[f.key]}%</span>
        </label>
      ))}
      <p className="hint">{t('loot.hint')}</p>
    </div>
  )
}
