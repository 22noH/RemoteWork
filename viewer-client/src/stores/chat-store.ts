import { create } from 'zustand'

export interface ChatMessage {
  id: string
  sender: 'host' | 'viewer'
  content: string
  timestamp: number
}

interface ChatStore {
  messages: ChatMessage[]
  isTyping: boolean
  addMessage: (msg: ChatMessage) => void
  setTyping: (typing: boolean) => void
  clearMessages: () => void
}

export const useChatStore = create<ChatStore>((set) => ({
  messages: [],
  isTyping: false,
  addMessage: (msg) => set((state) => ({ messages: [...state.messages, msg] })),
  setTyping: (typing) => set({ isTyping: typing }),
  clearMessages: () => set({ messages: [] }),
}))
