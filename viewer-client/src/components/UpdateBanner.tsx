import React, { useEffect, useState } from 'react'

interface UpdateInfo {
  version?: string
}

interface DownloadProgress {
  percent?: number
}

export default function UpdateBanner() {
  const [downloading, setDownloading] = useState(false)
  const [downloaded, setDownloaded] = useState(false)
  const [version, setVersion] = useState<string | null>(null)
  const [percent, setPercent] = useState(0)

  useEffect(() => {
    if (!window.electronAPI) return

    window.electronAPI.onUpdateAvailable((info: unknown) => {
      const v = (info as UpdateInfo).version ?? null
      setVersion(v)
      setDownloading(true)
    })

    window.electronAPI.onUpdateDownloaded((info: unknown) => {
      const v = (info as UpdateInfo).version ?? null
      setVersion(v)
      setDownloading(false)
      setDownloaded(true)
    })

    window.electronAPI.onUpdateProgress((progress: unknown) => {
      const p = (progress as DownloadProgress).percent ?? 0
      setPercent(Math.round(p))
    })
  }, [])

  if (!downloading && !downloaded) return null

  return (
    <div className="bg-blue-700 text-white text-sm px-4 py-2 flex items-center justify-between">
      {downloading ? (
        <span>
          Downloading update{version ? ` v${version}` : ''}... {percent}%
        </span>
      ) : (
        <>
          <span>Update{version ? ` v${version}` : ''} ready to install</span>
          <button
            onClick={() => window.electronAPI?.installUpdate()}
            className="ml-4 px-3 py-1 bg-white text-blue-700 rounded text-sm font-medium hover:bg-blue-50 transition-colors"
          >
            Restart now
          </button>
        </>
      )}
    </div>
  )
}
