# measure_inner: Atom + Container 측정 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** measure_inner()를 Atom/Container 노드별로 실제 측정하도록 구현하고, Paginator에서 padding과 alignment을 반영한다.

**Architecture:** measure_inner를 노드 타입별 헬퍼 함수로 dispatch. Measurement에 alignment, MeasuredContent::Container에 padding(EdgeInsets) 추가. Paginator가 padding offset과 cross-axis alignment을 반영하여 Fragment를 배치.

**Tech Stack:** Rust (edition 2024), editor-view crate, editor-model (Node, Modifier, ModifierType, NodeRef)

**Spec:** `docs/editor-architecture/view-measure-atom-container-design.md`

---

## File Structure

**New files:**
- `crates/editor-view/src/measure/alignment.rs` — Alignment enum
- `crates/editor-view/src/measure/edge_insets.rs` — EdgeInsets struct
- `crates/editor-view/src/engine/resolve.rs` — resolve_inherited, resolve_gap_after
- `crates/editor-view/src/engine/measure_nodes.rs` — measure_atom, measure_list_item, measure_blockquote, measure_callout, measure_default_container

**Modified files:**
- `crates/editor-view/src/measure/mod.rs` — Measurement에 alignment 추가, Container에 padding 추가, 모듈 선언
- `crates/editor-view/src/engine/mod.rs` — measure_inner dispatch 재작성, 모듈 선언
- `crates/editor-view/src/engine/paginator.rs` — OpenContainer에 padding 추가, place()에서 padding/alignment 반영

---

### Task 1: Alignment + EdgeInsets 타입 추가

**Files:**
- Create: `crates/editor-view/src/measure/alignment.rs`
- Create: `crates/editor-view/src/measure/edge_insets.rs`
- Modify: `crates/editor-view/src/measure/mod.rs`

- [ ] **Step 1: alignment.rs 생성**

```rust
// crates/editor-view/src/measure/alignment.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
}
```

- [ ] **Step 2: edge_insets.rs 생성**

```rust
// crates/editor-view/src/measure/edge_insets.rs
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgeInsets {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self {
        top: 0.0,
        left: 0.0,
        bottom: 0.0,
        right: 0.0,
    };
}
```

- [ ] **Step 3: measure/mod.rs 업데이트 — 모듈 선언 + 타입 변경**

`crates/editor-view/src/measure/mod.rs` 변경:

```rust
mod alignment;
mod edge_insets;

pub use alignment::Alignment;
pub use edge_insets::EdgeInsets;

use editor_common::Size;
use editor_model::NodeId;
use std::sync::Arc;

use crate::fragment::LineSegment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub size: Size,
    pub gap_after: f32,
    pub content: MeasuredContent,
    pub alignment: Alignment,
}

#[derive(Debug, Clone)]
pub enum MeasuredContent {
    Container {
        children: Vec<ChildMeasurement>,
        scope: bool,
        direction: LayoutDirection,
        padding: EdgeInsets,
    },
    TextBlock {
        lines: Vec<MeasuredLine>,
    },
    Atom {
        parent_id: NodeId,
        index: usize,
    },
    PageBreak,
}

#[derive(Debug, Clone)]
pub struct ChildMeasurement {
    pub node_id: NodeId,
    pub measurement: Arc<Measurement>,
}

#[derive(Debug, Clone)]
pub struct MeasuredLine {
    pub height: f32,
    pub baseline: f32,
    pub segments: Vec<LineSegment>,
}
```

- [ ] **Step 4: engine/mod.rs 컴파일 오류 수정**

`crates/editor-view/src/engine/mod.rs`���서 Measurement 생성 부분 수정:

measure_inner 함수의 PageBreak 반환:
```rust
Measurement {
    size: Size { width, height: 0.0 },
    gap_after: 0.0,
    content: MeasuredContent::PageBreak,
    alignment: Alignment::Start,
}
```

measure_inner 함수의 Container 반환:
```rust
Measurement {
    size: Size { width, height },
    gap_after: 0.0,
    content: MeasuredContent::Container {
        children,
        scope,
        direction,
        padding: EdgeInsets::ZERO,
    },
    alignment: Alignment::Start,
}
```

engine/mod.rs 테스트의 `dummy()` 함수:
```rust
fn dummy() -> Arc<Measurement> {
    Arc::new(Measurement {
        size: Size { width: 100.0, height: 20.0 },
        gap_after: 0.0,
        content: MeasuredContent::Atom {
            parent_id: NodeId::ROOT,
            index: 0,
        },
        alignment: Alignment::Start,
    })
}
```

- [ ] **Step 5: engine/paginator.rs 컴파일 오류 수정**

paginator.rs의 `place()` 메서드에서 Container destructure에 `padding` 추가:

```rust
MeasuredContent::Container {
    children,
    scope,
    direction,
    padding,  // 추가 (아직 사용하지 않음, _ 가능)
} => match direction {
```

`position_subtree()`의 Container destructure도 동일하게:

```rust
MeasuredContent::Container {
    children,
    scope,
    direction,
    padding: _,  // 아직 미사용
} => {
```

paginator.rs 테스트 헬퍼 함수들 수정:

`container_m`:
```rust
fn container_m(height: f32, children: Vec<ChildMeasurement>) -> Arc<Measurement> {
    Arc::new(Measurement {
        size: Size { width: 200.0, height },
        gap_after: 0.0,
        content: MeasuredContent::Container {
            children,
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
        },
        alignment: Alignment::Start,
    })
}
```

