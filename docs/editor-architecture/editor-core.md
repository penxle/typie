# Editor Core (Step 4)

## 개요

Message → Command 디스패치와 상태 관리를 담당하는 에디터의 중심 계층.
`editor-core` crate에 Editor, Message, History, handle()이 위치하고,
`editor-view` crate에 View가 분리된다.

## Crate 구조

```
editor-model
    ↑
editor-state
    ↑
editor-schema
    ↑
editor-transaction
    ↑              ↑
editor-commands    editor-view
    ↑              ↑
      editor-core
```

### editor-core

```
editor-core/src/
├── lib.rs
├── message.rs        # Message + 모든 하위 타입
├── editor.rs         # Editor struct, update(), apply()
├── handle.rs         # handle() — Message → Command 디스패치
└── history.rs        # History (Step inverse 기반 undo/redo)
```

### editor-view

```
editor-view/src/
├── lib.rs
├── view.rs           # View struct, reconcile(), render()
├── geometry.rs       # hit_test, cursor_rect, resolve_movement
└── viewport.rs       # Viewport 상태
```

## Message

플랫폼 추상화 계약. 모든 플랫폼(Web, Android, iOS)의 입력을 통일하는 단일 타입.
FFI 경계에서 에디터로 들어오는 모든 의도를 표현한다.

```rust
pub enum Message {
    /// 커서 이동 / 선택 확장
    Navigate { movement: Movement, extend: bool },

    /// Movement 방향으로 삭제
    Delete { movement: Movement },

    /// 커서 위치에 콘텐츠 삽입
    Insert(Content),

    /// 인라인/블록 서식 변경
    Format(FormatOp),

    /// 선택 영역 조작
    Select(SelectOp),

    /// 특정 노드 대상 조작
    Node { id: NodeId, op: NodeOp },

    /// 클립보드
    Clipboard(ClipboardOp),

    /// IME 조합
    Compose(ComposeOp),

    /// 포인터/제스처
    Pointer(PointerEvent),

    /// Undo
    Undo,

    /// Redo
    Redo,

    /// 시스템 이벤트
    System(SystemEvent),
}
```

### Movement (Navigate + Delete 공유)

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

pub enum Direction { Forward, Backward }
```

### Content

```rust
pub enum Content {
    Text(String),
    Break(BreakKind),
    Block(BlockContent),
}

pub enum BreakKind {
    Block,   // Enter — 블록 분할
    Line,    // Shift+Enter — 블록 내 줄바꿈
    Page,
}

pub enum BlockContent {
    HorizontalRule,
    Image { upload_id: Option<String> },
    File { upload_id: Option<String> },
    Embed,
    Table { rows: u32, cols: u32 },
    Fold,
}
```

### FormatOp

```rust
pub enum FormatOp {
    ToggleModifier(ModifierType),
    SetModifier(Modifier),
    Clear,
    SetTextAlign(TextAlign),
    SetLineHeight(u32),
    ToggleWrap(WrapperType),
    Indent,
    Outdent,
}

pub enum WrapperType { BulletList, OrderedList, Blockquote, Callout }
```

### SelectOp

```rust
pub enum SelectOp {
    All,
    Set(Selection),
    Collapse { to_anchor: bool },
}
```

### NodeOp

```rust
pub enum NodeOp {
    Delete,
    SetProperty(NodeProperty),
    ToggleFold,
    Table(TableOp),
}

pub enum TableOp {
    InsertAxis { axis: Axis, index: usize, before: bool },
    DeleteAxis { axis: Axis, index: usize },
    MoveAxis { axis: Axis, from: usize, to: usize },
    SelectAxis(Option<Axis>),
    SetProperty(TableProperty),
}

pub enum Axis { Row, Column }

pub enum TableProperty {
    ColumnWidths(Vec<f32>),
    BorderStyle(String),
    Align(TableAlign),
    Proportion(f32),
}
```

### ClipboardOp

```rust
pub enum ClipboardOp {
    Paste { html: Option<String>, text: String },
    Cut,
    Copy,
}
```

### ComposeOp

```rust
pub enum ComposeOp {
    Update { text: String, replace_length: Option<usize> },
    End,
}
```

### PointerEvent

```rust
pub enum PointerEvent {
    Down { x: f32, y: f32, count: u32, button: PointerButton, modifiers: KeyModifiers },
    Move { x: f32, y: f32, buttons: u16 },
    Up { x: f32, y: f32, button: PointerButton },
    Drag(DragEvent),
}

