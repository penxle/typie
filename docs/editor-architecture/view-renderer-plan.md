# View Renderer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Note:** Do not run `git commit`. The user commits manually.

**Goal:** Fragment 기반 렌더링 파이프라인 구축 — editor-view의 Fragment에 Placeholder/GlyphRun 확장, editor-renderer crate 신규 생성.

**Architecture:** editor-view가 순수 layout(위치, 크기, Placeholder)을 생산하고, editor-renderer가 Doc + Theme + IconRegistry를 읽어 시각 결정 및 그리기를 수행. Fragment에는 렌더링 정보(색상, 아이콘 이름)가 포함되지 않으며, GlyphRun의 color token만 예외(문서 수준 텍스트 속성).

**Tech Stack:** Rust (Edition 2024), parley (text shaping), kurbo/peniko/vello (rendering backends, sink 구현부에서만 사용)

**Spec:** `docs/editor-architecture/view-renderer-design.md`

---

## File Structure

### New Files

```
crates/editor-common/src/color.rs                     — Color (RGBA8)
crates/editor-view/src/fragment/placeholder.rs         — PlaceholderFragment, PlaceholderData
crates/editor-view/src/fragment/glyph_run.rs           — GlyphRun, Glyph, Synthesis, FontId
crates/editor-renderer/Cargo.toml                      — crate manifest
crates/editor-renderer/src/lib.rs                      — pub mod + re-exports
crates/editor-renderer/src/sink.rs                     — RenderSink trait
crates/editor-renderer/src/types.rs                    — Transform, Path, PathElement, Stroke
crates/editor-renderer/src/theme.rs                    — Theme struct
crates/editor-renderer/src/icons.rs                    — IconRegistry
crates/editor-renderer/src/renderer.rs                 — Renderer struct + render_page entry
crates/editor-renderer/src/nodes/mod.rs                — render_fragment dispatch
crates/editor-renderer/src/nodes/container.rs          — draw_container_background, draw_container_border
crates/editor-renderer/src/nodes/line.rs               — draw_line (GlyphRun 렌더링)
crates/editor-renderer/src/nodes/atom.rs               — draw_atom (HR variants)
crates/editor-renderer/src/nodes/placeholder.rs        — draw_placeholder (아이콘, 마커)
```

### Modified Files

```
crates/editor-common/src/lib.rs                        — mod color + pub use
crates/editor-view/src/fragment/mod.rs                 — Fragment enum + Placeholder variant, position_in_line 수정
crates/editor-view/src/fragment/line.rs                — LineFragment.segments → glyph_runs
crates/editor-view/src/fragment/container.rs           — ContainerFragment + border field
crates/editor-view/src/measure.rs                      — MeasuredLine.segments → glyph_runs
crates/editor-view/src/cursor/mod.rs                   — x_at_offset: LineSegment → GlyphRun
crates/editor-view/src/cursor/hit_test.rs              — LineSegment → GlyphRun
crates/editor-view/src/cursor/navigation.rs            — LineSegment → GlyphRun
crates/editor-view/src/cursor/search.rs                — LineSegment → GlyphRun
crates/editor-view/src/engine/paginator.rs             — place_line/position_subtree: glyph_runs + border + Placeholder
crates/editor-view/src/engine/measure_nodes/paragraph/line_extraction.rs — GlyphRun 생성
crates/editor-view/src/engine/measure_nodes/callout.rs — Placeholder 자식 추가
crates/editor-view/src/engine/measure_nodes/blockquote.rs — Placeholder 자식 추가
crates/editor-view/src/engine/measure_nodes/fold.rs    — Placeholder 자식 추가
crates/editor-view/src/engine/measure_nodes/list_item.rs — Placeholder 자식 추가
crates/editor-view/Cargo.toml                          — (필요 시 의존성 추가)
```

---

### Task 1: Color 타입 추가 (editor-common)

**Files:**
- Create: `crates/editor-common/src/color.rs`
- Modify: `crates/editor-common/src/lib.rs`

- [ ] **Step 1: Color 타입 작성**

```rust
// crates/editor-common/src/color.rs

/// RGBA8 색상.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
}
```

- [ ] **Step 2: lib.rs에 모듈 등록**

`crates/editor-common/src/lib.rs`의 모듈 선언 목록에 추가:

```rust
mod color;
// ... existing mods ...

pub use color::*;
// ... existing re-exports ...
```

- [ ] **Step 3: 컴파일 확인**

Run: `cargo check -p editor-common`
Expected: 성공

---

### Task 2: GlyphRun 관련 타입 정의 (editor-view)

**Files:**
- Create: `crates/editor-view/src/fragment/glyph_run.rs`
- Create: `crates/editor-view/src/fragment/placeholder.rs`
- Modify: `crates/editor-view/src/fragment/mod.rs` — mod 선언 추가

- [ ] **Step 1: Glyph, Synthesis, FontId, GlyphRun 정의**

```rust
// crates/editor-view/src/fragment/glyph_run.rs
use editor_common::NodeId;

/// 폰트 식별자. FontRegistry에서 interning된 값.
pub type FontId = u16;

/// 개별 glyph의 위치 정보.
#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

/// Faux bold/italic 합성 정보.
#[derive(Debug, Clone, Copy, Default)]
pub struct Synthesis {
    pub embolden: bool,
    pub skew: Option<f32>,
}

/// 하나의 glyph run. 렌더링(glyph 데이터)과 커서 내비게이션(char_advances)을 통합.
///
/// parley는 스타일 경계에서 run을 분리하므로 각 glyph run은 하나의 node_id에 대응.
/// 폰트 fallback으로 하나의 텍스트 노드가 여러 glyph run을 가질 수 있다.
#[derive(Debug, Clone)]
pub struct GlyphRun {
    // 렌더용
    pub font_id: FontId,
    pub font_size: f32,
    pub synthesis: Synthesis,
    pub color: String,
    pub background_color: Option<String>,
    pub glyphs: Vec<Glyph>,

    // 커서용
    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
    pub x: f32,
    pub width: f32,
    pub char_advances: Vec<f32>,
}
```

