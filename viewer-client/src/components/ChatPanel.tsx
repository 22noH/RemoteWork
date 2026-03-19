import React, { useState, useRef, useEffect } from 'react'
import { useChatStore } from '../stores/chat-store'
import { useConnectionStore } from '../stores/connection-store'

export default function ChatPanel() {
  const { messages } = useChatStore()
  const { sendMessage } = useConnectionStore()
  const [input, setInput] = useState('')
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  const handleSend = (e: React.FormEvent) => {
    e.preventDefault()
    if (!input.trim() || !sendMessage) return

    // Optimistic update: add to local store immediately
    useChatStore.getState().addMessage({
      id: crypto.randomUUID(),
      sender: 'viewer' as const,
      content: input.trim(),
      timestamp: Date.now(),
    })

    sendMessage(input.trim())
    setInput('')
  }

  return (
    <div className="w-72 bg-gray-800 border-l border-gray-700 flex flex-col">
      <div className="px-4 py-3 border-b border-gray-700">
        <h3 className="text-white font-medium text-sm">Chat</h3>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {messages.map((msg) => (
          <div key={msg.id} className={`flex ${msg.sender === 'viewer' ? 'justify-end' : 'justify-start'}`}>
            <div
              className={`max-w-[200px] rounded-lg px-3 py-2 text-sm ${
                msg.sender === 'viewer' ? 'bg-blue-600 text-white' : 'bg-gray-700 text-gray-200'
              }`}
            >
              {msg.content}
            </div>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>

      <form onSubmit={handleSend} className="p-3 border-t border-gray-700">
        <div className="flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={sendMessage ? 'Type a message...' : 'Not connected'}
            disabled={!sendMessage}
            className="flex-1 bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500 disabled:opacity-50"
          />
          <button
            type="submit"
            disabled={!sendMessage}
            className="px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm disabled:opacity-50"
          >
            Send
          </button>
        </div>
      </form>
    </div>
  )
}
