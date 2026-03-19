import React, { useRef, useState } from 'react'
import { useFileTransferStore } from '../stores/file-transfer-store'
import { useConnectionStore } from '../stores/connection-store'

export default function FileTransfer() {
  const { transfers } = useFileTransferStore()
  const { sendFile } = useConnectionStore()
  const [isDragging, setIsDragging] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)

  const handleFiles = (files: FileList | null) => {
    if (!files || !sendFile) return
    for (const file of Array.from(files)) {
      sendFile(file)
    }
  }

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(true)
  }

  const handleDragLeave = () => setIsDragging(false)

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
    handleFiles(e.dataTransfer.files)
  }

  return (
    <div className="w-72 bg-gray-800 border-l border-gray-700 flex flex-col">
      <div className="px-4 py-3 border-b border-gray-700">
        <h3 className="text-white font-medium text-sm">File Transfer</h3>
      </div>

      {/* Drop zone */}
      <div
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        className={`m-3 border-2 border-dashed rounded-lg p-4 text-center cursor-pointer transition-colors ${
          isDragging
            ? 'border-blue-400 bg-blue-900/20'
            : 'border-gray-600 hover:border-gray-500'
        } ${!sendFile ? 'opacity-50 pointer-events-none' : ''}`}
        onClick={() => fileInputRef.current?.click()}
      >
        <input
          ref={fileInputRef}
          type="file"
          multiple
          className="hidden"
          onChange={(e) => handleFiles(e.target.files)}
        />
        <p className="text-gray-400 text-xs">
          {sendFile ? 'Drop files or click to send' : 'Not connected'}
        </p>
      </div>

      {/* Transfer list */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {transfers.length === 0 ? (
          <p className="text-gray-500 text-xs text-center">No active transfers</p>
        ) : (
          transfers.map((t) => (
            <div key={t.id} className="bg-gray-700 rounded p-3">
              <div className="flex justify-between text-sm text-white mb-1">
                <span className="truncate text-xs">{t.fileName || t.id.slice(0, 8)}</span>
                <span className="text-gray-400 ml-2">{t.direction === 'upload' ? '\u2191' : '\u2193'}</span>
              </div>
              <div className="w-full bg-gray-600 rounded-full h-1.5">
                <div
                  className="bg-blue-500 h-1.5 rounded-full transition-all"
                  style={{
                    width:
                      t.fileSize > 0
                        ? `${Math.min(100, (t.bytesTransferred / t.fileSize) * 100)}%`
                        : t.status === 'completed'
                          ? '100%'
                          : '0%',
                  }}
                />
              </div>
              <div className="text-xs text-gray-400 mt-1">{t.status}</div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
