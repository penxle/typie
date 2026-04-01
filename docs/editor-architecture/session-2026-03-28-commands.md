# Editor Commands 구현 세션 (2026-03-28 ~ 03-29)

## 개요

`editor-commands` crate에 12개 command, 3개 transaction utility, 4개 command helper를 구현. 기본 텍스트 편집(입력/삭제/분할/병합)에서 cross-node selection 삭제까지 다루며, 레거시 에디터의 ~1,650줄 monolithic `delete_selection`을 구조적으로 분해된 설계로 재구현.

## 구현된 Commands (12개)

### 입력 계열
| Command | 역할 |
|---------|------|
| `insert_text` | 텍스트 입력. modifier 상속, pending modifiers 적용 |
| `insert_hard_break` | Shift+Enter. paragraph 내 줄바꿈 (HardBreakNode 삽입) |
| `split_paragraph` | Enter. paragraph를 cursor 위치에서 분할 (`split_node` 활용) |

### Backspace 체인
```rust
first([delete_text_backward, select_node_backward, delete_node_backward,
       join_paragraph_backward, sink_paragraph_backward])
```

| Command | 역할 |
|---------|------|
| `delete_text_backward` | text 문자 1개 삭제. cross-sibling text 삭제, 빈 노드 제거 |
| `delete_node_backward` | 같은 parent 내 cursor 직전의 non-text 노드 삭제 |
| `join_paragraph_backward` | paragraph 시작에서 이전 paragraph와 병합 (`merge_node`) |
| `sink_paragraph_backward` | paragraph 시작에서 이전 container의 가장 깊은 호환 위치로 이동. content + context 검증, `fulfill`로 source parent 수리 |

### Delete 체인
```rust
first([delete_text_forward, select_node_forward, delete_node_forward,
       join_paragraph_forward, lift_paragraph_forward])
```

| Command | 역할 |
|---------|------|
| `delete_text_forward` | Delete 키 텍스트 삭제. Backspace의 거울상 |
| `delete_node_forward` | Delete 키 노드 삭제. Backspace의 거울상 |
| `join_paragraph_forward` | paragraph 끝에서 다음 paragraph와 병합 |
| `lift_paragraph_forward` | paragraph 끝에서 다음 container의 첫 paragraph를 꺼내 병합. `prune`/`dissolve`로 cleanup |

### Selection 삭제
| Command | 역할 |
|---------|------|
| `delete_selection` | 선택 영역 삭제. same-node / cross-node 분기. LCA 기반 trim → collect → merge → cleanup |

## Transaction Utilities (3개)

| Utility | 위치 | 역할 |
|---------|------|------|
| `fulfill` | 기존 | content violation을 InsertSubtree로 수리 (노드 삽입) |
| `prune` | 신규 | 빈 container 재귀 제거. `min_required() > 0`이고 children 비어있을 때 |
| `dissolve` | 신규 | content invalid 노드의 children을 parent로 promote 후 제거. 재귀적 |

### `Transaction::batch` 개선

```rust
// Before: StepError만 허용
pub fn batch<F>(&mut self, f: F) -> Result<(), StepError>

// After: generic error type
pub fn batch<F, E>(&mut self, f: F) -> Result<(), E>
where E: From<StepError>
```

Command에서 `CommandError`를 직접 사용 가능. `into_step` 브릿지 불필요:
```rust
tr.batch::<_, CommandError>(|tr| {
    trim_from(tr, &from)?;        // CommandError
    tr.remove_subtree(node_id)?;  // StepError → CommandError via From
    Ok(())
})?;
```

## Command Helpers (4개)

| Helper | 위치 | 역할 |
|--------|------|------|
| `find_lowest_common_ancestor` | `helpers/position.rs` | 두 노드의 LCA 탐색 |
| `trim_from` / `trim_to` | `helpers/delete_range.rs` | cross-node 삭제에서 양쪽 끝 부분 삭제 |
| `collect_fully_selected` | `helpers/delete_range.rs` | LCA 기반으로 완전 선택된 중간 노드 수집 |
| `merge_after_delete` | `helpers/merge_after_delete.rs` | 삭제 후 textblock merge + container merge + cleanup |

## Schema 확장

| 추가 | 위치 | 역할 |
|------|------|------|
| `ContentExpr::matches_sequence` | `editor-schema/content.rs` | 기존 `validate`를 감싸는 bool wrapper. content 호환성 검증에 사용 |

## 아키텍처 결정사항

1. **Command 책임 원칙**: 트랜잭션은 deterministic해야 하므로 auto-fix 불가. 구조적 integrity 유지는 command의 책임. move/remove 전에 fix-up(fulfill/prune/dissolve) 수행.

2. **Command 네이밍 체계**:
   - `delete_text_*` — text 문자 삭제만
   - `delete_node_*` — non-text 노드 삭제만
   - `join_paragraph_*` — sibling paragraph 간 merge만
   - `sink_paragraph_*` — container 안으로 이동
   - `lift_paragraph_*` — container에서 꺼내기

3. **delete_selection 분해**: 레거시의 monolithic 구현 대신 4단계 decomposition:
   - `trim_edges` → `delete_fully_selected` → `merge_after_delete` → cleanup
   - 각 단계가 독립 헬퍼로 단독 테스트 가능

4. **Multi-level merge**: 삭제 후 textblock merge + ancestor chain을 올라가며 인접 동일 타입 container merge. 비대칭 depth에서는 textblock merge만 수행.

## 테스트 현황

| Crate | Tests |
|-------|-------|
| editor-commands | 120 |
| editor-transaction | 83 |
| editor-model | 51 |
| editor-state | 18 |
| editor-schema | 17 |
| **Total** | **289** |

## 파일 구조

```
crates/editor-commands/src/
├── commands/
│   ├── delete_node_backward.rs
│   ├── delete_node_forward.rs
│   ├── delete_selection.rs
│   ├── delete_text_backward.rs
│   ├── delete_text_forward.rs
│   ├── insert_hard_break.rs
│   ├── insert_text.rs
│   ├── join_paragraph_backward.rs
│   ├── join_paragraph_forward.rs
│   ├── lift_paragraph_forward.rs
│   ├── sink_paragraph_backward.rs
│   └── split_paragraph.rs
├── helpers/
│   ├── delete_range.rs
│   ├── merge_after_delete.rs
│   ├── position.rs
│   └── resolve_effective_modifiers.rs
├── compose.rs
├── error.rs
├── lib.rs
└── test_utils.rs

crates/editor-transaction/src/
├── dissolve.rs    (신규)
├── fulfill.rs
├── prune.rs       (신규)
├── step.rs
├── steps/
├── transaction.rs (batch 개선)
└── validate.rs

crates/editor-schema/src/
└── content.rs     (matches_sequence 추가)
```

## 관련 설계 문서

| 문서 | 내용 |
|------|------|
| `docs/editor-architecture/content-repair-utility.md` | fulfill/prune/dissolve 설계 배경 |
| `docs/editor-architecture/doc-bound-selection.md` | ResolvedPosition/ResolvedSelection 설계 |
| `docs/superpowers/specs/2026-03-28-sink-paragraph-backward-design.md` | sink command 상세 설계 |
| `docs/superpowers/specs/2026-03-29-lift-paragraph-forward-design.md` | lift + prune + dissolve 상세 설계 |
| `docs/superpowers/specs/2026-03-29-delete-selection-design.md` | delete_selection 상세 설계 |
