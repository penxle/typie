# editor-ffi Design

에디터 아키텍처의 최종 레이어. editor-core를 플랫폼(KMP, Web)에 노출하는 FFI crate.

## 배경

현재 `crates/editor/`는 레거시 runtime 로직과 FFI 바인딩이 한 crate에 혼재되어 있다.
새 아키텍처에서 runtime은 `editor-core`가 담당하므로, FFI 레이어를 별도 crate으로 분리한다.

- `crates/editor/`의 runtime 로직은 점진적으로 새 crate들로 이관
- FFI 부분을 먼저 `editor-ffi`로 분리
- 레거시와 당분간 공존

## 설계 원칙

- **editor-core만 의존** — 렌더링은 core 내부에서 해결, FFI는 결과만 전달
- **Adapter 패턴** — FFI 경계 전용 타입을 정의하고 editor-core 타입과 변환
- **대칭 바인딩** — UniFFI(네이티브)와 WASM(웹) 바인딩을 동등한 구조로 제공
- **최소 스켈레톤** — editor-core의 현재 구현 범위만 노출, core 성장에 맞춰 확장
- **API 재설계** — 레거시 API를 미러링하지 않고 새 아키텍처에 맞게 완전 재설계

## Crate 구조

```
crates/editor-ffi/
├── Cargo.toml
├── uniffi.toml
├── justfile
└── src/
    ├── lib.rs              # setup + 모듈 선언
    ├── macros.rs           # __ffi_gen + derive_ffi! 매크로
    ├── prelude.rs          # 바인딩 프리루드 (cfg-if 타겟별 타입 추상화)
    ├── convert.rs          # FromFfi/IntoFfi trait
    ├── types.rs            # derive_ffi! 호출
    ├── host.rs             # EditorHost (cfg_attr로 타겟별 어노테이션)
    ├── editor.rs           # Editor (cfg_attr로 타겟별 어노테이션)
    └── platform/           # 플랫폼별 코드
        ├── mod.rs
        ├── desktop.rs      # CPU pixel buffer
        ├── android.rs      # AHardwareBuffer
        └── ios.rs          # IOSurface
```

### Cargo.toml

```toml
```toml
[package]
name = "editor-ffi"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib", "rlib", "staticlib"]

[features]
default = []
uniffi = ["dep:uniffi"]
wasm = ["dep:wasm-bindgen", "dep:serde-wasm-bindgen", "dep:serde", "dep:serde_json"]

[dependencies]
editor-core = { path = "../editor-core" }
editor-common = { path = "../editor-common" }
editor-model = { path = "../editor-model" }
editor-state = { path = "../editor-state" }
editor-transaction = { path = "../editor-transaction" }
editor-view = { path = "../editor-view" }
editor-macros = { path = "../editor-macros" }
hashbrown = "0.16"
paste = "1"
cfg-if = "1"
thiserror = "2"
uniffi = { version = "0.31", optional = true }
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }
serde = { version = "1", optional = true }
serde_json = { version = "1", optional = true }
```

## 매크로 시스템: `#[ffi]` + `derive_ffi!()`

타입 중복 없이 FFI 경계 타입을 자동 생성하는 2단계 매크로 시스템.

### Phase 1: `#[ffi]` (proc-macro, editor-macros crate)

원본 타입에 마킹. 타입 구조를 기술하는 companion 선언적 매크로를 자동 생성한다.
원본 타입 자체는 변경하지 않는다.

```rust
// editor-state/src/position.rs
#[ffi]
pub struct Position {
    pub node_id: NodeId,
    pub offset: usize,
    pub affinity: Affinity,
}

// #[ffi]가 자동 생성 ↓
#[macro_export]
macro_rules! __ffi_describe_Position {
    ($callback:ident) => {
        $callback! {
            struct Position [editor_state] {
                node_id: NodeId,
                offset: usize,
                affinity: Affinity,
            }
        }
    };
}
```

enum에도 동일하게 동작:

```rust
// editor-state/src/affinity.rs
#[ffi]
pub enum Affinity {
    Downstream,
    Upstream,
}

// 자동 생성 ↓
#[macro_export]
macro_rules! __ffi_describe_Affinity {
    ($callback:ident) => {
        $callback! {
            enum Affinity [editor_state] {
                Downstream,
                Upstream,
            }
        }
    };
}
```

데이터를 가진 enum variant도 지원:

```rust
#[ffi]
pub enum Message {
    Key(KeyEvent),
    Pointer(PointerEvent),
    Intent(Intent),
    System(SystemEvent),
}

// 자동 생성 ↓
#[macro_export]
macro_rules! __ffi_describe_Message {
    ($callback:ident) => {
        $callback! {
            enum Message [editor_core] {
                Key(KeyEvent),
                Pointer(PointerEvent),
                Intent(Intent),
                System(SystemEvent),
            }
        }
    };
}
```

### Phase 2: `derive_ffi!()` (선언적 매크로, editor-ffi crate)

