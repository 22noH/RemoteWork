import { useEffect, useRef } from 'react'
import { useConnectionStore, IDLE_TIMEOUT_MS, IDLE_WARNING_MS } from '../stores/connection-store'

/**
 * Tracks user input inactivity while connected.
 * - At IDLE_WARNING_MS (4m30s): shows idle warning overlay
 * - At IDLE_TIMEOUT_MS (5m):    auto-disconnects with reason 'idle_timeout'
 *
 * The timer resets automatically whenever sendInput() is called (via updateLastInput in store).
 */
export function useIdleTimeout() {
  const { isConnected, lastInputAt, setIdleWarning, disconnect, setDisconnectReason } =
    useConnectionStore()
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (!isConnected) {
      if (timerRef.current) clearTimeout(timerRef.current)
      return
    }

    const baseline = lastInputAt ?? Date.now()
    const elapsed = Date.now() - baseline
    const remainingWarning = IDLE_WARNING_MS - elapsed
    const remainingExpiry = IDLE_TIMEOUT_MS - elapsed

    if (remainingExpiry <= 0) {
      setDisconnectReason('idle_timeout')
      disconnect()
      return
    }

    if (timerRef.current) clearTimeout(timerRef.current)

    if (remainingWarning > 0) {
      // Schedule warning first
      timerRef.current = setTimeout(() => {
        setIdleWarning(true)
        // Then schedule expiry
        timerRef.current = setTimeout(() => {
          setDisconnectReason('idle_timeout')
          disconnect()
        }, IDLE_TIMEOUT_MS - IDLE_WARNING_MS)
      }, remainingWarning)
    } else {
      // Already past warning threshold — show it and schedule expiry
      setIdleWarning(true)
      timerRef.current = setTimeout(() => {
        setDisconnectReason('idle_timeout')
        disconnect()
      }, remainingExpiry)
    }

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current)
    }
  }, [isConnected, lastInputAt])
}
