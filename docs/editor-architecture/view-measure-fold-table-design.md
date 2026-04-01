# View — Fold & Table 측정 설계

`measure_inner`에 Fold와 Table 측정을 추가한다. 레거시 에디터의 레이아웃 로직을 새 Measurement/Fragment 아키텍처에 맞게 포팅.

## 범위

- MeasuredContent::Container에 `border: EdgeInsets`, `border_mode: BorderMode` 추가
- Paginator: border 지원 — Separate 모드의 페이지 split, Collapse 모드의 인접 border 겹침
- Fold: measure_fold, measure_fold_title, measure_fold_content
- Table: measure_table, measure_table_cell, table_width 유틸리티

Paragraph(TextBlock) 측정은 별도 사이클에서 다룬다.

---

## Container에 border 추가

### 동기

Table의 외곽 테두리는 padding으로 표현할 수 없다. 페이지 경계에서 Table이 분할될 때, 양쪽 페이지 모두에 테두리가 필요하다. Padding은 콘텐츠 내부 여백이고, border는 외곽 장식 공간이다. 페이지 split 시 동작이 다르므로 별도 필드로 분리한다.

또한 Table처럼 자식들이 각자 border를 가지는 경우, 인접한 border가 겹쳐져야 한다 (border collapse). 이를 `BorderMode`로 제어한다.

### 타입 변경

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderMode {
    #[default]
    Separate,  // border 독립 — 각 요소의 border가 겹치지 않음
    Collapse,  // 인접 자식의 border를 겹침 처리
}

pub enum MeasuredContent {
    Container {
        children: Vec<ChildMeasurement>,
        scope: bool,
        direction: LayoutDirection,
        padding: EdgeInsets,
        border: EdgeInsets,
        border_mode: BorderMode,
    },
    // ...
}
```

대부분의 Container에서 `border = EdgeInsets::ZERO, border_mode = Separate`.

### Box model

```
┌─ border ─────────────────────┐
│ ┌─ padding ────────────────┐ │
│ │                          │ │
│ │        content           │ │
│ │                          │ │
│ └──────────────────────────┘ │
└──────────────────────────────┘
```

Container의 size는 border + padding + content를 모두 포함:
```
size.width  = border.left + padding.left + content_width + padding.right + border.right
size.height = border.top + padding.top + content_height + padding.bottom + border.bottom
```

### BorderMode::Separate — Paginator 동작

Padding과 border의 차이:
- padding: 콘텐츠 여백. split 시 reopen에서 복원되지만 split 지점에 추가되지 않음.
- border: 외곽 장식. split 시 **양쪽 모두** 공간이 추가됨.

동작:
1. Container open: `current_y += border.top + padding.top`
2. `current_x()`: `margin_left + sum(border.left + padding.left)` (container stack 누적)
3. `break_page()` container close: `current_y += border.bottom` 후 Fragment 생성
4. Container reopen: `current_y += border.top + padding.top`
5. 정상 container close: `current_y += padding.bottom + border.bottom`

### BorderMode::Collapse — Paginator 동작

각 요소가 자신의 border를 full size로 선언하고, Paginator가 placement 시 인접 border를 겹침 처리한다.

**Collapse 규칙 (layout 방향):**
```
Vertical container:
  container.border.top ↔ first_child.border.top → overlap
  child_i.border.bottom ↔ child_{i+1}.border.top → overlap
  last_child.border.bottom ↔ container.border.bottom → overlap

Horizontal container:
  container.border.left ↔ first_child.border.left → overlap
  cell_i.border.right ↔ cell_{i+1}.border.left → overlap
  last_cell.border.right ↔ container.border.right → overlap
```

overlap = `min(border_a, border_b)` (균일 border에서는 항상 border_width).

**Cross-axis collapse:**
```
Vertical container: container.border.left/right ↔ children.border.left/right overlap
Horizontal container: container.border.top/bottom ↔ children.border.top/bottom overlap
```

**OpenContainer 변경:**
```rust
struct OpenContainer {
    // ... 기존 필드
    border: EdgeInsets,
    border_mode: BorderMode,
    last_child_border_end: f32,  // Collapse: 이전 자식의 끝 border (overlap 계산용)
}
```

**Vertical Collapse placement:**
```
Container open:
  current_y += border.top

