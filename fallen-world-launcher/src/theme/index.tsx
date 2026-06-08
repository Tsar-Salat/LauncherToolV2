import { createContext, useContext, useState, useCallback, useEffect, ReactNode } from 'react'

/** A selectable colour theme. The `id` maps to a `[data-theme="<id>"]` block in
 *  index.css that overrides the root CSS variables. `fallout` is the default
 *  (the bare `:root`), so it needs no attribute but is listed for the picker. */
export interface ThemeDef {
  id: string
  label: string
  /** Two swatch colours for the picker preview: [accent, surface]. */
  swatch: [string, string]
}

export const THEMES: ThemeDef[] = [
  { id: 'fallout', label: 'Pip-Boy Green (Default)', swatch: ['#9fd24f', '#0a0d08'] },
  { id: 'amber', label: 'Amber Terminal', swatch: ['#ffcf5c', '#0c0805'] },
  { id: 'vault', label: 'Vault-Tec Blue', swatch: ['#5fb3f0', '#070b14'] },
  { id: 'enclave', label: 'Enclave Steel', swatch: ['#d23a3a', '#0a0d10'] },
  { id: 'institute', label: 'Institute (Light)', swatch: ['#0b8f9c', '#eef3f6'] },
]

const STORAGE_KEY = 'fwl.theme'
const VALID = new Set(THEMES.map((t) => t.id))

function initialTheme(): string {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (saved && VALID.has(saved)) return saved
  } catch {
    /* localStorage unavailable; fall back to default */
  }
  return 'fallout'
}

interface ThemeValue {
  theme: string
  setTheme: (id: string) => void
  themes: ThemeDef[]
}

const ThemeContext = createContext<ThemeValue | null>(null)

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<string>(initialTheme)

  // Reflect the active theme onto <html data-theme> so CSS variable overrides
  // take effect app-wide.
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
  }, [theme])

  const setTheme = useCallback((id: string) => {
    const next = VALID.has(id) ? id : 'fallout'
    setThemeState(next)
    try {
      localStorage.setItem(STORAGE_KEY, next)
    } catch {
      /* theme simply won't persist */
    }
  }, [])

  return (
    <ThemeContext.Provider value={{ theme, setTheme, themes: THEMES }}>
      {children}
    </ThemeContext.Provider>
  )
}

export function useTheme(): ThemeValue {
  const ctx = useContext(ThemeContext)
  if (!ctx) throw new Error('useTheme must be used within a ThemeProvider')
  return ctx
}
