import React, { useEffect, useState } from 'react'
import { useConnectionStore } from '../stores/connection-store'

function formatElapsed(ms: number): string {
  const s = Math.floor(ms / 1000)
  if (s < 60) return `${s}s`
  return `${Math.floor(s / 60)}m ${s % 60}s`
}

export default function SessionStatusOverlay() {
  const { connectionState, reconnectingSince, disconnectReason, disconnect, setDisconnectReason } =
    useConnectionStore()
  const [elapsed, setElapsed] = useState(0)

  useEffect(() => {
    if (connectionState !== 'reconnecting' || !reconnectingSince) return
    const id = setInterval(() => {
      setElapsed(Date.now() - reconnectingSince)
    }, 1000)
    return () => clearInterval(id)
  }, [connectionState, reconnectingSince])

  // Reconnecting overlay
  if (connectionState === 'reconnecting') {
    return (
      <div className="absolute inset-0 bg-black/60 flex items-center justify-center z-50">
        <div className="bg-gray-800 rounded-lg p-8 flex flex-col items-center gap-4 shadow-2xl">
          {/* Spinner */}
          <div className="w-12 h-12 border-4 border-gray-600 border-t-blue-400 rounded-full animate-spin" />
          <div className="text-white text-lg font-medium">Reconnecting...</div>
          <div className="text-gray-400 text-sm">Elapsed: {formatElapsed(elapsed)}</div>
          <button
            onClick={disconnect}
            className="mt-2 px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded text-sm transition-colors"
          >
            Disconnect
          </button>
        </div>
      </div>
    )
  }

  // Session expired / disconnect reason dialog
  if (
    connectionState === 'disconnected' &&
    disconnectReason &&
    disconnectReason !== 'user'
  ) {
    const reasonText: Record<string, string> = {
      timeout: 'The connection timed out.',
      network: 'Unable to reconnect after multiple attempts.',
      host_closed: 'The host closed the session.',
    }
    const msg = reasonText[disconnectReason] ?? 'The session ended unexpectedly.'

    return (
      <div className="absolute inset-0 bg-black/70 flex items-center justify-center z-50">
        <div className="bg-gray-800 rounded-lg p-8 flex flex-col items-center gap-4 shadow-2xl max-w-sm w-full mx-4">
          <div className="text-red-400 text-xl font-semibold">Session Ended</div>
          <div className="text-gray-300 text-sm text-center">{msg}</div>
          <div className="flex gap-3 mt-2">
            <button
              onClick={() => setDisconnectReason(null)}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    )
  }

  return null
}
