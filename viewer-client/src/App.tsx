import React, { useState } from 'react'
import ConnectionDialog from './components/ConnectionDialog'
import RemoteScreen from './components/RemoteScreen'
import Toolbar from './components/Toolbar'
import ChatPanel from './components/ChatPanel'
import FileTransfer from './components/FileTransfer'
import { useConnectionStore } from './stores/connection-store'

function App() {
  const { isConnected } = useConnectionStore()
  const [showChat, setShowChat] = useState(false)
  const [showFiles, setShowFiles] = useState(false)

  if (!isConnected) {
    return <ConnectionDialog />
  }

  return (
    <div className="flex flex-col h-screen bg-gray-900">
      <Toolbar
        onToggleChat={() => setShowChat((s) => !s)}
        onToggleFiles={() => setShowFiles((s) => !s)}
      />
      <div className="flex flex-1 overflow-hidden">
        <RemoteScreen />
        {showFiles && <FileTransfer />}
        {showChat && <ChatPanel />}
      </div>
    </div>
  )
}

export default App
