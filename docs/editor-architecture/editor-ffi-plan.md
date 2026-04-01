# editor-ffi Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** editor-core를 네이티브/웹 플랫폼에 노출하는 editor-ffi crate 추가 (enqueue/tick/surface 초기 스코프)

**Architecture:** `#[ffi]` proc-macro가 소스 타입에 companion 매크로를 생성하고, editor-ffi에서 `derive_ffi!`로 FFI 경계 타입 + From 변환을 자동 생성. EditorHost(앱 싱글턴) + Editor(문서 레벨)를 UniFFI/WASM 대칭 바인딩으로 노출.

**Tech Stack:** Rust 2024, UniFFI 0.31, wasm-bindgen, syn/quote/proc-macro2

**Spec:** `docs/editor-architecture/editor-ffi-design.md`

**Note:** 이 프로젝트에서 git commit은 사용자가 수동으로 수행합니다. "Commit" 스텝은 사용자에게 커밋 시점을 알리는 체크포인트입니다.

---

## File Structure

### 신규 생성

```
crates/editor-ffi/
├── Cargo.toml
├── uniffi.toml
├── src/
│   ├── lib.rs          # setup + 모듈 선언
│   ├── macros.rs       # __ffi_gen + derive_ffi! 매크로
│   ├── prelude.rs      # 바인딩 프리루드 (cfg-if 타겟별 타입 추상화)
│   ├── convert.rs      # FromFfi/IntoFfi trait
│   ├── types.rs        # derive_ffi! 호출
│   ├── host.rs         # EditorHost (cfg_attr로 타겟별 어노테이션)
│   ├── editor.rs       # Editor (cfg_attr로 타겟별 어노테이션)
│   └── platform/
│       ├── mod.rs
│       └── desktop.rs

crates/editor-macros/src/
├── ffi_macro/
│   ├── mod.rs
│   ├── parse.rs
│   └── codegen.rs
```

### 수정 대상

```
crates/editor-macros/Cargo.toml          — (변경 없을 수 있음, syn/quote 이미 존재)
crates/editor-macros/src/lib.rs          — #[ffi], derive_ffi! 엔트리 추가
crates/editor-common/src/geometry.rs     — #[ffi] 추가 (Rect, Size)
crates/editor-common/src/movement.rs     — #[ffi] 추가 (Movement, Direction)
crates/editor-model/src/id.rs            — #[ffi(custom)] 추가 (NodeId) + Ffi trait 구현
crates/editor-common/src/ffi.rs          — Ffi trait 정의 (#[ffi(custom)]용)
crates/editor-ffi/src/convert.rs         — FromFfi/IntoFfi trait 정의 (타입 변환용)
crates/editor-model/src/modifier.rs      — #[ffi] 추가 (Modifier), #[strum_discriminants(ffi)] (ModifierType)
crates/editor-model/src/nodes/mod.rs     — #[ffi] 추가 (Node), #[strum_discriminants(ffi)] (NodeType)
crates/editor-state/src/position.rs      — #[ffi] 추가 (Position)
crates/editor-state/src/selection.rs     — #[ffi] 추가 (Selection)
crates/editor-state/src/affinity.rs      — #[ffi] 추가 (Affinity)
crates/editor-core/src/message.rs        — #[ffi] 추가 (Message, Intent, 모든 하위 타입)
crates/editor-transaction/src/effect.rs  — #[ffi] 추가 (Effect)
crates/editor-view/src/viewport.rs       — #[ffi] 추가 (Viewport)
```

---

## Task 1: Crate Scaffolding

**Files:**
- Create: `crates/editor-ffi/Cargo.toml`
- Create: `crates/editor-ffi/uniffi.toml`
- Create: `crates/editor-ffi/src/lib.rs`
- Create: `crates/editor-ffi/src/prelude.rs`
- Create: `crates/editor-ffi/src/convert.rs`
- Create: `crates/editor-ffi/src/types.rs`
- Create: `crates/editor-ffi/src/host.rs`
- Create: `crates/editor-ffi/src/editor.rs`
- Create: `crates/editor-ffi/src/platform/mod.rs`

- [ ] **Step 1: Create Cargo.toml**

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
editor-macros = { path = "../editor-macros" }
uniffi = { version = "0.31", optional = true }
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }
serde = { version = "1", optional = true }
serde_json = { version = "1", optional = true }
```

- [ ] **Step 2: Create uniffi.toml**

```toml
[bindings.swift]
ffi_module_name = "EditorFFI"
ffi_module_filename = "EditorFFI"
```

- [ ] **Step 3: Create src/lib.rs**

```rust
#[cfg(all(feature = "uniffi", feature = "wasm"))]
compile_error!("features \"uniffi\" and \"wasm\" are mutually exclusive");

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

mod prelude;
mod convert;
mod types;
pub mod host;
pub mod editor;
mod platform;

