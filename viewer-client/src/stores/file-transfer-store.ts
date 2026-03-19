import { create } from 'zustand'

export interface FileTransfer {
  id: string
  fileName: string
  fileSize: number
  bytesTransferred: number
  direction: 'upload' | 'download'
  status: 'pending' | 'active' | 'completed' | 'error' | 'cancelled'
  error?: string
}

interface FileTransferStore {
  transfers: FileTransfer[]
  addTransfer: (transfer: FileTransfer) => void
  updateProgress: (id: string, bytes: number) => void
  completeTransfer: (id: string) => void
  failTransfer: (id: string, error: string) => void
  cancelTransfer: (id: string) => void
}

export const useFileTransferStore = create<FileTransferStore>((set) => ({
  transfers: [],
  addTransfer: (transfer) =>
    set((state) => ({ transfers: [...state.transfers, transfer] })),
  updateProgress: (id, bytes) =>
    set((state) => ({
      transfers: state.transfers.map((t) =>
        t.id === id ? { ...t, bytesTransferred: bytes } : t
      ),
    })),
  completeTransfer: (id) =>
    set((state) => ({
      transfers: state.transfers.map((t) =>
        t.id === id ? { ...t, status: 'completed' } : t
      ),
    })),
  failTransfer: (id, error) =>
    set((state) => ({
      transfers: state.transfers.map((t) =>
        t.id === id ? { ...t, status: 'error', error } : t
      ),
    })),
  cancelTransfer: (id) =>
    set((state) => ({
      transfers: state.transfers.map((t) =>
        t.id === id ? { ...t, status: 'cancelled' } : t
      ),
    })),
}))
