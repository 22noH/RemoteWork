# ERemote — 서브에이전트 운용 전략

> ⚠️ **이 문서는 개발 당시의 기록이다.** 이후 실제 구현이 바뀐 부분이 있다 —
> 대표적으로 Phase 4의 "시스템 트레이"는 최종적으로 **호스트 GUI(egui, `ui.rs`)** 로 대체되었다.
> 아래 내용은 그때의 계획/전략을 그대로 남긴 것이며, 현재 상태는 [`../README.md`](../README.md)와
> [`04_SETUP_AND_BUILD.md`](04_SETUP_AND_BUILD.md)를 참고한다.

## 개요

이 프로젝트는 Claude Code의 서브에이전트(Sub-agent) 기능을 활용해 Phase별 구현을 병렬로 처리한다. 각 Phase마다 작업 영역이 겹치지 않는 에이전트 여러 개를 동시에 실행해 개발 속도를 극대화한다.

---

## Phase 2에서 사용한 전략 (실제 사례)

### 병렬 실행 3개 에이전트

```
Main Thread
    ├── Agent A (백그라운드): Signaling Server 듀얼 프로토콜
    ├── Agent B (백그라운드): Host Agent WebRTC + 캡처 파이프라인
    └── Agent C (백그라운드): Viewer Client 스트림 연결
         ↓ 모두 완료 대기
    └── Main Thread: 통합 검증 (cargo check, TypeScript 확인)
```

**총 소요 시간:** 에이전트 3개 순차 실행 대비 약 3배 빠름 (가장 오래 걸린 Agent B 시간이 전체 시간)

---

### Agent A: Signaling Server 듀얼 프로토콜

**담당 파일 (수정/신규):**
- `signaling-server/src/main.rs` — `mod json_protocol` 추가
- `signaling-server/src/json_protocol.rs` — **신규** JSON serde 타입 + 빌더 함수
- `signaling-server/src/ws_server.rs` — 전체 리라이트

**제공한 컨텍스트:**
- `ws_server.rs` 현재 전체 코드 (Protobuf only)
- `messages.proto` 전체 (Envelope oneof 구조)
- `signaling.ts`의 JSON 메시지 형식 (`{ type, payload }`)
- 뷰어가 기대하는 응답 JSON 형식

**에이전트에게 명시한 제약:**
- `session_registry.rs`, `relay.rs` 수정 금지 (기존 `mpsc::UnboundedSender<Vec<u8>>` 인터페이스 유지)
- `serde`, `serde_json`은 이미 Cargo.toml에 있음 — 추가 불필요

---

### Agent B: Host Agent WebRTC + 캡처 파이프라인

**담당 파일 (수정/신규):**
- `host-agent/Cargo.toml` — `webrtc = "0.11"`, `bytes` workspace 추가
- `host-agent/crates/network/Cargo.toml` — `webrtc`, `sha2`, `hex`, `bytes` 추가; `auth` 제거
- `host-agent/crates/network/src/lib.rs` — `SignalingEvent`, `HostPeerConnection` export
- `host-agent/crates/network/src/signaling_client.rs` — 전체 리라이트
- `host-agent/crates/network/src/peer_connection.rs` — 전체 리라이트
- `host-agent/src/app.rs` — 전체 리라이트

**제공한 컨텍스트:**
- network 크레이트 전체 현재 코드
- capture 크레이트 전체 코드 (Capturer, Encoder, Frame, EncodedFrame API)
- 현재 `app.rs`, `config.rs` 코드
- `messages.proto` Envelope 구조
- auth 불일치 문제 및 SHA-256으로 통일해야 하는 이유

**에이전트에게 명시한 제약:**
- `auth` 크레이트 수정 금지 (Argon2id는 Phase 5용)
- `auth::hash_password` 사용 금지 — `sha2 + hex`로 직접 구현
- `capture` 크레이트 수정 금지 — API만 사용

---

### Agent C: Viewer Client 스트림 연결

**담당 파일 (수정):**
- `viewer-client/src/stores/connection-store.ts`
- `viewer-client/src/core/peer-connection.ts`
- `viewer-client/src/components/RemoteScreen.tsx`

**제공한 컨텍스트:**
- 3개 파일 현재 전체 코드
- `peerConnection.ontrack` 충돌 문제 설명 (컴포넌트와 클래스가 동시에 설정하는 문제)
- 원하는 결과: store의 `remoteStream`을 통한 단방향 데이터 흐름

**에이전트에게 명시한 제약:**
- 변경 최소화 — `onStreamCb` 패턴 제거 금지 (하위 호환 유지)
- `disconnect()` 시 `remoteStream: null` 초기화 필수

---

## 에이전트 실행 원칙

### 1. 파일 영역 분리 (핵심)
에이전트 간에 수정하는 파일이 겹치면 충돌이 발생한다. 에이전트를 설계할 때 반드시 각 에이전트의 **담당 파일 목록이 겹치지 않도록** 분리해야 한다.

```
✅ 좋은 분리:
Agent A: signaling-server/src/*
Agent B: host-agent/crates/network/src/*, host-agent/src/app.rs
Agent C: viewer-client/src/*

❌ 나쁜 분리 (충돌 위험):
Agent A: host-agent/src/app.rs  ← 같은 파일
Agent B: host-agent/src/app.rs  ← 충돌!
```