// 각 모듈에서 use crate::prelude::*; 로 직접 import
```

- [ ] **Step 3a: Create src/prelude.rs**

```rust
cfg_if::cfg_if! {
    if #[cfg(feature = "uniffi")] {
        #[derive(Debug, thiserror::Error, uniffi::Error)]
        pub enum EditorError {
            #[error("{msg}")]
            General { msg: String },
        }

        pub type Owned<T> = std::sync::Arc<T>;
        pub type Input<T> = T;
        pub type Output<T> = T;

        pub fn into_owned<T>(val: T) -> std::sync::Arc<T> { std::sync::Arc::new(val) }
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

- [ ] **Step 4: Create empty module files**

`src/types.rs`:
```rust
// derive_ffi! 호출은 Task 6에서 추가
```

`src/host.rs`:
```rust
use editor_core::Editor as CoreEditor;

pub struct EditorHost;
```

`src/editor.rs`:
```rust
pub struct Editor;
```

`src/platform/mod.rs`:
```rust
cfg_if::cfg_if! {
    if #[cfg(target_os = "android")] {
        mod android;
        pub use android::SurfaceHandle;
    } else if #[cfg(target_os = "ios")] {
        mod ios;
        pub use ios::SurfaceHandle;
    } else {
        mod desktop;
        pub use desktop::SurfaceHandle;
    }
}
```

- [ ] **Step 5: Compile 확인**

Run: `cargo check -p editor-ffi`
Expected: 성공 (빈 모듈이므로 warning만 있을 수 있음)

- [ ] **Step 6: Commit checkpoint**

`feat(editor-ffi): add crate scaffolding`

---

## Task 2: `#[ffi]` Proc-Macro — Struct 지원

기존 editor-macros의 패턴(parse.rs + codegen.rs)을 따라 `ffi_macro` 모듈을 추가한다.

**Files:**
- Create: `crates/editor-macros/src/ffi_macro/mod.rs`
- Create: `crates/editor-macros/src/ffi_macro/parse.rs`
- Create: `crates/editor-macros/src/ffi_macro/codegen.rs`
- Modify: `crates/editor-macros/src/lib.rs`

- [ ] **Step 1: Create parse.rs**

`#[ffi]`에 전달된 struct/enum 정의를 파싱한다.

```rust
use syn::{Data, DeriveInput, Fields, Ident, Type, Variant};

pub struct FfiInput {
    pub item: DeriveInput,
    pub custom: bool,
}

impl FfiInput {
    pub fn from_attr_and_item(attr: TokenStream, item: DeriveInput) -> Self {
        let custom = !attr.is_empty() && attr.to_string() == "custom";
        Self { item, custom }
    }
}

pub enum FfiTypeKind {
    Custom,
    Struct { fields: Vec<StructField> },
    Enum { variants: Vec<EnumVariant> },
}

pub struct StructField {
    pub name: Ident,
    pub ty: Type,
}

pub enum EnumVariant {
    Unit { name: Ident },
    Tuple { name: Ident, fields: Vec<Type> },
    Struct { name: Ident, fields: Vec<StructField> },
}

impl FfiInput {
    pub fn kind(&self) -> FfiTypeKind {
        if self.custom {
            return FfiTypeKind::Custom;
        }
        match &self.item.data {
            Data::Struct(data) => {
                let fields = match &data.fields {
                    Fields::Named(named) => named
                        .named
                        .iter()
                        .map(|f| StructField {
                            name: f.ident.clone().unwrap(),
                            ty: f.ty.clone(),
                        })
                        .collect(),
                    _ => panic!("#[ffi] structs must have named fields"),
                };
                FfiTypeKind::Struct { fields }
            }
            Data::Enum(data) => {
                let variants = data
                    .variants
                    .iter()
                    .map(|v| parse_variant(v))
                    .collect();
                FfiTypeKind::Enum { variants }
            }
            Data::Union(_) => panic!("#[ffi] does not support unions"),
        }
    }
}

fn parse_variant(v: &Variant) -> EnumVariant {
    let name = v.ident.clone();
    match &v.fields {
        Fields::Unit => EnumVariant::Unit { name },
        Fields::Unnamed(fields) => {
            let types = fields.unnamed.iter().map(|f| f.ty.clone()).collect();
            EnumVariant::Tuple { name, fields: types }
        }
        Fields::Named(fields) => {
            let parsed = fields
                .named
                .iter()
                .map(|f| StructField {
                    name: f.ident.clone().unwrap(),
                    ty: f.ty.clone(),
                })
                .collect();
            EnumVariant::Struct { name, fields: parsed }
        }
    }
}
```

- [ ] **Step 2: Create codegen.rs**

companion 매크로를 생성한다. `$crate::Type`으로 소스 경로를 전달하고, 각 enum variant의 전체 경로를 match arm용으로 전달한다.

```rust
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{EnumVariant, FfiInput, FfiTypeKind, StructField};

pub fn generate(input: &FfiInput) -> TokenStream {
    let name = &input.item.ident;
    let describe_name = format_ident!("__ffi_describe_{}", name);
    let kind = input.kind();

    let descriptor = match kind {
        FfiTypeKind::Custom => quote! { @custom_type #name = $crate :: #name ; },
        FfiTypeKind::Struct { fields } => generate_struct_descriptor(name, &fields),
        FfiTypeKind::Enum { variants } => generate_enum_descriptor(name, &variants),
    };

    let original_item = &input.item;

    quote! {
        #original_item

        #[macro_export]
        macro_rules! #describe_name {
            ($callback:ident) => {
                $callback! {
                    #descriptor
                }
            };
        }
    }
}

fn generate_struct_descriptor(name: &syn::Ident, fields: &[StructField]) -> TokenStream {
    let field_entries: Vec<_> = fields
        .iter()
        .map(|f| {
            let fname = &f.name;
            let fty = &f.ty;
            quote! { @field #fname : #fty ; }
        })
        .collect();

    quote! {
        @struct #name = $crate :: #name ;
        #(#field_entries)*
        @end;
    }
}

fn generate_enum_descriptor(name: &syn::Ident, variants: &[EnumVariant]) -> TokenStream {
    let variant_entries: Vec<_> = variants
        .iter()
        .map(|v| match v {
            EnumVariant::Unit { name: vname } => {
                quote! { @unit #vname = $crate :: #name :: #vname ; }
            }
            EnumVariant::Tuple { name: vname, fields } => {
                let bindings: Vec<_> = fields.iter().enumerate().map(|(i, ty)| {
                    let var = format_ident!("_{}", i);
                    quote! { #var : #ty }
                }).collect();
                quote! { @tuple #vname ( #(#bindings),* ) = $crate :: #name :: #vname ; }
            }
            EnumVariant::Struct { name: vname, fields } => {
                let field_defs: Vec<_> = fields
                    .iter()
                    .map(|f| {
                        let fname = &f.name;
                        let fty = &f.ty;
                        quote! { #fname : #fty }
                    })
                    .collect();
                quote! { @named #vname { #(#field_defs),* } = $crate :: #name :: #vname ; }
            }
        })
        .collect();

    quote! {
        @enum #name = $crate :: #name ;
        #(#variant_entries)*
        @end;
    }
}
```

- [ ] **Step 3: Create mod.rs**

```rust
pub mod codegen;
pub mod parse;
```

- [ ] **Step 4: Register in lib.rs**

`crates/editor-macros/src/lib.rs`에 추가:

```rust
mod ffi_macro;
```

그리고 `#[ffi]` proc-macro 엔트리를 추가:

```rust
#[proc_macro_attribute]
pub fn ffi(attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(input as syn::DeriveInput);
    let input = ffi_macro::parse::FfiInput::from_attr_and_item(attr.into(), item);
    ffi_macro::codegen::generate(&input).into()
}
```

`derive_ffi!`는 editor-macros가 아닌 editor-ffi에 선언적 매크로로 정의한다 (Task 3 참조).

- [ ] **Step 5: Compile 확인**

Run: `cargo check -p editor-macros`
Expected: 성공

- [ ] **Step 6: Commit checkpoint**

`feat(editor-macros): add #[ffi] and derive_ffi! proc-macros`

---

## Task 3: `__ffi_gen` Callback Macro

editor-ffi에서 사용할 `__ffi_gen` 선언적 매크로를 구현한다.
TT-munching 패턴으로 unit/data/named variant를 순차 처리한다.

**Files:**
- Modify: `crates/editor-ffi/src/lib.rs`
- Modify: `crates/editor-ffi/Cargo.toml`

- [ ] **Step 1: Cargo.toml에 re-export 의존성 추가**

editor-ffi가 사용하는 소스 crate들을 의존성에 추가 (From 구현에서 참조):

```toml
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

- [ ] **Step 2: `__ffi_gen` 매크로 구현**

`src/lib.rs`에 `__ffi_gen` 매크로를 정의한다. 이 매크로는 `derive_ffi!` 확장을 통해 companion 매크로에서 호출된다.

```rust
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

mod types;
mod host;
mod editor;
mod bindings;
mod platform;

/// FFI 타입 생성 콜백 매크로.
/// `#[ffi]`가 생성한 companion 매크로가 이 매크로를 호출한다.
///
/// 지원하는 형태:
/// - `@custom_type Name = source::path ;`
/// - `@struct Name = source::path ; @field name : Type ; ... @end;`
/// - `@enum Name = source::path ; @unit V = source::V ; @tuple V(...) = source::V ; @named V { f: T } = source::V ; ... @end;`
macro_rules! __ffi_gen {
    // ── Custom Type (Ffi trait에 위임) ──
    (
        @custom_type $name:ident = $source:path ;
    ) => {
        #[cfg(feature = "uniffi")]
        ::uniffi::custom_type!($source, <$source as ::editor_common::ffi::Ffi>::Target);

        #[cfg(feature = "uniffi")]
        impl ::uniffi::UniffiCustomTypeConverter for $source {
            type Builtin = <Self as ::editor_common::ffi::Ffi>::Target;

            fn into_custom(val: Self::Builtin) -> ::uniffi::Result<Self> {
                Ok(::editor_common::ffi::Ffi::from_ffi(val))
            }

            fn from_custom(obj: Self) -> Self::Builtin {
                ::editor_common::ffi::Ffi::to_ffi(&obj)
            }
        }

        // C4: identity FromFfi/IntoFfi — 다른 struct 필드에서 재귀 변환 시 사용
        impl crate::convert::FromFfi<$source> for $source {
            fn from_ffi(self) -> $source { self }
        }
        impl crate::convert::IntoFfi<$source> for $source {
            fn into_ffi(self) -> $source { self }
        }
    };

    // ── Struct ──
    (
        @struct $name:ident = $source:path ;
        $( @field $field:ident : $ty:ty ; )*
        @end;
    ) => {
        #[cfg_attr(feature = "uniffi", derive(::uniffi::Record))]
        #[cfg_attr(feature = "wasm", derive(::serde::Serialize, ::serde::Deserialize))]
        #[derive(Debug, Clone)]
        pub struct $name {
            $( pub $field : $ty , )*
        }

        // 입력: FFI 타입 → core 타입 (message.from_ffi())
        impl crate::convert::FromFfi<$source> for $name {
            fn from_ffi(self) -> $source {
                $source {
                    $( $field : crate::convert::FromFfi::from_ffi(self.$field) , )*
                }
            }
        }

        // 출력: core 타입 → FFI 타입 (selection.into_ffi())
        impl crate::convert::IntoFfi<$name> for $source {
            fn into_ffi(self) -> $name {
                $name {
                    $( $field : crate::convert::IntoFfi::into_ffi(self.$field) , )*
                }
            }
        }

        // WASM 입력: JsValue → core 타입 (message.from_ffi())
        #[cfg(feature = "wasm")]
        impl crate::convert::FromFfi<$source> for wasm_bindgen::JsValue {
            fn from_ffi(self) -> $source {
                let ffi: $name = serde_wasm_bindgen::from_value(self).expect(concat!("invalid ", stringify!($name)));
                crate::convert::FromFfi::from_ffi(ffi)
            }
        }

        // WASM 출력: core 타입 → JsValue (selection.into_ffi())
        #[cfg(feature = "wasm")]
        impl crate::convert::IntoFfi<wasm_bindgen::JsValue> for $source {
            fn into_ffi(self) -> wasm_bindgen::JsValue {
                let ffi: $name = crate::convert::IntoFfi::into_ffi(self);
                serde_wasm_bindgen::to_value(&ffi).expect(concat!("serialization failed: ", stringify!($name)))
            }
        }
    };

    // ── Enum entry point: 시작 후 TT-munching으로 전환 ──
    (
        @enum $name:ident = $source:path ;
        $( $variants:tt )*
    ) => {
        __ffi_gen!(@enum_build $name = [$source]
            @gen_variants []
            @input_arms []
            @output_arms []
            @remaining [ $($variants)* ]
        );
    };

    // ── TT-munch: unit variant ──
    (
        @enum_build $name:ident = [$source:path]
        @gen_variants [ $($gv:tt)* ]
        @input_arms [ $($fs:tt)* ]
        @output_arms [ $($ff:tt)* ]
        @remaining [ @unit $variant:ident = $sv:path ; $($rest:tt)* ]
    ) => {
        __ffi_gen!(@enum_build $name = [$source]
            @gen_variants [ $($gv)* $variant , ]
            @input_arms [ $($fs)* $sv => Self::$variant , ]
            @output_arms [ $($ff)* $name::$variant => $sv , ]
            @remaining [ $($rest)* ]
        );
    };

    // ── TT-munch: tuple variant (1개 이상 unnamed fields) ──
    // proc-macro가 positional 변수명(_0, _1, ...)을 생성하여 전달한다.
    (
        @enum_build $name:ident = [$source:path]
        @gen_variants [ $($gv:tt)* ]
        @input_arms [ $($fs:tt)* ]
        @output_arms [ $($ff:tt)* ]
        @remaining [ @tuple $variant:ident ( $( $var:ident : $ty:ty ),+ ) = $sv:path ; $($rest:tt)* ]
    ) => {
        __ffi_gen!(@enum_build $name = [$source]
            @gen_variants [ $($gv)* $variant ( $($ty),+ ) , ]
            @input_arms [ $($fs)* $sv( $($var),+ ) => Self::$variant( $(crate::convert::FromFfi::from_ffi($var)),+ ) , ]
            @output_arms [ $($ff)* $name::$variant( $($var),+ ) => $sv( $(crate::convert::IntoFfi::into_ffi($var)),+ ) , ]
            @remaining [ $($rest)* ]
        );
    };

    // ── TT-munch: named-field variant ──
    (
        @enum_build $name:ident = [$source:path]
        @gen_variants [ $($gv:tt)* ]
        @input_arms [ $($fs:tt)* ]
        @output_arms [ $($ff:tt)* ]
        @remaining [ @named $variant:ident { $( $field:ident : $ty:ty ),* } = $sv:path ; $($rest:tt)* ]
    ) => {
        __ffi_gen!(@enum_build $name = [$source]
            @gen_variants [ $($gv)* $variant { $( $field : $ty ),* } , ]
            @input_arms [ $($fs)* $sv { $($field),* } => Self::$variant { $( $field : crate::convert::FromFfi::from_ffi($field) ),* } , ]
            @output_arms [ $($ff)* $name::$variant { $($field),* } => $sv { $( $field : crate::convert::IntoFfi::into_ffi($field) ),* } , ]
            @remaining [ $($rest)* ]
        );
    };

    // ── TT-munch: terminal ──
    (
        @enum_build $name:ident = [$source:path]
        @gen_variants [ $($gv:tt)* ]
        @input_arms [ $($fs:tt)* ]
        @output_arms [ $($ff:tt)* ]
        @remaining [ @end; ]
    ) => {
        #[cfg_attr(feature = "uniffi", derive(::uniffi::Enum))]
        #[cfg_attr(feature = "wasm", derive(::serde::Serialize, ::serde::Deserialize))]
        #[derive(Debug, Clone)]
        pub enum $name {
            $($gv)*
        }

        // 입력: FFI 타입 → core 타입 (message.from_ffi())
        impl crate::convert::FromFfi<$source> for $name {
            fn from_ffi(self) -> $source {
                match self {
                    $($fs)*
                }
            }
        }

        // 출력: core 타입 → FFI 타입 (selection.into_ffi())
        impl crate::convert::IntoFfi<$name> for $source {
            fn into_ffi(self) -> $name {
                match self {
                    $($ff)*
                }
            }
        }

        // WASM 입력: JsValue → core 타입
        #[cfg(feature = "wasm")]
        impl crate::convert::FromFfi<$source> for wasm_bindgen::JsValue {
            fn from_ffi(self) -> $source {
                let ffi: $name = serde_wasm_bindgen::from_value(self).expect(concat!("invalid ", stringify!($name)));
                crate::convert::FromFfi::from_ffi(ffi)
            }
        }

        // WASM 출력: core 타입 → JsValue
        #[cfg(feature = "wasm")]
        impl crate::convert::IntoFfi<wasm_bindgen::JsValue> for $source {
            fn into_ffi(self) -> wasm_bindgen::JsValue {
                let ffi: $name = crate::convert::IntoFfi::into_ffi(self);
                serde_wasm_bindgen::to_value(&ffi).expect(concat!("serialization failed: ", stringify!($name)))
            }
        }
    };
}

// derive_ffi!에서 콜백으로 사용되므로 crate 내부에서 접근 가능해야 함
pub(crate) use __ffi_gen;

/// `derive_ffi!(crate_name::TypeName)` — companion 매크로를 호출하여 FFI 타입을 생성.
/// paste crate로 매크로 이름을 동적 조합한다.
macro_rules! derive_ffi {
    ($crate_name:ident :: $type_name:ident) => {
        paste::paste! {
            $crate_name :: [<__ffi_describe_ $type_name>] ! (__ffi_gen);
        }
    };
}

pub(crate) use derive_ffi;
```

- [ ] **Step 3: Compile 확인**

Run: `cargo check -p editor-ffi`
Expected: 성공

- [ ] **Step 4: Commit checkpoint**

`feat(editor-ffi): implement __ffi_gen callback macro`

---

## Task 4: 매크로 시스템 통합 검증

간단한 타입에 `#[ffi]`를 적용하고 `derive_ffi!`로 FFI 타입을 생성하여 전체 파이프라인을 검증한다.

**Files:**
- Modify: `crates/editor-state/src/affinity.rs`
- Modify: `crates/editor-ffi/src/types.rs`

- [ ] **Step 1: Affinity에 `#[ffi]` 적용**

`crates/editor-state/src/affinity.rs`에서 Affinity enum에 `#[ffi]` 추가:

```rust
use editor_macros::ffi;

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Affinity {
    #[default]
    Downstream,
    Upstream,
}
```

- [ ] **Step 2: editor-ffi에서 derive_ffi! 호출**

`crates/editor-ffi/src/types.rs`:

```rust
derive_ffi!(editor_state::Affinity);
```

- [ ] **Step 3: Compile 확인**

Run: `cargo check -p editor-ffi`
Expected: 성공. `types::selection::Affinity` 타입이 생성되고, `From<editor_state::Affinity>` 양방향 변환이 존재.

이 단계에서 매크로 hygiene 문제가 발생할 수 있다. 만약 타입 이름이 잘못된 scope에서 resolve되면, `__ffi_gen`을 proc-macro로 전환하여 span을 `Span::call_site()`로 재작성해야 한다.

- [ ] **Step 4: 런타임 테스트 작성**

`crates/editor-ffi/src/types.rs`에 테스트 추가:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::{FromFfi, IntoFfi};

    #[test]
    fn affinity_round_trip() {
        let source = editor_state::Affinity::Upstream;
        let ffi: Affinity = source.into_ffi();
        let back: editor_state::Affinity = ffi.from_ffi();
        assert_eq!(source, back);
    }
}
```

Run: `cargo test -p editor-ffi`
Expected: PASS

- [ ] **Step 5: Struct도 검증 — Position에 적용**

`crates/editor-state/src/position.rs`에 `#[ffi]` 추가:

```rust
use editor_macros::ffi;

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub node_id: NodeId,
    pub offset: usize,
    pub affinity: Affinity,
}
```

`crates/editor-ffi/src/types.rs`에 추가:

```rust
derive_ffi!(editor_state::Position);

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::NodeId;

    // ... 기존 affinity_round_trip 테스트 ...

    #[test]
    fn position_round_trip() {
        let source = editor_state::Position::new(NodeId::ROOT, 5);
        let ffi: Position = source.into_ffi();
        let back: editor_state::Position = ffi.from_ffi();
        assert_eq!(source, back);
    }
}
```

Run: `cargo test -p editor-ffi`
Expected: PASS.

**중요:** Position의 필드 타입 `NodeId`와 `Affinity`가 FFI 컨텍스트에서 올바르게 resolve되는지 이 단계에서 확인한다. 만약 `NodeId`가 `editor_model::NodeId`로 resolve되어 UniFFI 호환 문제가 생기면, `types/mod.rs`에 `use editor_model::NodeId;` re-export 또는 `uniffi::custom_type!` 선언이 필요하다.

- [ ] **Step 6: Commit checkpoint**

`feat(editor-ffi): verify macro system end-to-end with Affinity and Position`

---

## Task 5: 소스 타입에 `#[ffi]` 마커 적용

