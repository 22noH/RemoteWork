import { create } from 'zustand'

export type ConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'authenticating'
  | 'negotiating'
  | 'connected'
  | 'reconnecting'
  | 'error'

export type DisconnectReason = 'timeout' | 'host_closed' | 'network' | 'user' | null

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
    })
  },
}))
