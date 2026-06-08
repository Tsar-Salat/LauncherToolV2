import { createContext, useContext, useState, useCallback, ReactNode } from 'react'
import { DICTIONARIES, LANGUAGES, LangCode } from './locales'

const STORAGE_KEY = 'fwl.lang'

function initialLang(): LangCode {
  const saved = (typeof localStorage !== 'undefined' && localStorage.getItem(STORAGE_KEY)) as LangCode | null
  return saved && saved in LANGUAGES ? saved : 'en'
}

interface I18nValue {
  lang: LangCode
  setLang: (l: LangCode) => void
  /** Translate a key, falling back to English then the key itself. */
  t: (key: string) => string
  languages: typeof LANGUAGES
}

const I18nContext = createContext<I18nValue | null>(null)

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<LangCode>(initialLang)

  const setLang = useCallback((l: LangCode) => {
    setLangState(l)
    try {
      localStorage.setItem(STORAGE_KEY, l)
    } catch {
      /* localStorage may be unavailable; language simply won't persist */
    }
  }, [])

  const t = useCallback(
    (key: string): string => DICTIONARIES[lang]?.[key] ?? DICTIONARIES.en[key] ?? key,
    [lang]
  )

  return (
    <I18nContext.Provider value={{ lang, setLang, t, languages: LANGUAGES }}>
      {children}
    </I18nContext.Provider>
  )
}

export function useI18n(): I18nValue {
  const ctx = useContext(I18nContext)
  if (!ctx) throw new Error('useI18n must be used within an I18nProvider')
  return ctx
}

export type { LangCode }
