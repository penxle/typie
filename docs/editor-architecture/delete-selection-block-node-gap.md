# delete_selection: Block-Level Node Selection 미지원 문제

## 문제

현재 `delete_selection`의 cross-node 삭제 알고리즘은 from/to가 **inline 위치(text node 안 또는 empty paragraph)**에 있다고 가정한다. Block-level leaf node(image, horizontal_rule, file, embed 등)에서 selection이 시작되거나 끝나는 경우를 올바르게 처리하지 못한다.

## 현재 알고리즘의 가정

```
trim_from(from)           ← from node 내부에서 뒷부분 삭제
trim_to(to)               ← to node 내부에서 앞부분 삭제
collect_fully_selected()  ← from과 to 사이의 완전 선택된 노드 수집
merge_after_delete()      ← 경계 textblock 병합
```

이 4단계는 from/to가 "내부 content를 가진 노드" (text node의 character 위치, 또는 paragraph의 child index)일 때만 올바르게 동작한다.

## 실패 케이스

### Case 1: from이 block-level leaf node

```
root {
  |image {}|                    ← selection start (from)
  paragraph { text("Wor|ld") } ← selection end (to)
}
```

**Position 표현 방식에 따른 문제:**

- `from = (image, 0)`: `trim_from`이 image의 children을 삭제하려 하지만 image는 children이 없음 → **image가 삭제되지 않음**
- `from = (root, index_of_image)`: `trim_from`이 root의 children을 해당 index부터 전부 삭제 → **to 쪽 paragraph까지 삭제되어 데이터 손실**

### Case 2: to가 block-level leaf node

```
root {
  paragraph { text("He|llo") } ← selection start (from)
  |image {}|                    ← selection end (to)
}
```

- `to = (image, 0)`: `trim_to`가 offset 0이므로 아무것도 삭제 안 함 → **image가 남아있음**
- `to = (root, index_after_image)`: `trim_to`가 root의 children을 처음부터 해당 index까지 삭제 → **from 쪽 paragraph도 삭제**

### Case 3: 양쪽 모두 block-level

```
root {
  |image {}|                    ← from
  |horizontal_rule {}|          ← to
}
```

어떤 position 표현을 쓰더라도 현재 알고리즘으로는 처리 불가.

## 근본 원인

`trim_from`/`trim_to`는 "노드 내부의 부분 삭제"를 수행한다. 하지만 block-level leaf node는 "부분 삭제"라는 개념 자체가 없다 — 전체를 삭제하거나 유지하거나 둘 중 하나다.

현재 알고리즘은 이 두 가지 타입의 노드를 구분하지 않고 동일한 trim 로직을 적용한다.

## 커서 위치 문제

Cross-node 삭제 후 커서 위치도 block-level node에서 문제가 있다:

```rust
let cursor = if tr.doc().node(from.node_id).is_some() {
    from
} else {
    Position { node_id: lca_id, offset: 0, ... }  // 부정확한 fallback
};
```

- `from.node_id`가 삭제된 text node일 때 → `(lca_id, 0)` fallback이 부정확
- `from.node_id`가 삭제된 image일 때 → 같은 문제

올바른 커서: from의 parent에서 from이 있던 index 위치. Batch 전에 기록 필요:
```rust
let from_parent_id = doc.node(from.node_id).and_then(|n| n.parent()).map(|p| p.id());
let from_index = doc.node(from.node_id).and_then(|n| n.index());
```

## 해결 방향

### 접근 1: Position 정규화

Cross-node 삭제 진입 전에 from/to position을 정규화하여, block-level leaf node에 대한 position을 parent의 child index로 변환:

```
normalize(image, 0) → (root, index_of_image)      // "image 앞"
normalize(image, 0) → (root, index_of_image + 1)   // "image 뒤" (to일 때)
```

이렇게 하면 from/to가 항상 "container + child index" 형태가 되고, 기존 trim/collect/merge 로직을 확장하여 처리 가능.

### 접근 2: Trim 분리

`trim_from`/`trim_to`를 두 경우로 분리:
- **Inline position** (text node 안): 기존 로직 유지
- **Block position** (container의 child index): from/to node 자체를 `collect_fully_selected`에 포함하고, trim은 skip

이 경우 `collect_fully_selected`이 from/to node도 수집 대상에 포함해야 할 수 있다.

### 접근 3: 통합 deletion 모델

From/to 위치를 "document path + offset"으로 표현하고, path의 각 레벨에서 삭제 범위를 결정하는 통합 모델. ProseMirror의 `ReplaceStep`이 이 방식. 가장 범용적이지만 구현 복잡도가 높다.

### 추천: 접근 1

Position 정규화가 가장 비침투적(non-invasive). 기존 알고리즘의 큰 변경 없이 진입부에 정규화 단계만 추가. Block-level node의 position을 parent child index로 변환하면, 해당 node는 `collect_fully_selected`에서 수집되어 `remove_subtree`로 삭제된다.

## 영향 받는 노드 타입

현재 schema에서 block-level leaf node (children이 없거나 external):

| Node | selectable | external | 비고 |
|------|-----------|----------|------|
| Image | true | true | 블록 이미지 |
| File | true | true | 파일 첨부 |
| Embed | true | true | 임베드 |
| Archived | true | true | 아카이브된 노드 |
| HorizontalRule | true | false | 구분선 |

## 현재 상태

`delete_selection`은 inline selection (text-to-text) 케이스만 올바르게 동작. Block-level node selection은 미지원. 이 문제를 해결하기 전까지 block-level node가 포함된 selection 삭제는 정의되지 않은 동작(undefined behavior)이다.

## 관련 코드

| 파일 | 관련 함수 |
|------|----------|
| `crates/editor-commands/src/commands/delete_selection.rs` | `trim_from`, `trim_to`, `collect_fully_selected`, 커서 fallback |
| `crates/editor-commands/src/helpers/tree.rs` | `find_ancestor_textblock` (merge 대상 탐색) |
