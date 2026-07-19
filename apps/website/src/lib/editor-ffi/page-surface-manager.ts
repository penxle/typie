import type { AttachOutcome, LeaseToken, PageZone, SurfaceBackend } from './gl-context-pool';

// glContextPool의 실제 토큰 API를 (editorKey, page)로 커링한 어댑터가 구현해야 하는 포트.
// updateDemand는 정책 조회(자기 자신은 콜백을 받지 않음), acquireLease는 실제 슬롯 예약을
// 별도 호출로 분리한다 — 풀이 간접 승격 시 건네는 acquireHint를 그대로 소비하기 위함이다.
export type PoolPort = {
  updateDemand: (zone: PageZone) => SurfaceBackend;
  acquireLease: (requested: SurfaceBackend) => { backend: 'cpu' } | { backend: 'gl'; token: LeaseToken };
  ackAttached: (token: LeaseToken, actual: AttachOutcome) => void;
  cancelReservation: (token: LeaseToken, reason: string) => void;
  beginRelease: (token: LeaseToken) => void;
  ackReleased: (token: LeaseToken) => void;
  notePresent: (token?: LeaseToken) => void;
  noteGlFailure: (incident: LeaseToken) => void;
  // gl 요청이 예산 부족으로 cpu로 폴백해 그 cpu가 실제 커밋됐을 때만 호출한다(timeout/gl-dead 아님).
  noteBudgetFallback: () => void;
  backendOf: () => SurfaceBackend | undefined;
  leave: () => void;
  forget: () => void;
};

export type ManagerEffects<C> = {
  createCanvas: (backend: SurfaceBackend) => C;
  styleCanvas: (canvas: C) => void;
  attach: (canvas: C, backend: SurfaceBackend) => AttachOutcome | 'cpu-oversized';
  detach: () => void;
  requestRender: () => void;
  // 숨김 탭(rAF 정지)에서는 present가 커밋될 수 없다 — 클록 게이트가 이를 조회한다.
  isSuspended: () => boolean;
  onPresented: (listener: () => void) => () => void;
  addContextListeners: (canvas: C, isCurrent: () => boolean) => () => void;
  disposeGlContext: (canvas: C) => void;
  releaseCpuBacking: (canvas: C) => void;
  promote: (next: C, previous: C | undefined) => void;
  removeNode: (canvas: C) => void;
  schedule: (fn: () => void, ms: number) => () => void;
  defer: (fn: () => void) => void;
  pool: PoolPort;
  // 선택적 계측 훅(surface-stats). 없으면(테스트·프로덕션 기본) 매니저 동작에 영향이 없다.
  // 이벤트: 'mount:<cause>' | 'park' | 'swapTimeout1' | 'swapTimeout2' | 'finishSwap:<ms>' | 'resume'.
  note?: (event: string) => void;
};

export const SWAP_TIMEOUT_MS = 1000;
export const RESTORE_WATCHDOG_MS = 2000;
// overscan 반경 진입(빠른 스크롤 churn의 원천) 마운트를 이만큼 지연시켜, 페이지가 반경에
// 머무름을 증명한 뒤에만 GL 컨텍스트를 만든다 — transit-only 페이지의 mount→park churn 차단.
export const ACQUIRE_DEBOUNCE_MS = 200;
// failedParked 재시도 백오프(2s→5s→10s, 마지막에서 고정). 커밋(finishSwap)·park 시 리셋된다.
export const PARKED_RETRY_MS = [2000, 5000, 10_000] as const;

export type VisibilityState = { inAcquire: boolean; inRelease: boolean; isVisible: boolean };

type Slot<C> = {
  canvas: C;
  isGl: boolean;
  token: LeaseToken | undefined;
  acked: boolean;
  listeners: () => void;
  // gl을 요청했으나 예산 부족으로 cpu가 붙은 슬롯 — 커밋(finishSwap) 시 pool에 un-poison 신호를 보낸다.
  budgetFallback: boolean;
  // note가 활성일 때만 채워지는 마운트 시각(finishSwap의 mount→commit 지연 계측용). 없으면 0.
  mountedAt: number;
};

