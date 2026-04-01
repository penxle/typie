# View — 커서 Movement 확장 설계

## 목적

현재 Grapheme(←/→)/Line(↑/↓)만 구현된 커서 이동을 Word, Sentence, Block, Document, LineStart/End, PageUp/Down으로 확장한다.

## 범위

- Movement enum 재설계 (의미적/시각적 이동 분리)
- 신규 movement 구현: Word, Sentence, Block, Document, LineStart, LineEnd, PageUp, PageDown
- `icu_segmenter` 사용 (Word/Sentence 경계 탐색)
- Viewport에 height 추가

## Movement enum 재설계

기존:
```rust
pub enum Movement {
    Grapheme(Direction),
    Word(Direction),
    Line(Direction),
    Sentence(Direction),
    Block(Direction),
    Page(Direction),
    Document(Direction),
}
```

신규:
```rust
pub enum Direction {
    Forward,
    Backward,
}

pub enum Movement {
    // 의미적 이동 — 문서 모델만으로 결정 가능
    Grapheme(Direction),
    Word(Direction),
    Sentence(Direction),
    Block(Direction),
    Document(Direction),

    // 시각적 이동 — 레이아웃(줄바꿈, 페이지네이션)에 의존
    LineStart,
    LineEnd,
    LineUp,
    LineDown,
    PageUp,
    PageDown,
}
```

설계 근거:
- 의미적 이동은 Forward/Backward 방향이 의미 있음 (텍스트 흐름에서 다음/이전)
- 시각적 이동은 방향이 variant 이름에 내포됨 — `LineUp`은 항상 위, `LineEnd`는 항상 끝
- Line이라는 이름의 모호성을 제거: `LineUp`/`LineDown`(시각적 줄 이동)과 `LineStart`/`LineEnd`(줄 시작/끝 점프)가 명확히 구분됨

기존 매핑:
- `Movement::Grapheme(dir)` → `Movement::Grapheme(dir)` (동일)
- `Movement::Line(Forward)` → `Movement::LineDown`
- `Movement::Line(Backward)` → `Movement::LineUp`

## TextSegmenters 및 ICU 데이터 주입

```rust
// editor-view에 정의
pub struct TextSegmenters {
    pub word: WordSegmenter,
    pub sentence: SentenceSegmenter,
}
```

- `TextSegmenters`는 editor-core의 `Editor`가 소유
- 앱 시작 시 ICU 데이터 provider로부터 생성하여 Editor에 주입
- View는 segmenter를 모름 — `resolve_movement` 호출 시 Editor가 전달

```rust
// editor-view: resolve_movement 시그니처
pub fn resolve_movement(
    pages: &[Page],
    pos: &Position,
    movement: &Movement,
    viewport: &Viewport,
    segmenters: Option<&TextSegmenters>,
) -> Option<Selection>

// View facade
impl View {
    pub fn resolve_movement(
        &self,
        pos: &Position,
        movement: &Movement,
        segmenters: Option<&TextSegmenters>,
    ) -> Option<Selection>
}

// editor-core: Editor가 소유하고 전달
impl Editor {
    fn handle_navigate(&mut self, movement: Movement, extend: bool) {
        let sel = self.view.resolve_movement(&pos, &movement, self.segmenters.as_ref());
        // ...
    }
}
```

ICU 데이터 주입 흐름:
1. 앱이 ICU 데이터를 로드 → `BlobDataProvider`
2. `WordSegmenter::try_new_dictionary_unstable(&provider, ...)` + `SentenceSegmenter::try_new_unstable(&provider, ...)`
3. `TextSegmenters { word, sentence }` 생성
4. `Editor`에 주입

서버 사이드 렌더링 등 커서 이동이 불필요한 환경에서는 segmenters를 None으로 유지. Word/Sentence movement는 segmenters가 None이면 None을 반환.

## Viewport 확장

```rust
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub scale_factor: f64,
}
```

