import React, { useState } from 'react'
import { SignalingClient } from '../core/signaling'
import { RemotePeerConnection } from '../core/peer-connection'
import { useConnectionStore } from '../stores/connection-store'

const SIGNALING_URL = import.meta.env.VITE_SIGNALING_URL ?? 'ws://localhost:8080'

export default function ConnectionDialog() {
  const [hostId, setHostId] = useState('')
  const [password, setPassword] = useState('')
  const [isConnecting, setIsConnecting] = useState(false)
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [turnUrl, setTurnUrl] = useState(
    import.meta.env.VITE_TURN_URL ?? ''
  )
  const [turnUsername, setTurnUsername] = useState('')
  const [turnCredential, setTurnCredential] = useState('')
  const { error, setError, setConnectionState, setHostId: storeSetHostId, setSessionToken } =
    useConnectionStore()

  const handleConnect = async (e: React.FormEvent) => {
    e.preventDefault()
    if (hostId.length !== 9 || password.length !== 6) return

    setIsConnecting(true)
    setError(null)
    setConnectionState('connecting')

    try {
      const signaling = new SignalingClient(SIGNALING_URL)
      await signaling.connect()
      setConnectionState('authenticating')

      const viewerSessionId = crypto.randomUUID()

      signaling.onConnectionResponse((accepted, token, errorMsg) => {
        if (accepted) {
          setSessionToken(token)
          storeSetHostId(hostId)
          setConnectionState('negotiating')

          const iceServers: RTCIceServer[] = [
            { urls: 'stun:stun.l.google.com:19302' },
            { urls: 'stun:stun1.l.google.com:19302' },
          ]
          if (turnUrl) {
            iceServers.push({
              urls: turnUrl,
              username: turnUsername || undefined,
              credential: turnCredential || undefined,
            })
          }
          const pc = new RemotePeerConnection(signaling, iceServers)
          pc.createOffer(token).catch(console.error)
        } else {
          setError(errorMsg ?? 'Connection rejected')
          setConnectionState('error')
          signaling.disconnect()
        }
        setIsConnecting(false)
      })

      signaling.requestConnect(hostId, password, viewerSessionId)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to connect')
      setConnectionState('error')
      setIsConnecting(false)
    }
  }

  return (
    <div className="min-h-screen bg-gray-900 flex items-center justify-center p-4">
      <div className="bg-gray-800 rounded-2xl shadow-2xl w-full max-w-md p-8">
        <div className="text-center mb-8">
          <div className="w-16 h-16 bg-blue-600 rounded-full flex items-center justify-center mx-auto mb-4">
            <svg className="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
            </svg>
          </div>
          <h1 className="text-2xl font-bold text-white">Remote Work</h1>
          <p className="text-gray-400 mt-1">Connect to a remote computer</p>
        </div>

        <form onSubmit={handleConnect} className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-1">Host ID</label>
            <input
              type="text"
              inputMode="numeric"
              value={hostId}
              onChange={(e) => setHostId(e.target.value.replace(/\D/g, '').slice(0, 9))}
              placeholder="123456789"
              className="w-full bg-gray-700 border border-gray-600 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 text-lg tracking-widest text-center"
              disabled={isConnecting}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-300 mb-1">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value.slice(0, 6))}
              placeholder="••••••"
              className="w-full bg-gray-700 border border-gray-600 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 text-lg tracking-widest text-center"
              disabled={isConnecting}
            />
          </div>

          <div>
            <button
              type="button"
              onClick={() => setShowAdvanced(s => !s)}
              className="text-xs text-gray-500 hover:text-gray-400 transition-colors"
            >
              {showAdvanced ? '\u25B2 Hide Advanced' : '\u25BC Advanced Settings'}
            </button>
            {showAdvanced && (
              <div className="mt-2 space-y-2">
                <div>
                  <label className="block text-xs font-medium text-gray-400 mb-1">
                    TURN Server URL (optional)
                  </label>
                  <input
                    type="text"
                    value={turnUrl}
                    onChange={(e) => setTurnUrl(e.target.value)}
                    placeholder="turn:your-server.com:3478"
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm placeholder-gray-500 focus:outline-none focus:border-blue-500"
                    disabled={isConnecting}
                  />
                </div>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={turnUsername}
                    onChange={(e) => setTurnUsername(e.target.value)}
                    placeholder="TURN username"
                    className="flex-1 bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm placeholder-gray-500 focus:outline-none focus:border-blue-500"
                    disabled={isConnecting}
                  />
                  <input
                    type="password"
                    value={turnCredential}
                    onChange={(e) => setTurnCredential(e.target.value)}
                    placeholder="TURN password"
                    className="flex-1 bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm placeholder-gray-500 focus:outline-none focus:border-blue-500"
                    disabled={isConnecting}
                  />
                </div>
              </div>
            )}
          </div>

          {error && (
            <div className="bg-red-900/50 border border-red-700 rounded-lg px-4 py-3 text-red-300 text-sm">
              {error}
            </div>
          )}

          <button
            type="submit"
            disabled={isConnecting || hostId.length !== 9 || password.length !== 6}
            className="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white font-semibold py-3 rounded-lg transition-colors"
          >
            {isConnecting ? (
              <span className="flex items-center justify-center gap-2">
                <svg className="animate-spin w-4 h-4" viewBox="0 0 24 24" fill="none">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                </svg>
                Connecting...
              </span>
            ) : 'Connect'}
          </button>
        </form>
      </div>
    </div>
  )
}