`page_break_m`:
```rust
fn page_break_m() -> Arc<Measurement> {
    Arc::new(Measurement {
        size: Size { width: 200.0, height: 0.0 },
        gap_after: 0.0,
        content: MeasuredContent::PageBreak,
        alignment: Alignment::Start,
    })
}
```

`text_block_m`:
```rust
fn text_block_m(line_heights: &[f32]) -> Arc<Measurement> {
    let lines: Vec<MeasuredLine> = line_heights
        .iter()
        .map(|&h| MeasuredLine {
            height: h,
            baseline: h * 0.8,
            segments: vec![],
        })
        .collect();
    let height: f32 = line_heights.iter().sum();
    Arc::new(Measurement {
        size: Size { width: 200.0, height },
        gap_after: 0.0,
        content: MeasuredContent::TextBlock { lines },
        alignment: Alignment::Start,
    })
}
```

- [ ] **Step 6: 전체 테스트 실행**

Run: `cargo test -p editor-view`
Expected: 기존 32개 paginator 테스트 + 3개 engine 테스트 모두 PASS

---

### Task 2: Paginator padding 지원

**Files:**
- Modify: `crates/editor-view/src/engine/paginator.rs`

- [ ] **Step 1: padding 적용 테스트 작성**

paginator.rs의 `#[cfg(test)] mod tests` 안에 추가:

```rust
fn container_m_with_padding(
    height: f32,
    children: Vec<ChildMeasurement>,
    padding: EdgeInsets,
) -> Arc<Measurement> {
    Arc::new(Measurement {
        size: Size { width: 200.0, height },
        gap_after: 0.0,
        content: MeasuredContent::Container {
            children,
            scope: false,
            direction: LayoutDirection::Vertical,
            padding,
        },
        alignment: Alignment::Start,
    })
}

#[test]
fn container_padding_offsets_children() {
    let mut p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);

    let inner_child = ChildMeasurement {
        node_id: NodeId::new(),
        measurement: leaf_container_m(30.0),
    };
    let padding = EdgeInsets {
        top: 10.0,
        left: 20.0,
        bottom: 10.0,
        right: 0.0,
    };
    let outer = container_m_with_padding(50.0, vec![inner_child], padding);
    let outer_id = NodeId::new();

    p.place(outer_id, &outer);
    let pages = p.finish();

    let container = pages[0].fragments[0].as_container().unwrap();
    assert_eq!(container.rect.height, 50.0);

    let child = container.children[0].as_container().unwrap();
    assert_eq!(child.rect.y, 10.0, "child y should start at padding.top");
    assert_eq!(child.rect.x, 20.0, "child x should start at padding.left");
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-view container_padding_offsets_children`
Expected: FAIL — Fragment에 `as_container()` 메서드가 없거나, child 위치가 padding을 반영하지 않음

참고: `as_container()` 없으면 패�� 매칭으로 대체:
```rust
let Fragment::Container(container) = &pages[0].fragments[0] else { panic!() };
let Fragment::Container(child) = &container.children[0] else { panic!() };
```

- [ ] **Step 3: OpenContainer에 padding 추가 + open_container 수정**

paginator.rs의 `OpenContainer` struct:
```rust
struct OpenContainer {
    node_id: NodeId,
    scope: bool,
    start_y: f32,
    width: f32,
    children: Vec<Fragment>,
    pending_gap: f32,
    split_top: bool,
    padding: EdgeInsets,  // 추가
}
```

`open_container` 메서드에 padding 파라미터 추가:
```rust
fn open_container(&mut self, node_id: NodeId, scope: bool, padding: EdgeInsets) {
    self.current_y += padding.top;
    self.container_stack.push(OpenContainer {
        node_id,
        scope,
        start_y: self.current_y - padding.top,
        width: self.width,
        children: vec![],
        pending_gap: 0.0,
        split_top: false,
        padding,
    });
}
```

- [ ] **Step 4: close_container에서 padding.bottom 반영**

```rust
fn close_container(&mut self) {
    let c = self.container_stack.pop().expect("close without open");
    self.current_y += c.padding.bottom;
    let fragment = Fragment::Container(ContainerFragment {
        node_id: c.node_id,
        rect: Rect {
            x: self.margin_left,
            y: c.start_y,
            width: c.width,
            height: self.current_y - c.start_y,
        },
        children: c.children,
        scope: c.scope,
        breaks: Breaks {
            top: c.split_top,
            bottom: false,
        },
    });
    self.add_to_current(fragment);
}
```

- [ ] **Step 5: place()에서 padding을 open_container에 전달 + child x offset**

place() 메서드의 Vertical Container 분기:

```rust
LayoutDirection::Vertical if children.is_empty() => {
    // 기존 로직 유지 (빈 container)
}
LayoutDirection::Vertical => {
    self.open_container(node_id, *scope, *padding);

    for child in children {
        self.apply_gap_or_break(&child.measurement);
        self.place(child.node_id, &child.measurement);
        if let Some(top) = self.container_stack.last_mut() {
            top.pending_gap = child.measurement.gap_after;
        }
    }

    self.close_container();
}
```

TextBlock 분기:
```rust
MeasuredContent::TextBlock { lines } => {
    self.open_container(node_id, false, EdgeInsets::ZERO);
    // ...
}
```

