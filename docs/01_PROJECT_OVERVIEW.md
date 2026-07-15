# Remote Work — 프로젝트 개요 및 기술 스택

## 프로젝트 목표

TeamViewer와 유사한 원격 데스크탑 애플리케이션.
- **호스트(Host)**: 화면을 공유하고 원격 제어를 받는 측
- **뷰어(Viewer)**: 호스트 화면을 보고 원격 제어를 보내는 측
- **시그널링 서버(Signaling Server)**: 두 클라이언트의 WebRTC 연결을 중개

P2P WebRTC 연결로 실시간 화면 공유 및 입력 제어를 구현한다. 시그널링 서버는 연결 수립에만 관여하고, 이후 미디어/데이터는 P2P로 직접 전송된다.

---

## 디렉토리 구조

```
Remote_Work/
├── proto/                        # Protobuf 메시지 정의
│   ├── messages.proto            # 시그널링 메시지 (Envelope oneof)
│   ├── input.proto               # 입력 이벤트 (키보드/마우스)
│   ├── file_transfer.proto       # 파일 전송 청크
│   └── chat.proto                # 채팅 메시지
│
├── signaling-server/             # Rust WebSocket 시그널링 서버
│   └── src/
│       ├── main.rs
│       ├── ws_server.rs          # 듀얼 프로토콜 핸들러 (JSON + Protobuf)
│       ├── json_protocol.rs      # JSON serde 헬퍼
│       ├── session_registry.rs   # Host/Viewer/Session 관리 (DashMap)
│       ├── auth.rs               # Argon2id 비밀번호 검증
│       └── relay.rs              # SDP/ICE 릴레이
│
├── host-agent/                   # Rust 호스트 에이전트 (Cargo workspace)
│   ├── src/
│   │   ├── main.rs               # 진입점, tracing 초기화
│   │   ├── app.rs                # 이벤트 루프, 연결 승인/1:1/뷰 전용/모니터 선택/채팅/파일(→다운로드)
│   │   ├── config.rs             # JSON 설정 파일 (host_id, stun, turn, allow_control)
│   │   └── ui.rs                 # 호스트 GUI (egui) — ID/일회용 비밀번호, 뷰 전용 토글, 승인 프롬프트, 채팅
│   └── crates/
│       ├── capture/              # 화면 캡처 + VP8 인코딩
│       ├── input/                # 입력 주입 enigo (Phase 3)
│       ├── network/              # WebSocket 시그널링 + WebRTC
│       ├── file_transfer/        # 파일 전송 (Phase 4)
│       ├── audio/                # cpal 마이크 캡처/재생, opus 인코딩/디코딩
│       ├── auth/                 # Argon2id 해싱 + 자격증명 생성
│       └── proto/                # prost 코드 생성
│
├── viewer-client/                # TypeScript React 뷰어
│   ├── electron/
│   │   ├── main.ts               # electron-updater, 트레이
│   │   └── preload.ts            # contextBridge IPC
│   └── src/
│       ├── App.tsx
│       ├── core/
│       │   ├── signaling.ts      # WebSocket JSON 시그널링 클라이언트
│       │   ├── peer-connection.ts # RTCPeerConnection 래퍼
│       │   ├── input-sender.ts   # 입력 이벤트 직렬화 + 전송 (Phase 3)
│       │   ├── file-transfer.ts  # 파일 전송 (Phase 4)
│       │   └── chat.ts           # 채팅 (Phase 4)
│       ├── components/
│       │   ├── ConnectionDialog.tsx
│       │   ├── RemoteScreen.tsx  # <video> WebRTC 스트림 표시
│       │   ├── Toolbar.tsx
│       │   ├── ChatPanel.tsx
│       │   ├── FileTransfer.tsx
│       │   ├── SessionStatusOverlay.tsx  # 재연결/idle 경고/세션 만료 오버레이
│       │   └── UpdateBanner.tsx          # 자동 업데이터 알림 배너
│       ├── hooks/
│       │   └── useIdleTimeout.ts # 5분 비활성 감지 훅
│       └── stores/
│           ├── connection-store.ts   # Zustand: 연결 상태, remoteStream, reconnectingSince, disconnectReason, lastInputAt, idleWarning
│           ├── chat-store.ts
│           └── file-transfer-store.ts
│
└── docs/                         # 이 문서들
```

---

## 기술 스택 상세

### Signaling Server (Rust)

