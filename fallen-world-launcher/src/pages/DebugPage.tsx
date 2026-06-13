import { useState, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { debugApi } from '../utils/tauriApi'
import '../styles/pages/DebugPage.css'

/** Map a log line to a severity class for colour coding. */
function logLevelClass(line: string): string {
  if (line.includes('[ERROR]')) return 'log-line log-line--error'
  if (line.includes('[WARN]')) return 'log-line log-line--warn'
  if (line.includes('[INFO]')) return 'log-line log-line--info'
  if (line.includes('[DEBUG]')) return 'log-line log-line--debug'
  return 'log-line'
}

export default function DebugPage() {
  const [logs, setLogs] = useState<string[]>([])
  const [diagnosticResult, setDiagnosticResult] = useState('')
  const [loading, setLoading] = useState(false)
  const [logPath, setLogPath] = useState('')
  const logsEndRef = useRef<HTMLDivElement>(null)

  const scrollToBottom = () => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const runDiagnostics = async () => {
    setLoading(true)
    try {
      const result = await invoke<string>('run_diagnostics')
      const path = await invoke<string>('get_log_file_path')
      const allLogs = await invoke<string[]>('get_logs')

      setDiagnosticResult(result)
      setLogs(allLogs)
      setLogPath(path)

      setTimeout(scrollToBottom, 100)
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      setDiagnosticResult(`Error: ${msg}`)
    } finally {
      setLoading(false)
    }
  }

  const exportLogs = async () => {
    const filename = `launcher-logs-${new Date().toISOString().split('T')[0]}.txt`
    try {
      const result = await invoke<{ success: boolean; message: string }>('export_logs', { filename })
      if (result.success) {
        // Extract the path from "Logs exported to:\n<path>" and reveal in Explorer
        const path = result.message.split('\n')[1]?.trim()
        if (path) await debugApi.revealLogFile(path).catch(() => {})
        alert(result.message)
      } else {
        alert(`Export failed: ${result.message}`)
      }
    } catch (err) {
      alert(`Export failed: ${err}`)
    }
  }

  const clearLogs = async () => {
    if (confirm('Clear all logs from memory?')) {
      try {
        await invoke('clear_logs')
        setLogs([])
        setDiagnosticResult('')
        setLogPath('')
      } catch (err) {
        alert(`Clear failed: ${err}`)
      }
    }
  }

  return (
    <div className="page-content debug-page">
      <div className="debug-controls panel">
        <button className="btn btn-primary" onClick={runDiagnostics} disabled={loading}>
          {loading ? 'Running Diagnostics…' : 'Run Full Diagnostics'}
        </button>
        <button className="btn btn-secondary" onClick={exportLogs} disabled={logs.length === 0}>
          Export Logs
        </button>
        <button className="btn btn-secondary" onClick={clearLogs} disabled={logs.length === 0}>
          Clear Logs
        </button>
      </div>

      {logPath && (
        <div className="debug-info panel">
          <span className="debug-info__label">Log File</span>
          <code>{logPath}</code>
        </div>
      )}

      {diagnosticResult && (
        <div className="diagnostic-result panel">
          <h3 className="debug-heading">Diagnostic Results</h3>
          <pre>{diagnosticResult}</pre>
        </div>
      )}

      <div className="logs-container panel">
        <h3 className="debug-heading">
          Live Logs <span className="debug-count">{logs.length}</span>
        </h3>
        <div className="logs-viewer">
          {logs.length === 0 ? (
            <p className="logs-empty">No logs yet. Run diagnostics to begin.</p>
          ) : (
            logs.map((log, idx) => (
              <div key={idx} className={logLevelClass(log)}>
                {log}
              </div>
            ))
          )}
          <div ref={logsEndRef} />
        </div>
      </div>

      <p className="debug-footer">
        Run diagnostics to test all system components and generate debug logs for troubleshooting.
      </p>
    </div>
  )
}
