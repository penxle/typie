import '../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ReducedMotionPresentationsTestRoot from './reduced-motion-presentations-test-root.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

const reducedMotion = vi.hoisted(() => {
  let matches = false;
  const listeners = new Set<EventListenerOrEventListenerObject>();
  const mediaQuery = {
    get matches() {
      return matches;
    },
    media: '(prefers-reduced-motion: reduce)',
    onchange: null,
    addEventListener: (_type: string, listener: EventListenerOrEventListenerObject) => listeners.add(listener),
    removeEventListener: (_type: string, listener: EventListenerOrEventListenerObject) => listeners.delete(listener),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: () => true,
  } satisfies MediaQueryList;

  vi.stubGlobal('matchMedia', (query: string) => {
    if (query === mediaQuery.media) return mediaQuery;
    return {
      ...mediaQuery,
      matches: false,
      media: query,
    } satisfies MediaQueryList;
  });

  return {
    reset() {
      matches = false;
    },
    set(next: boolean) {
      matches = next;
      const event = new Event('change');
      for (const listener of listeners) {
        if (typeof listener === 'function') listener(event);
        else listener.handleEvent(event);
      }
    },
  };
});

let component: ReturnType<typeof mount> | undefined;

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
const animatedDescendant = (root: HTMLElement | null) =>
  root ? [...root.querySelectorAll<HTMLElement>('*')].find((element) => getComputedStyle(element).animationName !== 'none') : undefined;

const presentationElements = () => {
  const sharedRoot = document.querySelector<HTMLElement>('[data-testid="shared-scrollbar"]');
  const editorRoot = document.querySelector<HTMLElement>('[data-testid="editor-scrollbar"]');
  const sharedTrack = sharedRoot?.querySelector<HTMLElement>('[role="scrollbar"]');
  const editorTrack = editorRoot?.querySelector<HTMLElement>('[role="scrollbar"]');
  const editorIndicator = editorTrack?.previousElementSibling as HTMLElement | null;
  const entitySkeleton = document.querySelector<HTMLElement>('[data-testid="entity-skeleton"]');
  const homeSkeleton = document.querySelector<HTMLElement>('[data-testid="home-skeleton"]');
  const entityAnimated = animatedDescendant(entitySkeleton ?? null);
  const homeAnimated = animatedDescendant(homeSkeleton ?? null);

  if (!sharedTrack || !editorTrack || !editorIndicator || !entityAnimated || !homeAnimated) {
    throw new Error(
      `Expected reduced-motion presentation fixtures: ${JSON.stringify({
        editorIndicator: Boolean(editorIndicator),
        editorTrack: Boolean(editorTrack),
        entityAnimated: Boolean(entityAnimated),
        homeAnimated: Boolean(homeAnimated),
        sharedTrack: Boolean(sharedTrack),
      })}`,
    );
  }
  return { editorIndicator, editorTrack, entityAnimated, homeAnimated, sharedTrack };
};

beforeEach(async () => {
  reducedMotion.reset();
  localStorage.clear();
  component = mount(ReducedMotionPresentationsTestRoot, { target: document.body });
  await tick();
  await frame();
  await expect.poll(() => document.querySelectorAll('[role="scrollbar"]').length).toBe(2);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

describe('reduced-motion presentations', () => {
  it('reserves the focus-mode control immediately before the pane close button', () => {
    const focusModeControl = document.querySelector('[data-pane-skeleton-focus-mode-control]');
    const closeButton = document.querySelector('[data-pane-skeleton-close-button]');

    expect(focusModeControl).not.toBeNull();
    expect(focusModeControl?.nextElementSibling).toBe(closeButton);
  });

  it('keeps the existing interpolation with normal motion', () => {
    const elements = presentationElements();

    expect(getComputedStyle(elements.sharedTrack).transitionDuration).not.toBe('0s');
    expect(getComputedStyle(elements.editorTrack).transitionDuration).not.toBe('0s');
    expect(getComputedStyle(elements.editorIndicator).transitionDuration).not.toBe('0s');
    expect(getComputedStyle(elements.entityAnimated).animationName).not.toBe('none');
    expect(getComputedStyle(elements.homeAnimated).animationName).not.toBe('none');
  });

  it('settles mounted scrollbars and skeletons when reduced motion turns on', async () => {
    const before = presentationElements();
    const entityBounds = before.entityAnimated.getBoundingClientRect();
    const homeBounds = before.homeAnimated.getBoundingClientRect();

    reducedMotion.set(true);
    await tick();
    await frame();

    expect(getComputedStyle(before.sharedTrack).transitionDuration).toBe('0s');
    expect(getComputedStyle(before.editorTrack).transitionDuration).toBe('0s');
    expect(getComputedStyle(before.editorIndicator).transitionDuration).toBe('0s');
    expect(getComputedStyle(before.entityAnimated).animationName).toBe('none');
    expect(getComputedStyle(before.homeAnimated).animationName).toBe('none');
    expect(before.entityAnimated.getBoundingClientRect()).toEqual(entityBounds);
    expect(before.homeAnimated.getBoundingClientRect()).toEqual(homeBounds);
  });
});