child fragment의 x 좌표를 padding.left로 offset하기 위해, open_container 시 `self.margin_left`을 임시 조정하거나, 별도 메커니즘 필요.

**접근법:** open_container에서 container의 margin_left을 padding.left로 override. 실제 구현은 container_stack에서 현재 x offset을 ���산:

place() 내 Atom과 Line의 x 좌표를 결정하는 `self.margin_left`을 container padding 기반으로 계산하는 헬퍼:

```rust
fn current_x(&self) -> f32 {
    self.margin_left
        + self.container_stack.iter().map(|c| c.padding.left).sum::<f32>()
}
```

중첩된 padded container (예: BulletList > ListItem)에서 padding.left이 누적됨.

기존 `self.margin_left` 사용 부분을 `self.current_x()`로 교체:
- place() 내 빈 Container의 rect.x
- place() 내 Atom의 rect.x
- place_line() 내 rect.x
- close_container() 내 rect.x — pop 후 current_x()는 부모 컨텍스트의 x를 반환하므로 container 자체 위치로 올바름

- [ ] **Step 6: break_page()에서 padding 보존**

break_page() 내 reopens에 padding 추가:

```rust
fn break_page(&mut self) {
    let mut reopens: Vec<(NodeId, bool, f32, EdgeInsets)> = vec![];

    while let Some(c) = self.container_stack.pop() {
        reopens.push((c.node_id, c.scope, c.width, c.padding));
        // ... 기존 로직
    }

    // ... 페이지 저장 로직

    for (node_id, scope, width, padding) in reopens.into_iter().rev() {
        self.current_y += padding.top;
        self.container_stack.push(OpenContainer {
            node_id,
            scope,
            start_y: self.current_y - padding.top,
            width,
            children: vec![],
            pending_gap: 0.0,
            split_top: true,
            padding,
        });
    }
}
```

- [ ] **Step 7: 테스트 실행**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 PASS (기존 32개 + 새 padding 테스트)

---

### Task 3: Paginator alignment 지원

**Files:**
- Modify: `crates/editor-view/src/engine/paginator.rs`

- [ ] **Step 1: alignment 테스트 작성**

```rust
#[test]
fn alignment_end_positions_child_at_right() {
    let mut p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);

    let inner = Arc::new(Measurement {
        size: Size { width: 100.0, height: 30.0 },
        gap_after: 0.0,
        content: MeasuredContent::Container {
            children: vec![],
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
        },
        alignment: Alignment::End,
    });
    let wrapper = Arc::new(Measurement {
        size: Size { width: 200.0, height: 30.0 },
        gap_after: 0.0,
        content: MeasuredContent::Container {
            children: vec![ChildMeasurement {
                node_id: NodeId::new(),
                measurement: inner,
            }],
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
        },
        alignment: Alignment::Start,
    });

    p.place(NodeId::new(), &wrapper);
    let pages = p.finish();

    let Fragment::Container(container) = &pages[0].fragments[0] else { panic!() };
    let Fragment::Container(child) = &container.children[0] else { panic!() };
    assert_eq!(child.rect.x, 100.0, "End alignment: x = container_width - child_width");
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-view alignment_end_positions_child_at_right`
Expected: FAIL — child.rect.x가 0.0

- [ ] **Step 3: place()에서 alignment 반영**

Vertical container의 자식 배치 시 x 좌표를 alignment에 따라 결정. place() 내에서 child를 배치한 후 x를 조정하는 대신, Atom/Line/빈Container fragment 생성 시 x를 계���:

기존 `self.current_x()`를 alignment-aware로 확장:

```rust
fn child_x(&self, child_measurement: &Measurement) -> f32 {
    let base_x = self.current_x();
    let container_width = self.container_stack.last()
        .map_or(self.width, |c| c.width - c.padding.left - c.padding.right);

    match child_measurement.alignment {
        Alignment::Start => base_x,
        Alignment::Center => {
            base_x + (container_width - child_measurement.size.width) / 2.0
        }
        Alignment::End => {
            base_x + container_width - child_measurement.size.width
        }
    }
}
```

place() 내 빈 Container, Atom, place_line 등에서 x 좌표를 `self.child_x(measurement)`로:

빈 Container:
```rust
LayoutDirection::Vertical if children.is_empty() => {
    // ...
    let fragment = Fragment::Container(ContainerFragment {
        node_id,
        rect: Rect {
            x: self.child_x(measurement),
            // ...
```

Atom:
```rust
MeasuredContent::Atom { parent_id, index } => {
    // ...
    let fragment = Fragment::Atom(AtomFragment {
        // ...
        rect: Rect {
            x: self.child_x(measurement),
            // ...
```

TextBlock의 open_container 후 line 배치: line에는 alignment이 적용되지 않으므로 current_x() 유지.

Vertical container with children: children은 자체 measurement의 alignment을 갖고 있으므로, 각 child의 place() 호출 시 자동으로 child_x()가 적용됨.

- [ ] **Step 4: 테스트 실행**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 PASS

---

### Task 4: resolve_inherited + resolve_gap_after

**Files:**
- Create: `crates/editor-view/src/engine/resolve.rs`
- Modify: `crates/editor-view/src/engine/mod.rs` (모듈 선언)

- [ ] **Step 1: 테스트 작성**

