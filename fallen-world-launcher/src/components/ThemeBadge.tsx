import enclaveBadge from '../Icons/enclave.webp'
import { useTheme } from '../theme'
import '../styles/ThemeBadge.css'

/** A faction crest shown in the sidebar footer that swaps with the active theme.
 *  The default (Fallen World) uses the radiation trefoil we shipped originally;
 *  Enclave uses the supplied badge artwork. Glyph crests recolour with the
 *  theme accent. Drop extra image badges into src/Icons and reference them in
 *  the `img` field below to upgrade any theme from a glyph to artwork. */
const EMBLEM: Record<string, { glyph?: string; img?: string; label: string }> = {
  fallout: { glyph: '☢', label: 'Fallen World' },
  amber: { glyph: '☣', label: 'Wasteland Signal' },
  vault: { glyph: '⚙', label: 'Vault-Tec' },
  enclave: { img: enclaveBadge, label: 'Enclave' },
  institute: { glyph: '⚛', label: 'The Institute' },
}

export default function ThemeBadge() {
  const { theme } = useTheme()
  const emblem = EMBLEM[theme] ?? EMBLEM.fallout
  return (
    <div className="theme-badge" title={emblem.label}>
      {emblem.img ? (
        <img className="theme-badge__img" src={emblem.img} alt={emblem.label} />
      ) : (
        <span className="theme-badge__crest" aria-label={emblem.label}>
          {emblem.glyph}
        </span>
      )}
    </div>
  )
}