Place first child:
  overlap = min(container.border.top, child.border.top)
  current_y -= overlap
  place child at current_y
  current_y += child.size.height

Place subsequent child:
  overlap = min(prev_child.border.bottom, child.border.top)
  current_y -= overlap
  place child at current_y
  current_y += child.size.height

Container close (normal 및 split 모두 동일):
  extra = max(container.border.bottom - last_child.border.bottom, 0)
  current_y += extra
```

**Horizontal Collapse (place_horizontal):**
동일한 규칙을 x축에 적용. `position_subtree` 대신 overlap-aware 배치.

**Cross-axis Collapse (current_x for Vertical container):**
```
current_x() 계산 시:
  collapsed parent가 있으면, 자식의 cross-axis border는 부모와 overlap
  effective_x += max(parent.border.left, child.border.left) (overlap, 합산 아님)
```

**Page split (Collapse 모드):**

child_i와 child_{i+1} 사이에서 split 발생 시:
- Page 1: close 공식 적용 → `extra = max(container.border.bottom - last_child.border.bottom, 0)`
  - uniform border: extra = 0 (child의 border.bottom이 충분)
  - outer > inner: extra > 0 (부족한 만큼 추가)
- Page 2: container reopen → border.top 적용 → first child와 다시 collapse.
- Normal close와 split close가 동일한 공식 → 특별 분기 없음.

```
break_page() in Collapse mode:
  extra = max(container.border.bottom - last_child.border.bottom, 0)
  current_y += extra

Container reopen in Collapse mode:
  current_y += border.top  // 일반 open과 동일
  // first child placement에서 overlap 차감
```

---

## Fold 측정

### 노드 구조 매핑

```
Fold → Container(Vertical, border = {all: 1.0}, border_mode = Separate)
├── FoldTitle → Container(Vertical, padding = {left:40, right:12, top:8, bottom:8})
│   └── children (Text 등)
└── FoldContent → Container(Vertical, padding = {left:24, right:24, top:16, bottom:16})
    └── children (블록들)
    (접힘 시: children에 포함하지 않음)
