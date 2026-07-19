import { describe, expect, it } from 'vitest';
import {
  ACQUIRE_DEBOUNCE_MS,
  createPageSurfaceManager,
  PARKED_RETRY_MS,
  RESTORE_WATCHDOG_MS,
  SWAP_TIMEOUT_MS,
} from './page-surface-manager';
import type { AttachOutcome, LeaseToken, SurfaceBackend } from './gl-context-pool';
import type { ManagerEffects, PoolPort } from './page-surface-manager';

type FakeCanvas = { id: number; disposedGl: boolean; disposedCpu: boolean; removed: boolean; listeners: number };

type LeasePhase = 'reserved' | 'live' | 'releasing';

// A small in-memory stand-in for GlContextPool's token ledger (reserved/live/releasing phases
// only — no tiering/budget policy, that's gl-context-pool.test.ts's job). It exists so the
// manager's cancelReservation-vs-beginRelease+ackReleased branching is exercised against real
// phase transitions instead of a rubber-stamp stub that would hide ordering bugs.
const createFakePool = (options?: { budget?: number; demand?: SurfaceBackend }) => {
  const budget = options?.budget ?? 4;
  const demand = options?.demand ?? 'gl';
  let nextToken = 0;
  const leases = new Map<LeaseToken, LeasePhase>();
  const calls = {
    updateDemand: 0,
    acquireLease: 0,
    ackAttached: [] as [LeaseToken, AttachOutcome][],
    cancelReservation: [] as LeaseToken[],
    beginRelease: [] as LeaseToken[],
    ackReleased: [] as LeaseToken[],
    notePresent: [] as (LeaseToken | undefined)[],
    noteGlFailure: [] as LeaseToken[],
    noteBudgetFallback: 0,
    leave: 0,
    forget: 0,
  };
  const port: PoolPort = {
    updateDemand: () => {
      calls.updateDemand += 1;
      return demand;
    },
    acquireLease: (requested) => {
      calls.acquireLease += 1;
      if (requested !== 'gl' || leases.size >= budget) return { backend: 'cpu' };
      const lastToken = ++nextToken;
      leases.set(lastToken, 'reserved');
      return { backend: 'gl', token: lastToken };
    },
    ackAttached: (leaseToken, actual) => {
      calls.ackAttached.push([leaseToken, actual]);
      if (!leases.has(leaseToken)) return;
      if (actual === 'gl') leases.set(leaseToken, 'live');
      else if (actual === 'cpu') leases.delete(leaseToken);
      else leases.set(leaseToken, 'releasing'); // gl-dead
    },
    cancelReservation: (leaseToken) => {
      calls.cancelReservation.push(leaseToken);
      if (leases.get(leaseToken) === 'reserved') leases.delete(leaseToken);
    },
    beginRelease: (leaseToken) => {
      calls.beginRelease.push(leaseToken);
      if (leases.has(leaseToken)) leases.set(leaseToken, 'releasing');
    },
    ackReleased: (leaseToken) => {
      calls.ackReleased.push(leaseToken);
      leases.delete(leaseToken);
    },
    notePresent: (leaseToken) => {
      calls.notePresent.push(leaseToken);
    },
    noteGlFailure: (incident) => {
      calls.noteGlFailure.push(incident);
    },
    noteBudgetFallback: () => {
      calls.noteBudgetFallback += 1;
    },
    backendOf: () => demand,
    leave: () => {
      calls.leave += 1;
    },
    forget: () => {
      calls.forget += 1;
    },
  };
  return { port, calls, activeCount: () => leases.size };
};

