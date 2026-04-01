# Undo/Redo

## 개요

Step의 inverse를 활용한 operation 기반 undo/redo. History는 Runtime이 소유하며, State에 포함하지 않는다.

## 구조

```rust
pub struct History {
    undos: Vec<HistoryEntry>,
    redos: Vec<HistoryEntry>,
    last_push_time: Option<Instant>,
    merge_interval: Duration,          // e.g. 300~500ms
}

pub struct HistoryEntry {
    steps: Vec<Step>,            // SetSelection 포함
    tag: Option<HistoryTag>,     // 특수 동작 판별용
}

pub enum HistoryTag {
    AutoReplacement,                 // 자동 치환 (-- → —). 즉시 백스페이스 시 undo
    PasteHtml { plain_text: String }, // HTML 붙여넣기. 텍스트로 재붙여넣기 지원
}
```

## Undo / Redo 동작

### Undo

```rust
fn undo(&mut self) -> Option<Vec<Step>> {
    let entry = self.undos.pop()?;
    let inverse_steps: Vec<Step> = entry.steps.iter()
        .rev()
        .map(|s| s.inverse())
        .collect();
    self.redos.push(entry);
    Some(inverse_steps)
}
```

### Redo

```rust
fn redo(&mut self) -> Option<Vec<Step>> {
    let entry = self.redos.pop()?;
    let steps = entry.steps.clone();
    self.undos.push(entry);
    Some(steps)
}
```

Selection 복원은 별도 필드 없이 해결됨:
- Steps에 `SetSelection { old: A, new: B }` 포함
- inverse하면 `SetSelection { old: B, new: A }` → 자동으로 원래 커서 위치 복원

## 기록 규칙

```rust
fn push(&mut self, steps: &[Step]) {
    // 선택만 바뀐 경우 — 기록하지 않음
    let has_content_change = steps.iter().any(|s| !matches!(s, Step::SetSelection { .. }));
    if !has_content_change { return; }

    // redo 스택 초기화
    self.redos.clear();

    // 시간 기반 병합 판단
    let now = Instant::now();
    let should_merge = self.last_push_time
        .map(|t| now.duration_since(t) < self.merge_interval)
        .unwrap_or(false);

    if should_merge {
        let last = self.undos.last_mut().unwrap();
        last.steps.extend_from_slice(steps);
    } else {
        self.undos.push(HistoryEntry { steps: steps.to_vec() });
    }

    self.last_push_time = Some(now);
}
```

### 병합 예외

시간 내여도 작업 유형이 바뀌면 별도 entry로 분리해야 자연스러움 (e.g. 타이핑 직후 볼드 적용).
세부 휴리스틱은 구현 시 조정.

## Runtime 통합

```rust
impl Runtime {
    fn tick(&mut self, raw_inputs: Vec<RawInput>) {
        let messages = self.view.translate(&self.state, raw_inputs);

        for msg in messages {
            match msg {
                Message::Undo => {
                    if let Some(steps) = self.history.undo() {
                        self.state = apply(&self.state, &steps);
                        self.view.reconcile(&self.state, &steps);
                    }
                }
                Message::Redo => {
                    if let Some(steps) = self.history.redo() {
                        self.state = apply(&self.state, &steps);
                        self.view.reconcile(&self.state, &steps);
                    }
                }
                _ => {
                    let (steps, effects) = handle(&self.state, msg);
                    if !steps.is_empty() {
                        self.history.push(&steps);
                        self.state = apply(&self.state, &steps);
                        self.view.reconcile(&self.state, &steps);
                    }
                    self.process_effects(effects);
                }
            }
        }

        self.view.render();
    }
}
```

## 설계 근거

- **Operation 기반 (Slate 방식)**: Step이 inverse 데이터를 자체 보존하므로 state 스냅샷 불필요. 메모리 효율적이고 sync와 호환
- **시간 기반 병합**: 사용자가 기대하는 undo 단위는 "멈췄다 다시 친 것". 오프셋 인접 여부보다 시간이 더 신뢰할 수 있는 기준
- **History는 State 외부**: undo 자체는 undo 대상이 아님. Session 수준의 관심사로 Runtime이 소유
