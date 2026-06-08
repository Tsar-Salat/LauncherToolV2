import { useState, useCallback, useEffect } from 'react'

export interface GameState {
  gameRoot: string
  modsFolder: string
  iniFile: string
}

const defaultGameState: GameState = {
  gameRoot: '',
  modsFolder: '',
  iniFile: '',
}

export function useAppState() {
  const [gameState, setGameState] = useState<GameState>(defaultGameState)
  const [initialized, setInitialized] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    // Initialize game state on mount
    const initState = async () => {
      try {
        // This would call Rust backend to detect paths
        setInitialized(true)
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to initialize')
      }
    }

    initState()
  }, [])

  const updateGameState = useCallback((newState: Partial<GameState>) => {
    setGameState((prev) => ({ ...prev, ...newState }))
  }, [])

  return {
    gameState,
    updateGameState,
    initialized,
    error,
  }
}
