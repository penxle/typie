import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import PrismObject from './PrismObject.svelte';
import PrismSpinner from './PrismSpinner.svelte';
import type { PrismTarget } from '@typie/prism-ui';

const runtime = vi.hoisted(() => {
  const object = {
    destroy: vi.fn(),
    setTarget: vi.fn(),
    subscribe: vi.fn(() => vi.fn()),
    update: vi.fn(),
    whenReady: vi.fn(() => Promise.resolve('ready')),
  };
  const spinner = {
    destroy: vi.fn(),
    subscribe: vi.fn(() => vi.fn()),
    update: vi.fn(),
  };
  return {
    mountObject: vi.fn(() => object),
    mountSpinner: vi.fn(() => spinner),
    object,
    spinner,
  };
});

vi.mock('./runtime.ts', () => ({ prismRuntime: runtime }));

afterEach(() => {
  vi.clearAllMocks();
  document.body.replaceChildren();
});

describe('Typie Prism wrappers', () => {
  it('mounts one transition-capable object and destroys it with its host', async () => {
    const target = document.createElement('div');
    const component = mount(PrismObject, {
      target,
      props: { edgeColor: 'rgb(120 120 120)', reducedMotion: true, target: 'prism' as PrismTarget },
    });
    try {
      await tick();
      expect(runtime.mountObject).toHaveBeenCalledOnce();
      expect(runtime.mountObject).toHaveBeenCalledWith(expect.any(HTMLElement), {
        edgeColor: 'rgb(120 120 120)',
        preload: false,
        reducedMotion: true,
        target: 'icon',
      });
      expect(runtime.object.setTarget).toHaveBeenCalledWith('prism');
      expect(runtime.object.update).toHaveBeenCalledWith({ edgeColor: 'rgb(120 120 120)', reducedMotion: true });
    } finally {
      await unmount(component);
    }
    expect(runtime.object.destroy).toHaveBeenCalledOnce();
  });

  it('mounts the standalone spinner without initializing an object renderer', async () => {
    const target = document.createElement('div');
    const component = mount(PrismSpinner, { target, props: { label: '응답을 기다리는 중', reducedMotion: true } });
    try {
      await tick();
      expect(runtime.mountSpinner).toHaveBeenCalledOnce();
      expect(runtime.mountObject).not.toHaveBeenCalled();
      expect(target.querySelector('[role="status"]')?.getAttribute('aria-label')).toBe('응답을 기다리는 중');
    } finally {
      await unmount(component);
    }
    expect(runtime.spinner.destroy).toHaveBeenCalledOnce();
  });

  it('preloads WebGPU behind the icon and forwards a target journey budget', async () => {
    const target = document.createElement('div');
    const component = mount(PrismObject, {
      target,
      props: { preload: true, spinnerPlaybackStartedAt: 1234, target: 'spinner' as PrismTarget, targetDurationMs: 2200 },
    });
    try {
      await tick();
      expect(runtime.mountObject).toHaveBeenCalledWith(expect.any(HTMLElement), {
        edgeColor: undefined,
        preload: true,
        reducedMotion: false,
        target: 'icon',
      });
      expect(runtime.object.whenReady).toHaveBeenCalledOnce();
      expect(runtime.object.setTarget).toHaveBeenCalledWith('spinner', { spinnerPlaybackStartedAt: 1234, totalDurationMs: 2200 });
    } finally {
      await unmount(component);
    }
  });
});