pub enum DragEvent {
    Start { x: f32, y: f32 },
    Over { x: f32, y: f32 },
    Enter,
    Leave,
    End,
    Drop { x: f32, y: f32, payload: DropPayload },
}
```

### SystemEvent

```rust
pub enum SystemEvent {
    Initialize { theme: Theme, width: f32, height: f32, scale_factor: f64 },
    Resize { width: f32, height: f32, scale_factor: f64 },
    SetTheme(Theme),
    SetFocused(bool),
    SetLayoutMode(LayoutMode),
    FontsLoaded { family: String, weight: u16, mappings: Vec<FontMapping> },
    SetExternalHeight { node_id: NodeId, height: f32 },
}
```

### 설계 원칙

- **플랫폼 추상화**: 모든 플랫폼(Web composition, Android InputConnection, iOS UITextInput)이 동일한 Message 타입으로 수렴
- **단일 enum**: 입력 순서 보장, FFI 진입점 단일화. 내부 처리 경로 차이는 타입이 아닌 match 분기로 표현
- **Movement 공유**: Navigate와 Delete가 동일한 Movement 타입 사용. 21개 variant → 2개로 압축
- **의도 수준**: 플랫폼이 아는 정보만 포함. 문서 위치 같은 에디터 내부 정보 없음

## State

```rust
#[derive(Clone)]
pub struct State {
    pub doc: Doc,
    pub selection: Selection,
    pub pending_modifiers: EnumMap<ModifierType, Option<Modifier>>,
    pub composition: Option<Composition>,
    pub preferred_x: Option<f32>,
}
```

State는 불변. 변이 = 새 State 생성. `Clone`은 imbl 구조적 공유 덕분에 O(1).

`font_registry`는 State에 포함하지 않는다 — 폰트 정보는 변이/undo/sync 대상이 아닌 외부 리소스.

## Editor

```rust
pub struct Editor {
    state: State,
    view: View,
    history: History,
    font_registry: FontRegistry,
}
```

### 처리 파이프라인

```rust
impl Editor {
    pub fn update(&mut self, msg: Message) {
        let result = match msg {
            // 순수 편집 (State만으로 처리)
            Message::Insert(..) | Message::Delete { .. } | Message::Format(..)
            | Message::Select(..) | Message::Node { .. } | Message::Clipboard(..)
            | Message::Compose(..) => {
                handle(&self.state, &self.font_registry, msg)
            }

            // 기하 의존 (View 질의 필요)
            Message::Navigate { movement, extend } => {
                self.handle_navigate(movement, extend)
            }
            Message::Pointer(event) => {
                self.handle_pointer(event)
            }

            // 히스토리
            Message::Undo => self.handle_undo(),
            Message::Redo => self.handle_redo(),

            // 시스템 (Steps 없음)
            Message::System(event) => {
                self.handle_system(event);
                return;
            }
        };

        self.apply(result);
    }