```

Fold는 단일 외곽 border, 자식에는 border 없음 → `Separate` 사용.

### 상수

| 이름 | 값 | 출처 |
|------|-----|------|
| FOLD_TITLE_PADDING_X | 12.0 | 레거시 TITLE_PADDING_X |
| FOLD_TITLE_PADDING_Y | 8.0 | 레거시 TITLE_PADDING_Y |
| FOLD_TITLE_ICON_WIDTH | 20.0 | 레거시 TOGGLE_ICON_WIDTH |
| FOLD_TITLE_ICON_GAP | 8.0 | 레거시 TOGGLE_ICON_PADDING |
| FOLD_CONTENT_PADDING_X | 24.0 | 레거시 CONTENT_PADDING_X |
| FOLD_CONTENT_PADDING_Y | 16.0 | 레거시 CONTENT_PADDING_Y |
| FOLD_BORDER_WIDTH | 1.0 | 레거시 FOLD_BORDER_WIDTH |

### measure_fold(node, width, view_state) → Measurement

1. FoldTitle 자식 노드를 `self.measure()`로 측정
2. `ViewState.fold_states`에서 expanded 여부 확인 (기본값: false)
3. expanded일 때: FoldContent 자식 노드를 `self.measure()`로 측정
4. collapsed일 때: FoldContent를 ChildMeasurement에 포함하지 않음
5. Fold 자체에 `resolve_gap_after()` 적용

반환:
```rust
Measurement {
    size: Size {
        width,
        height: FOLD_BORDER_WIDTH * 2.0 + title_height + content_height_if_expanded,
    },
    gap_after: resolve_gap_after(node),
    alignment: Alignment::Start,
    content: MeasuredContent::Container {
        children: [fold_title, fold_content?],
        scope: false,
        direction: LayoutDirection::Vertical,
        padding: EdgeInsets::ZERO,
        border: EdgeInsets::all(FOLD_BORDER_WIDTH),
        border_mode: BorderMode::Separate,
    },
}
```

### measure_fold_title(node, width) → Measurement

FoldTitle의 padding.left = `FOLD_TITLE_PADDING_X + FOLD_TITLE_ICON_WIDTH + FOLD_TITLE_ICON_GAP` = 40.0

아이콘은 Fragment로 생성하지 않음. 렌더러가 padding 영역에 아이콘을 그린다.

`measure_padded_container()` 재사용:
- padding = `EdgeInsets { top: 8, left: 40, bottom: 8, right: 12 }`
- border = `EdgeInsets::ZERO`
- scope = false
- direction = Vertical

### measure_fold_content(node, width) → Measurement

`measure_padded_container()` 재사용:
- padding = `EdgeInsets { top: 16, left: 24, bottom: 16, right: 24 }`
- border = `EdgeInsets::ZERO`
- scope = false
- direction = Vertical

### 접힘/펼침 동작

- `ViewState.fold_states: FxHashMap<NodeId, bool>` — true = expanded
- Fold의 NodeId로 조회, 없으면 collapsed (false)
- collapsed 시 Fold의 size.height = FOLD_BORDER_WIDTH * 2 + FoldTitle height
- `View.set_fold_state(node_id, expanded)` 호출 시 해당 Fold 및 조상 캐시 무효화 필요

---

## Table 측정

### 노드 구조 매핑

```
Table → Container(Vertical, border = {all: 1}, border_mode = Collapse)
├── TableRow₁ → Container(Horizontal, border = {all: 1}, border_mode = Collapse)
│   ├── TableCell₁ → Container(Vertical, scope=true, padding=8, border = {all: 1})
│   ├── TableCell₂ → Container(Vertical, scope=true, padding=8, border = {all: 1})
│   └── ...
├── TableRow₂ → ...
└── ...
```

모든 Table/Row/Cell이 자신의 border를 `{all: TABLE_BORDER_WIDTH}` 로 선언. Collapse에 의해 인접 border가 겹쳐져 최종 grid가 형성된다:
- (row_count + 1) 개의 수평 border 선
- (col_count + 1) 개의 수직 border 선

### 상수

| 이름 | 값 | 설명 |
|------|-----|------|
| TABLE_BORDER_WIDTH | 1.0 | 테두리 두께 |
| TABLE_CELL_PADDING | 8.0 | 셀 내부 패딩 (사방) |
| MIN_CELL_WIDTH | 40.0 | 열 최소 너비 |

### table_width 모듈 (순수 함수)

`table_width.rs` — struct 없이 순수 함수로 구성.

```rust
pub const TABLE_BORDER_WIDTH: f32 = 1.0;
pub const TABLE_CELL_PADDING: f32 = 8.0;
pub const MIN_CELL_WIDTH: f32 = 40.0;

/// 테두리 총 너비: (col_count + 1) * TABLE_BORDER_WIDTH
/// Collapse 후 기준: N열 → N+1개의 수직 border 선
pub fn border_width(col_count: usize) -> f32;

/// 테이블 최소 너비: col_count * MIN_CELL_WIDTH + border_width
pub fn min_table_width(col_count: usize) -> f32;

