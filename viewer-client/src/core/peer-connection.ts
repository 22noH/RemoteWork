import { SignalingClient } from './signaling'
import { ChatManager } from './chat'
import { FileTransferManager } from './file-transfer'
import { useConnectionStore } from '../stores/connection-store'

export class RemotePeerConnection {
  private pc: RTCPeerConnection
  private signaling: SignalingClient
  private sessionToken = ''
  private inputChannel: RTCDataChannel | null = null
  private chatChannel: RTCDataChannel | null = null
  private fileChannel: RTCDataChannel | null = null
  private controlChannel: RTCDataChannel | null = null
  private chatManager: ChatManager | null = null
  private fileManager: FileTransferManager | null = null

  private onStreamCb?: (stream: MediaStream) => void

  constructor(
    signaling: SignalingClient,
    iceServers: RTCIceServer[],
    private onConnectionLost?: () => void,
  ) {
    this.signaling = signaling
    this.pc = new RTCPeerConnection({ iceServers })
    useConnectionStore.getState().setPeerConnection(this.pc)

    // Incoming media track (host screen)
    this.pc.ontrack = (event) => {
      if (event.streams[0]) {
        this.onStreamCb?.(event.streams[0])
        useConnectionStore.getState().setRemoteStream(event.streams[0])
      }
    }

    // ICE candidate gathering
    this.pc.onicecandidate = (event) => {
      if (event.candidate) {
        this.signaling.sendIceCandidate(event.candidate.toJSON(), this.sessionToken)
      }
    }

    // Recovery is owned by ConnectionManager: on a real loss we ask it to
    // re-establish a fresh session. (The host doesn't support in-place ICE
    // restart, so a clean reconnect is the reliable path.)
    let lostFired = false
    const fireLost = () => {
      if (lostFired) return
      lostFired = true
      this.onConnectionLost?.()
    }
    this.pc.onconnectionstatechange = () => {
      const state = this.pc.connectionState
      console.log('[PeerConnection] State:', state)
      switch (state) {
        case 'connected':
          useConnectionStore.getState().setConnectionState('connected')
          break
        case 'disconnected':
          // Brief blips self-recover; give a short grace before reconnecting.
          useConnectionStore.getState().setConnectionState('reconnecting')
          setTimeout(() => {
            if (
              this.pc.connectionState === 'disconnected' ||
              this.pc.connectionState === 'failed'
            ) {
              fireLost()
            }
          }, 6000)
          break
        case 'failed':
          fireLost()
          break
        case 'closed':
          useConnectionStore.getState().setSendInput(null)
          useConnectionStore.getState().setSendMessage(null)
          useConnectionStore.getState().setSendFile(null)
          break
      }
    }

    // Register signaling handlers
    this.signaling.onAnswer((sdp, token) => {
      this.sessionToken = token
      this.pc.setRemoteDescription({ type: 'answer', sdp })
        .catch(e => console.error('setRemoteDescription failed:', e))
    })

    this.signaling.onRemoteIce((candidate) => {
      this.pc.addIceCandidate(candidate)
        .catch(e => console.error('addIceCandidate failed:', e))
    })
  }

