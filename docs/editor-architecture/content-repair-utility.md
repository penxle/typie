# Content Repair Utility

## 문제

노드를 이동/삭제하는 command에서 source parent의 content expression이 깨질 수 있다. 예를 들어 `sink_paragraph_backward`가 Root의 마지막 Paragraph를 blockquote로 이동하면, Root는 `(...)*, Paragraph` 패턴의 trailing Paragraph 요구사항을 충족하지 못한다.

Transaction의 step(`move_node`, `remove_node` 등)은 실행 시점에 즉시 content/context validation을 수행하므로, "이동 후 fix-up"은 불가능하다. **Fix-up을 먼저 수행하고, 그 다음에 이동/삭제 step을 실행**해야 한다.

## 아키텍처 원칙

1. **트랜잭션은 deterministic** — auto-fix 시스템은 도입 불가. 각 step은 실행 시점에 valid해야 한다.
2. **구조적 integrity는 command의 책임** — step 인프라가 아닌 command 레벨에서 fix-up을 수행한다.
3. **Savepoint/rollback은 command에서 사용하지 않는다** — 이것은 `compose.rs`의 `first`/`can` 등 조합 유틸리티를 위한 것이다.

## 필요한 유틸리티

### 위치

`crates/editor-commands/src/helpers/` 모듈에 범용 헬퍼로 구현한다.

### API 설계 방향

```rust
/// 특정 child가 parent에서 제거될 때, parent의 content expression을 유지하기 위해
/// 필요한 fix-up 노드를 삽입한다. remove/move step보다 먼저 호출해야 한다.
pub fn ensure_content_after_removal(
    tr: &mut Transaction,
    parent_id: NodeId,
    child_id: NodeId,
) -> Result<(), CommandError>
```

### 처리해야 하는 케이스

현재 schema에서 child 제거 시 content가 깨질 수 있는 패턴:

| Parent | Content Expression | 위반 조건 | Fix-up |
|--------|-------------------|-----------|--------|
| **Root** | `(choice)*, Paragraph` | 마지막(trailing) Paragraph 제거 | 빈 Paragraph를 끝에 삽입 |
| **Blockquote** | `(P\|BL\|OL)+` | 유일한 child 제거 | 빈 Paragraph 삽입 |
| **Callout** | `(P\|BL\|OL)+` | 유일한 child 제거 | 빈 Paragraph 삽입 |
| **ListItem** | `Paragraph, (BL\|OL)*` | 첫 번째 Paragraph 제거 | position 0에 빈 Paragraph 삽입 |
| **BulletList** | `ListItem+` | 유일한 ListItem 제거 | 빈 ListItem(+빈 Paragraph) 삽입 |
| **OrderedList** | `ListItem+` | 유일한 ListItem 제거 | 빈 ListItem(+빈 Paragraph) 삽입 |
| **FoldContent** | `(big_choice)+` | 유일한 child 제거 | 빈 Paragraph 삽입 |
| **TableCell** | `(big_choice)+` | 유일한 child 제거 | 빈 Paragraph 삽입 |
| **Fold** | `FoldTitle, FoldContent` | structural이므로 독립 제거 불가 | 해당 없음 |
| **Table/TableRow** | `TableRow+` / `TableCell+` | structural | 해당 없음 |

### 구현 전략

#### 접근 1: Content expression 기반 범용 repair

`ContentExpr`의 구조를 분석하여 "제거 후 남는 children"이 valid한지 확인하고, invalid할 경우 최소한의 노드를 삽입하여 valid하게 만든다.

```
repair(content_expr, remaining_children) -> Vec<(index, NodeType)>
```

이 접근은 schema가 변경되어도 자동으로 대응하지만, content expression이 grammar이므로 "최소 repair"를 계산하는 것이 복잡하다. 특히 Seq 패턴에서 어떤 위치에 무엇을 삽입해야 하는지 결정하려면 grammar matching + gap filling이 필요하다.

#### 접근 2: 패턴 기반 분류

Content expression을 구조적으로 분류하여 repair 전략을 결정한다:

- **`OneOrMore(expr)` / `ZeroOrMore(expr)`**: 유일한 child 제거 시 expr의 "default type"을 1개 삽입
- **`Seq([..., Single(T)])`**: trailing required element 제거 시 해당 type의 default를 끝에 삽입
- **`Seq([Single(T), ...])`**: leading required element 제거 시 해당 type의 default를 0번에 삽입
- **structural node**: 독립 제거 불가 → repair 불필요

이 접근은 현재 schema의 실제 패턴에 맞춰 구현하므로 단순하고 정확하다. 새 content expression 패턴이 추가되면 명시적으로 지원을 추가해야 한다.

#### 추천: 접근 2

현재 schema의 content expression은 상대적으로 단순한 패턴들의 조합이다. 범용 grammar repair를 구현하는 것은 과도하며, 패턴 분류 기반이 실용적이다.

### "Default type" 결정

Fix-up 시 삽입할 기본 노드:

| 필요한 타입 | Default 생성 |
|------------|-------------|
| Paragraph | `NodeEntry::new(Node::Paragraph(ParagraphNode::default()))` |
| ListItem | ListItem + 빈 Paragraph (structural child 포함) |
| FoldTitle | `NodeEntry::new(Node::FoldTitle(FoldTitleNode::default()))` |
| FoldContent | FoldContent + 빈 Paragraph |
| TableRow | TableRow + TableCell + 빈 Paragraph |
| TableCell | TableCell + 빈 Paragraph |

ListItem, FoldContent, TableRow, TableCell은 자체 content expression이 빈 상태를 허용하지 않으므로, fix-up 시 **재귀적으로 최소 유효 서브트리**를 생성해야 한다.

### 검증 방법

```rust
// 제거 전: remaining children이 parent content에 valid한지 확인
let remaining: Vec<NodeType> = parent.children()
    .filter(|c| c.id() != child_id)
    .map(|c| c.node().as_type())
    .collect();

if parent.node().spec().content.matches_sequence(&remaining) {
    return Ok(()); // fix-up 불필요
}

// fix-up 수행 후 재검증
// ...
```

## 사용처

현재 `sink_paragraph_backward`에 ad-hoc 구현(`ensure_valid_after_removal`)이 있으며, 이것을 범용 헬퍼로 교체해야 한다.

향후 사용 예상:
- `sink_paragraph_backward` — paragraph를 컨테이너로 이동 시 source parent fix-up
- `lift` / `lift_from_ancestor` — 노드를 컨테이너 밖으로 꺼낼 때 source container fix-up
- `drag_and_drop` — 노드 이동 시 source parent fix-up
- `delete_node_with_selection_adjustment` — 노드 삭제 시 parent fix-up

## 현재 상태

`sink_paragraph_backward` 내 `ensure_valid_after_removal`이 ad-hoc으로 "끝에 빈 Paragraph 삽입"만 수행한다. Root에서는 동작하지만 다른 parent 타입에서는 올바른 fix-up을 생성하지 못한다.

## 관련 코드

| 파일 | 역할 |
|------|------|
| `crates/editor-schema/src/content.rs` | `ContentExpr`, `matches_sequence`, `validate` |
| `crates/editor-schema/src/lib.rs` | 전체 schema 정의 (각 노드의 content expression) |
| `crates/editor-commands/src/commands/sink_paragraph_backward.rs` | 현재 ad-hoc 구현 |
| `crates/editor-commands/src/helpers/` | 범용 헬퍼 모듈 (여기에 구현 예정) |
