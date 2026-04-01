# View — Remaining Work

editor-view crate의 남은 작업. 현재 골격(Fragment, Measurement, Paginator, cursor, View facade)은 구현 완료.

## 완료된 작업

### measure_inner: Atom + Container 측정

설계: `view-measure-atom-container-design.md`

**타입 추가:**
- `Alignment` (Start/Center/End) — editor-common geometry.rs
- `EdgeInsets` (top/left/bottom/right + ZERO, symmetric) — editor-common geometry.rs
- `Measurement`에 `alignment` 필드 추가
- `MeasuredContent::Container`에 `padding` 필드 추가

**editor-model 변경:**
- `NodeRef::entry()` 반환 타입을 `&'a NodeEntry`로 수정 (lifetime 정확성)

**measure_inner dispatch:**
- Atom 노드 (Image, File, Embed, Archived, HorizontalRule) → `measure_atom`
- ListItem → `measure_list_item` (padding.left = 28.0)
- Blockquote → `measure_blockquote` (4개 variant별 padding + MessageSent alignment)
- Callout → `measure_callout` (padding = {16, 40, 16, 12})
- 기본 Container → `measure_default_container` (gap_after resolve 포함)
- PageBreak → 기존 로직 유지

**gap_after:**
- `resolve_inherited()` — 조상 체인에서 Modifier를 cascading resolve
- `resolve_gap_after()` — BlockGap modifier를 px로 변환 (value / 100.0 * 16.0)
- Root, Text, PageBreak에는 적용하지 않음

**Paginator:**
- `OpenContainer`에 `padding` 필드 추가
- `current_x()` — 중첩 container의 padding.left 누적
- `child_x()` — cross-axis alignment 반영 (Start/Center/End)
- `break_page()`에서 padding 보존 및 재적용

**파일 구조:**
```
crates/editor-common/src/geometry.rs        — Alignment, EdgeInsets 추가
crates/editor-model/src/node_ref.rs         — entry() lifetime 수정
crates/editor-view/src/measure.rs           — Measurement, MeasuredContent 변경
crates/editor-view/src/engine/resolve.rs    — resolve_inherited, resolve_gap_after
crates/editor-view/src/engine/measure_nodes/
  mod.rs                                    — re-exports
  atom.rs                                   — measure_atom
  container.rs                              — measure_padded_container, measure_default_container
  list_item.rs                              — measure_list_item
  blockquote.rs                             — measure_blockquote
  callout.rs                                — measure_callout
crates/editor-view/src/engine/paginator.rs  — padding + alignment 지원
```

---

## 1. measure_inner 구현 — 남은 항목

### ~~Fold 처리~~ (완료)

설계: `view-measure-fold-table-design.md`

- measure_fold: ViewState.fold_expanded 기반 접힘/펼침, border={all:1} Separate
- measure_fold_title: padding={top:8, left:40, bottom:8, right:12} (아이콘 영역 포함)
- measure_fold_content: padding=symmetric(24, 16)

### ~~Table 열 너비 계산 + Horizontal Row 측정~~ (완료)

설계: `view-measure-fold-table-design.md`

- measure_table: TableWidthModel 포팅, col_widths 계산, Row 직접 구성 (2-pass height 균등화)
- measure_table_cell: padding={all:8}, border={all:1}, scope=true
- Table/Row: border={all:1} Collapse 모드
- Paginator: BorderMode::Separate/Collapse 지원, border collapse overlap 배치

**추가된 타입:**
- `BorderMode` (Separate/Collapse) — measure.rs
- `MeasuredContent::Container`에 `border: EdgeInsets`, `border_mode: BorderMode` 필드

**파일 구조:**
```
crates/editor-view/src/measure.rs              — BorderMode, ContainerContent에 border/border_mode
crates/editor-view/src/engine/measure_nodes/
  fold.rs                                      — measure_fold, measure_fold_title, measure_fold_content
  table.rs                                     — measure_table, measure_table_cell, 열 너비 계산
  container.rs                                 — measure_padded_container에 border/scope 파라미터
crates/editor-view/src/engine/paginator.rs     — Separate/Collapse border 배치
crates/editor-common/src/geometry.rs           — EdgeInsets::all()
crates/editor-model/src/nodes/table.rs         — TableNode Default 수정 (proportion=1.0)
crates/editor-model/src/nodes/image.rs         — ImageNode Default 수정 (proportion=1.0)
```