```rust
// crates/editor-view/src/engine/resolve.rs

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::*;

    #[test]
    fn resolve_inherited_finds_on_self() {
        let node_id = NodeId::new();
        let doc = Doc::default().insert_node(
            node_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode { align: TextAlign::Left }))
                .with_parent(NodeId::ROOT)
                .with_modifiers(vec![Modifier::BlockGap(200)]),
        );
        let node_ref = doc.node(node_id).unwrap();
        let result = resolve_inherited(&node_ref, ModifierType::BlockGap);
        assert!(matches!(result, Some(Modifier::BlockGap(200))));
    }

    #[test]
    fn resolve_inherited_walks_up_to_ancestor() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();
        let doc = Doc::default()
            .with_node_updated(NodeId::ROOT, |e| e.with_modifiers(vec![Modifier::BlockGap(150)]))
            .insert_node(
                para_id,
                NodeEntry::new(Node::Paragraph(ParagraphNode { align: TextAlign::Left }))
                    .with_parent(NodeId::ROOT),
            )
            .insert_node(
                text_id,
                NodeEntry::new(Node::Text(TextNode { text: "hi".into() }))
                    .with_parent(para_id),
            );
        let node_ref = doc.node(text_id).unwrap();
        let result = resolve_inherited(&node_ref, ModifierType::BlockGap);
        assert!(matches!(result, Some(Modifier::BlockGap(150))));
    }

    #[test]
    fn resolve_inherited_returns_none_when_absent() {
        let node_id = NodeId::new();
        let doc = Doc::default().insert_node(
            node_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode { align: TextAlign::Left }))
                .with_parent(NodeId::ROOT),
        );
        let node_ref = doc.node(node_id).unwrap();
        let result = resolve_inherited(&node_ref, ModifierType::BlockGap);
        assert!(result.is_none());
    }

    #[test]
    fn resolve_gap_after_converts_block_gap() {
        let node_id = NodeId::new();
        let doc = Doc::default()
            .with_node_updated(NodeId::ROOT, |e| e.with_modifiers(vec![Modifier::BlockGap(100)]));
        let doc = doc.insert_node(
            node_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode { align: TextAlign::Left }))
                .with_parent(NodeId::ROOT),
        );
        let node_ref = doc.node(node_id).unwrap();
        assert_eq!(resolve_gap_after(&node_ref), 16.0); // 100 / 100.0 * 16.0
    }

    #[test]
    fn resolve_gap_after_returns_zero_when_no_block_gap() {
        let node_id = NodeId::new();
        let doc = Doc::default().insert_node(
            node_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode { align: TextAlign::Left }))
                .with_parent(NodeId::ROOT),
        );
        let node_ref = doc.node(node_id).unwrap();
        assert_eq!(resolve_gap_after(&node_ref), 0.0);
    }
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-view resolve_inherited`
Expected: FAIL — 함수 미정의

- [ ] **Step 3: resolve.rs 구현**

```rust
// crates/editor-view/src/engine/resolve.rs
use editor_model::{Modifier, ModifierType, NodeRef};

pub fn resolve_inherited<'a>(
    node_ref: &NodeRef<'a>,
    modifier_type: ModifierType,
) -> Option<&'a Modifier> {
    node_ref
        .modifiers()
        .iter()
        .find(|m| ModifierType::from(*m) == modifier_type)
        .or_else(|| {
            node_ref
                .parent()
                .and_then(|p| resolve_inherited(&p, modifier_type))
        })
}

const BLOCK_GAP_BASE_PX: f32 = 16.0;

pub fn resolve_gap_after(node_ref: &NodeRef<'_>) -> f32 {
    match resolve_inherited(node_ref, ModifierType::BlockGap) {
        Some(Modifier::BlockGap(v)) => *v as f32 / 100.0 * BLOCK_GAP_BASE_PX,
        _ => 0.0,
    }
}
```

- [ ] **Step 4: engine/mod.rs에 모듈 선언 추가**

```rust
mod cache;
mod paginator;
pub(crate) mod resolve;  // 추가
```

- [ ] **Step 5: 테스트 실행**

Run: `cargo test -p editor-view resolve`
Expected: 5개 테스트 PASS

참고: `ModifierType::from(&modifier)` 변환이 strum의 `EnumDiscriminants`에 의해 자동 생성되어 있음. `==` 비교도 derive(PartialEq)로 가능.

---

### Task 5: measure_atom

**Files:**
- Create: `crates/editor-view/src/engine/measure_nodes.rs`
- Modify: `crates/editor-view/src/engine/mod.rs` (모듈 선언)

- [ ] **Step 1: 테스트 작성**

