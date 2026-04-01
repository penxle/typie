# View

## 개요

브라우저의 DOM에 해당하는 중간 표현 계층.
레이아웃 트리, 렌더러를 소유하고, 기하 질의 API와 입력 변환을 담당한다.

## 구조

```rust
pub struct View {
    layout_engine: LayoutEngine,
    renderer: Renderer,
    fold_states: HashMap<NodeId, bool>,
    external_heights: HashMap<NodeId, f32>,
    viewport: Viewport,
}

pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub scroll_offset: f32,
    pub scale_factor: f64,
}
```

View가 소유하는 것은 시각적 관심사뿐:
- 레이아웃 엔진 + 렌더러 (기존 CPU/GPU dual 파이프라인)
- fold 상태, 외부 요소 높이 (문서 상태가 아닌 표시 방법)
- 뷰포트 정보

## Reconcile

Step에서 레이아웃 무효화를 도출하여 증분 재계산.

```rust
impl View {
    pub fn reconcile(&mut self, state: &State, steps: &[Step]) {
        let mut invalidations = Vec::new();

        for step in steps {
            match step {
                InsertText { node_id, .. } |
                RemoveText { node_id, .. } |
                AddMark { node_id, .. } |
                RemoveMark { node_id, .. } |
                SetNode { node_id, .. } => {
                    invalidations.push(Invalidation::NodeAndAncestors(*node_id));
                }

                InsertNode { parent_id, .. } |
                RemoveNode { parent_id, .. } => {
                    invalidations.push(Invalidation::SubtreeAndAncestors(*parent_id));
                }

                SplitNode { node_id, .. } |
                MergeNode { target_id: node_id, .. } => {
                    if let Some(parent) = state.doc.parent(*node_id) {
                        invalidations.push(Invalidation::SubtreeAndAncestors(parent));
                    }
                }

                MoveNode { old_parent, new_parent, .. } => {
                    invalidations.push(Invalidation::SubtreeAndAncestors(*old_parent));
                    invalidations.push(Invalidation::SubtreeAndAncestors(*new_parent));
                }

                SetSelection { .. } => {}
            }
        }

        self.layout_engine.invalidate(&invalidations);
        self.layout_engine.recompute(&state.doc);
    }
}
```

## 기하 질의 API

주로 입력 변환에서 사용. View 외부에 레이아웃 결과(Pages)를 노출할 필요 없음.

```rust
impl View {
    // 좌표 ↔ 문서 위치
    pub fn hit_test(&self, x: f32, y: f32) -> Option<Position>;
    pub fn cursor_rect(&self, pos: &Position) -> Option<Rect>;

    // 시각적 줄 경계
    pub fn line_start(&self, pos: &Position) -> Position;
    pub fn line_end(&self, pos: &Position) -> Position;

    // 시각적 커서 이동
    pub fn move_up(&self, pos: &Position, x: f32) -> Position;
    pub fn move_down(&self, pos: &Position, x: f32) -> Position;
    pub fn move_left(&self, pos: &Position) -> Position;
    pub fn move_right(&self, pos: &Position) -> Position;
    pub fn move_word_left(&self, doc: &Doc, pos: &Position) -> Position;
    pub fn move_word_right(&self, doc: &Doc, pos: &Position) -> Position;

    // 노드 경계
    pub fn node_bounds(&self, node_id: NodeId) -> Option<Rect>;

    // 뷰포트
    pub fn viewport_height(&self) -> f32;
}
```

## 입력 변환

Raw input을 문서 수준 Message로 변환. 기하 해석이 필요한 것만 View에서 처리.

```rust
impl View {
    pub fn translate(&self, state: &State, input: RawInput) -> Option<Message> {
        match input {
            // 기하 해석 필요
            RawInput::ArrowUp { extend } => {
                let pos = self.move_up(&state.selection.head, state.preferred_x?);
                Some(Message::Navigate { pos, extend })
            }
            RawInput::Click { x, y } => {
                let pos = self.hit_test(x, y)?;
                Some(Message::SetSelection(Selection::collapsed(pos)))
            }
            RawInput::CmdBackspace => {
                let start = self.line_start(&state.selection.head);
                Some(Message::DeleteRange(start, state.selection.head))
            }

            // 그대로 전달
            RawInput::Char(ch) => Some(Message::InsertText(ch.to_string())),
            RawInput::Enter => Some(Message::InsertNewline),
            RawInput::Backspace => Some(Message::DeleteBackward),
            RawInput::CmdZ => Some(Message::Undo),
            RawInput::CmdB => Some(Message::ToggleBold),
            // ...
        }
    }
}
```

## View 수준 상태

문서 상태가 아닌 시각적 상태는 View가 관리.

```rust
impl View {
    pub fn set_fold_state(&mut self, node_id: NodeId, expanded: bool);
    pub fn set_external_height(&mut self, node_id: NodeId, height: f32);
    pub fn resize(&mut self, width: f32, height: f32);
}
```

## 렌더링

```rust
impl View {
    pub fn render(&mut self, state: &State) {
        let selection_decor = build_selection_decorations(state, &self.layout_engine);
        self.renderer.render(&self.layout_engine.pages(), &selection_decor);
    }
}
```

기존 CPU/GPU dual 렌더 파이프라인을 그대로 사용.
