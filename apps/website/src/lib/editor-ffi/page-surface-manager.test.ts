import { describe, expect, it } from 'vitest';
import { createPageSurfaceManager, RESTORE_WATCHDOG_MS, SWAP_TIMEOUT_MS } from './page-surface-manager';
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
    updateDemand: () => demand,
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
}) => {
  let nextId = 0;
  const canvases: FakeCanvas[] = [];
  const presented: (() => void)[] = [];
  const timers: { fn: () => void; ms: number; cancelled: boolean }[] = [];
  const deferred: (() => void)[] = [];
  let attachedCount = 0;
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
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- render scheduling isn't under test here
    requestRender: () => {},
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

  return { manager, canvases, timers, pool, firePresent, fireTimer, flushDeferred, activeTimers, attachedCount: () => attachedCount };
};

const inView = { inAcquire: true, inRelease: true, isVisible: true };
const outOfView = { inAcquire: false, inRelease: false, isVisible: false };

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
    expect(h.activeTimers()).toBe(0);
    expect(h.manager.debug().wantsLive).toBe(true); // still wanted — re-entry, not normal park
    expect(h.canvases[0].removed).toBe(true);
    expect(h.canvases[1].removed).toBe(true);

    h.manager.reconcile(inView); // acquire re-entry
    expect(h.attachedCount()).toBe(1);
    expect(h.canvases.length).toBe(3);
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
    expect(h.activeTimers()).toBe(0); // the mid-flight watchdog must not survive the failedParked give-up
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
