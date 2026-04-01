# 커서 Movement 확장 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Movement enum을 재설계하고, Word/Sentence/LineStart/LineEnd/Block/Page/Document 커서 이동을 구현한다.

**Architecture:** Movement enum을 의미적/시각적 이동으로 재설계. TextSegmenters(WordSegmenter + SentenceSegmenter)는 editor-core의 Editor가 소유하고 resolve_movement 호출 시 전달. boundary.rs에 Word/Sentence 경계 탐색 구현. 나머지는 기존 search 함수 재사용.

**Tech Stack:** icu_segmenter 2.1 (Word/Sentence 경계), editor-common (Movement 타입), editor-view (cursor 모듈)

**설계 문서:** `docs/editor-architecture/view-cursor-movement-design.md`

**주의:** 이 프로젝트는 git commit을 에이전트가 직접 수행하지 않습니다.

---

## 파일 구조

| 동작 | 파일 | 역할 |
|------|------|------|
| Modify | `crates/editor-common/src/movement.rs` | Movement, Direction enum 재설계 |
| Modify | `crates/editor-view/src/viewport.rs` | Viewport에 height 필드 추가 |
| Modify | `crates/editor-view/Cargo.toml` | icu_segmenter 의존성 추가 |
| Create | `crates/editor-view/src/cursor/boundary.rs` | TextSegmenters 정의 + Word/Sentence 경계 탐색 + 줄 내 Position 변환 |
| Modify | `crates/editor-view/src/cursor/navigation.rs` | resolve_movement 시그니처 변경 + 새 movement 함수들 |
| Modify | `crates/editor-view/src/cursor/search.rs` | find_scope_container_at 추가 (Block용) |
| Modify | `crates/editor-view/src/cursor/mod.rs` | boundary 모듈 선언, re-export 수정 |
| Modify | `crates/editor-view/src/view.rs` | resolve_movement에 segmenters 파라미터 추가 |
| Modify | `crates/editor-core/src/message.rs` | Movement 사용처 수정 |
| Modify | `crates/editor-core/src/editor.rs` | TextSegmenters 소유, handle_navigate 수정 |
| Modify | `crates/editor-core/src/handle.rs` | Movement 사용처 수정 |

---

## Task 1: Movement enum 재설계 + Viewport height

**Files:**
- Modify: `crates/editor-common/src/movement.rs`
- Modify: `crates/editor-view/src/viewport.rs`

- [ ] **Step 1: Movement enum 재설계**

`crates/editor-common/src/movement.rs` 전체를 교체:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    // 의미적 이동 — 문서 모델만으로 결정 가능
    Grapheme(Direction),
    Word(Direction),
    Sentence(Direction),
    Block(Direction),
    Document(Direction),

    // 시각적 이동 — 레이아웃에 의존
    LineStart,
    LineEnd,
    LineUp,
    LineDown,
    PageUp,
    PageDown,
}
```

- [ ] **Step 2: Viewport에 height 추가**

`crates/editor-view/src/viewport.rs`:

```rust
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub scale_factor: f64,
}

impl Viewport {
    pub fn new(width: f32, height: f32, scale_factor: f64) -> Self {
        Self { width, height, scale_factor }
    }
}
```

- [ ] **Step 3: 사용처 수정 — 전체 crate**

**editor-view/src/cursor/navigation.rs** — resolve_movement dispatch 수정:

```rust
use crate::cursor::boundary::TextSegmenters;
use crate::viewport::Viewport;

