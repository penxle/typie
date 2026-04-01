# Command System Redesign

editor-commands crate를 제네릭하고 합성 가능한 커맨드 시스템으로 재설계한다.

## 배경

현재 `crates/editor/src/transaction/` 모듈에 127개 메서드가 혼재되어 있다. 이를 3계층으로 분리한다:

| 계층 | 위치 | 역할 |
|------|------|------|
| Transaction Core API | `editor-transaction` | commit, set_selection 등 트랜잭션 수명주기 (구현 완료) |
| Command Helper | `editor-commands/src/helpers/` | 여러 커맨드가 공유하는 Step 기반 저수준 유틸리티 |
| Command | `editor-commands/src/commands/` | 제네릭한 고수준 편집 오퍼레이션 |

## 설계 원칙

1. **Parameterize, don't specialize** — `toggle_bullet_list` + `toggle_ordered_list` → `toggle_list(tr, list_type)`. 노드 타입이나 프로퍼티에 특화된 커맨드를 만들지 않는다.
2. **Command = 단일 책임** — 각 커맨드는 한 가지 케이스만 처리하고, 적용 불가 시 `Ok(false)`를 반환한다.
3. **Composition at handler** — handler 계층에서 `first()`/`chain()`으로 컨텍스트별 동작을 합성한다.
4. **Enum으로 변종 통합** — `Axis::Row`/`Column`, `Direction::Backward`/`Forward` 등으로 파라미터화한다.
5. **Transaction Step 직접 호출** — 커맨드는 Transaction Step API를 직접 사용한다. Helper는 여러 커맨드가 공유하는 로직이 있을 때만 사용한다.

## Crate 구조

```
editor-commands/src/
  lib.rs              # pub mod commands, helpers, compose, error
  commands/
    mod.rs            # 모든 커맨드 pub mod + re-export
    insert_text.rs    # 커맨드 당 1파일
    split_block.rs
    ...
  helpers/
    mod.rs            # 모든 helper pub mod + re-export
    split_block_at.rs # helper 당 1파일
    ...
  compose.rs          # first(), chain(), can(), optional()
  error.rs            # CommandError, CommandResult
  test_utils.rs       # 테스트 인프라
```

## Command 시그니처

```rust
// 파라미터 없는 커맨드
pub fn split_block(tr: &mut Transaction) -> CommandResult { ... }

// 파라미터 있는 커맨드
pub fn insert_text(tr: &mut Transaction, text: &str) -> CommandResult { ... }
pub fn toggle_list(tr: &mut Transaction, list_type: ListType) -> CommandResult { ... }
```

- 반환: `CommandResult = Result<bool, CommandError>`
- `true` = 변경 발생, `false` = no-op (적용 불가)

## Helper 시그니처

```rust
// 반환 타입 자유
pub fn split_block_at(tr: &mut Transaction, pos: Position) -> Result<Option<(NodeId, Position)>> { ... }
pub fn word_range_at(doc: &Doc, position: Position) -> Option<(usize, usize)> { ... }
```

## 합성 패턴

Handler에서 `first()`/`chain()`으로 커맨드를 합성한다:

```rust
// Enter 키 처리
fn handle_enter(tr: &mut Transaction) -> CommandResult {
    first(tr, &[
        &|tr| chain(tr, &[&|tr| can_lift_empty(tr), &|tr| lift(tr)]),
        &|tr| split_block(tr),
    ])
}

// Backspace 키 처리
fn handle_delete_backward(tr: &mut Transaction) -> CommandResult {
    first(tr, &[
        &|tr| join_backward(tr),
        &|tr| delete_backward(tr),
    ])
}

// 특화 없이 파라미터로 처리
fn handle_toggle_bold(tr: &mut Transaction) -> CommandResult {
    toggle_style(tr, Style::Bold)
}

fn handle_set_text_align(tr: &mut Transaction, align: TextAlign) -> CommandResult {
    set_block_attr(tr, BlockAttr::TextAlign(align))
}
```

## 커맨드 목록

기존 77개 특화 커맨드를 ~35개 제네릭 커맨드로 재설계한다. 세부 시그니처는 구현 시 확정한다.

