import React, { useRef, useEffect, useCallback } from 'react'
import { useConnectionStore } from '../stores/connection-store'
import { InputSender } from '../core/input-sender'

export default function RemoteScreen() {
  const videoRef = useRef<HTMLVideoElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const inputSenderRef = useRef<InputSender | null>(null)

  const { remoteStream, sendInput } = useConnectionStore()

  // Attach/detach video stream
  useEffect(() => {
    if (videoRef.current) {
      videoRef.current.srcObject = remoteStream
    }
  }, [remoteStream])

  // Stable sendFn that always reads the latest sendInput from the store
  const stableSendFn = useCallback((data: ArrayBuffer) => {
    useConnectionStore.getState().sendInput?.(data)
  }, [])

  // Create InputSender once and keep it alive for the component lifetime
  useEffect(() => {
    inputSenderRef.current = new InputSender(stableSendFn)
    return () => {
      inputSenderRef.current?.detach()
      inputSenderRef.current = null
    }
  }, [stableSendFn])

  // Attach/detach input capturing when the container mounts or sendInput changes
  useEffect(() => {
    const container = containerRef.current
    const sender = inputSenderRef.current
    if (!container || !sender) return

    if (sendInput) {
      sender.attach(container)
    } else {
      sender.detach()
    }

    return () => {
      sender.detach()
    }
  }, [sendInput])

  return (
    <div
      ref={containerRef}
      className="flex-1 bg-black flex items-center justify-center overflow-hidden"
      // Make div focusable so keyboard events are received
      tabIndex={0}
      style={{ outline: 'none', cursor: sendInput ? 'none' : 'default' }}
    >
      <video
        ref={videoRef}
        autoPlay
        playsInline
        muted={false}
        className="max-w-full max-h-full block"
        // Prevent default drag behavior on the video element
        onDragStart={(e) => e.preventDefault()}
      />
    </div>
  )
}
