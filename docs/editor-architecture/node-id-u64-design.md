# NodeId: UUID → u64 변경 설계

## 목적

- **메모리/성능**: 16바이트(UUID) → 8바이트(u64)로 줄여 HashMap 키 해싱, 비교, 메모리 사용량 개선
- **단순화**: `uuid` 크레이트 의존성 제거, 더 간결한 ID 체계

## 범위

- `crates/editor-model` 크레이트만 변경
- 레거시 `crates/editor`는 자체 `NodeId` 정의를 가지고 있으며 이번 변경에서 제외
- 변경 파일: `id.rs`, `Cargo.toml` (2개)

## 설계

### 타입 정의

```rust
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

- `ROOT`는 `0` (기존 `Uuid::nil()` 대응)
- 랜덤 생성은 `getrandom` 크레이트 사용 (WASM `js` feature 포함)
- 충돌 검사 불필요 — 단일 문서 내 노드 수 대비 64비트 랜덤 공간은 충분

### 직렬화 (Base62)

- `base62` 크레이트 사용 (직접 구현 대비 최적화된 인코딩/디코딩)
- `Serialize` / `Deserialize`는 derive 대신 커스텀 구현하여 Base62 문자열로 직렬화
- `Display` / `FromStr`도 `base62` 크레이트에 위임 (`FromStr`은 `u64` 범위 검사 포함)
- `ROOT(0)` → `"0"`, `u64::MAX` → 최대 11자

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

### 에러 타입

- `FromStr`의 에러 타입을 `uuid::Error` → `ParseNodeIdError`(커스텀)로 변경

### 의존성 변경 (Cargo.toml)

```diff
-uuid = { version = "1", features = ["v4", "serde"] }
+getrandom = { version = "0.3", features = ["wasm_js"] }
+base62 = "2"
```

### Default

```rust
impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}
```

### 테스트

| 기존 테스트 | 변경 |
|---|---|
| `root_is_nil_uuid` | → `root_is_zero`: `NodeId::ROOT.to_string() == "0"` |
| `new_generates_unique_ids` | 로직 동일 |
| `from_str_roundtrip` | Base62 라운드트립 |
| `from_str_invalid_returns_err` | 유효하지 않은 Base62 입력 |
| `copy_semantics` | 로직 동일 |

## 영향 분석

### editor-model 내부

- `doc.rs` — `imbl::HashMap<NodeId, NodeEntry>`: NodeId가 Hash/Eq 유지하므로 변경 없음
- `entry.rs` — `parent: Option<NodeId>`, `children: imbl::Vector<NodeId>`: 변경 없음
- `subtree.rs` — `id: NodeId`: 변경 없음
- `node_ref.rs` — NodeId 사용: 변경 없음

### editor-model 외부

- `editor-transaction`, `editor-commands` 등: 공개 API 동일하므로 코드 변경 없음
- 레거시 `editor` 크레이트: 자체 `NodeId` 정의, 영향 없음

## 결정 사항 요약

| 항목 | 결정 |
|---|---|
| 내부 타입 | `u64` |
| ROOT 값 | `0` |
| 랜덤 생성 | `getrandom` (WASM `js` feature) |
| 충돌 검사 | 불필요 |
| 직렬화 포맷 | Base62 문자열 (`base62` 크레이트) |
| 변경 범위 | `editor-model`만 |
