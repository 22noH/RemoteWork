# Remote Work

TeamViewer와 유사한 오픈소스 원격 데스크탑 애플리케이션.
Rust 호스트 + TypeScript 뷰어 하이브리드 구조로, WebRTC P2P 연결을 통해 실시간 화면 공유와 원격 제어를 제공한다.

## 구성

| 컴포넌트 | 설명 |
|----------|------|
| `signaling-server/` | Rust WebSocket 시그널링 서버 (Docker 지원) |
| `host-agent/` | Rust 호스트 에이전트 — 화면 캡처, WebRTC, 입력 주입 |
| `viewer-client/` | TypeScript + React + Electron 뷰어 |
| `proto/` | Protobuf 메시지 정의 |

## 빠른 시작

**전체 설정 및 빌드 방법은 [`docs/04_SETUP_AND_BUILD.md`](docs/04_SETUP_AND_BUILD.md) 를 참고한다.**

```bash
# 1. 시그널링 서버
cd signaling-server && cargo run

# 2. 호스트 에이전트 (화면을 공유할 PC)
cd host-agent && cargo run

# 3. 뷰어 (원격 접속할 PC)
cd viewer-client && npm install && npm run dev
```

## 문서

| 문서 | 내용 |
|------|------|
| [`docs/01_PROJECT_OVERVIEW.md`](docs/01_PROJECT_OVERVIEW.md) | 프로젝트 구조 및 기술 스택 상세 |
| [`docs/02_PHASE_PLAN.md`](docs/02_PHASE_PLAN.md) | Phase 1~5 전체 개발 계획 |
| [`docs/03_SUBAGENT_STRATEGY.md`](docs/03_SUBAGENT_STRATEGY.md) | AI 서브에이전트 운용 전략 |
| [`docs/04_SETUP_AND_BUILD.md`](docs/04_SETUP_AND_BUILD.md) | 개발 환경 설정 및 빌드 가이드 |

## 개발 현황

| Phase | 상태 | 내용 |
|-------|------|------|
| 1 | ✅ 완료 | Proto 정의, 시그널링 서버, Host/Viewer 스켈레톤 |
| 2 | ✅ 완료 | WebRTC 화면 공유 (xcap → VP8 → P2P 스트림) |
| 3 | 🔲 대기 | 원격 입력 제어 (enigo) |
| 4 | 🔲 대기 | 파일 전송, 채팅, 시스템 트레이 |
| 5 | 🔲 대기 | Argon2id 보안, TURN 릴레이, Electron 패키징 |

## 기술 스택

- **Signaling Server**: Rust, tokio, tungstenite, prost (Protobuf), serde_json
- **Host Agent**: Rust, xcap, vpx-encode, webrtc-rs 0.11, enigo
- **Viewer Client**: TypeScript, React 18, Vite, Electron, Zustand, WebRTC API
