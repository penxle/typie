# View — Paragraph → TextBlock 측정 설계

## 목적

Paragraph 노드의 자식 Text 노드들을 parley로 텍스트 셰이핑 및 줄바꿈하여 `MeasuredContent::TextBlock { lines: Vec<MeasuredLine> }`을 생성한다.

## 범위

- 기본 텍스트 셰이핑 (modifier → parley 스타일, 줄바꿈, MeasuredLine/LineSegment 생성)
- Font mapping (문자별 폰트 대체, CJK 등)
- Strut 기반 최소 line height 보장

제외:
- Preedit (IME)
- Ruby text

## 핵심 설계 결정

### 1. Parley 직접 의존

editor-view가 parley에 직접 의존한다. Trait 추상화 없음.

- parley는 Rust 텍스트 레이아웃의 사실상 유일한 선택지로, 교체 가능성이 낮아 추상화는 YAGNI
- 텍스트 셰이핑은 editor-view의 핵심 책임에 해당

### 2. ShapingContext — 에디터 간 공유 자원

```rust
pub struct ShapingContext {
    pub fcx: FontContext,
    pub lcx: LayoutContext<TextBrush>,
    pub font_registry: FontRegistry,
}
```

- `LayoutEngine`이 `Arc<Mutex<ShapingContext>>`를 보유
- 모든 에디터 인스턴스가 하나의 ShapingContext를 공유
- parley의 `FontContext`와 `LayoutContext<TextBrush>`는 모두 Send + Sync이므로 안전
- font mapping 테이블과 family interning은 `FontRegistry`가 관리 (아래 참조)

**FontRegistry 확장:**

```rust
pub struct FontRegistry {
    families: FxHashMap<String, SmallVec<[u16; 9]>>,
    // family name interning
    family_names: Vec<String>,
    family_index: FxHashMap<String, u16>,
    // codepoint별 폰트 매핑: (family_id, weight) → { codepoint → (resolved_family_id, resolved_weight) }
    font_mappings: FxHashMap<(u16, u16), FxHashMap<u32, (u16, u16)>>,
}
```

- family_id는 intern된 u16 인덱스 — 매핑 테이블의 키/값 크기를 최소화
- `font_mappings`는 외부에서 `FontsLoaded` 메시지로 비동기 갱신됨

**LayoutEngine 변경:**

```rust
pub struct LayoutEngine {
    pub cache: LayoutCache,
    pages: Vec<Page>,
    shaping: Arc<Mutex<ShapingContext>>,
}
```

`compute()` 시그니처는 변경 없음. 내부에서 `self.shaping.lock()`으로 접근한다.

### 3. TextBrush — 식별 정보만

```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextBrush {
    pub node_id: NodeId,
}
```

- 렌더링 정보(color, bold 등)는 담지 않음 — 렌더러가 Doc에서 node_id로 직접 조회
- parley glyph run 출력에서 `brush.node_id`를 읽어 `LineSegment.node_id`를 설정하는 용도

### 4. Strut — CSS 표준 모델

Paragraph당 단일 StrutMetrics. 레거시의 per-line strut 방식과 다르다.

```rust
pub struct StrutMetrics {
    pub ascent: f32,
    pub descent: f32,
}
```

- Paragraph의 cascade 기본 폰트(family, weight, size)로 skrifa에서 측정
- 모든 line에 동일하게 적용 — 최소 ascent/descent를 보장
- 인라인 콘텐츠가 strut보다 크면 확장, 작아도 strut 아래로 줄어들지 않음

**레거시와의 차이:** paragraph 기본 폰트보다 작은 인라인 텍스트가 있는 줄에서, 레거시는 줄 높이가 줄어들었으나 새 설계는 기본 폰트 기준 최소 높이를 유지한다. CSS inline formatting context 표준에 부합.

## 타입 정의

### ResolvedTextStyle

측정에 실제로 영향을 주는 속성만 포함. Bold, Italic, Underline, Strikethrough는 faux/합성 처리되어 셰이핑에 영향 없으므로 제외.

```rust
pub struct ResolvedTextStyle {
    pub font_family: String,
    pub font_weight: u16,
    pub font_size: f32,        // px
    pub letter_spacing: f32,   // px
    pub line_height: f32,      // ratio (1.6 = 160%)
}
```

### TextRun

`collect_text_runs`의 출력 단위. 하나의 Text 노드에 대응.

```rust
pub struct TextRun {
    pub node_id: NodeId,
    pub byte_range: Range<usize>,
    pub style: ResolvedTextStyle,
}
```

### FontRun

Font mapping 결과. 실제 렌더링에 사용할 폰트.

```rust
pub struct FontRun {
    pub byte_range: Range<usize>,
    pub family: String,
    pub weight: u16,
}
```

## 파이프라인

4단계 파이프라인. 각 단계가 명확한 입출력을 갖는다.

### 1단계: collect_text_runs

```rust
fn collect_text_runs(doc: &Doc, paragraph: &NodeRef) -> (String, Vec<TextRun>)
```