Message hierarchy와 지원 타입 전체에 `#[ffi]`를 적용한다.

**Files:**
- Modify: 아래 나열된 모든 파일

- [ ] **Step 0: `#[ffi]` 대상 crate들에 editor-macros 의존성 추가**

`#[ffi]`를 사용하는 모든 crate의 Cargo.toml에 `editor-macros` 의존성 추가.
현재 editor-model은 이미 의존하고 있으므로 제외.

```
crates/editor-common/Cargo.toml     — [dependencies]에 editor-macros = { path = "../editor-macros" } 추가
crates/editor-state/Cargo.toml      — [dependencies]에 editor-macros = { path = "../editor-macros" } 추가
crates/editor-view/Cargo.toml       — [dependencies]에 editor-macros = { path = "../editor-macros" } 추가
crates/editor-transaction/Cargo.toml — [dependencies]에 editor-macros = { path = "../editor-macros" } 추가
crates/editor-core/Cargo.toml       — [dependencies]에 editor-macros = { path = "../editor-macros" } 추가 (현재 dev-deps에만 있음)
```

순환 의존성 없음 확인 완료 — editor-macros는 proc-macro crate이고 이 crate들을 dev-dependencies로만 참조.

- [ ] **Step 1: editor-common 타입들**

`crates/editor-common/src/geometry.rs` — `Rect`, `Size`에 `#[ffi]` 추가:
```rust
use editor_macros::ffi;

#[ffi]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }

#[ffi]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size { pub width: f32, pub height: f32 }
```

