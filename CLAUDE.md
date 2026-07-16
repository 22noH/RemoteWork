# ERemote

TeamViewer류 소스 공개(BSL 1.1) 원격 데스크탑. Rust 호스트 + TypeScript 뷰어 하이브리드,
WebRTC P2P로 화면 공유·원격 제어. 비상업 무료 / 상업 유료 — [`LICENSE`](LICENSE).

## 구조

| 폴더 | 내용 |
|------|------|
| `proto/` | Protobuf 메시지 정의 (messages, input, file_transfer, chat) |
| `signaling-server/` | Rust WebSocket 시그널링 (JSON=뷰어 / Protobuf=호스트, TLS 필수) |
| `host-agent/` | Rust 호스트 — 화면 캡처(xcap+VP8), WebRTC, 입력(enigo), 오디오(cpal+opus), 호스트 GUI(egui) |
| `viewer-client/` | TypeScript + React + Vite + Electron 뷰어 |
| `deploy/` | 자체 배포 패키지 (nginx + TLS + certbot, opt-in TURN 포함) — `deploy/README.md` |
| `docs/` | 프로젝트 문서 (개요·Phase·빌드 가이드) |

## 로컬 실행

```bash
# 1. 시그널링 (TLS 없이는 시작 안 됨 → 로컬은 --insecure)
cd signaling-server && cargo run -- --insecure

# 2. 호스트 (화면 공유할 PC)
cd host-agent && cargo run

# 3. 뷰어 (원격 접속할 PC)
cd viewer-client && npm install && npm run dev
```

빌드 전제조건(protoc, libvpx/opus/OpenSSL, Windows env 등)과 상세 절차는
[`docs/04_SETUP_AND_BUILD.md`](docs/04_SETUP_AND_BUILD.md). 프로덕션 배포는
[`deploy/README.md`](deploy/README.md).

## Git 규칙

- 소스 변경은 **feature 브랜치 → PR → 머지**. `master` 직접 push 금지.
- 커밋 메시지에 공동 작업자(Co-authored-by) 제외.
