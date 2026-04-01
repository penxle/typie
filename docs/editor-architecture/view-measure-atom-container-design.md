# measure_inner 구현: Atom + Container 측정

view-remaining-work.md의 1단계 중 Atom 노드 측정, 기본 Container 노드 측정(padding/indent), gap_after 계산을 다룬다. Paragraph(TextBlock), Fold, Table은 별도 sub-project.

## 스코프

**포함:**
- Atom 측정: Image, File, Embed, Archived, HorizontalRule
- Container 측정: ListItem, Blockquote (4개 variant), Callout
- gap_after: Modifier::BlockGap cascading resolve
- MeasuredContent::Container에 padding 추가
- Measurement에 alignment 추가
- Paginator에서 padding, alignment 반영

**제외:**
- Paragraph → TextBlock (텍스트 셰이핑, parley 연동)
- Fold (ViewState.fold_expanded 연동)
- Table (열 너비 계산, Horizontal Row)
- 커서 Movement 확장
- editor-core 연동

## 타입 변경

### Alignment (신규)

```rust
pub enum Alignment {
    Start,   // Vertical 부모 → Left, Horizontal 부모 → Top
    Center,
    End,     // Vertical 부모 → Right, Horizontal 부모 → Bottom
}
```

부모 Container의 direction에 따라 cross-axis 정렬로 해석한다.

### EdgeInsets (신규)

```rust
pub struct EdgeInsets {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
}
```

### Measurement (변경)

```rust
pub struct Measurement {
    pub size: Size,
    pub gap_after: f32,
    pub content: MeasuredContent,
    pub alignment: Alignment,  // 추가. 기본값 Start.
}
```

### MeasuredContent::Container (변경)

```rust
Container {
    children: Vec<ChildMeasurement>,
    scope: bool,
    direction: LayoutDirection,
    padding: EdgeInsets,  // 추가
}
```

## Atom 측정

`measure_atom(node_ref, width, view_state) -> Measurement`

자식 없음. `MeasuredContent::Atom { parent_id, index }`를 반환한다. `parent_id`와 `index`는 `node_ref.parent().id()`와 `node_ref.index()`로 조회.

| 노드 | 너비 | 높이 |
|------|------|------|
| Image | `ImageNode.proportion * width` | `view_state.external_height(id)` 또는 `0.0` |
| File | `width` | `view_state.external_height(id)` 또는 `0.0` |
| Embed | `width` | `view_state.external_height(id)` 또는 `0.0` |
| Archived | `width` | `view_state.external_height(id)` 또는 `0.0` |
| HorizontalRule | `width` | `24.0` 고정 |

모든 Atom의 `alignment = Start`.

## Container 측정

공통 패턴:

1. padding 결정 (노드별 상수)
2. `content_width = width - padding.left - padding.right`
3. children을 `content_width`로 재귀 측정
4. `height = padding.top + sum(children heights + gap_after) + padding.bottom` (마지막 child의 gap_after 제외)
5. `gap_after = resolve_gap_after(node_ref)`
6. `alignment` 결정

### BulletList / OrderedList

- padding: `EdgeInsets::ZERO`
- 기존 stub과 동일한 Vertical Container. gap_after만 추가.

### ListItem

- `padding.left = 28.0` (MARKER_WIDTH 20.0 + MARKER_GAP 8.0)
- children은 `width - 28.0`으로 측정

### Blockquote

variant별 분기. `BlockquoteNode.variant`로 판별.

| Variant | padding | 너비 | alignment |
|---------|---------|------|-----------|
| LeftLine | left: `20.0` | `width` | Start |
| LeftQuote | left: `32.0` | `width` | Start |
| MessageSent | left/right: `14.0`, top/bottom: `8.0` | `(width * 0.8).max(40.0).min(width)` | **End** |
| MessageReceived | left/right: `14.0`, top/bottom: `8.0` | `(width * 0.8).max(40.0).min(width)` | Start |

Blockquote 상수 (레거시에서 포팅):
- `LINE_WIDTH = 4.0`, `CONTENT_PADDING = 16.0` → LeftLine left = 20.0
- `QUOTE_SIZE = 16.0`, `QUOTE_CONTENT_GAP = 16.0` → LeftQuote left = 32.0
- `MESSAGE_PADDING_X = 14.0`, `MESSAGE_PADDING_Y = 8.0`
- `MESSAGE_MAX_WIDTH_RATIO = 0.8`, `MESSAGE_MIN_WIDTH = 40.0`

### Callout

- `padding = { top: 16.0, left: 40.0, bottom: 16.0, right: 12.0 }`
- left = PADDING_X(12.0) + ICON_WIDTH(20.0) + ICON_CONTENT_GAP(8.0)
- right = PADDING_X(12.0)
- top/bottom = PADDING_Y(16.0)

### 기본 Container (Root, Paragraph, Text 등)

현재 stub 로직 유지. padding = `EdgeInsets::ZERO`, alignment = `Start`. gap_after만 resolve 추가.

## gap_after 계산

