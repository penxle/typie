# NodeId u64 변경 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** editor-model의 NodeId를 UUID 기반에서 u64 랜덤 기반으로 변경하여 메모리/성능 개선 및 uuid 의존성 제거

**Architecture:** NodeId(Uuid) → NodeId(u64). getrandom으로 랜덤 생성, base62 크레이트로 직렬화. 커스텀 Serialize/Deserialize, Display, FromStr 구현.

**Tech Stack:** Rust, getrandom (0.3, js feature), base62 (2), serde

**Design doc:** `docs/editor-architecture/node-id-u64-design.md`

---

## File Map

| 파일 | 작업 | 설명 |
|---|---|---|
| `crates/editor-model/Cargo.toml` | Modify | uuid 제거, getrandom + base62 추가 |
| `crates/editor-model/src/id.rs` | Modify | NodeId 타입 재구현 |

---

### Task 1: 의존성 변경

**Files:**
- Modify: `crates/editor-model/Cargo.toml:15`

- [ ] **Step 1: Cargo.toml 의존성 교체**

`crates/editor-model/Cargo.toml`에서 uuid를 제거하고 getrandom + base62를 추가한다:

```toml
# 15행의 uuid 라인을 제거하고 다음 두 줄로 교체:
getrandom = { version = "0.3", features = ["js"] }
base62 = "2"
```

변경 후 전체 dependencies 섹션:

```toml
[dependencies]
editor-common = { path = "../editor-common" }
editor-macros = { path = "../editor-macros" }
enum-map = "2"
imbl = "7"
strum = { version = "0.28", features = ["derive"] }
getrandom = { version = "0.3", features = ["js"] }
base62 = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: 의존성 해결 확인**

Run: `cargo check -p editor-model 2>&1 | head -20`
Expected: uuid 관련 import 에러 발생 (아직 id.rs가 uuid를 사용 중이므로). 의존성 자체는 resolve 되어야 함.

---

### Task 2: NodeId 타입 및 생성자 변경

**Files:**
- Modify: `crates/editor-model/src/id.rs:1-16`

- [ ] **Step 1: import와 타입 정의 변경**

`crates/editor-model/src/id.rs`의 1~16행을 다음으로 교체한다:

```rust
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u64);

impl NodeId {
    pub const ROOT: Self = Self(0);

    pub fn new() -> Self {
        let mut buf = [0u8; 8];
        getrandom::fill(&mut buf).expect("failed to generate random bytes");
        Self(u64::from_le_bytes(buf))
    }
}
```

핵심 변경: `Serialize`/`Deserialize` derive 제거 (커스텀 구현으로 대체), `uuid` import 제거.

- [ ] **Step 2: cargo check로 컴파일 상태 확인**

Run: `cargo check -p editor-model 2>&1 | head -30`
Expected: Display, FromStr, Serialize, Deserialize 관련 에러 (아직 구현을 업데이트하지 않았으므로).

---

### Task 3: Display, FromStr, 에러 타입 구현

**Files:**
- Modify: `crates/editor-model/src/id.rs:18-36`

- [ ] **Step 1: 에러 타입, Display, FromStr, Default 구현 교체**

`crates/editor-model/src/id.rs`의 기존 `impl fmt::Display` ~ `impl Default` 블록 (18~36행)을 다음으로 교체한다:

```rust
#[derive(Debug)]
pub struct ParseNodeIdError;

impl fmt::Display for ParseNodeIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid NodeId")
    }
}

impl std::error::Error for ParseNodeIdError {}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&base62::encode(self.0))
    }
}

impl FromStr for NodeId {
    type Err = ParseNodeIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n = base62::decode(s).map_err(|_| ParseNodeIdError)?;
        u64::try_from(n).map(Self).map_err(|_| ParseNodeIdError)
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: cargo check로 Serialize/Deserialize 누락만 남았는지 확인**

Run: `cargo check -p editor-model 2>&1 | head -30`
Expected: Serialize/Deserialize trait 미구현 에러만 남아야 함.

---

### Task 4: 커스텀 Serde 구현

**Files:**
- Modify: `crates/editor-model/src/id.rs` (Default impl 뒤에 추가)

- [ ] **Step 1: Serialize, Deserialize 구현 추가**

`crates/editor-model/src/id.rs`의 `Default` impl 블록 바로 뒤에 다음을 추가한다:

```rust
impl Serialize for NodeId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&base62::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for NodeId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let n = base62::decode(&s).map_err(serde::de::Error::custom)?;
        u64::try_from(n)
            .map(Self)
            .map_err(|_| serde::de::Error::custom("NodeId overflow"))
    }
}
```

- [ ] **Step 2: 전체 크레이트 컴파일 확인**

Run: `cargo check -p editor-model 2>&1`
Expected: 컴파일 성공 (warning은 가능).

---

### Task 5: 테스트 업데이트

**Files:**
- Modify: `crates/editor-model/src/id.rs:38-73` (tests 모듈)

- [ ] **Step 1: 테스트 모듈 교체**

`crates/editor-model/src/id.rs`의 `#[cfg(test)] mod tests` 블록 전체를 다음으로 교체한다:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_zero() {
        assert_eq!(NodeId::ROOT.to_string(), "0");
    }

    #[test]
    fn new_generates_unique_ids() {
        let a = NodeId::new();
        let b = NodeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn from_str_roundtrip() {
        let id = NodeId::new();
        let s = id.to_string();
        let parsed = NodeId::from_str(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn from_str_invalid_returns_err() {
        assert!(NodeId::from_str("!!!invalid").is_err());
    }

    #[test]
    fn from_str_overflow_returns_err() {
        // u64::MAX + 1 in base62
        assert!(NodeId::from_str("LygHa16AHYG").is_err());
    }

    #[test]
    fn copy_semantics() {
        let a = NodeId::new();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn serde_roundtrip() {
        let id = NodeId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }
}
```

- [ ] **Step 2: 테스트 실행**

Run: `cargo test -p editor-model --lib -- id::tests -v`
Expected: 7개 테스트 모두 PASS.

- [ ] **Step 3: 전체 editor-model 테스트 실행**

Run: `cargo test -p editor-model --lib`
Expected: 기존 51개 테스트 모두 PASS (id 모듈 테스트 수 변경으로 총 수는 약간 달라질 수 있음).

---

### Task 6: 의존 크레이트 컴파일 검증

**Files:** 없음 (검증만)

- [ ] **Step 1: editor-model을 사용하는 크레이트 컴파일 확인**

Run: `cargo check -p editor-transaction -p editor-commands 2>&1`
Expected: 컴파일 성공. NodeId의 공개 API(new, ROOT, Display, FromStr, Serialize, Deserialize, Hash, Eq, Ord)가 동일하므로 변경 불필요.

- [ ] **Step 2: 전체 crates 워크스페이스 테스트**

Run: `cargo test -p editor-model -p editor-transaction -p editor-commands 2>&1 | tail -20`
Expected: 모든 테스트 PASS.
