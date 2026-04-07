import React, { useState } from 'react'
import ConnectionDialog from './components/ConnectionDialog'
import RemoteScreen from './components/RemoteScreen'
import Toolbar from './components/Toolbar'
import ChatPanel from './components/ChatPanel'
import FileTransfer from './components/FileTransfer'
import UpdateBanner from './components/UpdateBanner'
import SessionStatusOverlay from './components/SessionStatusOverlay'
import { useConnectionStore } from './stores/connection-store'
import { useIdleTimeout } from './hooks/useIdleTimeout'

function App() {
  const { isConnected, connectionState } = useConnectionStore()
  useIdleTimeout()
  const [showChat, setShowChat] = useState(false)
  const [showFiles, setShowFiles] = useState(false)

  // Show connected view during reconnecting too (overlay goes on top)
  const showConnectedView = isConnected || connectionState === 'reconnecting'

  if (!showConnectedView) {
    return (
      <>
        <UpdateBanner />
        <ConnectionDialog />
      </>
    )
  }

  return (
    <div className="flex flex-col h-screen bg-gray-900 relative">
      <UpdateBanner />
      <Toolbar
        onToggleChat={() => setShowChat((s) => !s)}
        onToggleFiles={() => setShowFiles((s) => !s)}
      />
      <div className="flex flex-1 overflow-hidden relative">
        <RemoteScreen />
        {showFiles && <FileTransfer />}
        {showChat && <ChatPanel />}
        <SessionStatusOverlay />
      </div>
    </div>
  )
}

export default App