/// 비율 기반 열 너비 계산.
/// custom_widths: 첫 번째 Row의 각 Cell col_width (전부 Some이면 사용, 아니면 None)
/// available_width: border 제외한 사용 가능 너비
///
/// 알고리즘:
/// 1. custom_widths 없으면 균등 비율 (1/col_count)
/// 2. available_width <= col_count * MIN_CELL_WIDTH이면 전부 MIN_CELL_WIDTH
/// 3. 비율 오름차순 정렬, 작은 것부터 MIN_CELL_WIDTH 미달 여부 검사
/// 4. 미달 열은 MIN_CELL_WIDTH 고정, 나머지에 남은 너비 재분배
/// 5. 부동소수점 오차를 마지막 유효 열에 보정
pub fn calculate_col_widths(
    col_count: usize,
    custom_widths: Option<&[f32]>,
    available_width: f32,
) -> Vec<f32>;
```

### measure_table(node, width) → Measurement

**col_widths 계산:**
1. 자식 Row 노드 수집, col_count = 첫 번째 Row의 자식(Cell) 수
2. 빈 테이블 (row=0 또는 col=0): 높이 0 Measurement 반환
3. 첫 번째 Row의 각 Cell에서 `col_width: Option<f32>` 추출
   - 전부 Some → custom_widths로 사용
   - 하나라도 None → custom_widths = None (균등 분배)
4. `proportion.clamp(0.0, 1.0) * width` → target_width
5. `target_width.max(min_table_width().min(width))` → table_width
6. `table_width - border_width()` → inner_width (collapse 후 border 제외)
7. `calculate_col_widths(col_count, custom_widths, inner_width)` → col_widths

**각 Row 측정 (Table이 직접 구성):**
1. Row의 각 Cell에 대해 `self.measure(cell, col_widths[i])` 호출 (캐싱 경유)
   - Cell은 measure_inner → measure_table_cell로 dispatch
   - Cell size = border(1) + padding(8) + content + padding(8) + border(1) = content + 18
   - Cell height = border(1) + padding(8) + content_h + padding(8) + border(1) = content_h + 18
2. 1st pass: 각 Cell 측정 → 자연 높이 수집
3. 2nd pass: max_height 계산, 각 Cell Measurement의 size.height를 max_height로 조정
   - `Arc<Measurement>` 이므로 새 Measurement 생성하여 size.height만 교체
4. Row Measurement 구성:
   ```rust
   Measurement {
       size: Size {
           width: actual_table_width,  // collapse 후 총 너비
           height: max_cell_height,    // Cell full height (border 포함)
       },
       content: MeasuredContent::Container {
           children: [cell_measurements...],
           direction: LayoutDirection::Horizontal,
           border: EdgeInsets::all(TABLE_BORDER_WIDTH),
           border_mode: BorderMode::Collapse,
           ..
       },
   }
   ```

**Table Measurement 반환:**
```rust
Measurement {
    size: Size {
        width: actual_table_width,
        height: collapsed_total_height,
        // collapsed: TABLE_BORDER_WIDTH + sum(row_inner_heights) + (row_count - 1) * TABLE_BORDER_WIDTH + TABLE_BORDER_WIDTH
        // = (row_count + 1) * TABLE_BORDER_WIDTH + sum(row_inner_heights)
        // 여기서 row_inner_height = max_cell_height - 2 * TABLE_BORDER_WIDTH
    },
    gap_after: resolve_gap_after(node),
    alignment: match table.align {
        Left => Alignment::Start,
        Center => Alignment::Center,
        Right => Alignment::End,
    },
    content: MeasuredContent::Container {
        children: [row_measurements...],
        scope: false,
        direction: LayoutDirection::Vertical,
        padding: EdgeInsets::ZERO,
        border: EdgeInsets::all(TABLE_BORDER_WIDTH),
        border_mode: BorderMode::Collapse,
    },
}
```

**Size 계산 예시 (2행 3열, 각 셀 content 20px):**
```
Cell size: 1+8+20+8+1 = 38 (height), 1+8+content_w+8+1 (width)

Row (horizontal collapse):
  (col_count+1)*1 + sum(cell_inner_w) = 4 + sum(content_w+16)
  height = max_cell_height = 38

Table (vertical collapse):
  height = (row_count+1)*1 + sum(row_inner_h) = 3 + (38-2)*2 = 3 + 72 = 75
  width = (col_count+1)*1 + sum(col_widths+16) = 4 + sum(content_w+16)
