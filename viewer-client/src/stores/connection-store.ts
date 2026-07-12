import { create } from 'zustand'

export type ConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'authenticating'
  | 'negotiating'
  | 'connected'
  | 'reconnecting'
  | 'error'

export type DisconnectReason = 'timeout' | 'host_closed' | 'network' | 'user' | 'idle_timeout' | null

export interface MonitorInfo {
  index: number
  name: string
  width: number
  height: number
  primary: boolean
}

// Idle timeout: 5 minutes of no input → warning at 30s before expiry
export const IDLE_TIMEOUT_MS = 5 * 60 * 1000
export const IDLE_WARNING_MS = IDLE_TIMEOUT_MS - 30 * 1000

interface ConnectionStore {
  isConnected: boolean
  connectionState: ConnectionState
  sessionToken: string | null
  hostId: string | null
  error: string | null
  signalingWs: WebSocket | null
  peerConnection: RTCPeerConnection | null
  remoteStream: MediaStream | null
  sendInput: ((data: ArrayBuffer) => void) | null
  sendMessage: ((content: string) => void) | null
  sendFile: ((file: File) => void) | null
  isMuted: boolean
  localAudioTrack: MediaStreamTrack | null
  reconnectingSince: number | null
  disconnectReason: DisconnectReason
  lastInputAt: number | null
  idleWarning: boolean
  monitors: MonitorInfo[]
  selectedMonitor: number
  selectMonitor: ((index: number) => void) | null

  setMonitors: (monitors: MonitorInfo[], selected: number) => void
  setSelectedMonitor: (index: number) => void
  setSelectMonitor: (fn: ((index: number) => void) | null) => void
  setConnectionState: (state: ConnectionState) => void
  setSessionToken: (token: string) => void
  setHostId: (id: string) => void
  setError: (error: string | null) => void
  setSignalingWs: (ws: WebSocket | null) => void
  setPeerConnection: (pc: RTCPeerConnection | null) => void
  setRemoteStream: (stream: MediaStream | null) => void
  setSendInput: (fn: ((data: ArrayBuffer) => void) | null) => void
  setSendMessage: (fn: ((content: string) => void) | null) => void
  setSendFile: (fn: ((file: File) => void) | null) => void
  setMuted: (muted: boolean) => void
  setLocalAudioTrack: (track: MediaStreamTrack | null) => void
  setDisconnectReason: (reason: DisconnectReason) => void
  updateLastInput: () => void
  setIdleWarning: (v: boolean) => void
  disconnect: () => void
}

export const useConnectionStore = create<ConnectionStore>((set, get) => ({
  isConnected: false,
  connectionState: 'disconnected',
  sessionToken: null,
  hostId: null,
  error: null,
  signalingWs: null,
  peerConnection: null,
  remoteStream: null,
  sendInput: null,
  sendMessage: null,
  sendFile: null,
  isMuted: false,
  localAudioTrack: null,
  reconnectingSince: null,
  disconnectReason: null,
  lastInputAt: null,
  idleWarning: false,
  monitors: [],
  selectedMonitor: 0,
  selectMonitor: null,

  setMonitors: (monitors, selected) => set({ monitors, selectedMonitor: selected }),
  setSelectedMonitor: (index) => set({ selectedMonitor: index }),
  setSelectMonitor: (fn) => set({ selectMonitor: fn }),
  setConnectionState: (state) =>
    set((prev) => ({
      connectionState: state,
      isConnected: state === 'connected',
      reconnectingSince:
        state === 'reconnecting'
          ? (prev.reconnectingSince ?? Date.now())
          : state === 'connected'
          ? null
          : prev.reconnectingSince,
    })),
  setSessionToken: (token) => set({ sessionToken: token }),
  setHostId: (id) => set({ hostId: id }),
  setError: (error) => set({ error }),
  setSignalingWs: (ws) => set({ signalingWs: ws }),
  setPeerConnection: (pc) => set({ peerConnection: pc }),
  setRemoteStream: (stream) => set({ remoteStream: stream }),
  setSendInput: (fn) => set({ sendInput: fn }),
  setSendMessage: (fn) => set({ sendMessage: fn }),
  setSendFile: (fn) => set({ sendFile: fn }),
  setMuted: (muted) => set({ isMuted: muted }),
  setLocalAudioTrack: (track) => set({ localAudioTrack: track }),
  setDisconnectReason: (reason) => set({ disconnectReason: reason }),
  updateLastInput: () => set({ lastInputAt: Date.now(), idleWarning: false }),
  setIdleWarning: (v) => set({ idleWarning: v }),

  disconnect: () => {
    const { signalingWs, peerConnection, localAudioTrack } = get()
    localAudioTrack?.stop()
    signalingWs?.close()
    peerConnection?.close()
    set({
      isConnected: false,
      connectionState: 'disconnected',
      sessionToken: null,
      signalingWs: null,
      peerConnection: null,
      remoteStream: null,
      sendInput: null,
      sendMessage: null,
      sendFile: null,
      isMuted: false,
      localAudioTrack: null,
      reconnectingSince: null,
      disconnectReason: 'user',
      lastInputAt: null,
      idleWarning: false,
      monitors: [],
      selectedMonitor: 0,
      selectMonitor: null,
    })
  },
}))