### Text (5)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `insert_text` | `text: &str` | 텍스트 삽입 (선택 영역 삭제 포함) |
| `delete_backward` | — | 뒤로 1 grapheme 삭제 |
| `delete_forward` | — | 앞으로 1 grapheme 삭제 |
| `delete_selection` | — | 선택 영역 삭제 |
| `surround_selection` | `left: &str, right: &str` | 선택 영역 감싸기 |

### Block Structure (9)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `split_block` | — | 현재 블록 분할 (컨텍스트 인식) |
| `join_backward` | — | 이전 블록과 합치기 (컨텍스트 인식) |
| `join_forward` | — | 다음 블록과 합치기 |
| `insert_node` | `node: Node` | 임의 노드 삽입 |
| `wrap` | `wrapper: Node` | 선택 영역 감싸기 |
| `lift` | — | 즉시 부모에서 꺼내기 |
| `lift_from` | `predicate: F` | 매칭 조상에서 꺼내기 |
| `toggle_wrap` | `predicate: F, wrapper: Node` | 단순 wrapper 토글 (blockquote, callout) |
| `toggle_list` | `list_type: ListType` | 리스트 토글 (list item 처리 포함) |

### Selection (3)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `select_unit_at` | `pos: Position, unit: TextUnit` | Word/Sentence/Paragraph 선택 |
| `collapse_selection` | — | 선택 영역 접기 |
| `expand_selection_until` | `predicate: F` | 조건까지 확장 |

### Inline Style (3)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `set_style` | `style: Style` | 인라인 스타일 설정 |
| `toggle_style` | `style: Style` | 인라인 스타일 토글 |
| `reset_styles` | — | 모든 인라인 스타일 초기화 |

### Block Attribute (2)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `set_block_attr` | `attr: BlockAttr` | 블록 속성 설정 (TextAlign, LineHeight 등) |
| `reset_block` | — | 블록 기본값 복원 |

### Table (7)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `insert_table` | `rows: u32, cols: u32` | 테이블 삽입 |
| `table_add_lane` | `table_id, axis: Axis, index, before` | 행/열 추가 |
| `table_delete_lane` | `table_id, axis: Axis, index` | 행/열 삭제 |
| `table_move_lane` | `table_id, axis: Axis, from, to` | 행/열 이동 |
| `table_select` | `table_id, target: TableTarget` | 테이블/행/열 선택 |
| `set_table_attr` | `table_id, attr: TableAttr` | 테이블 속성 설정 |
| `delete_structure_selection` | `info: &StructureSelectionInfo` | 구조 선택 삭제 |

### Clipboard / Drop (2)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `paste` | `content: PasteContent` | 통합 붙여넣기 |
| `drop_content` | `target, content, mode: DropMode` | 통합 드롭 |

### IME (2)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `set_preedit` | `text: String` | IME 조합 텍스트 설정 |
| `commit_preedit` | — | IME 조합 확정 |

### Document (1)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `set_document_attr` | `attr: DocumentAttr` | 문서 속성 설정 |

### Annotation (2)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `set_annotation` | `annotation: Annotation` | 주석 추가/수정 |
| `remove_annotation` | `ann_type: AnnotationType` | 주석 제거 |

### Remark (3)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `add_remark` | `node_id, remark` | 리마크 추가 |
| `update_remark` | `node_id, remark_id, text` | 리마크 수정 |
| `remove_remark` | `node_id, remark_id` | 리마크 제거 |

### Text Replacement (2)

| 커맨드 | 파라미터 | 역할 |
|--------|---------|------|
| `try_text_replacement` | `input_byte_len: usize` | 자동 텍스트 치환 |
| `try_undo_text_replacement` | `undo: &ReplacementUndoState` | 치환 취소 |

## 파라미터 Enum 타입

```rust
enum ListType { Bullet, Ordered }
enum TextUnit { Word, Sentence, Paragraph }
enum Axis { Row, Column }
enum TableTarget { Table, Row(usize), Column(usize) }
enum DropMode { Move, Copy, External }
enum PasteContent { Text(String), Fragment(Fragment), Mixed(Fragment, String) }
enum BlockAttr { TextAlign(TextAlign), LineHeight(u32) }
enum TableAttr { Border(String), Align(TableAlign), Width(f32, f32), Proportion(f32), ColumnWidths(Vec<f32>) }
enum DocumentAttr { Settings(DocumentSettings), LayoutMode(LayoutMode), BlockGap(u32), ParagraphIndent(u32), DefaultAttrs(DefaultAttrs) }
```

