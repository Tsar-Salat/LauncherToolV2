import { useState, useEffect, useRef, useCallback } from 'react'
import '../styles/pages/RobCoTerminal.css'

interface Props {
  onComplete: () => void
}

type Phase = 'gif' | 'boot' | 'hack' | 'matrix' | 'welcome'

// ── Static content ───────────────────────────────────────────────────────────

const BOOT_LINES = [
  'ROBCO INDUSTRIES (TM) TERMLINK PROTOCOL',
  'COPYRIGHT 2075-2077 ROBCO INDUSTRIES',
  '',
  '> BIOS v4.7.7 INITIALIZED',
  '> CHECKING HARDWARE INTEGRITY ........... [OK]',
  '> MOUNTING SECURE FILESYSTEMS ........... [OK]',
  '> LOADING KERNEL MODULES ................ [OK]',
  '> STARTING NETWORK DAEMON ............... [OK]',
  '> VERIFYING VAULT-TEC CREDENTIALS ....... [OK]',
  '> CALIBRATING GEIGER ARRAY .............. [OK]',
  '',
  '> ALL SYSTEMS NOMINAL',
  '> ESTABLISHING ENCRYPTED MAINFRAME LINK...',
]

const HACK_LINES = [
  '> INTERCEPTING HANDSHAKE SEQUENCE...',
  '> INJECTING EXPLOIT PAYLOAD v3.1...',
  '> ANALYZING ENCRYPTION PROTOCOL......',
  '> BYPASSING SECURITY LAYER 1/3.......',
  '> BYPASSING SECURITY LAYER 2/3.......',
  '> BYPASSING SECURITY LAYER 3/3.......',
  '> DECRYPTION COMPLETE',
  '> ADMINISTRATOR OVERRIDE ACCEPTED',
  '> *** ACCESS GRANTED ***',
]

const MATRIX_CHARS =
  'アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホ' +
  '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!@#$%^&*<>?/'

const GIF_URL =
  'https://media3.giphy.com/media/v1.Y2lkPTc5MGI3NjExZHd5b2V3dGZvajk3YnA0cXc4MTlidTQzZWxjZGgwZXFqdmFwMWFwNSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/Lny6Rw04nsOOc/giphy.gif'

// ── Helpers ───────────────────────────────────────────────────────────────────

function randHexDump(): string {
  return Array.from({ length: 3 }, (_, r) => {
    const addr = (0x3f00 + r * 0x10).toString(16).toUpperCase().padStart(4, '0')
    const bytes = Array.from({ length: 8 }, () =>
      Math.floor(Math.random() * 256).toString(16).toUpperCase().padStart(2, '0')
    ).join(' ')
    return `0x${addr}  ${bytes}`
  }).join('\n')
}

// ── Component ─────────────────────────────────────────────────────────────────