`crates/editor-common/src/movement.rs` — `Direction`, `Movement`에 `#[ffi]` 추가:
```rust
use editor_macros::ffi;

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction { Forward, Backward }

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    Grapheme(Direction),
    Word(Direction),
    Sentence(Direction),
    Line(Direction, Axis),
    Block(Direction),
    Page(Direction),
    Document(Direction),
}
```

`Movement::Line(Direction, Axis)`은 multi-field tuple variant. `#[ffi]` proc-macro가 `@tuple` descriptor와 positional 변수명(`_0`, `_1`)을 생성하고, `__ffi_gen`이 이를 처리한다.

`crates/editor-common/src/geometry.rs` — `Axis`에 `#[ffi]` 추가:
```rust
#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis { Horizontal, Vertical }
```

- [ ] **Step 2: editor-common compile 확인**

Run: `cargo check -p editor-common`
Expected: 성공

- [ ] **Step 3: editor-model 타입들**

`crates/editor-common/src/ffi.rs` — `Ffi` trait 정의:
```rust
/// FFI 경계에서 커스텀 타입으로 변환되는 타입이 구현하는 trait.
/// `#[ffi(custom)]`과 함께 사용한다.
pub trait Ffi {
    type Target;
    fn to_ffi(&self) -> Self::Target;
    fn from_ffi(value: Self::Target) -> Self;
}
```

`crates/editor-common/src/lib.rs`에 `pub mod ffi;` 추가.

`crates/editor-ffi/src/convert.rs` — `FromFfi`/`IntoFfi` trait 정의:
```rust
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

// ── 기본형 identity impls ──
// __ffi_gen이 생성하는 FromFfi/IntoFfi impl의 필드 변환에서 재귀 호출됨.

macro_rules! impl_ffi_identity {
    ($($ty:ty),*) => {
        $(
            impl FromFfi<$ty> for $ty {
                fn from_ffi(self) -> $ty { self }
            }
            impl IntoFfi<$ty> for $ty {
                fn into_ffi(self) -> $ty { self }
            }
        )*
    };
}

impl_ffi_identity!(
    bool, u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, usize, String
);

// Option<T>: 내부 값을 재귀 변환
impl<F, C> FromFfi<Option<C>> for Option<F> where F: FromFfi<C> {
    fn from_ffi(self) -> Option<C> { self.map(FromFfi::from_ffi) }
}
impl<F, C> IntoFfi<Option<C>> for Option<F> where F: IntoFfi<C> {
    fn into_ffi(self) -> Option<C> { self.map(IntoFfi::into_ffi) }
}

// Vec<T>: 내부 값을 재귀 변환
impl<F, C> FromFfi<Vec<C>> for Vec<F> where F: FromFfi<C> {
    fn from_ffi(self) -> Vec<C> { self.into_iter().map(FromFfi::from_ffi).collect() }
}
impl<F, C> IntoFfi<Vec<C>> for Vec<F> where F: IntoFfi<C> {
    fn into_ffi(self) -> Vec<C> { self.into_iter().map(IntoFfi::into_ffi).collect() }
}
```

`crates/editor-ffi/src/lib.rs`에 `pub mod convert;` 추가.

`crates/editor-model/src/id.rs` — `#[ffi(custom)]` 적용 + `Ffi` trait 구현:
```rust
use editor_macros::ffi;
use editor_common::ffi::Ffi;

#[ffi(custom)]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u64);

impl Ffi for NodeId {
    type Target = String;
    fn to_ffi(&self) -> String { self.to_string() }
    fn from_ffi(value: String) -> Self { value.parse().expect("invalid NodeId") }
}
```

`crates/editor-model/src/modifier.rs` — `Modifier`에 `#[ffi]` 추가, `ModifierType`에도 `#[strum_discriminants(ffi)]`로 적용:
```rust
use editor_macros::ffi;

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(name(ModifierType))]
#[strum_discriminants(ffi)]
#[strum_discriminants(derive(Hash, PartialOrd, Ord, Serialize, Deserialize, EnumIter, EnumCount, Enum))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
pub enum Modifier {
    Bold,
    Italic,
    // ... 모든 기존 variant 유지
}
```

`crates/editor-model/src/nodes/mod.rs` — `Node`에 `#[ffi]` 추가, `NodeType`에도 `#[strum_discriminants(ffi)]`로 적용:
```rust
use editor_macros::ffi;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumDiscriminants, FromDiscriminant)]
#[strum_discriminants(name(NodeType))]
#[strum_discriminants(ffi)]
// ... 기존 strum/serde attributes 유지 ...
pub enum Node {
    Root(RootNode),
    Paragraph(ParagraphNode),
    // ... 모든 기존 variant 유지
}
```

Node의 각 inner struct (ParagraphNode, TextNode 등)에도 `#[ffi]`를 적용해야 한다. 각 노드 정의 파일을 확인하고 named fields가 있는 struct에 `#[ffi]`를 추가한다.

- [ ] **Step 4: editor-state 타입들**

Task 4에서 이미 적용한 `Affinity`, `Position` 외에:

`crates/editor-state/src/selection.rs` — `Selection`에 `#[ffi]` 추가:
```rust
use editor_macros::ffi;

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}
```

- [ ] **Step 5: editor-view 타입들**

`crates/editor-view/src/viewport.rs` — `Viewport`에 `#[ffi]` 추가:
```rust
use editor_macros::ffi;

#[ffi]
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub scale_factor: f64,
}
```

- [ ] **Step 6: editor-core Message hierarchy**

`crates/editor-core/src/message.rs` — 모든 타입에 `#[ffi]` 추가:

