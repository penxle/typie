import { describe, expect, it } from 'vitest';
import { createSurfaceDriver } from './surface-driver';
import type { AttachResult, SurfaceDriverEffects } from './surface-driver';

type FakeCanvas = { id: number; pixels: string; disposed: boolean; removed: boolean; listeners: number };

function harness(attachResult: AttachResult | AttachResult[] = 'cpu') {
  let nextId = 0;
  let attached = false;
  let recoverCount = 0;
  let detachCount = 0;
  let promoteCount = 0;
  let replacementFailureCount = 0;
  let maxLiveCanvasCount = 0;
  const canvases: FakeCanvas[] = [];
  const lifecycle: string[] = [];
  const effects: SurfaceDriverEffects<FakeCanvas> = {
    createCanvas: () => {
      const canvas = { id: nextId++, pixels: '', disposed: false, removed: false, listeners: 0 };
      canvases.push(canvas);
      maxLiveCanvasCount = Math.max(maxLiveCanvasCount, canvases.filter((candidate) => !candidate.disposed).length);
      return canvas;
    },
    styleCanvas: () => {
      // Canvas styling is outside this lifecycle fixture.
    },
    attach: () => {
      const result = Array.isArray(attachResult) ? (attachResult.shift() ?? 'cpu') : attachResult;
      attached = result !== 'none';
      return result;
    },
    detach: () => {
      attached = false;
      detachCount += 1;
    },
    recover: () => {
      recoverCount += 1;
    },
    addContextListeners: (canvas) => {
      canvas.listeners += 1;
      return () => {
        canvas.listeners -= 1;
      };
    },
    releaseCpuBacking: (canvas) => {
      lifecycle.push(`release:${canvas.id}`);
      canvas.pixels = '';
      canvas.disposed = true;
    },
    promote: (canvas, previous) => {
      lifecycle.push(`promote:${canvas.id}:${previous?.id ?? 'none'}`);
      promoteCount += 1;
      canvas.removed = false;
    },
    removeNode: (canvas) => {
      lifecycle.push(`remove:${canvas.id}`);
      canvas.removed = true;
    },
    replacementFailed: () => {
      replacementFailureCount += 1;
    },
  };
  const driver = createSurfaceDriver(effects);
  return {
    driver,
    canvases,
    isAttached: () => attached,
    recoverCount: () => recoverCount,
    detachCount: () => detachCount,
    promoteCount: () => promoteCount,
    replacementFailureCount: () => replacementFailureCount,
    maxLiveCanvasCount: () => maxLiveCanvasCount,
    lifecycle,
  };
}

