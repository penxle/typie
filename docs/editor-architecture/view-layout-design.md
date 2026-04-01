# View Layout System Design

## Overview

editor-view crate의 레이아웃 시스템. 브라우저 레이아웃 파이프라인을 모델로, 레거시 LayoutEngine/Element/CursorNavigable 구조를 Fragment 기반 파이프라인으로 대체한다.

## 설계 원칙

1. **레이아웃 출력은 순수 데이터** (Fragment) — 행동(메서드)을 갖지 않음
2. **소비자는 별도 모듈** — 커서/히트테스트가 Fragment를 읽기만 함
3. **레이아웃은 모델에서 분리** — Node.layout() 제거, editor-view의 함수로 이동
4. **Step이 유일한 변경 원천** — 수동 무효화 플래그 없음
5. **측정(Measurement)과 배치(Paginator)를 분리** — 측정은 캐시, 배치는 매번 수행

## 파이프라인

```
Doc (editor-model, 불변)
  |
  v
Measure (캐시 가능)
  - Doc 트리 재귀 순회, 각 노드의 크기와 내부 구조 계산
  - 위치 정보 없음 — 형제 크기에 의존하므로 캐시 불가
  |
  v
Measurement Tree (Arc 공유, 캐시)
  |
  v
Paginator (매번 수행)
  - Measurement → Fragment 변환의 유일한 경로
  - 위치 부여 + 페이지/타일 분할을 동시 수행
  - Container 스택으로 페이지 경계에서 계층 보존
  |
  v
Pages (Vec<Page>, 각 Page는 Fragment Tree)
  |
  +---> cursor 모듈: hit_test, resolve_movement, cursor_rect
  +---> renderer (이번 범위 밖)
```

## Fragment

레이아웃의 최종 출력. 위치(절대 좌표)가 포함된 순수 기하 데이터.

```rust
pub enum Fragment {
    Container(ContainerFragment),
    Line(LineFragment),
    Atom(AtomFragment),
}

pub struct ContainerFragment {
    pub node_id: NodeId,
    pub rect: Rect,
    pub children: Vec<Fragment>,
    pub scope: bool,
    pub breaks: Breaks,
}

pub struct Breaks {
    pub top: bool,
    pub bottom: bool,
}

pub struct LineFragment {
    pub node_id: NodeId,
    pub rect: Rect,
    pub baseline: f32,
    pub segments: Vec<LineSegment>,
}

pub struct AtomFragment {
    pub node_id: NodeId,
    pub parent_id: NodeId,
    pub index: usize,
    pub rect: Rect,
}

pub struct LineSegment {
    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
    pub x: f32,
    pub width: f32,
    pub char_advances: Vec<f32>,
}
```

세 가지 카테고리:
- **Container** — 자신은 투명, 자식에게 위임. scope=true면 커서 네비게이션의 스코프 경계 (TableCell).
- **Line** — 텍스트 줄. 커서가 문자 단위로 위치할 수 있음.
- **Atom** — 원자 블록 (image, embed, file, horizontal rule). 전체가 하나의 단위로 선택됨.

**Breaks**: Paginated 모드에서 Container가 페이지 경계에서 분할될 때 설정. 렌더러가 모서리/배경 처리에 사용. Continuous 모드에서는 항상 default.

**장식**: Fragment에 포함하지 않음. 렌더러가 Fragment + Doc을 함께 보고 그린다.

## Measurement

측정 결과를 캐시. **위치 정보 없음** — 크기와 내부 구조만 저장.

```rust
pub struct Measurement {
    pub size: Size,
    pub gap_after: f32,
    pub content: MeasuredContent,
}

pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

pub enum MeasuredContent {
    Container {
        children: Vec<ChildMeasurement>,
        scope: bool,
        direction: LayoutDirection,
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

pub struct ChildMeasurement {
    pub node_id: NodeId,
    pub measurement: Arc<Measurement>,
}

pub struct MeasuredLine {
    pub height: f32,
    pub baseline: f32,
    pub segments: Vec<LineSegment>,
}
```

- **gap_after**: BlockGap modifier에서 도출. 다음 형제와의 수직 간격.
- **LayoutDirection**: Vertical(자식을 수직 배치)은 Paginator가 자식 경계에서 분할 가능. Horizontal(TableRow)은 분할 불가.
- **PageBreak**: 명시적 페이지 분할 요청. Paginated 모드에서만 동작.

## Layout Engine

```rust
pub struct LayoutEngine {
    cache: FxHashMap<NodeId, Arc<Measurement>>,
    pages: Vec<Page>,
}

pub struct Page {
    pub fragments: Vec<Fragment>,
    pub height: f32,
}
```

### compute: measure → paginate

```rust
pub fn compute(&mut self, doc: &Doc, viewport: &Viewport, view_state: &ViewState) {
    let (content_width, paginator) = match doc.attrs().layout_mode {
        LayoutMode::Paginated { page_width, page_height, page_margin_top, page_margin_bottom, page_margin_left, page_margin_right } => {
            let cw = page_width - page_margin_left - page_margin_right;
            (cw, Paginator::new_paginated(cw, page_height, page_margin_top, page_margin_bottom, page_margin_left))
        }
        LayoutMode::Continuous { max_width } => {
            let cw = viewport.width.min(max_width) - CONTINUOUS_MARGIN * 2.0;
            (cw, Paginator::new_continuous(cw, CONTINUOUS_MAX_CONTENT_HEIGHT, CONTINUOUS_MARGIN, CONTINUOUS_MARGIN, CONTINUOUS_MARGIN))
        }
    };

    let root_m = self.measure(doc, NodeId::ROOT, content_width, view_state);

    if let MeasuredContent::Container { children, .. } = &root_m.content {
        for child in children {
            paginator.place(child.node_id, &child.measurement);
        }
    }

    self.pages = paginator.finish();
}
```

