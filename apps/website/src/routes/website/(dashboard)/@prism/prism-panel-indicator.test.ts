import themeData from '@typie/assets/theme.json' with { type: 'json' };
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import PrismPanelIndicator from './PrismPanelIndicator.svelte';
import { reactiveProps } from './PrismPanelIndicator.test-props.svelte.ts';
import type { PrismRuntimeSnapshot } from '@typie/prism-ui';

const runtime = vi.hoisted(() => {
  let listener: ((snapshot: PrismRuntimeSnapshot) => void) | undefined;
  let snapshot: PrismRuntimeSnapshot = {
    journeyProgress: null,
    owner: 'svg',
    readiness: 'loading',
    requestedTarget: 'icon',
    settledTarget: 'icon',
  };
  const object = {
    destroy: vi.fn(),
    setTarget: vi.fn(),
    subscribe: vi.fn((next: (value: PrismRuntimeSnapshot) => void) => {
      listener = next;
      next({ ...snapshot });
      return () => {
        if (listener === next) listener = undefined;
      };
    }),
    update: vi.fn(),
    whenReady: vi.fn(() => Promise.resolve('ready')),
  };
  return {
    emit(next: Partial<PrismRuntimeSnapshot>) {
      snapshot = { ...snapshot, ...next };
      listener?.({ ...snapshot });
    },
    mountObject: vi.fn(() => object),
    mountSpinner: vi.fn(),
    object,
    reset() {
      listener = undefined;
      snapshot = {
        journeyProgress: null,
        owner: 'svg',
        readiness: 'loading',
        requestedTarget: 'icon',
        settledTarget: 'icon',
      };
    },
  };
});

vi.mock('$lib/prism-ui/runtime.ts', () => ({ prismRuntime: runtime }));

const rect = (centerX: number, centerY: number, size = 0): DOMRect => ({
  bottom: centerY + size / 2,
  height: size,
  left: centerX - size / 2,
  right: centerX + size / 2,
  top: centerY - size / 2,
  width: size,
  x: centerX - size / 2,
  y: centerY - size / 2,
  toJSON() {
    // DOMRect serialization is irrelevant to indicator placement tests.
  },
});

let animationFrames: Map<number, FrameRequestCallback>;
let idleCallbacks: Map<number, IdleRequestCallback>;
let nextFrameId: number;
let nextIdleId: number;
let originalAnimateDescriptor: PropertyDescriptor | undefined;
let animate: ReturnType<typeof vi.fn>;

const stepAnimationFrame = (now = 0) => {
  const callbacks = [...animationFrames.values()];
  animationFrames.clear();
  for (const callback of callbacks) callback(now);
};

const stepIdleCallback = () => {
  const callbacks = [...idleCallbacks.values()];
  idleCallbacks.clear();
  for (const callback of callbacks) callback({ didTimeout: false, timeRemaining: () => 50 });
};

beforeEach(() => {
  runtime.reset();
  vi.clearAllMocks();
  vi.useFakeTimers();
  animationFrames = new Map();
  idleCallbacks = new Map();
  nextFrameId = 1;
  nextIdleId = 1;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    const id = nextFrameId++;
    animationFrames.set(id, callback);
    return id;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => animationFrames.delete(id));
  vi.stubGlobal('requestIdleCallback', (callback: IdleRequestCallback) => {
    const id = nextIdleId++;
    idleCallbacks.set(id, callback);
    return id;
  });
  vi.stubGlobal('cancelIdleCallback', (id: number) => idleCallbacks.delete(id));
  originalAnimateDescriptor = Object.getOwnPropertyDescriptor(Element.prototype, 'animate');
  animate = vi.fn(
    () =>
      ({
        cancel: vi.fn(),
        currentTime: 0,
        effect: null,
        onfinish: null,
        playState: 'running',
      }) as unknown as Animation,
  );
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: animate,
  });
});

afterEach(() => {
  if (originalAnimateDescriptor) Object.defineProperty(Element.prototype, 'animate', originalAnimateDescriptor);
  else delete (Element.prototype as Partial<Element>).animate;
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.body.replaceChildren();
});

const destination = () => {
  const element = document.createElement('span');
  element.getBoundingClientRect = () => rect(40, 80, 18);
  return element;
};