pub fn resolve_movement(
    pages: &[Page],
    pos: &Position,
    movement: &Movement,
    viewport: &Viewport,
    segmenters: Option<&TextSegmenters>,
) -> Option<Selection> {
    match movement {
        Movement::Grapheme(Direction::Forward) => move_right(pages, pos),
        Movement::Grapheme(Direction::Backward) => move_left(pages, pos),
        Movement::LineDown => move_down(pages, pos),
        Movement::LineUp => move_up(pages, pos),
        _ => None, // 나머지는 이후 Task에서 구현
    }
}
```

`Movement::Line(Direction::Forward)` → `Movement::LineDown`, `Movement::Line(Direction::Backward)` → `Movement::LineUp` 으로 변경.

**editor-view/src/cursor/mod.rs** — re-export 확인, boundary 모듈 선언은 아직 하지 않음 (Task 5에서).

**editor-view/src/view.rs** — 시그니처 변경:

```rust
pub fn resolve_movement(
    &self,
    pos: &Position,
    movement: &Movement,
    segmenters: Option<&TextSegmenters>,
) -> Option<Selection> {
    cursor::resolve_movement(self.engine.pages(), pos, movement, &self.viewport, segmenters)
}
```

`View::new`에 height 파라미터 추가: `View::new(width, height, scale_factor)`. `View::resize`에도 height 추가.

**editor-core/src/message.rs** — `Movement::Grapheme(Direction::Forward)` 등은 동일. `Movement::Word(Direction::Backward)` 등도 동일. 변경 없을 수 있음.

**editor-core/src/handle.rs** — 동일하게 확인.

**editor-core/src/editor.rs** — `handle_navigate`의 Movement 타입은 자동으로 새 enum 사용.

**editor-view 테스트 전체** — `Viewport { width: 400.0, scale_factor: 1.0 }` → `Viewport { width: 400.0, height: 800.0, scale_factor: 1.0 }`. `Movement::Line(Direction::Forward)` → `Movement::LineDown` 등. `resolve_movement` 호출에 `&viewport, None` 파라미터 추가.

모든 `Viewport::new` 및 `Viewport { ... }` 사용처를 crate 전체에서 검색하여 수정한다. 모든 `resolve_movement` 호출처도 검색하여 시그니처에 맞게 수정한다.

- [ ] **Step 4: 컴파일 및 테스트**

Run: `cargo check -p editor-common -p editor-view -p editor-core`
Run: `cargo test -p editor-view`
Expected: 모든 기존 테스트 통과.

---

## Task 2: LineStart / LineEnd

**Files:**
- Modify: `crates/editor-view/src/cursor/navigation.rs`

- [ ] **Step 1: 테스트 작성**

`navigation.rs`의 `mod tests`에 추가. 기존 `two_line_page()` fixture 재사용:

```rust
#[test]
fn line_end_moves_to_end_of_line() {
    let (page, id1, _) = two_line_page();
    let pages = [page];
    let viewport = Viewport { width: 200.0, height: 800.0, scale_factor: 1.0 };
    let sel = resolve_movement(
        &pages,
        &Position::new(id1, 2),
        &Movement::LineEnd,
        &viewport,
        None,
    ).unwrap();

    assert_eq!(sel.head.node_id, id1);
    assert_eq!(sel.head.offset, 5); // "hello" 끝
}

