# Command Implementation Status

## Overview

editor 2.5 아키텍처의 step 3 — command 시스템 구현 현황.
기존 Loro 기반 에디터의 편집 로직을 새 imbl 기반 불변 모델로 이식하는 작업.

## Crate 구조

```
crates/
├── editor-common/       # FontRegistry 등 공통 타입
├── editor-model/        # imbl 기반 불변 문서 모델 (Doc, Node, NodeEntry)
├── editor-state/        # State (doc, selection, pending_modifiers, composition)
├── editor-schema/       # NodeSpec, ContentExpr, 스키마 검증
├── editor-transaction/  # Step (14종), Transaction, Effect
├── editor-commands/     # Command 함수들 (이 문서의 대상)
└── editor-macros/       # doc!, state! 테스트 매크로
```

## 테스트 현황

| Crate | Passed | Failed | Ignored |
|-------|--------|--------|---------|
| editor-commands | 281 | 0 | 86 |
| editor-transaction | 48 | 0 | 0 |
| editor-model | 44 | 0 | 0 |
| editor-common | 10 | 0 | 0 |

## 완료된 작업

### A단계: Command 인프라 + PoC

- `editor-commands` crate 생성
- `CommandError` (`From<StepError>` for `?` 전파), `CommandResult` type alias
- 합성 유틸리티: `first()`, `chain()`, `can()`
- `Savepoint`에 `Clone` derive
- PoC command 3개: `split_paragraph`, `delete_selection`, `toggle_modifier`

### B단계: 핵심 편집 command 이식

39개 command를 독립 함수로 이식. 기존 테스트를 기존 이름 그대로 포팅.

**구현된 command:**

| 그룹 | Commands |
|------|----------|
| 텍스트 입력/삭제 | `insert_text`, `surround_selection`, `insert_hard_break`, `insert_page_break`, `delete_text_backward`, `delete_text_forward`, `delete_selection` |
| 문단/블록 구조 | `split_paragraph`, `join_backward`, `join_forward`, `set_text_align`, `set_line_height`, `reset_fully_selected_paragraphs`, `insert_paragraph_on_nontextblock_selection` |
| 서식/스타일 | `toggle_modifier`, `set_modifier`, `toggle_bold_modifier`, `reset_all_modifiers`, `recompute_pending_modifiers` |
| 노드 조작 | `insert_node`, `wrap_in`, `wrap_in_ancestor`, `lift`, `lift_from_ancestor`, `lift_on_empty_paragraph`, `expand_selection_until` |
| 리스트 | `split_list_item`, `lift_list_item`, `sink_list_item`, `merge_list_item_backward`, `merge_list_item_forward`, `toggle_bullet_list`, `toggle_ordered_list` |
| 선택 | `select_word_at`, `select_sentence_at`, `select_paragraph_at`, `collapse_selection` |
| 합성 | `first`, `chain`, `can` |

**헬퍼:**

| 파일 | 역할 |
|------|------|
| `helpers/selection.rs` | `normalize_selection`, `update_selection` (set_selection + pending 재계산) |
| `helpers/offset.rs` | `find_child_at_offset`, `calculate_offset_before_child`, `block_content_len` |
| `helpers/text.rs` | `collect_text_ranges` |
| `helpers/modifier.rs` | `text_node_has_modifier`, `range_has_modifier`, `split_text_at_offset`, `apply_modifier_to_ranges`, `remove_modifier_from_ranges`, `modifiers_to_enum_map`, `compute_pending_modifiers`, `update_modifiers_if_empty_textblock`, `update_modifiers_on_empty_textblocks_in_range`, `remove_modifier_on_empty_textblocks_in_range` |
| `helpers/grapheme.rs` | `find_prev_grapheme_boundary`, `find_next_grapheme_boundary` |
| `helpers/split_block_at.rs` | `split_block_at` (내부 헬퍼) |
| `helpers/split_paragraph_at_cursor.rs` | `split_paragraph_at_cursor` (내부 헬퍼) |
| `helpers/join_blocks.rs` | join 관련 헬퍼 |
| `helpers/move_to_next_block.rs` | `move_to_next_block` |

### C1단계: 시스템 인프라 (진행 중)

**완료:**

1. `editor-common` crate — `FontRegistry` (FxHashMap + SmallVec 기반)
2. `State` 확장 — `pending_modifiers: EnumMap<ModifierType, Option<Modifier>>`, `composition: Option<Composition>`
3. Step 구성 — `AddModifier`, `RemoveModifier`, `SetModifiers`, `SetPendingModifiers`, `SetComposition`
4. `LoadFont` Effect 추가
5. Transaction 코어 메서드 — `pending_modifiers()`, `set_pending_modifiers()`, `add_modifier()`, `remove_modifier()`, `set_modifiers()`, `composition()`, `set_composition()`
6. 기존 Step apply에서 새 State 필드 보존
7. `state!` 매크로 — `pending_modifiers: [italic, bold]` 구문, 모든 노드에 modifier shorthand `[bold, line_height(200)]`, positional args `[font_weight(700)]`
8. Modifier 통합 — `Attr`/`Mark`/`Style`/`StyleType` → `Modifier`/`ModifierType` 단일 타입, segment 기반 → node-level modifier
9. Pending modifiers 연동 — `insert_text`, `toggle_modifier`, `set_modifier` collapsed, `delete_text_backward`/`forward`
10. 빈 textblock modifier 동기화 — `recompute_pending_modifiers` fallback, `update_modifiers_if_empty_textblock`, split/delete 후 동기화
11. Pending styles 테스트 활성화 — `delete_selection`, `delete_text_backward`, `delete_text_forward`, `insert_text`, `join_backward`, `join_forward` 등 23개 real test `#[ignore]` 해제

