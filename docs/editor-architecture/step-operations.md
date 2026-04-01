# Step/Operation 설계

## 개요

11가지 원자 Step으로 에디터의 모든 변이를 표현한다.
각 Step은 apply, inverse, transform을 지원하여 변이/undo/sync를 단일 통화로 해결한다.

## Step 정의

```rust
enum Step {
    // ── 텍스트 ──
    InsertText {
        node_id: NodeId,
        offset: usize,
        text: String,
        styles: Vec<Style>,
        annotations: Vec<Annotation>,
    },
    RemoveText {
        node_id: NodeId,
        offset: usize,
        text: String,            // 삭제된 내용 보존 (inverse용)
        styles: Vec<Style>,
        annotations: Vec<Annotation>,
    },

    // ── 노드 구조 ──
    InsertNode {
        parent_id: NodeId,
        index: usize,
        node_id: NodeId,
        entry: NodeEntry,
    },
    RemoveNode {
        parent_id: NodeId,
        index: usize,
        node_id: NodeId,
        entry: NodeEntry,        // 삭제된 entry 보존 (inverse용)
    },
    MoveNode {
        node_id: NodeId,
        old_parent: NodeId,
        old_index: usize,
        new_parent: NodeId,
        new_index: usize,
    },
    SplitNode {
        node_id: NodeId,
        offset: usize,          // 텍스트 노드: 문자 오프셋 / 엘리먼트: 자식 인덱스
        new_node_id: NodeId,
    },
    MergeNode {
        node_id: NodeId,         // 병합되어 사라지는 노드
        target_id: NodeId,       // 병합 대상
        offset: usize,
    },

    // ── 속성 ──
    SetNode {
        node_id: NodeId,
        old_properties: NodeProperties,
        new_properties: NodeProperties,
    },

    // ── 마크 ──
    AddMark {
        node_id: NodeId,
        from: usize,
        to: usize,
        mark: Mark,              // Style | Annotation
    },
    RemoveMark {
        node_id: NodeId,
        from: usize,
        to: usize,
        mark: Mark,
    },

    // ── 선택 ──
    SetSelection {
        old: Selection,
        new: Selection,
    },
}
```

## Inverse 관계

| Step | Inverse |
|---|---|
| InsertText | RemoveText (같은 위치, 같은 내용) |
| RemoveText | InsertText |
| InsertNode | RemoveNode |
| RemoveNode | InsertNode |
| MoveNode | MoveNode (old↔new 교환) |
| SplitNode | MergeNode |
| MergeNode | SplitNode |
| SetNode | SetNode (old↔new 교환) |
| AddMark | RemoveMark |
| RemoveMark | AddMark |
| SetSelection | SetSelection (old↔new 교환) |

## 상위 연산 → Step 분해 예시

### 문단 분할 (Enter)

```
Paragraph(P1)
  ├── Text(T1, "Hello ", bold)
  ├── Text(T2, "beautiful ", italic)    ← 커서: offset 4
  └── Text(T3, "world")
```

Steps:
1. `SplitNode { node_id: T2, offset: 4, new_node_id: T4 }` — 텍스트 분할
2. `SplitNode { node_id: P1, offset: 2, new_node_id: P2 }` — 문단 분할, 인덱스 2부터 이동
3. `SetSelection { new: collapsed(P2, 0) }`

Undo (역순 inverse):
1. `SetSelection` (old↔new)
2. `MergeNode { node_id: P2, target_id: P1 }` — 문단 병합
3. `MergeNode { node_id: T4, target_id: T2 }` — 텍스트 병합

### 볼드 적용 (선택 범위)

```
1. AddMark { node_id: T1, from: 3, to: 8, mark: Bold }
2. SetSelection { ... }
```

### 리스트 항목 들여쓰기

```
1. RemoveNode { old_parent: list, index: 2, ... }
2. InsertNode { parent: prev_item, index: ..., ... }
3. SetSelection { ... }
```

## SplitNode 상세

SplitNode의 offset은 노드 타입에 따라 의미가 다르다:
- **텍스트 노드**: 문자 오프셋. 원본은 [0, offset), 새 노드는 [offset, end)
- **엘리먼트 노드**: 자식 인덱스. 원본은 children[0..offset], 새 노드는 children[offset..]

새 노드는 원본의 부모에서 원본 바로 뒤에 삽입된다.

imbl map 연산 (텍스트 분할 시):
1. `nodes.update(원본)` — 텍스트 잘림
2. `nodes.insert(새 노드)` — 뒷부분
3. `nodes.update(부모)` — children에 새 노드 추가
= 3회 O(log n)

## 설계 근거

- Slate의 9가지 Operation을 기반으로, 텍스트 마크(스타일/어노테이션) 연산 2개를 추가 (총 11개)
- SplitNode/MergeNode를 원자 연산으로 포함: 문단 분할이 2 Step으로 표현 가능, inverse가 자연스러움
- 모든 Step이 inverse를 위한 데이터를 자체 보존 (RemoveText는 삭제된 텍스트, RemoveNode는 entry 등)