### Step 기반 캐시 무효화

Step에서 영향받는 노드와 그 조상을 캐시에서 제거. 다음 compute()에서 해당 노드가 새로 측정됨.

## Paginator

Measurement → Fragment 변환의 유일한 경로. Container 스택을 유지하여 페이지/타일 분할 시 양쪽에 완전한 Container 계층을 보장.

### 두 가지 모드

| | Paginated | Continuous |
|---|---|---|
| 용도 | 문서 페이지 (A4 등) | 렌더 타일 (캔버스 크기 제한) |
| 분할 기준 | content_height (page - margins) | max_content_height (설정 가능) |
| 마진 | 모든 페이지에 margin_top/bottom | 첫 타일만 margin_top, 마지막 타일만 margin_bottom |
| Breaks | 설정 (렌더러가 모서리 조정) | 항상 default |
| Wrapper 확장 | 페이지 하단까지 | 안 함 |
| Gap at break | 마진과 collapsing | 전체 보존 |
| 페이지 높이 (비-마지막) | 고정 page_height | 실제 콘텐츠 |
| 페이지 높이 (마지막) | 고정 page_height | 실제 콘텐츠 + margin_bottom |
| 빈 문서 | 1페이지 (page_height) | 1페이지 (margin_top + margin_bottom) |

### Container 스택과 페이지 분할

```
place(Fold) → open_container(Fold)
  place(FoldContent) → open_container(FoldContent)
    place(Para A) → 배치
    place(Para B) → 배치
    place(Para C) → 안 들어감 → break_page()
      FoldContent 닫기 (비어있으면 생략) → Fold에 추가
      Fold 닫기 → 페이지에 추가
      페이지 완성
      Fold 재오픈, FoldContent 재오픈
    place(Para C) → 새 페이지에 배치
  close_container(FoldContent)
close_container(Fold)
```

### 분할 단위

- **Container (Vertical)** — 자식 경계에서 분할. 빈 leaf Container는 atomic 배치.
- **Container (Horizontal)** — 분할 불가. position_subtree로 자식을 수평 배치.
- **TextBlock** — 줄 경계에서 분할.
- **Atom** — 분할 불가. 안 들어가면 다음 페이지로.
- **PageBreak** — 빈 페이지가 아니면 강제 분할 (Paginated만).

### Gap 처리

Measurement.gap_after가 다음 형제와의 간격. Paginator의 OpenContainer.pending_gap이 추적.

- **페이지 내**: gap + 자식 높이를 합산 검사 후 정상 적용.
- **Paginated break**: 마진이 gap을 흡수. gap > margin_spacing이면 초과분만 다음 페이지에 반영.
- **Continuous break**: gap 전체 보존 (타일 사이가 시각적으로 이어짐).
- **페이지 상단**: Container 재오픈 시 pending_gap=0 → gap 없음.
- **페이지 하단**: gap 적용 전에 break 발생 → gap 제거됨.

## Cursor Module

Fragment Tree만 읽는 독립 모듈. Doc에 의존하지 않음.

```rust
pub fn hit_test(page: &Page, x: f32, y: f32) -> Option<Selection>;
pub fn resolve_movement(pages: &[Page], pos: &Position, movement: &Movement) -> Option<Selection>;
pub fn cursor_rect(pages: &[Page], pos: &Position) -> Option<(usize, Rect)>;
```

- Line → collapsed Selection. Atom → range Selection (parent_id, index).
- 수직 이동은 기하 탐색 (y좌표 + preferred_x로 navigable Fragment 검색).
- 페이지 경계: 전체 pages를 받아 인접 페이지까지 탐색.
- hit_test는 단일 Page + 상대 좌표 (플랫폼이 page_idx 제공).

## View Facade

```rust
pub struct View {
    engine: LayoutEngine,
    viewport: Viewport,
    view_state: ViewState,
}

impl View {
    pub fn new(width: f32, scale_factor: f64) -> Self;

    pub fn reconcile(&mut self, state: &State, steps: &[Step]);
    pub fn layout(&mut self, doc: &Doc);

    pub fn hit_test(&self, page_idx: usize, x: f32, y: f32) -> Option<Selection>;
    pub fn resolve_movement(&self, pos: &Position, movement: &Movement) -> Option<Selection>;
    pub fn cursor_rect(&self, pos: &Position) -> Option<(usize, Rect)>;

    pub fn resize(&mut self, width: f32, scale_factor: f64);
    pub fn set_fold_state(&mut self, node_id: NodeId, expanded: bool);
    pub fn set_external_height(&mut self, node_id: NodeId, height: f32);

    pub fn page_count(&self) -> usize;
    pub fn page_height(&self, page_idx: usize) -> Option<f32>;
}
```

## 범위 밖

- **렌더링**: renderer 연동
- **입력 변환**: RawInput → Message 변환
- **Sync**: transform 구현 (step 8)
- **실제 텍스트 측정**: measure_inner는 stub. 레거시 레이아웃 코드 포팅 필요.
- **Movement::Word, Sentence, Block, Page, Document**: 추후 구현.
- **다단 레이아웃**: 추후.