const harness = (overrides?: {
  attach?: (canvas: FakeCanvas, backend: SurfaceBackend) => AttachOutcome | 'cpu-oversized';
  pool?: ReturnType<typeof createFakePool>;
  deferSync?: boolean;
  suspended?: boolean;
}) => {
  let nextId = 0;
  const canvases: FakeCanvas[] = [];
  const presented: (() => void)[] = [];
  const timers: { fn: () => void; ms: number; cancelled: boolean }[] = [];
  const deferred: (() => void)[] = [];
  let attachedCount = 0;
  let suspended = overrides?.suspended ?? false;
  let requestRenderCount = 0;
  const pool = overrides?.pool ?? createFakePool();

  const effects: ManagerEffects<FakeCanvas> = {
    createCanvas: () => {
      const canvas = { id: nextId++, disposedGl: false, disposedCpu: false, removed: false, listeners: 0 };
      canvases.push(canvas);
      return canvas;
    },
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- styling isn't under test here
    styleCanvas: () => {},
    attach: (canvas, backend) => {
      attachedCount += 1;
      return overrides?.attach ? overrides.attach(canvas, backend) : backend;
    },
    detach: () => {
      attachedCount -= 1;
    },
    requestRender: () => {
      requestRenderCount += 1;
    },
    isSuspended: () => suspended,
    onPresented: (listener) => {
      presented.push(listener);
      return () => {
        const index = presented.indexOf(listener);
        if (index !== -1) presented.splice(index, 1);
      };
    },
    addContextListeners: (canvas) => {
      canvas.listeners += 1;
      return () => {
        canvas.listeners -= 1;
      };
    },
    disposeGlContext: (canvas) => {
      canvas.disposedGl = true;
    },
    releaseCpuBacking: (canvas) => {
      canvas.disposedCpu = true;
    },
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- DOM append isn't under test here
    promote: () => {},
    removeNode: (canvas) => {
      canvas.removed = true;
    },
    schedule: (fn, ms) => {
      const timer = { fn, ms, cancelled: false };
      timers.push(timer);
      return () => {
        timer.cancelled = true;
      };
    },
    defer: (fn) => {
      if (overrides?.deferSync) fn();
      else deferred.push(fn);
    },
    pool: pool.port,
  };

  const manager = createPageSurfaceManager(effects);
  const firePresent = () => {
    const due = [...presented];
    presented.length = 0;
    for (const listener of due) listener();
  };
  // Most-recently-scheduled uncancelled timer of this delay wins, and firing consumes it — a
  // stale earlier timer of the same delay must never refire on a later fireTimer call.
  const fireTimer = (ms: number) => {
    const timer = timers.findLast((entry) => entry.ms === ms && !entry.cancelled);
    if (timer) {
      timer.cancelled = true;
      timer.fn();
    }
    return timer;
  };
  const flushDeferred = () => {
    const due = [...deferred];
    deferred.length = 0;
    for (const fn of due) fn();
  };
  const activeTimers = () => timers.filter((entry) => !entry.cancelled).length;
  const setSuspended = (value: boolean) => {
    suspended = value;
  };

  return {
    manager,
    canvases,
    timers,
    pool,
    firePresent,
    fireTimer,
    flushDeferred,
    activeTimers,
    setSuspended,
    requestRenderCount: () => requestRenderCount,
    attachedCount: () => attachedCount,
  };
};

const inView = { inAcquire: true, inRelease: true, isVisible: true };
const outOfView = { inAcquire: false, inRelease: false, isVisible: false };
// overscan 진입: 반경 안이지만 아직 화면에 보이지는 않는다(빠른 스크롤 churn의 원천) — F-A1 디바운스 대상.
const overscan = { inAcquire: true, inRelease: true, isVisible: false };

