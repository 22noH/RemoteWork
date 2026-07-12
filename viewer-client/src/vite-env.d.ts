/// <reference types="vite/client" />

// electronAPI is exposed by electron/preload.ts via contextBridge; optional
// because the renderer also runs in a plain browser during `vite dev`.
interface Window {
  electronAPI?: {
    getVersion: () => Promise<string>
    getPlatform: () => Promise<string>
    isElectron: boolean
    checkForUpdates: () => Promise<unknown>
    installUpdate: () => Promise<void>
    onUpdateAvailable: (cb: (info: unknown) => void) => void
    onUpdateDownloaded: (cb: (info: unknown) => void) => void
    onUpdateProgress: (cb: (progress: unknown) => void) => void
    onUpdateError: (cb: (err: string) => void) => void
  }
}