```rust
// crates/editor-view/src/engine/measure_nodes.rs 끝에

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::*;

    #[test]
    fn measure_atom_horizontal_rule() {
        let hr_id = NodeId::new();
        let doc = Doc::default().insert_node(
            hr_id,
            NodeEntry::new(Node::HorizontalRule(HorizontalRuleNode {}))
                .with_parent(NodeId::ROOT),
        );
        let node_ref = doc.node(hr_id).unwrap();
        let result = measure_atom(&node_ref, 300.0, &ViewState::new());

        assert_eq!(result.size.width, 300.0);
        assert_eq!(result.size.height, 24.0);
        assert!(matches!(result.content, MeasuredContent::Atom { parent_id, index } if parent_id == NodeId::ROOT && index == 0));
    }

    #[test]
    fn measure_atom_image_with_proportion_and_external_height() {
        let img_id = NodeId::new();
        let doc = Doc::default().insert_node(
            img_id,
            NodeEntry::new(Node::Image(ImageNode { id: None, proportion: 0.5 }))
                .with_parent(NodeId::ROOT),
        );
        let node_ref = doc.node(img_id).unwrap();
        let mut vs = ViewState::new();
        vs.external_heights.insert(img_id, 200.0);

        let result = measure_atom(&node_ref, 400.0, &vs);
        assert_eq!(result.size.width, 200.0); // 0.5 * 400
        assert_eq!(result.size.height, 200.0);
    }

    #[test]
    fn measure_atom_image_without_external_height() {
        let img_id = NodeId::new();
        let doc = Doc::default().insert_node(
            img_id,
            NodeEntry::new(Node::Image(ImageNode { id: None, proportion: 0.8 }))
                .with_parent(NodeId::ROOT),
        );
        let node_ref = doc.node(img_id).unwrap();
        let result = measure_atom(&node_ref, 400.0, &ViewState::new());
        assert_eq!(result.size.width, 320.0); // 0.8 * 400
        assert_eq!(result.size.height, 0.0);
    }

    #[test]
    fn measure_atom_file_with_external_height() {
        let file_id = NodeId::new();
        let doc = Doc::default().insert_node(
            file_id,
            NodeEntry::new(Node::File(FileNode::default()))
                .with_parent(NodeId::ROOT),
        );
        let node_ref = doc.node(file_id).unwrap();
        let mut vs = ViewState::new();
        vs.external_heights.insert(file_id, 48.0);

        let result = measure_atom(&node_ref, 300.0, &vs);
        assert_eq!(result.size.width, 300.0);
        assert_eq!(result.size.height, 48.0);
    }
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-view measure_atom`
Expected: FAIL — 함수 미정의

- [ ] **Step 3: measure_atom 구현**

```rust
// crates/editor-view/src/engine/measure_nodes.rs
use editor_common::Size;
use editor_model::{Node, NodeRef};

use crate::measure::*;
use crate::view_state::ViewState;

const HORIZONTAL_RULE_HEIGHT: f32 = 24.0;

pub fn measure_atom(
    node_ref: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let node_id = node_ref.id();
    let parent_id = node_ref.parent().expect("atom must have parent").id();
    let index = node_ref.index().expect("atom must have index");

    let (w, h) = match node_ref.node() {
        Node::Image(img) => {
            let w = img.proportion * width;
            let h = view_state.external_height(node_id).unwrap_or(0.0);
            (w, h)
        }
        Node::HorizontalRule(_) => (width, HORIZONTAL_RULE_HEIGHT),
        _ => {
            // File, Embed, Archived
            let h = view_state.external_height(node_id).unwrap_or(0.0);
            (width, h)
        }
    };

    Measurement {
        size: Size { width: w, height: h },
        gap_after: 0.0,
        content: MeasuredContent::Atom { parent_id, index },
        alignment: Alignment::Start,
    }
}
```

- [ ] **Step 4: engine/mod.rs에 모듈 선언 추가**

```rust
mod cache;
mod paginator;
pub(crate) mod resolve;
pub(crate) mod measure_nodes;  // 추가
```

- [ ] **Step 5: 테스트 실행**

Run: `cargo test -p editor-view measure_atom`
Expected: 4개 테스트 PASS

---

### Task 6: measure_default_container

**Files:**
- Modify: `crates/editor-view/src/engine/measure_nodes.rs`

- [ ] **Step 1: 테스트 작성**

```rust
// measure_nodes.rs 테스트 모듈에 추가

#[test]
fn measure_default_container_sums_children() {
    let para_id = NodeId::new();
    let text_id = NodeId::new();
    let doc = Doc::default()
        .insert_node(
            para_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode { align: TextAlign::Left }))
                .with_parent(NodeId::ROOT),
        )
        .insert_node(
            text_id,
            NodeEntry::new(Node::Text(TextNode { text: "hello".into() }))
                .with_parent(para_id),
        );
    let node_ref = doc.node(para_id).unwrap();
    let mut engine = LayoutEngine::new();
    let result = measure_default_container(&mut engine, &doc, &node_ref, 300.0, &ViewState::new());

    assert!(matches!(result.content, MeasuredContent::Container { direction: LayoutDirection::Vertical, .. }));
    assert_eq!(result.alignment, Alignment::Start);
    assert_eq!(result.size.width, 300.0);
}

#[test]
fn measure_default_container_resolves_gap_after() {
    let para_id = NodeId::new();
    let doc = Doc::default()
        .with_node_updated(NodeId::ROOT, |e| e.with_modifiers(vec![Modifier::BlockGap(200)]))
        .insert_node(
            para_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode { align: TextAlign::Left }))
                .with_parent(NodeId::ROOT),
        );
    let node_ref = doc.node(para_id).unwrap();
    let mut engine = LayoutEngine::new();
    let result = measure_default_container(&mut engine, &doc, &node_ref, 300.0, &ViewState::new());

    assert_eq!(result.gap_after, 32.0); // 200 / 100 * 16.0
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-view measure_default_container`
Expected: FAIL

- [ ] **Step 3: measure_default_container 구현**