```

### measure_table_cell(node, width) → Measurement

`measure_inner` dispatch를 통해 호출됨 (`self.measure` → cache → `measure_inner`).

```rust
Measurement {
    size: Size {
        width: TABLE_BORDER_WIDTH * 2.0 + TABLE_CELL_PADDING * 2.0 + content_width,
        height: TABLE_BORDER_WIDTH * 2.0 + TABLE_CELL_PADDING * 2.0 + content_height,
    },
    content: MeasuredContent::Container {
        padding: EdgeInsets::all(TABLE_CELL_PADDING),
        border: EdgeInsets::all(TABLE_BORDER_WIDTH),
        border_mode: BorderMode::Separate,  // Cell 자식은 border 없음
        scope: true,
        direction: LayoutDirection::Vertical,
        ..
    },
}
```

### TableRow — dispatch 없음

- Row는 col_widths 없이 단독 측정 불가
- 항상 Table의 자식이므로 measure_table이 직접 구성
- measure_inner dispatch에 case 추가하지 않음
- (만약 dispatch에 도달하면 measure_default_container fallthrough)

### TableAlign 처리

Table의 `alignment` 필드로 매핑:
- `TableAlign::Left` → `Alignment::Start`
- `TableAlign::Center` → `Alignment::Center`
- `TableAlign::Right` → `Alignment::End`

Paginator의 `child_x()`가 이미 alignment에 따라 cross-axis 위치를 계산하므로 추가 변경 불필요.

### Cell height 균등화 방식

Cell의 Measurement는 `Arc<Measurement>` 뒤에 있으므로 in-place 수정 불가. 2nd pass에서 max_height와 다른 Cell은 **새 Measurement를 생성**하여 size.height만 교체한다.

---

## measure_inner dispatch 변경

`engine/mod.rs`의 `measure_inner()`에 추가:

```rust
Node::Fold => measure_fold(self, node, width),
Node::FoldTitle => measure_fold_title(self, node, width),
Node::FoldContent => measure_fold_content(self, node, width),
Node::Table => measure_table(self, node, width),
Node::TableCell => measure_table_cell(self, node, width),
// TableRow: dispatch 없음 (measure_default_container fallthrough)
```

`measure_fold`는 `&self.view_state` 참조가 추가로 필요.

---

## 파일 구조

```
crates/editor-view/src/measure.rs           — BorderMode 추가, Container에 border/border_mode 필드
crates/editor-view/src/engine/measure_nodes/
  mod.rs             — re-exports 추가
  fold.rs            — measure_fold, measure_fold_title, measure_fold_content
  table.rs           — measure_table, measure_table_cell
  table_width.rs     — 상수 + border_width, min_table_width, calculate_col_widths
crates/editor-view/src/engine/paginator.rs  — OpenContainer에 border/border_mode, collapse 배치 로직
```

---

## Paginator 변경 요약

### OpenContainer 변경

```rust
struct OpenContainer {
    // ... 기존 필드
    border: EdgeInsets,
    border_mode: BorderMode,
    last_child_border_end: f32,  // Collapse: 이전 자식의 끝 border
    is_first_child: bool,        // Collapse: 첫 자식 여부
}
```

### Separate 모드

기존 padding과 유사하되, split 시 양쪽에 border 추가:
1. Container open: `current_y += border.top + padding.top`
2. `break_page()` close: `current_y += border.bottom`
3. Container reopen: `current_y += border.top + padding.top`
4. 정상 close: `current_y += padding.bottom + border.bottom`
5. `current_x()`: `border.left + padding.left` 누적

### Collapse 모드

1. Container open: `current_y += border.top`
2. First child placement: `current_y -= min(container.border.top, child.border.top)`
3. Subsequent child: `current_y -= min(prev.border.bottom, child.border.top)`
4. Close (normal 및 split 동일): `extra = max(container.border.bottom - last_child.border.bottom, 0)`, `current_y += extra`
5. Container reopen: `current_y += border.top` → first child와 다시 collapse
7. `current_x()`: `max(container.border.left, child.border.left)` (overlap, 합산 아님)

### Horizontal Collapse (place_horizontal)

동일한 규칙을 x축에 적용:
- 인접 Cell의 border.right ↔ border.left overlap
- Container border.left ↔ first child border.left overlap
- Container border.right ↔ last child border.right overlap

### border = ZERO인 Container

기존과 동일하게 동작. 추가 연산 없음.