const setSourceRect = (target: HTMLElement) => {
  const source = target.querySelector<HTMLElement>('[data-prism-indicator-source]');
  if (!source) throw new Error('missing indicator source');
  source.getBoundingClientRect = () => rect(200, 300);
};

describe('Prism panel indicator', () => {
  test('uses the theme-state edge color before the view-transition DOM catches up', async () => {
    const target = document.createElement('div');
    const previousTheme = document.documentElement.dataset.theme;
    const previousLightVariant = document.documentElement.dataset.variantLight;
    const previousDarkVariant = document.documentElement.dataset.variantDark;
    document.documentElement.dataset.theme = 'dark';
    document.documentElement.dataset.variantDark = 'black';
    const darkColor = themeData.variants['dark-black']['ui.border.default'];
    const lightColor = themeData.variants['light-white']['ui.border.default'];
    const props = reactiveProps({ phase: 'welcome' as const, themeVariant: 'dark-black' as 'dark-black' | 'light-white' });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      await tick();
      expect(runtime.object.update).toHaveBeenCalledWith(expect.objectContaining({ edgeColor: darkColor }));

      runtime.object.update.mockClear();
      props.themeVariant = 'light-white';
      await tick();
      await tick();
      expect(runtime.object.update).toHaveBeenCalledWith(expect.objectContaining({ edgeColor: lightColor }));
    } finally {
      if (previousTheme === undefined) delete document.documentElement.dataset.theme;
      else document.documentElement.dataset.theme = previousTheme;
      if (previousLightVariant === undefined) delete document.documentElement.dataset.variantLight;
      else document.documentElement.dataset.variantLight = previousLightVariant;
      if (previousDarkVariant === undefined) delete document.documentElement.dataset.variantDark;
      else document.documentElement.dataset.variantDark = previousDarkVariant;
      await unmount(component);
    }
  });

  test('starts only after both the 700 ms dwell and renderer readiness', async () => {
    const target = document.createElement('div');
    const props = reactiveProps({ phase: 'welcome' as const });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      expect(runtime.object.whenReady).toHaveBeenCalledOnce();
      expect(runtime.mountObject).toHaveBeenCalledWith(expect.any(HTMLElement), expect.objectContaining({ edgeColor: undefined }));
      expect(runtime.object.setTarget).toHaveBeenCalledWith('icon');

      await vi.advanceTimersByTimeAsync(699);
      expect(runtime.object.setTarget).not.toHaveBeenCalledWith('prism');
      await vi.advanceTimersByTimeAsync(1);
      expect(runtime.object.setTarget).not.toHaveBeenCalledWith('prism');

      runtime.emit({ readiness: 'ready' });
      await tick();
      expect(runtime.object.setTarget).not.toHaveBeenCalledWith('prism');
      stepAnimationFrame();
      await tick();
      expect(runtime.object.setTarget).not.toHaveBeenCalledWith('prism');

      stepIdleCallback();
      stepAnimationFrame();
      stepAnimationFrame();
      stepAnimationFrame();
      await tick();
      expect(runtime.object.setTarget).toHaveBeenCalledWith('prism');
    } finally {
      await unmount(component);
    }
  });

  test('waits for parent welcome admission without restarting the dwell', async () => {
    const target = document.createElement('div');
    const props = reactiveProps({ phase: 'welcome' as const, welcomeAdmission: false });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      await vi.advanceTimersByTimeAsync(700);
      runtime.emit({ readiness: 'ready' });
      stepIdleCallback();
      stepAnimationFrame();
      stepAnimationFrame();
      stepAnimationFrame();
      await tick();

      expect(runtime.object.setTarget).not.toHaveBeenCalledWith('prism');

      props.welcomeAdmission = true;
      await tick();
      stepAnimationFrame();
      await tick();

      expect(runtime.object.setTarget).toHaveBeenCalledWith('prism');
    } finally {
      await unmount(component);
    }
  });

  test('reveals the welcome message only after the icon-to-prism morph starts and keeps it outside the moving actor', async () => {
    const random = vi.spyOn(Math, 'random').mockReturnValue(0);
    const target = document.createElement('div');
    const props = reactiveProps({ phase: 'welcome' as const, welcomeMessageVisible: true });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      expect(target.querySelector('[data-prism-indicator-message]')).toBeNull();

      await vi.advanceTimersByTimeAsync(700);
      runtime.emit({ readiness: 'ready' });
      stepIdleCallback();
      stepAnimationFrame();
      stepAnimationFrame();
      stepAnimationFrame();
      await tick();

      const actor = target.querySelector('[data-prism-indicator-actor]');
      const message = target.querySelector('[data-prism-indicator-message]');
      expect(message?.textContent).toBe('도울 일이 있다면 맡겨주세요.');
      expect(actor?.contains(message)).toBe(false);
      expect(animate).toHaveBeenCalledWith(expect.any(Array), { duration: 1700, fill: 'forwards' });
    } finally {
      random.mockRestore();
      await unmount(component);
    }
  });

  test('keeps the welcome message until the indicator leaves the welcome phase', async () => {
    const target = document.createElement('div');
    const props = reactiveProps({
      phase: 'welcome' as 'submitting' | 'welcome',
      reducedMotion: true,
      welcomeMessageVisible: true,
    });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      expect(target.querySelector('[data-prism-indicator-message]')).not.toBeNull();
      expect(animate).not.toHaveBeenCalled();

      props.welcomeMessageVisible = false;
      await tick();
      expect(target.querySelector('[data-prism-indicator-message]')).not.toBeNull();

      props.phase = 'submitting';
      await tick();
      expect(target.querySelector('[data-prism-indicator-message]')).toBeNull();
    } finally {
      await unmount(component);
    }
  });

  test('starts the welcome message transition without a delay when morphing is unavailable', async () => {
    const target = document.createElement('div');
    const props = reactiveProps({ phase: 'welcome' as const, welcomeMessageVisible: true });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      runtime.emit({ readiness: 'unavailable' });
      await tick();

      expect(target.querySelector('[data-prism-indicator-message]')).not.toBeNull();
      expect(animate).toHaveBeenCalledWith(expect.any(Array), { duration: 0, fill: 'forwards' });
    } finally {
      await unmount(component);
    }
  });

  test('keeps the SVG icon idle and hands submission directly to the row spinner with reduced motion', async () => {
    const target = document.createElement('div');
    const owners: string[] = [];
    const props = reactiveProps({
      destination: destination() as HTMLElement | undefined,
      onSpinnerOwnerChange: (owner: string) => {
        owners.push(owner);
      },
      phase: 'welcome' as 'failed' | 'submitting' | 'welcome',
      reducedMotion: true,
    });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      setSourceRect(target);
      expect(runtime.object.whenReady).not.toHaveBeenCalled();
      expect(runtime.object.setTarget).toHaveBeenCalledWith('icon');

      await vi.advanceTimersByTimeAsync(700);
      runtime.emit({ readiness: 'ready' });
      stepIdleCallback();
      stepAnimationFrame();
      stepAnimationFrame();
      await tick();
      expect(runtime.object.setTarget).not.toHaveBeenCalledWith('prism');

      props.phase = 'submitting';
      await tick();
      expect(owners.at(-1)).toBe('row');
      expect(target.querySelector<HTMLElement>('[data-prism-indicator-actor]')?.style.visibility).toBe('hidden');
      expect(runtime.object.setTarget).not.toHaveBeenCalledWith('spinner', expect.anything());

      props.phase = 'failed';
      await tick();
      expect(target.querySelector<HTMLElement>('[data-prism-indicator-actor]')?.style.visibility).not.toBe('hidden');
      expect(runtime.object.setTarget).not.toHaveBeenCalledWith('prism');
    } finally {
      await unmount(component);
    }
  });

  test('finishes screen travel before the 2200 ms morph settles', async () => {
    const target = document.createElement('div');
    const props = reactiveProps({
      destination: destination() as HTMLElement | undefined,
      phase: 'welcome' as 'submitting' | 'welcome',
    });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      setSourceRect(target);
      runtime.emit({ readiness: 'ready' });
      props.phase = 'submitting';
      await tick();
      stepAnimationFrame();
      await tick();
      expect(runtime.object.setTarget).toHaveBeenCalledWith('spinner', { totalDurationMs: 2200 });

      const actor = target.querySelector<HTMLElement>('[data-prism-indicator-actor]');
      runtime.emit({ journeyProgress: 0.25, owner: 'webgpu', requestedTarget: 'spinner', settledTarget: null });
      const quarterTransform = actor?.style.transform;
      expect(quarterTransform).not.toBe('translate3d(0px, 0px, 0px)');
      expect(quarterTransform).not.toBe('translate3d(-160px, -220px, 0px)');

      runtime.emit({ journeyProgress: 0.75 });
      expect(actor?.style.transform).not.toBe(quarterTransform);
      expect(actor?.style.transform).toBe('translate3d(-160px, -220px, 0px)');

      runtime.emit({ journeyProgress: 1, owner: 'atlas', settledTarget: 'spinner' });
      expect(actor?.style.transform).toBe('translate3d(-160px, -220px, 0px)');
    } finally {
      await unmount(component);
    }
  });

  test('still lands on the destination when its DOM origin moves in flight', async () => {
    const target = document.createElement('div');
    const props = reactiveProps({
      destination: destination() as HTMLElement | undefined,
      phase: 'welcome' as 'submitting' | 'welcome',
    });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      setSourceRect(target);
      runtime.emit({ readiness: 'ready' });
      props.phase = 'submitting';
      await tick();
      stepAnimationFrame();
      await tick();

      const source = target.querySelector<HTMLElement>('[data-prism-indicator-source]');
      if (!source) throw new Error('missing indicator source');
      source.getBoundingClientRect = () => rect(220, 300);
      runtime.emit({ journeyProgress: 1, owner: 'atlas', requestedTarget: 'spinner', settledTarget: 'spinner' });

      expect(target.querySelector<HTMLElement>('[data-prism-indicator-actor]')?.style.transform).toBe('translate3d(-180px, -220px, 0px)');
    } finally {
      await unmount(component);
    }
  });

  test('hands an in-flight submission to the row spinner when the renderer becomes unavailable', async () => {
    const target = document.createElement('div');
    const owners: string[] = [];
    const props = reactiveProps({
      destination: destination() as HTMLElement | undefined,
      onSpinnerOwnerChange: (owner: string) => {
        owners.push(owner);
      },
      phase: 'welcome' as 'submitting' | 'welcome',
    });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      setSourceRect(target);
      runtime.emit({ readiness: 'ready' });
      props.phase = 'submitting';
      await tick();
      stepAnimationFrame();
      await tick();

      runtime.emit({ journeyProgress: 0.4, owner: 'webgpu', requestedTarget: 'spinner', settledTarget: null });
      runtime.emit({ readiness: 'unavailable' });
      await tick();

      expect(owners.at(-1)).toBe('row');
      expect(target.querySelector<HTMLElement>('[data-prism-indicator-actor]')?.style.visibility).toBe('hidden');
    } finally {
      await unmount(component);
    }
  });

  test('keeps the admitted actor through an early answer and removes it after arrival', async () => {
    const target = document.createElement('div');
    const props = reactiveProps({
      destination: undefined as HTMLElement | undefined,
      phase: 'welcome' as 'answered' | 'submitting' | 'welcome',
    });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      setSourceRect(target);
      runtime.emit({ readiness: 'ready' });
      props.destination = destination();
      props.phase = 'submitting';
      await tick();
      stepAnimationFrame();
      await tick();

      runtime.emit({ journeyProgress: 0.5, owner: 'webgpu', requestedTarget: 'spinner', settledTarget: null });
      const actor = target.querySelector<HTMLElement>('[data-prism-indicator-actor]');
      expect(actor?.style.transform).not.toBe('translate3d(0px, 0px, 0px)');

      props.phase = 'answered';
      props.destination = undefined;
      await tick();
      expect(runtime.object.destroy).not.toHaveBeenCalled();

      runtime.emit({ journeyProgress: 1, owner: 'atlas', requestedTarget: 'spinner', settledTarget: 'spinner' });
      await tick();
      expect(runtime.object.destroy).toHaveBeenCalledOnce();
    } finally {
      await unmount(component);
    }
  });

  test('reverses a partial morph and its current screen position together', async () => {
    const target = document.createElement('div');
    const props = reactiveProps({
      destination: destination() as HTMLElement | undefined,
      phase: 'welcome' as 'failed' | 'submitting' | 'welcome',
    });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      setSourceRect(target);
      runtime.emit({ readiness: 'ready' });
      props.phase = 'submitting';
      await tick();
      stepAnimationFrame();
      await tick();

      runtime.emit({ journeyProgress: 0.4, owner: 'webgpu', requestedTarget: 'spinner', settledTarget: null });
      const actor = target.querySelector<HTMLElement>('[data-prism-indicator-actor]');
      const departureTransform = actor?.style.transform;
      expect(departureTransform).not.toBe('translate3d(0px, 0px, 0px)');

      props.phase = 'failed';
      await tick();
      expect(runtime.object.setTarget).toHaveBeenCalledWith('prism', { totalDurationMs: 880 });

      runtime.emit({ journeyProgress: 0, requestedTarget: 'prism' });
      expect(actor?.style.transform).toBe(departureTransform);
      runtime.emit({ journeyProgress: 0.5 });
      expect(actor?.style.transform).not.toBe(departureTransform);
      expect(actor?.style.transform).toBe('translate3d(0px, 0px, 0px)');
      runtime.emit({ journeyProgress: 1, settledTarget: 'prism' });
      expect(actor?.style.transform).toBe('translate3d(0px, 0px, 0px)');
    } finally {
      await unmount(component);
    }
  });

  test('returns an arrived spinner along the same path without a position jump', async () => {
    const target = document.createElement('div');
    const props = reactiveProps({
      destination: destination() as HTMLElement | undefined,
      phase: 'welcome' as 'failed' | 'submitting' | 'welcome',
    });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      setSourceRect(target);
      runtime.emit({ readiness: 'ready' });
      props.phase = 'submitting';
      await tick();
      stepAnimationFrame();
      await tick();
      runtime.emit({ journeyProgress: 1, owner: 'atlas', requestedTarget: 'spinner', settledTarget: 'spinner' });

      const actor = target.querySelector<HTMLElement>('[data-prism-indicator-actor]');
      expect(actor?.style.transform).toBe('translate3d(-160px, -220px, 0px)');
      runtime.object.setTarget.mockClear();

      props.phase = 'failed';
      await tick();
      expect(actor?.style.transform).toBe('translate3d(-160px, -220px, 0px)');
      expect(runtime.object.setTarget).toHaveBeenCalledWith('prism', { totalDurationMs: 2200 });

      runtime.emit({ journeyProgress: 0, requestedTarget: 'prism', settledTarget: null });
      runtime.emit({ journeyProgress: 0.5 });
      expect(actor?.style.transform).not.toBe('translate3d(-160px, -220px, 0px)');
      expect(actor?.style.transform).toBe('translate3d(0px, 0px, 0px)');
      runtime.emit({ journeyProgress: 1, settledTarget: 'prism' });
      expect(actor?.style.transform).toBe('translate3d(0px, 0px, 0px)');
    } finally {
      await unmount(component);
    }
  });

  test('uses row APNG ownership atomically when submission starts before readiness', async () => {
    const target = document.createElement('div');
    const owners: string[] = [];
    const props = reactiveProps({
      destination: destination() as HTMLElement | undefined,
      onSpinnerOwnerChange: (owner: string) => {
        owners.push(owner);
      },
      phase: 'welcome' as 'failed' | 'submitting' | 'welcome',
    });
    const component = mount(PrismPanelIndicator, { target, props });
    try {
      await tick();
      setSourceRect(target);
      props.phase = 'submitting';
      await tick();
      stepAnimationFrame();
      expect(owners.at(-1)).toBe('row');
      expect(target.querySelector<HTMLElement>('[data-prism-indicator-actor]')?.style.visibility).toBe('hidden');

      runtime.emit({ readiness: 'ready' });
      expect(runtime.object.setTarget).not.toHaveBeenCalledWith('spinner', { totalDurationMs: 2200 });

      props.phase = 'failed';
      await tick();
      expect(target.querySelector<HTMLElement>('[data-prism-indicator-actor]')?.style.visibility).not.toBe('hidden');
      expect(runtime.object.setTarget).toHaveBeenCalledWith('prism');
    } finally {
      await unmount(component);
    }
  });
});