```rust
// measure_nodes.rs에 추가

use editor_model::{Doc, Node, NodeRef};
use super::LayoutEngine;
use super::resolve::resolve_gap_after;

pub fn measure_default_container(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node_ref: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let children: Vec<ChildMeasurement> = node_ref
        .children()
        .map(|child| {
            let m = engine.measure(doc, child.id(), width, view_state);
            ChildMeasurement {
                node_id: child.id(),
                measurement: m,
            }
        })
        .collect();

    let height: f32 = children.iter().map(|c| c.measurement.size.height).sum();

    let scope = matches!(node_ref.node(), Node::TableCell(_));
    let direction = if matches!(node_ref.node(), Node::TableRow(_)) {
        LayoutDirection::Horizontal
    } else {
        LayoutDirection::Vertical
    };

    let gap_after = if matches!(node_ref.node(), Node::Root(_) | Node::Text(_)) {
        0.0
    } else {
        resolve_gap_after(node_ref)
    };

    Measurement {
        size: Size { width, height: height.max(0.0) },
        gap_after,
        content: MeasuredContent::Container {
            children,
            scope,
            direction,
            padding: EdgeInsets::ZERO,
        },
        alignment: Alignment::Start,
    }
}
```

참고: `engine.measure()`는 현재 `fn measure(&mut self, ...)` — pub(crate)가 아닌 private. measure_nodes에서 호출하려면 `pub(crate)`로 변경 필요. engine/mod.rs의 `fn measure` 시그니처를 `pub(crate) fn measure`로 수정.

- [ ] **Step 4: engine/mod.rs에서 measure를 pub(crate)로 변경**

```rust
pub(crate) fn measure(
    &mut self,
    doc: &Doc,
    node_id: NodeId,
    width: f32,
    view_state: &ViewState,
) -> Arc<Measurement> {
```

- [ ] **Step 5: 테스트 실행**

Run: `cargo test -p editor-view measure_default_container`
Expected: PASS

---

### Task 7: measure_inner dispatch 재작성

**Files:**
- Modify: `crates/editor-view/src/engine/mod.rs`

- [ ] **Step 1: measure_inner를 dispatch로 재작성**

engine/mod.rs의 `measure_inner` 전체 교체:

```rust
fn measure_inner(
    &mut self,
    doc: &Doc,
    node_ref: &editor_model::NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    match node_ref.node() {
        Node::Image(_) | Node::File(_) | Node::Embed(_)
        | Node::Archived(_) | Node::HorizontalRule(_) => {
            measure_nodes::measure_atom(node_ref, width, view_state)
        }

        Node::PageBreak(_) => Measurement {
            size: Size { width, height: 0.0 },
            gap_after: 0.0,
            content: MeasuredContent::PageBreak,
            alignment: Alignment::Start,
        },

        _ => measure_nodes::measure_default_container(self, doc, node_ref, width, view_state),
    }
}
```

- [ ] **Step 2: 테스트 실행**

Run: `cargo test -p editor-view`
Expected: 모든 기존 테스트 PASS

---

### Task 8: measure_list_item

**Files:**
- Modify: `crates/editor-view/src/engine/measure_nodes.rs`
- Modify: `crates/editor-view/src/engine/mod.rs` (dispatch 추가)

- [ ] **Step 1: 테스트 작성**

```rust
#[test]
fn measure_list_item_applies_left_indent() {
    let list_id = NodeId::new();
    let item_id = NodeId::new();
    let para_id = NodeId::new();
    let doc = Doc::default()
        .insert_node(
            list_id,
            NodeEntry::new(Node::BulletList(BulletListNode {}))
                .with_parent(NodeId::ROOT),
        )
        .insert_node(
            item_id,
            NodeEntry::new(Node::ListItem(ListItemNode {}))
                .with_parent(list_id),
        )
        .insert_node(
            para_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode { align: TextAlign::Left }))
                .with_parent(item_id),
        );
    let node_ref = doc.node(item_id).unwrap();
    let mut engine = LayoutEngine::new();
    let result = measure_list_item(&mut engine, &doc, &node_ref, 300.0, &ViewState::new());

    let MeasuredContent::Container { padding, .. } = &result.content else { panic!() };
    assert_eq!(padding.left, 28.0); // MARKER_WIDTH(20) + MARKER_GAP(8)
    assert_eq!(result.size.width, 300.0);
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-view measure_list_item`
Expected: FAIL

- [ ] **Step 3: measure_list_item 구현**

```rust
// measure_nodes.rs에 추가

const LIST_ITEM_MARKER_WIDTH: f32 = 20.0;
const LIST_ITEM_MARKER_GAP: f32 = 8.0;

pub fn measure_list_item(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node_ref: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let padding = EdgeInsets {
        left: LIST_ITEM_MARKER_WIDTH + LIST_ITEM_MARKER_GAP,
        ..EdgeInsets::ZERO
    };
    measure_padded_container(engine, doc, node_ref, width, view_state, padding, Alignment::Start)
}

fn measure_padded_container(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node_ref: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
    padding: EdgeInsets,
    alignment: Alignment,
) -> Measurement {
    let content_width = width - padding.left - padding.right;

    let children: Vec<ChildMeasurement> = node_ref
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

    let height = padding.top + children_height + padding.bottom;

    Measurement {
        size: Size { width, height },
        gap_after: resolve_gap_after(node_ref),
        content: MeasuredContent::Container {
            children,
            scope: false,
            direction: LayoutDirection::Vertical,
            padding,
        },
        alignment,
    }
}
```

- [ ] **Step 4: measure_inner dispatch에 ListItem 추가**

engine/mod.rs의 measure_inner:
```rust
Node::ListItem(_) => {
    measure_nodes::measure_list_item(self, doc, node_ref, width, view_state)
}
```

