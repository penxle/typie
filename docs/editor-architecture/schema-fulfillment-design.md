# Schema Fulfillment Design

## 문제

에디터에서 노드를 삽입/이동/삭제하는 command는 content expression의 구조적 무결성을 유지해야 한다. 현재 두 가지 문제가 있다:

1. **`InsertNode` step이 삽입된 노드 자신의 content expression을 검증하지 않는다.** 빈 BulletList(content: `ListItem+`)를 삽입해도 통과하여, 중간 상태에서 document가 invalid해진다.

2. **Content fix-up이 원자적이지 않다.** 노드 이동 시 source parent의 content가 깨질 수 있으며, 현재는 ad-hoc한 전처리(ensure_valid_after_removal)로 대응한다.

## 설계 원칙

1. **모든 step 사이 중간 상태에서 document는 valid해야 한다** — 단, batch 내에서는 최종 상태만 valid하면 된다.
2. **구조적 integrity는 command의 책임** — step 인프라가 auto-fix하지 않는다. `fulfill`은 command가 명시적으로 호출하는 헬퍼 함수다.
3. **단일 노드는 크기 1인 서브트리** — insert/remove에 별도의 단일 노드 경로가 없다.

## 변경 사항

### 1. Subtree 타입 (editor-model)

`editor-model/src/subtree.rs`에 추가:

```rust
pub struct Subtree {
    pub id: NodeId,
    pub node: Node,
    pub modifiers: Vec<Modifier>,
    pub children: Vec<Subtree>,
}
```

- 구조가 곧 데이터 — orphan reference 불가능
- `parent`/`children` (NodeEntry의)는 step 적용 시 구조에서 도출
- 단일 노드 삽입은 `Subtree::leaf(id, node)`

### 2. Step 변경 (editor-transaction)

#### InsertSubtree

```rust
InsertSubtree {
    parent_id: NodeId,
    index: usize,
    subtree: Subtree,
}
```

적용 후 검증:
1. `validate_content(parent_id)` — parent의 content expression
2. `validate_context(subtree.root)` — root의 ancestor path
3. **`validate_content(node)` for every node in subtree** — 서브트리 내 모든 노드의 content expression

#### RemoveSubtree

```rust
RemoveSubtree {
    parent_id: NodeId,
    index: usize,
    subtree: Subtree,  // 제거 전 캡처 (undo용)
}
```

- root + 모든 descendants를 map에서 제거
- undo 시 전체 서브트리 복원

#### Inverse 대칭

```
InsertSubtree.inverse() → RemoveSubtree (동일 subtree)
RemoveSubtree.inverse() → InsertSubtree (동일 subtree)
```

#### MoveNode — 변경 없음

서브트리가 그대로 이동하고 parent pointer만 변경되므로 현재 구조 유지.

### 3. Transaction API (editor-transaction)

#### insert_subtree 시그니처

```rust
// Before
pub fn insert_node(&mut self, parent_id: NodeId, index: usize, node_id: NodeId, entry: NodeEntry) -> Result<(), StepError>

// After
pub fn insert_subtree(&mut self, parent_id: NodeId, index: usize, subtree: Subtree) -> Result<(), StepError>
```

#### remove_subtree

시그니처 동일. 내부적으로 document를 순회하여 descendants를 포함한 Subtree를 캡처.

```rust
pub fn remove_subtree(&mut self, node_id: NodeId) -> Result<(), StepError>
```

#### 새 메서드

```rust
/// Batch 내 step들은 개별 validation을 건너뛰고,
/// batch 종료 시 변경된 모든 노드의 content + context를 1회 검증한다.
/// 실패 시 batch 시작 시점의 상태로 rollback.
/// Undo는 batch 내 모든 step이 하나의 단위로 묶인다.
pub fn batch<F>(&mut self, f: F) -> Result<(), StepError>
where
    F: FnOnce(&mut Transaction) -> Result<(), StepError>;

/// 복수 step을 순차 적용.
pub fn apply_steps(&mut self, steps: Vec<Step>) -> Result<(), StepError>;
```

