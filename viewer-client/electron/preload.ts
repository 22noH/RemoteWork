import { contextBridge, ipcRenderer } from 'electron'

contextBridge.exposeInMainWorld('electronAPI', {
  getVersion: () => ipcRenderer.invoke('get-version'),
  getPlatform: () => ipcRenderer.invoke('get-platform'),
  isElectron: true,
  checkForUpdates: () => ipcRenderer.invoke('updater:check'),
  installUpdate: () => ipcRenderer.invoke('updater:install'),
  onUpdateAvailable: (cb: (info: unknown) => void) => {
    ipcRenderer.on('updater:available', (_e, info) => cb(info))
  },
  onUpdateDownloaded: (cb: (info: unknown) => void) => {
    ipcRenderer.on('updater:downloaded', (_e, info) => cb(info))
  },
  onUpdateProgress: (cb: (progress: unknown) => void) => {
    ipcRenderer.on('updater:progress', (_e, progress) => cb(progress))
  },
  onUpdateError: (cb: (err: string) => void) => {
    ipcRenderer.on('updater:error', (_e, err) => cb(err))
  },
})

declare global {
  interface Window {
    electronAPI: {
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
}
