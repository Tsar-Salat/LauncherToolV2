import { useEffect, useRef, useState } from 'react'
import { useI18n } from '../i18n'

// Fallen World radio = a YouTube live stream. We intentionally do NOT create the
// player (and therefore make no connection to YouTube) until the user hits Play.
const STREAM_VIDEO_ID = 'H71TRXLkFdA'
const STATION_NAME = 'Fallen World Radio'

/** Load the YouTube IFrame API exactly once, resolving when it's ready. */
let ytApiPromise: Promise<void> | null = null
function ensureYouTubeApi(): Promise<void> {
  const w = window as any
  if (w.YT && w.YT.Player) return Promise.resolve()
  if (ytApiPromise) return ytApiPromise
  ytApiPromise = new Promise<void>((resolve) => {
    const prev = w.onYouTubeIframeAPIReady
    w.onYouTubeIframeAPIReady = () => {
      prev?.()
      resolve()
    }
    const tag = document.createElement('script')
    tag.src = 'https://www.youtube.com/iframe_api'
    document.head.appendChild(tag)
  })
  return ytApiPromise
}

export default function RadioPlayer() {
  const { t } = useI18n()
  const hostRef = useRef<HTMLDivElement | null>(null)
  const playerRef = useRef<any>(null)
  const [playing, setPlaying] = useState(false)
  const [loading, setLoading] = useState(false)
  const [volume, setVolume] = useState(35)

  useEffect(() => {
    // Apply volume changes to a live player.
    if (playerRef.current?.setVolume) playerRef.current.setVolume(volume)
  }, [volume])

  useEffect(() => {
    return () => {
      // Tear down on unmount → closes the stream connection.
      try { playerRef.current?.destroy?.() } catch { /* noop */ }
    }
  }, [])

  const togglePlay = async () => {
    if (playing) {
      playerRef.current?.pauseVideo?.()
      setPlaying(false)
      return
    }

    if (playerRef.current) {
      playerRef.current.playVideo?.()
      setPlaying(true)
      return
    }

    // First play — create the player now (this is the first network hit).
    setLoading(true)
    try {
      await ensureYouTubeApi()
      const w = window as any
      playerRef.current = new w.YT.Player(hostRef.current, {
        videoId: STREAM_VIDEO_ID,
        playerVars: { autoplay: 1, controls: 0, disablekb: 1, playsinline: 1 },
        events: {
          onReady: (e: any) => {
            e.target.setVolume(volume)
            e.target.playVideo()
            setPlaying(true)
            setLoading(false)
          },
          onStateChange: (e: any) => {
            // 1 = playing, 2 = paused, 0 = ended
            if (e.data === 2 || e.data === 0) setPlaying(false)
            if (e.data === 1) setPlaying(true)
          },
          onError: () => {
            setPlaying(false)
            setLoading(false)
          },
        },
      })
    } catch {
      setLoading(false)
      setPlaying(false)
    }
  }

  return (
    <div className="radio">
      <span className="radio__label">{t('topbar.radio')}</span>
      <button
        className={`radio__btn ${playing ? 'is-playing' : ''}`}
        onClick={togglePlay}
        disabled={loading}
        title={playing ? 'Pause' : 'Play'}
        aria-label={playing ? 'Pause radio' : 'Play radio'}
      >
        {loading ? '…' : playing ? '❚❚' : '▶'}
      </button>
      <span className="radio__station" title={STATION_NAME}>{STATION_NAME}</span>
      <input
        className="radio__vol"
        type="range"
        min={0}
        max={100}
        step={1}
        value={volume}
        onChange={(e) => setVolume(parseInt(e.target.value, 10))}
        aria-label="Volume"
      />
      <span className="radio__pct">{volume}%</span>
      {/* Hidden host the IFrame API replaces with the (audio-only) stream */}
      <div className="radio__host" aria-hidden="true"><div ref={hostRef} /></div>
    </div>
  )
}
