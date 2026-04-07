# Remote Work — 전체 Phase 계획

## 로드맵 요약

| Phase | 상태 | 핵심 기능 |
|-------|------|-----------|
| **1** | ✅ 완료 | 프로젝트 기초 — Proto, 시그널링 서버, Host/Viewer 스켈레톤 |
| **2** | ✅ 완료 | 화면 공유 — WebRTC 비디오 스트림 (xcap→VP8→P2P) |
| **3** | ✅ 완료 | 원격 제어 — 뷰어 입력 → 호스트 enigo 주입 |
| **4** | ✅ 완료 | 파일 전송 + 채팅 + 시스템 트레이 |
| **5** | ✅ 완료 | 보안 강화 + TURN 릴레이 + Electron 패키징 |

---

## Phase 1 — 기초 구조 ✅

### 목표
빌드 가능한 뼈대와 시그널링 프로토콜 정의

### 구현 내용

#### Protobuf 메시지 정의 (`proto/`)
- `messages.proto`: `Envelope` oneof — `RegisterHost`, `RegisterAck`, `ConnectRequest`, `ConnectResponse`, `IncomingConnection`, `SdpOffer`, `SdpAnswer`, `IceCandidate`, `Error`, `Ping`, `Pong`
- `input.proto`, `file_transfer.proto`, `chat.proto`: Phase 3-4용 사전 정의

#### Signaling Server (`signaling-server/`)
- WebSocket 서버 (tokio-tungstenite)
- Host 등록 (`RegisterHost`) + ACK
- Viewer 연결 요청 + 비밀번호 검증 (`==` 비교)
- SDP/ICE 릴레이 (`relay.rs`)
- 세션 관리 (`DashMap`)
- 속도 제한: 5회 실패 → 10분 차단

#### Host Agent (`host-agent/`)
- 설정 파일 자동 생성 (`~/.config/remote-work/config.json`)
  - `host_id`: 9자리 숫자 난수
  - `password`: 6자리 영숫자 난수 (혼동 방지 문자 제외)
- 시그널링 서버 WebSocket 연결
- Protobuf 메시지 송수신 (단방향 수신 스텁)

#### Viewer Client (`viewer-client/`)
- React UI: `ConnectionDialog`, `RemoteScreen`, `Toolbar`, `ChatPanel`, `FileTransfer`
- Zustand 스토어: 연결 상태, 세션 토큰
- `SignalingClient`: JSON over WebSocket
- `RemotePeerConnection`: `RTCPeerConnection` 래퍼, SDP offer 생성, 데이터 채널 4개

#### Capture Crate (`host-agent/crates/capture/`)
- `Capturer`: xcap 모니터 캡처 → RGBA `Frame`
- `Encoder`: vpx-encode VP8 인코더 (bitrate, fps 설정 가능)
- `Frame::to_i420()`: BT.601 RGBA→I420 색공간 변환

### 발견된 문제 (Phase 2에서 해결)
1. 시그널링 프로토콜 불일치: 서버는 Protobuf, 뷰어는 JSON
2. 인증 해시 불일치: 호스트 Argon2id vs 뷰어 SHA-256
3. `peer_connection.rs` 빈 스텁
4. `SignalingClient` 단방향 (수신만, 응답 불가)

---

## Phase 2 — 화면 공유 ✅

### 목표
뷰어에서 Host ID + 비밀번호 입력 → WebRTC P2P 수립 → 호스트 화면 실시간 표시

### Step 1: 시그널링 서버 듀얼 프로토콜
- `json_protocol.rs` 신규 작성: JSON serde 타입 + 메시지 빌더 함수
- `ws_server.rs` 리라이트:
  - `Message::Text` → JSON 파싱 → `handle_json_message()`
  - `Message::Binary` → Protobuf 디코딩 → `handle_proto_payload()`
  - `Arc<AtomicBool> use_json`으로 send_task 프레임 타입 제어
  - 호스트→뷰어 릴레이 시 Protobuf → JSON 재인코딩

