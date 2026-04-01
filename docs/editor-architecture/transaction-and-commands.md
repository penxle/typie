# Transaction & Commands

## 개요

- **Transaction**: Step을 다루는 핵심 도구. 코어 메서드(Step과 1:1 대응)만 제공
- **Command**: 에디터 의미를 아는 편집 로직. 독립 함수로 정의, Transaction을 조작

## Transaction

```rust
pub struct Transaction {
    state: State,           // 작업 복사본, Step마다 갱신
    steps: Vec<Step>,
    effects: Vec<Effect>,
}

impl Transaction {
    pub fn new(state: &State) -> Self;

    // 중간 상태 읽기
    pub fn doc(&self) -> &Doc;
    pub fn selection(&self) -> &Selection;

    // 코어 메서드 — Step과 1:1 대응
    pub fn insert_text(&mut self, node_id: NodeId, offset: usize, text: &str);
    pub fn remove_text(&mut self, node_id: NodeId, offset: usize, len: usize);
    pub fn insert_node(&mut self, parent_id: NodeId, index: usize, entry: NodeEntry);
    pub fn remove_node(&mut self, node_id: NodeId);
    pub fn move_node(&mut self, node_id: NodeId, new_parent: NodeId, new_index: usize);
    pub fn split_node(&mut self, node_id: NodeId, offset: usize) -> NodeId;
    pub fn merge_node(&mut self, node_id: NodeId, target_id: NodeId);
    pub fn set_node(&mut self, node_id: NodeId, new_properties: NodeProperties);
    pub fn add_mark(&mut self, node_id: NodeId, from: usize, to: usize, mark: Mark);
    pub fn remove_mark(&mut self, node_id: NodeId, from: usize, to: usize, mark: Mark);
    pub fn set_selection(&mut self, selection: Selection);

    // Savepoint
    pub fn savepoint(&self) -> Savepoint;
    pub fn rollback(&mut self, sp: Savepoint);

    // 완료
    pub fn finish(self) -> (Vec<Step>, Vec<Effect>);
}
```

### 코어 메서드 동작 방식

각 코어 메서드는 Step 생성 → apply → 축적의 패턴을 따른다:

```rust
pub fn insert_text(&mut self, node_id: NodeId, offset: usize, text: &str) {
    let step = Step::InsertText { node_id, offset, text: text.to_string(), ... };
    self.state = step.apply(&self.state);
    self.steps.push(step);
}
```

### Savepoint

State clone이 O(1)이라 거의 무료. Command chaining과 atomic 실행에 사용.

```rust
pub struct Savepoint {
    state: State,          // O(1) clone
    steps_len: usize,
    effects_len: usize,
}
```

## Commands

에디터 편집 로직을 독립 함수로 정의. `mod commands`에서 flat하게 노출.

### 시그니처

```rust
pub fn command_name(tr: &mut Transaction, /* 추가 파라미터 */) -> Result<bool>
```

- 첫 인자: 항상 `&mut Transaction`
- 반환: `Result<bool>` — `Ok(true)` 적용됨, `Ok(false)` 적용 불가, `Err` 에러
- 추가 파라미터: 자유

### 예시

```rust
mod commands {
    pub fn split_paragraph(tr: &mut Transaction) -> Result<bool> { ... }
    pub fn join_backward(tr: &mut Transaction) -> Result<bool> { ... }
    pub fn toggle_bullet_list(tr: &mut Transaction) -> Result<bool> { ... }
    pub fn split_list_item(tr: &mut Transaction) -> Result<bool> { ... }
    pub fn insert_table(tr: &mut Transaction, rows: u32, cols: u32) -> Result<bool> { ... }
    pub fn toggle_mark(tr: &mut Transaction, mark: Mark) -> Result<bool> { ... }
    pub fn insert_fold(tr: &mut Transaction) -> Result<bool> { ... }
    pub fn paste_fragment(tr: &mut Transaction, fragment: Fragment) -> Result<bool> { ... }
    // ...
}
```

### Command 합성

#### `commands::first` — 첫 번째 성공하는 command만 실행

여러 command를 순서대로 시도하고, 가장 먼저 성공하는 것만 적용. 실패 시 savepoint로 롤백.

```rust
pub fn first(tr: &mut Transaction, commands: &[&dyn Fn(&mut Transaction) -> Result<bool>]) -> Result<bool> {
    for cmd in commands {
        let sp = tr.savepoint();
        match cmd(tr)? {
            true => return Ok(true),
            false => tr.rollback(sp),
        }
    }
    Ok(false)
}
```

사용 예시 — Enter 키 처리:

```rust
fn handle_enter(state: &State) -> (Vec<Step>, Vec<Effect>) {
    let mut tr = Transaction::new(state);
    commands::first(&mut tr, &[
        &|tr| commands::lift_on_empty_paragraph(tr),
        &|tr| commands::split_list_item(tr),
        &|tr| commands::split_paragraph(tr),
    ]);
    tr.finish()
}
```

#### `commands::chain` — 모든 command가 성공해야 atomic 적용

모든 command가 성공해야 전체가 적용됨. 하나라도 실패하면 전체 롤백.

```rust
pub fn chain(tr: &mut Transaction, commands: &[&dyn Fn(&mut Transaction) -> Result<bool>]) -> Result<bool> {
    let sp = tr.savepoint();
    for cmd in commands {
        if !cmd(tr)? {
            tr.rollback(sp);
            return Ok(false);
        }
    }
    Ok(true)
}
```

사용 예시 — 선택 삭제 후 붙여넣기:

```rust
fn handle_paste(state: &State, fragment: Fragment) -> (Vec<Step>, Vec<Effect>) {
    let mut tr = Transaction::new(state);
    commands::chain(&mut tr, &[
        &|tr| commands::delete_selection(tr),
        &|tr| commands::paste_fragment(tr, fragment.clone()),
    ]);
    tr.finish()
}
```

### Dry run

Transaction이 외부 상태를 변이하지 않으므로, 만들고 버리면 무료 dry run:

```rust
fn can_apply(state: &State, cmd: &dyn Fn(&mut Transaction) -> Result<bool>) -> bool {
    let mut tr = Transaction::new(state);  // O(1)
    cmd(&mut tr).unwrap_or(false)
    // tr drop — 아무 side effect 없음
}
```

## 설계 근거

### Transaction/Command 분리 (ProseMirror 방식)

- Transaction은 코어 메서드 10개 + 유틸리티. 역할이 작고 명확
- Command는 독립 함수. 조합, 테스트, 확장이 자유로움
- 새 기능 = 새 command 함수 추가. Transaction 수정 불필요

### Savepoint 기반 합성

- 불변 모델에서 State clone이 O(1)이므로 savepoint가 거의 무료
- 기존 mutable 모델에서는 불가능했던 패턴:
  - 시도 → 실패 → 롤백 → 다음 시도 (first)
  - 전체 성공 아니면 전체 롤백 (chain)
  - 적용 가능 여부만 확인 (dry run)