export default function RobCoTerminal({ onComplete }: Props) {
  const [phase, setPhase]         = useState<Phase>('gif')
  const [fading, setFading]       = useState(false)
  const [bootLines, setBootLines] = useState<string[]>([])
  const [hackCount, setHackCount] = useState(0)   // render HACK_LINES.slice(0, hackCount)
  const [hackPct, setHackPct]     = useState(0)
  const [garbage, setGarbage]     = useState('')
  const [wStep, setWStep]         = useState(0)
  const [lineFlash, setLineFlash] = useState(false) // brief screen flash on each line reveal

  const canvasRef   = useRef<HTMLCanvasElement>(null)
  const bootRef     = useRef<HTMLDivElement>(null)
  const skippedRef  = useRef(false)
  const musicRef    = useRef<HTMLAudioElement | null>(null)
  const typingSndRef = useRef<HTMLAudioElement | null>(null)

  // ── Audio setup — start music immediately on mount ─────────────────────
  useEffect(() => {
    const music = new Audio('/audio/openmusic.mp3')
    music.volume = 0.32
    music.loop = false
    music.play().catch(() => {/* autoplay blocked — silent fail */})
    musicRef.current = music

    const typing = new Audio('/audio/textdigital.mp3')
    typing.volume = 0.4
    typingSndRef.current = typing

    return () => {
      music.pause()
      music.src = ''
    }
  }, [])

  // Each line reveal: plays the typing sound + triggers a brief screen flash
  const onLineReveal = useCallback(() => {
    const s = typingSndRef.current
    if (s) { s.currentTime = 0; s.play().catch(() => {}) }
    // Brief screen flash to visualise the "received text" burst
    setLineFlash(true)
    setTimeout(() => setLineFlash(false), 80)
  }, [])

  // ── Skip ──────────────────────────────────────────────────────────────────
  const skip = useCallback(() => {
    if (skippedRef.current) return
    skippedRef.current = true
    if (musicRef.current) {
      // Fade music out over 600 ms
      const m = musicRef.current
      const fade = setInterval(() => {
        m.volume = Math.max(0, m.volume - 0.07)
        if (m.volume <= 0) { clearInterval(fade); m.pause() }
      }, 40)
    }
    setFading(true)
    setTimeout(onComplete, 700)
  }, [onComplete])

  // ── PHASE: GIF — 5 000 ms ─────────────────────────────────────────────────
  useEffect(() => {
    if (phase !== 'gif') return
    const t = setTimeout(() => setPhase('boot'), 5000)
    return () => clearTimeout(t)
  }, [phase])

  // ── PHASE: BOOT — ~6 000 ms ──────────────────────────────────────────────
  // Delay schedule (post-idx++, so idx is already the NEXT line's index):
  //   idx ≤ 2  → 120 ms (header lines)
  //   empty    → 60 ms
  //   idx ≥ 12 → 350 ms (final two lines)
  //   else     → 550 ms (status lines with [OK])
  //   end      → 1 000 ms before hack
  useEffect(() => {
    if (phase !== 'boot') return
    let cancelled = false
    let idx = 0

    const step = () => {
      if (cancelled) return
      const line = BOOT_LINES[idx]
      idx++
      setBootLines(prev => [...prev, line])
      onLineReveal()

      if (idx < BOOT_LINES.length) {
        const delay = line === '' ? 60 : idx <= 2 ? 120 : idx >= 12 ? 350 : 550
        setTimeout(step, delay)
      } else {
        setTimeout(() => { if (!cancelled) setPhase('hack') }, 1000)
      }
    }

    const t = setTimeout(step, 400)
    return () => { cancelled = true; clearTimeout(t) }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase])

  // Auto-scroll boot output
  useEffect(() => {
    if (bootRef.current) bootRef.current.scrollTop = bootRef.current.scrollHeight
  }, [bootLines])

  // ── PHASE: HACK — ~5.8 s ─────────────────────────────────────────────────
  // 300 ms init + 8 × 560 ms intervals + 1 000 ms end = 5 780 ms
  useEffect(() => {
    if (phase !== 'hack') return
    let cancelled = false

    const garbageId = setInterval(() => setGarbage(randHexDump()), 60)
    const progressId = setInterval(() => setHackPct(p => (p >= 100 ? 100 : p + 1)), 30)

    let count = 0
    const addLine = () => {
      if (cancelled || count >= HACK_LINES.length) return
      count++
      setHackCount(count)
      onLineReveal()
      if (count < HACK_LINES.length) {
        setTimeout(addLine, 560)
      } else {
        setTimeout(() => {
          if (!cancelled) { clearInterval(garbageId); setPhase('matrix') }
        }, 1000)
      }
    }
    const t = setTimeout(addLine, 300)

    return () => {
      cancelled = true
      clearInterval(garbageId)
      clearInterval(progressId)
      clearTimeout(t)
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase])

  // ── PHASE: MATRIX — 5 000 ms ─────────────────────────────────────────────
  useEffect(() => {
    if (phase !== 'matrix') return
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const W = canvas.parentElement?.clientWidth ?? window.innerWidth
    const H = canvas.parentElement?.clientHeight ?? window.innerHeight
    canvas.width = W
    canvas.height = H

    const FS = 18
    const cols = Math.floor(W / FS)
    const drops = Array.from({ length: cols }, () => Math.random() * -60)
    let cancelled = false

    const draw = () => {
      if (cancelled) return
      ctx.fillStyle = 'rgba(0, 4, 0, 0.055)'
      ctx.fillRect(0, 0, W, H)
      ctx.font = `${FS}px "Share Tech Mono", monospace`
      for (let i = 0; i < cols; i++) {
        const ch = MATRIX_CHARS[Math.floor(Math.random() * MATRIX_CHARS.length)]
        const inBounds = drops[i] * FS > 0 && drops[i] * FS < H
        ctx.fillStyle = inBounds ? '#c7f06a' : '#1e5c08'
        ctx.fillText(ch, i * FS, drops[i] * FS)
        if (drops[i] * FS > H && Math.random() > 0.975) drops[i] = 0
        drops[i] += 0.65
      }
    }

    let rafId = 0
    const loop = () => { draw(); rafId = requestAnimationFrame(loop) }
    rafId = requestAnimationFrame(loop)

    const t = setTimeout(() => {
      cancelled = true
      cancelAnimationFrame(rafId)
      setPhase('welcome')
    }, 5000)
    return () => { cancelled = true; cancelAnimationFrame(rafId); clearTimeout(t) }
  }, [phase])

  // ── PHASE: WELCOME — ~8 000 ms ───────────────────────────────────────────
  useEffect(() => {
    if (phase !== 'welcome') return
    const timers: ReturnType<typeof setTimeout>[] = []
    const q = (fn: () => void, ms: number) => timers.push(setTimeout(fn, ms))

    q(() => setWStep(1), 100)
    q(() => setWStep(2), 900)
    q(() => setWStep(3), 1800)
    q(() => setWStep(4), 3000)
    q(() => setFading(true), 7000)
    q(onComplete, 8000)

    const handler = (e: KeyboardEvent) => {
      if (['Enter', ' ', 'Escape', 'ArrowRight'].includes(e.key)) skip()
    }
    window.addEventListener('keydown', handler)

    return () => {
      timers.forEach(clearTimeout)
      window.removeEventListener('keydown', handler)
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase, onComplete])

  // ── Render ────────────────────────────────────────────────────────────────

  const visibleHackLines = HACK_LINES.slice(0, hackCount)

  return (
    <div className={`rct${fading ? ' rct--fade' : ''}${lineFlash ? ' rct--line-flash' : ''}`}>
      <button className="rct__skip" onClick={skip}>SKIP ›</button>

      {/* ── GIF ── */}
      {phase === 'gif' && (
        <div className="rct__gif" onClick={() => setPhase('boot')}>
          <img
            className="rct__gif-img"
            src={GIF_URL}
            alt="Fallen World"
            onError={() => setPhase('boot')}
          />
          <p className="rct__gif-hint">[ CLICK TO SKIP ]</p>
        </div>
      )}

      {/* ── BOOT ── */}
      {phase === 'boot' && (
        <div className="rct__screen" ref={bootRef}>
          {bootLines.map((line, i) => (
            <div
              key={i}
              className={[
                'rct__line',
                line === '' ? 'rct__line--blank' : '',
                i === bootLines.length - 1 ? 'rct__line--cursor' : '',
              ].join(' ')}
            >
              {line || ' '}
            </div>
          ))}
        </div>
      )}

      {/* ── HACK ── */}
      {phase === 'hack' && (
        <div className="rct__screen rct__screen--hack">
          <pre className="rct__garbage">{garbage}</pre>
          <div className="rct__bar-wrap">
            <div className="rct__bar-label">DECRYPTION PROGRESS</div>
            <div className="rct__bar-track">
              <div className="rct__bar-fill" style={{ width: `${hackPct}%` }} />
              <span className="rct__bar-pct">{hackPct}%</span>
            </div>
          </div>
          <div className="rct__hack-lines">
            {visibleHackLines.map((line, i) => (
              <div
                key={i}
                className={`rct__line${
                  line.includes('GRANTED') || line.includes('***') || line.includes('COMPLETE')
                    ? ' rct__line--bright'
                    : ''
                }`}
              >
                {line}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* ── MATRIX ── */}
      {phase === 'matrix' && (
        <div className="rct__matrix">
          <canvas ref={canvasRef} className="rct__canvas" />
          <div className="rct__matrix-overlay">
            <span className="rct__matrix-word" style={{ animationDelay: '1.4s' }}>FALLEN</span>
            <span className="rct__matrix-word" style={{ animationDelay: '2.1s' }}>WORLD</span>
          </div>
        </div>
      )}

      {/* ── WELCOME ── */}
      {phase === 'welcome' && (
        <div className="rct__welcome" onClick={skip}>
          {wStep >= 1 && <div className="rct__wl-symbol">☢</div>}
          {wStep >= 2 && <h1 className="rct__wl-title">FALLEN WORLD</h1>}
          {wStep >= 3 && <p className="rct__wl-sub">WELCOME, SURVIVOR</p>}
          {wStep >= 4 && <p className="rct__wl-quote">&ldquo;Kept you waiting, huh?&rdquo;</p>}
          {wStep >= 4 && <p className="rct__wl-hint">[ PRESS ENTER OR CLICK TO CONTINUE ]</p>}
        </div>
      )}

      {/* CRT scanlines — sits above all phase content */}
      <div className="rct__scanlines" />
    </div>
  )
}