### ~~Paragraph → TextBlock 변환~~ (완료)

설계: `view-measure-paragraph-design.md`

- ShapingContext (FontContext + LayoutContext + FontRegistry) — Arc<Mutex<>>로 에디터 간 공유
- 4단계 파이프라인: collect_text_runs → resolve_font_mapping (문자별 codepoint 매핑) → build_parley_layout → extract_measured_lines
- CSS 표준 strut 모델로 최소 line height 보장
- FontRegistry에 family interning + codepoint 매핑 테이블 통합

**파일 구조:**
```
crates/editor-common/src/font.rs                  — FontRegistry 확장 (interning, font_mappings)
crates/editor-view/src/shaping.rs                  — ShapingContext, TextBrush, StrutMetrics
crates/editor-view/src/engine/resolve.rs           — resolve_text_style, resolve_paragraph_indent
crates/editor-view/src/engine/measure_nodes/
  paragraph/
    mod.rs                                         — measure_paragraph (파이프라인 조립)
    text_run.rs                                    — TextRun, collect_text_runs
    font_run.rs                                    — FontRun, resolve_font_mapping
    layout_builder.rs                              — build_parley_layout
    line_extraction.rs                             — extract_measured_lines
```

## ~~2. 커서 Movement 확장~~ (완료)

설계: `view-cursor-movement-design.md`

- Movement enum 재설계: 의미적 이동(Grapheme, Word, Sentence, Block, Document)과 시각적 이동(LineStart/End, LineUp/Down, PageUp/Down) 분리
- Word/Sentence: `icu_segmenter` WordSegmenter/SentenceSegmenter 사용, `TextSegmenters`로 묶어 Editor가 소유
- LineStart/End: 현재 줄의 first/last position
- Block: scope ContainerFragment의 first/last navigable
- PageUp/Down: viewport.height만큼 커서 이동
- Document: 문서 첫/마지막 navigable
- Viewport에 height 필드 추가

## 3. 렌더링

View 설계에서 범위 밖으로 둔 렌더러 연동. 별도 설계 필요.

### Fragment 기반 렌더링

렌더러는 Fragment Tree + Doc을 읽어서 그림:
- ContainerFragment → Doc에서 노드 타입 조회 → 장식 결정 (배경, 테두리, marker 등)
- LineFragment → 텍스트 렌더링 (segments의 text, x, char_advances)
- AtomFragment → 이미지/임베드/HR 렌더링
- Breaks → 모서리 처리 (Paginated: 잘린 쪽 둥근 모서리 → 직각)
- Wrapper 확장 영역 → 배경 채우기

### 기존 렌더 파이프라인

레거시는 CPU/GPU dual 파이프라인:
- CPU: 소프트웨어 렌더링 (fallback)
- GPU: wgpu 기반

이 파이프라인을 새 Fragment 구조에 맞게 연결해야 함.

### 페이지 캐싱

레거시는 `FxHashMap<usize, PageCache>`로 페이지별 렌더 결과를 캐싱. 새 구조에서도 Page 단위 캐싱이 필요.

## 4. 다단 레이아웃

현재 단일 컬럼만 지원. 문서 내에서 유저가 배치하는 2단 이상의 컬럼 레이아웃.

Measurement에서는 각 컬럼의 너비를 계산하고 자식을 해당 너비로 측정. Paginator에서는 Container(Vertical)로 자연스럽게 처리됨 (컬럼은 scope가 아니므로 커서가 자유롭게 넘나듦).

## 5. editor-core 연동 완성

현재 Editor의 stub:
- `handle_navigate()` — View.resolve_movement 연결 필요
- `handle_pointer()` — View.hit_test 연결 필요
- `process_effects()` — FontNeeded effect 처리 필요

이것들은 View의 실제 측정이 구현된 후에 연결 가능.

## 의존 관계

```
measure_inner 남은 구현 (Fold, Table, Paragraph)
  |
  +---> 렌더링 (Fragment에 실제 데이터가 있어야 그릴 수 있음)
  +---> Word/Sentence Movement (LineSegment에 실제 텍스트가 있어야 탐색 가능)
  +---> editor-core 연동 (실제 레이아웃이 있어야 navigate/pointer가 동작)

Block/Page/Document Movement
  (measure_inner 없이도 구현 가능 — Fragment 트리 구조만 있으면 됨)
```