Note: `x`와 `width`를 유지한다 — 기존 LineSegment이 가지고 있던 필드로, 커서 hit_test에서 run의 x 범위를 빠르게 판별하는 데 필요.

- [ ] **Step 2: PlaceholderFragment, PlaceholderData 정의**

```rust
// crates/editor-view/src/fragment/placeholder.rs
use editor_common::Rect;

/// layout이 renderer에 전달하는 opaque 데이터.
#[derive(Debug, Clone)]
pub enum PlaceholderData {
    None,
    Bool(bool),
    Number(f64),
    Text(String),
}

/// 장식 요소의 위치를 layout으로 잡는 placeholder.
/// parley의 inline box에서 영감.
#[derive(Debug, Clone)]
pub struct PlaceholderFragment {
    pub id: u32,
    pub rect: Rect,
    pub data: PlaceholderData,
}
```

- [ ] **Step 3: fragment/mod.rs에 모듈 등록**

`crates/editor-view/src/fragment/mod.rs` 상단에 추가:

```rust
mod glyph_run;
mod placeholder;

pub use glyph_run::*;
pub use placeholder::*;
```

- [ ] **Step 4: 컴파일 확인**

Run: `cargo check -p editor-view`
Expected: 성공 (아직 사용하지 않으므로 warning만)

---

### Task 3: Fragment 구조체 마이그레이션

LineSegment → GlyphRun, ContainerFragment에 border 추가, Fragment enum에 Placeholder 추가.

**Files:**
- Modify: `crates/editor-view/src/fragment/line.rs`
- Modify: `crates/editor-view/src/fragment/container.rs`
- Modify: `crates/editor-view/src/fragment/mod.rs`
- Modify: `crates/editor-view/src/measure.rs`

- [ ] **Step 1: LineFragment의 segments를 glyph_runs로 변경**

`crates/editor-view/src/fragment/line.rs` — LineSegment 정의는 유지하되 (MeasuredLine에서 임시 사용), LineFragment를 수정:

```rust
// crates/editor-view/src/fragment/line.rs
use crate::fragment::glyph_run::GlyphRun;
use editor_common::{NodeId, Rect};

/// 레거시 호환용. line_extraction에서 GlyphRun 전환 완료 후 제거 예정.
#[derive(Debug, Clone)]
pub struct LineSegment {
    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
    pub x: f32,
    pub width: f32,
    pub char_advances: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct LineFragment {
    pub node_id: NodeId,
    pub rect: Rect,
    pub baseline: f32,
    pub glyph_runs: Vec<GlyphRun>,
}
```

- [ ] **Step 2: ContainerFragment에 border 추가**

`crates/editor-view/src/fragment/container.rs`:

```rust
use editor_common::{EdgeInsets, NodeId, Rect};
use crate::fragment::Fragment;

#[derive(Debug, Clone, Copy, Default)]
pub struct Breaks {
    pub top: bool,
    pub bottom: bool,
}

#[derive(Debug, Clone)]
pub struct ContainerFragment {
    pub node_id: NodeId,
    pub rect: Rect,
    pub children: Vec<Fragment>,
    pub scope: bool,
    pub breaks: Breaks,
    pub border: EdgeInsets,
}
```

- [ ] **Step 3: Fragment enum에 Placeholder variant 추가**

`crates/editor-view/src/fragment/mod.rs`에서 Fragment enum 수정:

```rust
#[derive(Debug, Clone)]
pub enum Fragment {
    Container(ContainerFragment),
    Line(LineFragment),
    Atom(AtomFragment),
    Placeholder(PlaceholderFragment),
}
```

`Fragment::rect()` 메서드에 Placeholder 분기 추가:

```rust
pub fn rect(&self) -> &Rect {
    match self {
        Fragment::Container(f) => &f.rect,
        Fragment::Line(f) => &f.rect,
        Fragment::Atom(f) => &f.rect,
        Fragment::Placeholder(f) => &f.rect,
    }
}
```

`Fragment::node_id()` 메서드 — Placeholder는 node_id가 없으므로 Option으로 변경하거나, match에서 panic. 기존 코드가 node_id()를 어떻게 사용하는지에 따라 결정. (현재 코드를 읽고 판단할 것.)

- [ ] **Step 4: MeasuredLine.segments → glyph_runs**

`crates/editor-view/src/measure.rs`의 MeasuredLine 수정:

```rust
use crate::fragment::GlyphRun;

#[derive(Debug, Clone)]
pub struct MeasuredLine {
    pub height: f32,
    pub baseline: f32,
    pub glyph_runs: Vec<GlyphRun>,
}
```

Note: 이 변경은 line_extraction.rs와 paginator.rs에서 컴파일 에러를 발생시킨다. 다음 Task들에서 수정.

- [ ] **Step 5: fragment/mod.rs의 position_in_line 수정**

`position_in_line` 함수가 `line.segments`를 순회한다. `line.glyph_runs`로 변경:

```rust
pub fn position_in_line(line: &LineFragment, x: f32) -> Position {
    for run in &line.glyph_runs {
        if x < run.x || x > run.x + run.width {
            continue;
        }
        let mut cx = run.x;
        for (i, &advance) in run.char_advances.iter().enumerate() {
            if x < cx + advance / 2.0 {
                return Position {
                    node_id: run.node_id,
                    offset: run.offset + i,
                };
            }
            cx += advance;
        }
        return Position {
            node_id: run.node_id,
            offset: run.offset + run.char_advances.len(),
        };
    }
    // fallback: 마지막 run의 끝
    if let Some(run) = line.glyph_runs.last() {
        Position {
            node_id: run.node_id,
            offset: run.offset + run.char_advances.len(),
        }
    } else {
        Position {
            node_id: line.node_id,
            offset: 0,
        }
    }
}
```

- [ ] **Step 6: 컴파일 시도 (에러 예상)**

Run: `cargo check -p editor-view`
Expected: 컴파일 에러 — cursor 모듈, paginator, line_extraction에서 `segments` 필드 참조. 다음 Task들에서 수정.

