# View Renderer 설계

Fragment 기반 렌더링 파이프라인. editor-view의 Fragment tree + Doc을 읽어 화면에 그리는 editor-renderer crate 설계.

## 설계 원칙

1. **Fragment = Layout Tree** — Fragment는 순수 layout 정보 (위치, 크기, placeholder 공간 확보). 색상, 아이콘 이름 등 렌더링 정보를 포함하지 않음.
2. **렌더러가 시각 결정** — 렌더러가 Doc에서 노드 타입을 읽고, Theme에서 색상을, IconRegistry에서 path를 해석하여 "무엇을 어떻게 그릴지" 결정.
3. **Placeholder 패턴** — 장식 요소(아이콘, 마커 등)의 위치는 editor-view가 layout으로 결정. 렌더러는 placeholder의 부모 노드 타입에 따라 적절한 시각 요소를 그림. (parley inline box에서 영감)
4. **라이브러리 독립** — 공개 API에 외부 라이브러리 타입(kurbo, peniko, parley) 노출 없음. 자체 primitive 타입 사용. sink 구현부에서만 변환.

## Crate 구조

```
crates/editor-renderer/
  src/
    lib.rs              — pub mod + re-exports
    sink.rs             — RenderSink trait
    renderer.rs         — Renderer struct + render_page
    theme.rs            — Theme struct
    icons.rs            — IconRegistry
    types.rs            — 자체 primitive 타입
    nodes/
      mod.rs            — render_fragment dispatch
      container.rs      — draw_container_background, draw_container_border
      line.rs           — draw_line (glyph_runs 렌더링)
      atom.rs           — draw_atom (HR variants 등)
      placeholder.rs    — draw_placeholder (아이콘, 마커)
```

RenderSink의 구현체(CpuSink, GpuSink)는 기존 레거시 crate(`crates/editor/`)에 머무른다. 새 RenderSink trait을 impl하는 어댑터를 플랫폼 레이어에서 제공.

## 의존성

```
editor-renderer → editor-view  (Fragment, Page)
               → editor-model (Doc, NodeRef — 노드 타입 조회)
               → editor-common (NodeId, Rect, Size, Color, FontRegistry)
```

editor-renderer는 Doc을 렌더링 시점에 읽어 노드 타입별 시각 결정을 수행한다.

## RenderSink Trait

외부 라이브러리 타입을 사용하지 않는 자체 primitive 기반:

```rust
pub trait RenderSink {
    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform);
    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform);
    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform);
    fn draw_glyphs(&mut self, run: &GlyphRun, color: Color, transform: Transform);
}
```

### 자체 Primitive 타입

editor-common:
- `Color` — RGBA8. Theme 등 렌더링 외에서도 사용.

editor-renderer:
- `Transform` — 2D affine (6 floats)
- `Path`, `PathElement` — MoveTo, LineTo, QuadTo, CurveTo, Close
- `Stroke` — width, dash 옵션
- `Glyph` — `{ id: u32, x: f32, y: f32 }`
- `Synthesis` — `{ embolden: bool, skew: Option<f32> }`

sink 구현부에서 자체 타입 → 라이브러리 타입 변환을 경계에서 수행.

## Renderer Struct

```rust
pub struct Renderer {
    icons: IconRegistry,
    fonts: Arc<FontRegistry>,   // ShapingContext와 공유
    theme: Theme,
}

impl Renderer {
    pub fn new(icons: IconRegistry, fonts: Arc<FontRegistry>, theme: Theme) -> Self;
    pub fn set_theme(&mut self, theme: Theme);
    pub fn render_page(&self, sink: &mut impl RenderSink, page: &Page, doc: &Doc);
}
```

소유 관계:
- `IconRegistry` — 아이콘 name → Path 생성. Renderer 소유.
- `FontRegistry` — FontId → 폰트 데이터. ShapingContext와 Arc로 공유.
- `Theme` — 색상 토큰 → Color 해석. Renderer 소유. `set_theme()`으로 갱신.

## Theme

```rust
pub struct Theme {
    colors: HashMap<String, Color>,
}

impl Theme {
    pub fn color(&self, token: &str) -> Color;
    pub fn color_with_alpha(&self, token: &str, alpha: u8) -> Color;
}
```

## IconRegistry

아이콘 이름 + rect → Path 생성:

```rust
pub struct IconRegistry { /* 내부 */ }

impl IconRegistry {
    /// name + rect로 Path 생성
    /// - "lucide/info" → 컴파일타임 SVG 데이터에서 path, rect에 맞게 스케일
    /// - "hr/zigzag" → rect 크기 기반 동적 path 생성
    /// - "typie/blockquote-quote" → 커스텀 SVG 아이콘
    pub fn resolve(&self, name: &str, rect: Rect) -> Path;
}
```

기존 `svg_icon_path!` 매크로의 컴파일타임 SVG 파싱 결과와 HR 변형의 절차적 생성 로직을 등록.

## Fragment 변경 (editor-view)

### PlaceholderFragment (신규)

장식 요소의 위치를 layout으로 잡는 placeholder. parley의 inline box에서 영감.

