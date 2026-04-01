# Doc-Bound Selection Type

## 동기

현재 `Selection`은 bare value type으로, doc 컨텍스트가 필요한 연산마다 `&Doc`을 매번 전달해야 한다:

```rust
let (from, to) = selection.as_sorted(&doc);
compare_positions(&doc, pos_a, pos_b);
position_in_selection(&doc, pos, &selection);
```

`NodeId` → `NodeRef`의 관계처럼, `Selection` → `SelectionRef`로 Doc에 바인딩하면 이런 연산들이 메서드로 자연스러워진다.

## 현재 구조

```rust
// crates/editor-state/src/position.rs
pub struct Position {
    pub node_id: NodeId,
    pub offset: usize,
    pub affinity: Affinity,
}

// crates/editor-state/src/selection.rs
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn collapsed(pos: Position) -> Self;
    pub fn new(anchor: Position, head: Position) -> Self;
    pub fn is_collapsed(&self) -> bool;
    pub fn as_sorted(&self, doc: &Doc) -> (Position, Position);  // doc 필요
}
```

## 제안: `SelectionRef<'a>`

### 타입 정의

```rust
// crates/editor-model/src/selection_ref.rs (또는 editor-state에 배치)
pub struct SelectionRef<'a> {
    doc: &'a Doc,
    selection: Selection,
}
```

### 생성

```rust
// Doc에서 생성
let sel_ref = doc.selection(&selection);
// 또는 Selection에서 생성
let sel_ref = selection.resolve(&doc);
```

### Doc-dependent 메서드

`SelectionRef`에서만 제공되는 메서드들 (doc 파라미터 불필요):

```rust
impl<'a> SelectionRef<'a> {
    /// anchor와 head를 문서 순서로 정렬
    pub fn as_sorted(&self) -> (Position, Position);

    /// from (문서 순서 기준 앞쪽)
    pub fn from(&self) -> Position;

    /// to (문서 순서 기준 뒤쪽)
    pub fn to(&self) -> Position;

    /// position이 이 selection 범위 안에 있는지
    pub fn contains(&self, pos: Position) -> bool;

    /// selection이 걸쳐있는 모든 block-level node ID 수집
    pub fn selected_blocks(&self) -> Vec<NodeId>;

    /// from과 to의 lowest common ancestor
    pub fn lowest_common_ancestor(&self) -> Option<NodeId>;

    /// from/to가 속한 textblock (있으면)
    pub fn from_textblock(&self) -> Option<NodeRef<'a>>;
    pub fn to_textblock(&self) -> Option<NodeRef<'a>>;
}
```

### 기존 Selection은 유지

`Selection`은 bare value type으로 유지. State에 저장되고, Step으로 변이되고, serialize되는 것은 여전히 `Selection`. `SelectionRef`는 읽기 전용 view.

```rust
// State에서
pub struct State {
    pub doc: Doc,
    pub selection: Selection,  // bare type 유지
    ...
}

// Command에서 사용
let sel = tr.selection();           // Selection (bare)
let doc = tr.doc();
let sel_ref = sel.resolve(&doc);    // SelectionRef (doc-bound)
let (from, to) = sel_ref.as_sorted();
```

## NodeRef와의 패턴 비교

| | NodeRef | SelectionRef |
|---|---|---|
| Bare type | `NodeId` | `Selection` |
| Bound type | `NodeRef<'a>` | `SelectionRef<'a>` |
| 바인딩 대상 | `&'a Doc` | `&'a Doc` |
| 생성 | `doc.node(id)` | `selection.resolve(&doc)` |
| 용도 | 트리 탐색, 속성 조회 | 위치 비교, 범위 연산 |

## 위치 결정

두 가지 선택지:

**A. `editor-model`에 배치** — `NodeRef`와 같은 crate. `Doc`을 직접 참조하므로 자연스러움. 단, `Selection`과 `Position`이 `editor-state`에 있어서 cross-crate 의존이 생김.

**B. `editor-state`에 배치** — `Selection`과 같은 crate. `editor-state`가 `editor-model`에 이미 의존하므로 `Doc`에 접근 가능. `Selection`의 확장으로 자연스러움.

**추천: B.** `SelectionRef`는 Selection의 "resolved" 형태이므로 Selection과 같은 crate에 두는 것이 맞다.

## 영향 범위

### 즉시 마이그레이션 가능한 사용처

- `Selection::as_sorted(&doc)` → `SelectionRef::as_sorted()`
- `compare_positions(&doc, a, b)` → position path 비교 로직이 SelectionRef 내부로
- `find_lowest_common_ancestor` (delete_selection에서 사용) → `SelectionRef::lowest_common_ancestor()`

### 향후 활용

- `delete_selection` — `sel_ref.as_sorted()`, `sel_ref.lowest_common_ancestor()`
- `delete_selection`의 merge 단계 — `sel_ref.from_textblock()`, `sel_ref.to_textblock()`
- 범위 서식 적용 — `sel_ref.selected_blocks()`
- 선택 영역 하이라이팅 — `sel_ref.contains(pos)`

## position_path 헬퍼

`SelectionRef` 내부에서 position 비교를 위해 필요:

```rust
fn position_path(doc: &Doc, pos: &Position) -> Vec<usize> {
    let mut path = doc.node(pos.node_id)
        .map(|n| n.path())
        .unwrap_or_default();
    path.push(pos.offset);
    path
}
```

이것은 `NodeRef::path()` (root→self까지의 child index 경로)에 의존. `NodeRef::path()`가 아직 없으면 함께 추가 필요.

## 구현 순서

1. `NodeRef::path()` 추가 (editor-model)
2. `SelectionRef` 타입 정의 (editor-state)
3. `as_sorted`, `from`, `to` 구현
4. `contains`, `lowest_common_ancestor` 구현
5. 기존 `Selection::as_sorted(&doc)` → `SelectionRef`로 마이그레이션
6. command에서 `SelectionRef` 활용

## 관련 파일

| 파일 | 역할 |
|------|------|
| `crates/editor-model/src/node_ref.rs` | `path()` 메서드 추가 |
| `crates/editor-state/src/selection_ref.rs` | SelectionRef 타입 (새로 생성) |
| `crates/editor-state/src/lib.rs` | export 추가 |
| `crates/editor-commands/src/commands/*.rs` | 마이그레이션 대상 |