describe('page-surface-manager', () => {
  it('mounts on acquire, acks the attach via defer, swaps on committed present, releases fully on park', () => {
    const h = harness();
    h.manager.reconcile(inView);
    expect(h.pool.activeCount()).toBe(1); // reserved, not yet acked
    expect(h.pool.calls.ackAttached).toEqual([]);

    h.flushDeferred();
    expect(h.pool.calls.ackAttached).toEqual([[1, 'gl']]);

    h.firePresent();
    expect(h.manager.debug().live).toBe(h.canvases[0]);
    expect(h.pool.calls.notePresent).toEqual([1]);

    h.manager.reconcile(outOfView);
    expect(h.attachedCount()).toBe(0);
    expect(h.canvases[0].disposedGl).toBe(true);
    expect(h.canvases[0].removed).toBe(true);
    expect(h.canvases[0].listeners).toBe(0);
    expect(h.pool.calls.beginRelease).toEqual([1]);

    h.flushDeferred();
    expect(h.pool.calls.ackReleased).toEqual([1]);
    expect(h.pool.activeCount()).toBe(0);
    expect(h.pool.calls.leave).toBe(1);
  });

  it('parks a pending mount before its attach ack flushes without leaking the reservation', () => {
    const h = harness();
    h.manager.reconcile(inView);
    expect(h.pool.activeCount()).toBe(1);

    h.manager.reconcile(outOfView);
    expect(h.attachedCount()).toBe(0);
    expect(h.canvases[0].listeners).toBe(0);
    expect(h.canvases[0].disposedGl).toBe(true);
    expect(h.canvases[0].removed).toBe(true);
    expect(h.pool.calls.cancelReservation).toEqual([1]);
    expect(h.pool.calls.beginRelease).toEqual([]);
    expect(h.pool.activeCount()).toBe(0);

    // the already-queued ackAttached still flushes later — it must no-op against the cancelled token
    h.flushDeferred();
    expect(h.pool.calls.ackAttached).toEqual([[1, 'gl']]);
    expect(h.pool.activeCount()).toBe(0);

    h.firePresent();
    expect(h.manager.debug().live).toBeUndefined();
  });

  it('disposes a cancelled cpu-backed pending as cpu and never touches the gl lease API', () => {
    const h = harness({ attach: () => 'cpu', pool: createFakePool({ demand: 'cpu' }) });
    h.manager.reconcile(inView);
    expect(h.pool.calls.acquireLease).toBe(0); // cpu demand never asks the pool for a lease

    h.manager.reconcile(outOfView);
    expect(h.canvases[0].disposedCpu).toBe(true);
    expect(h.canvases[0].disposedGl).toBe(false);
    expect(h.pool.calls.beginRelease).toEqual([]);
    expect(h.pool.calls.cancelReservation).toEqual([]);
  });

  it('acks a mismatched cpu attach (self-retiring the lease) and keeps the cpu surface live', () => {
    const h = harness({ attach: () => 'cpu' }); // demand defaults to 'gl'
    h.manager.reconcile(inView);
    expect(h.pool.calls.acquireLease).toBe(1);
    expect(h.pool.activeCount()).toBe(1);

    h.flushDeferred();
    expect(h.pool.calls.ackAttached).toEqual([[1, 'cpu']]);
    expect(h.pool.activeCount()).toBe(0); // ackAttached('cpu') self-retires in the real pool too

    h.firePresent();
    expect(h.manager.debug().live).toBe(h.canvases[0]);
    expect(h.canvases.length).toBe(1);
  });

  it('retries gl-dead once on a fresh canvas then forces cpu, acking the dead lease release without renegotiating', () => {
    let attachCalls = 0;
    const h = harness({
      attach: () => {
        attachCalls += 1;
        return attachCalls === 1 ? 'gl-dead' : 'cpu';
      },
      deferSync: true,
    });
    h.manager.reconcile(inView);
    expect(attachCalls).toBe(2);
    expect(h.pool.calls.acquireLease).toBe(1); // only the first (gl) attempt asks the pool
    expect(h.canvases[0].removed).toBe(true);
    expect(h.canvases[0].disposedGl).toBe(true);
    expect(h.pool.calls.ackAttached).toEqual([[1, 'gl-dead']]);
    expect(h.pool.calls.beginRelease).toEqual([1]);
    expect(h.pool.calls.ackReleased).toEqual([1]);
    expect(h.pool.activeCount()).toBe(0);
    expect(h.attachedCount()).toBe(1); // the cpu retry is attached
    expect(h.canvases[1].removed).toBe(false);
  });

  it('discards an uncommitted pending on timeout, keeps the old live, and forces cpu without renegotiating the pool', () => {
    const h = harness();
    h.manager.reconcile(inView);
    h.flushDeferred();
    h.firePresent();
    const firstLive = h.manager.debug().live;
    expect(h.pool.calls.acquireLease).toBe(1);

    // Simulate the pool offering an upgrade with a pre-reserved acquireHint, exactly as a real
    // indirect promotion callback would.
    const preReserved = h.pool.port.acquireLease('gl');
    if (preReserved.backend !== 'gl') throw new Error('test setup invariant broken');
    h.manager.onPoolBackend('gl', preReserved.token);
    expect(h.pool.calls.acquireLease).toBe(2); // the manager itself never called acquireLease
    h.flushDeferred(); // let the attach ack land, as a real microtask would well before the 1s timeout

    const timer = h.fireTimer(SWAP_TIMEOUT_MS);
    expect(timer).toBeDefined();
    h.flushDeferred();

    expect(h.manager.debug().live).toBe(firstLive);
    expect(h.canvases[1].removed).toBe(true);
    expect(h.canvases[1].disposedGl).toBe(true);
    expect(h.pool.calls.beginRelease).toContain(preReserved.token);
    expect(h.pool.calls.ackReleased).toContain(preReserved.token);
    expect(h.pool.calls.noteGlFailure).toEqual([preReserved.token]);
    expect(h.pool.calls.acquireLease).toBe(2); // forced cpu fallback bypasses acquireLease entirely
    expect(h.manager.debug().pending).toBe(h.canvases[2]);
  });

  it('gives up after a second consecutive swap timeout, disposing everything, then retries on re-entry', () => {
    const h = harness({ deferSync: true });
    h.manager.reconcile(inView); // first (gl) attempt — never presented

    const firstTimeout = h.fireTimer(SWAP_TIMEOUT_MS);
    expect(firstTimeout).toBeDefined();
    expect(h.manager.debug().pending).toBe(h.canvases[1]); // forced cpu fallback now pending

    const secondTimeout = h.fireTimer(SWAP_TIMEOUT_MS);
    expect(secondTimeout).toBeDefined();

    expect(h.manager.debug().live).toBeUndefined();
    expect(h.manager.debug().pending).toBeUndefined();
    expect(h.attachedCount()).toBe(0);
    expect(h.activeTimers()).toBe(1); // only the F-B parked-retry timer is armed (self-heal backoff)
    expect(h.manager.debug().wantsLive).toBe(true); // still wanted — re-entry, not normal park
    expect(h.canvases[0].removed).toBe(true);
    expect(h.canvases[1].removed).toBe(true);

    h.manager.reconcile(inView); // acquire re-entry (immediate mount cancels the parked-retry timer)
    expect(h.attachedCount()).toBe(1);
    expect(h.canvases.length).toBe(3);
    // I1 회귀 고정: 재진입 mount가 대기 중 parked-retry 타이머를 취소했으므로 swap 타이머 하나만
    // 남는다. 취소하지 않았다면 고아 parked-retry가 남아 2가 된다.
    expect(h.activeTimers()).toBe(1);
  });

  it('a second swap timeout also sweeps up a restore watchdog armed mid-flight', () => {
    const h = harness({ deferSync: true });
    h.manager.reconcile(inView); // first (gl) attempt — never presented
    h.fireTimer(SWAP_TIMEOUT_MS); // 1st timeout — forced cpu fallback now pending

    h.manager.onContextLost(); // arms a restore watchdog while the cpu fallback is still in flight
    expect(h.activeTimers()).toBe(2); // the fallback's own swap timer + the watchdog

    h.fireTimer(SWAP_TIMEOUT_MS); // 2nd timeout — the fallback also never committed

    expect(h.manager.debug().live).toBeUndefined();
    expect(h.manager.debug().pending).toBeUndefined();
    // the mid-flight watchdog must not survive the failedParked give-up: only the F-B parked-retry
    // timer remains armed (were the watchdog to survive, this would be 2, not 1).
    expect(h.activeTimers()).toBe(1);
  });

  it('arms a restore watchdog on context loss that remounts if restore never arrives', () => {
    const h = harness({ deferSync: true });
    h.manager.reconcile(inView);
    h.firePresent();
    const firstLive = h.manager.debug().live;

    h.manager.onContextLost();
    expect(h.pool.calls.noteGlFailure).toEqual([1]); // the live slot's token is the incident
    expect(h.activeTimers()).toBe(1);

    const fired = h.fireTimer(RESTORE_WATCHDOG_MS);
    expect(fired).toBeDefined();
    expect(h.manager.debug().live).toBe(firstLive); // old live kept until the remount commits
    expect(h.manager.debug().pending).toBeDefined();
    expect(h.manager.debug().pending).not.toBe(firstLive);
  });

  it('cancels the restore watchdog when restored arrives before it fires', () => {
    const h = harness({ deferSync: true });
    h.manager.reconcile(inView);
    h.firePresent();

    h.manager.onContextLost();
    const watchdogTimer = h.timers.find((entry) => entry.ms === RESTORE_WATCHDOG_MS);
    expect(watchdogTimer?.cancelled).toBe(false);

    h.manager.onContextRestored();
    expect(watchdogTimer?.cancelled).toBe(true);
    expect(h.manager.debug().pending).toBeDefined(); // restored triggers its own remount
  });

  it('disposes the previous live surface when a new pending commits', () => {
    const h = harness();
    h.manager.reconcile(inView);
    h.flushDeferred();
    h.firePresent();
    const oldLive = h.manager.debug().live;
    if (!oldLive) throw new Error('test setup invariant broken');

    h.manager.onPoolBackend('cpu');
    h.flushDeferred();
    h.firePresent();
    h.flushDeferred(); // flush the old live's deferred ackReleased, queued during finishSwap's dispose

    expect(oldLive.removed).toBe(true);
    expect(oldLive.disposedGl).toBe(true);
    expect(h.manager.debug().live).not.toBe(oldLive);
    expect(h.pool.calls.beginRelease).toContain(1);
    expect(h.pool.calls.ackReleased).toContain(1);
  });

  it('converges to a single live canvas under worst-case synchronous defer reentry', () => {
    const h = harness({ deferSync: true });
    h.manager.reconcile(inView);
    h.manager.onPoolBackend('cpu');
    h.manager.onPoolBackend('gl');
    h.firePresent();

    const debug = h.manager.debug();
    expect(debug.pending).toBeUndefined();
    expect(debug.live).toBeDefined();
    expect(h.attachedCount()).toBe(1);
    const alive = h.canvases.filter((canvas) => !canvas.removed);
    expect(alive).toEqual([debug.live]);
  });

  it('R-N1: a suspended swap timeout reschedules without disposing the pending or counting a gl failure', () => {
    const h = harness({ suspended: true });
    h.manager.reconcile(inView);
    h.flushDeferred();
    const pendingCanvas = h.manager.debug().pending;
    expect(pendingCanvas).toBe(h.canvases[0]);
    expect(h.activeTimers()).toBe(1);

    const timer = h.fireTimer(SWAP_TIMEOUT_MS);
    expect(timer).toBeDefined();
    h.flushDeferred();

    // suspended: pending survives intact, no forced cpu fallback, no failure counted, no escalation
    expect(h.manager.debug().pending).toBe(pendingCanvas);
    expect(h.manager.debug().live).toBeUndefined();
    expect(h.manager.debug().timedOutOnce).toBe(false);
    expect(h.pool.calls.noteGlFailure).toEqual([]);
    expect(h.canvases.length).toBe(1); // no cpu fallback canvas was created
    expect(h.canvases[0].removed).toBe(false);
    expect(h.attachedCount()).toBe(1);
    expect(h.activeTimers()).toBe(1); // a fresh swap timer was rescheduled

    // becoming visible then presenting commits the untouched gl pending
    h.setSuspended(false);
    h.firePresent();
    expect(h.manager.debug().live).toBe(pendingCanvas);
    expect(h.manager.debug().pending).toBeUndefined();
    expect(h.pool.calls.notePresent).toEqual([1]); // gl lease token committed
  });

  it('R-N2: a suspended restore watchdog reschedules instead of remounting until visible', () => {
    const h = harness({ deferSync: true, suspended: true });
    h.manager.reconcile(inView);
    h.firePresent();
    const firstLive = h.manager.debug().live;
    expect(firstLive).toBeDefined();

    h.manager.onContextLost();
    expect(h.activeTimers()).toBe(1); // watchdog armed

    const fired = h.fireTimer(RESTORE_WATCHDOG_MS);
    expect(fired).toBeDefined();
    // suspended: no remount, old live kept, watchdog rescheduled
    expect(h.manager.debug().pending).toBeUndefined();
    expect(h.manager.debug().live).toBe(firstLive);
    expect(h.activeTimers()).toBe(1);

    // once visible the rescheduled watchdog fire remounts
    h.setSuspended(false);
    const refired = h.fireTimer(RESTORE_WATCHDOG_MS);
    expect(refired).toBeDefined();
    expect(h.manager.debug().pending).toBeDefined();
    expect(h.manager.debug().live).toBe(firstLive); // old live kept until the remount commits
  });

  it('R-N3: resume() remounts a failedParked page', () => {
    const h = harness({ deferSync: true });
    h.manager.reconcile(inView); // gl pending, never presented
    h.fireTimer(SWAP_TIMEOUT_MS); // 1st timeout — forced cpu fallback
    h.fireTimer(SWAP_TIMEOUT_MS); // 2nd timeout — failedParked

    expect(h.manager.debug().live).toBeUndefined();
    expect(h.manager.debug().pending).toBeUndefined();
    expect(h.manager.debug().wantsLive).toBe(true);

    const canvasesBefore = h.canvases.length;
    h.manager.resume();
    expect(h.canvases.length).toBeGreaterThan(canvasesBefore);
    expect(h.manager.debug().pending).toBeDefined();
  });

  it('R-N4: resume() no-ops without demand, nudges a render for a live pending, and no-ops when settled', () => {
    const h = harness({ deferSync: true });

    // wantsLive=false (no reconcile yet) — full no-op
    h.manager.resume();
    expect(h.canvases.length).toBe(0);
    expect(h.requestRenderCount()).toBe(0);

    // pending in flight — nudge a render, do not mount another canvas
    h.manager.reconcile(inView);
    const rendersAfterMount = h.requestRenderCount();
    const canvasesAfterMount = h.canvases.length;
    h.manager.resume();
    expect(h.requestRenderCount()).toBe(rendersAfterMount + 1);
    expect(h.canvases.length).toBe(canvasesAfterMount);

    // settled live, no pending — no-op
    h.firePresent();
    const rendersAfterPresent = h.requestRenderCount();
    h.manager.resume();
    expect(h.requestRenderCount()).toBe(rendersAfterPresent);
    expect(h.manager.debug().pending).toBeUndefined();
  });

  it('R-P1: overscan(가시 아님) 진입은 즉시 마운트/updateDemand 없이 디바운스하고, ACQUIRE_DEBOUNCE_MS 경과 후에 마운트한다', () => {
    const h = harness();
    h.manager.reconcile(overscan);
    // 디바운스 중: 캔버스도, 풀 수요도, lease도 없다(반경에 머무름을 증명하기 전).
    expect(h.canvases.length).toBe(0);
    expect(h.pool.calls.updateDemand).toBe(0);
    expect(h.pool.calls.acquireLease).toBe(0);
    expect(h.manager.debug().pending).toBeUndefined();
    expect(h.activeTimers()).toBe(1); // 디바운스 타이머 하나

    const fired = h.fireTimer(ACQUIRE_DEBOUNCE_MS);
    expect(fired).toBeDefined();
    // 발화 후에야 updateDemand 1회 + 마운트 1개.
    expect(h.pool.calls.updateDemand).toBe(1);
    expect(h.canvases.length).toBe(1);
    expect(h.manager.debug().pending).toBe(h.canvases[0]);
  });

  it('R-P2: overscan 진입 후 디바운스 만료 전 park되면 디바운스가 취소돼 마운트도 풀 수요/lease도 발생하지 않는다(churn kill)', () => {
    const h = harness();
    h.manager.reconcile(overscan);
    expect(h.activeTimers()).toBe(1);

    h.manager.reconcile(outOfView); // 디바운스 만료 전 반경 이탈 → park
    expect(h.activeTimers()).toBe(0); // 디바운스 취소됨
    expect(h.canvases.length).toBe(0);
    expect(h.pool.calls.updateDemand).toBe(0);
    expect(h.pool.calls.acquireLease).toBe(0);

    // 취소된 디바운스 타이머를 발화시켜도 마운트되지 않는다(transit-only churn을 원천 차단).
    h.fireTimer(ACQUIRE_DEBOUNCE_MS);
    expect(h.canvases.length).toBe(0);
    expect(h.pool.calls.updateDemand).toBe(0);
  });

  it('R-P3: 디바운스 중 가시 진입(isVisible=true)이 오면 디바운스를 취소하고 즉시 1회만 마운트한다', () => {
    const h = harness();
    h.manager.reconcile(overscan);
    expect(h.canvases.length).toBe(0);

    h.manager.reconcile(inView); // 가시 진입 → 지연 없이 즉시 마운트
    expect(h.canvases.length).toBe(1);
    expect(h.pool.calls.updateDemand).toBe(1);
    expect(h.manager.debug().pending).toBe(h.canvases[0]);

    // 취소된 디바운스가 뒤늦게 발화해도 두 번째 마운트는 없다(더블 마운트 금지).
    h.fireTimer(ACQUIRE_DEBOUNCE_MS);
    expect(h.canvases.length).toBe(1);
  });

  it('R-P4: failedParked 재시도는 2000→5000→10000 백오프로 재마운트하고, 커밋 성공 시 백오프가 리셋된다', () => {
    const h = harness({ deferSync: true });
    h.manager.reconcile(inView); // gl pending, never presented
    h.fireTimer(SWAP_TIMEOUT_MS); // 1st timeout — forced cpu fallback
    h.fireTimer(SWAP_TIMEOUT_MS); // 2nd timeout — failedParked, retry armed at PARKED_RETRY_MS[0]
    expect(h.manager.debug().live).toBeUndefined();
    expect(h.manager.debug().pending).toBeUndefined();

    // 재시도 #1: 2000ms
    let before = h.canvases.length;
    expect(h.fireTimer(PARKED_RETRY_MS[0])).toBeDefined();
    expect(h.canvases.length).toBeGreaterThan(before);
    expect(h.manager.debug().pending).toBeDefined();

    // 다시 failedParked로 몰면 다음 재시도는 5000ms
    h.fireTimer(SWAP_TIMEOUT_MS);
    h.fireTimer(SWAP_TIMEOUT_MS);
    before = h.canvases.length;
    expect(h.fireTimer(PARKED_RETRY_MS[1])).toBeDefined();
    expect(h.canvases.length).toBeGreaterThan(before);

    // 세 번째 재시도는 10000ms(cap)
    h.fireTimer(SWAP_TIMEOUT_MS);
    h.fireTimer(SWAP_TIMEOUT_MS);
    before = h.canvases.length;
    expect(h.fireTimer(PARKED_RETRY_MS[2])).toBeDefined();
    expect(h.canvases.length).toBeGreaterThan(before);

    // 커밋 성공 → 백오프 리셋. 새 pending을 다시 2회 타임아웃시키면 재시도가 2000ms로 돌아온다.
    h.firePresent(); // 현재 pending 커밋 → finishSwap이 백오프 리셋
    expect(h.manager.debug().live).toBeDefined();

    h.manager.onPoolBackend('cpu'); // 백엔드 전환으로 새 pending 생성
    h.fireTimer(SWAP_TIMEOUT_MS);
    h.fireTimer(SWAP_TIMEOUT_MS); // 다시 failedParked — 리셋됐다면 재시도는 다시 2000ms
    before = h.canvases.length;
    expect(h.fireTimer(PARKED_RETRY_MS[0])).toBeDefined(); // 5000이 아니라 2000(리셋 확인)
    expect(h.canvases.length).toBeGreaterThan(before);
  });

  it('R-P5: park는 재시도 타이머를 취소하고, 숨김 중 재시도 발화는 마운트 없이 같은 지연으로 재예약한다', () => {
    // park가 재시도 타이머를 취소한다
    const h = harness({ deferSync: true });
    h.manager.reconcile(inView);
    h.fireTimer(SWAP_TIMEOUT_MS);
    h.fireTimer(SWAP_TIMEOUT_MS); // failedParked, retry armed
    expect(h.activeTimers()).toBe(1);
    h.manager.reconcile(outOfView); // park
    expect(h.activeTimers()).toBe(0); // 재시도 타이머 취소됨

    // 숨김 중 재시도 발화는 재마운트 없이 같은 지연으로 재예약한다
    const s = harness({ deferSync: true });
    s.manager.reconcile(inView);
    s.fireTimer(SWAP_TIMEOUT_MS);
    s.fireTimer(SWAP_TIMEOUT_MS); // failedParked, retry armed
    const before = s.canvases.length;
    s.setSuspended(true);
    expect(s.fireTimer(PARKED_RETRY_MS[0])).toBeDefined();
    expect(s.canvases.length).toBe(before); // 숨김 중 — 재마운트 없음
    expect(s.activeTimers()).toBe(1); // 같은 지연으로 재예약됨

    // 복귀 후 재시도 발화는 재마운트한다
    s.setSuspended(false);
    expect(s.fireTimer(PARKED_RETRY_MS[0])).toBeDefined();
    expect(s.canvases.length).toBeGreaterThan(before);
    expect(s.manager.debug().pending).toBeDefined();
  });

  it('R-P6: 재진입 mount는 대기 중 parked-retry 타이머를 취소해 고아를 누적하지 않고, 재실패 시 백오프는 다음 단계로 진행한다', () => {
    const h = harness({ deferSync: true });
    h.manager.reconcile(inView); // gl pending
    h.fireTimer(SWAP_TIMEOUT_MS); // 1차 타임아웃 — 강제 cpu 폴백
    h.fireTimer(SWAP_TIMEOUT_MS); // 2차 타임아웃 — failedParked, parked-retry 무장(index→1, 2000)
    expect(h.activeTimers()).toBe(1); // parked-retry 하나

    // 재진입 mount(가시): 대기 중 parked-retry를 반드시 취소해야 한다(고아 방지).
    h.manager.reconcile(inView);
    // swap 타이머 하나만 남는다 — 취소하지 않았다면 고아 parked-retry가 남아 2가 된다.
    expect(h.activeTimers()).toBe(1);

    // 이 재진입 mount를 다시 이중 타임아웃시켜 failedParked로 몬다.
    h.fireTimer(SWAP_TIMEOUT_MS); // 1차
    h.fireTimer(SWAP_TIMEOUT_MS); // 2차 → scheduleParkedRetry
    // scheduleParkedRetry 시점: 활성 parked-retry는 정확히 1개(고아 누적 없음).
    expect(h.activeTimers()).toBe(1);

    // 첫 재시도(2000)는 취소돼 활성 타이머가 없고, 고아 발화로 인한 잉여 mount도 없다.
    const canvasesBeforeRetry = h.canvases.length;
    expect(h.fireTimer(PARKED_RETRY_MS[0])).toBeUndefined(); // 2000짜리 활성 타이머 없음
    expect(h.canvases.length).toBe(canvasesBeforeRetry); // 잉여 mount 없음

    // 백오프는 다음 단계(5000)로 진행 — 정상 재시도는 5000에서 발화해 재마운트한다.
    expect(h.fireTimer(PARKED_RETRY_MS[1])).toBeDefined();
    expect(h.canvases.length).toBeGreaterThan(canvasesBeforeRetry);
  });

  it('destroy fully disposes local resources and forgets the pool entry', () => {
    const h = harness();
    h.manager.reconcile(inView);
    h.flushDeferred();
    h.firePresent();

    h.manager.destroy();
    expect(h.manager.debug().live).toBeUndefined();
    expect(h.attachedCount()).toBe(0);
    expect(h.pool.calls.leave).toBe(1);
    expect(h.pool.calls.forget).toBe(1);
  });
});
