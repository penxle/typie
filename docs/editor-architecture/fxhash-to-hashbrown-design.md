# FxHash → hashbrown 교체 설계

## 목적

비-레거시 editor crate들에서 `rustc-hash` (`FxHashMap`/`FxHashSet`)를 `hashbrown::HashMap`/`HashSet`으로 교체한다. hashbrown의 기본 hasher인 `foldhash`를 사용한다.

## 변경 범위

레거시 `crates/editor`는 제외.

| Crate | 파일 | 변경 |
|-------|------|------|
| `editor-common` | `Cargo.toml`, `font.rs` | `rustc-hash` → `hashbrown`, `FxHashMap` → `HashMap` |
| `editor-view` | `Cargo.toml`, `engine/cache.rs`, `view_state.rs` | `rustc-hash` → `hashbrown`, `FxHashMap` → `HashMap` |
| `editor-commands` | `Cargo.toml` | 미사용 `rustc-hash` 의존성 제거 |

## 의존성 변경

```toml
# Before
rustc-hash = "2"

# After (editor-common, editor-view)
hashbrown = "0.15"

# After (editor-commands)
# rustc-hash 라인 삭제, hashbrown 추가 안 함
```

## 코드 변경 패턴

```rust
// Before
use rustc_hash::FxHashMap;
field: FxHashMap<K, V>,

// After
use hashbrown::HashMap;
field: HashMap<K, V>,
```

## Public API 영향

`editor-common::font::FontRegistry`의 public 시그니처가 `FxHashMap` → `HashMap`으로 변경된다. 레거시 `editor` crate은 `editor-common`을 사용하지 않으므로 호환성 문제 없음.

## 리스크

없음. 기계적 치환이며 레거시 영향 없음.
