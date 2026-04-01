# Remaining Work

에디터 2.5 아키텍처 재설계의 남은 작업 목록.

## 구현 순서 현황

| Step | 내용 | 상태 |
|------|------|------|
| 1. Doc | imbl 기반 불변 문서 모델 | 완료 |
| 2. Step + apply/inverse | Doc 위에 구축 | 완료 |
| 3. Transaction + Command | Step 위에 구축, 기존 로직 이식 | 진행 중 (C1 잔여 + C2) |
| 4. State + Handler | 불변 State, Message → Command 디스패치 | 완료 |
| 5. History | Step inverse 기반 undo/redo | 완료 |
| 6. View | 레이아웃 파이프라인 | 골격 완료, 실제 측정 미구현 |
| 7. Runtime | 새 이벤트 루프 조립 | 미착수 |
| 8. Sync | transform 구현, 서버 연동 | 미착수 |

## Step 3: Command 이식 잔여

상세: [command-implementation-status.md](command-implementation-status.md)

### C1 잔여: 86 ignored tests 활성화

| 카테고리 | 수 | 필요한 작업 |
|----------|---|------------|
| Font normalization | 48 | FontRegistry 기반 font-weight-aware toggle/set 로직 |
| Cascade attrs | 8 | Block-level modifier 상속 (root → paragraph) |
| Trailing paragraph | 6 | split/join/delete 후 빈 문단 정규화 |
| Effects | 5 | LoadFont effect emit 연동 |
| Table selection | 5 | Table rectangular selection (C2에서 처리) |
| Slot positions | 3 | Position affinity 기반 slot 처리 |
| Layout | 3 | Layout cache invalidation (View 연동) |
| Insert text | 3 | text node 분할 후 target 해석 |
| Pending modifiers | 2 | 나머지 command pending_modifiers 연동 |
| 기타 버그 수정 | 3 | merge modifier 보존, join 빈 문단, cross-paragraph range |

### C2: 나머지 command 전량 이식

블록인용/콜아웃, 폴드, 문서 레벨 조작, 클립보드, 드래그앤드롭, 테이블, IME composition, 어노테이션/리마크, 기타. 전체 목록은 command-implementation-status.md 참조.

## Step 6: View 레이아웃 잔여

상세: [view-layout-design.md](view-layout-design.md)

### 실제 텍스트 측정 (measure_inner 포팅)

현재 `measure_inner()`는 모든 노드를 빈 Container로 측정하는 stub. 레거시 레이아웃 코드를 포팅하여 실제 텍스트 측정, 줄바꿈, LineSegment 생성을 구현해야 함.

필요한 것:
- 텍스트 셰이핑 (parley/swash 연동)
- 줄바꿈 알고리즘
- 노드 타입별 측정 로직 (Paragraph → TextBlock, Image → Atom, Table → Horizontal Row 등)
- BlockGap modifier에서 gap_after 도출

### 커서 이동 확장

현재 `resolve_movement`는 Grapheme(Forward/Backward)과 Line(Forward/Backward)만 구현.

| Movement | 상태 |
|----------|------|
| Grapheme | 구현됨 |
| Line | 구현됨 |
| Word | 미구현 |
| Sentence | 미구현 |
| Block | 미구현 |
| Page | 미구현 |
| Document | 미구현 |

### 다단 레이아웃

현재 단일 컬럼만 지원. Paginator가 컬럼을 인식하여 배치하는 로직 필요.

## Step 6.5: 렌더링

View 설계에서 범위 밖으로 둔 렌더러 연동.

필요한 것:
- Fragment Tree를 읽어서 렌더링하는 renderer
- Breaks를 보고 모서리/배경 처리 (Paginated 모드)
- 장식 요소 (blockquote 테두리, callout 배경, list marker 등) — Fragment + Doc에서 도출
- Wrapper 확장 영역의 시각적 처리
- CPU/GPU dual 파이프라인 연동

## Step 7: Runtime

새 이벤트 루프로 조립.

```rust
pub struct Editor {
    state: State,
    view: View,
    history: History,
    font_registry: FontRegistry,
}
```

현재 editor-core에 Editor 골격이 있음. 남은 것:
- handle_navigate: View.resolve_movement 연동
- handle_pointer: View.hit_test 연동
- handle() 함수에서 나머지 Message 분기 구현 (현재 Insert, Delete, Format 등은 stub)
- Effect 처리 (FontNeeded → 플랫폼에 폰트 요청)
- can_apply() dry run 구현

## Step 8: Sync

transform 구현, 서버 연동. 가장 나중.