(`_ =>` 위에 추가)

- [ ] **Step 5: 테스트 실행**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 PASS

---

### Task 9: measure_blockquote

**Files:**
- Modify: `crates/editor-view/src/engine/measure_nodes.rs`
- Modify: `crates/editor-view/src/engine/mod.rs` (dispatch 추가)

- [ ] **Step 1: 테스트 작성**

```rust
#[test]
fn measure_blockquote_left_line() {
    let bq_id = NodeId::new();
    let doc = Doc::default().insert_node(
        bq_id,
        NodeEntry::new(Node::Blockquote(BlockquoteNode {
            variant: BlockquoteVariant::LeftLine,
        }))
        .with_parent(NodeId::ROOT),
    );
    let node_ref = doc.node(bq_id).unwrap();
    let mut engine = LayoutEngine::new();
    let result = measure_blockquote(&mut engine, &doc, &node_ref, 300.0, &ViewState::new());

    let MeasuredContent::Container { padding, .. } = &result.content else { panic!() };
    assert_eq!(padding.left, 20.0);
    assert_eq!(result.alignment, Alignment::Start);
}

#[test]
fn measure_blockquote_left_quote() {
    let bq_id = NodeId::new();
    let doc = Doc::default().insert_node(
        bq_id,
        NodeEntry::new(Node::Blockquote(BlockquoteNode {
            variant: BlockquoteVariant::LeftQuote,
        }))
        .with_parent(NodeId::ROOT),
    );
    let node_ref = doc.node(bq_id).unwrap();
    let mut engine = LayoutEngine::new();
    let result = measure_blockquote(&mut engine, &doc, &node_ref, 300.0, &ViewState::new());

    let MeasuredContent::Container { padding, .. } = &result.content else { panic!() };
    assert_eq!(padding.left, 32.0);
}

#[test]
fn measure_blockquote_message_sent_alignment_end() {
    let bq_id = NodeId::new();
    let doc = Doc::default().insert_node(
        bq_id,
        NodeEntry::new(Node::Blockquote(BlockquoteNode {
            variant: BlockquoteVariant::MessageSent,
        }))
        .with_parent(NodeId::ROOT),
    );
    let node_ref = doc.node(bq_id).unwrap();
    let mut engine = LayoutEngine::new();
    let result = measure_blockquote(&mut engine, &doc, &node_ref, 300.0, &ViewState::new());

    assert_eq!(result.alignment, Alignment::End);
    assert_eq!(result.size.width, 240.0); // 300 * 0.8
    let MeasuredContent::Container { padding, .. } = &result.content else { panic!() };
    assert_eq!(padding.left, 14.0);
    assert_eq!(padding.right, 14.0);
    assert_eq!(padding.top, 8.0);
    assert_eq!(padding.bottom, 8.0);
}

#[test]
fn measure_blockquote_message_min_width() {
    let bq_id = NodeId::new();
    let doc = Doc::default().insert_node(
        bq_id,
        NodeEntry::new(Node::Blockquote(BlockquoteNode {
            variant: BlockquoteVariant::MessageSent,
        }))
        .with_parent(NodeId::ROOT),
    );
    let node_ref = doc.node(bq_id).unwrap();
    let mut engine = LayoutEngine::new();
    // width=30 → 30*0.8=24 < 40 → clamp to min(40, 30)=30
    let result = measure_blockquote(&mut engine, &doc, &node_ref, 30.0, &ViewState::new());
    assert_eq!(result.size.width, 30.0);
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-view measure_blockquote`
Expected: FAIL

- [ ] **Step 3: measure_blockquote 구현**

```rust
// measure_nodes.rs에 추가

const BQ_LINE_WIDTH: f32 = 4.0;
const BQ_CONTENT_PADDING: f32 = 16.0;
const BQ_QUOTE_SIZE: f32 = 16.0;
const BQ_QUOTE_CONTENT_GAP: f32 = 16.0;
const BQ_MESSAGE_PADDING_X: f32 = 14.0;
const BQ_MESSAGE_PADDING_Y: f32 = 8.0;
const BQ_MESSAGE_MAX_WIDTH_RATIO: f32 = 0.8;
const BQ_MESSAGE_MIN_WIDTH: f32 = 40.0;

pub fn measure_blockquote(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node_ref: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let Node::Blockquote(bq) = node_ref.node() else {
        unreachable!()
    };

    match bq.variant {
        BlockquoteVariant::LeftLine => {
            let padding = EdgeInsets {
                left: BQ_LINE_WIDTH + BQ_CONTENT_PADDING,
                ..EdgeInsets::ZERO
            };
            measure_padded_container(engine, doc, node_ref, width, view_state, padding, Alignment::Start)
        }
        BlockquoteVariant::LeftQuote => {
            let padding = EdgeInsets {
                left: BQ_QUOTE_SIZE + BQ_QUOTE_CONTENT_GAP,
                ..EdgeInsets::ZERO
            };
            measure_padded_container(engine, doc, node_ref, width, view_state, padding, Alignment::Start)
        }
        BlockquoteVariant::MessageSent | BlockquoteVariant::MessageReceived => {
            let bubble_width = (width * BQ_MESSAGE_MAX_WIDTH_RATIO)
                .max(BQ_MESSAGE_MIN_WIDTH)
                .min(width);
            let padding = EdgeInsets {
                top: BQ_MESSAGE_PADDING_Y,
                left: BQ_MESSAGE_PADDING_X,
                bottom: BQ_MESSAGE_PADDING_Y,
                right: BQ_MESSAGE_PADDING_X,
            };
            let alignment = if bq.variant == BlockquoteVariant::MessageSent {
                Alignment::End
            } else {
                Alignment::Start
            };
            measure_padded_container(engine, doc, node_ref, bubble_width, view_state, padding, alignment)
        }
    }
}
```

