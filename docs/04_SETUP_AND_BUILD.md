# Remote Work — 개발 환경 설정 및 빌드 가이드

이 문서는 프로젝트를 처음 받은 사람이 빌드하고 실행하기까지 필요한 모든 것을 담고 있다.

---

## 목차

1. [전체 구성 요약](#전체-구성-요약)
2. [공통 필수 설치](#공통-필수-설치)
3. [플랫폼별 네이티브 의존성](#플랫폼별-네이티브-의존성)
   - [Windows](#windows)
   - [Linux (Ubuntu/Debian)](#linux-ubuntudebian)
   - [macOS](#macos)
4. [컴포넌트별 빌드](#컴포넌트별-빌드)
   - [Signaling Server](#signaling-server)
   - [Host Agent](#host-agent)
   - [Viewer Client](#viewer-client)
5. [Docker로 시그널링 서버 실행](#docker로-시그널링-서버-실행)
6. [개발 환경에서 전체 실행하기](#개발-환경에서-전체-실행하기)
7. [환경 변수](#환경-변수)
8. [자주 발생하는 빌드 에러](#자주-발생하는-빌드-에러)

---

## 전체 구성 요약

이 프로젝트는 3개의 독립적인 컴포넌트로 구성된다.

| 컴포넌트 | 언어 | 빌드 도구 | 실행 환경 |
|----------|------|-----------|-----------|
| Signaling Server | Rust | cargo | 서버 (Linux 권장) / Docker |
| Host Agent | Rust | cargo | 화면을 공유하는 PC |
| Viewer Client | TypeScript | npm + vite | 원격으로 접속하는 PC (브라우저 또는 Electron) |

---

## 공통 필수 설치

### 1. Rust (1.75 이상)

```bash
# rustup 설치 (공식 방법)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 설치 후 확인
rustc --version   # rustc 1.75.0 이상
cargo --version
```

Windows는 https://rustup.rs 에서 `rustup-init.exe` 다운로드 후 실행.

> **Windows 주의:** GNU toolchain이 아닌 **MSVC toolchain**이 필요하다.
> 설치 중 "Visual Studio C++ Build Tools" 설치를 요구하면 반드시 설치한다.
> 없으면 `rustup default stable-x86_64-pc-windows-msvc` 로 수동 설정.

---

### 2. protoc (Protocol Buffer 컴파일러)

Rust의 `prost-build`가 빌드 시 `protoc` 바이너리를 직접 호출한다. **빌드 전에 반드시 설치해야 한다.**

**Windows:**
```powershell
# Chocolatey 사용
choco install protoc

# 또는 GitHub에서 직접 다운로드
# https://github.com/protocolbuffers/protobuf/releases
# protoc-XX.X-win64.zip 압축 해제 후 bin/ 을 PATH에 추가
```

**Linux:**
```bash
sudo apt install -y protobuf-compiler   # Ubuntu/Debian
sudo dnf install -y protobuf-compiler   # Fedora/RHEL
```

**macOS:**
```bash
brew install protobuf
```

설치 확인:
```bash
protoc --version   # libprotoc 3.x 이상
```

---

### 3. Node.js (18 LTS 이상) — Viewer Client용

```bash
# nvm 사용 권장 (버전 관리 편리)
nvm install 20
nvm use 20

# 또는 공식 설치 https://nodejs.org
node --version   # v18.0.0 이상
npm --version
```

---

### 4. Git

```bash
git clone <repository-url>
cd Remote_Work
```

---

## 플랫폼별 네이티브 의존성

### Windows

#### (A) Visual Studio Build Tools

Rust MSVC toolchain이 C/C++ 컴파일러를 필요로 한다.

1. https://visualstudio.microsoft.com/visual-cpp-build-tools/ 에서 설치
2. "C++ 빌드 도구" 워크로드 선택
3. 또는 Visual Studio 2019/2022 Community 설치 (C++ 워크로드 포함)

#### (B) vcpkg 설치 (libvpx + OpenSSL 관리)

```powershell
# 임의 위치에 vcpkg 설치 (예: C:\vcpkg)
git clone https://github.com/microsoft/vcpkg.git C:\vcpkg
cd C:\vcpkg
.\bootstrap-vcpkg.bat

# libvpx 설치 (vpx-encode 크레이트 의존성)
.\vcpkg install libvpx:x64-windows

# OpenSSL 설치 (webrtc-rs 의존성)
.\vcpkg install openssl:x64-windows

# libopus 설치 (오디오 크레이트 의존성)
.\vcpkg install opus:x64-windows

# 전역 통합 (Visual Studio / MSBuild가 자동으로 찾도록)
.\vcpkg integrate install
```

#### (C) 환경 변수 설정

PowerShell 또는 시스템 환경 변수에 추가:

```powershell
# vcpkg 설치 경로에 맞게 수정
$env:VCPKG_ROOT = "C:\vcpkg"
$env:OPENSSL_DIR = "C:\vcpkg\installed\x64-windows"
$env:OPENSSL_LIB_DIR = "C:\vcpkg\installed\x64-windows\lib"
$env:OPENSSL_INCLUDE_DIR = "C:\vcpkg\installed\x64-windows\include"
$env:VPX_VERSION = "1.13"
$env:VPX_INCLUDE_PATH = "C:\vcpkg\installed\x64-windows\include"
$env:VPX_LIB_PATH = "C:\vcpkg\installed\x64-windows\lib"
```

> 시스템 재시작 없이 적용하려면 현재 터미널 세션에 위 명령어를 실행한다.
> 영구 적용은 "시스템 속성 → 환경 변수"에서 설정.

---

### Linux (Ubuntu/Debian)

한 번에 설치:

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    protobuf-compiler \
    libvpx-dev \
    libssl-dev \
    libopus-dev \
    libx11-dev \
    libxext-dev \
    libxcb1-dev \
    libxcb-shm0-dev \
    libxcb-xfixes0-dev \
    libdbus-1-dev
```

| 패키지 | 용도 |
|--------|------|
| `libvpx-dev` | `vpx-encode` 크레이트 (VP8 인코딩) |
| `libssl-dev` | `webrtc-rs` 크레이트 |
| `libopus-dev` | `audio` 크레이트 (Opus 인코딩/디코딩) |
| `libx11-dev`, `libxext-dev` | `xcap` 크레이트 (X11 화면 캡처) |
| `libxcb*` | `xcap` Wayland/XCB 지원 |
| `libdbus-1-dev` | 시스템 트레이 (Phase 4) |

> Fedora/RHEL: `libvpx-devel`, `openssl-devel`, `libX11-devel`, `libXext-devel` 패키지명 사용

---

### macOS

```bash
brew install libvpx openssl opus pkg-config

# OpenSSL 경로 환경 변수 (Apple Silicon / Intel 모두)
export OPENSSL_DIR=$(brew --prefix openssl)
export PKG_CONFIG_PATH="$(brew --prefix libvpx)/lib/pkgconfig:$PKG_CONFIG_PATH"
```

`.zshrc` 또는 `.bash_profile`에 추가해 영구 적용.

> xcap는 macOS의 `ScreenCaptureKit` API를 사용하므로 추가 의존성 없음.
> 단, macOS 12.3 이상 필요.

---

## 컴포넌트별 빌드

### Signaling Server

```bash
cd signaling-server
cargo build --release
```

**출력:** `target/release/signaling-server` (또는 `.exe`)

개발 중 빠른 빌드:
```bash
cargo build          # debug 빌드 (느리지만 빠른 컴파일)
cargo check          # 컴파일 없이 타입 검사만 (가장 빠름)
```

---

### Host Agent

```bash
cd host-agent
cargo build --release
```

**출력:** `target/release/host-agent` (또는 `.exe`)

> `capture` 크레이트가 `libvpx`를 링크하고, `network` 크레이트가 OpenSSL을 링크한다.
> 위 플랫폼별 네이티브 의존성이 올바르게 설치되어 있어야 한다.

개발 중:
```bash
cargo check          # 빠른 타입 검사
cargo build          # debug 빌드
```

특정 크레이트만 체크:
```bash
cargo check -p network
cargo check -p capture
```

---

### Viewer Client

```bash
cd viewer-client

# 의존성 설치 (최초 1회)
npm install

# 개발 서버 실행 (브라우저)
npm run dev
# → http://localhost:5173 접속

# 프로덕션 빌드 (브라우저)
npm run build

# Electron 개발 실행 (Phase 5)
npm run electron:dev

# Electron 패키지 빌드 (Phase 5)
npm run electron:build
```

---

## Docker로 시그널링 서버 실행

로컬 개발이나 서버 배포 시 Docker를 사용하면 네이티브 의존성(protoc 등) 없이 실행할 수 있다.

```bash
# 프로젝트 루트에서 실행
docker compose up signaling-server

# 백그라운드 실행
docker compose up -d signaling-server

# 로그 확인
docker compose logs -f signaling-server
```

시그널링 서버는 `0.0.0.0:8080`에서 WebSocket을 수신한다.

> `docker-compose.yml`에 coturn(TURN 서버)도 정의되어 있으나 Phase 5 전까지는 사용하지 않는다.

---

## 개발 환경에서 전체 실행하기

세 개의 터미널을 열어 순서대로 실행한다.

### 터미널 1 — 시그널링 서버

```bash
cd signaling-server
RUST_LOG=debug cargo run
# 또는 Docker:
# docker compose up signaling-server
```

`Signaling server listening on ws://0.0.0.0:8080` 메시지 확인.

### 터미널 2 — Host Agent

```bash
cd host-agent
RUST_LOG=debug cargo run
```

최초 실행 시 `~/.config/remote-work/config.json`이 자동 생성된다:
```json
{
  "host_id": "123456789",
  "password": "AB3KP7",
  "signaling_server_url": "ws://localhost:8080",
  ...
}
```

터미널에 출력된 `host_id`와 `password`를 메모한다.

### 터미널 3 — Viewer Client

```bash
cd viewer-client
npm run dev
```

브라우저에서 `http://localhost:5173` 접속 → Host ID와 Password 입력 → Connect.

---

## 환경 변수

### Signaling Server

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `LISTEN_ADDR` | `0.0.0.0:8080` | WebSocket 수신 주소:포트 |
| `RUST_LOG` | (없음) | 로그 레벨 (`debug`, `info`, `warn`, `error`) |

예시:
```bash
LISTEN_ADDR=0.0.0.0:9000 RUST_LOG=info ./signaling-server
```

### Host Agent

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `SIGNALING_URL` | `ws://localhost:8080` | 시그널링 서버 URL |
| `RUST_LOG` | (없음) | 로그 레벨 |
| `TURN_URL` | (없음) | TURN 서버 URL (coturn) |
| `TURN_USERNAME` | (없음) | TURN 사용자명 |
| `TURN_CREDENTIAL` | (없음) | TURN 비밀번호 |

예시 (원격 서버 연결):
```bash
SIGNALING_URL=ws://myserver.com:8080 RUST_LOG=info ./host-agent
```

> `SIGNALING_URL`은 `config.json`에도 설정 가능하다. 환경 변수가 우선 적용된다.

### Viewer Client

`viewer-client/` 루트에 `.env` 파일 생성:

```env
VITE_SIGNALING_URL=ws://localhost:8080
```

---

## 자주 발생하는 빌드 에러

### `cannot find -lvpx` 또는 `vpx/vpx_encoder.h not found`

**원인:** libvpx가 설치되지 않았거나 경로를 못 찾음

**Windows 해결:**
```powershell
.\vcpkg install libvpx:x64-windows
$env:VPX_LIB_PATH = "C:\vcpkg\installed\x64-windows\lib"
$env:VPX_INCLUDE_PATH = "C:\vcpkg\installed\x64-windows\include"
```

**Linux 해결:**
```bash
sudo apt install libvpx-dev
```

---

### `openssl/ssl.h not found` 또는 `failed to find OpenSSL`

**원인:** OpenSSL 개발 헤더가 없거나 `OPENSSL_DIR`이 설정되지 않음

**Windows 해결:**
```powershell
.\vcpkg install openssl:x64-windows
$env:OPENSSL_DIR = "C:\vcpkg\installed\x64-windows"
```

**Linux 해결:**
```bash
sudo apt install libssl-dev
```

**macOS 해결:**
```bash
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)
```

---

### `protoc` 관련 빌드 에러

```
Error: Custom { kind: Other, error: "protoc failed: ..." }
```

**해결:** `protoc`가 PATH에 없는 경우.
```bash
protoc --version   # 이 명령이 안 되면 protoc가 없는 것
```
위 [공통 필수 설치 → protoc](#2-protoc-protocol-buffer-컴파일러) 섹션 참고.

---

### Windows: `LINK : fatal error LNK1181`

**원인:** `.lib` 파일 경로를 찾지 못함

**해결:** vcpkg integrate가 제대로 됐는지 확인
```powershell
C:\vcpkg\vcpkg integrate install
```
그래도 안 되면 환경 변수를 직접 지정:
```powershell
$env:LIB = "C:\vcpkg\installed\x64-windows\lib;$env:LIB"
```

---

### Viewer: `npm install` 후 vite 실행 안 됨

```bash
# node_modules 완전 초기화
rm -rf node_modules package-lock.json
npm install
npm run dev
```

---

### Host Agent: 화면 캡처 권한 오류 (macOS)

macOS 12 이상에서 화면 캡처 권한이 필요하다.
- 시스템 설정 → 개인 정보 보호 및 보안 → 화면 기록
- `host-agent` 바이너리(또는 터미널)를 허용 목록에 추가

---

## 빠른 시작 체크리스트

```
□ Rust stable 설치 (rustup)
□ Windows: Visual Studio Build Tools (MSVC) 설치
□ protoc 설치 및 PATH 확인
□ Node.js 18+ 설치

플랫폼별:
□ Windows: vcpkg → libvpx:x64-windows, openssl:x64-windows 설치
           환경 변수 OPENSSL_DIR, VPX_LIB_PATH 설정
□ Linux:   apt install libvpx-dev libssl-dev libx11-dev libxcb1-dev
□ macOS:   brew install libvpx openssl, export OPENSSL_DIR=...

빌드:
□ cd signaling-server && cargo build --release
□ cd host-agent && cargo build --release
□ cd viewer-client && npm install && npm run dev

실행:
□ 시그널링 서버 기동 확인 (포트 8080)
□ 호스트 에이전트 실행 → host_id / password 확인
□ 뷰어 접속 → ID + 비밀번호 입력 → 연결 성공

Electron 패키지 빌드 (선택):
□ cd viewer-client && npm run electron:build
  (아이콘: resources/icon.ico, icon.icns, icon.png 필요)
```
