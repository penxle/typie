# Editor Architecture Redesign

## 배경

기존 Runtime이 5800줄짜리 god object로, 상태 관리/레이아웃/렌더링/입력 처리가 모두 혼재.
근본 원인은 Loro CRDT의 공유 가변성(`Rc<Doc>`)이 불변 스냅샷을 불가능하게 만든 것.
Loro는 요구사항 대비 과도하며, 성능 오버헤드가 발목을 잡고 있음.

## 핵심 결정사항

### 1. Loro 제거

Loro를 제거하고 자체 문서 모델로 대체한다.

실제 요구사항:
- 단일 사용자의 멀티 기기 동기화
- 기초적 수준의 멀티 유저 동시편집 (nice to have)
- 오프라인 편집 후 온라인 복귀 시 best-effort 동기화

Loro에서 실제로 사용하지 않는 것:
- Peritext 기반 mark (자체 스타일 시스템 사용 중)
- 높은 수준의 동시편집 보장

### 2. 불변 문서 모델 (ProseMirror 방식)

```
State { doc, selection, ... }  <- 모든 것이 불변

Step = 변경의 최소 단위
Transaction = Vec<Step> + 중간 상태 접근

apply(state, steps) -> new_state   <- 순수 함수
invert(step) -> inverse_step       <- undo
transform(step, step) -> step      <- sync 충돌 해결
```

- Doc은 persistent tree + rope (구조적 공유)
- Step이 변경, undo, sync를 모두 담당하는 단일 통화
- Transaction은 로컬 계산 도구: 내부에 중간 상태 복사본 유지, Step 축적 후 반환

### 3. 브라우저 아날로지 — State / View / Runtime 3계층

에디터는 사실상 작은 브라우저. DOM에 해당하는 중간 표현 계층(View)이 필요.

```
브라우저                    우리 에디터
-----------                -----------
JavaScript (앱 상태)        State + Handler
    | ^                        | ^
    | DOM API (변경)           | Steps (변경)
    | Events (이벤트)          | Messages (이벤트)
    v |                        v |
DOM (중간 표현)             View (중간 표현)
    |                          |
    | Layout / Reflow          | reconcile + layout
    v                          v
Paint / Composite           Render (CPU/GPU)
```

#### State + Handler (= JavaScript)
- State는 불변. 변이 = 새 State 생성
- Handler는 순수 함수: `(State, Message) -> (Vec<Step>, Vec<Effect>)`
- Handler는 레이아웃에 의존하지 않음. State만 봄

#### View (= DOM)
- 레이아웃 트리, 레이아웃 엔진, 렌더러를 소유
- State로부터 reconcile을 통해 동기화 (Step 기반 증분 업데이트)
- 기하 질의 API 제공 (hit test, cursor rect, line bounds 등)
- Raw input을 문서 수준 Message로 변환
- 외부에 레이아웃 결과(Pages)를 노출할 필요 없음

#### Runtime (= 이벤트 루프)

```rust
fn tick(&mut self, raw_inputs: Vec<RawInput>) {
    let messages = self.view.translate(&self.state, raw_inputs);

    for msg in messages {
        let (steps, effects) = handle(&self.state, msg);
        if !steps.is_empty() {
            self.state = apply(&self.state, &steps);
            self.view.reconcile(&self.state, &steps);
        }
        self.process_effects(effects);
    }

    self.view.render();
}
```

### 4. Transaction / Command / Handler

#### Transaction
Step을 다루는 핵심 도구. 코어 메서드(Step과 1:1 대응)만 제공.
Steps와 Effects를 축적하고 `finish()`로 반환.

#### Command
에디터 의미를 아는 편집 로직. 독립 함수로 정의.
시그니처: `fn(tr: &mut Transaction, ...) -> Result<bool>`

```rust
mod commands {
    pub fn split_paragraph(tr: &mut Transaction) -> Result<bool> { ... }
    pub fn toggle_mark(tr: &mut Transaction, mark: Mark) -> Result<bool> { ... }
}
```

#### Handler
Message → Command 디스패치. 얇은 매핑 레이어.

```rust
fn handle(state: &State, msg: Message) -> (Vec<Step>, Vec<Effect>) {
    let mut tr = Transaction::new(state);
    match msg {
        Message::InsertNewline => {
            commands::first(&mut tr, &[
                &|tr| commands::lift_on_empty_paragraph(tr),
                &|tr| commands::split_list_item(tr),
                &|tr| commands::split_paragraph(tr),
            ]);
        }
        Message::ToggleBold => {
            commands::toggle_mark(&mut tr, Mark::Style(Style::Bold));
        }
        // ...
    }
    tr.finish()
}
```

관련 메시지를 그룹 함수로 위임 (handle_input, handle_deletion, handle_formatting, ...).