---

### Task 4: 커서 코드 마이그레이션

cursor/ 모듈의 모든 LineSegment 참조를 GlyphRun으로 변경.

**Files:**
- Modify: `crates/editor-view/src/cursor/mod.rs`
- Modify: `crates/editor-view/src/cursor/hit_test.rs`
- Modify: `crates/editor-view/src/cursor/navigation.rs`
- Modify: `crates/editor-view/src/cursor/search.rs`

- [ ] **Step 1: cursor/mod.rs — x_at_offset 수정**

`line.segments` → `line.glyph_runs`, `seg` → `run`:

```rust
pub(crate) fn x_at_offset(line: &LineFragment, pos: &Position) -> f32 {
    for run in &line.glyph_runs {
        if run.node_id != pos.node_id {
            continue;
        }

        let local_offset = pos.offset.saturating_sub(run.offset);
        if local_offset > run.char_advances.len() {
            continue;
        }

        return run.x + run.char_advances[..local_offset].iter().sum::<f32>();
    }

    0.0
}
```

- [ ] **Step 2: cursor/hit_test.rs 수정**

Fragment match에 Placeholder 분기 추가 (Placeholder는 hit_test 대상이 아님 — 무시):

```rust
Fragment::Placeholder(_) => None,
```

나머지 LineSegment 참조가 있으면 GlyphRun으로 변경.

- [ ] **Step 3: cursor/navigation.rs 수정**

`line.segments` → `line.glyph_runs`, 변수명 `seg` → `run`:

- `move_grapheme_forward()`: `line.glyph_runs` 순회
- `move_grapheme_backward()`: `line.glyph_runs` 순회
- `first_position_in_line()`: `line.glyph_runs.first()`
- `last_position_in_line()`: `line.glyph_runs.last()`

Fragment match에 Placeholder 분기 추가 (navigable하지 않음):

```rust
Fragment::Placeholder(_) => { /* skip */ },
```

- [ ] **Step 4: cursor/search.rs 수정**

`find_line_for_position()` 내부의 `line.segments.iter()` → `line.glyph_runs.iter()`:

```rust
let contains = line.glyph_runs.iter().any(|run| {
    run.node_id == pos.node_id
        && pos.offset >= run.offset
        && pos.offset <= run.offset + run.char_advances.len()
});
```

Fragment match에 Placeholder 분기 추가.

- [ ] **Step 5: 테스트 픽스처 수정**

cursor 모듈의 모든 테스트에서 `LineSegment { ... }` 생성을 `GlyphRun { ... }` 생성으로 변경. 새 필드에는 기본값:

```rust
GlyphRun {
    font_id: 0,
    font_size: 16.0,
    synthesis: Synthesis::default(),
    color: String::new(),
    background_color: None,
    glyphs: vec![],
    // 기존 LineSegment 필드
    node_id: /* test value */,
    offset: /* test value */,
    text: /* test value */,
    x: /* test value */,
    width: /* test value */,
    char_advances: /* test value */,
}
```

테스트 헬퍼 함수를 만들어 중복 줄이기:

```rust
fn make_run(node_id: NodeId, offset: usize, text: &str, x: f32, advances: Vec<f32>) -> GlyphRun {
    let width = advances.iter().sum();
    GlyphRun {
        font_id: 0,
        font_size: 16.0,
        synthesis: Synthesis::default(),
        color: String::new(),
        background_color: None,
        glyphs: vec![],
        node_id,
        offset,
        text: text.to_string(),
        x,
        width,
        char_advances: advances,
    }
}
```

- [ ] **Step 6: 커서 테스트 통과 확인**

Run: `cargo test -p editor-view -- cursor`
Expected: 모든 커서 테스트 통과

---

### Task 5: Paginator 마이그레이션

Paginator가 GlyphRun, border, Placeholder를 처리하도록 수정.

**Files:**
- Modify: `crates/editor-view/src/engine/paginator.rs`

- [ ] **Step 1: place_line() — segments → glyph_runs**

`place_line()` (약 line 387)에서:

```rust
fn place_line(&mut self, node_id: NodeId, line: &MeasuredLine) {
    // ... page break check ...

    let fragment = Fragment::Line(LineFragment {
        node_id,
        rect: Rect {
            x: self.current_x(),
            y: self.current_y,
            width: self.width,
            height: line.height,
        },
        baseline: line.baseline,
        glyph_runs: line.glyph_runs.clone(),  // CHANGED
    });

    self.add_to_current(fragment);
    self.current_y += line.height;
}
```

- [ ] **Step 2: position_subtree() TextBlock — segments → glyph_runs**

`position_subtree()` (약 line 466)에서 TextBlock 처리:

```rust
MeasuredContent::TextBlock { lines } => {
    let mut line_y = y;
    let line_frags: Vec<Fragment> = lines
        .iter()
        .map(|line| {
            let frag = Fragment::Line(LineFragment {
                node_id,
                rect: Rect {
                    x,
                    y: line_y,
                    width: measurement.size.width,
                    height: line.height,
                },
                baseline: line.baseline,
                glyph_runs: line.glyph_runs.clone(),  // CHANGED
            });
            line_y += line.height;
            frag
        })
        .collect();
    // ...
}
```

- [ ] **Step 3: ContainerFragment 생성 시 border 전달**

Paginator에서 ContainerFragment를 생성하는 모든 위치에서 `border` 필드 추가. OpenContainer가 이미 `border: EdgeInsets`를 가지고 있으므로:

```rust
// close_container() 또는 finish_container() 내부에서
Fragment::Container(ContainerFragment {
    node_id: open.node_id,
    rect: /* ... */,
    children: open.children,
    scope: open.scope,
    breaks: /* ... */,
    border: open.border,  // NEW
})
```

모든 ContainerFragment 생성 위치를 찾아 `border` 필드 추가. (Paginator 코드를 읽고 해당 위치를 전부 식별할 것.)

- [ ] **Step 4: 컴파일 확인**

