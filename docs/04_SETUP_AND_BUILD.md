# ERemote — 개발 환경 설정 및 빌드 가이드

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
5. [시그널링 서버 실행 (로컬/프로덕션)](#시그널링-서버-실행-로컬프로덕션)
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
$env:VCPKGRS_DYNAMIC = "1"                       # vcpkg libvpx 동적 링크
$env:OPENSSL_DIR = "C:\vcpkg\installed\x64-windows"
$env:OPENSSL_LIB_DIR = "C:\vcpkg\installed\x64-windows\lib"
$env:OPENSSL_INCLUDE_DIR = "C:\vcpkg\installed\x64-windows\include"
$env:VPX_VERSION = "1.13"
$env:VPX_INCLUDE_PATH = "C:\vcpkg\installed\x64-windows\include"
$env:VPX_LIB_PATH = "C:\vcpkg\installed\x64-windows\lib"

# protoc 경로 (prost-build)
$env:PROTOC = "C:\ProgramData\chocolatey\bin\protoc.exe"

# vpx-sys bindgen용 libclang — 반드시 x64 (ARM64 아님)
$env:LIBCLANG_PATH = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin"

# pkgconf (vcpkg libvpx 탐색)
$env:PKG_CONFIG = "C:\vcpkg\installed\x64-windows\tools\pkgconf\pkgconf.exe"
$env:PKG_CONFIG_PATH = "C:\vcpkg\installed\x64-windows\lib\pkgconfig"

# audiopus_sys가 번들 opus를 빌드할 때 CMake 4.x가 pre-3.5 최소 버전을 거부하는 문제 회피
$env:CMAKE_POLICY_VERSION_MINIMUM = "3.5"
```

> **중요:** `host-agent`를 Windows에서 `cargo build`/`cargo check` 하려면 위 변수들이 모두 필요하다.
> - `PROTOC`: `prost-build`가 protoc 바이너리를 호출한다.
> - `LIBCLANG_PATH`: `vpx-sys`의 bindgen이 사용한다. 반드시 Visual Studio LLVM **x64** bin을 가리켜야 하며 ARM64 libclang을 쓰면 안 된다.
> - `VCPKGRS_DYNAMIC=1` + `VCPKG_ROOT`: vcpkg의 libvpx를 동적 링크한다.
> - `PKG_CONFIG` / `PKG_CONFIG_PATH`: pkgconf가 vcpkg에 설치된 libvpx를 찾도록 한다.
> - `CMAKE_POLICY_VERSION_MINIMUM=3.5`: `audiopus_sys`가 번들 opus를 빌드하며, 그 CMakeLists의 pre-3.5 최소 버전을 CMake 4.x가 거부하는 것을 우회한다.
>
> `signaling-server`만 빌드할 때는 `PROTOC`만 있으면 된다. 오디오 지원을 위해 vcpkg에 `opus:x64-windows`도 필요하다.

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
| `libdbus-1-dev` | 호스트 GUI (egui) 및 데스크톱 통합 |

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

## 시그널링 서버 실행 (로컬/프로덕션)

> ⚠️ **시그널링 서버는 TLS 없이는 시작되지 않는다.** 반드시 다음 중 하나를 선택해야 한다.
> - 프로덕션(WSS): `--tls-cert`와 `--tls-key`를 함께 전달
> - 로컬 개발(평문 WS): `--insecure` 전달 (비밀번호가 평문으로 오가므로 개발 전용)
>
> 옵션 없이 `cargo run` 또는 `./signaling-server`만 실행하면 에러와 함께 종료된다.

### 로컬 개발

프로젝트 루트의 `docker-compose.yml`은 제거되었다. 로컬에서는 cargo로 직접 실행한다.

```bash
cd signaling-server
cargo run -- --insecure
```

시그널링 서버는 `0.0.0.0:8080`에서 평문 WebSocket(`ws://`)을 수신한다.

### 프로덕션 배포

프로덕션 배포는 `deploy/` 폴더의 패키지로 구성된다. 자세한 절차는 `deploy/README.md`를 참고한다.

- **nginx**가 TLS를 종단(terminate)하고, 뷰어 SPA를 서빙하며, `/signal` 경로를 시그널링 서버로 리버스 프록시한다. (시그널링 서버는 프라이빗 Docker 네트워크 안에서 `--insecure`로 동작한다.)
- **certbot**이 Let's Encrypt 인증서를 자동 갱신한다.
- **coturn**(TURN 서버)은 compose 프로파일로 opt-in이다: `docker compose --profile turn up -d`

```bash
cd deploy
cp .env.example .env        # DOMAIN 과 이메일 설정
sh init-letsencrypt.sh      # 최초 인증서 발급
docker compose up -d
```

> 과거의 루트에서 실행하던 `docker compose up signaling-server` 명령은 더 이상 동작하지 않는다.

---

## 개발 환경에서 전체 실행하기

세 개의 터미널을 열어 순서대로 실행한다.

### 터미널 1 — 시그널링 서버

```bash
cd signaling-server
RUST_LOG=debug cargo run -- --insecure
```

> `--insecure`는 로컬 개발용(평문 WS)이다. 이 옵션이나 `--tls-cert`/`--tls-key` 중 하나가 없으면 서버가 시작되지 않는다.

`Signaling server listening on ws://0.0.0.0:8080` 메시지 확인.

### 터미널 2 — Host Agent

```bash
cd host-agent
RUST_LOG=debug cargo run
```

최초 실행 시 OS별 설정 경로에 `config.json`이 자동 생성된다 (`dirs::config_dir()/remote-work/config.json`):

| OS | 경로 |
|----|------|
| Windows | `%APPDATA%\remote-work\config.json` |
| Linux | `~/.config/remote-work/config.json` |
| macOS | `~/Library/Application Support/remote-work/config.json` |

```json
{
  "host_id": "123456789",
  "signaling_server_url": "ws://localhost:8080",
  "stun_servers": ["..."],
  "turn_server": null,
  "allowed_dirs": ["..."],
  "allow_control": true
}
```

> **일회용 비밀번호:** 호스트 비밀번호는 매 실행마다 새로 생성되며 디스크에 저장되지 않는다(`serde` skip). 따라서 `config.json`에는 비밀번호 필드가 없다. 유출되어도 세션이 끝나면 쓸모가 없다.

`host_id`는 최초 실행 시 생성되어 유지되는 9자리 숫자다. 실행 시 **호스트 GUI (egui)** 창에 표시되는 Host ID와 **일회용 비밀번호**를 확인해 뷰어에 입력한다.

### 터미널 3 — Viewer Client

```bash
cd viewer-client
npm run dev
```

브라우저에서 `http://localhost:5173` 접속 → Host ID와 Password 입력 → Connect.

---

## 환경 변수

### Signaling Server

| 변수 | 대응 CLI 플래그 | 기본값 | 설명 |
|------|----------------|--------|------|
| `LISTEN_ADDR` | `--listen` | `0.0.0.0:8080` | WebSocket 수신 주소:포트 |
| `TLS_CERT` | `--tls-cert` | (없음) | TLS 인증서 경로 (WSS, 프로덕션) |
| `TLS_KEY` | `--tls-key` | (없음) | TLS 개인 키 경로 (WSS, 프로덕션) |
| `ALLOW_INSECURE` | `--insecure` | (없음) | 평문 WS 허용 (개발 전용) |
| `RUST_LOG` | — | (없음) | 로그 레벨 (`debug`, `info`, `warn`, `error`) |

> CLI는 `clap` 기반이다. TLS 없이는 서버가 시작되지 않으므로 `--tls-cert`+`--tls-key`(프로덕션) 또는 `--insecure`(개발) 중 하나가 반드시 필요하다.

예시:
```bash
# 로컬 개발 (평문 WS)
LISTEN_ADDR=0.0.0.0:9000 RUST_LOG=info ./signaling-server --insecure

# 프로덕션 (WSS)
TLS_CERT=/etc/ssl/cert.pem TLS_KEY=/etc/ssl/key.pem ./signaling-server
```

### Host Agent

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `SIGNALING_URL` | `ws://localhost:8080` | 시그널링 서버 URL |
| `ALLOW_CONTROL` | `1` (true) | 뷰 전용 모드 제어. `0`/`false`면 입력을 무시(뷰 전용) |
| `RUST_LOG` | (없음) | 로그 레벨 |
| `TURN_URL` | (없음) | TURN 서버 URL (coturn) |
| `TURN_USERNAME` | (없음) | TURN 사용자명 |
| `TURN_CREDENTIAL` | (없음) | TURN 비밀번호 |

예시 (프로덕션 WSS 서버 연결):
```bash
SIGNALING_URL=wss://myserver.com/signal RUST_LOG=info ./host-agent
```

> `ALLOW_CONTROL`은 `config.json`의 `allow_control` 필드에 대응하며, 호스트 GUI (egui)의 뷰 전용 모드 토글로도 켜고 끌 수 있다.

> `SIGNALING_URL`은 `config.json`에도 설정 가능하다. 환경 변수가 우선 적용된다.

### Viewer Client

뷰어는 시그널링 URL을 자신의 페이지 origin에서 자동으로 유추한다. 따라서 도메인마다 다시 빌드할 필요 없이 하나의 빌드가 어떤 도메인에서도 동작한다.

- 배포 환경: `wss://<host>/signal`
- localhost: `ws://localhost:8080`
- `VITE_SIGNALING_URL`이 설정된 경우: 해당 값을 사용

로컬 개발에서 다른 서버를 강제하려면 `viewer-client/` 루트에 `.env` 파일을 만든다:

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
□ Windows: vcpkg → libvpx:x64-windows, openssl:x64-windows, opus:x64-windows 설치
           환경 변수 OPENSSL_DIR, VPX_LIB_PATH, VCPKGRS_DYNAMIC=1,
           PROTOC, LIBCLANG_PATH(x64), PKG_CONFIG, PKG_CONFIG_PATH,
           CMAKE_POLICY_VERSION_MINIMUM=3.5 설정
□ Linux:   apt install libvpx-dev libssl-dev libx11-dev libxcb1-dev
□ macOS:   brew install libvpx openssl, export OPENSSL_DIR=...

빌드:
□ cd signaling-server && cargo build --release
□ cd host-agent && cargo build --release
□ cd viewer-client && npm install && npm run dev

실행:
□ 시그널링 서버 기동 확인 (cargo run -- --insecure, 포트 8080)
  (TLS 없이는 시작 안 됨: 개발은 --insecure, 프로덕션은 --tls-cert/--tls-key)
□ 호스트 에이전트 실행 → 호스트 GUI (egui)에서 Host ID / 일회용 비밀번호 확인
□ 뷰어 접속 → ID + 일회용 비밀번호 입력 → 호스트 GUI 연결 승인(Allow) → 연결 성공

Electron 패키지 빌드 (선택):
□ cd viewer-client && npm run electron:build
  (아이콘: resources/icon.ico, icon.icns, icon.png 필요)
```