| 항목 | 기술 | 버전 | 용도 |
|------|------|------|------|
| 런타임 | tokio | 1.x | 비동기 I/O |
| WebSocket | tokio-tungstenite | 0.21 | WS 서버 |
| 직렬화 (호스트) | prost | 0.12 | Protobuf 디코딩 |
| 직렬화 (뷰어) | serde + serde_json | 1.x | JSON 파싱/생성 |
| 세션 저장 | dashmap | 5.x | 동시성 안전 HashMap |
| 세션 ID | uuid | 1.x (v4) | 무작위 세션 토큰 |
| 인증 (Phase 5) | argon2 | 0.5 | Argon2id 해시 |
| 속도 제한 | 자체 구현 | — | 5회 실패 → 10분 차단 |
| 로깅 | tracing + tracing-subscriber | 0.1 | 구조화 로그 |

**설계 포인트:**
- `Message::Text` → JSON (TypeScript 뷰어)
- `Message::Binary` → Protobuf (Rust 호스트)
- `Arc<AtomicBool> use_json`으로 연결별 인코딩 모드 추적
- 호스트→뷰어 릴레이 시 Protobuf → JSON 재인코딩

---

### Host Agent (Rust Workspace)

#### 메인 바이너리 (`host-agent`)

| 항목 | 기술 | 버전 | 용도 |
|------|------|------|------|
| 런타임 | tokio | 1.x | 비동기 I/O |
| 로깅 | tracing | 0.1 | 구조화 로그 |
| 설정 | serde_json | 1.x | config.json 읽기/쓰기 |
| 설정 경로 | dirs | 5 | OS별 설정 디렉토리 |
| GUI | eframe / egui | 0.28 | 호스트 데스크탑 창 (ID/일회용 비밀번호, 승인 프롬프트, 채팅) |
| UI 로케일 | sys-locale | — | OS 언어 감지 |

> **호스트 GUI (egui):** 고정 크기(~380x590, 최소화/최대화 버튼 없음) 창. Host ID + 일회용 비밀번호, 뷰 전용 모드 토글, 연결 상태와 "Quit" 버튼을 표시한다. 뷰어 접속 시 **연결 승인 프롬프트(Allow/Deny)** 가 뜨고 Allow 시에만 세션이 수립된다. 파일 수신 시 저장 경로를 보여주는 **파일 수신 승인 프롬프트(Accept/Deny)** 가 뜨고, 단일 창 채팅 화면(뒤로 "←" 버튼, 상대 메시지 좌측/호스트 메시지 우측 정렬, 한글 입력 지원)을 제공한다. 창을 닫으면(X) 종료 대신 작업 표시줄로 최소화된다. (이전 tray-icon 시스템 트레이는 제거됨.)

#### `capture` 크레이트

| 항목 | 기술 | 버전 | 용도 |
|------|------|------|------|
| 화면 캡처 | xcap | — | 모니터 RGBA 캡처 |
| VP8 인코딩 | vpx-encode | — | RGBA→I420→VP8 |
| 색공간 변환 | 자체 구현 | — | BT.601 RGBA→I420 |

**캡처 파이프라인:** `Monitor::capture_image()` → `Frame (RGBA)` → `Frame::to_i420()` → `VpxEncoder::encode()` → `EncodedFrame (VP8 bytes)`

**빌드 전제조건:** `libvpx` 설치 필요
- Windows: `vcpkg install libvpx:x64-windows`

#### `network` 크레이트

| 항목 | 기술 | 버전 | 용도 |
|------|------|------|------|
| WebRTC | webrtc (webrtc-rs) | 0.11 | P2P 연결, VP8 트랙 |
| WebSocket | tokio-tungstenite | 0.21 | 시그널링 서버 연결 |
| 패스워드 해시 | sha2 + hex | 0.10 / 0.4 | SHA-256 hex (뷰어와 통일) |
| Protobuf | prost | 0.12 | 시그널링 메시지 인코딩 |
| 바이너리 데이터 | bytes | 1.x | WebRTC Sample 데이터 |

**DataChannel (4개):**
- `"input"` (뷰어→호스트, Protobuf `InputEvent`) — 마우스/키보드
- `"chat"` (양방향, Protobuf `ChatEnvelope`)
- `"file"` (뷰어→호스트, Protobuf `FileTransferMessage`) — SHA-256 검증, 64KB 청크
- `"control"` (호스트→뷰어) — 모니터 목록 / 제어

**mDNS 비활성화:** 호스트 WebRTC는 `MulticastDnsMode::Disabled`로 설정하여 Windows에서 "Failed to send mDNS packet" 로그 스팸을 방지한다.

**빌드 전제조건:** OpenSSL (webrtc-rs 의존성)
- Windows: `OPENSSL_DIR` 환경변수 설정 또는 rustls feature 사용

#### `input` 크레이트 (Phase 3)

| 항목 | 기술 | 버전 | 용도 |
|------|------|------|------|
| 입력 주입 | enigo | 0.1 | 키보드/마우스 OS 이벤트 발생 |

