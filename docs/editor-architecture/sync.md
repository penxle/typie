# Sync

## 개요

중앙 서버 + 버전 기반 Step 교환 (ProseMirror collab 방식).
Step.transform()을 통한 OT로 충돌 해결.

## 요구사항

1. 단일 사용자 멀티 기기 동기화 (primary)
2. 기초적 멀티 유저 동시편집 (nice to have)
3. 오프라인 편집 후 best-effort 동기화
4. **일반적 이용 시나리오에서 유실 없는 동기화**

## 서버

문서의 선형 Step 히스토리를 유지하는 단일 authority.

```rust
pub struct SyncServer {
    version: u64,
    steps: Vec<Step>,
    doc: Doc,
}

impl SyncServer {
    fn receive(&mut self, request_id: Uuid, client_version: u64, client_steps: Vec<Step>) -> SyncResponse {
        // 멱등성: 중복 요청 감지
        if self.is_duplicate(request_id) {
            return SyncResponse::Duplicate;
        }

        if client_version == self.version {
            // 충돌 없음
            self.apply_and_persist(client_steps);
            SyncResponse::Accepted { version: self.version }
        } else {
            // 충돌 — transform 필요
            let server_steps = &self.steps[client_version as usize..];
            let rebased = Step::transform_many(&client_steps, server_steps);
            self.apply_and_persist(rebased);
            SyncResponse::Rebased {
                version: self.version,
                server_steps: server_steps.to_vec(),
            }
        }
    }
}
```

## 클라이언트

3가지 상태의 Step을 관리.

```rust
pub struct SyncClient {
    confirmed_version: u64,
    in_flight: Vec<Step>,       // 서버에 보냈지만 확인 안 됨
    pending: Vec<Step>,         // 아직 보내지 않음
}
```

```
확인됨 (synced)          전송됨 (in_flight)       대기중 (pending)
[v0 ... v3]              [step A, step B]         [step C, step D]
```

### 로컬 편집

```rust
fn on_local_steps(&mut self, steps: Vec<Step>) {
    self.pending.extend(steps);
    self.persist_pending();     // 로컬 영속화 (크래시 복구)
    self.try_send();
}

fn try_send(&mut self) {
    if self.in_flight.is_empty() && !self.pending.is_empty() {
        self.in_flight = std::mem::take(&mut self.pending);
        send_to_server(self.confirmed_version, &self.in_flight);
    }
}
```

### 서버 확인 수신

```rust
fn on_confirmed(&mut self, new_version: u64) {
    self.confirmed_version = new_version;
    self.in_flight.clear();
    self.try_send();
}
```

### 리모트 Steps 수신

```rust
fn on_remote_steps(&mut self, server_steps: Vec<Step>) -> Vec<Step> {
    self.in_flight = Step::transform_many(&self.in_flight, &server_steps);
    self.pending = Step::transform_many(&self.pending, &server_steps);
    server_steps  // Runtime이 State에 적용
}
```

## 오프라인 지원

```
온라인: 편집 → pending에 추가 → 즉시 전송
오프라인: 편집 → pending에 계속 축적 (로컬 영속화)
복귀: pending 전체를 서버에 전송 → 서버가 transform → 적용
```

## Step.transform

OT의 핵심 프리미티브. 두 동시 Step이 있을 때 서로의 효과를 반영한 변환 생성.

불변식:
```
apply(apply(S, A), transform(B, A)) == apply(apply(S, B), transform(A, B))
```

점진적 구현 전략:
1. **1단계**: 텍스트 삽입/삭제의 오프셋 변환 (가장 빈번한 케이스)
2. **2단계**: 노드 삽입/삭제의 인덱스 변환
3. **3단계**: 구조 변경(SplitNode, MoveNode 등)의 transform (드문 케이스)

## Undo 스택 rebase

리모트 Steps 도착 시 undo 스택도 transform:

```rust
fn on_remote_steps(&mut self, remote_steps: &[Step]) {
    for entry in &mut self.history.undos {
        entry.steps = Step::transform_many(&entry.steps, remote_steps);
    }
}
```

## 유실 방지 조건

일반적 시나리오에서 데이터 유실이 없으려면 4가지가 필요:

### 1. Step.transform() 정확성
텍스트 OT 불변식이 모든 Step 조합에 대해 성립해야 함. 철저한 테스트 필수.

### 2. 서버 측 내구성
ACK 전에 steps를 디스크/DB에 영속화. 서버 크래시 후에도 steps 보존.

### 3. 클라이언트 측 내구성
pending steps를 로컬 스토리지에 영속화. 앱 크래시 시 미전송 steps 복구.

### 4. 멱등성
```rust
struct SyncRequest {
    request_id: Uuid,
    client_version: u64,
    steps: Vec<Step>,
}
```
네트워크 불안정으로 재전송 시 서버가 request_id로 중복 감지.

## 시나리오별 데이터 유실 분석

| 시나리오 | 유실 | 비고 |
|---|---|---|
| 디바이스 A 편집 → B에서 열기 | 없음 | 순차적, 충돌 없음 |
| 오프라인 편집 → 복귀 (혼자) | 없음 | 다른 변경 없음 |
| 두 사용자, 다른 문단 편집 | 없음 | transform이 오프셋만 조정 |
| 두 사용자, 같은 문단 텍스트 삽입 | 없음 | OT가 양쪽 모두 보존 |
| 한 사용자 삭제 + 다른 사용자 같은 위치 편집 | 없음 | 삭제 범위 안의 삽입은 보존 |
| 한 사용자 노드 삭제 + 다른 사용자 그 안에서 편집 | 유실 가능 | 엣지 케이스, 일반적 시나리오 아님 |
