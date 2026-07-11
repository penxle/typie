# DocumentEditorLoad Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** raw `GraphIngest`를 restartable Compose effect에서 제거하고 snapshot마다 한 번만 Editor를 생성하는 `DocumentEditorLoad` 경계를 도입한다.

**Architecture:** `DocumentGraphLoader`는 streaming ingest와 handle transfer만 담당하고, 새 `DocumentEditorLoad`가 handle·baseline·pending 및 memoized Editor 생성 수명을 소유한다. `EditorScreen`은 load 교체와 catch-up sync를, `EditorView`는 최신 viewport/theme reconciliation과 runtime attach를 담당한다.

**Tech Stack:** Kotlin Multiplatform, Compose Multiplatform, coroutines, Rust editor FFI, Gradle desktop tests

---

### Task 1: one-shot load 경계

**Files:**
- Create: `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/sync/DocumentEditorLoad.kt`
- Modify: `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/sync/ws/DocumentGraphLoader.kt`
- Modify: `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/Editor.kt`
- Create: `apps/mobile/compose/src/commonTest/kotlin/co/typie/editor/sync/DocumentEditorLoadTest.kt`
- Modify: `apps/mobile/compose/src/commonTest/kotlin/co/typie/editor/sync/ws/DocumentGraphLoaderTest.kt`

- [ ] production `DocumentEditorLoad` API를 반복 await하는 회귀 테스트를 먼저 추가한다. fake `GraphIngest.finishWithPending()`은 count 후 sentinel failure를 던지고, 두 await가 같은 memoized failure를 받으며 count가 1인지 검증한다. 테스트 전용 production seam은 추가하지 않는다.
- [ ] build queue로 해당 desktop test를 실행해 기존 코드에 load가 없어 실패하는 것을 확인한다.
- [ ] `DocumentGraphLoader`의 숫자 generation을 제거하고 기존 restart/transfer/abort 계약을 유지한다.
- [ ] 기존 호출부가 전환될 때까지 `EditorGraphSource` 호환 경계는 유지하고, load만 사용할 좁은 internal ingest 생성 함수를 먼저 둔다.
- [ ] `DocumentEditorLoad`에 immutable input, read-only initial baseline, document/session scope를 parent로 하고 개별 awaiter와 분리된 memoized 생성, finish/abort 직렬화, idempotent close와 late Editor disposal을 구현한다.
- [ ] targeted desktop test를 다시 실행해 통과시킨다.

### Task 2: Compose와 sync 연결

**Files:**
- Modify: `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/EditorView.kt`
- Modify: `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/body/EditorBody.kt`
- Modify: `apps/mobile/compose/src/commonMain/kotlin/co/typie/screen/editor/editor/EditorScreen.kt`

- [ ] `EditorBody`와 `EditorView` 입력을 `DocumentEditorLoad`로 교체한다.
- [ ] 모든 호출부 전환 뒤 `EditorGraphSource`와 `Editor.createFromSource()`를 제거한다.
- [ ] `EditorView`가 같은 load를 재await하고, viewport/theme 동기화의 suspend 경계 뒤에 load·환경을 재검증하며, 최종 check와 attach를 Main의 같은 non-suspending 구간에서 수행하게 한다.
- [ ] `SnapshotEnd` 후 pending read 동안 화면 block이 transferred handle을 임시 소유하고, 실패·취소·session 교체 시 `try/finally`에서 한 번 abort한 뒤 성공 시에만 load로 transfer한다.
- [ ] snapshot 교체/화면 종료를 `runtime.clear()` 후 old load `close()` 순서로 정리하고 stale 결과·오류를 current runtime에 연결하지 않는다.
- [ ] catch-up queue를 load identity에 묶고, drain 후 identity를 재검증하며 마지막 event의 `seq`, `heads`, `durableHeads` 전체로 effective baseline을 전진시킨다. Main의 같은 non-suspending 구간에서 live subscription을 먼저 활성화한 뒤 queueing을 종료해 누락·중복 틈을 막는다.
- [ ] `:compose:compileKotlinDesktop`으로 wiring과 visibility를 확인한다.

### Task 3: 좁은 검증

**Files:**
- Verify only; production 분할은 추가하지 않음

- [ ] build queue로 `apps/mobile/gradlew -p apps/mobile :compose:desktopTest --tests 'co.typie.editor.sync.DocumentEditorLoadTest' --tests 'co.typie.editor.sync.ws.DocumentGraphLoaderTest'`를 실행한다.
- [ ] build queue로 `apps/mobile/gradlew -p apps/mobile :compose:compileKotlinDesktop :compose:compileAndroidMain :compose:compileKotlinIosSimulatorArm64`를 실행한다.
- [ ] `rg`로 raw `GraphIngest`가 Compose state/`EditorBody`/`EditorView`에 남지 않았고 `EditorGraphSource` 호출이 제거됐는지 확인한다.
- [ ] 가능한 경우 JVM desktop에서 로딩 중 resize/theme 변경, reload/retry, 화면 이탈/재진입을 반복하고 새 `graph ingest already finished` 로그가 없는지 확인한다.
- [ ] 관련 없는 파일을 수정하거나 stage하지 않고 최종 diff와 `git diff --check`를 확인한다. 저장소 지침에 따라 커밋은 만들지 않는다.