### Cascading Inheritance Resolve

```rust
fn resolve_inherited<'a>(
    node_ref: &NodeRef<'a>,
    modifier_type: ModifierType,
) -> Option<&'a Modifier>
```

자신의 modifiers를 먼저 확인하고, 없으면 parent 체인을 올라간다. Root까지 올라가도 없으면 `None`.

`ModifierType`이 없으면 `Modifier`의 discriminant에 대응하는 enum으로 추가한다.

### BlockGap 변환

```rust
fn resolve_gap_after(node_ref: &NodeRef<'_>) -> f32 {
    match resolve_inherited(node_ref, ModifierType::BlockGap) {
        Some(Modifier::BlockGap(v)) => *v as f32 / 100.0 * 16.0,
        _ => 0.0,
    }
}
```

BlockGap modifier가 조상 체인 어디에도 없으면 `gap_after = 0.0`.

적용 대상: 모든 block-level 노드. Text(인라인), Root(최상위), PageBreak에는 적용하지 않음.

## Paginator 수정

### EdgeInsets 반영

`OpenContainer`에 padding 정보 추가:

```rust
struct OpenContainer {
    // 기존 필드...
    padding: EdgeInsets,  // 추가
}
```

- `open_container()` 시 padding 저장
- children의 x 시작점을 `padding.left`, y 시작점을 `padding.top`으로 offset
- `close_container()` 시 height에 `padding.bottom` 추가

### Alignment 반영

Vertical container에서 자식 배치 시:

```rust
let x = match child_measurement.alignment {
    Alignment::Start => padding.left,
    Alignment::Center => (container_width - child_width) / 2.0,
    Alignment::End => container_width - child_width,
};
```

Horizontal container에서 자식 배치 시 (Table sub-project에서 구현 예정):

```rust
let y = match child_measurement.alignment {
    Alignment::Start => padding.top,
    Alignment::Center => (container_height - child_height) / 2.0,
    Alignment::End => container_height - child_height,
};
```

Horizontal container의 vertical alignment는 최종 row 높이를 먼저 확정한 뒤 y를 조정하는 2-pass가 필요하다. 현재 스코프에선 Horizontal container(TableRow)가 제외되므로 이 로직은 구현하지 않는다.

## measure_inner dispatch

```rust
fn measure_inner(&mut self, doc, node_ref, width, view_state) -> Measurement {
    match node_ref.node() {
        // Atom
        Node::Image(_) | Node::File(_) | Node::Embed(_)
        | Node::Archived(_) | Node::HorizontalRule(_) => {
            measure_atom(node_ref, width, view_state)
        }

        // Container + padding
        Node::ListItem(_) => measure_list_item(self, doc, node_ref, width, view_state),
        Node::Blockquote(_) => measure_blockquote(self, doc, node_ref, width, view_state),
        Node::Callout(_) => measure_callout(self, doc, node_ref, width, view_state),

        // PageBreak
        Node::PageBreak(_) => Measurement {
            size: Size { width, height: 0.0 },
            gap_after: 0.0,
            content: MeasuredContent::PageBreak,
            alignment: Alignment::Start,
        },

        // 기본: Vertical Container
        _ => measure_default_container(self, doc, node_ref, width, view_state),
    }
}
```

## 레거시 상수 출처

| 상수 | 값 | 레거시 파일 |
|------|-----|-----------|
| HR DEFAULT_HEIGHT | 24.0 | model/nodes/horizontal_rule.rs:9 |
| LIST_ITEM_MARKER_WIDTH | 20.0 | model/nodes/list_item.rs:10 |
| LIST_ITEM_MARKER_GAP | 8.0 | model/nodes/list_item.rs:11 |
| BQ LINE_WIDTH | 4.0 | model/nodes/blockquote.rs:17 |
| BQ CONTENT_PADDING | 16.0 | model/nodes/blockquote.rs:18 |
| BQ QUOTE_SIZE | 16.0 | model/nodes/blockquote.rs:20 |
| BQ QUOTE_CONTENT_GAP | 16.0 | model/nodes/blockquote.rs:21 |
| BQ MESSAGE_PADDING_X | 14.0 | model/nodes/blockquote.rs:23 |
| BQ MESSAGE_PADDING_Y | 8.0 | model/nodes/blockquote.rs:24 |
| BQ MESSAGE_MAX_WIDTH_RATIO | 0.8 | model/nodes/blockquote.rs:25 |
| BQ MESSAGE_MIN_WIDTH | 40.0 | model/nodes/blockquote.rs:26 |
| CALLOUT PADDING_X | 12.0 | model/nodes/callout.rs:13 |
| CALLOUT PADDING_Y | 16.0 | model/nodes/callout.rs:14 |
| CALLOUT ICON_WIDTH | 20.0 | model/nodes/callout.rs:10 |
| CALLOUT ICON_CONTENT_GAP | 8.0 | model/nodes/callout.rs:12 |
| BlockGap 변환 base | 16.0 | model/nodes/root.rs:45 |