- [ ] **Step 4: measure_inner dispatch에 Blockquote 추가**

engine/mod.rs의 measure_inner:
```rust
Node::Blockquote(_) => {
    measure_nodes::measure_blockquote(self, doc, node_ref, width, view_state)
}
```

- [ ] **Step 5: 테스트 실행**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 PASS

---

### Task 10: measure_callout

**Files:**
- Modify: `crates/editor-view/src/engine/measure_nodes.rs`
- Modify: `crates/editor-view/src/engine/mod.rs` (dispatch 추가)

- [ ] **Step 1: 테스트 작성**

```rust
#[test]
fn measure_callout_padding() {
    let callout_id = NodeId::new();
    let doc = Doc::default().insert_node(
        callout_id,
        NodeEntry::new(Node::Callout(CalloutNode::default()))
            .with_parent(NodeId::ROOT),
    );
    let node_ref = doc.node(callout_id).unwrap();
    let mut engine = LayoutEngine::new();
    let result = measure_callout(&mut engine, &doc, &node_ref, 300.0, &ViewState::new());

    let MeasuredContent::Container { padding, .. } = &result.content else { panic!() };
    assert_eq!(padding.top, 16.0);
    assert_eq!(padding.left, 40.0);  // 12 + 20 + 8
    assert_eq!(padding.bottom, 16.0);
    assert_eq!(padding.right, 12.0);
    assert_eq!(result.size.height, 32.0); // padding.top + padding.bottom (no children)
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-view measure_callout`
Expected: FAIL

- [ ] **Step 3: measure_callout 구현**

```rust
// measure_nodes.rs에 추가

const CALLOUT_PADDING_X: f32 = 12.0;
const CALLOUT_PADDING_Y: f32 = 16.0;
const CALLOUT_ICON_WIDTH: f32 = 20.0;
const CALLOUT_ICON_CONTENT_GAP: f32 = 8.0;

pub fn measure_callout(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node_ref: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let padding = EdgeInsets {
        top: CALLOUT_PADDING_Y,
        left: CALLOUT_PADDING_X + CALLOUT_ICON_WIDTH + CALLOUT_ICON_CONTENT_GAP,
        bottom: CALLOUT_PADDING_Y,
        right: CALLOUT_PADDING_X,
    };
    measure_padded_container(engine, doc, node_ref, width, view_state, padding, Alignment::Start)
}
```

- [ ] **Step 4: measure_inner dispatch에 Callout 추가**

engine/mod.rs의 measure_inner:
```rust
Node::Callout(_) => {
    measure_nodes::measure_callout(self, doc, node_ref, width, view_state)
}
```

- [ ] **Step 5: 테스트 실행**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 PASS

---

### Task 11: 통합 검증

**Files:**
- Modify: `crates/editor-view/src/engine/mod.rs` (통합 테스트)

- [ ] **Step 1: 통합 테스트 — atom이 포함된 문서 전체 layout**

engine/mod.rs 테스트에 추가:

```rust
#[test]
fn compute_layout_with_atom_and_container() {
    use editor_model::*;

    let hr_id = NodeId::new();
    let list_id = NodeId::new();
    let item_id = NodeId::new();
    let para_id = NodeId::new();

    let doc = Doc::default()
        .with_node_updated(NodeId::ROOT, |e| e.with_modifiers(vec![Modifier::BlockGap(100)]))
        .insert_node(
            hr_id,
            NodeEntry::new(Node::HorizontalRule(HorizontalRuleNode {}))
                .with_parent(NodeId::ROOT),
        )
        .insert_node(
            list_id,
            NodeEntry::new(Node::BulletList(BulletListNode {}))
                .with_parent(NodeId::ROOT),
        )
        .insert_node(
            item_id,
            NodeEntry::new(Node::ListItem(ListItemNode {}))
                .with_parent(list_id),
        )
        .insert_node(
            para_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode { align: TextAlign::Left }))
                .with_parent(item_id),
        );

    let mut engine = LayoutEngine::new();
    let viewport = Viewport { width: 400.0, scale_factor: 1.0 };
    let view_state = ViewState::new();
    engine.compute(&doc, &viewport, &view_state);

    let pages = engine.pages();
    assert!(!pages.is_empty());

    // HR should be an Atom fragment
    let has_atom = pages[0].fragments.iter().any(|f| matches!(f, Fragment::Atom(_)));
    assert!(has_atom, "should have atom fragment for HR");
}
```

- [ ] **Step 2: 테스트 실행**

Run: `cargo test -p editor-view compute_layout_with_atom_and_container`
Expected: PASS

- [ ] **Step 3: 전체 crate 테스트 + 컴파일 확인**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 PASS

Run: `cargo build -p editor-view`
Expected: 빌드 성공, 경고 없음

- [ ] **Step 4: 관련 crate 빌드 확인**

Run: `cargo build -p editor-core`
Expected: 빌드 성공 (editor-core가 editor-view에 의존하므로 타입 변경 영향 확인)