Run: `cargo check -p editor-view`
Expected: line_extraction.rs에서 에러 (MeasuredLine 구조 변경). 다음 Task에서 수정.

---

### Task 6: line_extraction — GlyphRun 생성

MeasuredLine이 GlyphRun을 생성하도록 line_extraction을 수정. 초기 구현에서는 glyph 데이터를 캡처하고, font_id와 synthesis도 parley에서 추출.

**Files:**
- Modify: `crates/editor-view/src/engine/measure_nodes/paragraph/line_extraction.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/paragraph/mod.rs` (필요 시)

- [ ] **Step 1: extract_measured_lines에서 GlyphRun 생성으로 전환**

핵심 변경: cluster 단위 순회에서 run + cluster 순회로 전환하여 glyph 데이터를 캡처.

기존 로직은 cluster를 순회하며 같은 node_id의 cluster를 병합하여 LineSegment를 만든다. 새 로직은 glyph run을 순회하며 GlyphRun을 만든다.

parley의 run API:
- `run.font()` → Font (font data 접근)
- `run.font_size()` → f32
- `run.synthesis()` → Synthesis (embolden, skew)
- `run.visual_clusters()` → cluster iterator
- `glyph_run.glyphs()` → glyph iterator (PositionedLayoutItem에서)

```rust
pub fn extract_measured_lines(
    text: &str,
    layout: &Layout<TextBrush>,
    strut: &StrutMetrics,
    line_height_ratio: f32,
    base_font_size: f32,
    doc: &Doc,          // NEW — color token resolve용
) -> Vec<MeasuredLine> {
    let mut lines = Vec::new();

    for line in layout.lines() {
        let metrics = line.metrics();
        // ... height/baseline calculation (기존과 동일) ...

        let mut glyph_runs = Vec::new();

        for item in line.items() {
            match item {
                PositionedLayoutItem::GlyphRun(positioned_run) => {
                    let run = positioned_run.run();
                    let style = positioned_run.style();
                    let node_id = style.brush.node_id;

                    // glyph 데이터 캡처
                    let glyphs: Vec<Glyph> = positioned_run.glyphs().map(|g| {
                        Glyph { id: g.id as u32, x: g.x, y: g.y }
                    }).collect();

                    // font_id: FontRegistry의 intern 사용
                    // (ShapingContext를 통해 접근 — 파라미터 추가 필요)
                    let font_id = 0; // TODO: FontRegistry에서 resolve

                    let font_size = run.font_size();
                    let synthesis = {
                        let s = run.synthesis();
                        Synthesis {
                            embolden: s.vars.contains(&parley::style::FontVariation::new(
                                // parley synthesis API 확인 필요
                            )),
                            skew: None, // parley synthesis API 확인 필요
                        }
                    };

                    // color token resolve (Doc에서 FontColor modifier)
                    let color = resolve_color_token(doc, node_id);
                    let background_color = resolve_background_token(doc, node_id);

                    // char_advances 계산 (기존 cluster 기반 로직 유지)
                    let mut char_advances = Vec::new();
                    let mut run_text = String::new();
                    let mut run_x = positioned_run.offset();
                    let mut run_width = 0.0f32;
                    let mut char_offset = 0usize;
                    let mut first = true;

                    for cluster in run.visual_clusters() {
                        let cluster_range = cluster.text_range();
                        let cluster_text = &text[cluster_range.clone()];
                        let advance = cluster.advance();
                        let char_count = cluster_text.chars().count();
                        let per_char = if char_count > 0 { advance / char_count as f32 } else { 0.0 };

                        if first {
                            let byte_start = cluster_range.start;
                            char_offset = text[..byte_start].chars().count();
                            first = false;
                        }

                        run_text.push_str(cluster_text);
                        run_width += advance;
                        for _ in 0..char_count {
                            char_advances.push(per_char);
                        }
                    }

                    glyph_runs.push(GlyphRun {
                        font_id,
                        font_size,
                        synthesis,
                        color,
                        background_color,
                        glyphs,
                        node_id,
                        offset: char_offset,
                        text: run_text,
                        x: run_x,
                        width: run_width,
                        char_advances,
                    });
                }
                PositionedLayoutItem::InlineBox(_) => {
                    // Placeholder inline box — 무시 (현재 사용하지 않음)
                }
            }
        }

        lines.push(MeasuredLine {
            height: line_box_height,
            baseline,
            glyph_runs,
        });
    }

    lines
}
```

Note: parley API의 정확한 시그니처는 구현 시 확인 필요. 위 코드는 구조를 보여주기 위한 것이며, `positioned_run.glyphs()`, `run.synthesis()` 등의 정확한 API는 parley 0.7 문서/소스를 참조할 것. font_id resolve와 synthesis 추출은 parley API에 맞게 조정할 것.

- [ ] **Step 2: resolve_color_token, resolve_background_token 헬퍼**

`line_extraction.rs` 또는 `resolve.rs`에 추가:

```rust
fn resolve_color_token(doc: &Doc, node_id: NodeId) -> String {
    // Doc에서 node_id의 FontColor modifier 조회
    // modifier가 있으면 해당 color token 반환
    // 없으면 기본 텍스트 색상 토큰 반환
    // 정확한 modifier 조회 방법은 resolve_text_style 패턴 참조
    "text.default".to_string() // placeholder — 실제 구현 시 modifier 조회
}

fn resolve_background_token(doc: &Doc, node_id: NodeId) -> Option<String> {
    // Doc에서 Highlight modifier 조회
    None // placeholder
}
```

Note: 기존 `resolve.rs`의 `resolve_text_style` 함수가 modifier 조회 패턴을 보여준다. 같은 패턴으로 FontColor와 Highlight modifier를 조회할 것.

- [ ] **Step 3: measure_paragraph 호출부 수정**

`extract_measured_lines` 시그니처에 `doc` 파라미터가 추가되었으므로, 호출하는 `measure_paragraph`(또는 `mod.rs`)에서 doc를 전달하도록 수정.

- [ ] **Step 4: 컴파일 확인**