## 남은 작업

### C1단계 잔여 (86 ignored tests 활성화)

72개 stub (구현 필요) + 14개 real test (버그 수정 필요)

| 카테고리 | Ignored 수 | 타입 | 필요한 작업 |
|----------|-----------|------|------------|
| Font normalization | 48 | stub | `toggle_bold_modifier`/`set_modifier`/`toggle_modifier` — `FontRegistry` 기반 font-weight-aware 로직 구현 |
| Cascade attrs | 8 | stub | Block-level modifier 상속 시스템 (root → paragraph 계층 전파) |
| Trailing paragraph | 3+3 | real | split/join/delete 후 빈 문단 정규화 로직 |
| Effects | 5 | 4 stub + 1 real | `LoadFont` effect emit 연동 |
| Table selection | 5 | 4 stub + 1 real | Table rectangular selection (C2에서 처리) |
| Slot positions | 3 | stub | Position affinity 기반 slot 처리 |
| Layout | 3 | stub | Layout cache invalidation (View 시스템 의존) |
| Insert text | 3 | real | text node 분할 후 target 해석, pending modifier 적용 |
| Pending modifiers | 2 | stub | 나머지 command에 pending_modifiers 연동 |
| Merge node | 1 | real | merge 시 distinct modifier 보존 (`join_backward_style_diff`) |
| Cross-paragraph | 1 | stub | `collect_text_ranges` cross-parent 확장 |
| Command logic fix | 1 | real | `join_backward` 빈 문단 삭제 로직 수정 |

### C2단계: 나머지 command 전량 이식

| 그룹 | Commands |
|------|----------|
| 블록인용/콜아웃 | `toggle_blockquote`, `set_blockquote`, `toggle_callout`, `cycle_callout_variant` |
| 폴드 | `insert_fold`, `unwrap_fold`, `toggle_fold` |
| 문서 레벨 | `insert_fragment`, `delete_range`, `replace_range`, `merge_blocks_content`, `move_node_range`, `delete_node_recursive`, `clean_up_empty_ancestors`, `delete_text_range`, `merge_adjacent_text_nodes`, `try_lift_block`, `delete_node_with_selection_adjustment` |
| 클립보드 | `paste_text`, `paste_text_fragment`, `paste_fragment` |
| 드래그앤드롭 | `drag_and_drop`, `drag_and_copy`, `drop_external` |
| 테이블 | `insert_table`, `add_table_row`, `add_table_column`, `delete_table_row`, `delete_table_column`, `move_table_row`, `move_table_column`, `set_column_widths`, `set_table_*`, `select_table_*` |
| Composition (IME) | `set_composition`, `complete_composition`, `commit_composition` |
| 어노테이션/리마크 | `add_annotation`, `update_annotation`, `remove_annotation`, `add_remark`, `update_remark`, `remove_remark` |
| 기타 | `insert_horizontal_rule`, `set_horizontal_rule`, `text_replacement`, `set_document_settings`, `set_layout_mode`, `set_block_gap`, `set_paragraph_indent`, `set_default_attrs` |

## 아키텍처 결정사항

1. **Command 시그니처**: `pub fn command_name(tr: &mut Transaction, ...) -> CommandResult`
2. **Modifier 통합**: `Attr`/`Mark`/`Style`/`Annotation` → `Modifier` 단일 enum. `NodeEntry.modifiers: Vec<Modifier>`에 노드 수준으로 적용
3. **Pending modifiers**: `EnumMap<ModifierType, Option<Modifier>>` — State에 포함, `SetPendingModifiers` Step으로 변이
4. **텍스트 범위 서식**: segment 기반이 아닌 node split + `AddModifier`/`RemoveModifier`. `split_text_at_offset` → `apply_modifier_to_ranges` 패턴
5. **빈 textblock modifier**: `set_modifiers(block_id, ...)` — 빈 paragraph에 pending modifiers 동기화 (기존 `cascade_attrs` 대체)
6. **Composition**: `Option<Composition>` — State에 포함, `SetComposition` Step으로 변이
7. **FontRegistry**: `FxHashMap<String, SmallVec<[u16; 9]>>` — `editor-common` crate, command에 `&FontRegistry` 파라미터로 전달
8. **Effect**: `LoadFont { family, weight, codepoints }` — command에서 `tr.push_effect()`로 emit
9. **Schema 활용**: `Schema::node_spec()`, `Schema::modifier_spec()` — 노드/modifier 속성 질의에 하드코딩 대신 사용
10. **`update_selection` 헬퍼**: `set_selection` + `recompute_pending_modifiers` 결합 — command에서 `tr.set_selection()` 대신 사용
11. **`imbl` re-export**: `editor-model`에서 `pub use imbl;` — downstream crate에서 직접 의존 불필요