- Paragraph의 자식 Text 노드를 순회하여 텍스트를 하나의 String으로 합침
- 각 Text 노드의 modifier를 resolve하여 TextRun 생성 (byte_range는 합쳐진 문자열 내 위치)

StrutMetrics는 이 단계에서 계산하지 않는다. skrifa 폰트 메트릭 접근이 필요하므로 ShapingContext lock을 잡은 뒤 `measure_paragraph` 조립 단계에서 별도 계산한다.

**Modifier 해석:** `resolve_text_style(doc, node) -> ResolvedTextStyle`

- 조상 체인을 한 번 순회하면서 모든 텍스트 관련 modifier를 한꺼번에 수집
- cascading 원칙은 기존 `resolve_inherited`와 동일 (자신 → 조상 방향, 타입별 첫 매칭)
- 한 번의 워킹으로 처리하여 Text 노드가 많은 문서에서의 성능을 확보

**단위 변환:**
- FontSize: `modifier_value / 100.0` → pt, `pt * 96.0 / 72.0` → px
- LetterSpacing: `modifier_value / 100.0 * font_size_px` (em → px)
- LineHeight: `modifier_value / 100.0` (ratio)

### 2단계: resolve_font_mapping

```rust
fn resolve_font_mapping(
    text: &str,
    runs: &[TextRun],
    font_registry: &FontRegistry,
) -> Vec<FontRun>
```

- 각 TextRun 내에서 문자별로 `font_mappings` 테이블을 조회 (codepoint 기반)
- 매핑이 있으면 대체 폰트 사용, 없으면 원래 폰트에서 `FontRegistry.nearest_weight()`로 weight 결정
- 같은 (family, weight)의 연속 문자를 하나의 FontRun으로 병합

### 3단계: build_parley_layout

```rust
fn build_parley_layout(
    text: &str,
    runs: &[TextRun],
    font_runs: &[FontRun],
    align: TextAlign,
    indent: f32,
    width: f32,
    shaping: &mut ShapingContext,
) -> Layout<TextBrush>
```

- Style run builder로 순차적으로 text + style을 push
- TextRun에서: font_size, font_weight, letter_spacing, LineHeight 적용
- FontRun 경계에서 run을 분할하여 실제 폰트 family/weight 적용
- TextBrush { node_id }를 각 run에 태깅
- `builder.build(text)`
- `layout.set_indent(indent)`
- `layout.break_all_lines(width)`
- `layout.align(align)`

### 4단계: extract_measured_lines

```rust
fn extract_measured_lines(
    text: &str,
    layout: &Layout<TextBrush>,
    strut: &StrutMetrics,
    line_height_ratio: f32,
    base_font_size: f32,
) -> Vec<MeasuredLine>
```

- `layout.lines()` 순회
- 각 line의 glyph run에서 cluster 순회 → `char_advances: Vec<f32>` 계산
- byte offset → char offset 변환
- `brush.node_id`로 `LineSegment.node_id` 설정

**Line height 계산 (line별):**
1. Parley의 line ascent/descent 추출
2. Strut minimum 적용: `ascent = max(parley_ascent, strut.ascent)`, `descent = max(parley_descent, strut.descent)`
3. `line_box_height = max(base_font_size × line_height_ratio, ascent + descent)`
4. `leading = line_box_height - (ascent + descent)`, 위아래 균등 분배
5. `baseline = leading / 2 + ascent`

## measure_paragraph 조립

```rust
fn measure_paragraph(
    &mut self,
    doc: &Doc,
    node: &NodeRef,
    width: f32,
) -> Measurement
```

- 1단계(collect_text_runs)는 lock 없이 실행
- ShapingContext lock을 잡고: StrutMetrics 계산 (skrifa) → 2단계 (FontRegistry) → 3단계 (fcx/lcx)
- lock 해제 후: 4단계(extract_measured_lines) 실행

**반환:**
```rust
Measurement {
    size: Size { width, height: 모든 line의 line_box_height 합 },
    gap_after: resolve_gap_after(doc, node),
    alignment: Alignment::Start,
    content: MeasuredContent::TextBlock {
        lines,  // extract_measured_lines 결과
    },
}
```

**measure_inner dispatch 추가:**
```rust
NodeType::Paragraph => measure_paragraph(self, doc, node, width),
```

## 파일 구조

```
crates/editor-view/src/
  shaping.rs                              — ShapingContext, TextBrush, StrutMetrics
  engine/measure_nodes/
    paragraph.rs                          — measure_paragraph (파이프라인 조립)
    paragraph/
      text_run.rs                         — TextRun, ResolvedTextStyle, collect_text_runs
      font_run.rs                         — FontRun, resolve_font_mapping
      layout_builder.rs                   — build_parley_layout
      line_extraction.rs                  — extract_measured_lines
  engine/resolve.rs                       — resolve_text_style 추가
```

## 의존 관계

```
editor-view
  ├── parley (직접 의존, 텍스트 셰이핑)
  ├── skrifa (StrutMetrics 계산용 폰트 메트릭)
  ├── editor-model (Doc, NodeRef, Modifier, TextNode, ParagraphNode)
  └── editor-common (FontRegistry, geometry)
```