Run: `cargo check -p editor-view`
Expected: 성공

- [ ] **Step 5: 기존 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 모든 테스트 통과

---

### Task 7: measure_nodes — Placeholder 자식 추가

장식 요소가 필요한 노드의 measure 함수에서 PlaceholderFragment에 대응하는 Placeholder ChildMeasurement를 추가.

**접근:** Placeholder는 Fragment 수준의 개념이므로, Measurement(측정) 단계가 아닌 Paginator(배치) 단계에서 추가하는 것이 더 자연스러울 수 있다. 하지만 Placeholder의 위치는 측정 단계에서 padding과 함께 결정되므로, MeasuredContent에 placeholder 정보를 포함시키는 것이 맞다.

**Files:**
- Modify: `crates/editor-view/src/measure.rs` — ContainerContent에 placeholders 필드 추가
- Modify: `crates/editor-view/src/engine/measure_nodes/callout.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/blockquote.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/fold.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/list_item.rs`
- Modify: `crates/editor-view/src/engine/paginator.rs` — Placeholder를 Fragment로 변환

- [ ] **Step 1: ContainerContent에 placeholders 필드 추가**

`crates/editor-view/src/measure.rs`:

```rust
use crate::fragment::{PlaceholderData, Rect};

/// 측정 단계에서 결정된 placeholder 위치.
#[derive(Debug, Clone)]
pub struct MeasuredPlaceholder {
    pub id: u32,
    pub rect: Rect,
    pub data: PlaceholderData,
}

#[derive(Debug, Clone)]
pub struct ContainerContent {
    pub children: Vec<ChildMeasurement>,
    pub scope: bool,
    pub direction: LayoutDirection,
    pub padding: EdgeInsets,
    pub border: EdgeInsets,
    pub border_mode: BorderMode,
    pub placeholders: Vec<MeasuredPlaceholder>,  // NEW
}
```

기존 ContainerContent를 생성하는 모든 위치에 `placeholders: vec![]` 추가 (measure_padded_container, measure_default_container, measure_fold, measure_table 등).

- [ ] **Step 2: measure_callout — 아이콘 Placeholder**

`crates/editor-view/src/engine/measure_nodes/callout.rs`:

아이콘 위치: x=CALLOUT_PADDING_X(12.0), y는 padding.top(16.0)에서 수직 중앙, 크기=CALLOUT_ICON_WIDTH(20.0)x20.0

```rust
use crate::measure::MeasuredPlaceholder;
use crate::fragment::PlaceholderData;

// measure_callout 내부, ContainerContent 생성 전에:
let icon_rect = Rect {
    x: 12.0,  // CALLOUT_PADDING_X
    y: 16.0,  // CALLOUT_PADDING_Y (아이콘 y는 첫 줄과 정렬)
    width: 20.0,   // CALLOUT_ICON_WIDTH
    height: 20.0,
};
let placeholders = vec![MeasuredPlaceholder {
    id: 0,
    rect: icon_rect,
    data: PlaceholderData::None,
}];
```

measure_padded_container에 placeholders를 전달하는 방법: measure_padded_container의 반환 Measurement의 ContainerContent.placeholders를 설정하거나, measure_callout에서 직접 ContainerContent를 구성. 기존 코드 구조를 읽고 가장 자연스러운 방법 선택.

- [ ] **Step 3: measure_blockquote — 인용 아이콘 / 왼쪽 바 Placeholder**

LeftQuote variant에 인용 아이콘 Placeholder 추가:

```rust
// LeftQuote variant
let placeholders = vec![MeasuredPlaceholder {
    id: 0,
    rect: Rect { x: 0.0, y: 0.0, width: 16.0, height: 16.0 },  // BQ_QUOTE_SIZE
    data: PlaceholderData::None,
}];
```

LeftLine variant — 왼쪽 바는 Placeholder가 아닌 border로 표현 가능. 또는 Placeholder로 추가:

```rust
let placeholders = vec![MeasuredPlaceholder {
    id: 0,
    rect: Rect { x: 0.0, y: 0.0, width: 4.0, height: /* container height */ },  // BQ_LINE_WIDTH
    data: PlaceholderData::None,
}];
```

Note: container height는 children 측정 후에 결정되므로, height를 0으로 설정하고 Paginator에서 container 높이에 맞게 조정하거나, 측정 후에 placeholders를 수정할 것.

- [ ] **Step 4: measure_fold — chevron Placeholder**

```rust
// measure_fold_title 내부
let expanded = view_state.fold_expanded(node.id());
let placeholders = vec![MeasuredPlaceholder {
    id: 0,
    rect: Rect { x: 12.0, y: 8.0, width: 20.0, height: 20.0 },
    data: PlaceholderData::Bool(expanded),
}];
```

- [ ] **Step 5: measure_list_item — marker Placeholder**

```rust
let placeholders = vec![MeasuredPlaceholder {
    id: 0,
    rect: Rect { x: 0.0, y: 0.0, width: 20.0, height: 20.0 },  // LIST_ITEM_MARKER_WIDTH
    data: PlaceholderData::None,  // bullet은 None, ordered는 Text("1.") 등
}];
```

Note: ordered list의 marker text 결정은 Doc의 list 컨텍스트에서 순서를 계산해야 한다. 초기 구현에서는 PlaceholderData::None으로 두고, 후속 작업에서 ordered marker를 구현할 수 있다.

- [ ] **Step 6: Paginator — Placeholder를 Fragment로 변환**

Paginator에서 ContainerFragment를 생성할 때, ContainerContent의 placeholders를 PlaceholderFragment로 변환하여 children에 추가:

```rust
// close_container() 또는 관련 함수에서
for ph in &container_content.placeholders {
    children.push(Fragment::Placeholder(PlaceholderFragment {
        id: ph.id,
        rect: Rect {
            x: ph.rect.x + container_x,
            y: ph.rect.y + container_y,
            width: ph.rect.width,
            height: ph.rect.height,
        },
        data: ph.data.clone(),
    }));
}
```

