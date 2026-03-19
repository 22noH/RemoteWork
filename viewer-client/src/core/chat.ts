import protobuf from 'protobufjs'
import { useChatStore } from '../stores/chat-store'

const CHAT_PROTO_TEXT = `
syntax = "proto3";
package remote_work;

message ChatMessage {
  string id = 1;
  string sender = 2;
  string content = 3;
  uint64 timestamp_ms = 4;
}

message ChatEnvelope {
  oneof payload {
    ChatMessage message = 1;
    TypingIndicator typing = 2;
  }
}

message TypingIndicator {
  string sender = 1;
  bool is_typing = 2;
}
`

const root = protobuf.parse(CHAT_PROTO_TEXT).root
const ChatEnvelopeType = root.lookupType('remote_work.ChatEnvelope')

function encodeEnvelope(payload: object): ArrayBuffer {
  const msg = ChatEnvelopeType.create(payload)
  const buf = ChatEnvelopeType.encode(msg).finish()
  return buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength)
}

export class ChatManager {
  constructor(private sendFn: (data: ArrayBuffer) => void) {}

  sendMessage(content: string): void {
    const id = crypto.randomUUID()
    const timestamp_ms = Date.now()

    const data = encodeEnvelope({
      message: {
        id,
        sender: 'viewer',
        content,
        timestamp_ms,
      },
    })
    this.sendFn(data)
  }

  handleIncoming(data: ArrayBuffer): void {
    try {
      const buf = new Uint8Array(data)
      const envelope = ChatEnvelopeType.decode(buf) as any

      if (envelope.message) {
        const msg = envelope.message
        useChatStore.getState().addMessage({
          id: msg.id || crypto.randomUUID(),
          sender: msg.sender === 'host' ? 'host' : 'viewer',
          content: msg.content || '',
          timestamp:
            typeof msg.timestamp_ms === 'number'
              ? msg.timestamp_ms
              : Date.now(),
        })
      }
      // Typing indicator: could update isTyping in store if desired
    } catch (e) {
      console.warn('[ChatManager] Failed to decode incoming message:', e)
    }
  }
}