export function createPageSurfaceManager<C>(effects: ManagerEffects<C>) {
  let live: Slot<C> | undefined;
  let pending: Slot<C> | undefined;
  let pendingCleanup: (() => void) | undefined;
  // pending의 현재 swap 타임아웃 canceller. 숨김 중 재예약이 이 참조를 교체하므로 factory
  // 스코프에 둔다 — pendingCleanup은 늘 최신 타이머를 취소한다.
  let cancelSwapTimeout: (() => void) | undefined;
  let attached = false;
  let epoch = 0;
  let timedOutOnce = false;
  let cancelWatchdog: (() => void) | undefined;
  // reconcile이 마지막으로 결정한 "이 페이지는 라이브여야 한다" — live/pending이 둘 다 비어도
  // (failedParked) 이 값이 true로 남아 있으면 재진입(acquire 재진입/쿨다운 웨이크)이 유효하다.
  let wantsLive = false;
  // overscan 진입 디바운스 타이머 canceller — park/mount 시 취소된다.
  let cancelDebounce: (() => void) | undefined;
  // failedParked 재시도 타이머 canceller와 백오프 인덱스(finishSwap·park에서 리셋).
  let cancelParkedRetry: (() => void) | undefined;
  let parkedRetryIndex = 0;

  const disposeSlot = (slot: Slot<C>) => {
    slot.listeners();
    if (slot.isGl) effects.disposeGlContext(slot.canvas);
    else effects.releaseCpuBacking(slot.canvas);
    effects.removeNode(slot.canvas);
    if (slot.token !== undefined) {
      const releasedToken = slot.token;
      if (slot.acked) {
        // 처분 시작 — ackReleased는 defer해 풀이 대기자를 승격시키는 시점을 현재 tick 밖으로 민다.
        effects.pool.beginRelease(releasedToken);
        effects.defer(() => effects.pool.ackReleased(releasedToken));
      } else {
        // ack가 아직 안 왔다 — released 왕복 없이 예약 자체를 원자적으로 취소한다. defer하면
        // 이미 큐에 쌓인 ackAttached보다 늦게 실행돼 예약을 놓칠 수 있으므로 항상 동기 호출.
        effects.pool.cancelReservation(releasedToken, 'abandoned');
      }
    }
  };

  const cancelPending = () => {
    epoch += 1;
    pendingCleanup?.();
    pendingCleanup = undefined;
    if (pending) {
      disposeSlot(pending);
      pending = undefined;
    }
  };

  const detachIfAttached = () => {
    if (!attached) return;
    effects.detach();
    attached = false;
  };

  const park = () => {
    effects.note?.('park');
    cancelDebounce?.();
    cancelDebounce = undefined;
    cancelParkedRetry?.();
    cancelParkedRetry = undefined;
    parkedRetryIndex = 0;
    cancelWatchdog?.();
    cancelWatchdog = undefined;
    cancelPending();
    detachIfAttached();
    if (live) {
      disposeSlot(live);
      live = undefined;
    }
    timedOutOnce = false;
    // leave는 등록된 적 없어도 안전한 no-op이므로 무조건 호출해 failedParked에서 곧바로
    // out-of-view로 전이하는 경로에서도 풀 수요가 새지 않게 한다.
    effects.pool.leave();
  };

  // 스와프 확정은 committed present에서만 일어난다. 타임아웃은 승격이 아니라
  // "pending 폐기 + 구 live 유지 + 1회 강제 cpu 폴백"이다.
  const finishSwap = (myEpoch: number) => {
    if (myEpoch !== epoch || !pending) return;
    // 커밋 성공 — failedParked 재시도 백오프를 리셋하고 대기 중 재시도 타이머를 취소한다.
    cancelParkedRetry?.();
    cancelParkedRetry = undefined;
    parkedRetryIndex = 0;
    cancelWatchdog?.();
    cancelWatchdog = undefined;
    pendingCleanup?.();
    pendingCleanup = undefined;
    const next = pending;
    pending = undefined;
    if (next.mountedAt > 0) effects.note?.(`finishSwap:${Date.now() - next.mountedAt}`);
    if (live) disposeSlot(live);
    effects.promote(next.canvas, live?.canvas);
    live = next;
    timedOutOnce = false;
    effects.pool.notePresent(next.token);
    // 예산 폴백 cpu가 실제 커밋됐고 구 gl 표면도 방금 처분됐다 — 이제 leaseless이므로 stale 'gl'
    // 원장을 un-poison하도록 신호한다(슬롯이 비면 승격 콜백을 받기 위함). 강제 cpu 폴백은 이 플래그가
    // false라 신호하지 않는다(계약 5).
    if (next.budgetFallback) effects.pool.noteBudgetFallback();
  };

  const onSwapTimeout = (myEpoch: number) => {
    if (myEpoch !== epoch || !pending) return;
    // 숨김 중엔 present가 커밋될 수 없다 — 실패로 계상하지 말고(강등·강제 폴백 없이) 같은
    // 에포크로 재예약만 한다. pendingCleanup은 factory 스코프의 cancelSwapTimeout을 읽으므로
    // 이 새 타이머를 취소한다.
    if (effects.isSuspended()) {
      cancelSwapTimeout = effects.schedule(() => onSwapTimeout(myEpoch), SWAP_TIMEOUT_MS);
      return;
    }
    const failedToken = pending.token;
    cancelWatchdog?.();
    cancelWatchdog = undefined;
    cancelPending();
    detachIfAttached();
    if (failedToken !== undefined) effects.defer(() => effects.pool.noteGlFailure(failedToken));
    if (timedOutOnce) {
      effects.note?.('swapTimeout2');
      // 2차 타임아웃 — 강제 폴백마저 커밋되지 못했다. 구 live까지 전부 처분하고 안정된
      // failedParked로 수렴한다. wantsLive는 건드리지 않아 재진입(acquire/웨이크)이 살아 있다.
      // radius 재진입/쿨다운 웨이크에만 기대지 않도록 백오프 재시도 타이머도 함께 건다 —
      // 뷰포트에 머무는 페이지가 스크롤 왕복 없이도 스스로 치유되게 한다.
      timedOutOnce = false;
      if (live) {
        disposeSlot(live);
        live = undefined;
      }
      scheduleParkedRetry();
      return;
    }
    effects.note?.('swapTimeout1');
    timedOutOnce = true;
    mount('cpu', undefined, 'forced-cpu');
  };

  const mount = (requested: SurfaceBackend, presetToken?: LeaseToken, cause = 'mount') => {
    effects.note?.(`mount:${cause}`);
    cancelDebounce?.();
    cancelDebounce = undefined;
    // 대기 중 failedParked 재시도 타이머를 취소한다 — 재진입 mount(reconcile/onPoolBackend/resume)가
    // 이를 취소하지 않으면 고아 타이머가 누적되고, 그 고아가 발화해 failedParked에서 잉여 mount(새 GL
    // 컨텍스트)를 유발해 F-A가 잡으려는 churn을 되먹인다. 백오프 단계(parkedRetryIndex)는 여기서
    // 리셋하지 않는다 — 진짜 연속 재실패엔 백오프가 계속 올라가야 하므로 리셋은 finishSwap/park에만.
    cancelParkedRetry?.();
    cancelParkedRetry = undefined;
    cancelWatchdog?.();
    cancelWatchdog = undefined;
    cancelPending();
    detachIfAttached();
    const myEpoch = epoch;

    let leaseToken: LeaseToken | undefined;
    let attachBackend: SurfaceBackend;
    let budgetFallback = false;
    if (requested === 'gl' && presetToken !== undefined) {
      // 풀이 간접 승격 콜백에 이미 예약해 둔 토큰 — 재예약(acquireLease 재호출) 금지.
      leaseToken = presetToken;
      attachBackend = 'gl';
    } else if (requested === 'gl') {
      const lease = effects.pool.acquireLease('gl');
      if (lease.backend === 'gl') {
        leaseToken = lease.token;
        attachBackend = 'gl';
      } else {
        // gl을 원했지만 예산이 없어 cpu로 떨어졌다(실패가 아님) — 커밋 시 원장 un-poison 대상.
        attachBackend = 'cpu';
        budgetFallback = true;
      }
    } else {
      // 강제 cpu 폴백을 포함해 cpu 요청은 풀과 절대 협상하지 않는다(enter 재호출 금지).
      attachBackend = 'cpu';
    }

    const canvas = effects.createCanvas(attachBackend);
    effects.styleCanvas(canvas);
    const listeners = effects.addContextListeners(canvas, () => canvas === live?.canvas || canvas === pending?.canvas);
    const actual = effects.attach(canvas, attachBackend);

    if (actual === 'cpu-oversized') {
      // 대형 cpu 할당으로 도망치지 않는다 — 즉시 처분하고 failedParked 상당으로 수렴한다
      // (재시도는 radius 재진입에만 맡긴다). gl 요청이 내부적으로 oversized cpu로 떨어진
      // 경우 예약 토큰이 남아있으므로, ack 없이 원자적으로 취소해 홀드가 새지 않게 한다.
      listeners();
      effects.detach();
      effects.releaseCpuBacking(canvas);
      effects.removeNode(canvas);
      if (leaseToken !== undefined) effects.pool.cancelReservation(leaseToken, 'cpu-oversized');
      if (live) {
        // 이 시도가 실패하기 전 이미 위쪽의 detachIfAttached()가 구 live의 wasm 바인딩을
        // 걷어갔다(단일 surface 슬롯 FFI 특성상 attach 시도 자체가 교체를 함의한다 — GL→GL
        // 재마운트 등 live가 있는 상태의 재시도). 구 캔버스는 더 이상 유효한 바인딩이 없는
        // 빈 껍데기이므로 시각적 잔상으로도 유지할 수 없다 — 함께 처분한다.
        disposeSlot(live);
        live = undefined;
      }
      return;
    }

    if (actual === 'gl-dead') {
      listeners();
      effects.detach();
      effects.disposeGlContext(canvas);
      effects.removeNode(canvas);
      if (leaseToken !== undefined) {
        const deadToken = leaseToken;
        effects.defer(() => {
          effects.pool.ackAttached(deadToken, 'gl-dead');
          effects.pool.beginRelease(deadToken);
          effects.pool.ackReleased(deadToken);
        });
      }
      mount('cpu', undefined, 'gl-dead-retry');
      return;
    }

    attached = true;
    const slot: Slot<C> = {
      canvas,
      isGl: actual === 'gl',
      token: leaseToken,
      acked: false,
      listeners,
      budgetFallback,
      mountedAt: effects.note ? Date.now() : 0,
    };
    pending = slot;
    if (leaseToken !== undefined) {
      const ackToken = leaseToken;
      effects.defer(() => {
        effects.pool.ackAttached(ackToken, actual);
        slot.acked = true;
      });
    }
    const offPresented = effects.onPresented(() => finishSwap(myEpoch));
    cancelSwapTimeout = effects.schedule(() => onSwapTimeout(myEpoch), SWAP_TIMEOUT_MS);
    pendingCleanup = () => {
      offPresented();
      cancelSwapTimeout?.();
      cancelSwapTimeout = undefined;
    };
    effects.requestRender();
  };

  // 2차 타임아웃으로 failedParked에 수렴했을 때 백오프 재시도를 예약한다. 발화 시 여전히 원하고
  // (live/pending 없음) 숨김이 아니면 재마운트하고, 숨김 중이면 같은 지연으로 재예약한다(숨김
  // 클록 게이트와 일관). backendOf는 최근 정책을 반영하므로 그대로 소비한다.
  const scheduleParkedRetry = () => {
    const delay = PARKED_RETRY_MS[Math.min(parkedRetryIndex, PARKED_RETRY_MS.length - 1)];
    parkedRetryIndex += 1;
    const onRetry = () => {
      if (effects.isSuspended()) {
        cancelParkedRetry = effects.schedule(onRetry, delay);
        return;
      }
      cancelParkedRetry = undefined;
      if (wantsLive && !live && !pending) mount(effects.pool.backendOf() ?? 'cpu', undefined, 'parked-retry');
    };
    cancelParkedRetry = effects.schedule(onRetry, delay);
  };

  return {
    reconcile(state: VisibilityState): void {
      const zone: PageZone = state.isVisible ? 'visible' : 'overscan';
      const shouldBeLive = live || pending ? state.inRelease : state.inAcquire;
      wantsLive = shouldBeLive;
      if (!shouldBeLive) {
        park();
        return;
      }
      // 최초 반경 진입 마운트(live/pending 모두 없음)만 디바운스 대상이다 — staleLive 백엔드
      // 전환·onPoolBackend·로스/resume 마운트는 건드리지 않는다.
      if (!live && !pending) {
        if (!state.isVisible) {
          // overscan 진입(churn 원천): 즉시 마운트/updateDemand를 하지 않는다. 페이지가 반경에
          // 머무름을 증명(ACQUIRE_DEBOUNCE_MS 유지)한 뒤에만 풀 수요 등록 + 마운트한다. 이미
          // 디바운스 중이면 재예약하지 않아 최초 진입 시각 기준 지연을 유지한다.
          if (!cancelDebounce) {
            cancelDebounce = effects.schedule(() => {
              cancelDebounce = undefined;
              if (!wantsLive || live || pending) return;
              mount(effects.pool.updateDemand(zone), undefined, 'reconcile-debounce');
            }, ACQUIRE_DEBOUNCE_MS);
          }
          return;
        }
        // 가시 진입: 사용자에게 보이는 지연 없이 즉시 마운트한다(mount가 대기 중 디바운스 취소).
        mount(effects.pool.updateDemand(zone), undefined, 'reconcile-visible');
        return;
      }
      const backend = effects.pool.updateDemand(zone);
      // updateDemand는 호출자 자신의 승격/강등을 콜백 없이 반환값으로만 통지한다(풀의 silent
      // 경로). live만 있고 pending이 없는데 그 반환값이 현재 live의 실제 backend와 달라진
      // 경우(제자리 zone 전환으로 인한 강등/승격)는 여기서 직접 mount를 걸어줘야 한다 —
      // 그렇지 않으면 정책은 바뀌었는데 캔버스는 영원히 안 바뀐다.
      const staleLive = !pending && live !== undefined && (live.isGl ? 'gl' : 'cpu') !== backend;
      if (staleLive) mount(backend, undefined, 'stale-live');
    },
    onPoolBackend(backend: SurfaceBackend, acquireHint?: LeaseToken): void {
      if (!wantsLive) {
        // 이 페이지는 더 이상 gl을 원치 않는다(park/재활용된 좌표에 뒤늦게 도착한 승격 콜백). 콜백에
        // 실려온 예약 토큰을 그냥 버리면 원장에 orphan reserved lease가 남아 예산이 샌다 —
        // 원자적으로 취소한다(consume 아니면 release, 계약).
        if (acquireHint !== undefined) effects.pool.cancelReservation(acquireHint, 'unwanted-promotion');
        return;
      }
      mount(backend, acquireHint, 'pool-route');
    },
    // 로스: 즉시 실패를 기록하고 복원 워치독을 건다 — restored가 제한 시간 내
    // 오지 않으면 fresh 캔버스로 재마운트한다(영구 blank 방지).
    onContextLost(): void {
      const incident = pending?.token ?? live?.token;
      if (incident !== undefined) effects.defer(() => effects.pool.noteGlFailure(incident));
      cancelWatchdog?.();
      // 숨김 중엔 재마운트해도 present가 커밋될 수 없어 컨텍스트 churn만 유발한다 — 발화 시점에
      // 여전히 숨김이면 재마운트 대신 재예약하고, 복귀 후 발화 때 비로소 재마운트한다.
      const onWatchdog = () => {
        if (effects.isSuspended()) {
          cancelWatchdog = effects.schedule(onWatchdog, RESTORE_WATCHDOG_MS);
          return;
        }
        cancelWatchdog = undefined;
        if (wantsLive) mount(effects.pool.backendOf() ?? 'cpu', undefined, 'loss-watchdog');
      };
      cancelWatchdog = effects.schedule(onWatchdog, RESTORE_WATCHDOG_MS);
    },
    onContextRestored(): void {
      cancelWatchdog?.();
      cancelWatchdog = undefined;
      if (wantsLive) mount(effects.pool.backendOf() ?? 'cpu', undefined, 'restored');
    },
    remountFromLoss(): void {
      // rAF gl-dead 폴 경유 재마운트 — webglcontextrestored('restored')와 구분해 계상한다
      // (필드 데이터에서 dead-remount vs restored 구분이 진단 핵심).
      if (wantsLive) mount(effects.pool.backendOf() ?? 'cpu', undefined, 'dead-remount');
    },
    // 가시성 복귀 시 손: failedParked(live/pending 없음)를 치유하고, 숨김 중 정체된 pending은
    // 렌더를 한 번 재촉해 커밋을 유도한다. 원치 않는(park된) 페이지는 건드리지 않는다.
    resume(): void {
      effects.note?.('resume');
      if (!wantsLive) return;
      if (pending) {
        effects.requestRender();
        return;
      }
      if (!live) mount(effects.pool.backendOf() ?? 'cpu', undefined, 'resume');
    },
    restyle(): void {
      if (live) effects.styleCanvas(live.canvas);
      if (pending) effects.styleCanvas(pending.canvas);
    },
    isAttached(): boolean {
      return attached;
    },
    destroy(): void {
      wantsLive = false;
      park();
      effects.pool.forget();
    },
    debug() {
      return { live: live?.canvas, pending: pending?.canvas, attached, wantsLive, timedOutOnce };
    },
  };
}
