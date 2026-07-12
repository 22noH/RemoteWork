import protobuf from 'protobufjs'
import { useFileTransferStore } from '../stores/file-transfer-store'

const FILE_PROTO_TEXT = `
syntax = "proto3";
package remote_work;

message FileTransferMessage {
  oneof payload {
    FileTransferRequest request = 1;
    FileTransferAccept accept = 2;
    FileTransferReject reject = 3;
    FileChunk chunk = 4;
    FileTransferComplete complete = 5;
    FileTransferError error = 6;
    FileTransferCancel cancel = 7;
  }
}

message FileTransferRequest {
  string transfer_id = 1;
  string file_name = 2;
  uint64 file_size = 3;
  string sha256_hash = 4;
  string destination_path = 5;
}

message FileTransferAccept { string transfer_id = 1; }
message FileTransferReject { string transfer_id = 1; string reason = 2; }
message FileChunk { string transfer_id = 1; uint64 offset = 2; bytes data = 3; bool last_chunk = 4; }
message FileTransferComplete { string transfer_id = 1; string sha256_hash = 2; }
message FileTransferError { string transfer_id = 1; string error = 2; }
message FileTransferCancel { string transfer_id = 1; }
`

// keepCase: this module addresses the proto fields by their snake_case names
// (file_name, transfer_id, …); without it protobufjs camelCases them and every
// field silently becomes empty.
const root = protobuf.parse(FILE_PROTO_TEXT, { keepCase: true }).root
const FileTransferMessageType = root.lookupType(
  'remote_work.FileTransferMessage',
)

const CHUNK_SIZE = 64 * 1024 // 64 KB

function encodeMsg(payload: object): ArrayBuffer {
  const msg = FileTransferMessageType.create(payload)
  const buf = FileTransferMessageType.encode(msg).finish()
  return buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength) as ArrayBuffer
}

async function sha256Hex(data: ArrayBuffer): Promise<string> {
  const hashBuffer = await crypto.subtle.digest('SHA-256', data)
  return Array.from(new Uint8Array(hashBuffer))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
}

export class FileTransferManager {
  private pendingAccept = new Map<
    string,
    { resolve: () => void; reject: (reason: string) => void }
  >()

  constructor(private sendFn: (data: ArrayBuffer) => void) {}

  async sendFile(file: File, destinationPath = '.'): Promise<void> {
    const transferId = crypto.randomUUID()
    const fileBuffer = await file.arrayBuffer()
    const sha256Hash = await sha256Hex(fileBuffer)

    useFileTransferStore.getState().addTransfer({
      id: transferId,
      fileName: file.name,
      fileSize: file.size,
      bytesTransferred: 0,
      direction: 'upload',
      status: 'pending',
    })

    const requestData = encodeMsg({
      request: {
        transfer_id: transferId,
        file_name: file.name,
        file_size: file.size,
        sha256_hash: sha256Hash,
        destination_path: destinationPath,
      },
    })
    this.sendFn(requestData)

    await new Promise<void>((resolve, reject) => {
      this.pendingAccept.set(transferId, { resolve, reject })
      setTimeout(() => {
        if (this.pendingAccept.has(transferId)) {
          this.pendingAccept.delete(transferId)
          reject('Accept timeout')
        }
      }, 10000)
    })

    useFileTransferStore.getState().updateProgress(transferId, 0)

    let offset = 0
    const bytes = new Uint8Array(fileBuffer)
    while (offset < bytes.length) {
      const end = Math.min(offset + CHUNK_SIZE, bytes.length)
      const chunk = bytes.slice(offset, end)
      const lastChunk = end >= bytes.length

      const chunkData = encodeMsg({
        chunk: {
          transfer_id: transferId,
          offset,
          data: chunk,
          last_chunk: lastChunk,
        },
      })
      this.sendFn(chunkData)

      offset = end
      useFileTransferStore.getState().updateProgress(transferId, offset)

      // Yield to event loop to avoid blocking UI
      await new Promise((r) => setTimeout(r, 0))
    }
  }

  handleIncoming(data: ArrayBuffer): void {
    try {
      const buf = new Uint8Array(data)
      const msg = FileTransferMessageType.decode(buf) as any

      if (msg.accept) {
        const pending = this.pendingAccept.get(msg.accept.transfer_id)
        if (pending) {
          this.pendingAccept.delete(msg.accept.transfer_id)
          pending.resolve()
        }
      } else if (msg.reject) {
        const pending = this.pendingAccept.get(msg.reject.transfer_id)
        if (pending) {
          this.pendingAccept.delete(msg.reject.transfer_id)
          pending.reject(msg.reject.reason || 'Rejected')
        }
        useFileTransferStore
          .getState()
          .failTransfer(msg.reject.transfer_id, msg.reject.reason || 'Rejected')
      } else if (msg.complete) {
        useFileTransferStore
          .getState()
          .completeTransfer(msg.complete.transfer_id)
      } else if (msg.error) {
        useFileTransferStore
          .getState()
          .failTransfer(
            msg.error.transfer_id,
            msg.error.error || 'Unknown error',
          )
      } else if (msg.cancel) {
        useFileTransferStore
          .getState()
          .cancelTransfer(msg.cancel.transfer_id)
      }
    } catch (e) {
      console.warn(
        '[FileTransferManager] Failed to decode incoming message:',
        e,
      )
    }
  }
}