```rust
use editor_macros::ffi;

#[ffi]
pub enum Message {
    Key(KeyEvent),
    Pointer(PointerEvent),
    Intent(Intent),
    System(SystemEvent),
}

#[ffi]
pub struct KeyEvent { pub key: Key, pub modifiers: KeyModifiers }

#[ffi]
pub enum Key { Enter, Backspace, Delete, Tab, Escape }

#[ffi]
pub struct KeyModifiers { pub shift: bool, pub ctrl: bool, pub alt: bool, pub meta: bool }

#[ffi]
pub enum PointerEvent {
    Down { x: f32, y: f32, count: u32, button: PointerButton, modifiers: KeyModifiers },
    Move { x: f32, y: f32, buttons: u16 },
    Up { x: f32, y: f32, button: PointerButton },
    Drag(DragEvent),
}

#[ffi]
pub enum PointerButton { Primary, Auxiliary, Secondary }

#[ffi]
pub enum DragEvent {
    Start { x: f32, y: f32 },
    Over { x: f32, y: f32 },
    Enter,
    Leave,
    End,
    Drop { x: f32, y: f32, payload: DragPayload },
}

#[ffi]
pub enum DragPayload {
    Internal,
    Text(String),
    Html { html: String, text: String },
    Files(Vec<String>),
}

#[ffi]
pub enum Intent {
    Insertion(InsertionIntent),
    Deletion(DeletionIntent),
    Formatting(FormattingIntent),
    Selection(SelectionIntent),
    Node(NodeIntent),
    Clipboard(ClipboardIntent),
    Composition(CompositionIntent),
    Navigation(NavigationIntent),
    History(HistoryIntent),
}

// 각 Intent sub-enum에도 #[ffi] 적용:
#[ffi]
pub enum InsertionIntent { Text(String), Break(BreakKind), Block(Node) }

#[ffi]
pub enum BreakKind { Block, Line, Page }

#[ffi]
pub enum DeletionIntent { Selection, Move(Movement) }

#[ffi]
pub enum FormattingIntent {
    ToggleModifier(ModifierType),
    SetModifier(Modifier),
    Clear,
    SetTextAlign(TextAlign),
    SetLineHeight(u32),
    ToggleWrap(NodeType),
    Indent,
    Outdent,
}

#[ffi]
pub enum SelectionIntent { All, Set(Selection) }

#[ffi]
pub enum NodeIntent {
    Delete { id: NodeId },
    SetAttrs { id: NodeId, attrs: Node },
    ToggleFold { id: NodeId },
    Table { id: NodeId, op: TableOp },
}

#[ffi]
pub enum TableOp {
    InsertAxis { axis: Axis, index: usize, before: bool },
    DeleteAxis { axis: Axis, index: usize },
    MoveAxis { axis: Axis, from: usize, to: usize },
    SelectAxis(Option<Axis>),
    SetColumnWidths(Vec<f32>),
}

#[ffi]
pub enum ClipboardIntent {
    Paste { html: Option<String>, text: String },
    Cut,
    Copy,
}

#[ffi]
pub enum CompositionIntent {
    Update { text: String, replace_length: Option<usize> },
    End,
}

#[ffi]
pub enum NavigationIntent { Move { movement: Movement, extend: bool } }

#[ffi]
pub enum HistoryIntent { Undo, Redo }

#[ffi]
pub enum SystemEvent {
    Initialize { width: f32, height: f32, scale_factor: f64 },
    Resize { width: f32, height: f32, scale_factor: f64 },
    SetFocused(bool),
    FontsLoaded { family: String, weight: u16 },
    SetExternalHeight { node_id: NodeId, height: f32 },
}
```

- [ ] **Step 7: editor-transaction Effect**

`crates/editor-transaction/src/effect.rs` — `Effect`에 `#[ffi]` 추가:
```rust
use editor_macros::ffi;

#[ffi]
#[derive(Clone, Debug)]
pub enum Effect {
    LoadFont { family: String, weight: u16, codepoints: Vec<u32> },
}
```

- [ ] **Step 8: 전체 compile 확인**

Run: `cargo check --workspace`
Expected: 성공. 모든 crate에 companion 매크로가 생성됨.

- [ ] **Step 9: Commit checkpoint**

`feat: apply #[ffi] markers to all editor types for FFI boundary`

---

## Task 6: FFI 타입 생성

editor-ffi의 `types.rs`에서 `derive_ffi!`를 호출하여 모든 FFI 경계 타입을 생성한다.

**Files:**
- Modify: `crates/editor-ffi/src/types.rs`

- [ ] **Step 1: types.rs — 모든 FFI 타입을 의존 순서대로 생성**

```rust
// ── Custom types (Ffi trait 구현 필요) ──
derive_ffi!(editor_model::NodeId);

// ── Geometry ──
derive_ffi!(editor_common::Axis);
derive_ffi!(editor_common::Direction);
derive_ffi!(editor_common::Movement);
derive_ffi!(editor_common::Rect);
derive_ffi!(editor_common::Size);
derive_ffi!(editor_view::Viewport);

// ── Selection ──
derive_ffi!(editor_state::Affinity);
derive_ffi!(editor_state::Position);
derive_ffi!(editor_state::Selection);

// ── Events ──
derive_ffi!(editor_core::Key);
derive_ffi!(editor_core::KeyModifiers);
derive_ffi!(editor_core::KeyEvent);
derive_ffi!(editor_core::PointerButton);
derive_ffi!(editor_core::DragPayload);
derive_ffi!(editor_core::DragEvent);
derive_ffi!(editor_core::PointerEvent);
derive_ffi!(editor_core::SystemEvent);

// ── Model (Modifier, Node + strum discriminants) ──
derive_ffi!(editor_model::Modifier);
derive_ffi!(editor_model::ModifierType);
derive_ffi!(editor_model::Node);
derive_ffi!(editor_model::NodeType);
// Node inner structs (ParagraphNode, TextNode 등)도 여기에 추가

// ── Intents (의존 타입 먼저) ──
derive_ffi!(editor_core::BreakKind);
derive_ffi!(editor_core::InsertionIntent);
derive_ffi!(editor_core::DeletionIntent);
derive_ffi!(editor_core::FormattingIntent);
derive_ffi!(editor_core::SelectionIntent);
derive_ffi!(editor_core::TableOp);
derive_ffi!(editor_core::NodeIntent);
derive_ffi!(editor_core::ClipboardIntent);
derive_ffi!(editor_core::CompositionIntent);
derive_ffi!(editor_core::NavigationIntent);
derive_ffi!(editor_core::HistoryIntent);
derive_ffi!(editor_core::Intent);
```

같은 파일에 이어서:

```rust
// ── Message (최상위) ──
derive_ffi!(editor_core::Message);

// ── Effect ──
derive_ffi!(editor_transaction::Effect);
```

- [ ] **Step 2: Compile 확인**

Run: `cargo check -p editor-ffi`
Expected: 성공. 매크로 hygiene 또는 타입 resolution 문제가 있으면 이 단계에서 드러남.

발생 가능한 문제와 대응:
- **타입 resolution**: derive_ffi! 로 생성된 타입들의 필드가 올바른 타입을 참조하는지 확인. 필요 시 `use` 문 추가.
- **순환 의존**: Intent 가 여러 sub-enum을 참조 → derive_ffi! 순서가 중요 (의존 타입 먼저)
- **Multi-field tuple variant**: `Movement::Line(Direction, Axis)` 등 — `@tuple` descriptor로 처리됨. positional 변수명(`_0`, `_1`)이 자동 생성.

- [ ] **Step 3: 런타임 테스트**

