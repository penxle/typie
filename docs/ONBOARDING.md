# Typie 온보딩 문서

작성일: 2026-05-18
대상: 입사 첫 날, 이 저장소를 처음 클론한 엔지니어 또는 그를 보조하는 AI
에이전트.

이 문서는 신규 입사자가 첫 날 안에 다음 세 가지를 완수하도록 안내한다.

1. 로컬에서 `apps/website`와 `apps/api`를 기동한다.
2. 클라이언트, BFF, API, 도메인 서비스로 이어지는 요청 흐름을 설명할 수 있다.
3. 작은 변경을 만들어 lint를 통과시키고 PR을 등록한다.

[로컬 개발 환경 구성](#로컬-개발-환경-구성) 섹션은 AI 에이전트가 순차적으로
실행할 수 있는 형식으로 기술되어 있다. 사용자가 직접 진행해도 동일한 절차를
따른다.

## 온보딩 체크리스트

아래 항목을 위에서 아래로 완료하면 첫 날 목표를 달성한 것으로 본다. 괄호 안의
단계 번호는 [로컬 개발 환경 구성](#로컬-개발-환경-구성)의 하위 단계를 가리킨다.

### 에이전트 실행 규칙

AI 에이전트가 이 체크리스트를 진행할 때 따라야 할 규칙이다.

1. 항목은 **위에서 아래로 순차 실행**한다. 이전 항목의 검증이 통과해야 다음
   항목으로 넘어간다.
2. 항목 본문에 명시된 명령을 그대로 실행하고, 검증 명령의 기대 출력과 일치할
   때만 해당 체크박스를 `- [x]`로 갱신한다.
3. 검증이 실패하면 해당 항목을 `- [ ]`로 유지하고, "실패 시" 분기 또는
   [트러블슈팅](#트러블슈팅)을 따른다. 분기 처리도 실패하면 사용자에게 보고하고
   멈춘다.
4. 사용자 개입이 필요한 항목(예: 단계 6 Doppler 연결, 단계 8 Graphite 인증)은
   자동화하지 않고 사용자에게 실행을 요청한다.
5. **세션 종료 시 모든 체크박스를 다시 `- [ ]`로 되돌린다.** 체크리스트는 다음
   입사자가 동일한 절차로 사용해야 하므로, 진행 상태가 파일에 남으면 안 된다.

### 체크리스트

- [ ] 단계 1 완료: `just` 설치 (Node/pnpm 버전 매니저는 선호하는 것 자유 선택)
- [ ] 단계 2 완료: `rustup` 설치, stable toolchain 활성화 확인
- [ ] 단계 3 완료: `doppler` CLI 설치 및 검증
  - [ ] 단계 3-A. `brew install dopplerhq/cli/doppler` 시도
  - [ ] 단계 3-B. 위가 Command Line Tools 버전 오류로 실패하면 공식 셸
        인스톨러로 우회 (`brew install gnupg` → `curl ... | sudo sh`, 사용자
        직접 실행)
- [ ] 단계 4 완료: Node 25.8.1, pnpm 10.32.1 준비 후 `pnpm install`, `pnpm run bootstrap` 성공
- [ ] 단계 5 완료: Tailscale에서 `penxle.io` tailnet 연결 확인 (`apps/api` 기동에 필수)
- [ ] 단계 6 완료: `doppler login` → `doppler setup` → `doppler me`로 워크스페이스 접근 확인
- [ ] 단계 7 완료: `caddy` 설치 및 검증
- [ ] 단계 8 완료: Graphite CLI(`gt`) 설치 + `gt auth` 인증 + `gt repo init` 완료
- [ ] 단계 9 완료: `pnpm run dev`로 `apps/website`, `apps/api`, `apps/caddy` 기동
- [ ] 브라우저에서 `http://localhost:4100` 접속 후 로그인 흐름 통과
- [ ] Graphite CLI(`gt`)로 작은 변경 브랜치를 만들어 PR 등록 후 `ci.yml` 통과
      (raw `git checkout/commit/push/pull/rebase`는 사용 금지)
- [ ] [요청 흐름](#요청은-어떻게-처리되는가)을 다이어그램 없이 설명 가능

이후에는 실제 티켓을 할당받아 작업한다.

---

## Typie는 어떤 서비스이고, 이 저장소에는 무엇이 있는가

이 섹션은 제품과 저장소의 전체 구조를 먼저 익히기 위한 것이다. 디렉터리 구조와
책임을 파악해두면 이후 코드를 탐색할 때 방향을 잡기 쉽다.

Typie는 작가를 위한 글쓰기 SaaS다. 핵심 자산은 Rust로 작성된 에디터 엔진이며,
WASM(브라우저, Node 서버)과 UniFFI(Android, iOS) 양쪽으로 빌드된다. 모든
클라이언트가 동일한 문서 모델, CRDT, 렌더 경로를 공유한다.

저장소의 나머지 코드는 이 엔진을 둘러싼 호스트 영역이다. 책임별로 보면 다음과
같다.

| 영역                              | 위치                                      |
| --------------------------------- | ----------------------------------------- |
| 편집 UI, 인증, 대시보드           | `apps/website`                            |
| 도큐먼트 영속화, 검색, 결제, 협업 | `apps/api`                                |
| 모바일 패키징 셸                  | `apps/mobile`                             |
| 운영 보조                         | `apps/bmo`, `apps/literoom`, `apps/caddy` |

루트 디렉터리는 다음과 같이 구성된다.

```
typie/
├── apps/         배포 단위 (서비스와 클라이언트)
│   └── desktop/  Tauri 실험용 셸
├── packages/     TypeScript, Svelte 공용 라이브러리
├── crates/       Rust 에디터 엔진 (editor-* 15개)
├── legacy/       구 에디터, Flutter 모바일
├── assets/       폰트, 아이콘, 테마
├── docs/         내부 문서
└── .github/workflows/
```

### apps — 배포 단위

서비스와 클라이언트가 모여 있다. 각 앱의 상세 구조는
[각 앱의 내부 구조](#각-앱의-내부-구조)에서 다룬다.

| 앱              | 역할                                           |
| --------------- | ---------------------------------------------- |
| `apps/website`  | SvelteKit SSR과 클라이언트, GraphQL BFF        |
| `apps/api`      | Hono 기반 GraphQL/REST/WS 통합 게이트웨이      |
| `apps/mobile`   | Kotlin Multiplatform과 Compose, UniFFI 사용    |
| `apps/bmo`      | Slack 멘션을 Lambda worker로 라우팅하는 운영봇 |
| `apps/literoom` | S3 Object Lambda 이미지 변환 (Sharp)           |
| `apps/caddy`    | `:4100`, `:4200`, `:4300`을 `:4000`으로 프록시 |

> `apps/desktop`은 실험용 Tauri 셸로 신규 작업 대상이 아니다.
> 상세는 [레거시](#레거시) 참조.

### packages — 여러 앱이 공유하는 라이브러리

여러 앱에서 중복으로 사용되는 코드가 모여 있다. 새로 유틸을 작성하기 전에 이
디렉터리를 먼저 확인한다.

| 패키지                   | 책임                                  |
| ------------------------ | ------------------------------------- |
| `packages/ui`            | Svelte 컴포넌트, 액션, 폼, 토스트     |
| `packages/lib`           | 로거, Hono 미들웨어, dayjs, Vite SVG  |
| `packages/styled-system` | Panda CSS 토큰, 레시피, 글로벌 스타일 |
| `packages/adapter-node`  | SvelteKit용 커스텀 Node adapter       |
| `packages/tsconfig`      | 공용 `tsconfig` 베이스                |
| `packages/lintconfig`    | ESLint, Prettier 공용 설정            |

### crates — Rust 에디터 엔진

도메인의 핵심이다. 한 번의 변경이 여러 클라이언트에 동시에 영향을 줄 수
있으므로, 각 크레이트의 책임을 미리 파악해두는 것이 중요하다.

| 크레이트               | 책임                                                  |
| ---------------------- | ----------------------------------------------------- |
| `editor-model`         | 문서 모델, 노드 스키마, fragment, validation          |
| `editor-state`         | selection, cursor, IME composition, resolved position |
| `editor-transaction`   | insert, remove, split, merge, move 등 편집 step       |
| `editor-commands`      | 사용자 명령 (삭제, 삽입, 리스트, lift, range)         |
| `editor-core`          | 이벤트, IME, history, handle을 묶는 에디터 런타임     |
| `editor-view`          | 측정, 페이지네이션, hit-test, navigation, search      |
| `editor-renderer`      | glyph, theme, render backend/sink                     |
| `editor-resource`      | 폰트, 브러시, segmentation, zstd, ICU 리소스          |
| `editor-crdt`          | RGA, OR-Set/Map, LWW register, sync, wire format      |
| `editor-ffi`           | 브라우저/서버 WASM, UniFFI native, 플랫폼별 host      |
| `editor-bindgen`       | Kotlin, Swift, JS, wasm-bindgen 보조 바이너리         |
| `editor-server`        | 서버용 폰트/리소스 처리                               |
| `editor-macros`        | 매크로                                                |
| `editor-common`        | 공통 타입                                             |
| `editor-introspection` | 검사, 디버깅                                          |

빌드 진입점은 `crates/editor-ffi/justfile`이다.

```bash
just wasm-browser # apps/website 용 WASM
just wasm-server  # apps/api 용 WASM (서버사이드 렌더)
just mobile       # apps/mobile 용 UniFFI 바인딩과 ICU (Android/iOS)
just desktop      # apps/mobile KMP의 desktop 타깃용 native (apps/desktop과 무관)
```

---

## 로컬 개발 환경 구성

이 섹션은 사용자 또는 AI 에이전트가 순차적으로 실행할 수 있는 형식으로 기술한다.
각 단계는 다음 항목으로 구성된다.

- **목적**: 단계의 의도
- **사전 조건**: 단계 진입 전 충족되어 있어야 할 상태
- **실행**: 실제 셸 명령
- **검증**: 단계 완료 여부를 판별하는 명령과 기대 출력
- **실패 시**: 검증이 실패할 때의 분기

명령은 macOS, zsh 셸을 전제로 한다. 환경이 다르면 실행 전 사용자에게 확인한다.

### 환경 요건

| 항목          | 요건                      |
| ------------- | ------------------------- |
| OS            | macOS                     |
| 셸            | zsh                       |
| 패키지 매니저 | Homebrew (사전 설치 필요) |
| 작업 디렉터리 | 저장소 루트 `typie/`      |

### 설치 대상 도구

| 도구        | 역할                                                         | 필수 여부 |
| ----------- | ------------------------------------------------------------ | --------- |
| `mise`      | Node/pnpm 버전 매니저 예시. 선호하는 다른 도구를 써도 무방함 | 선택      |
| `rustup`    | Rust toolchain 매니저 (stable)                               | 필수      |
| `just`      | command runner. `crates/editor-ffi`의 빌드 진입점            | 필수      |
| `tailscale` | 사내 tailnet 연결. `apps/api` 기동과 내부 리소스 접근에 필수 | 필수      |
| `doppler`   | 환경 변수 주입 통로. `apps/api` 기동에 필수                  | 필수      |
| `caddy`     | 로컬 개발용 reverse proxy (`4100/4200/4300` → `4000`)        | 필수      |
| `graphite`  | 브랜치/커밋/푸시 워크플로. raw git 명령 대신 `gt` 사용       | 필수      |

### 단계 1. 버전 매니저와 빌드 도구 설치

- **목적**: `just`를 설치한다. Node/pnpm 버전 관리는 선호하는 도구를 자유롭게
  쓰면 된다.
- **사전 조건**: `brew --version`이 정상 출력.
- **실행**:

  ```bash
  brew install just
  
  # 선택: Node/pnpm 버전 관리 도구. mise 외에 fnm, volta, nvm, asdf, Homebrew 등
  # 선호하는 것을 자유롭게 써도 된다. 이미 다른 매니저를 쓰고 있다면 설치하지 않아도 된다.
  brew install mise
  ```

- **검증**:

  ```bash
  just --version # 1.x 이상
  ```

  버전 매니저는 필수가 아니므로 검증 대상이 아니다. `mise`를 설치한 경우에 한해
  `mise --version`을 확인한다.

- **실패 시**: `brew doctor`로 Homebrew 상태를 점검한다.

### 단계 2. Rust(rustup) 설치

- **목적**: Rust toolchain 매니저와 stable 채널 설치.
- **사전 조건**: `command -v rustup`이 비어 있음(미설치 상태).
- **실행**:

  ```bash
  brew install rustup-init
  rustup-init -y --default-toolchain stable
  . "$HOME/.cargo/env"
  ```

  현재 셸에는 위 한 줄로 PATH가 적용된다. 다음 셸부터 자동 적용되도록 다음을 한
  번 실행한다(이미 적용되어 있다면 생략).

  ```bash
  grep -q 'cargo/env' "$HOME/.zshrc" || echo '. "$HOME/.cargo/env"' >> "$HOME/.zshrc"
  ```

- **검증**:

  ```bash
  rustc -V                     # rustc 1.x
  cargo -V                     # cargo 1.x
  rustup show active-toolchain # stable-aarch64-apple-darwin 등
  ```

- **실패 시**: `rustc`가 인식되지 않으면 새 셸을 열거나 `. "$HOME/.cargo/env"`를
  다시 실행한다.

#### 참고: Rust 채널 정책

이 저장소의 기본 Rust toolchain은 `rust-toolchain.toml`에 지정된 stable 단일
채널이다. 과거 문서나 코드 주석에 nightly 관련 안내가 남아 있더라도 현재는
유효하지 않으므로 따르지 않는다. 신규 작업과 빌드 검증은 모두 stable을 기준으로
한다.

관련 문서:

- [rustup book](https://rust-lang.github.io/rustup/)

### 단계 3. Doppler CLI 설치

환경 변수 주입 통로 확보를 위한 단계로, `apps/api` 기동에 필수다. brew 경로를
먼저 시도하고, Command Line Tools 버전 오류로 실패할 경우에만 공식 셸 인스톨러로
우회한다.

#### 단계 3-A. brew 경로

- **목적**: brew로 Doppler CLI 설치.
- **사전 조건**: `command -v doppler`가 비어 있음.
- **실행**:

  ```bash
  brew install dopplerhq/cli/doppler
  ```

- **검증**:

  ```bash
  doppler --version # v3.x 이상
  ```

- **분기**:
  - tap 등록 오류 → `brew tap dopplerhq/cli`로 명시적으로 추가한 뒤 재시도한다.
  - `Your Command Line Tools are too outdated` 오류 → brew는 현재 macOS 버전보다
    상위 빌드의 Command Line Tools를 요구한다. macOS 업데이트가 어려우면 **단계
    3-B**로 우회한다.
  - 그 외 오류 → 사용자에게 보고하고 중단한다.

#### 단계 3-B. 공식 셸 인스톨러 우회 경로 (단계 3-A 실패 시에만)

Doppler가 공식적으로 제공하는 셸 인스톨러 경로다. 패키지 매니저에 의존하지
않으며 GPG 서명 검증이 포함된다. 명령은
[Doppler 공식 문서](https://docs.doppler.com/docs/install-cli)의 macOS 설치
안내와 동일하다.

- **목적**: brew 의존성 없이 Doppler 공식 인스톨러로 설치.
- **사전 조건**: 단계 3-A가 Command Line Tools 버전 오류로 실패한 상태.
- **실행 (사용자 본인이 직접 수행)**:

  ```bash
  # 사전 조건: gnupg (바이너리 서명 검증용)
  brew install gnupg
  
  # Doppler 공식 인스톨러 (출처: https://docs.doppler.com/docs/install-cli)
  (curl -Ls --tlsv1.2 --proto "=https" --retry 3 https://cli.doppler.com/install.sh || wget -t 3 -qO- https://cli.doppler.com/install.sh) | sudo sh
  ```

  > 위 명령은 한 줄 그대로 실행해야 한다(`curl`과 `wget` 폴백 로직이 한 서브셸
  > 안에서 동작). `sudo` 비밀번호 입력이 필요하다. AI 에이전트는 이 경로를
  > 자동화하지 않고, 사용자에게 실행을 요청한 뒤 완료 보고를 받는다.

- **검증**:

  ```bash
  doppler --version # v3.x 이상
  which doppler     # /usr/local/bin/doppler
  ```

- **사후 메모**: 이후 업데이트는 `doppler update`로 관리한다. (단계 3-A의 brew
  경로로 설치된 경우는 `brew upgrade`.)

### 단계 4. 저장소 toolchain과 의존성 설치

- **목적**: `.node-version`과 `package.json`의 `packageManager`에 맞춰 Node,
  pnpm을 준비하고 워크스페이스 의존성을 가져온다.
- **사전 조건**: 작업 디렉터리가 저장소 루트(`typie/`)이며, 단계 1~3이 완료된
  상태.
- **실행**:

  ```bash
  # mise를 쓰는 경우. 다른 버전 매니저를 써도 된다.
  mise install
  
  pnpm install
  pnpm run bootstrap
  ```

- **검증**:

  ```bash
  node -v # v25.8.1
  pnpm -v # 10.32.1
  ```

  `pnpm install`이 종료 코드 0으로 끝났고, `node_modules/`와 `pnpm-lock.yaml`이
  존재해야 한다.

- **실패 시**:
  - `node -v`가 25.x가 아니면 사용하는 Node 버전 매니저로 `.node-version`의
    버전을 활성화한다.
  - 네트워크 오류면 사내 프록시 설정을 사용자에게 확인한다.

### 단계 5. Tailscale tailnet 연결 (사용자 개입 필요)

- **목적**: `penxle.io` Tailscale 네트워크에 이 기기를 연결한다.
- **사전 조건**: Tailscale 초대를 수락한 상태.
- **실행 (사용자 본인이 직접 수행)**:

  ```bash
  tailscale status
  tailscale switch --list
  ```

  `tailscale switch --list`에 `penxle.io`가 보이면 다음을 실행한다.

  ```bash
  tailscale switch penxle.io
  ```

  목록에 `penxle.io`가 없으면 Tailscale 앱에서 `Add account` 또는
  `Sign in to another account`를 선택하고, 초대를 수락한 계정으로 다시 로그인한
  뒤 tailnet 선택 화면에서 `penxle.io`를 고른다.

- **검증**:

  ```bash
  tailscale status
  ```

  현재 tailnet 또는 peer 목록이 `penxle.io` 기준으로 표시되어야 한다.
  `apps/api`는 내부 리소스 접근이 필요하므로 개발 서버 기동 전 이 단계가 완료되어야
  한다.

- **실패 시**:
  - 브라우저의 Tailscale 콘솔에서 우측 상단 tailnet을 `penxle.io`로 전환했을 때
    "Your account is a member of the penxle.io network"가 보이는지 확인한다.
  - 웹에서는 멤버로 보이지만 앱/CLI에 안 보이면 Tailscale 앱에서 계정을 추가로
    로그인한다. 초대 수락과 이 기기의 tailnet 연결은 별도 단계다.
  - 내부 라우트 접근이 안 되면 `sudo tailscale set --accept-routes=true` 실행이
    필요한지 관리자에게 확인한다.

### 단계 6. Doppler 워크스페이스 연결 (사용자 개입 필요)

- **목적**: 회사 Doppler 워크스페이스에 로그인하고 프로젝트 구성을 로컬에 연결.
- **사전 조건**: `doppler --version`이 정상 출력.
- **실행 (사용자 본인이 직접 수행)**:

  `doppler login`만으로는 환경 변수가 주입되지 않는다. **반드시
  `doppler setup`까지 함께 실행**해야 `doppler run`이 동작한다. 두 명령은 한
  세트로 다룬다.

  ```bash
  doppler login # 1단계: 브라우저 인증으로 계정 토큰 발급
  doppler setup # 2단계: 저장소 루트 doppler.yaml에 따라 프로젝트/구성 바인딩
  ```

  > `doppler login`은 브라우저 인증을 요구한다. `doppler setup`은 루트
  > `doppler.yaml`에 따라 `apps/api`, `apps/mobile`, `apps/website`의
  > `dev_local` 구성을 연결한다. AI 에이전트는 이 단계를 자동화하지 않고,
  > 사용자에게 실행을 요청한 뒤 완료 보고를 받는다.

- **검증**:

  ```bash
  doppler me            # 사용자 이메일과 워크스페이스 노출
  doppler configure get # 현재 디렉터리에 바인딩된 프로젝트/구성 표시
  ```

- **실패 시**:
  - `doppler me`는 통과하지만 `apps/api` 기동 시 환경 변수가 비어 있다면
    `doppler setup`이 수행되지 않은 상태다. 단계 6의 두 명령을 모두 실행한다.
  - 워크스페이스가 보이지 않으면 사용자에게 초대 여부를 확인한다.

### 단계 7. Caddy 설치

- **목적**: 로컬 개발용 reverse proxy를 설치한다.
- **사전 조건**: Homebrew 사용 가능.
- **실행**:

  ```bash
  brew install caddy
  ```

- **검증**:

  ```bash
  caddy version
  ```

- **실패 시**:
  - `caddy: command not found`가 나오면 설치가 완료되지 않은 상태다.
  - Homebrew가 Command Line Tools 문제로 실패하면 macOS의 Command Line Tools를
    먼저 설치한 뒤 재시도한다. OS 업데이트나 개인 디스크 구성 복구 절차는 이
    문서에 포함하지 않는다.

### 단계 8. Graphite CLI(`gt`) 설치 및 인증 (사용자 개입 필요)

- **목적**: 브랜치/커밋/푸시/리베이스를 Graphite CLI로 진행할 수 있도록 설치하고
  계정을 인증한다. 이 저장소는 **raw Git 명령(`git checkout`, `git commit`,
  `git push`, `git pull`, `git rebase`)을 사용하지 않으므로**, `gt`는 첫 PR을
  등록하기 위한 필수 도구다.
- **사전 조건**: Homebrew 사용 가능. GitHub 계정이 회사 organization에 속한
  상태.
- **실행**:

  ```bash
  brew install withgraphite/tap/graphite
  ```

- **추가 실행 (사용자 본인이 직접 수행)**: 브라우저 인증과 토큰 입력이 필요하다.

  ```bash
  gt auth      # 브라우저로 Graphite 토큰 발급 후 CLI에 입력
  gt repo init # 현재 저장소를 Graphite에 등록 (trunk 브랜치 설정 등)
  ```

  > `gt auth`는 [app.graphite.dev](https://app.graphite.dev) 로그인 후 발급된
  > 토큰을 CLI에 붙여넣는 과정이다. AI 에이전트는 이 단계를 자동화하지 않고,
  > 사용자에게 실행을 요청한 뒤 완료 보고를 받는다.

- **검증**:

  ```bash
  gt --version # 1.x 이상
  gt user      # 인증된 GitHub 계정 표시
  ```

- **실패 시**:
  - `gt: command not found`면 `brew install withgraphite/tap/graphite`가 완료되지
    않은 상태다.
  - `gt auth`에서 토큰 오류가 나면 [app.graphite.dev](https://app.graphite.dev)에서
    토큰을 재발급한다.
  - `gt user`는 통과하지만 PR 생성이 실패하면 GitHub organization 권한(회사
    레포 접근)을 관리자에게 확인한다.

### 단계 9. 개발 서버 기동

- **목적**: 로컬에서 `apps/website`와 `apps/api`를 기동.
- **사전 조건**: 단계 4, 단계 5, 단계 6, 단계 7이 완료된 상태.
- **실행**:

  ```bash
  pnpm run dev
  ```

  Turborepo가 `dev` 타깃을 가진 모든 워크스페이스를 동시에 기동한다.
  `apps/api`는 내부적으로 `doppler run -- concurrently tsdown + node --watch`로
  동작하므로 Doppler 설정이 없으면 기동이 중단된다. 또한 API 서버가 내부 리소스에
  접근하려면 Tailscale이 `penxle.io` tailnet에 연결되어 있어야 한다.

- **검증**: 브라우저로 `http://localhost:4100`에 접속하여 페이지가 로드되는지
  확인한다.

  | 주소                    | 역할                                     |
  | ----------------------- | ---------------------------------------- |
  | `http://localhost:4100` | 웹사이트                                 |
  | `http://localhost:4200` | 유저사이트                               |
  | `http://localhost:4300` | 인증                                     |
  | `http://localhost:4000` | Vite/SvelteKit 내부 포트. 직접 접속 금지 |

- **실패 시**: [트러블슈팅](#트러블슈팅) 표를 참조한다.

### 자주 사용하는 보조 명령

개발 서버 기동과 별개로, 자주 호출하는 lint 명령이다.

```bash
pnpm run lint:eslint    # ESLint
pnpm run lint:typecheck # tsc --noEmit (codegen 결과까지 검사됨)
pnpm run lint:svelte    # svelte-check
pnpm run lint:prettier  # 포맷 검사
```

---

## 요청은 어떻게 처리되는가

사용자가 글을 한 글자 입력했을 때 그 요청이 어떤 계층을 거쳐 데이터베이스에
도달하는지 정리한다. 코드를 수정하기 전에 전체 흐름을 파악해두면 변경의 영향
범위를 추정할 수 있다.

요청 한 건이 처리되는 순서는 다음과 같다.

1. 클라이언트(웹/모바일)가 GraphQL 쿼리, 뮤테이션, 구독을 발행한다.
2. 웹 클라이언트의 경우 SvelteKit `/graphql` 서버 라우트가 BFF로 동작하며,
   쿠키의 access token과 device id를 헤더로 변환하여 API에 전달한다. 모바일은
   API에 직접 연결한다.
3. API 서버는 `bootstrap.ts`에서 기동 상태와 maintenance 플래그를 확인한 뒤,
   `context.ts`에서 session, user, device, IP를 묶어 GraphQL과 REST가 공유하는
   요청 컨텍스트를 구성한다.
4. 리졸버는 PostgreSQL(Drizzle), Redis, Elasticsearch, S3, 서버 WASM 에디터,
   외부 API(PortOne, Apple/Google IAP, SSO, AI Gateway 등)를 조합하여 응답을
   구성한다.
5. 변경셋 적용, 검색 색인, 메일 발송, 구독 만료 등 비용이 큰 후속 작업은 BullMQ
   큐로 분리되어 워커가 비동기로 처리한다.
6. 실시간 이벤트는 GraphQL Subscription 위에서 Redis pubsub을 통해 여러 노드로
   전달된다.

전체 구조는 다음과 같다.

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                        [1] Clients  /  Edge                             │
└─────────────────────────────────────────────────────────────────────────┘

  ┌──────────────┐                            ┌──────────────┐
  │   Web        │                            │   Mobile     │
  │  SvelteKit   │                            │     KMP      │
  └──────┬───────┘                            └──────┬───────┘
         │                                           │
         ▼                                           │
  ┌──────────────┐                                   │
  │   website    │                                   │
  │  SSR / BFF   │                                   │
  └──────┬───────┘                                   │
         │                                           │
         ▼                                           │
  ┌──────────────┐                                   │
  │   /graphql   │                                   │
  │ cookie→header│                                   │
  └──────┬───────┘                                   │
         │                                           │
         └─────────────────────┬─────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      [2] API request layer (Hono)                       │
└─────────────────────────────────────────────────────────────────────────┘

                       ┌──────────────────────┐
                       │ bootstrap /          │
                       │ maintenance check    │
                       └──────────┬───────────┘
                                  ▼
                       ┌──────────────────────┐
                       │ request context      │
                       │ session / device /IP │
                       └──────────┬───────────┘
                                  │
            ┌─────────────────────┼─────────────────────┐
            ▼                     ▼                     ▼
   ┌────────────────┐    ┌────────────────┐    ┌────────────────┐
   │ GraphQL Yoga   │    │  REST routes   │    │ WebSocket subs │
   │ Pothos schema  │    │ auth / iap / og│    │  live events   │
   └────────┬───────┘    └────────┬───────┘    └────────┬───────┘
            │                     │                     │
            │                     │                     ▼
            │                     │            ┌────────────────┐
            │                     │            │  Redis pubsub  │
            │                     │            │ subscription   │
            │                     │            └────────────────┘
            └──────────┬──────────┘
                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                   [3] Domain services  /  Storage                       │
└─────────────────────────────────────────────────────────────────────────┘

   ┌────────────────┐    ┌────────────────┐    ┌────────────────┐
   │  PostgreSQL    │    │     Redis      │    │ Elasticsearch  │
   │ Drizzle schema │    │ cache / locks  │    │  search index  │
   └────────────────┘    └────────────────┘    └────────────────┘

   ┌────────────────┐    ┌────────────────┐    ┌────────────────┐
   │       S3       │    │  Rust editor   │    │ External APIs  │
   │ user contents  │    │   WASM / FFI   │    │ pay / auth / ai│
   └────────────────┘    └────────────────┘    └────────────────┘

                       ┌──────────────────────┐
                       │   BullMQ workers     │
                       │ changes  /  search   │
                       │ email    /  billing  │
                       └──────────────────────┘
```

---

## 각 앱의 내부 구조

이 섹션은 전체를 정독할 필요는 없다. 작업 대상 앱이 정해지면 해당 하위 섹션을
참조한다. `apps/desktop`은 [레거시](#레거시)에서 별도로 설명한다.

### apps/website

SvelteKit 2와 Svelte 5 runes 기반의 메인 웹 애플리케이션이다. GraphQL
클라이언트로 Mearie를 사용하며, 브라우저에서는 `graphql-ws`로 구독을 연결한다.

| 경로                                                | 책임                                     |
| --------------------------------------------------- | ---------------------------------------- |
| `src/routes/website/`                               | 랜딩, 가격, 변경 이력, 법적 문서         |
| `src/routes/website/(dashboard)/`                   | 로그인 후 작업공간 (문서, 폴더, 노트 등) |
| `src/routes/usersite/`                              | 공개 사용자 사이트와 커스텀 도메인       |
| `src/routes/auth/`                                  | 로그인, 회원가입, 비밀번호 재설정, SSO   |
| `src/routes/graphql/`                               | API GraphQL BFF (쿠키를 헤더로 변환)     |
| `src/lib/editor/`, `editor-ffi/`, `wasm*.svelte.ts` | 에디터 패키지 연결 계층                  |

### apps/api

Hono 위에 GraphQL Yoga, REST, WebSocket을 통합한 게이트웨이다. 기동 시
`bootstrap.ts`가 maintenance 상태를 점검하고, `context.ts`에서 GraphQL과 REST가
공유하는 컨텍스트를 구성한다.

| 경로                     | 책임                                                                     |
| ------------------------ | ------------------------------------------------------------------------ |
| `src/graphql/resolvers/` | 도메인별 리졸버 (`auth`, `document`, `payment` 등 약 24개)               |
| `src/rest/`              | `auth`, `bmo`, `entity`, `font`, `iap`, `og` 등                          |
| `src/db/schemas/`        | Drizzle 스키마. 테이블 59개, 마이그레이션 85개                           |
| `src/mq/tasks/`          | BullMQ 워커 (`changeset`, `document`, `email`, `search`, `subscription`) |
| `src/export/`            | PDF, DOCX, EPUB, HWP, 미리보기 이미지 생성                               |
| `src/external/`          | AWS, Elasticsearch, Firebase, PortOne, IAP, SSO, AI Gateway 어댑터       |

### apps/mobile

Kotlin Multiplatform과 Compose Multiplatform 기반의 Android 및 iOS
애플리케이션이다. 에디터는 `crates/editor-ffi`가 생성하는 UniFFI/JNA 바인딩과
ICU 리소스를 사용한다.

| 항목    | 구성                                                        |
| ------- | ----------------------------------------------------------- |
| GraphQL | Apollo. 환경값은 BuildKonfig로 주입                         |
| Android | Firebase Messaging, Play Billing, Google/Kakao/Naver 로그인 |
| iOS     | Swift Package bridge, Sentry, push notification             |

### apps/literoom

S3 Object Lambda 핸들러다. Sharp로 원본을 조회하고 리사이즈, WebP/PNG 변환을
수행한다. SVG는 bypass하며 raw 원본 반환도 지원한다.

### apps/bmo

Slack webhook의 서명을 검증한 뒤 app mention 이벤트를 Lambda worker로 전달한다.
worker는 Claude Agent SDK, AWS, Slack API를 사용한다.

### apps/caddy

로컬 개발용 Caddy 리버스 프록시다 (`:4100`, `:4200`, `:4300`을 `:4000`으로
프록시한다).

| 포트   | 역할                                     |
| ------ | ---------------------------------------- |
| `4100` | 웹사이트                                 |
| `4200` | 유저사이트                               |
| `4300` | 인증                                     |
| `4000` | Vite/SvelteKit 내부 포트. 직접 접속 금지 |

---

## 코드를 수정하기 전에 알아둘 것

작업 영역에 따라 변경의 영향 범위가 다르다. 변경 전에 다음 항목을 확인하면 빌드
실패의 원인을 미리 차단할 수 있다.

### GraphQL 스키마와 리졸버 변경 시

스키마가 변경되면 `apps/website`의 Mearie codegen과 `apps/mobile`의 Apollo
codegen이 모두 영향을 받는다. 변경 후 최소한 `pnpm run lint:typecheck`로 웹
codegen을 확인하고, 모바일은 Apollo codegen이 별도이므로 모바일 빌드를 추가로
실행해야 한다.

- 리졸버 추가와 수정: `apps/api/src/graphql/resolvers/<domain>.ts`
- 스키마 정의 (Pothos 빌더): `apps/api/src/graphql/builder.ts`

### DB 스키마(Drizzle) 변경 시

마이그레이션은 항상 PR 단위로 리뷰한다. 대용량 테이블 변경이나 backfill이 필요한
변경은 단독 PR로 분리하여 리뷰 부담을 줄인다.

- 스키마 정의: `apps/api/src/db/schemas/tables.ts` 외

### Rust 에디터 변경 시

변경 한 줄이 브라우저 WASM, 서버 WASM, 모바일 UniFFI/JNA, 모바일 KMP의 desktop
개발 타깃에 동시에 영향을 줄 수 있다. 영향받는 타깃에 해당하는 `just` 레시피로
로컬 빌드를 먼저 확인하는 것을 권장한다.

| 영향 영역                                           | 레시피              |
| --------------------------------------------------- | ------------------- |
| 브라우저 UI (`apps/website`)                        | `just wasm-browser` |
| 서버사이드 렌더링 (`apps/api`)                      | `just wasm-server`  |
| 모바일 Android/iOS 바인딩 (`apps/mobile`)           | `just mobile`       |
| 모바일 KMP desktop 개발 타깃 (`apps/mobile` 개발용) | `just desktop`      |

루트 `rust-toolchain.toml`은 stable 단일 채널을 지정한다. 빌드 실패 시
[주의 사항](#주의-사항)의 toolchain 항목을 참고한다.

### 클라이언트 UI 변경 시

스타일은 Panda CSS(`packages/styled-system`)를 사용한다. 새 토큰을 추가하기 전에
기존 토큰을 먼저 확인한다.

- 재사용 컴포넌트: `packages/ui`
- 페이지와 라우트: `apps/website/src/routes`
- Svelte 5 runes만 사용: `$props()`, `$state()`, `$derived()`

### BullMQ 워커 변경 시

큐를 추가할 때는 로컬 Redis 연결과 Doppler 환경 변수를 먼저 확인한다. 환경 변수
누락이 가장 흔한 원인이다.

- 큐 정의: `apps/api/src/mq/bullmq.ts`, `apps/api/src/mq/types.ts`
- 워커:
  `apps/api/src/mq/tasks/{changeset,document,email,search,subscription}.ts`

### Graphite(`gt`) 사용 원칙 (필수)

이 저장소는 Git을 직접 사용하지 않는다. 브랜치 생성, 체크아웃, 커밋, 푸시, 풀,
리베이스, 스택 동기화는 전부 Graphite CLI(`gt`)를 통해 수행한다. 다음 raw Git
하위 명령은 일상 워크플로에서 사용하지 않는다.

- `git checkout`, `git switch`
- `git commit`
- `git push`, `git pull`
- `git rebase`, `git merge`

대응되는 `gt` 명령은 다음과 같다.

| 목적           | raw git (사용 금지)     | Graphite                    |
| -------------- | ----------------------- | --------------------------- |
| 새 브랜치      | `git checkout -b foo`   | `gt create foo` (변경 포함) |
| 브랜치 이동    | `git checkout main`     | `gt checkout main`          |
| 새 커밋        | `git commit -m "..."`   | `gt modify -c -m "..."`     |
| 현재 커밋 수정 | `git commit --amend`    | `gt modify`                 |
| 원격 동기화    | `git pull` / `git push` | `gt sync`, `gt submit`      |
| 스택 리베이스  | `git rebase`            | `gt restack`                |

온보딩 문서와 AI 도구 안내에서도 raw Git 하위 명령을 직접 지시하지 않는다. 진단
목적의 `git status`, `git diff`, `git log` 같은 읽기 전용 명령은 그대로 써도
된다.

### 커밋과 코드 스타일

코드 스타일은 자동 포맷터가 대부분 처리한다. 자세한 내용은 `CLAUDE.md`를
참고한다.

| 영역       | 규칙                                                                            |
| ---------- | ------------------------------------------------------------------------------- |
| TypeScript | `type` 우선, named export만, `verbatimModuleSyntax`                             |
| Formatting | 2 spaces, 140 char width, single quote                                          |
| 파일명     | 유틸 `kebab-case.ts`, 컴포넌트 `PascalCase.svelte`, 상수 `SCREAMING_SNAKE_CASE` |
| Svelte 5   | `$props()`, `$state()`, `$derived()`                                            |
| Rust       | stable toolchain, Edition 2024, 커밋 전 `cargo fmt`                             |

---

## 첫 PR 등록

작은 변경으로 한 차례 PR 사이클을 진행해보는 것이 환경 검증에 가장 효과적이다.
브랜치/커밋/푸시는 [Graphite 사용 원칙](#graphitegt-사용-원칙-필수)을 따른다.

### 1. 브랜치 생성

```bash
gt create onboarding/<your-name>
```

### 2. 변경 후보 선택

다음 중 하나로 가볍게 시작한다.

- `apps/website`의 정적 카피 한 줄
- `apps/api`의 리졸버에 로그 한 줄

### 3. lint 실행

```bash
pnpm run lint:eslint
pnpm run lint:typecheck
pnpm run lint:svelte # Svelte 변경 시
```

### 4. 커밋과 PR 제출

```bash
gt modify -c -m "chore(onboarding): ..." # 커밋
gt submit                                # PR 생성/푸시
```

### 5. CI 통과 확인

PR에서 `ci.yml`이 eslint, prettier, spellcheck, svelte-check, syncpack,
typecheck를 실행한다. 모두 통과하면 첫 날 목표를 달성한 것이다.

PR 등록 이후 동작하는 워크플로는 다음과 같다.

| 워크플로         | 트리거       | 하는 일                                       |
| ---------------- | ------------ | --------------------------------------------- |
| `ci.yml`         | PR, push, MQ | eslint, prettier, spellcheck, svelte-check    |
| `build-wasm.yml` | 관련 변경    | editor-ffi 브라우저/서버 WASM artifact        |
| `build.yml`      | 관련 변경    | Turbo prune, API/website Docker, GHCR/ECR     |
| `cd.yml`         | main push    | WASM과 이미지 빌드, dev 배포                  |
| `production.yml` | 수동         | dev 이미지를 prod 태그로 재태깅 후 prod 배포  |
| `deployment.yml` | 배포 후      | `penxle/kube`, `penxle/kube2` 매니페스트 갱신 |

> `build-wasm-legacy.yml` (`legacy/editor` WASM 빌드) 등 레거시 워크플로는
> [참고: 현재 작업과 무관한 영역](#참고-현재-작업과-무관한-영역) 참조.

---

## 주의 사항

작업 전에 미리 알아두면 도움이 되는 항목이다. 실제 빌드가 실패한 뒤의 복구
절차는 [트러블슈팅](#트러블슈팅)을 본다.

### 코드젠 산출물 디렉터리

`.gitignore`에 들어 있고 빌드 명령으로 자동 생성된다. 직접 편집해도 다음
빌드에서 덮어써진다. 변경하려면 **원본 정의**(스키마, Panda 설정, Rust 코드,
UniFFI 인터페이스 등)를 고치고 codegen을 다시 돌린다.

| 디렉터리                 | 생성 명령                                   | 원본 정의 위치                                  |
| ------------------------ | ------------------------------------------- | ----------------------------------------------- |
| `.svelte-kit/`           | `sveltekit()` Vite 플러그인                 | `apps/website/src/routes/`, `svelte.config.js`  |
| `.mearie/`               | `mearie generate` (`pnpm codegen`)          | `apps/website/mearie.config.ts`, GraphQL 스키마 |
| `styled-system/`         | `panda codegen`                             | `packages/styled-system/` Panda 설정            |
| `crates/editor-ffi/pkg/` | `just wasm-browser`, `just wasm-server`     | `crates/editor-*` Rust 코드                     |
| 모바일 generated 바인딩  | `just mobile` (uniffi-bindgen Kotlin/Swift) | `crates/editor-ffi/` UniFFI 인터페이스          |

### `apps/api` 환경 변수와 내부 호스트

`apps/api/package.json`의 `dev` 스크립트는 `doppler run --`로 감싸져 있다. 즉
`pnpm run dev`로 `apps/api`를 띄우면 Doppler에서 환경 변수가 주입되어야만
프로세스가 정상 시작된다. `doppler login`만 해도 변수는 주입되지 않으니
`doppler setup`까지 완료한다 (`doppler.yaml`이 `apps/api` → `api` 프로젝트의
`dev_local` 구성으로 바인딩).

DB/Redis/Elasticsearch 접속은 다음 환경 변수로 결정된다 (`apps/api/src/env.ts`,
`apps/api/src/db/index.ts`, `apps/api/src/cache.ts`, `apps/api/src/search.ts`).

| 리소스        | 환경 변수                                         |
| ------------- | ------------------------------------------------- |
| PostgreSQL    | `DATABASE_URL`, `DATABASE_RO_URL` (선택)          |
| Redis         | `REDIS_URL`                                       |
| Elasticsearch | `ELASTICSEARCH_CLOUD_ID`, `ELASTICSEARCH_API_KEY` |

위 환경 변수가 가리키는 호스트가 사내 네트워크에 있으면 Tailscale `penxle.io`
tailnet에 붙어 있어야 접근이 된다. tailnet이 끊긴 상태에서는 서버 프로세스가
떠도 DB/Redis/Elasticsearch 핸드셰이크에서 실패한다.

### GraphQL 스키마 변경의 영향 범위

스키마 한 곳을 변경하면 세 빌드 파이프라인이 모두 영향을 받는다.

| 영역           | 도구   | 정의/설정                                                             |
| -------------- | ------ | --------------------------------------------------------------------- |
| `apps/api`     | Pothos | `apps/api/src/graphql/builder.ts`, `apps/api/src/graphql/schema.ts`   |
| `apps/website` | mearie | `apps/website/mearie.config.ts` (`schema: 'schema.graphql'`)          |
| `apps/mobile`  | Apollo | `apps/mobile/compose/src/commonMain/graphql/schema.graphqls` (Gradle) |

리졸버는 `apps/api/src/graphql/resolvers/` 아래 약 23개 도메인 파일로 나뉘어
있다. 변경 후 최소한 `pnpm run lint:typecheck`로 웹 측 codegen이 깨지지 않는지
확인하고, 모바일 영향은 별도로 Gradle 빌드를 돌려야 알 수 있다.

DB 스키마(Drizzle)는 마이그레이션 적용이 함께 따라오므로, 대용량 테이블 변경이나
backfill이 끼는 마이그레이션은 단독 PR로 분리해서 리뷰한다.

### Rust toolchain 채널

루트 `rust-toolchain.toml`은 두 줄짜리 파일이고 내용은 `channel = "stable"` 하나다.
`.github/workflows/`에도 nightly 사용처는 없다. 빌드 실패 메시지가 nightly 설치를
권하더라도 따르지 않는다.

빌드가 실패하면 `rustup show`로 활성 toolchain이 `rust-toolchain.toml`과 일치하는지
먼저 확인하고, 일치하는데도 실패한다면 toolchain이 아닌 다른 원인(의존성, 환경
변수, 코드)을 본다.

---

## 트러블슈팅

장애 대응과 버그 수정 시 참조하는 절차다. 환경 문제로 작업이 중단되었을 때 가장
먼저 확인한다.

| 증상                                       | 원인과 해결                                                                            |
| ------------------------------------------ | -------------------------------------------------------------------------------------- |
| `pnpm install` 실패                        | `node -v`가 25.x인지 확인. 사용하는 버전 매니저 점검                                   |
| `apps/api`가 기동되지 않음                 | Tailscale `penxle.io` 연결과 `doppler login` + `doppler setup` 모두 완료되었는지 확인  |
| `apps/api`가 기동되고도 DB/Redis 호출 실패 | Tailscale tailnet 미연결. `tailscale status`로 확인 후 `tailscale switch penxle.io`    |
| Doppler 환경 변수가 비어 있음              | `doppler login`만 수행하고 `doppler setup`이 누락된 상태. 단계 6의 두 명령을 모두 실행 |
| `@typie/caddy#dev`가 `caddy`를 못 찾음     | `brew install caddy` 후 `caddy version` 확인                                           |
| `gt: command not found` 또는 PR 제출 실패  | 단계 8 (`brew install withgraphite/tap/graphite` → `gt auth` → `gt repo init`) 재확인  |
| `localhost:4000`에서 500 발생              | 내부 Vite 포트이므로 브라우저에서는 `localhost:4100`을 사용                            |
| WASM 빌드 실패                             | `just wasm-browser` 등 개별 타깃부터 좁혀 확인                                         |
| Rust 빌드 실패                             | `rustup show`로 stable 활성화 확인. 메시지가 nightly 설치를 권하더라도 따르지 않는다   |

표에 없는 증상이거나 위 조치로 해결되지 않을 경우, 자신의 변경을 의심하기 전에
다음을 점검한다.

- 환경 변수가 최근에 변경되지 않았는지 (`doppler setup`과 동기화 상태)
- `pnpm-lock.yaml`이나 generated 디렉터리에 충돌이 남아 있지 않은지
- main 브랜치 자체가 정상인지 (`#dev` 채널 공지 확인)

장애 대응이 필요한 상황이라면 위 표보다 운영 채널 공유를 우선한다. 재현 경로를
짧게 기록한 뒤 함께 추적한다.

---

## 레거시

저장소에 남아 있지만 신규 작업 대상이 아닌 영역이다.

### `apps/desktop`

Tauri 2가 SvelteKit static build를 감싼 실험용 데스크톱 셸이다. 현재 제품 개발
흐름에서는 사용하지 않는다.

| 항목       | 값                                                                        |
| ---------- | ------------------------------------------------------------------------- |
| 식별자     | `co.typie` (제품명 `Typie`)                                               |
| dev 포트   | `5100`                                                                    |
| 딥링크     | `typie://`                                                                |
| 플러그인   | single-instance, deep-link, dialog, http, opener, process, store, updater |
| macOS 동작 | 창 닫기 시 종료 대신 hide                                                 |

> `crates/editor-ffi`의 `just desktop` 레시피는 이 앱이 아니라 `apps/mobile` KMP의
> desktop 타깃(모바일 앱 개발용 native)을 빌드한다. 이름이 비슷해서 자주
> 혼동되므로 주의한다.

### `legacy/` 디렉터리 (구 에디터, Flutter 모바일)

`legacy/`에는 다음이 들어 있다.

- `legacy/editor`: 구 에디터. `@typie/editor` 워크스페이스 패키지로 일부
  의존성에 여전히 남아 있다. **신규 작업은 모두 `crates/editor-*`에서 진행한다.**
- `legacy/` 하위의 Flutter 모바일 구현체: 현재 `apps/mobile`(Kotlin
  Multiplatform)로 대체되었다.

레거시 정리 PR이 아닌 한 이 디렉터리는 수정하지 않는다.

### Rust nightly 관련 안내 (모두 레거시)

저장소의 일부 문서, 코드 주석, 과거 CI 흔적에 **nightly toolchain 관련 안내가
남아 있을 수 있다.** 이 안내는 모두 레거시이며 더 이상 유효하지 않다.

- 현재 빌드는 `rust-toolchain.toml`에 지정된 **stable 단일 채널**만 사용한다.
- `rustup toolchain install nightly`, nightly 전용 cargo flag, "WASM은 nightly로
  빌드한다" 같은 안내는 따르지 않는다.
- 빌드 실패 메시지가 nightly를 지시하더라도, 원인은 다른 곳(의존성, 환경 변수
  등)에 있다고 가정하고 stable 기준으로 디버깅한다.

### `build-wasm-legacy.yml` 워크플로

GitHub Actions의 `build-wasm-legacy.yml`은 `legacy/editor` WASM을 빌드한다.
신규 에디터 빌드와는 무관하다. 일상 개발에서 이 워크플로의 결과를 참조할 일은
없다.
