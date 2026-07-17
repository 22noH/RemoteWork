import { SignalingClient } from './signaling'
import { RemotePeerConnection } from './peer-connection'
import { useConnectionStore } from '../stores/connection-store'

const MAX_RECONNECT_ATTEMPTS = 5

/**
 * Owns the connect/reconnect lifecycle so a dropped session re-establishes
 * itself (fresh connect_request + offer) without the user re-entering
 * credentials. Lives outside React components, which unmount once connected.
 *
 * The host supports a short "grace" on reconnect (no re-approval, replaces the
 * dead session), so a network blip re-heals seamlessly within the attempt budget.
 */
class ConnectionManager {
  private serverUrl = ''
  private iceServers: RTCIceServer[] = []
  private hostId = ''
  private password = ''
  private signaling: SignalingClient | null = null
  private pc: RemotePeerConnection | null = null
  private attempts = 0
  private stopped = true
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null

  /** Initial connect. Resolves when accepted; rejects on rejection/failure. */
  async start(
    hostId: string,
    password: string,
    serverUrl: string,
    iceServers: RTCIceServer[],
  ): Promise<void> {
    this.hostId = hostId
    this.password = password
    this.serverUrl = serverUrl
    this.iceServers = iceServers
    this.stopped = false
    this.attempts = 0
    await this.attempt('authenticating')
  }

  private async attempt(state: 'authenticating' | 'reconnecting'): Promise<void> {
    this.teardown()
    if (this.stopped) return

    const store = useConnectionStore.getState()
    store.setConnectionState(state)

    const signaling = new SignalingClient(this.serverUrl)
    signaling.setAutoReconnect(false)
    signaling.onClosed(() => this.onLost())
    this.signaling = signaling

    await signaling.connect() // rejects if the server is unreachable

    const viewerSessionId = crypto.randomUUID()

    await new Promise<void>((resolve, reject) => {
      signaling.onConnectionResponse((accepted, token, errorMsg) => {
        if (accepted) {
          store.setSessionToken(token)
          store.setHostId(this.hostId)
          store.setConnectionState('negotiating')
          const pc = new RemotePeerConnection(signaling, this.iceServers, () => this.onLost())
          this.pc = pc
          pc.createOffer(token).catch((e) => console.error('[Manager] offer failed:', e))
          this.attempts = 0
          resolve()
        } else {
          // A rejection (bad password / host offline / busy) is terminal.
          this.stopped = true
          store.setError(errorMsg ?? 'Connection rejected')
          store.setConnectionState('error')
          this.teardown()
          reject(new Error(errorMsg ?? 'rejected'))
        }
      })
      signaling.requestConnect(this.hostId, this.password, viewerSessionId)
    })
  }

  /** A live session was lost (media path failed or signaling closed). */
  private onLost(): void {
    if (this.stopped) return
    const store = useConnectionStore.getState()
    if (this.attempts >= MAX_RECONNECT_ATTEMPTS) {
      this.stopped = true
      store.setDisconnectReason('network')
      store.setConnectionState('disconnected')
      this.teardown()
      return
    }
    this.attempts++
    store.setConnectionState('reconnecting')
    const delay = Math.min(1000 * this.attempts, 5000)
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer)
    this.reconnectTimer = setTimeout(() => {
      // A failed reconnect attempt loops back through onLost until the budget
      // runs out.
      this.attempt('reconnecting').catch(() => this.onLost())
    }, delay)
  }

  /** User- or idle-initiated end; no reconnect. */
  stop(): void {
    this.stopped = true
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    this.teardown()
  }

  private teardown(): void {
    this.pc?.close()
    this.pc = null
    this.signaling?.disconnect()
    this.signaling = null
  }
}

export const connectionManager = new ConnectionManager()