### Step 2: 인증 해시 통일
- 호스트: `auth::hash_password` (Argon2id) → `sha2 + hex` SHA-256 hex로 교체
- 뷰어: Web Crypto SHA-256 hex (유지)
- 서버: `==` 비교 (양측이 같은 SHA-256을 보내므로 작동)

### Step 3: SignalingClient 리팩토링
- 양방향 설계: `connect()` → `(SignalingClient, UnboundedReceiver<SignalingEvent>)`
- `SignalingEvent` enum: `Registered`, `IncomingConnection`, `SdpOffer`, `IceCandidate`, `Disconnected`, `Error`
- `send_sdp_answer()`, `send_ice_candidate()` 메서드
- 백그라운드 태스크: `tokio::select!` — WS 수신, 발신 채널, 종료 신호

### Step 4: WebRTC PeerConnection 구현
- `HostPeerConnection::new(stun_servers, ice_tx)`: MediaEngine + APIBuilder + RTCConfiguration
- VP8 `TrackLocalStaticSample` 생성 및 트랙 추가
- `handle_offer(sdp)`: remote description 설정 → answer 생성 → ICE gathering 완료 대기 → answer SDP 반환
- `add_ice_candidate()`: trickle ICE 처리

### Step 5: App 이벤트 루프 + 캡처 파이프라인
- `HashMap<String, Session>`: session_token별 세션 관리
- `SignalingEvent::SdpOffer` 수신 시:
  1. `HostPeerConnection::new()` 생성
  2. ICE 포워딩 태스크 스폰
  3. `handle_offer()` → answer 전송
  4. `capture_loop()` 태스크 스폰
- `capture_loop()`: `interval(33ms)` → xcap → I420 → VP8 → `write_sample()`

### Step 6: 뷰어 스트림 연결
- `connection-store.ts`: `remoteStream: MediaStream | null` + `setRemoteStream()` 추가
- `peer-connection.ts`: `ontrack` → `setRemoteStream()` 호출
- `RemoteScreen.tsx`: `useEffect([remoteStream])` → `video.srcObject = remoteStream`

---

## Phase 3 — 원격 제어 ✅

### 목표
뷰어에서 마우스/키보드 입력 → 데이터 채널 → 호스트 enigo 주입

### 구현 계획

#### 뷰어 (`input-sender.ts`)
- `mousemove`, `mousedown`, `mouseup`, `wheel` 이벤트 캡처
- `keydown`, `keyup` 이벤트 캡처
- 좌표 정규화: 뷰어 비디오 크기 → 호스트 화면 크기 비율 변환
- Protobuf `MouseEvent` / `KeyEvent` 직렬화
- `inputChannel.send(bytes)` 전송

#### 호스트 (`input` crate)
- `enigo = "0.1"` 추가
- `InputHandler::handle(event: InputEvent)`:
  - `MouseEvent` → `enigo.mouse_move_to()`, `enigo.mouse_click()` 등
  - `KeyEvent` → `enigo.key_down()`, `enigo.key_up()`
- `HostPeerConnection`에 `ondatachannel` 핸들러 추가
- 데이터 채널 수신 → Protobuf 역직렬화 → `InputHandler`로 디스패치

#### 추가 작업
- `input.proto` Protobuf 타입: `KeyEvent { key_code, pressed }`, `MouseEvent { x, y, button, wheel_delta }`
- `host-agent/crates/input/Cargo.toml`에 `enigo = "0.1"` 추가
- 좌표 정규화를 위한 호스트 화면 해상도 정보 뷰어 전달 (SDP answer 확장 또는 별도 메시지)

---

## Phase 4 — 파일 전송 + 채팅 + 시스템 트레이 ✅

### 목표
파일 전송, 채팅, 시스템 트레이 아이콘

### 구현 계획

#### 파일 전송
- `file_transfer.proto`: `FileChunk { file_id, chunk_index, data, total_chunks }`, `FileInfo { name, size, mime_type }`
- 호스트 `sender.rs`: 파일을 청크로 나누어 `fileChannel`로 전송
- 호스트 `receiver.rs`: 청크 수신 → 파일 재조합 → `allowed_dirs` 내에 저장
- 뷰어 `file-transfer.ts`: 드래그 앤 드롭 or 파일 선택 → 청크 전송
- 뷰어 `FileTransfer.tsx`: 전송 진행률 UI

