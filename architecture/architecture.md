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
- 인증: 일회용 비밀번호 검증
- 세션 관리: DashMap 기반, 5분 idle timeout
- idle 4m30s: `session_timeout_warning` 전송 / 5m: `session_expired` 전송 후 종료
- TLS 필수: `--tls-cert` / `--tls-key`(운영 WSS) 또는 `--insecure`(로컬 개발, 평문 WS) 중 하나를 반드시 지정. 둘 다 없으면 서버는 시작을 거부한다. 로컬 실행: `cargo run -- --insecure`
- CLI: `--listen`(env `LISTEN_ADDR`, 기본 `0.0.0.0:8080`), `--tls-cert`(env `TLS_CERT`), `--tls-key`(env `TLS_KEY`), `--insecure`(env `ALLOW_INSECURE`)

### Host Agent (`host-agent/`)
- 화면 캡처 → VP8 인코딩 → WebRTC video track으로 전송
- 입력 수신 → enigo로 OS 이벤트 주입
- 오디오: cpal 마이크 캡처 → Opus 인코딩 → WebRTC audio track
- 채팅/파일: Protobuf DataChannel
- 호스트 GUI (egui): Host ID + 일회용 비밀번호 표시, 연결 승인 프롬프트(Allow 시에만 세션 성립), 뷰 전용 모드 토글, 단일 창 채팅, 파일 수신 승인(대상 경로 표시). 창 닫기(X)는 작업표시줄로 최소화
- 1:1 연결 강제: 활성 세션이 있으면 두 번째 뷰어의 SDP offer를 거부(다중 동시 제어 방지)
- 모니터 선택: 호스트가 `"control"` 채널로 모니터 목록 전송 → 뷰어가 선택 → 호스트가 캡처러/인코더 재생성
- 재연결: 지수 백오프(1s→30s), 시그널링 드롭 후 세션 유지
- 세션 health: 30초마다 Failed/Closed 세션 정리

### Viewer Client (`viewer-client/`)
- React + Electron (또는 브라우저)
- 원격 화면 표시: `<video>` + WebRTC MediaStream
- 입력 전송: 마우스/키보드 → Protobuf → `"input"` DataChannel
- 채팅/파일: `ChatManager` / `FileTransferManager`
- 모니터 선택: `"control"` 채널의 모니터 목록에서 볼 모니터 선택
- 시그널링 URL 자동 도출: 페이지 origin 기준(`wss://<host>/signal` 배포 시, `ws://localhost:8080` 로컬), 또는 `VITE_SIGNALING_URL` 지정. 하나의 빌드로 모든 도메인에서 동작(도메인별 재빌드 불필요)
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
| `"control"` | Host→Viewer | - | 모니터 목록 / 제어 |

## 보안
- 일회용 비밀번호: 실행할 때마다 새로 생성, 디스크에 저장하지 않음(serde skip). 세션 종료 후 유출된 비밀번호는 무의미
- 1:1 연결: 동시에 한 명의 뷰어만 활성 세션 허용
- 전송 암호화: WSS(TLS) + WebRTC DTLS
- mDNS 비활성화: 호스트 WebRTC가 `MulticastDnsMode::Disabled` 설정(Windows의 mDNS 로그 스팸 방지)
- 파일 수신: 호스트 승인(Accept/Deny) 후 OS Downloads 폴더에 저장, SHA-256 검증
- 속도 제한: 5회 인증 실패 → 10분 차단

## 설정 (config.json)
- 경로: `dirs::config_dir()/remote-work/config.json`
  - Windows: `%APPDATA%\remote-work\config.json`
  - Linux: `~/.config/remote-work/config.json`
  - macOS: `~/Library/Application Support/remote-work/config.json`
- 항목: `host_id`(영구 9자리), `signaling_server_url`, `stun_servers`, `turn_server`, `allowed_dirs`, `allow_control`
- 비밀번호는 일회용이라 config.json에 저장되지 않음
- `allow_control`: 뷰 전용 모드 제어(기본 true, env `ALLOW_CONTROL=0/false`로 비활성화). off일 때 뷰어는 화면만 보고 입력은 무시됨

## 배포
- 운영 배포는 `deploy/` 폴더에 위치. 상세는 `deploy/README.md` 참고
  - nginx가 TLS 종료, 뷰어 SPA 서빙, `/signal`을 시그널링 서버로 리버스 프록시(내부에서는 `--insecure`로 사설 Docker 네트워크 위에서 동작)
  - certbot이 Let's Encrypt 인증서 자동 갱신
  - coturn(TURN)은 compose 프로파일로 옵트인: `docker compose --profile turn up -d`
- 배포 순서: `cd deploy && cp .env.example .env`(DOMAIN + email 설정) → `sh init-letsencrypt.sh` → `docker compose up -d`
- 로컬 시그널링 서버는 `cargo run -- --insecure`

## GitHub
- Repository: https://github.com/22noH/RemoteWork.git
- Branch: master
