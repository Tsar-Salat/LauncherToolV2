import { useState, useCallback, useRef, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'

interface UseCommandReturn<T> {
  data: T | null
  loading: boolean
  error: string | null
  execute: (...args: any[]) => Promise<T>
}

export function useTauriCommand<T>(
  command: string,
  initialData: T | null = null
): UseCommandReturn<T> {
  const [data, setData] = useState<T | null>(initialData)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const isMountedRef = useRef(true)

  useEffect(() => {
    return () => {
      isMountedRef.current = false
    }
  }, [])

  const execute = useCallback(
    async (...args: any[]): Promise<T> => {
      if (!isMountedRef.current) {
        throw new Error('Component unmounted')
      }

      setLoading(true)
      setError(null)
      try {
        const result = await invoke<T>(command, args.length === 1 ? args[0] : {})
        if (isMountedRef.current) {
          setData(result)
        }
        return result
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err)
        if (isMountedRef.current) {
          setError(errorMsg)
        }
        throw err
      } finally {
        if (isMountedRef.current) {
          setLoading(false)
        }
      }
    },
    [command]
  )

  return { data, loading, error, execute }
}
