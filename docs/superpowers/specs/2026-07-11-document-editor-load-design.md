# DocumentEditorLoad 기반 문서 Editor 생성 수명 설계

Status: approved
Date: 2026-07-11
Scope: `apps/mobile/compose`의 문서 snapshot ingest, Editor 생성, Compose attach, sync handoff

## 배경

JVM desktop의 Android Studio 로그에 다음 Editor 오류가 기록되었다.

```text
Editor failed: co.typie.editor.EditorException: graph ingest already finished
```

Rust FFI의 `GraphIngest`는 내부 graph buffer를 `Option<Vec<u8>>`로 보유한다. `abort()`, `finish()`, `finishWithPending()`은 모두 이 buffer를 `take()`하는 terminal operation이다. 한 terminal operation이 실행된 뒤 같은 handle을 다시 abort하거나 finish하면 `graph ingest already finished`가 발생한다.

현재 모바일 Compose 경로에서는 `DocumentGraphLoader`가 streaming snapshot을 조립한 뒤 raw `GraphIngest`를 `EditorScreen`에 넘긴다. 화면은 이 handle을 Compose state에 저장하고 `EditorGraphSource.Ingest`를 거쳐 `EditorView`에 전달한다. `EditorView`의 Editor 생성 `LaunchedEffect`는 viewport, density, theme 등의 변경으로 재시작될 수 있다. 첫 effect가 handle을 finish한 뒤 같은 handle을 받은 다음 effect가 다시 finish하면 위 오류가 발생한다.

이 오류는 같은 날 발생한 Skiko Bitmap native crash와는 별개다. Bitmap crash는 게시된 Skia object의 수명 문제였고 별도 설계와 수정으로 다룬다. 본 문서는 one-shot `GraphIngest`가 restartable Compose effect에 노출된 수명 경계만 해결한다.

## 목표

- streaming snapshot ingest를 유지한다.
- `GraphIngest` handle당 `finishWithPending()` 또는 `abort()` 중 하나의 terminal operation만 실행한다.
- Compose effect 재시작과 awaiter 취소가 동일 handle의 재소비로 이어지지 않게 한다.
- 하나의 Editor가 하나의 snapshot, sync baseline, local pending changes 조합에서 생성되도록 한다.
- snapshot 교체, retry, 화면 종료에서 이전 load와 늦게 완료된 Editor를 안전하게 정리한다.
- Editor 생성 중 viewport, density, theme이 바뀌어도 생성을 재시작하지 않고 최신 환경으로 attach한다.
- snapshot 완료와 live sync 시작 사이의 remote changeset을 누락하거나 중복 적용하지 않는다.
- 기존 `EditorRuntime` 오류 보고와 Editor 소유권 경계를 유지한다.
- 테스트를 위해 production 코드를 불필요하게 분할하거나 test-only API를 추가하지 않는다.

## 비목표

- 전체 Editor bootstrap과 sync session을 하나의 대형 상태 머신으로 재설계하지 않는다.
- snapshot 전체를 별도 `ByteArray`로 다시 버퍼링하지 않는다.
- Rust `GraphIngest`를 replayable resource로 변경하지 않는다.
- 일반 Compose effect scheduler나 `EditorRuntime` 전체를 재설계하지 않는다.
- Skiko Bitmap crash, surface scheduler, render 성능 문제를 함께 수정하지 않는다.
- retry를 소비된 handle에 적용하지 않는다.

## 검토한 접근

### `DocumentEditorLoad`

하나의 snapshot마다 `DocumentEditorLoad` 객체를 만들고, raw handle과 Editor 생성 결과를 객체 내부에 숨긴다. 동일 객체를 여러 번 await해도 생성은 한 번만 실행한다.

- 장점: streaming을 유지하면서 one-shot 소비, snapshot identity, 취소, 정리를 한 경계에서 해결한다.
- 장점: UI, runtime, sync transport를 하나의 controller로 합치지 않는다.
- 단점: 내부에는 terminal operation을 직렬화하는 최소 수명 상태가 필요하다.