#### `auth` 크레이트

| 항목 | 기술 | 버전 | 용도 |
|------|------|------|------|
| 비밀번호 해시 | argon2 | 0.5 | Argon2id (Phase 5용) |
| 난수 | rand | 0.8 | host_id(영구 9자리) / 일회용 비밀번호 생성 |

> **일회용 비밀번호:** 호스트 비밀번호는 실행할 때마다 새로 생성되며 디스크에 저장되지 않는다(serde skip). 세션이 끝나면 유출된 비밀번호는 무용지물이 된다. 따라서 config.json에는 비밀번호 필드가 없다.

#### `proto` 크레이트

| 항목 | 기술 | 버전 | 용도 |
|------|------|------|------|
| Protobuf 빌드 | prost-build | 0.12 | `.proto` → Rust 코드 생성 |

---

### Viewer Client (TypeScript)

| 항목 | 기술 | 버전 | 용도 |
|------|------|------|------|
| 언어 | TypeScript | 5.x | 정적 타입 |
| UI 프레임워크 | React | 18 | 컴포넌트 기반 UI |
| 빌드 도구 | Vite | 5.x | 개발 서버 / 번들러 |
| 데스크탑 셸 | Electron | — | 네이티브 앱 패키징 (Phase 5) |
| 상태 관리 | Zustand | — | 전역 연결 상태 |
| 스타일 | Tailwind CSS | — | 유틸리티 CSS |
| WebRTC | 브라우저 내장 API | — | `RTCPeerConnection` |
| 시그널링 | WebSocket (브라우저 내장) | — | JSON 텍스트 프레임 |
| 패스워드 해시 | Web Crypto API | — | SHA-256 (`crypto.subtle.digest`) |

> **시그널링 URL 자동 유도:** 뷰어는 자신의 페이지 origin으로부터 시그널링 URL을 자동으로 유도한다 — 배포 시 `wss://<host>/signal`, localhost에서는 `ws://localhost:8080`, 또는 `VITE_SIGNALING_URL`이 설정되어 있으면 그 값을 사용한다. 하나의 빌드가 도메인 별 재빌드 없이 어떤 도메인에서든 동작한다.

**시그널링 메시지 포맷 (JSON):**
```json
{ "type": "connect_request",          "payload": { "target_host_id": "...", "password_hash": "...", "viewer_session_id": "..." } }
{ "type": "sdp_offer",                "payload": { "sdp": "...", "session_token": "..." } }
{ "type": "ice_candidate",            "payload": { "candidate": "...", "sdp_mid": "...", "sdp_mline_index": 0, "session_token": "..." } }
{ "type": "session_timeout_warning",  "payload": { "seconds_remaining": 30 } }
{ "type": "session_expired",          "payload": { "reason": "idle_timeout" } }
```

---

## 연결 흐름 (Phase 2 기준)

```
Viewer (JSON/WS)          Signaling Server           Host (Protobuf/WS)
       │                         │                          │
       │──connect_request───────▶│                          │
       │                         │──IncomingConnection─────▶│ (Protobuf)
       │◀──connect_response──────│                          │
       │                         │                          │
       │──sdp_offer─────────────▶│──SdpOffer (Proto)───────▶│
       │                         │                          │ 호스트 GUI 연결 승인(Allow/Deny)
       │                         │                          │ ※ 1:1: 활성 세션이 있으면 두 번째 offer 거절
       │                         │                          │ HostPeerConnection::handle_offer()
       │                         │                          │ → SDP answer 생성
       │                         │◀─SdpAnswer (Proto)───────│
       │◀──sdp_answer (JSON)─────│                          │
       │                         │                          │
       │◀──── ICE 교환 (trickle, 양방향) ──────────────────▶│
       │                         │                          │
       │◀═══════════ WebRTC P2P VP8 비디오 스트림 ══════════│
       │                         │                    xcap→I420→VP8→write_sample()
```

> Phase 5 이후: 시그널링 연결은 WSS(TLS), 비밀번호는 Argon2id 해시, 재연결은 지수 백오프(최대 10회)
>
> 현재: 연결마다 호스트 GUI 연결 승인(Allow/Deny)을 거치며, 활성 세션이 있으면 두 번째 뷰어는 거절된다(1:1 연결). **뷰 전용 모드**(config `allow_control`, 기본 true; 환경변수 `ALLOW_CONTROL=0/false`로 비활성화)가 꺼지면 입력이 무시된다. **모니터 선택**은 `"control"` DataChannel로 모니터 목록을 전달해 뷰어가 볼 모니터를 고르고, 호스트가 캡처러/인코더를 재구성한다.