FFI crate에서 호출. companion 매크로로부터 필드/variant 정보를 받아 래퍼 타입을 생성한다.

```rust
// editor-ffi/src/types.rs
derive_ffi!(editor_state::Affinity);
derive_ffi!(editor_state::Position);
derive_ffi!(editor_state::Selection);
derive_ffi!(editor_core::Message);
// ... 모든 FFI 경계 타입
```

`derive_ffi!`가 생성하는 코드:

1. 동일한 이름의 타입 (`Position`, `Affinity`, `Message`) — 모듈 네임스페이스로 원본과 구분
2. `#[cfg_attr(feature = "uniffi", derive(uniffi::Record/Enum))]` + `#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]`
3. `FromFfi<FFI타입> for 원본` 구현 — 입력 변환 (FFI → core): `message.from_ffi()`
4. `IntoFfi<FFI타입> for 원본` 구현 — 출력 변환 (core → FFI): `selection.into_ffi()`
5. WASM에서 추가: `FromFfi<JsValue>`, `IntoFfi<JsValue>` 구현

### 커스텀 타입 매핑: `Ffi` trait + `#[ffi(custom)]`

FFI 경계에서 다른 타입으로 표현되어야 하는 타입(예: NodeId → base62 String)은 `Ffi` trait을 구현하고 `#[ffi(custom)]`으로 마킹한다.

```rust
// editor-common — trait 정의
pub trait Ffi {
    type Target;
    fn to_ffi(&self) -> Self::Target;
    fn from_ffi(value: Self::Target) -> Self;
}
```

```rust
// editor-model — NodeId 구현
#[ffi(custom)]
pub struct NodeId(u64);

impl Ffi for NodeId {
    type Target = String;
    fn to_ffi(&self) -> String { self.to_string() }
    fn from_ffi(value: String) -> Self { value.parse().expect("invalid NodeId") }
}
```

`#[ffi(custom)]`은 companion 매크로에 `@custom_type` descriptor를 생성하고, `__ffi_gen`이 `Ffi` trait에 위임하여 `uniffi::custom_type!`을 생성한다. 매크로는 변환 대상 타입을 모른다 — trait의 `Target`이 유일한 진실의 원천이다.

```rust
// editor-ffi/src/types.rs — 동일한 derive_ffi! 경로
derive_ffi!(editor_model::NodeId);
```

## 공유 리소스: EditorContext

모든 Editor 인스턴스가 공유하는 환경 리소스. EditorHost가 소유하고, 각 Editor에 `Arc<Mutex<EditorContext>>`로 공유한다.

```rust
// editor-core
pub struct EditorContext {
    pub font_registry: FontRegistry,
    pub segmenters: Option<TextSegmenters>,
    // 추후: GPU 디바이스, ICU 데이터 등
}
```

editor-core::Editor는 `EditorContext`를 직접 소유하지 않고 공유 참조로 받는다. FontRegistry는 모든 에디터에서 동일 상태를 공유해야 하므로 반드시 공유되어야 한다.

## API Surface

### EditorHost (앱 레벨 싱글턴)

`EditorContext`를 소유하고 Editor 인스턴스를 생성한다.

```rust
impl EditorHost {
    fn new() -> Arc<Self>
    fn create_editor(width: f32, height: f32, scale_factor: f64) -> Arc<Editor>
}
```

### Editor (문서 레벨)

editor-core::Editor를 감싸고 메시지 처리, 서피스 관리를 담당한다.

```rust
impl Editor {
    // 메시지 큐
    fn enqueue(&self, message: Message)
    fn tick(&self)

    // 서피스
    fn attach_surface(&self, page: u32, surface: Arc<SurfaceHandle>)
    fn detach_surface(&self, page: u32)
    fn render(&self, page: u32)
}
```

- `enqueue` + `tick` — editor-core의 메시지 큐 패턴 그대로 노출
- undo/redo, 커서 이동 등은 Message의 Intent variant로 enqueue
- 쿼리, 히스토리, 동기화 메서드는 editor-core 성장에 맞춰 확장 예정

### SurfaceHandle

```rust
impl SurfaceHandle {
    fn new(width: u32, height: u32, scale_factor: f64) -> Arc<Self>
    fn native_handle(&self) -> u64
    fn pixel_data(&self) -> Vec<u8>
    fn width(&self) -> u32
    fn height(&self) -> u32
}
```

## FFI 경계 타입 분류

### FFI 경계를 넘는 타입

| 분류 | 타입 | FFI 형태 |
|------|------|----------|
| 입력 | Message 계층 전체 (Key, Pointer, Intent, System) | UniFFI: typed enum / WASM: JSON (serde) |
| 입력 | Node (21 variants), Modifier | UniFFI: typed enum / WASM: JSON (serde) |
| 출력 | Effect | UniFFI: typed enum / WASM: JSON (serde) |
| 출력 | Position, Selection, Affinity | FFI record/enum |
| 출력 | Rect, Size 등 기하 타입 | FFI record |
| 동기화 | Step | String (JSON) — 플랫폼은 pass-through |