Note: placeholder의 rect는 container 내부 좌표이므로, container의 x/y offset을 더해야 할 수 있다. Paginator의 좌표 시스템을 확인하고 조정할 것.

- [ ] **Step 7: 컴파일 및 테스트**

Run: `cargo check -p editor-view && cargo test -p editor-view`
Expected: 컴파일 성공, 테스트 통과

---

### Task 8: editor-renderer crate 스켈레톤

**Files:**
- Create: `crates/editor-renderer/Cargo.toml`
- Create: `crates/editor-renderer/src/lib.rs`
- Create: `crates/editor-renderer/src/types.rs`
- Create: `crates/editor-renderer/src/sink.rs`

- [ ] **Step 1: Cargo.toml 생성**

```toml
[package]
name = "editor-renderer"
version.workspace = true
edition.workspace = true

[dependencies]
editor-common = { path = "../editor-common" }
editor-model = { path = "../editor-model" }
editor-view = { path = "../editor-view" }
```

- [ ] **Step 2: types.rs — 렌더링 primitive 타입**

```rust
// crates/editor-renderer/src/types.rs
use editor_common::Rect;

/// 2D affine transform.
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub m: [f32; 6], // [a, b, c, d, e, f] — standard 2D affine
}

impl Transform {
    pub const IDENTITY: Self = Self { m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] };

    pub fn translate(self, x: f32, y: f32) -> Self {
        Self {
            m: [
                self.m[0],
                self.m[1],
                self.m[2],
                self.m[3],
                self.m[4] + x,
                self.m[5] + y,
            ],
        }
    }
}

/// Path의 구성 요소.
#[derive(Debug, Clone, Copy)]
pub enum PathElement {
    MoveTo { x: f32, y: f32 },
    LineTo { x: f32, y: f32 },
    QuadTo { x1: f32, y1: f32, x: f32, y: f32 },
    CurveTo { x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32 },
    Close,
}

/// 벡터 path.
#[derive(Debug, Clone)]
pub struct Path {
    pub elements: Vec<PathElement>,
}

impl Path {
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }

    pub fn rect(r: Rect) -> Self {
        Self {
            elements: vec![
                PathElement::MoveTo { x: r.x, y: r.y },
                PathElement::LineTo { x: r.x + r.width, y: r.y },
                PathElement::LineTo { x: r.x + r.width, y: r.y + r.height },
                PathElement::LineTo { x: r.x, y: r.y + r.height },
                PathElement::Close,
            ],
        }
    }
}

/// Stroke 스타일.
#[derive(Debug, Clone, Copy)]
pub struct Stroke {
    pub width: f32,
}

impl Stroke {
    pub fn new(width: f32) -> Self {
        Self { width }
    }
}
```

- [ ] **Step 3: sink.rs — RenderSink trait**

```rust
// crates/editor-renderer/src/sink.rs
use editor_common::{Color, Rect};
use editor_view::fragment::{GlyphRun, Synthesis};
use crate::types::{Transform, Path, Stroke};

pub trait RenderSink {
    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform);
    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform);
    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform);
    fn draw_glyphs(&mut self, run: &GlyphRun, color: Color, transform: Transform);
}
```

- [ ] **Step 4: lib.rs**

```rust
// crates/editor-renderer/src/lib.rs
pub mod sink;
pub mod types;

pub use sink::RenderSink;
pub use types::*;
```

- [ ] **Step 5: 컴파일 확인**

Run: `cargo check -p editor-renderer`
Expected: 성공

---

### Task 9: Theme + IconRegistry

**Files:**
- Create: `crates/editor-renderer/src/theme.rs`
- Create: `crates/editor-renderer/src/icons.rs`
- Modify: `crates/editor-renderer/src/lib.rs`

- [ ] **Step 1: Theme 구현**

```rust
// crates/editor-renderer/src/theme.rs
use editor_common::Color;
use hashbrown::HashMap;

pub struct Theme {
    colors: HashMap<String, Color>,
}

impl Theme {
    pub fn new(colors: HashMap<String, Color>) -> Self {
        Self { colors }
    }

    pub fn color(&self, token: &str) -> Color {
        self.colors.get(token).copied().unwrap_or(Color::BLACK)
    }

    pub fn color_with_alpha(&self, token: &str, alpha: u8) -> Color {
        self.color(token).with_alpha(alpha)
    }
}
```

- [ ] **Step 2: IconRegistry 기본 구현**

```rust
// crates/editor-renderer/src/icons.rs
use editor_common::Rect;
use crate::types::Path;

pub struct IconRegistry {
    // MVP: 빈 구현. 아이콘 path 생성은 후속 작업에서 레거시 매크로 시스템과 연동.
}

impl IconRegistry {
    pub fn new() -> Self {
        Self {}
    }

    /// name + rect로 Path 생성.
    /// MVP에서는 rect를 그대로 path로 반환 (placeholder).
    pub fn resolve(&self, _name: &str, rect: Rect) -> Path {
        Path::rect(rect)
    }
}
```

- [ ] **Step 3: hashbrown 의존성 추가**

`crates/editor-renderer/Cargo.toml`:

```toml
[dependencies]
editor-common = { path = "../editor-common" }
editor-model = { path = "../editor-model" }
editor-view = { path = "../editor-view" }
hashbrown = "0.16"
```

- [ ] **Step 4: lib.rs 업데이트**

```rust
pub mod icons;
pub mod sink;
pub mod theme;
pub mod types;

pub use icons::IconRegistry;
pub use sink::RenderSink;
pub use theme::Theme;
pub use types::*;
```

- [ ] **Step 5: 컴파일 확인**

Run: `cargo check -p editor-renderer`
Expected: 성공

---

### Task 10: Renderer struct + render_page

**Files:**
- Create: `crates/editor-renderer/src/renderer.rs`
- Create: `crates/editor-renderer/src/nodes/mod.rs`
- Create: `crates/editor-renderer/src/nodes/container.rs`
- Create: `crates/editor-renderer/src/nodes/line.rs`
- Create: `crates/editor-renderer/src/nodes/atom.rs`
- Create: `crates/editor-renderer/src/nodes/placeholder.rs`
- Modify: `crates/editor-renderer/src/lib.rs`