이 접근을 채택한다.

### 완전한 Editor bootstrap 상태 머신

snapshot 수신부터 Editor 생성, runtime attach, sync engine 시작까지 한 owner가 관리한다.

- 장점: 모든 전이를 한곳에서 명시할 수 있다.
- 단점: UI 환경, runtime, websocket, sync engine이 강하게 결합되고 현재 오류에 비해 변경 범위가 크다.

현재 범위에는 채택하지 않는다.

### 기존 raw handle에 consumed guard 추가

Compose state는 유지하고 boolean 또는 generation check로 두 번째 finish를 막는다.

- 장점: diff가 작다.
- 단점: raw one-shot handle이 restartable UI에 계속 노출된다.
- 단점: viewport/theme 재시작, snapshot 교체, baseline 결합, 실패 memoization을 각각 별도로 방어해야 한다.

근본 수명 경계를 고치지 않으므로 채택하지 않는다.

### 전체 snapshot byte buffering

stream을 모두 `ByteArray`로 모은 뒤 필요할 때마다 새 Editor를 만든다.

- 장점: 입력이 replayable해져 소비 문제가 단순해진다.
- 단점: streaming 유지 조건을 깨고 snapshot 크기만큼 추가 메모리와 복사를 요구한다.

채택하지 않는다.

## 핵심 계약

### load identity

`DocumentEditorLoad` 객체 하나가 snapshot 한 세대를 나타낸다. 별도 숫자 generation을 외부에 노출하지 않는다. 현재 load인지 비교할 때 객체 identity를 사용한다.

현재 `DocumentGraphLoader`의 숫자 generation은 production 소비자가 없고 테스트만 값을 확인한다. `DocumentEditorLoad` identity가 같은 역할을 담당하므로 `Loaded.generation`, 내부 `nextGeneration`, state의 generation을 제거한다.

`SnapshotRestart`는 아직 전송되지 않은 현재 ingest handle을 abort하고 새 handle로 수신을 이어 간다. `SnapshotEnd`에서 전송된 handle은 그때 생성되는 새 `DocumentEditorLoad`가 책임진다.

### 일관된 생성 입력

하나의 load는 다음 값을 같은 snapshot 단위로 묶는다.

- 외부에 노출하지 않는 `GraphIngest`
- snapshot 종료 event의 최초 `DocumentSyncBaseline`
- snapshot 종료 시점에 `ChangesetDeltaStore`에서 한 번 읽은 local pending changes 목록

pending changes는 load 생성 전에 한 번 읽어 list snapshot으로 전달한다. load는 `ChangesetDeltaStore`나 데이터베이스를 직접 알지 않는다. 목록과 내부 byte payload는 load 수명 동안 immutable input으로 취급한다.

`DocumentEditorLoad`는 최초 baseline을 read-only로 화면 orchestration에 제공한다. 화면은 이 값에서 catch-up event를 반영한 effective baseline을 계산한다. raw `GraphIngest`는 어떤 형태로도 다시 노출하지 않는다.

새 snapshot이나 retry는 pending changes를 다시 읽고 새 load를 만든다. 이전 load의 pending과 새 snapshot의 baseline을 섞지 않는다.

### 정확히 한 번의 Editor 생성

`DocumentEditorLoad`는 Compose effect와 분리된 비공개 생성 작업 및 memoized 결과를 보유한다. 외부 동작은 개념적으로 다음 두 가지다.

- 첫 유효 viewport와 theme을 seed로 Editor 결과를 await한다.
- load를 idempotent하게 close한다.

동일 load를 여러 coroutine이 await해도 실제 `finishWithPending()`은 한 번만 실행한다. 성공과 실패 모두 memoize한다. 한 awaiter가 취소되어도 내부 생성과 다른 awaiter는 취소되지 않는다.

