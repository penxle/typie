# Fold & Table 측정 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** editor-view의 measure_inner에 Fold와 Table 측정을 추가하고, Paginator에 border/border collapse 지원을 구현한다.

**Architecture:** MeasuredContent::Container에 border와 BorderMode를 추가하여 Fold(Separate)와 Table(Collapse) border를 처리. measure_padded_container를 border 인자를 받도록 확장. Paginator는 BorderMode에 따라 Separate(split 양쪽 border 추가)와 Collapse(인접 border overlap) 배치를 수행.

**Tech Stack:** Rust, editor-view crate, editor-model (Doc, NodeRef, Node types), editor-common (EdgeInsets, Alignment, Size)

**Spec:** `docs/editor-architecture/view-measure-fold-table-design.md`

---

## File Structure

```
수정:
  crates/editor-view/src/measure.rs                     — BorderMode enum, Container에 border/border_mode 필드 추가
  crates/editor-view/src/engine/measure_nodes/mod.rs    — fold, table, table_width re-exports
  crates/editor-view/src/engine/measure_nodes/container.rs — measure_padded_container에 border 파라미터 추가
  crates/editor-view/src/engine/paginator.rs            — OpenContainer에 border/border_mode, Separate/Collapse 배치
  crates/editor-view/src/engine/mod.rs                  — measure_inner dispatch에 Fold/Table case 추가

생성:
  crates/editor-view/src/engine/measure_nodes/fold.rs        — measure_fold, measure_fold_title, measure_fold_content
  crates/editor-view/src/engine/measure_nodes/table.rs       — measure_table, measure_table_cell
  crates/editor-view/src/engine/measure_nodes/table_width.rs — 상수, border_width, min_table_width, calculate_col_widths
```

---

### Task 1: BorderMode enum 및 Container 필드 추가

**Files:**
- Modify: `crates/editor-view/src/measure.rs`

- [ ] **Step 1: measure.rs에 BorderMode enum 추가 및 Container 필드 확장**