- [ ] **Step 1: Renderer struct**

```rust
// crates/editor-renderer/src/renderer.rs
use std::sync::Arc;
use editor_common::FontRegistry;
use editor_model::Doc;
use editor_view::Page;
use crate::sink::RenderSink;
use crate::theme::Theme;
use crate::icons::IconRegistry;
use crate::types::Transform;

pub struct Renderer {
    pub(crate) icons: IconRegistry,
    pub(crate) fonts: Arc<FontRegistry>,
    pub(crate) theme: Theme,
}

impl Renderer {
    pub fn new(icons: IconRegistry, fonts: Arc<FontRegistry>, theme: Theme) -> Self {
        Self { icons, fonts, theme }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn render_page(&self, sink: &mut impl RenderSink, page: &Page, doc: &Doc) {
        for fragment in &page.body {
            crate::nodes::render_fragment(self, sink, fragment, doc, None, Transform::IDENTITY);
        }
    }
}
```

- [ ] **Step 2: nodes/mod.rs — render_fragment dispatch**

```rust
// crates/editor-renderer/src/nodes/mod.rs
mod container;
mod line;
mod atom;
mod placeholder;

use editor_model::Doc;
use editor_view::fragment::Fragment;
use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::Transform;

pub fn render_fragment(
    renderer: &Renderer,
    sink: &mut impl RenderSink,
    fragment: &Fragment,
    doc: &Doc,
    parent_node_type: Option<&str>,  // 간이 노드 타입 식별자
    transform: Transform,
) {
    match fragment {
        Fragment::Container(cf) => container::draw(renderer, sink, cf, doc, transform),
        Fragment::Line(lf) => line::draw(renderer, sink, lf, transform),
        Fragment::Atom(af) => atom::draw(renderer, sink, af, doc, transform),
        Fragment::Placeholder(pf) => {
            placeholder::draw(renderer, sink, pf, doc, parent_node_type, transform)
        }
    }
}
```

- [ ] **Step 3: nodes/container.rs**

```rust
// crates/editor-renderer/src/nodes/container.rs
use editor_common::Rect;
use editor_model::{Doc, NodeType};
use editor_view::fragment::ContainerFragment;
use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::{Transform, Path, Stroke};

pub fn draw(
    renderer: &Renderer,
    sink: &mut impl RenderSink,
    cf: &ContainerFragment,
    doc: &Doc,
    transform: Transform,
) {
    let t = transform.translate(cf.rect.x, cf.rect.y);
    let local_rect = Rect { x: 0.0, y: 0.0, width: cf.rect.width, height: cf.rect.height };

    // 노드 타입 조회
    let node_type = get_node_type(doc, cf.node_id);

    // 1. 배경
    draw_background(renderer, sink, &local_rect, &node_type, t);

    // 2. 자식 재귀
    for child in &cf.children {
        super::render_fragment(renderer, sink, child, doc, Some(&node_type), t);
    }

    // 3. 테두리
    draw_border(renderer, sink, &local_rect, &cf.border, &node_type, t);
}

fn draw_background(
    renderer: &Renderer,
    sink: &mut impl RenderSink,
    rect: &Rect,
    node_type: &str,
    transform: Transform,
) {
    // 노드 타입별 배경 결정
    // 실제 구현 시 Doc의 NodeType enum에 맞게 match
    match node_type {
        "callout" => {
            // theme.color_with_alpha("ui.callout.{variant}", 8)
            // variant 정보는 Doc에서 추가 조회 필요
        }
        "fold" | "fold_title" => {
            let color = renderer.theme.color("ui.surface.muted");
            sink.fill_rect(*rect, color, transform);
        }
        _ => {}
    }
}

fn draw_border(
    renderer: &Renderer,
    sink: &mut impl RenderSink,
    rect: &Rect,
    border: &editor_common::EdgeInsets,
    node_type: &str,
    transform: Transform,
) {
    if border.top == 0.0 && border.left == 0.0 && border.bottom == 0.0 && border.right == 0.0 {
        return;
    }

    let color = renderer.theme.color("ui.border.default");
    let path = Path::rect(*rect);
    let stroke = Stroke::new(border.top.max(border.left).max(border.bottom).max(border.right));
    sink.stroke_path(&path, color, &stroke, transform);
}

fn get_node_type(doc: &Doc, node_id: editor_common::NodeId) -> String {
    // Doc에서 node type 조회
    // 실제 구현 시 doc.node(node_id).node_type() 등의 API 사용
    // MVP에서는 간이 구현
    "unknown".to_string()
}
```

Note: `get_node_type`과 배경/테두리 로직은 Doc의 실제 NodeType API에 맞게 구현해야 한다. 위 코드는 구조를 보여주기 위한 스켈레톤이며, editor-model의 NodeRef/NodeType API를 읽고 정확한 match 구현을 작성할 것.

- [ ] **Step 4: nodes/line.rs**

```rust
// crates/editor-renderer/src/nodes/line.rs
use editor_view::fragment::LineFragment;
use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::Transform;
use editor_common::Rect;

pub fn draw(
    renderer: &Renderer,
    sink: &mut impl RenderSink,
    lf: &LineFragment,
    transform: Transform,
) {
    let t = transform.translate(lf.rect.x, lf.rect.y);

    for run in &lf.glyph_runs {
        // 배경 (하이라이트)
        if let Some(ref bg_token) = run.background_color {
            let bg_color = renderer.theme.color(bg_token);
            let run_rect = Rect { x: run.x, y: 0.0, width: run.width, height: lf.rect.height };
            sink.fill_rect(run_rect, bg_color, t);
        }

        // 텍스트
        let color = renderer.theme.color(&run.color);
        sink.draw_glyphs(run, color, t);
    }
}
```

- [ ] **Step 5: nodes/atom.rs**