#### 채팅
- `chat.proto`: `ChatMessage { sender_id, content, timestamp_ms }`
- 데이터 채널 `chatChannel` 양방향 사용
- 뷰어 `ChatPanel.tsx` + `chat-store.ts` 연결

#### 시스템 트레이 (`tray.rs`)
- Host ID / Password 표시
- 연결 중인 뷰어 목록
- 연결 종료 버튼
- Windows: `tray-item` 크레이트 또는 `winrt` API

---

## Phase 5 — 보안 강화 + 패키징 ✅

### 목표
프로덕션 수준의 보안, TURN 릴레이, 배포 가능한 Electron 앱

### 구현 계획

#### 보안 강화
- **Argon2id 인증**: 서버가 Argon2id 해시 검증으로 업그레이드
  - 호스트: `auth::hash_password()` (이미 구현됨) 복원
  - 뷰어: argon2-browser WASM 라이브러리 적용
  - 서버: `auth::verify_password()` (Argon2id) 적용
- **TLS**: 시그널링 서버 WSS (HTTPS), Let's Encrypt
- **세션 토큰 검증**: 모든 릴레이 메시지에서 세션 토큰 유효성 검증
- **입력 검증**: `allowed_dirs` 체크, path traversal 방지

#### TURN 릴레이
- 대칭 NAT 환경에서 P2P 실패 시 TURN 서버로 트래픽 릴레이
- `coturn` 서버 배포
- `config.json`의 `turn_server` 필드 활성화
- 뷰어 ICE 서버 설정에 TURN 추가

#### Protobuf 시그널링 (선택적)
- 뷰어에서도 Protobuf 사용 (`protobufjs` 라이브러리)
- Phase 1-4에서 사용한 JSON 시그널링을 완전히 대체하거나 공존

#### Electron 패키징
- `electron-builder` 설정
- Windows: NSIS 설치 프로그램 (`.exe`)
- macOS: DMG
- 자동 업데이트 (`electron-updater`)
- 코드 서명 (Windows: EV 인증서, macOS: Developer ID)

#### 성능 최적화
- VP8 → VP9 또는 H.264 하드웨어 인코딩 (NVENC, QuickSync)
- 적응형 비트레이트: 네트워크 상태에 따라 bitrate 조정
- 화면 변경 감지: 전체 프레임 캡처 → diff 영역만 인코딩

---

## 기술 부채 및 알려진 한계

| 항목 | 현재 상태 | 개선 시점 |
|------|-----------|-----------|
| 비밀번호 비교 | Argon2id (완료) | Phase 5 (Argon2id) |
| 시그널링 암호화 | WSS TLS 지원 (완료) | Phase 5 (wss://) |
| P2P 실패 처리 | TURN + coturn (완료) | Phase 5 (TURN) |
| 입력 좌표 정규화 | 미구현 | Phase 3 |
| 다중 모니터 | 첫 번째 모니터만 | Phase 4 이후 |
| 재연결 | 지수 백오프 재연결 (완료) | Phase 4 이후 |
| 세션 토큰 만료 | 5분 idle timeout (완료) | Phase 5 |

---

## Post-Phase 5 — 추가 개선 ✅

| 항목 | 상태 | 내용 |
|------|------|------|
| 자동 업데이터 | ✅ 완료 | electron-updater, GitHub Releases 연동 |
| 세션 idle timeout UI | ✅ 완료 | 클라이언트 5분 비활성 경고/만료, 서버 session_expired 메시지 |
| 세션 health check | ✅ 완료 | 호스트 30초마다 dead session 자동 정리 |
| Reconnect UX | ✅ 완료 | 재연결 오버레이, 연결 상태 표시등 |
| 코드 서명 | ⬜ 보류 | 인증서 필요 (EV cert / Apple Developer) |