#[test]
fn line_start_moves_to_start_of_line() {
    let (page, id1, _) = two_line_page();
    let pages = [page];
    let viewport = Viewport { width: 200.0, height: 800.0, scale_factor: 1.0 };
    let sel = resolve_movement(
        &pages,
        &Position::new(id1, 3),
        &Movement::LineStart,
        &viewport,
        None,
    ).unwrap();

    assert_eq!(sel.head.node_id, id1);
    assert_eq!(sel.head.offset, 0);
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p editor-view line_end_moves -- --exact`
Expected: FAIL (None 반환).

- [ ] **Step 3: 구현**

`navigation.rs`에 LineFragment용 헬퍼 추출 (기존 `first_position_in`/`last_position_in` 리팩토링):

```rust
fn first_position_in_line(line: &LineFragment) -> Position {
    if let Some(seg) = line.segments.first() {
        Position::new(seg.node_id, seg.offset)
    } else {
        Position::new(line.node_id, 0)
    }
}

fn last_position_in_line(line: &LineFragment) -> Position {
    if let Some(seg) = line.segments.last() {
        Position::new(seg.node_id, seg.offset + seg.char_advances.len())
    } else {
        Position::new(line.node_id, 0)
    }
}
```

기존 `first_position_in`/`last_position_in`의 Line 분기를 이 헬퍼로 위임한다.

이동 함수:

```rust
fn move_line_start(pages: &[Page], pos: &Position) -> Option<Selection> {
    let (_, line) = search::find_line_at(pages, pos)?;
    Some(Selection::collapsed(first_position_in_line(line)))
}

fn move_line_end(pages: &[Page], pos: &Position) -> Option<Selection> {
    let (_, line) = search::find_line_at(pages, pos)?;
    Some(Selection::collapsed(last_position_in_line(line)))
}
```

dispatch에 추가:

```rust
Movement::LineStart => move_line_start(pages, pos),
Movement::LineEnd => move_line_end(pages, pos),
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모두 PASS.

---

## Task 3: Block / Document

**Files:**
- Modify: `crates/editor-view/src/cursor/navigation.rs`
- Modify: `crates/editor-view/src/cursor/search.rs`

- [ ] **Step 1: 테스트 작성**

`navigation.rs` 테스트에 scope container fixture 추가:

```rust
fn two_block_page() -> (Page, NodeId, NodeId, NodeId) {
    let line1_id = NodeId::new();
    let line2_id = NodeId::new();
    let line3_id = NodeId::new();
    let page = Page::new(
        vec![
            Fragment::Container(ContainerFragment {
                node_id: NodeId::new(),
                rect: Rect { x: 0.0, y: 0.0, width: 200.0, height: 40.0 },
                children: vec![
                    Fragment::Line(LineFragment {
                        node_id: line1_id,
                        rect: Rect { x: 0.0, y: 0.0, width: 200.0, height: 20.0 },
                        baseline: 16.0,
                        segments: vec![LineSegment {
                            node_id: line1_id, offset: 0, text: "hello".into(),
                            x: 0.0, width: 50.0, char_advances: vec![10.0; 5],
                        }],
                    }),
                    Fragment::Line(LineFragment {
                        node_id: line2_id,
                        rect: Rect { x: 0.0, y: 20.0, width: 200.0, height: 20.0 },
                        baseline: 16.0,
                        segments: vec![LineSegment {
                            node_id: line2_id, offset: 0, text: "world".into(),
                            x: 0.0, width: 50.0, char_advances: vec![10.0; 5],
                        }],
                    }),
                ],
                scope: true,
                breaks: Breaks::default(),
            }),
            Fragment::Container(ContainerFragment {
                node_id: NodeId::new(),
                rect: Rect { x: 0.0, y: 40.0, width: 200.0, height: 20.0 },
                children: vec![
                    Fragment::Line(LineFragment {
                        node_id: line3_id,
                        rect: Rect { x: 0.0, y: 40.0, width: 200.0, height: 20.0 },
                        baseline: 16.0,
                        segments: vec![LineSegment {
                            node_id: line3_id, offset: 0, text: "end".into(),
                            x: 0.0, width: 30.0, char_advances: vec![10.0; 3],
                        }],
                    }),
                ],
                scope: true,
                breaks: Breaks::default(),
            }),
        ],
        800.0,
    );
    (page, line1_id, line2_id, line3_id)
}

#[test]
fn block_forward_moves_to_end_of_block() {
    let (page, line1_id, line2_id, _) = two_block_page();
    let pages = [page];
    let viewport = Viewport { width: 200.0, height: 800.0, scale_factor: 1.0 };
    let sel = resolve_movement(
        &pages, &Position::new(line1_id, 2),
        &Movement::Block(Direction::Forward), &viewport, None,
    ).unwrap();
    assert_eq!(sel.head.node_id, line2_id);
    assert_eq!(sel.head.offset, 5);
}

#[test]
fn block_backward_moves_to_start_of_block() {
    let (page, line1_id, line2_id, _) = two_block_page();
    let pages = [page];
    let viewport = Viewport { width: 200.0, height: 800.0, scale_factor: 1.0 };
    let sel = resolve_movement(
        &pages, &Position::new(line2_id, 3),
        &Movement::Block(Direction::Backward), &viewport, None,
    ).unwrap();
    assert_eq!(sel.head.node_id, line1_id);
    assert_eq!(sel.head.offset, 0);
}

#[test]
fn document_forward_moves_to_end() {
    let (page, line1_id, _, line3_id) = two_block_page();
    let pages = [page];
    let viewport = Viewport { width: 200.0, height: 800.0, scale_factor: 1.0 };
    let sel = resolve_movement(
        &pages, &Position::new(line1_id, 0),
        &Movement::Document(Direction::Forward), &viewport, None,
    ).unwrap();
    assert_eq!(sel.head.node_id, line3_id);
    assert_eq!(sel.head.offset, 3);
}

#[test]
fn document_backward_moves_to_start() {
    let (page, line1_id, _, line3_id) = two_block_page();
    let pages = [page];
    let viewport = Viewport { width: 200.0, height: 800.0, scale_factor: 1.0 };
    let sel = resolve_movement(
        &pages, &Position::new(line3_id, 2),
        &Movement::Document(Direction::Backward), &viewport, None,
    ).unwrap();
    assert_eq!(sel.head.node_id, line1_id);
    assert_eq!(sel.head.offset, 0);
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p editor-view block_forward -- --exact`
Expected: FAIL.

- [ ] **Step 3: search.rs에 find_scope_container_at 추가**

```rust
pub fn find_scope_container_at<'a>(pages: &'a [Page], pos: &Position) -> Option<&'a ContainerFragment> {
    for page in pages {
        for frag in &page.fragments {
            if let Some(container) = find_scope_containing(frag, pos) {
                return Some(container);
            }
        }
    }
    None
}

fn find_scope_containing<'a>(fragment: &'a Fragment, pos: &Position) -> Option<&'a ContainerFragment> {
    match fragment {
        Fragment::Container(c) => {
            for child in &c.children {
                if let Some(inner) = find_scope_containing(child, pos) {
                    return Some(inner);
                }
            }
            if c.scope && contains_position(fragment, pos) {
                return Some(c);
            }
            None
        }
        _ => None,
    }
}

fn contains_position(fragment: &Fragment, pos: &Position) -> bool {
    match fragment {
        Fragment::Line(line) => line.segments.iter().any(|seg| {
            seg.node_id == pos.node_id
                && pos.offset >= seg.offset
                && pos.offset <= seg.offset + seg.char_advances.len()
        }),
        Fragment::Container(c) => c.children.iter().any(|child| contains_position(child, pos)),
        Fragment::Atom(atom) => atom.node_id == pos.node_id,
    }
}
```

- [ ] **Step 4: navigation.rs에 Block/Document 구현**

```rust
fn move_block_forward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let container = search::find_scope_container_at(pages, pos)?;
    let nav = container.children.iter().rev().find_map(search::find_last_navigable)?;
    Some(Selection::collapsed(last_position_in(nav)))
}

fn move_block_backward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let container = search::find_scope_container_at(pages, pos)?;
    let nav = container.children.iter().find_map(search::find_first_navigable)?;
    Some(Selection::collapsed(first_position_in(nav)))
}

fn move_document_forward(pages: &[Page]) -> Option<Selection> {
    let page = pages.last()?;
    let nav = page.fragments.iter().rev().find_map(search::find_last_navigable)?;
    Some(Selection::collapsed(last_position_in(nav)))
}

fn move_document_backward(pages: &[Page]) -> Option<Selection> {
    let page = pages.first()?;
    let nav = page.fragments.iter().find_map(search::find_first_navigable)?;
    Some(Selection::collapsed(first_position_in(nav)))
}
```

dispatch에 추가:

```rust
Movement::Block(Direction::Forward) => move_block_forward(pages, pos),
Movement::Block(Direction::Backward) => move_block_backward(pages, pos),
Movement::Document(Direction::Forward) => move_document_forward(pages),
Movement::Document(Direction::Backward) => move_document_backward(pages),
```

- [ ] **Step 5: 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모두 PASS.

---

## Task 4: PageUp / PageDown

**Files:**
- Modify: `crates/editor-view/src/cursor/navigation.rs`

- [ ] **Step 1: 테스트 작성**

```rust
#[test]
fn page_down_moves_by_viewport_height() {
    let (page, id1, id2) = two_line_page();
    let pages = [page];
    let viewport = Viewport { width: 200.0, height: 15.0, scale_factor: 1.0 };
    let sel = resolve_movement(
        &pages, &Position::new(id1, 2),
        &Movement::PageDown, &viewport, None,
    ).unwrap();
    assert_eq!(sel.head.node_id, id2);
}

#[test]
fn page_up_moves_by_viewport_height() {
    let (page, id1, id2) = two_line_page();
    let pages = [page];
    let viewport = Viewport { width: 200.0, height: 15.0, scale_factor: 1.0 };
    let sel = resolve_movement(
        &pages, &Position::new(id2, 2),
        &Movement::PageUp, &viewport, None,
    ).unwrap();
    assert_eq!(sel.head.node_id, id1);
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p editor-view page_down -- --exact`
Expected: FAIL.

- [ ] **Step 3: 구현**

```rust
fn move_page_down(pages: &[Page], pos: &Position, viewport: &Viewport) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let preferred_x = line.rect.x + super::x_at_offset(line, pos);
    let y = line.rect.y + viewport.height;
    let (_, target) = search::find_navigable_below(pages, page_idx, y, preferred_x)?;
    Some(navigate_to(target, preferred_x))
}

fn move_page_up(pages: &[Page], pos: &Position, viewport: &Viewport) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let preferred_x = line.rect.x + super::x_at_offset(line, pos);
    let y = line.rect.y - viewport.height;
    let (_, target) = search::find_navigable_above(pages, page_idx, y, preferred_x)?;
    Some(navigate_to(target, preferred_x))
}
```

dispatch에 추가:

```rust
Movement::PageDown => move_page_down(pages, pos, viewport),
Movement::PageUp => move_page_up(pages, pos, viewport),
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모두 PASS.

---

## Task 5: Word / Sentence (boundary.rs)

**Files:**
- Modify: `crates/editor-view/Cargo.toml`
- Create: `crates/editor-view/src/cursor/boundary.rs`
- Modify: `crates/editor-view/src/cursor/mod.rs`
- Modify: `crates/editor-view/src/cursor/navigation.rs`

- [ ] **Step 1: icu_segmenter 의존성 추가**

`crates/editor-view/Cargo.toml`의 `[dependencies]`에 추가:

```toml
icu_segmenter = { version = "2.1", default-features = false, features = ["serde"] }
```

기존 `crates/editor/Cargo.toml`의 패턴과 동일한 features 사용.

- [ ] **Step 2: 모듈 선언**

`crates/editor-view/src/cursor/mod.rs`에 추가:

```rust
pub mod boundary;
```

- [ ] **Step 3: boundary.rs 테스트 작성 (todo! stub 포함)**

`crates/editor-view/src/cursor/boundary.rs` 생성:

```rust
use editor_state::Position;
use icu_segmenter::{WordSegmenter, SentenceSegmenter};

use crate::cursor::search;
use crate::fragment::line::{LineFragment, LineSegment};
use crate::page::Page;

pub struct TextSegmenters {
    pub word: WordSegmenter,
    pub sentence: SentenceSegmenter,
}

/// Position → 줄 내 char index 변환
pub fn line_char_index(line: &LineFragment, pos: &Position) -> Option<usize> {
    todo!()
}

/// 줄 내 char index → Position 변환
pub fn position_at_char_index(line: &LineFragment, char_index: usize) -> Option<Position> {
    todo!()
}

pub fn move_word_forward(pages: &[Page], pos: &Position, segmenters: &TextSegmenters) -> Option<Selection> {
    todo!()
}

pub fn move_word_backward(pages: &[Page], pos: &Position, segmenters: &TextSegmenters) -> Option<Selection> {
    todo!()
}

pub fn move_sentence_forward(pages: &[Page], pos: &Position, segmenters: &TextSegmenters) -> Option<Selection> {
    todo!()
}

pub fn move_sentence_backward(pages: &[Page], pos: &Position, segmenters: &TextSegmenters) -> Option<Selection> {
    todo!()
}

#[cfg(test)]
mod tests {
    use editor_common::Rect;
    use editor_model::NodeId;
    use editor_state::Selection;

    use super::*;

    fn make_line(id: NodeId, text: &str) -> LineFragment {
        let advances = vec![10.0; text.chars().count()];
        LineFragment {
            node_id: id,
            rect: Rect { x: 0.0, y: 0.0, width: 200.0, height: 20.0 },
            baseline: 16.0,
            segments: vec![LineSegment {
                node_id: id,
                offset: 0,
                text: text.into(),
                x: 0.0,
                width: 10.0 * text.chars().count() as f32,
                char_advances: advances,
            }],
        }
    }

    fn make_multi_segment_line() -> (LineFragment, NodeId, NodeId) {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let line = LineFragment {
            node_id: id1,
            rect: Rect { x: 0.0, y: 0.0, width: 200.0, height: 20.0 },
            baseline: 16.0,
            segments: vec![
                LineSegment {
                    node_id: id1, offset: 0, text: "hello ".into(),
                    x: 0.0, width: 60.0, char_advances: vec![10.0; 6],
                },
                LineSegment {
                    node_id: id2, offset: 0, text: "world".into(),
                    x: 60.0, width: 50.0, char_advances: vec![10.0; 5],
                },
            ],
        };
        (line, id1, id2)
    }

    #[test]
    fn char_index_at_start() {
        let id = NodeId::new();
        let line = make_line(id, "hello");
        assert_eq!(line_char_index(&line, &Position::new(id, 0)), Some(0));
    }

    #[test]
    fn char_index_in_second_segment() {
        let (line, _, id2) = make_multi_segment_line();
        assert_eq!(line_char_index(&line, &Position::new(id2, 2)), Some(8));
    }

    #[test]
    fn position_at_start() {
        let id = NodeId::new();
        let line = make_line(id, "hello");
        let pos = position_at_char_index(&line, 0).unwrap();
        assert_eq!(pos.node_id, id);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn position_in_second_segment() {
        let (line, _, id2) = make_multi_segment_line();
        let pos = position_at_char_index(&line, 8).unwrap();
        assert_eq!(pos.node_id, id2);
        assert_eq!(pos.offset, 2);
    }
}
```

- [ ] **Step 4: 테스트 실패 확인**

Run: `cargo test -p editor-view boundary`
Expected: FAIL (todo! panic).

- [ ] **Step 5: line_char_index / position_at_char_index 구현**

```rust
pub fn line_char_index(line: &LineFragment, pos: &Position) -> Option<usize> {
    let mut char_count = 0;
    for seg in &line.segments {
        let seg_chars = seg.text.chars().count();
        if seg.node_id == pos.node_id {
            let local = pos.offset.checked_sub(seg.offset)?;
            if local <= seg_chars {
                return Some(char_count + local);
            }
        }
        char_count += seg_chars;
    }
    None
}

pub fn position_at_char_index(line: &LineFragment, char_index: usize) -> Option<Position> {
    let mut remaining = char_index;
    for seg in &line.segments {
        let seg_chars = seg.text.chars().count();
        if remaining <= seg_chars {
            return Some(Position::new(seg.node_id, seg.offset + remaining));
        }
        remaining -= seg_chars;
    }
    None
}
```

- [ ] **Step 6: Word/Sentence boundary 헬퍼 및 이동 함수 구현**

`segment_str()`은 **byte offset**을 반환하므로 byte↔char 변환이 필요하다:

```rust
fn line_text(line: &LineFragment) -> String {
    line.segments.iter().map(|s| s.text.as_str()).collect()
}

fn byte_to_char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

fn char_to_byte_offset(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .nth(char_offset)
        .map(|(i, _)| i)
        .unwrap_or(text.len())
}

fn next_word_boundary(line: &LineFragment, char_index: usize, segmenter: &WordSegmenter) -> Option<usize> {
    let text = line_text(line);
    let byte_idx = char_to_byte_offset(&text, char_index);
    segmenter
        .as_borrowed()
        .segment_str(&text)
        .find(|&b| b > byte_idx)
        .map(|b| byte_to_char_offset(&text, b))
}

fn prev_word_boundary(line: &LineFragment, char_index: usize, segmenter: &WordSegmenter) -> Option<usize> {
    let text = line_text(line);
    let byte_idx = char_to_byte_offset(&text, char_index);
    segmenter
        .as_borrowed()
        .segment_str(&text)
        .filter(|&b| b < byte_idx)
        .last()
        .map(|b| byte_to_char_offset(&text, b))
}

fn next_sentence_boundary(line: &LineFragment, char_index: usize, segmenter: &SentenceSegmenter) -> Option<usize> {
    let text = line_text(line);
    let byte_idx = char_to_byte_offset(&text, char_index);
    segmenter
        .as_borrowed()
        .segment_str(&text)
        .find(|&b| b > byte_idx)
        .map(|b| byte_to_char_offset(&text, b))
}

fn prev_sentence_boundary(line: &LineFragment, char_index: usize, segmenter: &SentenceSegmenter) -> Option<usize> {
    let text = line_text(line);
    let byte_idx = char_to_byte_offset(&text, char_index);
    segmenter
        .as_borrowed()
        .segment_str(&text)
        .filter(|&b| b < byte_idx)
        .last()
        .map(|b| byte_to_char_offset(&text, b))
}
```

이동 함수:

```rust
use editor_state::Selection;

pub fn move_word_forward(pages: &[Page], pos: &Position, segmenters: &TextSegmenters) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = next_word_boundary(line, char_idx, &segmenters.word) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line.rect.bottom();
    let (_, next) = search::find_navigable_below(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(super::navigation::first_position_in(next)))
}

pub fn move_word_backward(pages: &[Page], pos: &Position, segmenters: &TextSegmenters) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = prev_word_boundary(line, char_idx, &segmenters.word) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line.rect.y;
    let (_, prev) = search::find_navigable_above(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(super::navigation::last_position_in(prev)))
}

