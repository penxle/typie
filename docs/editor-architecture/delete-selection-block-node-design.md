# delete_selection: Block-Level Node Selection 지원 설계

## 배경

`delete_selection`의 cross-node 삭제 알고리즘은 from/to가 inline 위치(text node 안 또는 empty textblock)에 있다고 가정한다. Block-level leaf node(Image, HorizontalRule, File, Embed, Archived)에서 selection이 시작되거나 끝나는 경우를 처리하지 못한다.

문제의 상세 분석: [delete-selection-block-node-gap.md](delete-selection-block-node-gap.md)

## Block-Level Leaf Selection의 Position 표현

Block-level leaf node가 선택된 경우:

```
from: Position { node_id: parent_id, offset: index_of_node,     affinity: Downstream }
to:   Position { node_id: parent_id, offset: index_of_node + 1, affinity: Upstream }
```

Cross-node 삭제에서 from 또는 to가 이 형태일 때, `from.node_id`가 LCA 자체가 될 수 있다. 이 경우 기존 `trim_from(lca, offset)`이 LCA의 모든 children을 삭제하여 데이터 손실이 발생한다.

## 접근: Unified Range Deletion

기존 4-phase 알고리즘(trim_from -> trim_to -> collect_fully_selected -> merge_after_delete)에서 앞 3개를 하나의 재귀 walk으로 교체한다.

```
기존: trim_from -> trim_to -> collect_fully_selected -> merge_after_delete
변경: delete_range (재귀 walk)                        -> merge_after_delete
```

모든 boundary 조합(Inline x Inline, Block x Inline, Inline x Block, Block x Block)을 하나의 코드 경로로 통합 처리한다.

## 알고리즘: 재귀 Walk

### Path 계산

LCA 기준 상대 path를 계산한다. ResolvedPosition의 path (top-down `Vec<usize>`)를 활용한다.

```rust
fn path_from_ancestor(doc: &Doc, node_id: NodeId, ancestor_id: NodeId) -> Vec<usize>
```

- `node_id`에서 `ancestor_id`까지 올라가며 각 level의 child index 수집 후 reverse
- offset을 마지막에 append

예시:
- `from=(root, 0)` with LCA=root -> from_path = `[0]`
- `to=(text, 3)` in root[1].para[0] with LCA=root -> to_path = `[1, 0, 3]`
- `from=(fold_content, 0)` in root[0].fold[1] with LCA=root -> from_path = `[0, 1, 0]`

### 3개의 상호 재귀 함수

#### delete_range(from_path, to_path, node_id)

from과 to가 모두 이 subtree 안에 있을 때 호출.

```
from_idx = from_path[0]
to_idx = to_path[0]

if from_idx == to_idx:
  (from.len==1, to.len>1) -> delete_to(to_path[1..], child)
  (from.len>1, to.len==1) -> delete_from(from_path[1..], child)
  (둘 다 >1)              -> delete_range(from_path[1..], to_path[1..], child)

if from_idx < to_idx:
  1. From boundary:
     from.len==1 -> 이 level의 offset (child가 fully selected에 포함됨)
     from.len>1  -> delete_from(from_path[1..], children[from_idx])

  2. Fully selected 중간 노드:
     fully_from = if from.len==1 { from_idx } else { from_idx + 1 }
     fully_to   = to_idx
     children[fully_from..fully_to]를 역순으로 remove_subtree

  3. To boundary:
     to.len==1 -> no-op (to offset까지가 selection 범위)
     to.len>1  -> delete_to(to_path[1..], children[to_idx])
```

#### delete_from(path, node_id)

path 위치부터 subtree 끝까지 삭제. 기존 trim_from의 재귀 버전.

```
if path.len() == 1:  (leaf level)
  Text   -> offset==0: remove_subtree, else: remove_text(offset..end)
  Container -> remove children[offset..] 역순

else:
  idx = path[0]
  remove children[idx+1..] 역순
  delete_from(path[1..], children[idx])
```

#### delete_to(path, node_id)

subtree 시작부터 path 위치까지 삭제. 기존 trim_to의 재귀 버전.

```
if path.len() == 1:  (leaf level)
  Text   -> offset>=len: remove_subtree, else: remove_text(0..offset)
  Container -> remove children[..offset] 역순

else:
  idx = path[0]
  remove children[..idx] 역순
  delete_to(path[1..], children[idx])
```

## Cursor 위치 결정

삭제 후 결과는 `Selection`이다 (collapsed가 아닐 수 있음).