```rust
// crates/editor-renderer/src/nodes/atom.rs
use editor_common::Rect;
use editor_model::Doc;
use editor_view::fragment::AtomFragment;
use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::Transform;

pub fn draw(
    renderer: &Renderer,
    sink: &mut impl RenderSink,
    af: &AtomFragment,
    doc: &Doc,
    transform: Transform,
) {
    let t = transform.translate(af.rect.x, af.rect.y);
    let local_rect = Rect { x: 0.0, y: 0.0, width: af.rect.width, height: af.rect.height };

    // Doc에서 노드 타입 조회
    // HorizontalRule → variant에 따라 icons.resolve("hr/{variant}", rect)
    // Image/File/Embed → skip (플랫폼 렌더링)

    // MVP: HR을 placeholder rect로 그리기
    let color = renderer.theme.color("ui.text.default");
    let path = renderer.icons.resolve("hr/line", local_rect);
    sink.fill_path(&path, color, t);
}
```

- [ ] **Step 6: nodes/placeholder.rs**

```rust
// crates/editor-renderer/src/nodes/placeholder.rs
use editor_model::Doc;
use editor_view::fragment::{PlaceholderFragment, PlaceholderData};
use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::Transform;
use editor_common::Rect;

pub fn draw(
    renderer: &Renderer,
    sink: &mut impl RenderSink,
    pf: &PlaceholderFragment,
    _doc: &Doc,
    parent_node_type: Option<&str>,
    transform: Transform,
) {
    let t = transform.translate(pf.rect.x, pf.rect.y);
    let local_rect = Rect { x: 0.0, y: 0.0, width: pf.rect.width, height: pf.rect.height };

    let (icon_name, color_token) = match (parent_node_type, pf.id) {
        (Some("callout"), 0) => ("lucide/info", "ui.callout.info"),
        (Some("blockquote"), 0) => ("typie/blockquote-quote", "ui.text.muted"),
        (Some("fold"), 0) => {
            let expanded = matches!(pf.data, PlaceholderData::Bool(true));
            if expanded {
                ("lucide/chevron-up", "ui.text.faint")
            } else {
                ("lucide/chevron-down", "ui.text.faint")
            }
        }
        _ => return, // 알 수 없는 placeholder — 무시
    };

    let path = renderer.icons.resolve(icon_name, local_rect);
    let color = renderer.theme.color(color_token);
    sink.fill_path(&path, color, t);
}
```

- [ ] **Step 7: lib.rs 업데이트**

```rust
pub mod icons;
pub mod nodes;
pub mod renderer;
pub mod sink;
pub mod theme;
pub mod types;

pub use icons::IconRegistry;
pub use renderer::Renderer;
pub use sink::RenderSink;
pub use theme::Theme;
pub use types::*;
```

- [ ] **Step 8: 컴파일 확인**

Run: `cargo check -p editor-renderer`
Expected: 성공 (Doc API 등 실제 타입에 맞게 조정 필요할 수 있음)

- [ ] **Step 9: Mock sink 테스트**

```rust
// crates/editor-renderer/src/renderer.rs 하단 tests 모듈

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::{Color, Rect};
    use crate::types::{Transform, Path, Stroke};
    use editor_view::fragment::GlyphRun;

    #[derive(Default)]
    struct MockSink {
        fill_rects: Vec<(Rect, Color)>,
        fill_paths: Vec<Color>,
        stroke_paths: Vec<Color>,
        draw_glyphs_calls: Vec<Color>,
    }

    impl RenderSink for MockSink {
        fn fill_rect(&mut self, rect: Rect, color: Color, _: Transform) {
            self.fill_rects.push((rect, color));
        }
        fn fill_path(&mut self, _: &Path, color: Color, _: Transform) {
            self.fill_paths.push(color);
        }
        fn stroke_path(&mut self, _: &Path, color: Color, _: &Stroke, _: Transform) {
            self.stroke_paths.push(color);
        }
        fn draw_glyphs(&mut self, _: &GlyphRun, color: Color, _: Transform) {
            self.draw_glyphs_calls.push(color);
        }
    }

    #[test]
    fn test_render_empty_page() {
        let renderer = Renderer::new(
            IconRegistry::new(),
            Arc::new(FontRegistry::new()),
            Theme::new(Default::default()),
        );
        let page = Page { body: vec![], margin_top: 0.0, content_height: 0.0 };
        let doc = Doc::default();  // 또는 테스트용 빈 Doc 생성
        let mut sink = MockSink::default();

        renderer.render_page(&mut sink, &page, &doc);

        assert!(sink.fill_rects.is_empty());
        assert!(sink.draw_glyphs_calls.is_empty());
    }
}
```

Note: `Page`와 `Doc`의 테스트 생성자는 실제 API를 확인하여 작성할 것.

Run: `cargo test -p editor-renderer`
Expected: 테스트 통과

---

## Self-Review Notes

**Spec coverage:**
- [x] PlaceholderFragment + PlaceholderData → Task 2
- [x] ContainerFragment + border → Task 3
- [x] LineFragment glyph_runs (LineSegment 교체) → Task 3, 4
- [x] GlyphRun 통합 (렌더 + 커서) → Task 2, 4
- [x] MeasuredLine 변경 → Task 3
- [x] line_extraction 확장 → Task 6
- [x] measure_nodes Placeholder 추가 → Task 7
- [x] Paginator 변경 → Task 5
- [x] editor-renderer crate (RenderSink, primitive types) → Task 8
- [x] Theme, IconRegistry → Task 9
- [x] Renderer + render_page → Task 10
- [x] 노드별 렌더링 (container, line, atom, placeholder) → Task 10

**Known limitations (MVP):**
- font_id resolve는 placeholder 값 (0). FontRegistry 확장은 후속.
- IconRegistry는 rect fallback만 반환. 실제 SVG path 생성은 레거시 매크로 시스템 연동 후.
- 노드 타입 조회는 스켈레톤. Doc API에 맞게 구현 필요.
- color token resolve (FontColor modifier)는 placeholder. resolve_text_style 패턴 참조하여 구현.
- Ordered list marker text 계산은 후속.