describe('surface-driver', () => {
  it('does not expose a target canvas before Editor publishes its delivered frame', () => {
    const h = harness();
    expect(h.driver.hasSurface()).toBe(false);

    h.driver.setActive(true);

    expect(h.driver.debug().target).toBe(h.canvases[0]);
    expect(h.driver.debug().displayed).toBeUndefined();
    expect(h.driver.hasSurface()).toBe(true);

    h.driver.syncPublished(h.canvases[0]);
    expect(h.driver.debug().displayed).toBe(h.canvases[0]);
  });

  it('ignores stale delivery for a canvas that is not the current target', () => {
    const h = harness();
    h.driver.setActive(true);
    const stale = { id: 99, pixels: '', disposed: false, removed: false, listeners: 0 };

    h.driver.syncPublished(stale);

    expect(h.driver.debug().displayed).toBeUndefined();
  });

  it('has no timeout or retry scheduler and retries only on a real resume signal', () => {
    const h = harness('cpu-oversized');
    h.driver.setActive(true);
    expect(h.canvases).toHaveLength(1);
    expect(h.driver.debug().target).toBeUndefined();
    expect(h.isAttached()).toBe(false);
    expect(h.detachCount()).toBe(1);

    h.driver.resume();
    expect(h.canvases).toHaveLength(2);
    expect(h.replacementFailureCount()).toBe(1);
  });

  it('starts a fresh replacement attempt after the surface is parked', () => {
    const h = harness(['cpu-oversized', 'cpu-oversized']);
    h.driver.setActive(true);

    h.driver.setActive(false);
    h.driver.setActive(true);

    expect(h.replacementFailureCount()).toBe(0);
  });

  it('allows another replacement after a target publishes successfully', () => {
    const h = harness(['cpu-oversized', 'cpu', 'cpu-oversized', 'cpu-oversized']);
    h.driver.setActive(true);
    expect(h.replacementFailureCount()).toBe(0);

    h.driver.resume();
    h.driver.syncPublished(h.canvases[1]);
    h.driver.replace();
    expect(h.replacementFailureCount()).toBe(0);

    h.driver.resume();
    expect(h.replacementFailureCount()).toBe(1);
  });

  it('forwards recovery for the current target without creating another canvas', () => {
    const h = harness();
    h.driver.setActive(true);
    h.driver.resume();

    expect(h.canvases).toHaveLength(1);
    expect(h.recoverCount()).toBe(1);
  });

  it('re-promotes the same canvas after an in-place backing replacement publishes', () => {
    const h = harness();
    h.driver.setActive(true);
    h.driver.syncPublished(h.canvases[0]);
    h.canvases[0].removed = true;

    h.driver.syncPublished(h.canvases[0]);

    expect(h.canvases[0].removed).toBe(false);
    expect(h.promoteCount()).toBe(2);
  });

  it('keeps the published canvas visible until its replacement is published', () => {
    const h = harness();
    h.driver.setActive(true);
    h.canvases[0].pixels = 'published pixels';
    h.driver.syncPublished(h.canvases[0]);

    h.driver.replace();

    expect(h.canvases).toHaveLength(2);
    expect(h.canvases[0]).toMatchObject({ pixels: 'published pixels', disposed: false, removed: false, listeners: 0 });
    expect(h.driver.debug()).toMatchObject({ target: h.canvases[1], displayed: h.canvases[0] });
    expect(h.detachCount()).toBe(0);

    h.driver.syncPublished(h.canvases[1]);
    expect(h.canvases[0]).toMatchObject({ disposed: true, removed: true });
    expect(h.driver.debug().displayed).toBe(h.canvases[1]);
  });

  it('keeps repeated missing-target attempts outside replacement-failure escalation', () => {
    const h = harness(['cpu', 'none', 'none']);
    h.driver.setActive(true);
    h.canvases[0].pixels = 'published pixels';
    h.driver.syncPublished(h.canvases[0]);

    h.driver.replace();
    h.driver.resume();

    expect(h.driver.debug()).toMatchObject({ target: undefined, displayed: h.canvases[0] });
    expect(h.canvases[0]).toMatchObject({ pixels: 'published pixels', disposed: false, removed: false });
    expect(h.canvases[1]).toMatchObject({ pixels: '', disposed: true, removed: true });
    expect(h.canvases[2]).toMatchObject({ pixels: '', disposed: true, removed: true });
    expect(h.isAttached()).toBe(false);
    expect(h.replacementFailureCount()).toBe(0);

    h.driver.destroy();
    expect(h.detachCount()).toBe(0);
  });

  it('promotes a proven replacement before releasing the displayed backing', () => {
    const h = harness();
    h.driver.setActive(true);
    h.driver.syncPublished(h.canvases[0]);
    h.driver.replace();
    h.lifecycle.length = 0;

    h.driver.syncPublished(h.canvases[1]);

    expect(h.lifecycle).toEqual(['promote:1:0', 'release:0', 'remove:0']);
  });

  it('converges repeated replacement requests to the latest candidate while retaining only the displayed frame', () => {
    const h = harness();
    h.driver.setActive(true);
    h.driver.syncPublished(h.canvases[0]);

    h.driver.replace();
    h.driver.replace();

    expect(h.maxLiveCanvasCount()).toBe(2);
    expect(h.canvases[1]).toMatchObject({ disposed: true, removed: true });
    expect(h.driver.debug()).toMatchObject({ target: h.canvases[2], displayed: h.canvases[0] });

    h.driver.syncPublished(h.canvases[1]);
    expect(h.driver.debug().displayed).toBe(h.canvases[0]);

    h.driver.syncPublished(h.canvases[2]);
    expect(h.driver.debug().displayed).toBe(h.canvases[2]);
    expect(h.canvases.filter((canvas) => !canvas.disposed)).toEqual([h.canvases[2]]);
  });

  it('keeps the published canvas when a replacement is oversized', () => {
    const h = harness(['cpu', 'cpu-oversized']);
    h.driver.setActive(true);
    h.driver.syncPublished(h.canvases[0]);

    h.driver.replace();

    expect(h.driver.debug()).toMatchObject({ target: undefined, displayed: h.canvases[0] });
    expect(h.canvases[0]).toMatchObject({ disposed: false, removed: false });
    expect(h.canvases[1]).toMatchObject({ disposed: true, removed: true });
    expect(h.isAttached()).toBe(false);
  });

  it('releases the target, listeners, and displayed canvas when parked', () => {
    const h = harness();
    h.driver.setActive(true);
    h.driver.syncPublished(h.canvases[0]);
    h.driver.setActive(false);

    expect(h.isAttached()).toBe(false);
    expect(h.driver.hasSurface()).toBe(false);
    expect(h.canvases[0]).toMatchObject({ disposed: true, removed: true, listeners: 0 });
    expect(h.driver.debug()).toMatchObject({ target: undefined, displayed: undefined, wantsLive: false });
  });

  it('freezes the published canvas on terminal failure and discards only an unpublished replacement', () => {
    const h = harness();
    h.driver.setActive(true);
    h.canvases[0].pixels = 'published pixels';
    h.driver.syncPublished(h.canvases[0]);
    h.driver.replace();

    h.driver.freeze();

    expect(h.canvases[0]).toMatchObject({ pixels: 'published pixels', disposed: false, removed: false });
    expect(h.canvases[1]).toMatchObject({ disposed: true, removed: true, listeners: 0 });
    expect(h.driver.debug()).toMatchObject({ target: undefined, displayed: h.canvases[0], wantsLive: false });
  });
});
