import { describe, expect, it } from 'vitest';
import { createPageSurfaceManager, SWAP_TIMEOUT_MS } from './page-surface-manager';
import type { AttachResult, ManagerEffects } from './page-surface-manager';

type FakeCanvas = { id: number; disposedCpu: boolean; removed: boolean; listeners: number };

const harness = (overrides?: { attach?: (canvas: FakeCanvas) => AttachResult; suspended?: boolean }) => {
  let nextId = 0;
  const canvases: FakeCanvas[] = [];
  const presented: (() => void)[] = [];
  const timers: { fn: () => void; ms: number; cancelled: boolean }[] = [];
  let attachedCount = 0;
  let suspended = overrides?.suspended ?? false;
  let requestRenderCount = 0;

  const effects: ManagerEffects<FakeCanvas> = {
    createCanvas: () => {
      const canvas = { id: nextId++, disposedCpu: false, removed: false, listeners: 0 };
      canvases.push(canvas);
      return canvas;
    },
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- styling isn't under test here
    styleCanvas: () => {},
    attach: (canvas) => {
      attachedCount += 1;
      return overrides?.attach ? overrides.attach(canvas) : 'cpu';
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
  const activeTimers = () => timers.filter((entry) => !entry.cancelled).length;
  const setSuspended = (value: boolean) => {
    suspended = value;
  };

  return {
    manager,
    canvases,
    firePresent,
    fireTimer,
    activeTimers,
    setSuspended,
    requestRenderCount: () => requestRenderCount,
    attachedCount: () => attachedCount,
  };
};

const inView = { inAcquire: true, inRelease: true, isVisible: true };
const outOfView = { inAcquire: false, inRelease: false, isVisible: false };

describe('page-surface-manager', () => {
  it('mounts on acquire, swaps on committed present, releases fully on park', () => {
    const h = harness();
    h.manager.reconcile(inView);
    expect(h.manager.debug().pending).toBe(h.canvases[0]);
    expect(h.attachedCount()).toBe(1);

    h.firePresent();
    expect(h.manager.debug().live).toBe(h.canvases[0]);
    expect(h.manager.debug().pending).toBeUndefined();

    h.manager.reconcile(outOfView);
    expect(h.attachedCount()).toBe(0);
    expect(h.canvases[0].disposedCpu).toBe(true);
    expect(h.canvases[0].removed).toBe(true);
    expect(h.canvases[0].listeners).toBe(0);
    expect(h.manager.debug().live).toBeUndefined();
    expect(h.manager.debug().wantsLive).toBe(false);
  });

  it('parks a pending mount before it presents, disposing its cpu backing', () => {
    const h = harness();
    h.manager.reconcile(inView);
    expect(h.attachedCount()).toBe(1);

    h.manager.reconcile(outOfView);
    expect(h.attachedCount()).toBe(0);
    expect(h.canvases[0].listeners).toBe(0);
    expect(h.canvases[0].disposedCpu).toBe(true);
    expect(h.canvases[0].removed).toBe(true);

    // a present firing against the cancelled pending must not resurrect it
    h.firePresent();
    expect(h.manager.debug().live).toBeUndefined();
  });

  it('disposes an oversized cpu attach on mount, converging to failedParked', () => {
    const h = harness({ attach: () => 'cpu-oversized' });
    h.manager.reconcile(inView);

    expect(h.canvases[0].disposedCpu).toBe(true);
    expect(h.canvases[0].removed).toBe(true);
    expect(h.canvases[0].listeners).toBe(0);
    expect(h.manager.debug().live).toBeUndefined();
    expect(h.manager.debug().pending).toBeUndefined();
    expect(h.manager.debug().wantsLive).toBe(true); // still wanted — re-entry recovers
    expect(h.attachedCount()).toBe(0);
  });

  it('discards an uncommitted pending on the first swap timeout and remounts once', () => {
    const h = harness();
    h.manager.reconcile(inView);
    const firstPending = h.manager.debug().pending;
    expect(firstPending).toBe(h.canvases[0]);

    const timer = h.fireTimer(SWAP_TIMEOUT_MS);
    expect(timer).toBeDefined();

    expect(h.canvases[0].removed).toBe(true);
    expect(h.manager.debug().timedOutOnce).toBe(true);
    expect(h.manager.debug().pending).toBe(h.canvases[1]); // remounted once
    expect(h.attachedCount()).toBe(1);
    expect(h.canvases.length).toBe(2);
  });

  it('gives up after a second consecutive swap timeout, disposing everything, then retries on re-entry', () => {
    const h = harness();
    h.manager.reconcile(inView); // first attempt — never presented

    const firstTimeout = h.fireTimer(SWAP_TIMEOUT_MS);
    expect(firstTimeout).toBeDefined();
    expect(h.manager.debug().pending).toBe(h.canvases[1]); // remount now pending

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

  it('disposes the discarded pending when the retried mount commits', () => {
    const h = harness();
    h.manager.reconcile(inView); // first pending, never presented
    const firstPending = h.manager.debug().pending;
    if (!firstPending) throw new Error('test setup invariant broken');

    h.fireTimer(SWAP_TIMEOUT_MS); // discards the first pending, remounts once
    expect(firstPending.removed).toBe(true);
    expect(firstPending.disposedCpu).toBe(true);

    h.firePresent(); // the retry commits
    expect(h.manager.debug().live).toBe(h.canvases[1]);
    expect(h.manager.debug().live).not.toBe(firstPending);
    expect(h.manager.debug().pending).toBeUndefined();
  });

  it('converges to a single live canvas across park and re-acquire', () => {
    const h = harness();
    h.manager.reconcile(inView); // pending c0
    h.manager.reconcile(outOfView); // park disposes c0
    h.manager.reconcile(inView); // pending c1
    h.firePresent(); // c1 commits

    const debug = h.manager.debug();
    expect(debug.pending).toBeUndefined();
    expect(debug.live).toBe(h.canvases[1]);
    expect(h.attachedCount()).toBe(1);
    const alive = h.canvases.filter((canvas) => !canvas.removed);
    expect(alive).toEqual([debug.live]);
  });

  it('R-N1: a suspended swap timeout reschedules without disposing the pending', () => {
    const h = harness({ suspended: true });
    h.manager.reconcile(inView);
    const pendingCanvas = h.manager.debug().pending;
    expect(pendingCanvas).toBe(h.canvases[0]);
    expect(h.activeTimers()).toBe(1);

    const timer = h.fireTimer(SWAP_TIMEOUT_MS);
    expect(timer).toBeDefined();

    // suspended: pending survives intact, no remount, no escalation
    expect(h.manager.debug().pending).toBe(pendingCanvas);
    expect(h.manager.debug().live).toBeUndefined();
    expect(h.manager.debug().timedOutOnce).toBe(false);
    expect(h.canvases.length).toBe(1); // no remount canvas was created
    expect(h.canvases[0].removed).toBe(false);
    expect(h.attachedCount()).toBe(1);
    expect(h.activeTimers()).toBe(1); // a fresh swap timer was rescheduled

    // becoming visible then presenting commits the untouched pending
    h.setSuspended(false);
    h.firePresent();
    expect(h.manager.debug().live).toBe(pendingCanvas);
    expect(h.manager.debug().pending).toBeUndefined();
  });

  it('R-N3: resume() remounts a failedParked page', () => {
    const h = harness();
    h.manager.reconcile(inView); // pending, never presented
    h.fireTimer(SWAP_TIMEOUT_MS); // 1st timeout — remount
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
    const h = harness();

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

  it('destroy fully disposes local resources', () => {
    const h = harness();
    h.manager.reconcile(inView);
    h.firePresent();

    h.manager.destroy();
    expect(h.manager.debug().live).toBeUndefined();
    expect(h.manager.debug().wantsLive).toBe(false);
    expect(h.attachedCount()).toBe(0);
  });
});