    fn apply(&mut self, (steps, effects): (Vec<Step>, Vec<Effect>)) {
        if !steps.is_empty() {
            self.history.push(&steps);
            self.state = State::apply(&self.state, &steps);
            self.view.reconcile(&self.state, &steps);
        }
        self.process_effects(effects);
    }
}
```

### 처리 경로

| Message 분류 | 처리 경로 | View 필요 |
|---|---|---|
| Insert, Delete, Format, Select, Node, Clipboard, Compose | `handle()` 순수 함수 | 아니오 |
| Navigate, Pointer | Editor 메서드 (View 기하 질의 후 Transaction) | 예 |
| Undo, Redo | History.undo/redo() → inverse Steps | 아니오 |
| System | View/Editor 직접 변이, Steps 없음 | 예 |

모든 편집 경로가 `apply()`로 수렴: Steps → State 갱신 → View reconcile → Effects 처리.

## handle() — 순수 편집 디스패치

State에만 의존하는 메시지를 처리하는 순수 함수. View 없이 테스트 가능.

```rust
pub fn handle(
    state: &State,
    fonts: &FontRegistry,
    msg: Message,
) -> (Vec<Step>, Vec<Effect>) {
    let mut tr = Transaction::new(state);

    match msg {
        Message::Insert(content) => match content {
            Content::Text(text) => {
                commands::insert_text(&mut tr, &text, fonts);
            }
            Content::Break(BreakKind::Block) => {
                commands::first(&mut tr, &[
                    &|tr| commands::lift_on_empty_paragraph(tr),
                    &|tr| commands::split_list_item(tr),
                    &|tr| commands::split_paragraph(tr),
                ]);
            }
            Content::Break(BreakKind::Line) => {
                commands::insert_hard_break(&mut tr);
            }
            Content::Block(block) => {
                commands::insert_block(&mut tr, block);
            }
            _ => {}
        },

        Message::Delete { movement } => {
            commands::delete(&mut tr, movement);
        }

        Message::Format(op) => match op {
            FormatOp::ToggleModifier(m) => {
                commands::toggle_modifier(&mut tr, m);
            }
            FormatOp::SetModifier(m) => {
                commands::set_modifier(&mut tr, m);
            }
            FormatOp::Clear => {
                commands::clear_formatting(&mut tr);
            }
            FormatOp::SetTextAlign(align) => {
                commands::set_text_align(&mut tr, align);
            }
            FormatOp::SetLineHeight(height) => {
                commands::set_line_height(&mut tr, height);
            }
            FormatOp::ToggleWrap(wrapper) => {
                commands::toggle_wrap(&mut tr, wrapper);
            }
            FormatOp::Indent => {
                commands::indent(&mut tr);
            }
            FormatOp::Outdent => {
                commands::outdent(&mut tr);
            }
        },

        // ... 나머지 Message 분기
        _ => {}
    }

    tr.finish()
}
```

## View

editor-view crate. 레이아웃/렌더링 소유, 기하 질의, reconcile.

```rust
pub struct View {
    layout_engine: LayoutEngine,
    renderer: Renderer,
    viewport: Viewport,
}

pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub scroll_offset: f32,
    pub scale_factor: f64,
}

impl View {
    // 기하 질의
    pub fn hit_test(&self, x: f32, y: f32) -> Option<Position>;
    pub fn resolve_movement(&self, state: &State, movement: &Movement) -> Position;
    pub fn cursor_rect(&self, pos: &Position) -> Option<Rect>;

    // State 변경 반영 (Step 기반 증분 무효화)
    pub fn reconcile(&mut self, state: &State, steps: &[Step]);

    // 렌더링
    pub fn render(&mut self, state: &State);

    // 뷰포트
    pub fn resize(&mut self, width: f32, height: f32, scale_factor: f64);
}
```

## History

Step inverse 기반 undo/redo. 상세는 [undo-redo.md](undo-redo.md) 참조.

```rust
pub struct History {
    undos: Vec<HistoryEntry>,
    redos: Vec<HistoryEntry>,
    last_push_time: Option<Instant>,
    merge_interval: Duration,
}

pub struct HistoryEntry {
    steps: Vec<Step>,
    tag: Option<HistoryTag>,
}

impl History {
    pub fn push(&mut self, steps: &[Step]);
    pub fn undo(&mut self) -> Option<Vec<Step>>;
    pub fn redo(&mut self) -> Option<Vec<Step>>;
}
```

## Effect

Transaction에서 축적되는 부수효과. Steps에서 도출 불가능한 플랫폼 요청만 포함.

```rust
pub enum Effect {
    FontNeeded { family: String, weight: u16, codepoints: Vec<u32> },
}
```

추후 필요 시 확장 (ScrollToCursor, SetPointerStyle, CopyToClipboard 등).

## 상태 질의

플랫폼이 toolbar 활성화 등 UI 상태를 결정하기 위한 질의 API.

```rust
impl Editor {
    /// Message가 현재 상태에서 적용 가능한지 (dry run)
    pub fn can_apply(&self, msg: &Message) -> bool;

    /// 현재 State 참조
    pub fn state(&self) -> &State;
}
```

`can_apply()`는 Transaction dry run 기반 — Transaction을 만들고 command를 시도한 뒤 버림.
