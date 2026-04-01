# Document Model

## 개요

Loro를 제거하고, `imbl` 기반의 불변 문서 모델로 대체한다.

## 구조

```rust
#[derive(Clone)]  // O(1) — imbl 구조적 공유
pub struct Doc {
    nodes: imbl::HashMap<NodeId, NodeEntry>,
    settings: DocumentSettings,
    default_attrs: DefaultAttrs,
}

#[derive(Clone)]
pub struct NodeEntry {
    pub node: Node,
    pub parent: Option<NodeId>,
    pub children: imbl::Vector<NodeId>,
    pub cascade_attrs: Vec<Attr>,
    pub remarks: Vec<Remark>,
}
```

## 설계 결정사항

### Flat map (`imbl::HashMap<NodeId, NodeEntry>`)

- 기존 코드에서 `doc.node(node_id)`로 직접 접근하는 패턴이 광범위하므로 flat map 유지
- `imbl::HashMap`으로 불변성 + 구조적 공유 확보 (Clone O(1), update O(log n))
- nested tree 대비 NodeId 기반 O(log n) 임의 접근이 가능

### 양방향 parent-children 링크

- NodeEntry에 `parent`와 `children` 모두 유지
- parent 접근이 빈번 (스타일 캐스케이드, 스키마 검증, 조상 순회)
- 불변 모델에서는 변이가 Step.apply() 한 곳에서만 발생하므로 동기화 보장이 용이
- Lexical식 linked list (prev/next sibling)는 불변 모델에서 map update 횟수가 더 많아 부적합

### `imbl::Vector<NodeId>` for children

- `Vec<NodeId>` 대신 `imbl::Vector<NodeId>` 사용
- root 노드처럼 children이 수백~수천 개인 경우 Vec clone이 비쌈
- imbl::Vector: Clone O(1), insert/remove O(log n), index O(log n)
- children이 작을 때는 차이 없고, 클 때 극적으로 개선

### 노드 타입 (기존 19가지 유지)

Root, Paragraph, Text, Blockquote, Callout, BulletList, OrderedList, ListItem,
Fold, FoldTitle, FoldContent, Table, TableRow, TableCell, Image, File, Embed,
Archived, HardBreak, HorizontalRule, PageBreak

### 텍스트 (기존 세그먼트 구조 유지)

```rust
pub struct TextNode {
    pub segments: Vec<TextSegment>,
}

pub struct TextSegment {
    pub text: String,
    pub styles: Vec<Style>,
    pub annotations: Vec<Annotation>,
}
```

### 라이브러리 선택: `imbl` (not `im`)

- `im` crate는 2022년 이후 폐기 상태 (메인테이너 무응답, 미수정 버그 존재)
- `imbl`은 `im`의 공식 포크, API 호환, 활발히 유지보수 (최신 7.0.0, 2026-01)
- Rc 기본 (싱글 스레드 에디터 + WASM에 적합)

## 비용 분석

| 연산 | 비용 |
|---|---|
| `Doc::clone()` | O(1) |
| `doc.node(id)` | O(log n) |
| `doc.with_node(id, entry)` | O(log n) |
| `NodeEntry::clone()` (children 포함) | O(1) (imbl::Vector) |
| children insert/remove | O(log n) |
| children index 접근 | O(log n) |