### FFI 경계를 넘지 않는 타입 (opaque)

| 타입 | 이유 |
|------|------|
| State, Doc | imbl 기반 persistent 구조, 쿼리 메서드로만 접근 |
| View | 레이아웃 엔진 내부 구조 |

## 바인딩 전략: cfg-if 프리루드

별도 바인딩 파일 없이, `lib.rs`의 cfg-if 블록에서 타겟별 타입과 헬퍼를 정의한다.
host.rs와 editor.rs는 이 추상화된 타입을 사용하여 타겟 무관한 코드를 작성한다.

```rust
// lib.rs — 바인딩 프리루드
cfg_if::cfg_if! {
    if #[cfg(feature = "uniffi")] {
        #[derive(Debug, thiserror::Error, uniffi::Error)]
        pub enum EditorError {
            #[error("{msg}")]
            General { msg: String },
        }

        pub type Owned<T> = Arc<T>;
        pub type Input<T> = T;
        pub type Output<T> = T;
        pub fn into_owned<T>(val: T) -> Arc<T> { Arc::new(val) }
    } else if #[cfg(feature = "wasm")] {
        pub use wasm_bindgen::JsError as EditorError;

        pub type Owned<T> = T;
        pub type Input<T> = wasm_bindgen::JsValue;
        pub type Output<T> = wasm_bindgen::JsValue;
        pub fn into_owned<T>(val: T) -> T { val }
    } else {
        // feature 미선택 시 — cargo check --workspace 등에서 사용
        pub type EditorError = String;
        pub type Owned<T> = T;
        pub type Input<T> = T;
        pub type Output<T> = T;
        pub fn into_owned<T>(val: T) -> T { val }
    }
}

pub type EditorResult<T> = Result<T, EditorError>;
```

```rust
// convert.rs — FFI 타입 변환 trait (editor-ffi 로컬)

/// FFI 경계 입력 변환: FFI/Input 타입 → core 타입.
/// `message.from_ffi()` — "이 값은 FFI에서 왔다"
pub trait FromFfi<T> {
    fn from_ffi(self) -> T;
}

/// FFI 경계 출력 변환: core 타입 → FFI/Output 타입.
/// `selection.into_ffi()` — "이 값을 FFI로 내보낸다"
pub trait IntoFfi<T> {
    fn into_ffi(self) -> T;
}
```

`__ffi_gen` 매크로가 각 타입에 대해 `FromFfi` impl을 자동 생성한다.
WASM에서는 `FromFfi<JsValue>` impl도 추가 생성하여 `JsValue` ↔ core 타입 직접 변환을 지원한다.

```rust
// editor.rs — 타겟 무관한 코드, .into_ffi() 한 번으로 변환
#[cfg_attr(feature = "uniffi", uniffi::export)]
impl Editor {
    pub fn enqueue(&self, message: Input<Message>) -> EditorResult<()> {
        self.with_inner(|e| e.enqueue(message.from_ffi()))
    }
}
```

struct 어노테이션은 `#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]`로 조건부 적용.

## 플랫폼 서피스

레거시 `crates/editor/src/ffi/`의 플랫폼별 서피스 코드를 이관한다.
`#[cfg(target_os)]`로 분기.

| 플랫폼 | 파일 | 버퍼 타입 |
|--------|------|-----------|
| Desktop | platform/desktop.rs | CPU pixel buffer (`Vec<u8>`) |
| Android | platform/android.rs | AHardwareBuffer (ndk) |
| iOS | platform/ios.rs | IOSurface (core-foundation) |

## 빌드 시스템

레거시 `crates/editor/justfile`의 구조를 따르되 editor-ffi를 대상으로 한다.

| 명령 | 동작 |
|------|------|
| `just jvm` | Desktop dylib + UniFFI Kotlin 바인딩 생성 |
| `just android` | cargo-ndk로 .so 빌드 (arm64-v8a, armeabi-v7a, x86_64) |
| `just ios` | XCFramework + UniFFI Swift 바인딩 생성 |
| `just wasm` | wasm-pack 빌드 |

바인딩 출력 경로: `apps/mobile2/generated/uniffi/` (기존과 동일)

## 확장 계획

editor-core가 성장하면 Editor에 메서드를 추가:

| 기능 | 메서드 | 시점 |
|------|--------|------|
| 쿼리 | `selection()`, `cursor_rect()`, `hit_test()` | View 쿼리 완성 시 |
| 히스토리 | `can_undo()`, `can_redo()` | History 연동 시 |
| 동기화 | `export_steps()`, `import_steps()` | Sync 구현 시 |
| 뷰포트 | `resize()`, `page_count()` | View 레이아웃 완성 시 |
| 폰트 | `register_font()`, `set_available_fonts()` | FontRegistry 연동 시 |
