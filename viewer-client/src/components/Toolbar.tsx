import React from 'react'
import { useConnectionStore } from '../stores/connection-store'

interface Props {
  onToggleChat: () => void
  onToggleFiles: () => void
}

export default function Toolbar({ onToggleChat, onToggleFiles }: Props) {
  const { hostId, disconnect, isMuted, localAudioTrack, setMuted, connectionState } =
    useConnectionStore()

  const toggleMute = () => {
    if (localAudioTrack) {
      localAudioTrack.enabled = isMuted
      setMuted(!isMuted)
    }
  }

  const dotColor =
    connectionState === 'connected'
      ? 'bg-green-400'
      : connectionState === 'reconnecting'
      ? 'bg-yellow-400 animate-pulse'
      : 'bg-red-400'

  const statusText =
    connectionState === 'connected'
      ? `Connected to ${hostId}`
      : connectionState === 'reconnecting'
      ? 'Reconnecting...'
      : 'Disconnected'

  return (
    <div className="bg-gray-800 border-b border-gray-700 px-4 py-2 flex items-center justify-between">
      <div className="flex items-center gap-3">
        <div className={`w-2 h-2 rounded-full ${dotColor}`} />
        <span className="text-white font-medium text-sm">{statusText}</span>
      </div>
      <div className="flex items-center gap-2">
        {localAudioTrack && (
          <button
            onClick={toggleMute}
            className={`px-3 py-1 rounded text-sm transition-colors ${
              isMuted
                ? 'bg-red-700 hover:bg-red-600 text-white'
                : 'bg-gray-700 hover:bg-gray-600 text-white'
            }`}
          >
            {isMuted ? 'Unmute' : 'Mute'}
          </button>
        )}
        <button
          onClick={onToggleFiles}
          className="px-3 py-1 bg-gray-700 hover:bg-gray-600 text-white rounded text-sm transition-colors"
        >
          Files
        </button>
        <button
          onClick={onToggleChat}
          className="px-3 py-1 bg-gray-700 hover:bg-gray-600 text-white rounded text-sm transition-colors"
        >
          Chat
        </button>
        <button
          onClick={disconnect}
          className="px-3 py-1 bg-red-600 hover:bg-red-700 text-white rounded text-sm transition-colors"
        >
          Disconnect
        </button>
      </div>
    </div>
  )
}