load 생성 작업은 화면/document session scope의 child로 동작하지만 개별 `LaunchedEffect` job의 child로 두지 않는다. source 교체와 화면 session 종료가 load를 끝내는 명시적 경계다.

### UI와 runtime 경계

`DocumentEditorLoad`는 Editor 생성까지만 담당한다. `EditorRuntime`, Compose surface, dialog, Logger, Sentry를 직접 알지 않는다.

Editor consumer는 load 결과를 받은 뒤 다음을 수행한다.

1. 결과를 만든 load가 여전히 현재 load인지 확인한다.
2. 최신 viewport와 theme snapshot을 Editor에 동기화한다.
3. suspend 동기화가 끝난 뒤 load identity와 환경 snapshot을 다시 확인한다.
4. 환경이 바뀌었다면 최신 snapshot으로 동기화를 반복한다.
5. load와 환경이 모두 그대로일 때 Main-immediate의 같은 non-suspending 구간에서 최종 identity check와 `EditorRuntime.attach(editor)`를 수행한다.
6. 이미 교체되거나 닫힌 load의 결과는 attach하지 않고 폐기한다.

`EditorRuntime`은 active Editor, attach/clear에 따른 폐기, Editor 오류 상태를 계속 소유한다. load는 UI runtime의 역할을 흡수하지 않는다.

## 구성요소

### `DocumentGraphLoader`

websocket snapshot stream을 `GraphIngest`로 조립한다.

- `SnapshotChunkEvent`: 현재 handle에 append한다.
- `SnapshotRestart`: 아직 transferred되지 않은 handle을 abort하고 새 ingest를 시작한다.
- `SnapshotEndEvent`: handle과 baseline을 `Loaded` event로 transfer한다.
- `ReloadEvent`와 cancel: 아직 receiving 중인 handle만 abort한다.
- permanent error: receiving handle을 정리하고 실패 event를 반환한다.

Editor 생성, local pending read, Compose state, 오류 UI는 담당하지 않는다. 숫자 generation은 제거한다.

### `DocumentEditorLoad`

`co.typie.editor.sync` 영역의 `internal` 타입으로 둔다. transport 전용 loader보다 한 단계 위에서 snapshot 입력과 Editor 생성을 결합한다.

책임은 다음과 같다.

- raw `GraphIngest` 은닉
- 최초 baseline과 pending list snapshot 보관
- 최초 viewport/theme을 이용한 단일 Editor 생성
- 성공 또는 실패 결과 memoization
- awaiter 취소와 생성 작업 수명 분리
- close와 finish의 terminal operation 직렬화
- 닫힌 뒤 늦게 완료된 Editor 폐기

생성에 필요한 screen/editor scope와 Editor runtime-error callback은 기존 production 의존성을 전달받아 사용할 수 있다. 이 callback은 생성 실패를 보고하는 정책이 아니라, 생성된 Editor가 이후 발생시키는 오류를 기존 `runtime.reportError(editor, error)`로 연결하기 위한 것이다.

### `EditorScreen`

문서 단위 orchestration을 유지한다.

- snapshot 완료 시 pending changes를 한 번 읽는다.
- 이전 runtime/load/queue를 정리하고 새 load를 설치한다.
- 현재 load identity와 catch-up changeset queue를 관리한다.
- catch-up event를 적용해 effective sync baseline을 만든다.
- current load와 Editor가 일치할 때만 `SyncEngine`과 `RemoteChangesetPipeline`을 연결한다.
- retry 시 새 snapshot과 새 load를 만든다.

### `EditorBody`와 `EditorView`

raw `EditorGraphSource` 대신 `DocumentEditorLoad`를 받는다.

현재 `EditorGraphSource.Bytes`는 production 호출처가 없고 `EditorGraphSource.Ingest`만 문서 화면에서 사용된다. 따라서 `EditorGraphSource`와 `Editor.createFromSource()`를 제거한다. preview 등의 byte/doc 기반 경로는 기존 `Editor.create`, `Editor.createWithPending`, `Editor.createFromDoc` 계열을 유지한다.