pub fn move_sentence_forward(pages: &[Page], pos: &Position, segmenters: &TextSegmenters) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = next_sentence_boundary(line, char_idx, &segmenters.sentence) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line.rect.bottom();
    let (_, next) = search::find_navigable_below(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(super::navigation::first_position_in(next)))
}

pub fn move_sentence_backward(pages: &[Page], pos: &Position, segmenters: &TextSegmenters) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = prev_sentence_boundary(line, char_idx, &segmenters.sentence) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line.rect.y;
    let (_, prev) = search::find_navigable_above(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(super::navigation::last_position_in(prev)))
}
```

주의: `first_position_in`/`last_position_in`의 가시성을 `pub(crate)`로 변경하여 boundary.rs에서 접근 가능하게 한다.

- [ ] **Step 7: navigation.rs에 Word/Sentence dispatch 추가**

```rust
Movement::Word(Direction::Forward) => {
    segmenters.and_then(|s| boundary::move_word_forward(pages, pos, s))
}
Movement::Word(Direction::Backward) => {
    segmenters.and_then(|s| boundary::move_word_backward(pages, pos, s))
}
Movement::Sentence(Direction::Forward) => {
    segmenters.and_then(|s| boundary::move_sentence_forward(pages, pos, s))
}
Movement::Sentence(Direction::Backward) => {
    segmenters.and_then(|s| boundary::move_sentence_backward(pages, pos, s))
}
```

segmenters가 None이면 None 반환.

- [ ] **Step 8: 컴파일 및 테스트 통과 확인**

Run: `cargo check -p editor-view`
Run: `cargo test -p editor-view`
Expected: 모두 PASS.

주의: Word/Sentence 이동의 통합 테스트는 ICU data provider가 필요하다. 레거시처럼 `try_new_*_unstable`로 segmenter를 생성하려면 provider가 있어야 한다. 테스트에서 `compiled_data` feature 없이 segmenter를 생성하는 방법을 확인한다. 필요시 `[dev-dependencies]`에 `icu_segmenter`의 `compiled_data` feature를 추가하여 테스트에서만 `new_auto()` 사용:

```toml
[dev-dependencies]
icu_segmenter = { version = "2.1", features = ["compiled_data", "auto"] }
```

---

## Task 6: remaining-work 업데이트

**Files:**
- Modify: `docs/editor-architecture/view-remaining-work.md`

- [ ] **Step 1: 커서 Movement 항목을 완료로 표시**

`view-remaining-work.md`에서 "커서 Movement 확장" 섹션의 구현된 항목(Word, Sentence, Block, Page, Document)을 완료로 표시하고, 설계 문서 참조를 추가한다.