`crates/editor-ffi/src/types.rs` 하단에 테스트 추가:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::{FromFfi, IntoFfi};

    #[test]
    fn message_round_trip() {
        let source = editor_core::Message::System(editor_core::SystemEvent::Resize {
            width: 1024.0,
            height: 768.0,
            scale_factor: 2.0,
        });
        let ffi = Message::from(source.clone());
        let back: editor_core::Message = ffi.from_ffi();
        // Message가 PartialEq를 구현하지 않을 수 있으므로 variant만 확인
        assert!(matches!(back, editor_core::Message::System(editor_core::SystemEvent::Resize { .. })));
    }
}
```

Run: `cargo test -p editor-ffi`
Expected: PASS

- [ ] **Step 4: Commit checkpoint**

`feat(editor-ffi): generate all FFI boundary types via derive_ffi!`

---

## Task 7: EditorContext + EditorHost + Editor

EditorContext를 도입하여 FontRegistry 등 공유 리소스를 모든 에디터가 공유하도록 한다.
host.rs와 editor.rs에 바인딩 프리루드(lib.rs의 cfg-if 타입들)를 활용하여 타겟 무관한 코드를 작성한다.

**Files:**
- Create: `crates/editor-core/src/context.rs`
- Modify: `crates/editor-core/src/lib.rs`
- Modify: `crates/editor-core/src/editor.rs`
- Modify: `crates/editor-ffi/src/host.rs`
- Modify: `crates/editor-ffi/src/editor.rs`

- [ ] **Step 1: EditorContext 정의 (editor-core)**

`crates/editor-core/src/context.rs`:
```rust
use editor_common::{FontRegistry, TextSegmenters};

pub struct EditorContext {
    pub font_registry: FontRegistry,
    pub segmenters: Option<TextSegmenters>,
}

impl EditorContext {
    pub fn new() -> Self {
        Self {
            font_registry: FontRegistry::new(),
            segmenters: None,
        }
    }
}
```

`crates/editor-core/src/lib.rs`에 `pub use context::*;` 추가.

- [ ] **Step 2: CoreEditor가 EditorContext를 공유 참조로 받도록 변경**

`crates/editor-core/src/editor.rs`:
```rust
use std::sync::{Arc, Mutex};

use editor_common::time::Duration;
use editor_state::State;
use editor_transaction::{Effect, Step, Transaction};
use editor_view::View;
use editor_view::Viewport;

use crate::context::EditorContext;
use crate::handle::{self, HandleContext};
use crate::history::History;
use crate::message::*;

pub struct Editor {
    state: State,
    pub(crate) view: View,
    pub(crate) history: History,
    context: Arc<Mutex<EditorContext>>,
    message_queue: Vec<Message>,
}

impl Editor {
    pub fn new(state: State, viewport: Viewport, context: Arc<Mutex<EditorContext>>) -> Self {
        Self {
            state,
            view: View::new(viewport),
            history: History::new(Duration::from_millis(300)),
            context,
            message_queue: Vec::new(),
        }
    }

    // ... enqueue, tick 등 기존 메서드 유지 ...

    fn handle_transact(&mut self, f: impl FnOnce(&HandleContext, &mut Transaction) -> CommandResult) {
        let (steps, effects) = {
            let ctx_guard = self.context.lock().unwrap();
            let ctx = HandleContext {
                fonts: &ctx_guard.font_registry,
                view: &self.view,
                segmenters: ctx_guard.segmenters.as_ref(),
            };
            let mut tr = Transaction::new(&self.state);
            let _ = f(&ctx, &mut tr);
            let (_state, steps, effects) = tr.commit();
            (steps, effects)
        };
        self.apply_edit(steps, effects);
    }
}
```

기존 `font_registry` 필드와 `segmenters` 필드를 제거하고 `context`로 대체.
`set_segmenters()` 메서드도 제거 — segmenters는 EditorContext를 통해 관리.

- [ ] **Step 3: editor-core 테스트 수정**

`new_test()`도 EditorContext를 받도록 수정:
```rust
#[cfg(any(test, feature = "test-utils"))]
impl Editor {
    pub fn new_test(state: State) -> Self {
        Self {
            state,
            view: View::new_test(),
            history: History::new(Duration::from_millis(300)),
            context: Arc::new(Mutex::new(EditorContext::new())),
            message_queue: Vec::new(),
        }
    }
}
```

Run: `cargo test -p editor-core`
Expected: 기존 테스트 전부 PASS

- [ ] **Step 4: host.rs — 바인딩 프리루드 활용**

```rust
use std::sync::{Arc, Mutex};

use editor_core::{Editor as CoreEditor, EditorContext};
use editor_state::State;
use editor_view::Viewport;

