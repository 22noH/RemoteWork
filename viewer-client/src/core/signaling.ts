import { useConnectionStore } from '../stores/connection-store'

// Phase 1 uses JSON over WebSocket for ease of development.
// Phase 5 will switch to Protobuf binary encoding.

export interface SignalingMessage {
  type:
    | 'register_host'
    | 'register_ack'
    | 'connect_request'
    | 'connect_response'
    | 'incoming_connection'
    | 'sdp_offer'
    | 'sdp_answer'
    | 'ice_candidate'
    | 'error'
    | 'ping'
    | 'pong'
  payload: Record<string, unknown>
}

export class SignalingClient {
  private ws: WebSocket | null = null
  private onSdpAnswerCb?: (sdp: string, sessionToken: string) => void
  private onIceCandidateCb?: (candidate: RTCIceCandidateInit, sessionToken: string) => void
  private onConnectResponseCb?: (accepted: boolean, sessionToken: string, error?: string) => void

  private shouldReconnect = true
  private reconnectAttempts = 0
  private maxReconnectAttempts = 10
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private pingInterval: ReturnType<typeof setInterval> | null = null

  constructor(private serverUrl: string) {}

  connect(): Promise<void> {
    return this.connectInternal()
  }

  private connectInternal(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.serverUrl)
      this.ws.binaryType = 'arraybuffer'

      this.ws.onopen = () => {
        console.log('[Signaling] Connected to', this.serverUrl)
        this.reconnectAttempts = 0
        useConnectionStore.getState().setSignalingWs(this.ws!)

        // Start 30s ping keepalive
        this.clearPing()
        this.pingInterval = setInterval(() => {
          this.send({ type: 'ping', payload: { timestamp_ms: Date.now() } })
        }, 30000)

        resolve()
      }

      this.ws.onerror = () => {
        reject(new Error(`Cannot connect to signaling server at ${this.serverUrl}`))
      }

      this.ws.onclose = () => {
        console.log('[Signaling] Disconnected')
        this.clearPing()

        if (this.shouldReconnect && this.reconnectAttempts < this.maxReconnectAttempts) {
          const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000)
          this.reconnectAttempts++
          console.log(`[Signaling] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`)
          useConnectionStore.getState().setConnectionState('reconnecting')
          this.reconnectTimer = setTimeout(() => {
            this.connectInternal().catch((err) => {
              console.error('[Signaling] Reconnect failed:', err)
            })
          }, delay)
        } else {
          useConnectionStore.getState().setDisconnectReason('network')
          useConnectionStore.getState().setConnectionState('disconnected')
        }
      }

      this.ws.onmessage = (event) => {
        this.handleRaw(event.data)
      }
    })
  }

  private clearPing() {
    if (this.pingInterval !== null) {
      clearInterval(this.pingInterval)
      this.pingInterval = null
    }
  }

  private handleRaw(data: ArrayBuffer | string) {
    try {
      const text = data instanceof ArrayBuffer
        ? new TextDecoder().decode(data)
        : (data as string)
      const msg = JSON.parse(text) as SignalingMessage
      this.dispatch(msg)
    } catch (err) {
      console.error('[Signaling] Parse error', err)
    }
  }

  private dispatch(msg: SignalingMessage) {
    switch (msg.type) {
      case 'connect_response': {
        const p = msg.payload as { accepted: boolean; session_token: string; error_message?: string }
        this.onConnectResponseCb?.(p.accepted, p.session_token, p.error_message)
        break
      }
      case 'sdp_answer': {
        const p = msg.payload as { sdp: string; session_token: string }
        this.onSdpAnswerCb?.(p.sdp, p.session_token)
        break
      }
      case 'ice_candidate': {
        const p = msg.payload as {
          candidate: string
          sdp_mid: string
          sdp_mline_index: number
          session_token: string
        }
        this.onIceCandidateCb?.(
          { candidate: p.candidate, sdpMid: p.sdp_mid, sdpMLineIndex: p.sdp_mline_index },
          p.session_token
        )
        break
      }
      case 'error': {
        const p = msg.payload as { message: string }
        console.error('[Signaling] Server error:', p.message)
        useConnectionStore.getState().setError(p.message)
        break
      }
    }
  }

  private send(msg: SignalingMessage) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg))
    }
  }

  requestConnect(targetHostId: string, password: string, viewerSessionId: string) {
    this.send({
      type: 'connect_request',
      payload: {
        target_host_id: targetHostId,
        password: password,
        viewer_session_id: viewerSessionId,
      },
    })
  }

  sendSdpOffer(sdp: string, sessionToken: string) {
    this.send({ type: 'sdp_offer', payload: { sdp, session_token: sessionToken } })
  }

  sendSdpAnswer(sdp: string, sessionToken: string) {
    this.send({ type: 'sdp_answer', payload: { sdp, session_token: sessionToken } })
  }

  sendIceCandidate(candidate: RTCIceCandidateInit, sessionToken: string) {
    this.send({
      type: 'ice_candidate',
      payload: {
        candidate: candidate.candidate,
        sdp_mid: candidate.sdpMid,
        sdp_mline_index: candidate.sdpMLineIndex,
        session_token: sessionToken,
      },
    })
  }

  onAnswer(cb: (sdp: string, sessionToken: string) => void) {
    this.onSdpAnswerCb = cb
  }

  onRemoteIce(cb: (candidate: RTCIceCandidateInit, sessionToken: string) => void) {
    this.onIceCandidateCb = cb
  }

  onConnectionResponse(cb: (accepted: boolean, sessionToken: string, error?: string) => void) {
    this.onConnectResponseCb = cb
  }

  disconnect() {
    this.shouldReconnect = false
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    this.clearPing()
    this.ws?.close()
    this.ws = null
  }
}
