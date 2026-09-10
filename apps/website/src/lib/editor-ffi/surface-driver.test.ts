import { describe, expect, it } from 'vitest';
import { createSurfaceDriver } from './surface-driver';
import type { AttachResult, SurfaceDriverEffects } from './surface-driver';

type FakeSurface = { id: number; pixels: string; disposed: boolean; removed: boolean; listeners: number };

function harness(attachResult: AttachResult | AttachResult[] = 'cpu') {
  let nextId = 0;
  let attached = false;
  let recoverCount = 0;
  let detachCount = 0;
  let promoteCount = 0;
  let replacementFailureCount = 0;
  let maxLiveSurfaceCount = 0;
  const canvases: FakeSurface[] = [];
  const lifecycle: string[] = [];
  const effects: SurfaceDriverEffects<FakeSurface> = {
    createSurface: () => {
      const surface = { id: nextId++, pixels: '', disposed: false, removed: false, listeners: 0 };
      canvases.push(surface);
      maxLiveSurfaceCount = Math.max(maxLiveSurfaceCount, canvases.filter((candidate) => !candidate.disposed).length);
      return surface;
    },
    styleSurface: () => {
      // Surface styling is outside this lifecycle fixture.
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
    addContextListeners: (surface) => {
      surface.listeners += 1;
      return () => {
        surface.listeners -= 1;
      };
    },
    releaseCpuBacking: (surface) => {
      lifecycle.push(`release:${surface.id}`);
      surface.pixels = '';
      surface.disposed = true;
    },
    promote: (surface, previous) => {
      lifecycle.push(`promote:${surface.id}:${previous?.id ?? 'none'}`);
      promoteCount += 1;
      surface.removed = false;
    },
    removeNode: (surface) => {
      lifecycle.push(`remove:${surface.id}`);
      surface.removed = true;
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
    maxLiveSurfaceCount: () => maxLiveSurfaceCount,
    lifecycle,
  };
}

describe('surface-driver', () => {
  it('does not expose a target surface before Editor publishes its delivered frame', () => {
    const h = harness();
    expect(h.driver.hasSurface()).toBe(false);

    h.driver.setActive(true);

    expect(h.driver.debug().target).toBe(h.canvases[0]);
    expect(h.driver.debug().displayed).toBeUndefined();
    expect(h.driver.hasSurface()).toBe(true);

    h.driver.syncPublished(h.canvases[0]);
    expect(h.driver.debug().displayed).toBe(h.canvases[0]);
  });

  it('ignores stale delivery for a surface that is not the current target', () => {
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

  it('forwards recovery for the current target without creating another surface', () => {
    const h = harness();
    h.driver.setActive(true);
    h.driver.resume();

    expect(h.canvases).toHaveLength(1);
    expect(h.recoverCount()).toBe(1);
  });

  it('re-promotes the same surface after an in-place backing replacement publishes', () => {
    const h = harness();
    h.driver.setActive(true);
    h.driver.syncPublished(h.canvases[0]);
    h.canvases[0].removed = true;

    h.driver.syncPublished(h.canvases[0]);

    expect(h.canvases[0].removed).toBe(false);
    expect(h.promoteCount()).toBe(2);
  });

  it('keeps the published surface visible until its replacement is published', () => {
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

    expect(h.maxLiveSurfaceCount()).toBe(2);
    expect(h.canvases[1]).toMatchObject({ disposed: true, removed: true });
    expect(h.driver.debug()).toMatchObject({ target: h.canvases[2], displayed: h.canvases[0] });

    h.driver.syncPublished(h.canvases[1]);
    expect(h.driver.debug().displayed).toBe(h.canvases[0]);

    h.driver.syncPublished(h.canvases[2]);
    expect(h.driver.debug().displayed).toBe(h.canvases[2]);
    expect(h.canvases.filter((surface) => !surface.disposed)).toEqual([h.canvases[2]]);
  });

  it('keeps the published surface when a replacement is oversized', () => {
    const h = harness(['cpu', 'cpu-oversized']);
    h.driver.setActive(true);
    h.driver.syncPublished(h.canvases[0]);

    h.driver.replace();

    expect(h.driver.debug()).toMatchObject({ target: undefined, displayed: h.canvases[0] });
    expect(h.canvases[0]).toMatchObject({ disposed: false, removed: false });
    expect(h.canvases[1]).toMatchObject({ disposed: true, removed: true });
    expect(h.isAttached()).toBe(false);
  });

  it('releases an inactive target while retaining its published display backing', () => {
    const h = harness();
    h.driver.setActive(true);
    h.canvases[0].pixels = 'published pixels';
    h.driver.syncPublished(h.canvases[0]);

    h.driver.setActive(false);

    expect(h.isAttached()).toBe(false);
    expect(h.detachCount()).toBe(1);
    expect(h.canvases[0]).toMatchObject({ pixels: 'published pixels', disposed: false, removed: false, listeners: 0 });
    expect(h.driver.debug()).toMatchObject({ target: undefined, displayed: h.canvases[0], wantsLive: false });

    h.driver.setActive(true);
    expect(h.driver.debug()).toMatchObject({ target: h.canvases[1], displayed: h.canvases[0], wantsLive: true });
  });

  it('releases the target and displayed surface when destroyed', () => {
    const h = harness();
    h.driver.setActive(true);
    h.driver.syncPublished(h.canvases[0]);

    h.driver.destroy();

    expect(h.driver.hasSurface()).toBe(false);
    expect(h.canvases[0]).toMatchObject({ disposed: true, removed: true, listeners: 0 });
    expect(h.driver.debug()).toMatchObject({ target: undefined, displayed: undefined, wantsLive: false });
  });

  it('freezes the published surface on terminal failure and discards only an unpublished replacement', () => {
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