`measure.rs` 상단에 `BorderMode` enum을 추가하고, `MeasuredContent::Container`에 `border`와 `border_mode` 필드를 추가한다.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderMode {
    #[default]
    Separate,
    Collapse,
}
```

`MeasuredContent::Container`를 다음으로 변경:
```rust
Container {
    children: Vec<ChildMeasurement>,
    scope: bool,
    direction: LayoutDirection,
    padding: EdgeInsets,
    border: EdgeInsets,
    border_mode: BorderMode,
},
```

- [ ] **Step 2: Container를 생성하는 모든 기존 코드에 `border: EdgeInsets::ZERO, border_mode: BorderMode::Separate` 추가**

`MeasuredContent::Container`를 생성하는 모든 위치를 수정한다. 검색 방법: `MeasuredContent::Container {`로 grep.

수정 대상 파일들:
- `crates/editor-view/src/engine/measure_nodes/container.rs` — `measure_padded_container`, `measure_default_container`
- `crates/editor-view/src/engine/paginator.rs` — `place` (빈 vertical container), `position_subtree`
- `crates/editor-view/src/engine/paginator.rs` 테스트의 helper 함수 `container_m`

모든 위치에 `border: EdgeInsets::ZERO, border_mode: BorderMode::Separate` 추가.

- [ ] **Step 3: Paginator의 `place()` 패턴 매칭에 새 필드 추가**

`paginator.rs`의 `place()` 메서드에서 `MeasuredContent::Container` 패턴 매칭에 `border`, `border_mode` 필드를 추가 (현재는 `_`로 무시):

```rust
MeasuredContent::Container {
    children,
    scope,
    direction,
    padding,
    border: _,
    border_mode: _,
} => ...
```

`position_subtree()`에서도 동일하게 수정.

- [ ] **Step 4: 컴파일 확인**

Run: `cargo build -p editor-view`
Expected: 컴파일 성공. 기존 동작 변경 없음.

- [ ] **Step 5: 기존 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모든 기존 테스트 통과.

---

### Task 2: measure_padded_container에 border/scope 파라미터 추가

**Files:**
- Modify: `crates/editor-view/src/engine/measure_nodes/container.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/blockquote.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/callout.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/list_item.rs`

- [ ] **Step 1: `measure_padded_container` 시그니처 확장**

`container.rs`의 `measure_padded_container`에 `border: EdgeInsets`와 `scope: bool` 파라미터를 추가한다:

```rust
pub(super) fn measure_padded_container(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
    padding: EdgeInsets,
    border: EdgeInsets,
    scope: bool,
    alignment: Alignment,
) -> Measurement {
    let content_width = width - padding.left - padding.right - border.left - border.right;

    let children: Vec<ChildMeasurement> = node
        .children()
        .map(|child| {
            let m = engine.measure(doc, child.id(), content_width, view_state);
            ChildMeasurement {
                node_id: child.id(),
                measurement: m,
            }
        })
        .collect();

    let children_height: f32 = children.iter().map(|c| c.measurement.size.height).sum();
    let height = border.top + padding.top + children_height + padding.bottom + border.bottom;

    Measurement {
        size: Size { width, height },
        gap_after: resolve_gap_after(node),
        content: MeasuredContent::Container {
            children,
            scope,
            direction: LayoutDirection::Vertical,
            padding,
            border,
            border_mode: BorderMode::Separate,
        },
        alignment,
    }
}
```

- [ ] **Step 2: 기존 호출부 수정**

`blockquote.rs`, `callout.rs`, `list_item.rs`의 `measure_padded_container` 호출에 `border: EdgeInsets::ZERO, scope: false` 추가.

예시 (`blockquote.rs`):
```rust
measure_padded_container(engine, doc, node, width, view_state, padding, EdgeInsets::ZERO, false, alignment)
```

동일하게 `callout.rs`, `list_item.rs`도 수정.

- [ ] **Step 3: 컴파일 및 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 통과. 기존 동작 변경 없음.

---

### Task 3: Paginator — Separate 모드 border 지원

**Files:**
- Modify: `crates/editor-view/src/engine/paginator.rs`

- [ ] **Step 1: 기존 Paginator border 미지원 상태에서 Separate border 동작 테스트 작성**

Paginator 테스트에 Separate border가 적용된 Container를 배치하는 테스트를 추가한다:

```rust
#[test]
fn separate_border_adds_space() {
    let mut p = Paginator::new_continuous(200.0, 10000.0, 0.0, 0.0, 0.0);

    let child = ChildMeasurement {
        node_id: NodeId::new(),
        measurement: leaf_container_m(20.0),
    };

    let m = Arc::new(Measurement {
        size: Size { width: 200.0, height: 30.0 },  // 5 + 20 + 5
        gap_after: 0.0,
        alignment: Alignment::Start,
        content: MeasuredContent::Container {
            children: vec![child],
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets { top: 5.0, left: 0.0, bottom: 5.0, right: 0.0 },
            border_mode: BorderMode::Separate,
        },
    });

    let node_id = NodeId::new();
    p.place(node_id, &m);
    let pages = p.finish();

    let frag = &pages[0].fragments[0];
    let Fragment::Container(c) = frag else { panic!() };
    assert_eq!(c.rect.height, 30.0);

    // child는 border.top(5) 이후에 위치
    let Fragment::Container(child_c) = &c.children[0] else { panic!() };
    assert_eq!(child_c.rect.y, 5.0);
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p editor-view -- separate_border_adds_space`
Expected: FAIL — border가 아직 반영되지 않아 child.y가 0.0.

- [ ] **Step 3: OpenContainer에 border/border_mode 필드 추가 및 Separate 모드 구현**

`OpenContainer`에 새 필드 추가:

```rust
struct OpenContainer {
    node_id: NodeId,
    scope: bool,
    start_y: f32,
    width: f32,
    children: Vec<Fragment>,
    pending_gap: f32,
    split_top: bool,
    padding: EdgeInsets,
    border: EdgeInsets,
    border_mode: BorderMode,
}
```

`open_container` 시그니처에 `border: EdgeInsets, border_mode: BorderMode` 파라미터 추가:

```rust
fn open_container(&mut self, node_id: NodeId, scope: bool, padding: EdgeInsets, border: EdgeInsets, border_mode: BorderMode) {
    self.current_y += border.top + padding.top;
    self.container_stack.push(OpenContainer {
        node_id,
        scope,
        start_y: self.current_y - padding.top - border.top,
        width: self.width,
        children: vec![],
        pending_gap: 0.0,
        split_top: false,
        padding,
        border,
        border_mode,
    });
}
```

`close_container`에서 border.bottom 추가:

```rust
fn close_container(&mut self) {
    let c = self.container_stack.pop().expect("close without open");
    self.current_y += c.padding.bottom + c.border.bottom;
    // ... 나머지 기존 로직
}
```

`current_x()`에서 border.left 누적:

```rust
fn current_x(&self) -> f32 {
    self.margin_left
        + self
            .container_stack
            .iter()
            .map(|c| c.border.left + c.padding.left)
            .sum::<f32>()
}
```

`child_x()`에서 border도 감안:

```rust
fn child_x(&self, child_measurement: &Measurement) -> f32 {
    let base_x = self.current_x();
    let container_content_width = self
        .container_stack
        .last()
        .map_or(self.width, |c| c.width - c.padding.left - c.padding.right - c.border.left - c.border.right);
    // ... 나머지 동일
}
```

`place()` 메서드에서 `open_container` 호출 시 border/border_mode 전달:

```rust
LayoutDirection::Vertical => {
    self.open_container(node_id, *scope, *padding, *border, *border_mode);
    // ...
}
```

`TextBlock` 배치에서도 `open_container` 호출 수정:
```rust
MeasuredContent::TextBlock { lines } => {
    self.open_container(node_id, false, EdgeInsets::ZERO, EdgeInsets::ZERO, BorderMode::Separate);
    // ...
}
```

`break_page()`에서 reopens에 border/border_mode 포함:

```rust
fn break_page(&mut self) {
    let mut reopens: Vec<(NodeId, bool, f32, EdgeInsets, EdgeInsets, BorderMode)> = vec![];

    while let Some(c) = self.container_stack.pop() {
        reopens.push((c.node_id, c.scope, c.width, c.padding, c.border, c.border_mode));
        if c.children.is_empty() {
            continue;
        }

        // Separate 모드: close 시 border.bottom 추가
        if c.border_mode == BorderMode::Separate {
            self.current_y += c.border.bottom;
        }

        let fragment = Fragment::Container(ContainerFragment {
            node_id: c.node_id,
            rect: Rect {
                x: self.current_x(),
                y: c.start_y,
                width: c.width,
                height: self.container_height_on_break(c.start_y),
            },
            children: c.children,
            scope: c.scope,
            breaks: self.container_breaks_on_break(c.split_top),
        });

        self.add_to_current(fragment);
    }

    let frags = std::mem::take(&mut self.page_fragments);
    if !frags.is_empty() {
        self.pages.push(Page::new(frags, self.page_height_on_break()));
    }

    self.current_y = self.content_top();

    for (node_id, scope, width, padding, border, border_mode) in reopens.into_iter().rev() {
        self.current_y += border.top + padding.top;
        self.container_stack.push(OpenContainer {
            node_id,
            scope,
            start_y: self.current_y - padding.top - border.top,
            width,
            children: vec![],
            pending_gap: 0.0,
            split_top: true,
            padding,
            border,
            border_mode,
        });
    }
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: `separate_border_adds_space` 통과, 기존 테스트도 모두 통과.

- [ ] **Step 5: Separate border page split 테스트 작성 및 통과 확인**

```rust
#[test]
fn separate_border_split_adds_border_both_pages() {
    // page_height=100, margin_top=10, margin_bottom=10 → content_height=80
    let mut p = Paginator::new_paginated(200.0, 100.0, 10.0, 10.0, 0.0);

    let child1 = ChildMeasurement {
        node_id: NodeId::new(),
        measurement: leaf_container_m(60.0),
    };
    let child2 = ChildMeasurement {
        node_id: NodeId::new(),
        measurement: leaf_container_m(60.0),
    };

    // border 5+5 + children 60+60 = 130 → split 발생
    let m = Arc::new(Measurement {
        size: Size { width: 200.0, height: 130.0 },
        gap_after: 0.0,
        alignment: Alignment::Start,
        content: MeasuredContent::Container {
            children: vec![child1, child2],
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets { top: 5.0, left: 0.0, bottom: 5.0, right: 0.0 },
            border_mode: BorderMode::Separate,
        },
    });

    p.place(NodeId::new(), &m);
    let pages = p.finish();

    assert_eq!(pages.len(), 2);

    // Page 1: container가 breaks.bottom = true
    let Fragment::Container(c1) = &pages[0].fragments[0] else { panic!() };
    assert!(c1.breaks.bottom);

    // Page 2: container가 breaks.top = true
    let Fragment::Container(c2) = &pages[1].fragments[0] else { panic!() };
    assert!(c2.breaks.top);
}
```

Run: `cargo test -p editor-view`
Expected: 모든 테스트 통과.

---

### Task 4: Paginator — Collapse 모드 border 지원

**Files:**
- Modify: `crates/editor-view/src/engine/paginator.rs`

- [ ] **Step 1: Collapse 테스트 작성 — 인접 border overlap**

```rust
#[test]
fn collapse_border_overlaps_adjacent_children() {
    let mut p = Paginator::new_continuous(200.0, 10000.0, 0.0, 0.0, 0.0);

    let b = 2.0;  // border width
    let child1_h = 30.0;  // full height including border
    let child2_h = 30.0;

    let child1 = ChildMeasurement {
        node_id: NodeId::new(),
        measurement: Arc::new(Measurement {
            size: Size { width: 200.0, height: child1_h },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container {
                children: vec![],
                scope: false,
                direction: LayoutDirection::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(b),
                border_mode: BorderMode::Separate,
            },
        }),
    };
    let child2 = ChildMeasurement {
        node_id: NodeId::new(),
        measurement: Arc::new(Measurement {
            size: Size { width: 200.0, height: child2_h },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container {
                children: vec![],
                scope: false,
                direction: LayoutDirection::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(b),
                border_mode: BorderMode::Separate,
            },
        }),
    };

    // Collapsed container: border=2, two children each with border=2
    // collapsed height = (2+1)*2 + (30-4) + (30-4) = 6 + 26 + 26 = 58
    let collapsed_h = 3.0 * b + (child1_h - 2.0 * b) + (child2_h - 2.0 * b);
    let m = Arc::new(Measurement {
        size: Size { width: 200.0, height: collapsed_h },
        gap_after: 0.0,
        alignment: Alignment::Start,
        content: MeasuredContent::Container {
            children: vec![child1, child2],
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::all(b),
            border_mode: BorderMode::Collapse,
        },
    });

    let node_id = NodeId::new();
    p.place(node_id, &m);
    let pages = p.finish();

    let Fragment::Container(c) = &pages[0].fragments[0] else { panic!() };
    assert_eq!(c.children.len(), 2);

    // child1은 y=0 (container.border.top과 child1.border.top collapse)
    let Fragment::Container(ch1) = &c.children[0] else { panic!() };
    assert_eq!(ch1.rect.y, 0.0);

    // child2는 y = child1.height - overlap = 30 - 2 = 28
    let Fragment::Container(ch2) = &c.children[1] else { panic!() };
    assert_eq!(ch2.rect.y, 28.0);
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p editor-view -- collapse_border_overlaps`
Expected: FAIL — Collapse 모드가 아직 구현되지 않음.

- [ ] **Step 3: OpenContainer에 Collapse 추적 필드 추가 및 Collapse placement 구현**

`OpenContainer`에 추가:
```rust
struct OpenContainer {
    // ... 기존 필드
    border: EdgeInsets,
    border_mode: BorderMode,
    last_child_border_end: f32,
    is_first_child: bool,
}
```

`open_container`에서 Collapse 모드 분기:
```rust
fn open_container(&mut self, node_id: NodeId, scope: bool, padding: EdgeInsets, border: EdgeInsets, border_mode: BorderMode) {
    match border_mode {
        BorderMode::Separate => {
            self.current_y += border.top + padding.top;
        }
        BorderMode::Collapse => {
            self.current_y += border.top;
        }
    }
    self.container_stack.push(OpenContainer {
        node_id,
        scope,
        start_y: match border_mode {
            BorderMode::Separate => self.current_y - padding.top - border.top,
            BorderMode::Collapse => self.current_y - border.top,
        },
        width: self.width,
        children: vec![],
        pending_gap: 0.0,
        split_top: false,
        padding,
        border,
        border_mode,
        last_child_border_end: 0.0,
        is_first_child: true,
    });
}
```

`place()` 메서드에서 Vertical Collapse container의 child 배치 전에 overlap 적용. 현재 `place()`의 `LayoutDirection::Vertical` 분기에서 child를 place하기 직전에 collapse overlap을 적용한다:

```rust
LayoutDirection::Vertical => {
    self.open_container(node_id, *scope, *padding, *border, *border_mode);

    for child in children {
        // Collapse overlap 적용
        if let Some(top) = self.container_stack.last_mut() {
            if top.border_mode == BorderMode::Collapse {
                let child_border_top = child_border_top(&child.measurement);
                if top.is_first_child {
                    let overlap = top.border.top.min(child_border_top);
                    self.current_y -= overlap;
                    top.is_first_child = false;
                } else {
                    let overlap = top.last_child_border_end.min(child_border_top);
                    self.current_y -= overlap;
                }
            }
        }

        self.apply_gap_or_break(&child.measurement);
        self.place(child.node_id, &child.measurement);

        if let Some(top) = self.container_stack.last_mut() {
            top.pending_gap = child.measurement.gap_after;
            if top.border_mode == BorderMode::Collapse {
                top.last_child_border_end = child_border_bottom(&child.measurement);
            }
        }
    }

    self.close_container();
}
```

헬퍼 함수 추가:
```rust
fn child_border_top(measurement: &Measurement) -> f32 {
    match &measurement.content {
        MeasuredContent::Container { border, .. } => border.top,
        _ => 0.0,
    }
}

fn child_border_bottom(measurement: &Measurement) -> f32 {
    match &measurement.content {
        MeasuredContent::Container { border, .. } => border.bottom,
        _ => 0.0,
    }
}
```

`close_container`에서 Collapse 모드 close 로직:
```rust
fn close_container(&mut self) {
    let c = self.container_stack.pop().expect("close without open");
    match c.border_mode {
        BorderMode::Separate => {
            self.current_y += c.padding.bottom + c.border.bottom;
        }
        BorderMode::Collapse => {
            let extra = (c.border.bottom - c.last_child_border_end).max(0.0);
            self.current_y += extra;
        }
    }
    // ... 나머지 기존 ContainerFragment 생성 로직
}
```

`break_page()`에서 Collapse 모드 처리:
```rust
// container close 시 (break_page 내부 while 루프)
while let Some(c) = self.container_stack.pop() {
    reopens.push((c.node_id, c.scope, c.width, c.padding, c.border, c.border_mode));
    if c.children.is_empty() {
        continue;
    }

    match c.border_mode {
        BorderMode::Separate => {
            self.current_y += c.border.bottom;
        }
        BorderMode::Collapse => {
            let extra = (c.border.bottom - c.last_child_border_end).max(0.0);
            self.current_y += extra;
        }
    }

    // ... ContainerFragment 생성
}

// reopen 시
for (node_id, scope, width, padding, border, border_mode) in reopens.into_iter().rev() {
    let start_offset = match border_mode {
        BorderMode::Separate => border.top + padding.top,
        BorderMode::Collapse => border.top,
    };
    self.current_y += start_offset;
    self.container_stack.push(OpenContainer {
        node_id,
        scope,
        start_y: self.current_y - start_offset,
        width,
        children: vec![],
        pending_gap: 0.0,
        split_top: true,
        padding,
        border,
        border_mode,
        last_child_border_end: 0.0,
        is_first_child: true,
    });
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: `collapse_border_overlaps_adjacent_children` 통과, 기존 테스트도 모두 통과.

- [ ] **Step 5: Horizontal Collapse 테스트 작성**

```rust
#[test]
fn collapse_horizontal_overlaps_cells() {
    let mut p = Paginator::new_continuous(200.0, 10000.0, 0.0, 0.0, 0.0);

    let b = 1.0;
    let cell_w = 50.0;  // full width including border

    let cell1 = ChildMeasurement {
        node_id: NodeId::new(),
        measurement: Arc::new(Measurement {
            size: Size { width: cell_w, height: 30.0 },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container {
                children: vec![],
                scope: true,
                direction: LayoutDirection::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(b),
                border_mode: BorderMode::Separate,
            },
        }),
    };
    let cell2 = ChildMeasurement {
        node_id: NodeId::new(),
        measurement: Arc::new(Measurement {
            size: Size { width: cell_w, height: 30.0 },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container {
                children: vec![],
                scope: true,
                direction: LayoutDirection::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(b),
                border_mode: BorderMode::Separate,
            },
        }),
    };

    // Collapsed row: border=1, two cells w=50 each
    // collapsed width = (2+1)*1 + (50-2) + (50-2) = 3 + 48 + 48 = 99
    let collapsed_w = 3.0 * b + (cell_w - 2.0 * b) * 2.0;
    let m = Arc::new(Measurement {
        size: Size { width: collapsed_w, height: 30.0 },
        gap_after: 0.0,
        alignment: Alignment::Start,
        content: MeasuredContent::Container {
            children: vec![cell1, cell2],
            scope: false,
            direction: LayoutDirection::Horizontal,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::all(b),
            border_mode: BorderMode::Collapse,
        },
    });

    p.place(NodeId::new(), &m);
    let pages = p.finish();

    let Fragment::Container(row) = &pages[0].fragments[0] else { panic!() };
    assert_eq!(row.children.len(), 2);

    // cell1.x = 0 (container.border.left와 cell1.border.left collapse)
    let Fragment::Container(c1) = &row.children[0] else { panic!() };
    assert_eq!(c1.rect.x, 0.0);

    // cell2.x = cell1.width - overlap = 50 - 1 = 49
    let Fragment::Container(c2) = &row.children[1] else { panic!() };
    assert_eq!(c2.rect.x, 49.0);
}
```

- [ ] **Step 6: Horizontal Collapse 구현**

`place_horizontal`을 Collapse-aware로 수정:

```rust
fn place_horizontal(
    &mut self,
    node_id: NodeId,
    measurement: &Measurement,
    children: &[ChildMeasurement],
    scope: bool,
    border: EdgeInsets,
    border_mode: BorderMode,
) {
    if measurement.size.height > self.remaining() && !self.is_page_empty() {
        self.break_page();
    }

    let mut child_x = 0.0;

    let child_frags: Vec<Fragment> = children
        .iter()
        .enumerate()
        .map(|(i, child)| {
            if border_mode == BorderMode::Collapse {
                let child_border_left = match &child.measurement.content {
                    MeasuredContent::Container { border, .. } => border.left,
                    _ => 0.0,
                };
                if i == 0 {
                    let overlap = border.left.min(child_border_left);
                    child_x -= overlap;
                } else {
                    let prev_border_right = match &children[i - 1].measurement.content {
                        MeasuredContent::Container { border, .. } => border.right,
                        _ => 0.0,
                    };
                    let overlap = prev_border_right.min(child_border_left);
                    child_x -= overlap;
                }
            }

            let frag = self.position_subtree(
                &child.measurement,
                child.node_id,
                child_x,
                self.current_y,
            );
            child_x += child.measurement.size.width;
            frag
        })
        .collect();

    let fragment = Fragment::Container(ContainerFragment {
        node_id,
        rect: Rect {
            x: self.child_x(measurement),
            y: self.current_y,
            width: measurement.size.width,
            height: measurement.size.height,
        },
        children: child_frags,
        scope,
        breaks: Breaks::default(),
    });

    self.add_to_current(fragment);
    self.current_y += measurement.size.height;
}
```

`place()` 호출부에서 border/border_mode 전달:
```rust
LayoutDirection::Horizontal => {
    self.place_horizontal(node_id, measurement, children, *scope, *border, *border_mode);
}
```

- [ ] **Step 7: 전체 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 통과.

---

### Task 5: table_width 모듈 구현

**Files:**
- Create: `crates/editor-view/src/engine/measure_nodes/table_width.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/mod.rs`

- [ ] **Step 1: 테스트 작성**

`table_width.rs` 파일을 생성하고 테스트부터 작성:

```rust
pub const TABLE_BORDER_WIDTH: f32 = 1.0;
pub const TABLE_CELL_PADDING: f32 = 8.0;
pub const MIN_CELL_WIDTH: f32 = 40.0;

pub fn border_width(_col_count: usize) -> f32 { todo!() }
pub fn min_table_width(_col_count: usize) -> f32 { todo!() }
pub fn calculate_col_widths(
    _col_count: usize,
    _custom_widths: Option<&[f32]>,
    _available_width: f32,
) -> Vec<f32> { todo!() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_width_formula() {
        assert_eq!(border_width(0), 1.0);
        assert_eq!(border_width(2), 3.0);
        assert_eq!(border_width(3), 4.0);
    }

    #[test]
    fn min_table_width_formula() {
        assert_eq!(min_table_width(0), 1.0);
        assert_eq!(min_table_width(2), 83.0);  // 2*40 + 3
    }

    #[test]
    fn equal_distribution() {
        let widths = calculate_col_widths(3, None, 300.0);
        assert_eq!(widths.len(), 3);
        assert!((widths[0] - 100.0).abs() < 0.01);
        assert!((widths[1] - 100.0).abs() < 0.01);
        assert!((widths[2] - 100.0).abs() < 0.01);
    }

    #[test]
    fn custom_ratio_widths() {
        let widths = calculate_col_widths(2, Some(&[0.4, 0.6]), 500.0);
        assert!((widths[0] - 200.0).abs() < 0.01);
        assert!((widths[1] - 300.0).abs() < 0.01);
    }

    #[test]
    fn min_width_enforcement() {
        // available=60 < 2*40=80, 모든 열 MIN_CELL_WIDTH
        let widths = calculate_col_widths(2, None, 60.0);
        assert_eq!(widths[0], MIN_CELL_WIDTH);
        assert_eq!(widths[1], MIN_CELL_WIDTH);
    }

    #[test]
    fn small_ratio_gets_min_width() {
        // ratio [0.05, 0.95], available=500
        // 0.05 * scale → < 40 → clamped to 40
        let widths = calculate_col_widths(2, Some(&[0.05, 0.95]), 500.0);
        assert_eq!(widths[0], MIN_CELL_WIDTH);
        assert!((widths[0] + widths[1] - 500.0).abs() < 0.01);
    }

    #[test]
    fn zero_columns() {
        let widths = calculate_col_widths(0, None, 500.0);
        assert!(widths.is_empty());
    }
}
```

- [ ] **Step 2: mod.rs에 table_width 모듈 등록**

`measure_nodes/mod.rs`에 추가:
```rust
pub(crate) mod table_width;
```

- [ ] **Step 3: 테스트 실패 확인**

Run: `cargo test -p editor-view -- table_width`
Expected: FAIL — `todo!()` panic.

- [ ] **Step 4: border_width, min_table_width 구현**

```rust
pub fn border_width(col_count: usize) -> f32 {
    (col_count + 1) as f32 * TABLE_BORDER_WIDTH
}

pub fn min_table_width(col_count: usize) -> f32 {
    if col_count == 0 {
        return border_width(0);
    }
    col_count as f32 * MIN_CELL_WIDTH + border_width(col_count)
}
```

- [ ] **Step 5: calculate_col_widths 구현**

레거시 `TableWidthModel::calculate_col_widths` 알고리즘을 포팅:

```rust
pub fn calculate_col_widths(
    col_count: usize,
    custom_widths: Option<&[f32]>,
    available_width: f32,
) -> Vec<f32> {
    if col_count == 0 {
        return vec![];
    }

    let ratios: Vec<f32> = match custom_widths {
        Some(cw) => cw.iter().map(|&w| if w.is_finite() && w >= 0.0 { w } else { 0.0 }).collect(),
        None => vec![1.0 / col_count as f32; col_count],
    };

    let min_total = col_count as f32 * MIN_CELL_WIDTH;
    if available_width <= min_total {
        return vec![MIN_CELL_WIDTH; col_count];
    }

    let ratio_sum: f32 = ratios.iter().sum();
    if ratio_sum <= 1e-7 {
        let each = available_width / col_count as f32;
        return vec![each; col_count];
    }

    let mut widths = vec![MIN_CELL_WIDTH; col_count];

    let mut indexed_ratios: Vec<(usize, f32)> = ratios
        .iter()
        .enumerate()
        .filter(|(_, &r)| r > 0.0)
        .map(|(i, &r)| (i, r))
        .collect();
    indexed_ratios.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    if indexed_ratios.is_empty() {
        let each = available_width / col_count as f32;
        return vec![each; col_count];
    }

    let constrained_count = col_count - indexed_ratios.len();
    let mut remaining_width = available_width - constrained_count as f32 * MIN_CELL_WIDTH;
    let mut unconstrained_ratio_sum = ratio_sum;

    let mut constrained_end = 0;
    for (pos, &(_, ratio)) in indexed_ratios.iter().enumerate() {
        let scale = remaining_width / unconstrained_ratio_sum;
        if scale * ratio >= MIN_CELL_WIDTH {
            break;
        }
        remaining_width -= MIN_CELL_WIDTH;
        unconstrained_ratio_sum -= ratio;
        constrained_end = pos + 1;
    }

    if unconstrained_ratio_sum > 1e-7 {
        let scale = remaining_width / unconstrained_ratio_sum;
        for &(idx, ratio) in &indexed_ratios[constrained_end..] {
            widths[idx] = scale * ratio;
        }
    }

    // 부동소수점 보정
    let total: f32 = widths.iter().sum();
    let diff = available_width - total;
    let tolerance = (1e-6 * available_width).max(1e-4);
    if diff.abs() > tolerance {
        if let Some(&(last_idx, _)) = indexed_ratios.last() {
            if diff > 0.0 || widths[last_idx] + diff >= MIN_CELL_WIDTH {
                widths[last_idx] += diff;
            }
        }
    }

    widths
}
```

- [ ] **Step 6: 테스트 통과 확인**

Run: `cargo test -p editor-view -- table_width`
Expected: 모든 테스트 통과.

---

### Task 6: Fold 측정 구현

**Files:**
- Create: `crates/editor-view/src/engine/measure_nodes/fold.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/mod.rs`
- Modify: `crates/editor-view/src/engine/mod.rs`

- [ ] **Step 1: fold.rs 생성 — 테스트 작성**

```rust
use editor_common::{Alignment, EdgeInsets, Size};
use editor_model::{Doc, Node, NodeRef};
use std::sync::Arc;

use super::super::LayoutEngine;
use super::super::resolve::resolve_gap_after;
use super::container::measure_padded_container;
use crate::measure::*;
use crate::view_state::ViewState;

const FOLD_TITLE_PADDING_X: f32 = 12.0;
const FOLD_TITLE_PADDING_Y: f32 = 8.0;
const FOLD_TITLE_ICON_WIDTH: f32 = 20.0;
const FOLD_TITLE_ICON_GAP: f32 = 8.0;
const FOLD_CONTENT_PADDING_X: f32 = 24.0;
const FOLD_CONTENT_PADDING_Y: f32 = 16.0;
const FOLD_BORDER_WIDTH: f32 = 1.0;

pub fn measure_fold(
    _engine: &mut LayoutEngine,
    _doc: &Doc,
    _node: &NodeRef<'_>,
    _width: f32,
    _view_state: &ViewState,
) -> Measurement { todo!() }

pub fn measure_fold_title(
    _engine: &mut LayoutEngine,
    _doc: &Doc,
    _node: &NodeRef<'_>,
    _width: f32,
    _view_state: &ViewState,
) -> Measurement { todo!() }

pub fn measure_fold_content(
    _engine: &mut LayoutEngine,
    _doc: &Doc,
    _node: &NodeRef<'_>,
    _width: f32,
    _view_state: &ViewState,
) -> Measurement { todo!() }

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use super::*;

    #[test]
    fn fold_collapsed_excludes_content() {
        let (doc, fold_id) = doc! {
            root {
                fold_id: fold {
                    fold_title { paragraph { text("Title") } }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let node = doc.node(fold_id).unwrap();
        let mut engine = LayoutEngine::new();
        let vs = ViewState::new();  // fold_expanded 기본값 = true

        // collapsed 상태로 설정
        let mut vs_collapsed = vs.clone();
        vs_collapsed.fold_states.insert(fold_id, false);

        let result = measure_fold(&mut engine, &doc, &node, 300.0, &vs_collapsed);
        let MeasuredContent::Container { children, border, border_mode, .. } = &result.content else { panic!() };

        assert_eq!(children.len(), 1);  // FoldTitle만
        assert_eq!(border.top, FOLD_BORDER_WIDTH);
        assert_eq!(*border_mode, BorderMode::Separate);
    }

    #[test]
    fn fold_expanded_includes_content() {
        let (doc, fold_id) = doc! {
            root {
                fold_id: fold {
                    fold_title { paragraph { text("Title") } }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let node = doc.node(fold_id).unwrap();
        let mut engine = LayoutEngine::new();
        let mut vs = ViewState::new();
        vs.fold_states.insert(fold_id, true);

        let result = measure_fold(&mut engine, &doc, &node, 300.0, &vs);
        let MeasuredContent::Container { children, .. } = &result.content else { panic!() };

        assert_eq!(children.len(), 2);  // FoldTitle + FoldContent
    }

    #[test]
    fn fold_title_has_icon_padding() {
        let (doc, title_id) = doc! {
            root {
                fold {
                    title_id: fold_title { paragraph { text("Title") } }
                    fold_content
                }
            }
        };

        let node = doc.node(title_id).unwrap();
        let mut engine = LayoutEngine::new();
        let result = measure_fold_title(&mut engine, &doc, &node, 300.0, &ViewState::new());

        let MeasuredContent::Container { padding, .. } = &result.content else { panic!() };
        assert_eq!(padding.left, FOLD_TITLE_PADDING_X + FOLD_TITLE_ICON_WIDTH + FOLD_TITLE_ICON_GAP);
        assert_eq!(padding.top, FOLD_TITLE_PADDING_Y);
    }

    #[test]
    fn fold_content_has_padding() {
        let (doc, content_id) = doc! {
            root {
                fold {
                    fold_title
                    content_id: fold_content { paragraph { text("Content") } }
                }
            }
        };

        let node = doc.node(content_id).unwrap();
        let mut engine = LayoutEngine::new();
        let result = measure_fold_content(&mut engine, &doc, &node, 300.0, &ViewState::new());

        let MeasuredContent::Container { padding, .. } = &result.content else { panic!() };
        assert_eq!(padding.left, FOLD_CONTENT_PADDING_X);
        assert_eq!(padding.top, FOLD_CONTENT_PADDING_Y);
    }
}
```

- [ ] **Step 2: mod.rs에 fold 모듈 등록 및 re-export**

```rust
mod fold;
pub use fold::{measure_fold, measure_fold_title, measure_fold_content};
```

- [ ] **Step 3: 테스트 실패 확인**

Run: `cargo test -p editor-view -- fold`
Expected: FAIL — `todo!()` panic.

- [ ] **Step 4: measure_fold_title, measure_fold_content 구현**

```rust
pub fn measure_fold_title(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let padding = EdgeInsets {
        top: FOLD_TITLE_PADDING_Y,
        left: FOLD_TITLE_PADDING_X + FOLD_TITLE_ICON_WIDTH + FOLD_TITLE_ICON_GAP,
        bottom: FOLD_TITLE_PADDING_Y,
        right: FOLD_TITLE_PADDING_X,
    };
    measure_padded_container(engine, doc, node, width, view_state, padding, EdgeInsets::ZERO, false, Alignment::Start)
}

pub fn measure_fold_content(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let padding = EdgeInsets {
        top: FOLD_CONTENT_PADDING_Y,
        left: FOLD_CONTENT_PADDING_X,
        bottom: FOLD_CONTENT_PADDING_Y,
        right: FOLD_CONTENT_PADDING_X,
    };
    measure_padded_container(engine, doc, node, width, view_state, padding, EdgeInsets::ZERO, false, Alignment::Start)
}
```

- [ ] **Step 5: measure_fold 구현**

```rust
pub fn measure_fold(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let expanded = view_state.fold_expanded(node.id());
    let border = EdgeInsets::all(FOLD_BORDER_WIDTH);
    let content_width = width - border.left - border.right;

    let mut children: Vec<ChildMeasurement> = Vec::new();
    let mut children_height = 0.0;

    for child in node.children() {
        match child.node() {
            Node::FoldTitle(_) => {
                let m = engine.measure(doc, child.id(), content_width, view_state);
                children_height += m.size.height;
                children.push(ChildMeasurement { node_id: child.id(), measurement: m });
            }
            Node::FoldContent(_) if expanded => {
                let m = engine.measure(doc, child.id(), content_width, view_state);
                children_height += m.size.height;
                children.push(ChildMeasurement { node_id: child.id(), measurement: m });
            }
            _ => {}
        }
    }

    let height = border.top + children_height + border.bottom;

    Measurement {
        size: Size { width, height },
        gap_after: resolve_gap_after(node),
        alignment: Alignment::Start,
        content: MeasuredContent::Container {
            children,
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border,
            border_mode: BorderMode::Separate,
        },
    }
}
```

- [ ] **Step 6: measure_inner dispatch에 Fold case 추가**

`engine/mod.rs`의 `measure_inner`에:

```rust
Node::Fold(_) => measure_nodes::measure_fold(self, doc, node, width, view_state),
Node::FoldTitle(_) => measure_nodes::measure_fold_title(self, doc, node, width, view_state),
Node::FoldContent(_) => measure_nodes::measure_fold_content(self, doc, node, width, view_state),
```

기존 `_ =>` 패턴 위에 추가.

- [ ] **Step 7: 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 통과.

---

### Task 7: Table 측정 구현

**Files:**
- Create: `crates/editor-view/src/engine/measure_nodes/table.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/mod.rs`
- Modify: `crates/editor-view/src/engine/mod.rs`

- [ ] **Step 1: table.rs 생성 — 테스트 작성**

```rust
use editor_common::{Alignment, EdgeInsets, Size};
use editor_model::{Doc, Node, NodeRef};
use std::sync::Arc;

use super::super::LayoutEngine;
use super::super::resolve::resolve_gap_after;
use super::container::measure_padded_container;
use super::table_width::*;
use crate::measure::*;
use crate::view_state::ViewState;

pub fn measure_table(
    _engine: &mut LayoutEngine,
    _doc: &Doc,
    _node: &NodeRef<'_>,
    _width: f32,
    _view_state: &ViewState,
) -> Measurement { todo!() }

pub fn measure_table_cell(
    _engine: &mut LayoutEngine,
    _doc: &Doc,
    _node: &NodeRef<'_>,
    _width: f32,
    _view_state: &ViewState,
) -> Measurement { todo!() }

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::nodes::table::{TableAlign, TableBorderStyle};
    use super::*;

    #[test]
    fn table_cell_has_padding_border_scope() {
        let (doc, cell_id) = doc! {
            root {
                table {
                    table_row {
                        cell_id: table_cell {
                            paragraph { text("Hello") }
                        }
                    }
                }
            }
        };

        let node = doc.node(cell_id).unwrap();
        let mut engine = LayoutEngine::new();
        let result = measure_table_cell(&mut engine, &doc, &node, 100.0, &ViewState::new());

        let MeasuredContent::Container { padding, border, scope, .. } = &result.content else { panic!() };
        assert_eq!(padding.left, TABLE_CELL_PADDING);
        assert_eq!(padding.top, TABLE_CELL_PADDING);
        assert_eq!(border.left, TABLE_BORDER_WIDTH);
        assert_eq!(border.top, TABLE_BORDER_WIDTH);
        assert!(scope);
    }

    #[test]
    fn table_2x2_collapsed_size() {
        let (doc, table_id) = doc! {
            root {
                table_id: table {
                    table_row {
                        table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        table_cell { paragraph { text("C") } }
                        table_cell { paragraph { text("D") } }
                    }
                }
            }
        };

        let node = doc.node(table_id).unwrap();
        let mut engine = LayoutEngine::new();
        let result = measure_table(&mut engine, &doc, &node, 500.0, &ViewState::new());

        let MeasuredContent::Container { children, border, border_mode, direction, .. } = &result.content else { panic!() };
        assert_eq!(children.len(), 2);  // 2 rows
        assert_eq!(*border_mode, BorderMode::Collapse);
        assert_eq!(*direction, LayoutDirection::Vertical);
        assert_eq!(border.top, TABLE_BORDER_WIDTH);

        // 각 Row는 Horizontal Collapse Container
        let MeasuredContent::Container { children: row_cells, direction: row_dir, border_mode: row_bm, .. } = &children[0].measurement.content else { panic!() };
        assert_eq!(row_cells.len(), 2);
        assert_eq!(*row_dir, LayoutDirection::Horizontal);
        assert_eq!(*row_bm, BorderMode::Collapse);
    }

    #[test]
    fn table_align_center() {
        let (doc, table_id) = doc! {
            root {
                table_id: table(align: TableAlign::Center) {
                    table_row {
                        table_cell { paragraph }
                    }
                }
            }
        };

        let node = doc.node(table_id).unwrap();
        let mut engine = LayoutEngine::new();
        let result = measure_table(&mut engine, &doc, &node, 500.0, &ViewState::new());

        assert_eq!(result.alignment, Alignment::Center);
    }

    #[test]
    fn table_custom_col_widths() {
        let (doc, table_id) = doc! {
            root {
                table_id: table {
                    table_row {
                        table_cell(col_width: Some(0.3)) { paragraph }
                        table_cell(col_width: Some(0.7)) { paragraph }
                    }
                }
            }
        };

        let node = doc.node(table_id).unwrap();
        let mut engine = LayoutEngine::new();
        let result = measure_table(&mut engine, &doc, &node, 500.0, &ViewState::new());

        // proportion=1.0 → table_width=500
        // inner_width = 500 - 3 = 497  (border_width(2) = 3)
        // col_widths = [0.3*497, 0.7*497] ≈ [149.1, 347.9]
        let MeasuredContent::Container { children, .. } = &result.content else { panic!() };
        let MeasuredContent::Container { children: cells, .. } = &children[0].measurement.content else { panic!() };
        // Cell widths should reflect custom ratios (30/70 split)
        let w0 = cells[0].measurement.size.width;
        let w1 = cells[1].measurement.size.width;
        assert!((w0 / (w0 + w1) - 0.3).abs() < 0.01);
    }

    #[test]
    fn empty_table_returns_zero_height() {
        let (doc, table_id) = doc! {
            root {
                table_id: table
            }
        };

        let node = doc.node(table_id).unwrap();
        let mut engine = LayoutEngine::new();
        let result = measure_table(&mut engine, &doc, &node, 500.0, &ViewState::new());

        assert_eq!(result.size.height, 0.0);
    }
}
```

- [ ] **Step 2: mod.rs에 table 모듈 등록 및 re-export**

```rust
mod table;
pub use table::{measure_table, measure_table_cell};
```

- [ ] **Step 3: 테스트 실패 확인**

Run: `cargo test -p editor-view -- table`
Expected: FAIL — `todo!()` panic.

- [ ] **Step 4: measure_table_cell 구현**

```rust
pub fn measure_table_cell(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let padding = EdgeInsets::all(TABLE_CELL_PADDING);
    let border = EdgeInsets::all(TABLE_BORDER_WIDTH);
    measure_padded_container(engine, doc, node, width, view_state, padding, border, true, Alignment::Start)
}
```

- [ ] **Step 5: measure_table 구현**

```rust
pub fn measure_table(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let Node::Table(table_node) = node.node() else { unreachable!() };

    let rows: Vec<NodeRef<'_>> = node.children().collect();
    if rows.is_empty() {
        return empty_table_measurement(node, width);
    }

    let col_count = rows[0].children().count();
    if col_count == 0 {
        return empty_table_measurement(node, width);
    }

    // col_widths 계산
    let custom_widths = extract_custom_widths(&rows[0], col_count);
    let proportion = table_node.proportion.clamp(0.0, 1.0);
    let target_width = proportion * width;
    let floor = min_table_width(col_count).min(width);
    let table_width = target_width.max(floor);
    let inner_width = table_width - border_width(col_count);
    let col_widths = calculate_col_widths(col_count, custom_widths.as_deref(), inner_width);

    let b = TABLE_BORDER_WIDTH;

    // cell_width = col_width + border left + border right (full cell size)
    let cell_full_widths: Vec<f32> = col_widths.iter().map(|&cw| cw + 2.0 * b).collect();
    let actual_table_width = (col_count + 1) as f32 * b + col_widths.iter().sum::<f32>();

    // 각 Row 측정
    let mut row_measurements: Vec<ChildMeasurement> = Vec::new();

    for row_ref in &rows {
        let cells: Vec<NodeRef<'_>> = row_ref.children().collect();
        let cell_count = cells.len().min(col_count);

        // 1st pass: 각 Cell 측정
        let mut cell_measurements: Vec<(NodeId, Arc<Measurement>)> = Vec::new();
        let mut max_height: f32 = 0.0;

        for (i, cell_ref) in cells.iter().take(cell_count).enumerate() {
            let m = engine.measure(doc, cell_ref.id(), cell_full_widths[i], view_state);
            max_height = max_height.max(m.size.height);
            cell_measurements.push((cell_ref.id(), m));
        }

        // 2nd pass: height 균등화
        let row_children: Vec<ChildMeasurement> = cell_measurements
            .into_iter()
            .map(|(id, m)| {
                let measurement = if (m.size.height - max_height).abs() > f32::EPSILON {
                    Arc::new(Measurement {
                        size: Size { width: m.size.width, height: max_height },
                        ..(*m).clone()
                    })
                } else {
                    m
                };
                ChildMeasurement { node_id: id, measurement }
            })
            .collect();

        let row_m = Arc::new(Measurement {
            size: Size { width: actual_table_width, height: max_height },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container {
                children: row_children,
                scope: false,
                direction: LayoutDirection::Horizontal,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(b),
                border_mode: BorderMode::Collapse,
            },
        });

        row_measurements.push(ChildMeasurement {
            node_id: row_ref.id(),
            measurement: row_m,
        });
    }

    // collapsed table height
    let row_count = row_measurements.len();
    let row_inner_height_sum: f32 = row_measurements
        .iter()
        .map(|r| r.measurement.size.height - 2.0 * b)
        .sum();
    let collapsed_height = (row_count + 1) as f32 * b + row_inner_height_sum;

    let alignment = match table_node.align {
        editor_model::nodes::table::TableAlign::Left => Alignment::Start,
        editor_model::nodes::table::TableAlign::Center => Alignment::Center,
        editor_model::nodes::table::TableAlign::Right => Alignment::End,
    };

    Measurement {
        size: Size { width: actual_table_width, height: collapsed_height },
        gap_after: resolve_gap_after(node),
        alignment,
        content: MeasuredContent::Container {
            children: row_measurements,
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::all(b),
            border_mode: BorderMode::Collapse,
        },
    }
}

fn empty_table_measurement(node: &NodeRef<'_>, width: f32) -> Measurement {
    Measurement {
        size: Size { width, height: 0.0 },
        gap_after: resolve_gap_after(node),
        alignment: Alignment::Start,
        content: MeasuredContent::Container {
            children: vec![],
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            border_mode: BorderMode::Separate,
        },
    }
}

fn extract_custom_widths(first_row: &NodeRef<'_>, col_count: usize) -> Option<Vec<f32>> {
    let widths: Vec<Option<f32>> = first_row
        .children()
        .take(col_count)
        .map(|cell| {
            if let Node::TableCell(tc) = cell.node() {
                tc.col_width
            } else {
                None
            }
        })
        .collect();

    if widths.iter().all(|w| w.is_some()) {
        Some(widths.into_iter().map(|w| w.unwrap()).collect())
    } else {
        None
    }
}
```

- [ ] **Step 6: measure_inner dispatch에 Table case 추가**

`engine/mod.rs`의 `measure_inner`에 기존 `_ =>` 패턴 위에 추가:

```rust
Node::Table(_) => measure_nodes::measure_table(self, doc, node, width, view_state),
Node::TableCell(_) => measure_nodes::measure_table_cell(self, doc, node, width, view_state),
```

- [ ] **Step 7: `measure_default_container`에서 TableCell/TableRow 특수 처리 제거**

`container.rs`의 `measure_default_container`에서:
```rust
let scope = matches!(node.node(), Node::TableCell(_));
let direction = if matches!(node.node(), Node::TableRow(_)) {
    LayoutDirection::Horizontal
} else {
    LayoutDirection::Vertical
};
```

TableCell은 이제 `measure_table_cell`로 dispatch되므로 `scope` 줄을 변경:
```rust
let scope = false;  // TableCell은 measure_table_cell에서 처리
let direction = if matches!(node.node(), Node::TableRow(_)) {
    LayoutDirection::Horizontal
} else {
    LayoutDirection::Vertical
};
```

TableRow는 measure_table이 직접 구성하므로 measure_inner dispatch를 타지 않지만, fallthrough 시 기존 동작을 유지.

- [ ] **Step 8: 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 통과.

---

### Task 8: 통합 테스트 및 검증

**Files:**
- Modify: `crates/editor-view/src/engine/mod.rs` (테스트 추가)

- [ ] **Step 1: LayoutEngine.compute()를 통한 Fold 통합 테스트**

`engine/mod.rs` 테스트에 추가:

```rust
#[test]
fn compute_with_fold_collapsed() {
    use editor_model::{LayoutMode, DocumentAttrs};

    let (doc, fold_id) = doc! {
        root {
            fold_id: fold {
                fold_title { paragraph { text("Title") } }
                fold_content { paragraph { text("Content") } }
            }
        }
    };

    let viewport = Viewport { width: 400.0, scale_factor: 1.0 };
    let mut vs = ViewState::new();
    vs.fold_states.insert(fold_id, false);

    let mut engine = LayoutEngine::new();
    engine.compute(&doc, &viewport, &vs);

    assert!(!engine.pages().is_empty());
}
```

- [ ] **Step 2: LayoutEngine.compute()를 통한 Table 통합 테스트**

```rust
#[test]
fn compute_with_table() {
    let (doc, _table_id) = doc! {
        root {
            _table_id: table {
                table_row {
                    table_cell { paragraph { text("A") } }
                    table_cell { paragraph { text("B") } }
                }
            }
        }
    };

    let viewport = Viewport { width: 400.0, scale_factor: 1.0 };
    let vs = ViewState::new();

    let mut engine = LayoutEngine::new();
    engine.compute(&doc, &viewport, &vs);

    assert!(!engine.pages().is_empty());
}
```

- [ ] **Step 3: 전체 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 통과.

- [ ] **Step 4: 전체 프로젝트 빌드 확인**

Run: `cargo build -p editor-view`
Expected: 컴파일 성공, 경고 없음.