  async createOffer(sessionToken: string): Promise<void> {
    this.sessionToken = sessionToken

    // Receive the host's screen video. The host is the sender/answerer, so the
    // offer must carry a recvonly video m-line or the track is never negotiated
    // (ontrack never fires and the screen stays black).
    this.pc.addTransceiver('video', { direction: 'recvonly' })

    // Request microphone (non-blocking, don't fail if denied)
    try {
      const audioStream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true },
        video: false,
      })
      const audioTrack = audioStream.getAudioTracks()[0]
      if (audioTrack) {
        this.pc.addTrack(audioTrack, audioStream)
        useConnectionStore.getState().setLocalAudioTrack(audioTrack)
      }
    } catch (e) {
      console.warn('[PeerConnection] Microphone not available:', e)
    }

    // Create WebRTC data channels
    this.inputChannel = this.pc.createDataChannel('input', { ordered: true })
    this.chatChannel = this.pc.createDataChannel('chat', { ordered: true })
    this.fileChannel = this.pc.createDataChannel('file', { ordered: true })
    this.controlChannel = this.pc.createDataChannel('control', { ordered: true })

    // Set binary type to arraybuffer for consistent handling
    this.chatChannel.binaryType = 'arraybuffer'
    this.fileChannel.binaryType = 'arraybuffer'
    this.controlChannel.binaryType = 'arraybuffer'

    // Control channel: receive the monitor list, send monitor selection.
    this.controlChannel.onopen = () => {
      console.log('[PeerConnection] Control channel open')
      useConnectionStore.getState().setSelectMonitor((index: number) => {
        this.controlChannel?.send(JSON.stringify({ type: 'select_monitor', index }))
      })
    }
    this.controlChannel.onmessage = (e) => {
      try {
        const text =
          typeof e.data === 'string' ? e.data : new TextDecoder().decode(e.data)
        const msg = JSON.parse(text)
        if (msg.type === 'monitors' && Array.isArray(msg.list)) {
          useConnectionStore.getState().setMonitors(msg.list, msg.selected ?? 0)
        }
      } catch (err) {
        console.warn('[PeerConnection] Bad control message:', err)
      }
    }
    this.controlChannel.onclose = () => {
      useConnectionStore.getState().setSelectMonitor(null)
    }

    // Input channel lifecycle
    this.inputChannel.onopen = () => {
      console.log('[PeerConnection] Input channel open')
      useConnectionStore.getState().setSendInput((data: ArrayBuffer) => this.sendInput(data))
    }
    this.inputChannel.onclose = () => {
      console.log('[PeerConnection] Input channel closed')
      useConnectionStore.getState().setSendInput(null)
    }

    // Chat channel lifecycle
    this.chatChannel.onopen = () => {
      console.log('[PeerConnection] Chat channel open')
      const manager = new ChatManager((data) => this.chatChannel!.send(data))
      this.chatManager = manager
      useConnectionStore.getState().setSendMessage((content) => manager.sendMessage(content))
    }
    this.chatChannel.onmessage = (e) => {
      this.chatManager?.handleIncoming(e.data)
    }
    this.chatChannel.onclose = () => {
      console.log('[PeerConnection] Chat channel closed')
      useConnectionStore.getState().setSendMessage(null)
      this.chatManager = null
    }

    // File channel lifecycle
    this.fileChannel.onopen = () => {
      console.log('[PeerConnection] File channel open')
      const fileManager = new FileTransferManager((data) => this.fileChannel!.send(data))
      this.fileManager = fileManager
      useConnectionStore.getState().setSendFile((file: File) =>
        fileManager.sendFile(file).catch(console.error),
      )
    }
    this.fileChannel.onmessage = (e) => {
      this.fileManager?.handleIncoming(e.data)
    }
    this.fileChannel.onclose = () => {
      console.log('[PeerConnection] File channel closed')
      useConnectionStore.getState().setSendFile(null)
      this.fileManager = null
    }

    const offer = await this.pc.createOffer()
    await this.pc.setLocalDescription(offer)
    this.signaling.sendSdpOffer(offer.sdp!, sessionToken)
  }

  sendInput(data: ArrayBuffer): void {
    if (this.inputChannel?.readyState === 'open') {
      this.inputChannel.send(data)
      useConnectionStore.getState().updateLastInput()
    }
  }

  onStream(cb: (stream: MediaStream) => void) { this.onStreamCb = cb }

  close(): void {
    useConnectionStore.getState().setSendInput(null)
    useConnectionStore.getState().setSendMessage(null)
    useConnectionStore.getState().setSendFile(null)
    this.pc.close()
  }
}