## Helper 목록

여러 커맨드가 공유하는 Step 기반 저수준 유틸리티. 구현 시 필요에 따라 추가/조정한다.

| Helper | 역할 |
|--------|------|
| `split_block_at` | 지정 위치에서 블록 분할 |
| `clone_block_type` | 블록 타입 복제 |
| `delete_range` | 범위 삭제 |
| `replace_range` | 범위 교체 |
| `move_node_range` | 노드 범위 이동 |
| `delete_node_recursive` | 노드 재귀 삭제 |
| `delete_text_range` | 텍스트 범위 삭제 |
| `merge_blocks_content` | 블록 내용 병합 |
| `clean_up_empty_ancestors` | 빈 조상 정리 |
| `insert_fragment` | 프래그먼트 삽입 |
| `delete_node_with_selection_adjustment` | 선택 조정 포함 노드 삭제 |
| `merge_adjacent_text_nodes` | 인접 텍스트 노드 병합 |
| `try_lift_block` | 블록 lift 시도 |
| `delete_selection_with_merge` | 선택 삭제 + 병합 (DeleteResult 반환) |
| `remap_position` | 위치 재매핑 |
| `is_ancestor_of` | 조상 여부 확인 |
| `word_range_at` | 단어 범위 계산 |
| `sentence_range_at` | 문장 범위 계산 |
| `paragraph_range_at` | 문단 범위 계산 |
| `move_to_next_block` | 다음 블록으로 커서 이동 |
| `compute_styles_at_cursor` | 커서 위치 스타일 계산 |
| `compute_styles_at_char_position` | 문자 위치 스타일 계산 |
| `resolve_style_cascade` | 스타일 cascade 해석 |
| `resolved_font` | 해석된 폰트 조회 |
| `recompute_pending_styles` | 대기 스타일 재계산 |
| `can_drop` | 드롭 가능 여부 |
| `relocate_selection` | 선택 영역 재배치 |

## 기존 커맨드 → 새 커맨드 매핑 (주요 예시)

| 기존 (특화) | 새 (제네릭) |
|-------------|------------|
| `toggle_bullet_list()` | `toggle_list(tr, ListType::Bullet)` |
| `toggle_ordered_list()` | `toggle_list(tr, ListType::Ordered)` |
| `toggle_bold_style()` | `toggle_style(tr, Style::Bold)` |
| `toggle_blockquote(v)` | `toggle_wrap(tr, is_blockquote, blockquote(v))` |
| `toggle_callout()` | `toggle_wrap(tr, is_callout, callout())` |
| `select_word_at(pos)` | `select_unit_at(tr, pos, TextUnit::Word)` |
| `set_text_align(a)` | `set_block_attr(tr, BlockAttr::TextAlign(a))` |
| `set_line_height(h)` | `set_block_attr(tr, BlockAttr::LineHeight(h))` |
| `insert_horizontal_rule(v)` | `insert_node(tr, Node::horizontal_rule(v))` |
| `insert_fold()` | `insert_node(tr, Node::fold())` |
| `paste_text(s)` | `paste(tr, PasteContent::Text(s))` |
| `drag_and_drop(t)` | `drop_content(tr, t, content, DropMode::Move)` |
| `set_document_settings(s)` | `set_document_attr(tr, DocumentAttr::Settings(s))` |
| `split_paragraph()` / `split_list_item()` | `split_block(tr)` (컨텍스트 인식) |
| `merge_list_item_backward()` / `join_backward()` | `join_backward(tr)` (컨텍스트 인식) |
| `add_table_row(id, r, b)` | `table_add_lane(tr, id, Axis::Row, r, b)` |
| `add_table_column(id, c, b)` | `table_add_lane(tr, id, Axis::Column, c, b)` |

## 범위

- **이번 작업**: `editor-commands` crate에 commands + helpers 구현
- **별도 작업**: `runtime/handlers/` 리팩토링 (커맨드 호출로 전환)
- **세부 시그니처**: 구현 시 확정 — 이 문서는 방향과 구조만 정의