`EditorView`는 다음 UI 환경을 추적한다.

- viewport width와 height
- density가 반영된 scale factor
- theme variant

최초 유효 값은 생성 seed다. 생성 중 값이 바뀌어도 내부 작업을 취소하거나 재시작하지 않는다. 생성 직후 최신 환경을 적용한 뒤 attach한다. attach 이후에는 기존 resize와 `ThemeVariantChanged` event 경로를 사용한다.

### `EditorRuntime`

기존 역할을 유지한다.

- active Editor 보관
- 새 Editor attach와 이전 Editor dispose
- clear와 화면 이탈 정리
- Logger와 Sentry 보고
- `runtime.error`와 dialog 연결

`DocumentEditorLoad`가 오류 보고 정책을 중복 구현하지 않는다.

## 데이터 흐름

### snapshot 수신과 load 생성

1. `EditorScreen`이 `DocumentWsChannel.freshSubscribe()`를 collect한다.
2. `DocumentGraphLoader`가 chunk를 `GraphIngest`에 append한다.
3. `SnapshotEnd`에서 loader가 handle과 최초 baseline을 transfer한다.
4. transfer 직후부터 새 load를 만들 때까지는 snapshot-completion block이 handle의 임시 owner다.
5. 화면은 `ChangesetDeltaStore.load(documentId)`를 한 번 호출해 pending list snapshot을 만든다.
6. pending read가 실패·취소되거나 document/session이 교체되면 임시 owner가 handle을 한 번 abort한다.
7. pending read가 성공하고 같은 document/session이 여전히 current일 때만 handle을 새 `DocumentEditorLoad`에 transfer한다. 이 transfer 이후 임시 owner는 abort하지 않는다.
8. 이전 Editor는 `runtime.clear()`로 정리하고 이전 load를 `close()`한다.
9. 이전 catch-up queue를 폐기한 뒤 새 `DocumentEditorLoad`를 current load로 설치한다.
10. 이 시점 이후 Editor가 live sync를 받을 준비가 될 때까지 도착하는 changeset은 새 load identity에 묶인 queue에 쌓는다.

pending read와 load transfer는 `try/finally` 형태의 ownership handoff로 구현한다. pending read 중 websocket collector가 suspend되어도 channel buffer가 이후 event 순서를 유지한다. pending read가 실패하면 임시 handle을 abort하고 load를 만들지 않은 채 현재 화면 오류로 처리한다.

### Editor 생성과 attach

1. `EditorView`는 viewport가 유효할 때 current load를 await한다.
2. load 내부에서 pending을 encode하고 `finishWithPending()`을 한 번 실행한다.
3. effect가 viewport, density, theme 변경으로 취소되면 다음 effect가 같은 load 결과를 다시 await한다.
4. 생성 완료 시 current load identity를 다시 확인한다.
5. 교체된 load라면 결과를 폐기한다.
6. 현재 load라면 최신 viewport와 theme snapshot을 반영한다.
7. 동기화가 suspend된 동안 load 또는 환경이 바뀌었는지 다시 확인한다.
8. 환경이 바뀌었다면 최신 snapshot으로 동기화를 반복한다.
9. load와 환경이 그대로일 때 Main-immediate의 같은 non-suspending 구간에서 최종 current-load check와 `EditorRuntime.attach(editor)`를 수행한다.

### catch-up에서 live sync로 handoff

snapshot 종료 뒤 Editor와 live pipeline이 준비되기 전에 도착한 `ChangesetsEvent`를 버리지 않는다.

