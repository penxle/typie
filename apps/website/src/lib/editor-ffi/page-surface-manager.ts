// CPU가 유일한 present 백엔드다 — attach 결과는 'cpu'(정상) 또는 'cpu-oversized'(예산 초과로
// 백킹 할당 거부)뿐이다.
export type AttachResult = 'cpu' | 'cpu-oversized';

export type ManagerEffects<C> = {
  createCanvas: () => C;
  styleCanvas: (canvas: C) => void;
  attach: (canvas: C) => AttachResult;
  detach: () => void;
  requestRender: () => void;
  // 숨김 탭(rAF 정지)에서는 present가 커밋될 수 없다 — 클록 게이트가 이를 조회한다.
  isSuspended: () => boolean;
  onPresented: (listener: () => void) => () => void;
  addContextListeners: (canvas: C, isCurrent: () => boolean) => () => void;
  releaseCpuBacking: (canvas: C) => void;
  promote: (next: C, previous: C | undefined) => void;
  removeNode: (canvas: C) => void;
  schedule: (fn: () => void, ms: number) => () => void;
};

export const SWAP_TIMEOUT_MS = 1000;

export type VisibilityState = { inAcquire: boolean; inRelease: boolean; isVisible: boolean };

type Slot<C> = {
  canvas: C;
  listeners: () => void;
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
  // reconcile이 마지막으로 결정한 "이 페이지는 라이브여야 한다" — live/pending이 둘 다 비어도
  // (failedParked) 이 값이 true로 남아 있으면 재진입(acquire 재진입/복귀 resume)이 유효하다.
  let wantsLive = false;

  const disposeSlot = (slot: Slot<C>) => {
    slot.listeners();
    effects.releaseCpuBacking(slot.canvas);
    effects.removeNode(slot.canvas);
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
    cancelPending();
    detachIfAttached();
    if (live) {
      disposeSlot(live);
      live = undefined;
    }
    timedOutOnce = false;
  };

  // 스와프 확정은 committed present에서만 일어난다. 타임아웃은 승격이 아니라
  // "pending 폐기 + 구 live 유지 + 1회 재마운트"다.
  const finishSwap = (myEpoch: number) => {
    if (myEpoch !== epoch || !pending) return;
    pendingCleanup?.();
    pendingCleanup = undefined;
    const next = pending;
    pending = undefined;
    if (live) disposeSlot(live);
    effects.promote(next.canvas, live?.canvas);
    live = next;
    timedOutOnce = false;
  };

  const onSwapTimeout = (myEpoch: number) => {
    if (myEpoch !== epoch || !pending) return;
    // 숨김 중엔 present가 커밋될 수 없다 — 실패로 계상하지 말고(강등 없이) 같은 에포크로
    // 재예약만 한다. pendingCleanup은 factory 스코프의 cancelSwapTimeout을 읽으므로 이 새
    // 타이머를 취소한다.
    if (effects.isSuspended()) {
      cancelSwapTimeout = effects.schedule(() => onSwapTimeout(myEpoch), SWAP_TIMEOUT_MS);
      return;
    }
    cancelPending();
    detachIfAttached();
    if (timedOutOnce) {
      // 2차 타임아웃 — 재마운트마저 커밋되지 못했다. 구 live까지 전부 처분하고 안정된
      // failedParked로 수렴한다. wantsLive는 건드리지 않아 재진입(acquire/resume)이 살아 있다.
      timedOutOnce = false;
      if (live) {
        disposeSlot(live);
        live = undefined;
      }
      return;
    }
    timedOutOnce = true;
    mount();
  };

  const mount = () => {
    cancelPending();
    detachIfAttached();
    const myEpoch = epoch;

    const canvas = effects.createCanvas();
    effects.styleCanvas(canvas);
    const listeners = effects.addContextListeners(canvas, () => canvas === live?.canvas || canvas === pending?.canvas);
    const actual = effects.attach(canvas);

    if (actual === 'cpu-oversized') {
      // 대형 cpu 할당으로 도망치지 않는다 — 즉시 처분하고 failedParked 상당으로 수렴한다
      // (재시도는 radius 재진입에만 맡긴다).
      listeners();
      effects.detach();
      effects.releaseCpuBacking(canvas);
      effects.removeNode(canvas);
      if (live) {
        // 이 시도 직전 위쪽 detachIfAttached()가 구 live의 wasm 바인딩을 걷어갔다(단일 surface
        // 슬롯 FFI 특성상 attach 시도 자체가 교체를 함의한다). 구 캔버스는 더 이상 유효한
        // 바인딩이 없는 빈 껍데기이므로 시각적 잔상으로도 유지할 수 없다 — 함께 처분한다.
        disposeSlot(live);
        live = undefined;
      }
      return;
    }

    attached = true;
    const slot: Slot<C> = { canvas, listeners };
    pending = slot;
    const offPresented = effects.onPresented(() => finishSwap(myEpoch));
    cancelSwapTimeout = effects.schedule(() => onSwapTimeout(myEpoch), SWAP_TIMEOUT_MS);
    pendingCleanup = () => {
      offPresented();
      cancelSwapTimeout?.();
      cancelSwapTimeout = undefined;
    };
    effects.requestRender();
  };

  return {
    reconcile(state: VisibilityState): void {
      const shouldBeLive = live || pending ? state.inRelease : state.inAcquire;
      wantsLive = shouldBeLive;
      if (!shouldBeLive) {
        park();
        return;
      }
      if (!live && !pending) mount();
    },
    // 가시성 복귀 시 손: failedParked(live/pending 없음)를 치유하고, 숨김 중 정체된 pending은
    // 렌더를 한 번 재촉해 커밋을 유도한다. 원치 않는(park된) 페이지는 건드리지 않는다.
    resume(): void {
      if (!wantsLive) return;
      if (pending) {
        effects.requestRender();
        return;
      }
      if (!live) mount();
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
    },
    debug() {
      return { live: live?.canvas, pending: pending?.canvas, attached, wantsLive, timedOutOnce };
    },
  };
}