### Inline from (from.node_id가 Text)

- 텍스트 부분 삭제 (offset > 0): `Selection::collapsed(from.node_id, from.offset)` -- 노드 생존
- 텍스트 전체 삭제 (offset == 0): 기존 `resolve_cursor_after_removal`로 인접 text sibling 탐색

### Block from (from.node_id가 block-children container)

`resolve_selection_at(doc, container_id, offset)` -- 새 함수:

1. `children[offset]` 존재 (삭제 경계 바로 뒤):
   - block-level leaf -> node selection: `(container, offset, Downstream) -> (container, offset+1, Upstream)`
   - textblock/container -> walk down -> collapsed selection
2. `children[offset]` 없고 `children[offset-1]` 존재 (바로 앞):
   - block-level leaf -> node selection: `(container, offset-1, Downstream) -> (container, offset, Upstream)`
   - textblock/container -> walk down to last position -> collapsed selection
3. 둘 다 없음 -> fulfill 후 재탐색

### delete_within_node에도 적용

기존 delete_within_node의 container branch도 `(node_id, from_offset)` 반환 시 block-children container이면 유효하지 않은 collapsed selection을 반환하는 버그가 있다. `resolve_selection_at`을 여기에도 적용한다.

## merge_after_delete 조정

### Textblock 사전 기록

from/to의 text 노드가 delete_range에서 삭제될 수 있으므로 (offset 0이면 remove_subtree), textblock을 삭제 전에 기록한다:

```rust
// delete_range 전
let from_tb = find_ancestor_textblock(&doc, from.node_id);
let to_tb = find_ancestor_textblock(&doc, to.node_id);

// delete_range 실행

// merge -- pre-computed textblock 사용
merge_after_delete(tr, from_tb, to_tb, lca_id)?;
```

시그니처 변경: `from: &Position, to: &Position` 대신 `from_tb: Option<NodeId>, to_tb: Option<NodeId>`.

### Block boundary에서 merge가 일어나지 않는 이유

Block boundary의 from/to.node_id는 block-children container (root, fold_content 등)이다. `find_ancestor_textblock`이 이를 위로 탐색하면 textblock을 찾지 못하고 None을 반환한다. 따라서 merge가 자연스럽게 skip된다.

이는 올바른 동작이다. Block boundary는 textblock을 "가르지" 않으므로 병합할 대상이 없다.

### Merge 내부 로직

merge_node, cascade merge, prune, fulfill은 기존과 동일. 변경 없음.

### Fulfill 범위 확장

delete_range 후 중간 container가 비어질 수 있다 (예: fold_content에서 유일한 child가 삭제된 경우). 기존 fulfill(lca)만으로는 깊은 container까지 도달하지 못할 수 있다.

fulfill 대상을 확장한다:
- from.node_id (block from인 경우)
- to.node_id (block to인 경우)
- lca_id (기존)

## 코드 변경 요약

| 삭제 | 추가 | 수정 |
|------|------|------|
| `trim_from` | `delete_range` | `merge_after_delete` (시그니처, fulfill 범위) |
| `trim_to` | `delete_from` | `delete_within_node` (커서 resolve) |
| `collect_fully_selected` | `delete_to` | `delete_selection` (진입부 흐름) |
| | `path_from_ancestor` | |
| | `resolve_selection_at` | |

## 영향 받는 노드 타입

현재 schema에서 block-level leaf를 direct child로 허용하는 container:

| Container | Content | Isolating |
|-----------|---------|-----------|
| Root | `(Paragraph \| Image \| File \| ...)*` | No |
| FoldContent | `(Paragraph \| Image \| File \| ...)+` | Yes |
| TableCell | `(Paragraph \| Image \| File \| ...)+` | Yes |

Selection은 isolating 경계를 넘을 수 있다.

## 테스트 전략

기존 7개 테스트(inline-to-inline)가 모두 통과해야 한다. 추가 테스트:

1. Block from, inline to: image -> text 부분 삭제
2. Inline from, block to: text 부분 -> image 삭제
3. Block from, block to (same parent): image + hr 삭제
4. Block from, block to (different parents): 다른 container의 block node 삭제
5. Nested: FoldContent 안 image -> root의 text
6. Cursor: block 삭제 후 인접 block node 선택
7. Cursor: block 삭제 후 인접 textblock으로 collapsed selection
8. Merge 불발: block from일 때 인접 paragraph가 merge되지 않음 확인
9. Cleanup: 삭제 후 빈 container에 fulfill 적용 확인