1. 현재 load의 pipeline이 active가 될 때까지 event를 load identity와 함께 queue한다.
2. Editor가 준비되면 queue를 순서대로 적용한다.
3. 각 event의 `seq`, `heads`, `durableHeads`를 반영해 effective baseline을 마지막 event까지 전진시킨다.
4. queue drain의 suspend 지점 뒤에도 current load와 active Editor identity를 다시 확인한다.
5. queue가 비고 identity가 그대로일 때 effective baseline으로 `SyncEngine`과 `RemoteChangesetPipeline`을 구성한다.
6. pipeline 활성화 직전에 load와 Editor identity를 최종 확인한다.
7. main-thread에 직렬화된 구간에서 live subscription을 먼저 활성화한 뒤 current load를 sync-active 상태로 전환한다.
8. 그 뒤 event는 queue가 아니라 live pipeline만 처리한다.

구독 활성화와 queue 종료 사이에 event loop가 끼어들지 않게 기존 Main-immediate/undispatched 시작 패턴을 사용할 수 있다. 핵심 계약은 다음 두 가지다.

- pipeline이 구독하기 전에 queueing을 중단하지 않는다.
- pipeline이 구독한 뒤 같은 event를 queue에서도 적용하지 않는다.

이를 통해 snapshot과 live sync 사이의 event 누락과 중복을 모두 방지한다. 구현을 테스트하기 위해 별도 public coordinator를 만들지는 않는다. 화면 내부 상태가 실제로 복잡해져 독립 domain unit이 필요한 경우에만 production 개념으로 추출한다.

## viewport와 theme 정책

Editor 생성에는 최초 유효 viewport와 theme이 필요하다. 그러나 이 값들은 Compose 환경 변화로 자주 바뀔 수 있으며, 변화 자체가 snapshot을 다시 소비할 이유는 아니다.

- 최초 유효 viewport, scale factor, theme은 생성 seed다.
- 생성 중 resize, density 변경, theme 변경은 내부 생성 작업을 취소하지 않는다.
- `EditorView`는 최신 값을 별도로 관찰한다.
- 생성 완료 직후 최신 viewport와 theme을 적용한다.
- 그 뒤 runtime에 attach한다.
- attach 이후 viewport 변경은 기존 `SystemEvent.Resize`를 사용한다.
- attach 이후 theme 변경은 기존 `ThemeVariantChanged` 경로를 사용한다.

따라서 effect는 환경 변화에 따라 다시 실행될 수 있지만, load 결과를 await하고 최신 환경을 동기화할 뿐 graph ingest를 다시 finish하지 않는다.

## 교체와 정리

snapshot 교체 순서는 다음과 같다.

1. `runtime.clear()`로 active Editor를 화면에서 제거하고 dispose한다.
2. 이전 `DocumentEditorLoad.close()`를 호출한다.
3. 이전 load의 catch-up queue와 sync binding을 폐기한다.
4. 새 load를 설치한다.

화면 종료도 snapshot 교체와 동일하게 `runtime.clear()`를 먼저 수행한 뒤 current load를 `close()`하고 queue/sync binding을 정리한다. `Editor.dispose()`는 idempotent하므로 runtime과 load의 cleanup safety net이 같은 Editor에 도달해도 안전하다.

### GraphIngest terminal operation 직렬화

Rust FFI에서 `abort()`와 `finishWithPending()`은 같은 buffer를 소비한다. `close()`가 finish와 동시에 abort를 호출해서는 안 된다.

개념적 내부 규칙은 다음과 같다.

- 생성 시작 전 close: load가 handle을 한 번 abort한다.
- 생성 작업이 handle을 맡은 뒤: 그 작업만 terminal operation에 접근한다.
- 생성 중 close: 작업을 취소 표시하지만 별도 abort를 finish와 경합시키지 않는다.
- finish 호출 전에 취소를 관찰한 생성 작업: 자신이 맡은 handle을 abort하고 끝낸다.
- synchronous FFI finish가 이미 실행 중인 경우: finish 완료 뒤 load가 닫혔는지 확인하고 생성된 Editor를 폐기한다.
- 생성 완료 후 close: memoized Editor를 폐기한다.