`height` 필드 추가. PageUp/PageDown에서 뷰포트 높이만큼 커서를 이동할 때 사용.

## Movement별 설계

### Grapheme (기존)

기존 `move_left`/`move_right` 구현 그대로.

### LineUp / LineDown (기존 Line)

기존 `move_up`/`move_down` 구현 그대로. preferred_x를 보존하며 위/아래 navigable을 탐색.

### LineStart / LineEnd

- LineEnd: `find_line_at()`으로 현재 줄 → `last_position_in(line)`
- LineStart: `find_line_at()`으로 현재 줄 → `first_position_in(line)`

### Block

- `find_line_at()`으로 현재 줄 → 해당 줄이 속한 scope ContainerFragment를 찾음
- Forward: container의 `find_last_navigable()` → `last_position_in()`
- Backward: container의 `find_first_navigable()` → `first_position_in()`

### PageUp / PageDown

뷰포트 높이만큼 커서를 위/아래로 이동 (일반적인 에디터 PageUp/Down 동작):
1. `find_line_at()`으로 현재 줄의 y 좌표 확인
2. `y ± viewport.height` 위치에서 `find_navigable_below/above()` 탐색
3. preferred_x 보존 (LineUp/Down과 동일한 패턴)

### Document

- Forward: 마지막 page의 `find_last_navigable()` → `last_position_in()`
- Backward: 첫 page의 `find_first_navigable()` → `first_position_in()`

### Word

1. `find_line_at()`으로 현재 LineFragment를 찾음
2. segments에서 줄 전체 텍스트를 조합
3. Position을 줄 내 char index로 변환
4. `icu_segmenter::WordSegmenter`로 해당 index 이후/이전의 다음 단어 경계를 찾음 (byte offset 반환, char offset으로 변환 필요)
5. char index를 Position(node_id, offset)으로 역변환
6. 줄 끝/시작에 도달하면 다음/이전 줄의 첫/마지막 위치로 이동

### Sentence

Word와 동일한 패턴. `icu_segmenter::SentenceSegmenter` 사용.

## 줄 내 텍스트 ↔ Position 변환

Word/Sentence에서 공유하는 변환 로직:

```rust
/// Position(node_id, offset) → 줄 내 char index
fn line_char_index(line: &LineFragment, pos: &Position) -> Option<usize>

/// 줄 내 char index → Position(node_id, offset)
fn position_at_char_index(line: &LineFragment, char_index: usize) -> Position
```

LineFragment의 segments를 순회하며 char 수를 누적하여 변환한다. 여러 node_id에 걸친 segments를 정확히 처리한다.

## 파일 구조

```
crates/editor-common/src/movement.rs    — Movement, Direction 재설계
crates/editor-view/src/
  viewport.rs                           — Viewport에 height 추가
  cursor/
    navigation.rs                       — resolve_movement dispatch 수정 + 시그니처 변경
    boundary.rs                         — 신규: TextSegmenters 정의, Word/Sentence 경계 탐색, 줄 내 텍스트 ↔ Position 변환
    search.rs                           — find_scope_container_at 추가 (Block용)
    hit_test.rs                         — 변경 없음
    mod.rs                              — boundary 모듈 선언, resolve_movement re-export 시그니처 변경
  view.rs                               — resolve_movement에 segmenters 파라미터 전달
  Cargo.toml                            — icu_segmenter 의존성 추가
crates/editor-core/src/
  editor.rs                             — TextSegmenters 소유, handle_navigate에서 전달
  message.rs                            — Movement 사용처 수정
  handle.rs                             — Movement 사용처 수정
```

## 의존 관계

```
editor-view
  └── icu_segmenter (Word/Sentence 경계 탐색, TextSegmenters 타입)
editor-core
  └── editor-view (TextSegmenters 타입 참조)
  └── icu_segmenter + icu_provider + icu_provider_blob (Segmenter 생성)
```