### 2. 의존 관계가 없는 작업만 병렬 실행
에이전트 A의 출력이 에이전트 B의 입력이 되는 경우 순차 실행해야 한다.

```
병렬 가능:
- 서버 코드 변경 + 클라이언트 코드 변경
- Rust 백엔드 + TypeScript 프론트엔드

순차 필요:
- 인터페이스 정의 → 인터페이스 구현 (구현체가 정의에 의존)
- 빌드 에러 수정 → 테스트 실행
```

### 3. 컨텍스트를 충분히 제공
에이전트는 이전 대화 컨텍스트를 가지지 않는다. 에이전트 프롬프트에 반드시 포함해야 할 것:
- 수정 대상 파일의 **현재 전체 코드**
- 연관 파일의 **API/인터페이스** (전체 코드가 아니어도 됨)
- 변경해야 하는 **이유와 제약 조건**
- 수정하지 말아야 할 파일 명시

### 4. 제약 조건 명시
에이전트가 과도하게 리팩토링하거나 불필요한 파일을 수정하지 않도록 명확한 경계를 설정한다:
- "X 파일은 수정하지 말 것"
- "Y 인터페이스는 유지할 것"
- "Z 크레이트는 이미 Cargo.toml에 있으므로 추가 불필요"

### 5. Main Thread에서 통합 검증
에이전트들이 모두 완료된 후 Main Thread에서:
1. 생성된 파일 검토 (논리적 오류, 경계 조건)
2. `cargo check` — 컴파일 에러 확인 및 수정
3. TypeScript 타입 체크 (`tsc --noEmit`)
4. 필요한 경우 수동 패치

---

## Phase별 서브에이전트 계획

### Phase 3 — 원격 제어

```
Agent A: Host Input Handler
  담당: host-agent/crates/input/src/handler.rs (리라이트)
        host-agent/crates/input/Cargo.toml (enigo 추가)
        host-agent/crates/network/src/peer_connection.rs (ondatachannel 추가)

Agent B: Viewer Input Sender
  담당: viewer-client/src/core/input-sender.ts (구현)
        viewer-client/src/components/RemoteScreen.tsx (이벤트 핸들러 추가)

Main Thread 후속:
  - app.rs에 InputHandler 연결
  - 좌표 정규화 로직 통합
```

### Phase 4 — 파일 전송 + 채팅 + 트레이

```
Agent A: 파일 전송 (Host)
  담당: host-agent/crates/file_transfer/src/*

Agent B: 파일 전송 (Viewer)
  담당: viewer-client/src/core/file-transfer.ts
        viewer-client/src/components/FileTransfer.tsx
        viewer-client/src/stores/file-transfer-store.ts

Agent C: 채팅
  담당: viewer-client/src/core/chat.ts
        viewer-client/src/components/ChatPanel.tsx
        viewer-client/src/stores/chat-store.ts
        (host chat handler는 app.rs에서 간단히 처리)

Agent D: 시스템 트레이 (Host)
  담당: host-agent/src/tray.rs
```

### Phase 5 — 보안 + 패키징

```
Agent A: Argon2id 인증 업그레이드
  담당: signaling-server/src/auth.rs (verify_password → Argon2id)
        host-agent/crates/network/src/signaling_client.rs (auth::hash_password 복원)
        viewer-client: argon2-browser 통합

Agent B: TURN 서버 통합
  담당: host-agent/crates/network/src/peer_connection.rs (TURN 설정)
        viewer-client/src/core/peer-connection.ts (TURN 설정)

Agent C: Electron 패키징
  담당: viewer-client/electron/*
        viewer-client/package.json (electron-builder 설정)
```

---

## 에이전트 프롬프트 템플릿

```
당신은 [프로젝트명] Phase [N]를 구현하는 에이전트입니다.
담당 작업: [한 줄 요약]

## 수정할 파일
- [파일 경로 1] — [변경 내용]
- [파일 경로 2] — [신규 생성]

## 수정하지 말 것
- [파일 경로 3] — [이유]
- [파일 경로 4] — [이유]

## 현재 코드
### [파일 경로 1]:
[전체 코드 붙여넣기]

## 연관 API (참고용)
[관련 크레이트/모듈의 공개 인터페이스]

## 구현 요구사항
1. [구체적 요구사항]
2. [경계 조건]
3. [주의사항]

## 중요 제약
- [라이브러리 버전, 인터페이스 호환성 등]
- 코드는 cargo check / tsc 를 통과해야 함
```

---

## 트러블슈팅

### 에이전트가 잘못된 파일을 수정한 경우
- `git diff` 또는 파일 읽기로 변경 내용 확인
- 잘못된 변경은 직접 되돌리거나 수정
- 다음 에이전트 프롬프트에 "X는 수정하지 말 것" 명시

### 에이전트가 컴파일 오류를 낸 경우
- Main Thread에서 `cargo check` 오류 메시지 읽기
- 오류 파일만 직접 수정하거나 에이전트를 재실행
- 재실행 시 오류 메시지와 현재 파일 코드를 컨텍스트로 제공

### 에이전트 간 인터페이스 불일치
- Agent A가 정의한 타입을 Agent B가 다르게 사용하는 경우
- Main Thread에서 불일치 파악 후 한쪽을 수정
- 다음 Phase에서는 인터페이스 정의 에이전트를 먼저 실행 후 구현 에이전트 실행