```rust
pub struct PlaceholderFragment {
    pub id: u32,              // 측정 단계에서 부여
    pub rect: Rect,
    pub data: PlaceholderData,
}

pub enum PlaceholderData {
    None,
    Bool(bool),
    Number(f64),
    Text(String),
}
```

Fragment enum에 새 variant 추가:

```rust
pub enum Fragment {
    Container(ContainerFragment),
    Line(LineFragment),
    Atom(AtomFragment),
    Placeholder(PlaceholderFragment),   // NEW
}
```

측정 단계에서 장식 요소가 필요한 노드는 Placeholder를 자식으로 추가:

```
ContainerFragment (callout, node_id=42)
  ├── PlaceholderFragment { id: 0, rect: (12, 16, 16, 16) }
  ├── LineFragment { ... }
  └── ...
```

렌더러는 `(부모 node type, placeholder id)` 조합으로 무엇을 그릴지 결정:

| 노드 | id=0 | data |
|------|------|------|
| Callout(info) | lucide/info 아이콘 | None |
| Blockquote | typie/blockquote-quote 아이콘 | None |
| Fold | lucide/chevron 아이콘 | Bool(expanded) |
| ListItem(Bullet) | bullet 점 | None |
| ListItem(Ordered) | 숫자 마커 | Text("1.") |

### ContainerFragment 변경

```rust
pub struct ContainerFragment {
    pub node_id: NodeId,
    pub rect: Rect,
    pub children: Vec<Fragment>,
    pub scope: bool,
    pub breaks: Breaks,
    pub border: EdgeInsets,    // NEW — layout 정보. 렌더러가 테두리 위치/두께로 사용.
}
```

border는 layout 정보다 — 자식 배치에 영향을 주며 (content area = rect - border - padding), Paginator가 이미 사용하고 있다. 렌더러는 이 값을 테두리 그리기에 활용한다.

배경색, 테두리색, 아이콘 이름 등 시각 정보는 Fragment에 포함하지 않는다. 렌더러가 Doc에서 노드 타입을 읽어 결정한다.

### LineFragment 변경 — GlyphRun 통합

기존 `segments: Vec<LineSegment>`를 `glyph_runs: Vec<GlyphRun>`으로 교체:

```rust
pub struct LineFragment {
    pub node_id: NodeId,
    pub rect: Rect,
    pub baseline: f32,
    pub glyph_runs: Vec<GlyphRun>,    // CHANGED — segments 대체
}
```

GlyphRun은 렌더링(glyph 데이터)과 커서 내비게이션(char_advances)을 통합:

```rust
pub struct GlyphRun {
    // 렌더용
    pub font_id: FontId,
    pub font_size: f32,
    pub synthesis: Synthesis,
    pub color: String,                  // theme token
    pub background_color: Option<String>, // theme token (하이라이트)
    pub glyphs: Vec<Glyph>,

    // 커서용
    pub node_id: NodeId,
    pub offset: usize,                  // 텍스트 노드 내 code point offset
    pub text: String,
    pub char_advances: Vec<f32>,        // code point 단위 advance
}
```

**왜 Segment와 GlyphRun을 통합하는가:**

parley는 스타일 경계에서 run을 분리하므로 각 glyph run은 하나의 node_id에 대응한다. 폰트 fallback으로 하나의 텍스트 노드가 여러 glyph run을 가질 수 있다:

```
"Hello 안녕" (node_id=55)
→ GlyphRun { font: Latin, node_id: 55, offset: 0, text: "Hello " }
→ GlyphRun { font: CJK,   node_id: 55, offset: 6, text: "안녕" }
```

커서 코드는 `node_id + offset + char_advances`를, 렌더 코드는 `font_id + glyphs + color`를 읽는다.

**GlyphRun의 color token:**

텍스트 색상(color)과 배경색(background_color)은 theme token이다. 이 token은 Doc의 FontColor/Highlight modifier에서 유래하며, 측정 단계에서 resolve된다. 텍스트 스타일(font, weight, size)을 셰이핑을 위해 resolve하는 것과 동일한 맥락이다 — 문서 수준의 텍스트 속성이지 container 장식과는 다르다.

### AtomFragment — 변경 없음

```rust
pub struct AtomFragment {
    pub node_id: NodeId,
    pub parent_id: NodeId,
    pub index: usize,
    pub rect: Rect,
}
```

HR의 variant는 렌더러가 Doc에서 노드를 읽어 결정한다. Image/File/Embed은 외부 플랫폼이 렌더링하므로 렌더러는 건너뛴다.

## 렌더링 흐름

