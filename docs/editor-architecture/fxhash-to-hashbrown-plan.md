# FxHash → hashbrown 교체 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 비-레거시 editor crate에서 `rustc-hash`를 `hashbrown`으로 교체

**Architecture:** 3개 crate의 Cargo.toml 의존성 교체 + 소스 코드의 import/타입 치환. 기계적 변경.

**Tech Stack:** Rust, hashbrown 0.15 (foldhash default hasher)

**Spec:** `docs/editor-architecture/fxhash-to-hashbrown-design.md`

---

### Task 1: editor-commands — 미사용 rustc-hash 의존성 제거

**Files:**
- Modify: `crates/editor-commands/Cargo.toml:13`

- [ ] **Step 1: Cargo.toml에서 rustc-hash 제거**

`crates/editor-commands/Cargo.toml`에서 `rustc-hash = "2"` 라인 삭제.

```toml
[dependencies]
editor-common = { path = "../editor-common" }
editor-model = { path = "../editor-model" }
editor-schema = { path = "../editor-schema" }
editor-state = { path = "../editor-state" }
editor-transaction = { path = "../editor-transaction" }
enum-map = "2"
thiserror = "2"
unicode-segmentation = "1"
```

- [ ] **Step 2: 컴파일 확인**

Run: `cargo check -p editor-commands`
Expected: 성공 (사용처 없으므로)

---

### Task 2: editor-common — rustc-hash → hashbrown

**Files:**
- Modify: `crates/editor-common/Cargo.toml:11`
- Modify: `crates/editor-common/src/font.rs`

- [ ] **Step 1: Cargo.toml 의존성 교체**

`crates/editor-common/Cargo.toml`에서 `rustc-hash = "2"` → `hashbrown = "0.15"`.

```toml
[dependencies]
bytecount = { git = "https://github.com/devunt/bytecount.git", features = ["generic-simd"] }
hashbrown = "0.15"
smallvec = "1"
web-time = "1"
```

- [ ] **Step 2: font.rs import 교체**

`crates/editor-common/src/font.rs` 1행:

```rust
// Before
use rustc_hash::FxHashMap;

// After
use hashbrown::HashMap;
```

- [ ] **Step 3: font.rs 타입 치환**

`font.rs` 전체에서 `FxHashMap` → `HashMap` 일괄 치환. 대상:

- L5: `families: FxHashMap<String, SmallVec<[u16; 9]>>` → `HashMap<...>`
- L7: `family_index: FxHashMap<String, u16>` → `HashMap<...>`
- L8: `font_mappings: FxHashMap<(u16, u16), FxHashMap<u32, (u16, u16)>>` → `HashMap<(u16, u16), HashMap<u32, (u16, u16)>>`
- L14, L16, L17: `FxHashMap::default()` → `HashMap::default()`
- L21: `pub fn update(&mut self, families: FxHashMap<String, Vec<u16>>)` → `HashMap<...>`
- L97: `-> Option<&FxHashMap<u32, (u16, u16)>>` → `-> Option<&HashMap<u32, (u16, u16)>>`
- L140, L204, L214: `let mut families = FxHashMap::default()` → `HashMap::default()`

- [ ] **Step 4: 컴파일 확인**

Run: `cargo check -p editor-common`
Expected: 성공

---

### Task 3: editor-view — rustc-hash → hashbrown

**Files:**
- Modify: `crates/editor-view/Cargo.toml:12`
- Modify: `crates/editor-view/src/engine/cache.rs`
- Modify: `crates/editor-view/src/view_state.rs`

- [ ] **Step 1: Cargo.toml 의존성 교체**

`crates/editor-view/Cargo.toml`에서 `rustc-hash = "2"` → `hashbrown = "0.15"`.

```toml
[dependencies]
editor-common = { path = "../editor-common" }
editor-model = { path = "../editor-model" }
editor-state = { path = "../editor-state" }
editor-transaction = { path = "../editor-transaction" }
hashbrown = "0.15"
parley = { version = "0.7", default-features = false, features = ["std"] }
```

- [ ] **Step 2: engine/cache.rs 교체**

`crates/editor-view/src/engine/cache.rs`:

```rust
// Before
use rustc_hash::FxHashMap;
// ...
    entries: FxHashMap<NodeId, Arc<Measurement>>,

// After
use hashbrown::HashMap;
// ...
    entries: HashMap<NodeId, Arc<Measurement>>,
```

`FxHashMap` → `HashMap` 일괄 치환.

- [ ] **Step 3: view_state.rs 교체**

`crates/editor-view/src/view_state.rs`:

```rust
// Before
use rustc_hash::FxHashMap;
// ...
    pub fold_states: FxHashMap<NodeId, bool>,
    pub external_heights: FxHashMap<NodeId, f32>,

// After
use hashbrown::HashMap;
// ...
    pub fold_states: HashMap<NodeId, bool>,
    pub external_heights: HashMap<NodeId, f32>,
```

`FxHashMap` → `HashMap` 일괄 치환.

- [ ] **Step 4: 전체 컴파일 확인**

Run: `cargo check -p editor-view`
Expected: 성공

---

### Task 4: 전체 빌드 검증

- [ ] **Step 1: workspace 전체 check**

Run: `cargo check --workspace`
Expected: 성공. 레거시 `editor` crate은 자체 `rustc-hash` 의존성을 유지하므로 영향 없음.