이 상태는 load의 private 구현이다. 외부 API에 claim, consumed boolean, generation, public state machine을 노출하지 않는다.

`close()`는 반복 호출에 안전해야 한다. close가 생성 중 synchronous FFI 호출을 강제로 중단한다는 보장은 하지 않는다. 대신 닫힌 load에서 결과가 외부로 탈출하지 않고 완료 즉시 정리된다는 것을 보장한다.

## 오류 처리

### 보고하지 않는 정상 종료

- `EditorView` await coroutine 취소
- viewport/theme effect 재시작
- snapshot 교체에 따른 load close
- 화면 종료에 따른 load close
- 닫힌 load의 늦은 결과 폐기

이 경로는 Logger, Sentry, 오류 dialog에 보고하지 않는다.

### 보고하는 실패

local pending read 실패는 load 생성 전 실패다. 화면 session이 여전히 현재일 때 `EditorRuntime.reportError()`로 전달한다.

graph decode 또는 Editor 초기화 실패는 해당 load에 memoize한다. 같은 load를 다시 await해도 finish를 재실행하지 않는다. 실패한 load가 여전히 current일 때만 `EditorRuntime.reportError()`로 한 번 보고한다. 이미 교체된 load의 뒤늦은 실패는 사용자 오류나 Sentry event로 만들지 않는다.

attach 이후 Editor 내부 오류는 기존 `runtime.reportError(editor, error)` 경로를 유지한다. 이 경로는 stale Editor 오류를 무시하고, current Editor 오류만 Logger, Sentry, runtime state, dialog로 연결한다.

websocket permanent failure는 기존 경계를 유지한다. Editor가 아직 없는 초기 attach 실패는 loader retry dialog를 사용하고, active/current Editor session의 terminal failure는 runtime 오류 경로로 전달한다.

### retry

실패한 `DocumentEditorLoad`는 자동 재사용하지 않는다. 사용자가 retry하면 runtime 오류를 지우고 서버 snapshot부터 다시 요청한다. 새 snapshot, 새 baseline, 새 pending list, 새 `DocumentEditorLoad`를 만든다.

소비된 `GraphIngest`에 대한 자동 retry 경로는 존재하지 않는다.

## 테스트 정책

테스트가 production 설계를 끌고 가지 않게 한다.

- 테스트 전용 Editor factory, coordinator, public state getter, consumed flag를 production 코드에 추가하지 않는다.
- private 상태 이름이나 모든 close 조합을 고정하는 exhaustive test를 만들지 않는다.
- 기존 transport와 pipeline 테스트가 보장하는 동작을 새 테스트에서 중복하지 않는다.
- 자연스러운 production 경계로 검증할 수 있는 핵심 회귀만 자동화한다.

### 필수 회귀 테스트

`DocumentEditorLoad`의 실제 API를 사용해 동일 load를 반복 await해도 `finishWithPending()`이 한 번만 호출되는 테스트를 추가한다.

기존 `GraphIngest` interface의 fake가 finish 횟수를 기록한 뒤 의도적인 sentinel failure를 던지게 하면 실제 load의 failure memoization 경로를 통과할 수 있다. fake Editor 또는 테스트 전용 production factory가 필요하지 않다. 두 번째 await는 같은 실패 결과를 관찰하되 finish count는 계속 1이어야 한다.

이 테스트가 직접 보호하는 회귀는 다음과 같다.

```text
Compose effect 재진입
  -> 같은 DocumentEditorLoad 재await
  -> 같은 GraphIngest 두 번째 finish 금지
```

### 기존 테스트 조정

`DocumentGraphLoaderTest`는 generation number assertion을 제거한다. 다음 production 계약을 검증하는 기존 테스트는 유지한다.

- chunk append
- snapshot restart 시 이전 receiving handle abort
- snapshot end 시 handle transfer
- transfer 전 cancel/reload cleanup