Batch 밖에서 실행되는 step은 기존과 동일하게 즉시 validation.

### 4. fulfill 헬퍼 (editor-transaction)

`editor-transaction/src/fulfill.rs`에 추가:

```rust
/// 주어진 노드의 content expression을 만족시키기 위해 필요한 InsertSubtree step들을 계산한다.
/// 이미 valid하면 빈 Vec을 반환한다.
pub fn fulfill(node: &NodeRef) -> Vec<Step>
```

동작:
1. node의 현재 children 타입과 content expression 조회
2. `content_expr.validate(&children_types)` → valid이면 빈 Vec 반환
3. Invalid일 경우, content expression의 패턴을 분류하여 필요한 삽입 결정:
   - **`Seq([..., Single(T)])`** — trailing required 누락 → 끝에 삽입
   - **`Seq([Single(T), ...])`** — leading required 누락 → 0번에 삽입
   - **`OneOrMore(expr)`** — children 없음 → first choice의 default 삽입
4. 삽입할 각 노드에 대해 재귀적으로 fulfill하여 최소 유효 서브트리 구성
5. `Step::InsertSubtree` 목록 반환

Choice resolution: content expression에서 **첫 번째 옵션**을 default로 사용 (schema 선언 순서 의존).

미지원 content expression 패턴은 에러가 아닌 빈 Vec을 반환 — command가 직접 처리.

### 5. 마이그레이션

#### 기존 insert_node 호출자

기계적 변경: `(NodeId, NodeEntry)` → `Subtree::leaf(id, node)`

```rust
// Before
tr.insert_node(parent_id, idx, new_id, NodeEntry::new(Node::Paragraph(...)))?;

// After
tr.insert_subtree(parent_id, idx, Subtree::leaf(new_id, Node::Paragraph(...)))?;
```

#### 기존 remove_node 호출자

`remove_node` → `remove_subtree`로 이름 변경. 시그니처 동일.

#### sink_paragraph_backward

ad-hoc `ensure_valid_after_removal` 제거, `batch` + `fulfill`로 교체:

```rust
// Before
ensure_valid_after_removal(tr, source_parent_id, paragraph_id)?;
tr.move_node(paragraph_id, target_id, target_children_count)?;

// After
use editor_transaction::fulfill;

tr.batch(|tr| {
    tr.move_node(paragraph_id, target_id, target_children_count)?;
    let parent = tr.doc().node(source_parent_id).ok_or(...)?;
    tr.apply_steps(fulfill(&parent))?;
    Ok(())
})?;
```

## 관련 코드

| 파일 | 변경 |
|------|------|
| `crates/editor-model/src/subtree.rs` | 신규: Subtree 타입 |
| `crates/editor-transaction/src/step.rs` | InsertSubtree/RemoveSubtree에 Subtree 사용 |
| `crates/editor-transaction/src/steps/insert_subtree.rs` | 서브트리 삽입 + 자체 content validation |
| `crates/editor-transaction/src/steps/remove_subtree.rs` | 서브트리 캡처 + 일괄 제거 |
| `crates/editor-transaction/src/transaction.rs` | batch(), apply_steps() 추가, insert_subtree/remove_subtree |
| `crates/editor-transaction/src/validate.rs` | 기존 validate 함수 재사용 |
| `crates/editor-transaction/src/fulfill.rs` | 신규: fulfill 함수 |
| `crates/editor-commands/src/commands/sink_paragraph_backward.rs` | batch + fulfill로 교체 |

## 이전 문서와의 관계

`docs/editor-architecture/content-repair-utility.md`에서 제안한 "접근 2: 패턴 기반 분류"를 `fulfill` 함수의 구현 전략으로 채택한다. 단, 원래 문서의 `ensure_content_after_removal` API 대신 `fulfill` + `batch` 패턴으로 대체하며, `InsertSubtree` step의 Subtree 도입과 자체 content validation 추가가 전제 조건이다.