use crate::{EditorResult, Owned, into_owned};

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct EditorHost {
    context: Arc<Mutex<EditorContext>>,
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl EditorHost {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    #[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen(constructor))]
    pub fn new() -> Owned<Self> {
        into_owned(Self {
            context: Arc::new(Mutex::new(EditorContext::new())),
        })
    }

    pub fn create_editor(
        &self,
        width: f32,
        height: f32,
        scale_factor: f64,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        // 초기 스코프에서는 빈 문서로 시작
        let doc = editor_model::Doc::default();
        let selection = editor_state::Selection::collapsed(
            editor_state::Position::new(editor_model::NodeId::ROOT, 0),
        );
        let state = editor_state::State::new(doc, selection);
        let viewport = Viewport::new(width, height, scale_factor);
        let core = CoreEditor::new(state, viewport, Arc::clone(&self.context));
        Ok(into_owned(crate::editor::Editor::new(core)))
    }
}
```

**참고:** `create_editor`의 State 초기화는 임시 구현. 실제로는 플랫폼에서 snapshot을 전달하거나 별도 초기화 경로가 필요.

- [ ] **Step 5: editor.rs — 바인딩 프리루드 활용**

```rust
use std::sync::Mutex;

use editor_core::Editor as CoreEditor;

use crate::{EditorResult, Input};
use crate::convert::FromFfi;
use crate::types::Message;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Editor {
    inner: Mutex<CoreEditor>,
}

// SAFETY: CoreEditor is !Send due to TextSegmenters containing icu_segmenter's
// WordSegmenter (uses Rc<Box<[u8]>> internally). Mutex<CoreEditor> guarantees that:
// 1. Only one thread accesses CoreEditor at a time (exclusive lock)
// 2. Rc's reference count is never concurrently modified
// 3. All CoreEditor access goes through with_inner() which holds the lock
// TODO: icu_segmenter의 Rc를 Arc로 교체하여 근본 해결 (별도 이슈)
unsafe impl Send for Editor {}
unsafe impl Sync for Editor {}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl Editor {
    pub fn enqueue(&self, message: Input<Message>) -> EditorResult<()> {
        self.with_inner(|e| e.enqueue(message.from_ffi()))
    }

    pub fn tick(&self) -> EditorResult<()> {
        self.with_inner(|e| e.tick())
    }
}

impl Editor {
    pub(crate) fn new(core: CoreEditor) -> Self {
        Self {
            inner: Mutex::new(core),
        }
    }

    fn with_inner<R>(&self, f: impl FnOnce(&mut CoreEditor) -> R) -> EditorResult<R> {
        Ok(f(&mut self.inner.lock().unwrap()))
    }
}
```

- [ ] **Step 6: Compile 확인**

Run: `cargo check -p editor-ffi --features uniffi`
Expected: 성공

Run: `cargo check -p editor-ffi`
Expected: 성공

- [ ] **Step 7: Commit checkpoint**

`feat(editor-ffi): implement EditorHost and Editor with cfg-if binding prelude`

---

## Task 8: Desktop Platform Surface (Stub)

최소한의 SurfaceHandle stub을 추가한다. 실제 렌더링은 editor-core의 렌더 파이프라인 완성 후 연결.

**Files:**
- Modify: `crates/editor-ffi/src/platform/desktop.rs`
- Modify: `crates/editor-ffi/src/platform/mod.rs`

- [ ] **Step 1: platform/desktop.rs — SurfaceHandle stub**

```rust
pub struct SurfaceHandle {
    width: u32,
    height: u32,
    scale_factor: f64,
    pixels: Vec<u8>,
}

impl SurfaceHandle {
    pub fn new(width: u32, height: u32, scale_factor: f64) -> Self {
        let buffer_size = (width * height * 4) as usize;
        Self {
            width,
            height,
            scale_factor,
            pixels: vec![0; buffer_size],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixel_data(&self) -> &[u8] {
        &self.pixels
    }
}
```

- [ ] **Step 2: platform/mod.rs 업데이트**

```rust
cfg_if::cfg_if! {
    if #[cfg(target_os = "android")] {
        mod android;
        pub use android::SurfaceHandle;
    } else if #[cfg(target_os = "ios")] {
        mod ios;
        pub use ios::SurfaceHandle;
    } else {
        mod desktop;
        pub use desktop::SurfaceHandle;
    }
}
```

- [ ] **Step 3: Editor에 surface 메서드 추가**

Task 7 Step 5의 `editor.rs`에 surface 관련 필드와 메서드를 추가한다. 기존 struct에 `surfaces` 필드를 추가하고, `#[cfg_attr(feature = "uniffi", uniffi::export)] impl` 블록에 surface 메서드를 추가:

```rust
use hashbrown::HashMap;
use std::sync::Mutex;

use editor_core::Editor as CoreEditor;
use crate::{EditorResult, Input};
use crate::convert::FromFfi;
use crate::platform::SurfaceHandle;
use crate::types::Message;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Editor {
    inner: Mutex<CoreEditor>,
    surfaces: Mutex<HashMap<u32, SurfaceHandle>>,
}

unsafe impl Send for Editor {}
unsafe impl Sync for Editor {}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl Editor {
    pub fn enqueue(&self, message: Input<Message>) -> EditorResult<()> {
        self.with_inner(|e| e.enqueue(message.from_ffi()))
    }

    pub fn tick(&self) -> EditorResult<()> {
        self.with_inner(|e| e.tick())
    }

    pub fn attach_surface(&self, page: u32, width: u32, height: u32, scale_factor: f64) -> EditorResult<()> {
        let surface = SurfaceHandle::new(width, height, scale_factor);
        self.surfaces.lock().unwrap().insert(page, surface);
        Ok(())
    }

    pub fn detach_surface(&self, page: u32) -> EditorResult<()> {
        self.surfaces.lock().unwrap().remove(&page);
        Ok(())
    }

    pub fn render(&self, _page: u32) -> EditorResult<()> {
        // stub: editor-core 렌더 파이프라인 연결 후 구현
        Ok(())
    }
}

impl Editor {
    pub(crate) fn new(core: CoreEditor) -> Self {
        Self {
            inner: Mutex::new(core),
            surfaces: Mutex::new(HashMap::new()),
        }
    }

    fn with_inner<R>(&self, f: impl FnOnce(&mut CoreEditor) -> R) -> EditorResult<R> {
        Ok(f(&mut self.inner.lock().unwrap()))
    }
}
```

- [ ] **Step 4: Compile 확인**

Run: `cargo check -p editor-ffi`
Expected: 성공

- [ ] **Step 5: Commit checkpoint**

`feat(editor-ffi): add desktop SurfaceHandle stub and surface management`

---

## Task 9: 최종 빌드 검증

전체 workspace 빌드와 테스트를 실행하여 기존 코드에 영향이 없는지 확인한다.

**Files:** 없음 (검증만)

- [ ] **Step 1: Workspace 전체 빌드**

Run: `cargo check --workspace`
Expected: 성공. `#[ffi]` 마커가 기존 crate의 컴파일에 영향을 주지 않아야 함.

- [ ] **Step 2: 기존 테스트 실행**

Run: `cargo test --workspace`
Expected: 기존 테스트 전부 PASS. editor-ffi 테스트도 PASS.

- [ ] **Step 3: UniFFI feature 빌드**

Run: `cargo check -p editor-ffi --features uniffi`

**Known issues (후속 작업 필요):**
- `uniffi::custom_type!`에 `<NodeId as Ffi>::Target` 복합 타입 전달 불가 — 구체 타입으로 수정 필요
- `usize`가 UniFFI 네이티브 타입이 아님 — `u64`로 변환 필요
- `NodeId: uniffi::TypeId<UniFfiTag>` 미등록 — custom_type 등록 방식 수정 필요
- `Rc<Box<[u8]>>` Send 문제가 UniFFI derive에서 표면화 — Object derive와 unsafe Send+Sync 충돌

- [ ] **Step 4: WASM feature 빌드**

Run: `cargo check -p editor-ffi --features wasm --target wasm32-unknown-unknown`
Expected: 성공. `wasm32-unknown-unknown` 타겟이 없으면 `rustup target add wasm32-unknown-unknown` 먼저 실행.

**Known issues (후속 작업 필요):**
- `Input<T>` / `Output<T>`가 WASM에서 `JsValue`로 alias되어 `T`가 unused type parameter 에러 — PhantomData wrapping 또는 newtype wrapper 필요
- `wasm_bindgen`이 타입 별칭을 처리하지 못할 수 있음 — WASM 전용 별도 타입 또는 cfg 블록 필요

- [ ] **Step 5: 양 feature 동시 활성화 방지 확인**

Run: `cargo check -p editor-ffi --features uniffi,wasm`
Expected: 컴파일 에러 (`compile_error!`가 lib.rs에 포함됨)

- [ ] **Step 6: Commit checkpoint (최종)**

`feat(editor-ffi): complete initial editor-ffi crate with macro system, UniFFI and WASM bindings`