새 구현이 별도 seam 없이 자연스럽게 드러내는 중요한 회귀가 있다면 테스트를 추가할 수 있다. viewport/theme 조합, 모든 close interleaving, catch-up handoff를 테스트하기 위해 production 로직을 인위적으로 분할하지 않는다.

### 빌드와 런타임 검증

Gradle/KMP 명령은 Typie build queue를 통해 실행한다.

- 최소 관련 common/desktop test
- Desktop compile
- Android compile
- iOS compile

JVM desktop에서 실제 문서를 사용해 다음을 반복한다.

1. snapshot 로딩 중 창 resize
2. 로딩 중 theme 변경
3. 화면 이탈과 재진입
4. reload와 retry
5. 빠른 snapshot 교체
6. Android Studio/Gradle daemon 로그에서 새 `graph ingest already finished`가 발생하지 않는지 확인

실제 성공 경로의 viewport/theme/FFI 동작은 이 runtime 검증으로 확인한다. production 개념을 단순화하는 자연스러운 seam이 없다면 UI lifecycle만을 위한 별도 test abstraction을 만들지 않는다.

## 예상 변경 범위

- `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/sync/ws/DocumentGraphLoader.kt`
  - generation 제거
  - streaming ingest/transfer 책임 유지
- `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/sync/DocumentEditorLoad.kt`
  - 새 internal load 타입
- `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/Editor.kt`
  - `EditorGraphSource`와 `createFromSource()` 제거
  - load가 사용할 좁은 internal ingest 생성 경계 유지
- `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/EditorView.kt`
  - load await 및 최신 viewport/theme attach
- `apps/mobile/compose/src/commonMain/kotlin/co/typie/editor/body/EditorBody.kt`
  - raw source 대신 load 전달
- `apps/mobile/compose/src/commonMain/kotlin/co/typie/screen/editor/editor/EditorScreen.kt`
  - load 생성/교체/close
  - pending list snapshot
  - catch-up queue와 sync handoff
- 관련 common/desktop test

파일 배치는 구현 중 기존 package visibility와 dependency 방향에 맞춰 좁게 조정할 수 있다. `DocumentEditorLoad`의 책임을 분산하거나 테스트만을 위한 public API를 만드는 방향으로는 조정하지 않는다.

## 성공 조건

- raw `GraphIngest`가 Compose state, `EditorBody`, `EditorView` API에 노출되지 않는다.
- `EditorGraphSource.Ingest`와 호출되지 않는 `EditorGraphSource.Bytes` wrapper가 제거된다.
- 동일 `DocumentEditorLoad`를 여러 번 await해도 handle을 한 번만 finish한다.
- terminal 경로마다 handle은 finish 또는 abort 중 하나만 수행한다.
- viewport, density, theme effect 재시작이 Editor 생성을 재실행하지 않는다.
- 최신 viewport와 theme이 attach 전에 적용된다.
- 교체된 load의 Editor와 오류가 current runtime으로 유출되지 않는다.
- snapshot baseline과 local pending changes가 같은 load에 묶인다.
- catch-up에서 live sync로 넘어갈 때 remote changeset이 누락되거나 중복 적용되지 않는다.
- 오류 보고는 기존 `EditorRuntime` 경로에 한 번만 발생한다.
- streaming과 기존 preview 생성 경로를 유지한다.
- 테스트를 위해 불필요한 production abstraction을 추가하지 않는다.

## 범위 밖

- Rust `GraphIngest` API 자체 변경
- graph snapshot wire protocol 변경
- sync database schema 변경
- 일반-purpose Editor bootstrap/session framework
- `EditorRuntime` public API 재설계
- Compose effect scheduler 추상화
- snapshot buffering 또는 replay cache
- Skiko Bitmap 수명과 surface render scheduler
- editor preview 생성 경로 통합
- 테스트 전용 dependency injection framework