```
Renderer.render_page(sink, page, doc)
  for fragment in page.body:
    render_fragment(sink, fragment, doc, Transform::identity())

render_fragment(sink, fragment, doc, parent_node_type, transform):
  match fragment:
    Container(cf) →
      t = transform.translate(cf.rect.x, cf.rect.y)
      node_type = doc.node_type(cf.node_id)
      // 1. 배경 그리기 (node type에 따라)
      draw_container_background(sink, cf, node_type, t)
      // 2. 자식 재귀 — 현재 node_type을 부모로 전달
      for child in cf.children:
        render_fragment(sink, child, doc, Some(node_type), t)
      // 3. 테두리 그리기 (cf.border 사용)
      draw_container_border(sink, cf, node_type, t)

    Line(lf) →
      t = transform.translate(lf.rect.x, lf.rect.y)
      for run in lf.glyph_runs:
        color = theme.color(&run.color)
        if let Some(bg) = &run.background_color:
          sink.fill_rect(run_rect, theme.color(bg), t)
        sink.draw_glyphs(run, color, t)

    Atom(af) →
      t = transform.translate(af.rect.x, af.rect.y)
      node_type = doc.node_type(af.node_id)
      draw_atom(sink, af, node_type, t)

    Placeholder(pf) →
      t = transform.translate(pf.rect.x, pf.rect.y)
      draw_placeholder(sink, pf, parent_node_type, t)
```

### Container 배경/테두리 결정

렌더러가 node type 기반으로 결정:

| 노드 타입 | 배경 | 테두리 |
|-----------|------|--------|
| Callout(variant) | theme.color("ui.callout.{variant}"), alpha=8 | 왼쪽 2px, theme.color("ui.callout.{variant}") |
| Blockquote | 없음 | 왼쪽 2px, theme.color("ui.border.default") |
| Fold/FoldTitle | theme.color("ui.surface.muted") | 전체 1px, theme.color("ui.border.default") |
| Table/TableCell | 없음 | cf.border 기반, theme.color("ui.border.default") |
| 기본 | 없음 | 없음 |

### Placeholder 시각 결정

렌더러가 `(부모 node type, placeholder id)` 기반으로 결정:

| 부모 노드 | id | 그리기 |
|-----------|-----|--------|
| Callout(variant) | 0 | icons.resolve("lucide/{variant_icon}", pf.rect) |
| Blockquote::Quote | 0 | icons.resolve("typie/blockquote-quote", pf.rect) |
| Fold | 0 | data=Bool(true) → chevron-up, Bool(false) → chevron-down |
| ListItem(Bullet) | 0 | fill_rect로 bullet 점 그리기 |
| ListItem(Ordered) | 0 | data=Text("1.") → sink.draw_glyphs로 숫자 텍스트 렌더링 |

### Atom 시각 결정

| 노드 타입 | 그리기 |
|-----------|--------|
| HorizontalRule(variant) | variant에 따라 icons.resolve("hr/{variant}", af.rect) 또는 fill_rect 조합 |
| Image/File/Embed/Archived | 건너뜀 (외부 플랫폼 렌더링) |

## editor-view 변경 사항

### measure_nodes 확장

장식 요소가 필요한 노드의 measure 함수에서 Placeholder를 자식으로 추가:

- `measure_callout` → 아이콘 영역 Placeholder 추가
- `measure_blockquote` → 인용 아이콘 Placeholder 추가
- `measure_fold` → chevron 아이콘 Placeholder 추가
- `measure_list_item` → marker Placeholder 추가

padding 계산은 기존과 동일. Placeholder는 padding 영역 안에 위치하며, 자식 measure와 독립적이다.

### line_extraction 확장

`extract_measured_lines`에서 parley Layout의 glyph run 데이터를 캡처:

기존: cluster 메트릭만 추출 → LineSegment (text, x, width, char_advances)
변경: glyph run 순회 → GlyphRun (font_id, glyphs, synthesis + node_id, offset, text, char_advances, color)

텍스트 색상 token은 Doc의 FontColor modifier에서 resolve. 이 resolve는 line_extraction 단계에서 수행.

### Paginator 변경

- Placeholder를 Fragment 변환 시 그대로 전달
- ContainerFragment에 border: EdgeInsets 포함 (기존 Measurement의 border 값)

## 관심사 분리 요약

```
editor-view (layout):
  - 위치, 크기, padding, border
  - Placeholder로 장식 요소 공간 확보
  - GlyphRun으로 텍스트 속성 (font, color) 캡처

editor-renderer (rendering):
  - Doc 읽기: 노드 타입 → 배경/테두리/아이콘 결정
  - Theme: 색상 토큰 → 실제 Color 해석
  - IconRegistry: 아이콘 이름 → Path 생성
  - FontRegistry: FontId → 폰트 데이터 조회
  - RenderSink: 실제 그리기 실행
```

## MVP 범위

포함:
- PlaceholderFragment 추가 + measure_nodes에서 Placeholder 생성
- GlyphRun 통합 (LineSegment 교체) + line_extraction 확장
- ContainerFragment에 border 추가
- editor-renderer crate 생성 (Renderer, RenderSink, 자체 primitive 타입)
- Container 배경/테두리 렌더링
- 텍스트 렌더링 (GlyphRun 기반)
- Placeholder 렌더링 (아이콘)
- Atom 렌더링 (HR variants)
- Theme, IconRegistry 기본 구현

제외 (후속):
- Selection/Cursor 렌더링 (별도 overlay pass)
- 페이지 캐싱 (PageCache, dirty rect)
- GPU backend
- Export backend
