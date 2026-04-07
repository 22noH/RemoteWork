# Architecture

Technical architecture for 22NO — Remote Desktop Application.

## 시스템 구성

```
[Viewer Client]  ←──WSS/JSON──→  [Signaling Server]  ←──WSS/Protobuf──→  [Host Agent]
       ↕                                                                          ↕
  WebRTC P2P  ←──────────────────────────────────────────────────────────→  WebRTC P2P
  (video/audio/data channels)
```

## 컴포넌트별 역할

### Signaling Server (`signaling-server/`)
- WebSocket 서버. Viewer(JSON)와 Host(Protobuf) 동시 지원
- 역할: Host 등록, Viewer 연결 요청 수락, SDP/ICE 릴레이
- 인증: Argon2id 비밀번호 해시 검증
- 세션 관리: DashMap 기반, 5분 idle timeout
- idle 4m30s: `session_timeout_warning` 전송 / 5m: `session_expired` 전송 후 종료
- TLS: `--tls-cert` / `--tls-key` 또는 환경변수로 WSS 활성화

### Host Agent (`host-agent/`)
- 화면 캡처 → VP8 인코딩 → WebRTC video track으로 전송
- 입력 수신 → enigo로 OS 이벤트 주입
- 오디오: cpal 마이크 캡처 → Opus 인코딩 → WebRTC audio track
- 채팅/파일: Protobuf DataChannel
- 시스템 트레이: ID 표시, 연결 목록, 종료
- 재연결: 지수 백오프(1s→30s), 시그널링 드롭 후 세션 유지
- 세션 health: 30초마다 Failed/Closed 세션 정리

### Viewer Client (`viewer-client/`)
- React + Electron (또는 브라우저)
- 원격 화면 표시: `<video>` + WebRTC MediaStream
- 입력 전송: 마우스/키보드 → Protobuf → `"input"` DataChannel
- 채팅/파일: `ChatManager` / `FileTransferManager`
- 재연결: WS onclose 백오프(최대 10회), ICE restart
- Idle timeout: 5분 입력 없음 → 경고(30초 전) → 자동 disconnect
- 자동 업데이터: `electron-updater`, GitHub Releases

## 프로토콜

### Signaling (WebSocket)
- Viewer ↔ Server: JSON 텍스트 프레임
- Host ↔ Server: Protobuf 바이너리 프레임
- 서버가 Host→Viewer 릴레이 시 Protobuf→JSON 재인코딩

### WebRTC DataChannels
| 채널명 | 방향 | 형식 | 용도 |
|--------|------|------|------|
| `"input"` | Viewer→Host | Protobuf (InputEvent) | 마우스/키보드 |
| `"chat"` | 양방향 | Protobuf (ChatEnvelope) | 채팅 |
| `"file"` | Viewer→Host | Protobuf (FileTransferMessage) | 파일 전송 |

## 보안
- 비밀번호: Argon2id PHC 해시 (호스트 저장), 평문 전송 후 서버 검증
- 전송 암호화: WSS(TLS) + WebRTC DTLS
- 파일 수신: `FsAccess::validate_path()` path traversal 방지
- 속도 제한: 5회 인증 실패 → 10분 차단

## GitHub
- Repository: https://github.com/22noH/RemoteWork.git
- Branch: master