### 5. Effect

Step/State 시스템 바깥의 부수효과. 시스템 경계를 넘는 요청들.

```rust
pub enum Effect {
    FontNeeded { family: String, weight: u16, codepoints: Vec<u32> },
    ScrollToCursor,
    SetPointerStyle(PointerStyle),
    ExternalElementChanged { node_id: NodeId },
    CopyToClipboard { html: String, text: String },
}
```

기존 DocChanged, SelectionChanged, NodeMutated 등은 Steps에서 도출 가능하므로 Effect에서 제거됨.
Command가 `tr.push_effect()`로 Transaction에 축적하고, `tr.finish()`에서 Steps와 함께 반환.

### 6. Runtime 필드 재배치

기존 Runtime의 18개 mutable 필드가 새 구조에서 어디로 가는지:

| 기존 필드 | 새 위치 | 이유 |
|---|---|---|
| `layout_engine` | View | 레이아웃은 View의 핵심 책임 |
| `renderer` | View | 렌더링은 View의 핵심 책임 |
| `state` | Runtime (불변) | 불변 State로 변경 |
| `undo_manager` | History | Loro UndoManager → Step 기반 History |
| `loaded_font_codepoints` | View | 폰트 로딩 상태는 렌더링 관심사 |
| `missing_font_nodes` | View | 상동 |
| `selection_cache` | 제거 | State + View에서 도출 가능 |
| `pending` (19개 플래그) | 제거 | Steps에서 무효화 도출. View.reconcile()이 대체 |
| `message_queue` | Runtime | 외부 메시지 버퍼 |
| `pointer` | View | 포인터/드래그 상태는 입력 처리 관심사 |
| `slate` / `slab` | Runtime | FFI 출력 버퍼 |
| `history` | History | 별도 구조체 |
| `cached_plain_text` | 제거 | State에서 필요 시 도출 |
| `tracked_items` | View | 시각적 오버레이 |
| `is_focused` | View | 렌더링에 영향 |
| `last_table_overlays` | View | 오버레이 중복 방지 |
| `text_replacement_undo` | 제거 | History 태깅으로 대체 |
| `repaste_text` | 제거 | History 태깅으로 대체 |
| `tracing` | Runtime | 횡단 관심사 |

새 Runtime:

```rust
pub struct Runtime {
    state: State,
    view: View,
    history: History,
    sync: SyncClient,
    message_queue: Vec<Message>,
    slate: Slate,          // FFI 출력
    slab: Slab,
    tracing: TracingReporter,
}
```

## 구현 순서

1. **Doc** — imbl 기반 불변 문서 모델. 나머지 전부의 기반
2. **Step + apply/inverse** — Doc 위에 바로 구축
3. **Transaction + Command** — Step 위에 구축, 기존 Transaction 로직 이식
4. **State + Handler** — 불변 State 정의, Message → Command 디스패치
5. **History** — Step inverse 기반 undo/redo
6. **View** — 기존 LayoutEngine/Renderer를 View로 감싸기, reconcile
7. **Runtime** — 새 이벤트 루프로 조립
8. **Sync** — transform 구현, 서버 연동

1~3은 기존 코드와 독립적으로 구축 가능.
4~7에서 기존 코드를 교체.
8은 가장 나중.

## 상세 설계 문서

- [document-model.md](document-model.md) — imbl 기반 불변 문서 모델
- [step-operations.md](step-operations.md) — 11가지 원자 Step 정의, inverse, 분해 예시
- [transaction-and-commands.md](transaction-and-commands.md) — Transaction/Command 분리, savepoint, chaining
- [undo-redo.md](undo-redo.md) — Step inverse 기반 히스토리, 시간 기반 병합
- [sync.md](sync.md) — 중앙 서버 + 버전 기반 OT, 오프라인 지원
- [view.md](view.md) — 레이아웃/렌더 소유, reconcile, 기하 질의, 입력 변환

## 참고한 에디터 조사

| | Quill | Draft.js | Lexical | Slate |
|---|---|---|---|---|
| 불변성 | Mutable | Immutable (Immutable.js) | Immutable (COW + freeze) | Mutable (조상 shallow copy) |
| 변이 | Delta -> DOM 직접 변이 | DraftModifier 체이닝 -> push | editor.update() 클로저 | Commands -> Transforms -> apply(op) |
| Undo | 역 Delta 스택 | ContentState 스냅샷 | EditorState 스냅샷 | Operation.inverse() 역순 적용 |
| Sync | Delta OT 프리미티브 | 미지원 | Yjs 바인딩 | Yjs 바인딩 |

핵심 교훈: Step(Operation)이 변이/undo/sync를 동시에 해결하는 단일 통화 역할을 해야 함 (Slate 방식).
