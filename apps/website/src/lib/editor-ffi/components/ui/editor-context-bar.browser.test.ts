import '../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import EditorContextBarTestRoot from './editor-context-bar-test-root.svelte';
import type { DocumentZoomLandmark } from '$lib/editor-ffi/zoom';

const reducedMotionPreference = vi.hoisted(() => {
  const query = '(prefers-reduced-motion: reduce)';
  const listeners = new Set<EventListenerOrEventListenerObject>();
  let matches = false;
  const mediaQuery = {
    get matches() {
      return matches;
    },
    media: query,
    onchange: null,
    addEventListener: (_type: string, listener: EventListenerOrEventListenerObject) => listeners.add(listener),
    removeEventListener: (_type: string, listener: EventListenerOrEventListenerObject) => listeners.delete(listener),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: () => true,
  } satisfies MediaQueryList;

  vi.stubGlobal('matchMedia', (candidate: string) =>
    candidate === query
      ? mediaQuery
      : ({
          ...mediaQuery,
          matches: false,
          media: candidate,
        } satisfies MediaQueryList),
  );

  return {
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

type TestProps = {
  enabled: boolean;
  displayZoom: number;
  indicatorZoom: number;
  landmark: DocumentZoomLandmark | null;
  atMinimum: boolean;
  atMaximum: boolean;
  toggleTargetLandmark: DocumentZoomLandmark | null;
  onZoomOut: () => void;
  onZoomIn: () => void;
  onToggleZoom: () => Promise<unknown>;
};

const TRANSIENT_VISIBLE_MS = 1500;
const HOVER_INTENT_SETTLE_MS = 100;
const LANE_SPOT_DWELL_MS = 500;
const LANE_BACKGROUND_EXPAND_MS = 900;
const LANE_EXPANSION_TOTAL_MS = 1000;
const LANE_ENGAGED_SPOT_ENTER_DELAY_MS = 500;
const LANE_ENGAGED_SPOT_STRENGTH = 0.85;
const LANE_FOREGROUND_REVEAL_DELAY_MS = 100;
const LANE_FOREGROUND_REVEAL_MS = 900;
const LANE_SPOT_RADIUS = 88;
const SURFACE_FADE_IN_MS = 180;
const SURFACE_FADE_OUT_MS = 400;
let mounted: Record<string, unknown> | undefined;

const enter = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerenter'));
const leave = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerleave'));
const move = (element: HTMLElement, clientX: number, clientY: number) =>
  element.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX, clientY, pointerType: 'mouse' }));

async function mountContextBar(
  props: Partial<{
    surfaceWidth: number;
    breadcrumbWidth: number;
    viewControlsWidth: number;
    viewControlsPaneEntry: boolean;
    withinPane: boolean;
    useAppPinned: boolean;
  }> = {},
) {
  mounted = mount(EditorContextBarTestRoot, { target: document.body, props: { mode: 'context-bar', ...props } });
  await tick();
  const surface = document.querySelector<HTMLElement>('[data-testid="context-bar-underlay"]')?.parentElement;
  const pane = document.querySelector<HTMLElement>('[data-pane-id="zoom-overlay-test-pane"]');
  const breadcrumb = document.querySelector<HTMLElement>('[data-context-bar-segment="leading"]');
  const viewControls = document.querySelector<HTMLElement>('[data-context-bar-segment="view-controls"]');
  const bar = document.querySelector<HTMLElement>('[data-editor-context-bar]');
  const breadcrumbViewport = document.querySelector<HTMLElement>('[data-horizontal-scroll-viewport="breadcrumb"]');
  if (!surface || !breadcrumb || !viewControls || !bar || !breadcrumbViewport) throw new Error('Missing editor context bar fixture');
  return { surface, pane, breadcrumb, breadcrumbViewport, viewControls, bar, host: mounted };
}

async function mountOverlay(
  {
    displayZoom = 1,
    indicatorZoom = displayZoom,
    landmark = null,
    toggleTargetLandmark = landmark === 'unit' ? 'fit-width' : 'unit',
    onZoomOut = () => null,
    onZoomIn = () => null,
    onToggleZoom = async () => null,
  }: Partial<TestProps> = {},
  { withinPane = true }: { withinPane?: boolean } = {},
) {
  mounted = mount(EditorContextBarTestRoot, {
    target: document.body,
    props: {
      initialEnabled: true,
      initialZoom: displayZoom,
      initialIndicatorZoom: indicatorZoom,
      initialLandmark: landmark,
      initialToggleTargetLandmark: toggleTargetLandmark,
      onZoomOut,
      onZoomIn,
      onToggleZoom,
      withinPane,
    },
  });
  await tick();
  const overlay = document.querySelector<HTMLElement>('[data-context-bar-segment="view-controls"]');
  const paneContainer = document.querySelector<HTMLElement>('[data-pane-id="zoom-overlay-test-pane"]');
  const scrollContainer = document.querySelector<HTMLElement>('[data-testid="editor-pane"]');
  const editorViewSurface = document.querySelector<HTMLElement>('[data-testid="context-bar-underlay"]')?.parentElement;
  const toolbar = document.querySelector<HTMLElement>('[data-testid="editor-toolbar"]');
  if (!overlay || !scrollContainer || !editorViewSurface || !toolbar) throw new Error('Missing editor view-controls fixture');
  return { overlay, paneContainer: paneContainer ?? editorViewSurface, scrollContainer, editorViewSurface, toolbar, host: mounted };
}

async function mountFloatingOverlay({
  displayZoom = 1,
  indicatorZoom = displayZoom,
  landmark = null,
  fixed = true,
  revealOnHover = false,
  requiresChrome = false,
  rightInset = 0,
  topInset = 0,
}: Partial<Pick<TestProps, 'displayZoom' | 'indicatorZoom' | 'landmark'>> & {
  fixed?: boolean;
  revealOnHover?: boolean;
  requiresChrome?: boolean;
  rightInset?: number;
  topInset?: number;
} = {}) {
  mounted = mount(EditorContextBarTestRoot, {
    target: document.body,
    props: {
      mode: 'floating-zoom',
      initialEnabled: true,
      initialZoom: displayZoom,
      initialIndicatorZoom: indicatorZoom,
      initialLandmark: landmark,
      floatingFixed: fixed,
      floatingRevealOnHover: revealOnHover,
      floatingRequiresChrome: requiresChrome,
      floatingRightInset: rightInset,
      floatingTopInset: topInset,
    },
  });
  await tick();
  return {
    anchor: document.querySelector<HTMLElement>('[data-floating-editor-zoom-anchor]'),
    overlay: document.querySelector<HTMLElement>('[data-floating-editor-zoom-controls]'),
    contextBar: document.querySelector<HTMLElement>('[data-editor-context-bar]'),
    pane: document.querySelector<HTMLElement>('[data-pane-id="zoom-overlay-test-pane"]'),
    host: mounted,
  };
}

async function expireTransientVisibility() {
  await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS);
  await tick();
}

async function startHiddenLaneGapHover(props: { useAppPinned?: boolean } = {}) {
  const contextBar = await mountContextBar({ surfaceWidth: 640, ...props });
  const { pane, breadcrumb, viewControls, bar } = contextBar;
  if (!pane) throw new Error('Missing pane fixture');
  enter(pane);
  await expireTransientVisibility();

  const breadcrumbRect = breadcrumb.getBoundingClientRect();
  const viewControlsRect = viewControls.getBoundingClientRect();
  const barRect = bar.getBoundingClientRect();
  const gapX = (breadcrumbRect.right + viewControlsRect.left) / 2;
  const gapY = barRect.top + barRect.height / 2;
  move(pane, gapX, gapY);
  return { ...contextBar, pane, breadcrumbRect, viewControlsRect, barRect, gapX, gapY };
}

afterEach(async () => {
  if (mounted) await unmount(mounted);
  mounted = undefined;
  reducedMotionPreference.set(false);
  vi.useRealTimers();
  vi.restoreAllMocks();
  localStorage.clear();
  document.body.style.removeProperty('--usersite-sticky-header-bottom');
  document.body.replaceChildren();
});

describe('editor context bar', () => {
  it('reveals both segments once on initial load even when pane entry omits view controls', async () => {
    vi.useFakeTimers();
    const { breadcrumb, viewControls } = await mountContextBar({ viewControlsPaneEntry: false });

    expect(breadcrumb.style.opacity).toBe('1');
    expect(viewControls.style.opacity).toBe('1');

    await expireTransientVisibility();
    expect(breadcrumb.style.opacity).toBe('0');
    expect(viewControls.style.opacity).toBe('0');
  });

  it('keeps the transient surface mask while the context bar fades out', async () => {
    vi.useFakeTimers();
    const { breadcrumb, viewControls, bar } = await mountContextBar({ viewControlsPaneEntry: false });
    const surface = document.querySelector<HTMLElement>('[data-context-bar-lane-surface]');
    const blur = document.querySelector<HTMLElement>('[data-context-bar-blur-layer]');
    const fixture = document.querySelector<HTMLElement>('[data-context-bar-top-occlusion]');

    expect(fixture?.dataset.contextBarTopOcclusion).toBe(`${bar.getBoundingClientRect().height}`);

    await expireTransientVisibility();

    expect(breadcrumb.style.opacity).toBe('0');
    expect(viewControls.style.opacity).toBe('0');
    expect(surface?.style.opacity).toBe('0');
    expect(surface?.style.maskImage).toContain('linear-gradient(black, black)');
    expect(blur?.style.maskImage).toContain('linear-gradient(black, black)');
    expect(fixture?.dataset.contextBarTopOcclusion).toBe(`${bar.getBoundingClientRect().height}`);

    await vi.advanceTimersByTimeAsync(SURFACE_FADE_OUT_MS);
    expect(surface?.style.maskImage).not.toContain('linear-gradient(black, black)');
    expect(blur?.style.maskImage).not.toContain('linear-gradient(black, black)');
    expect(fixture?.dataset.contextBarTopOcclusion).toBe('0');
  });

  it('preserves view controls and lets a long breadcrumb consume only the remaining width', async () => {
    const { breadcrumb, viewControls } = await mountContextBar({ surfaceWidth: 280, breadcrumbWidth: 400, viewControlsWidth: 124 });

    await vi.waitFor(() => expect(viewControls.getBoundingClientRect().width).toBeGreaterThanOrEqual(124));
    expect(breadcrumb.getBoundingClientRect().right).toBeLessThanOrEqual(viewControls.getBoundingClientRect().left);
  });

  it('fans one pane entry out with breadcrumb and unit-zoom view-control policies kept separate', async () => {
    vi.useFakeTimers();
    const { pane, surface, breadcrumb, viewControls } = await mountContextBar({ viewControlsPaneEntry: false });
    if (!pane) throw new Error('Missing pane fixture');
    await expireTransientVisibility();

    enter(pane);
    await tick();
    expect(breadcrumb.style.opacity).toBe('1');
    expect(viewControls.style.opacity).toBe('0');

    await vi.advanceTimersByTimeAsync(500);
    enter(surface);
    await vi.advanceTimersByTimeAsync(1000);
    expect(breadcrumb.style.opacity).toBe('0');
  });

  it('falls back to the editor view surface for pane entry', async () => {
    vi.useFakeTimers();
    const { pane, surface, breadcrumb } = await mountContextBar({ withinPane: false });
    expect(pane).toBeNull();
    await expireTransientVisibility();

    enter(surface);
    await tick();
    expect(breadcrumb.style.opacity).toBe('1');
  });

  it('keeps unified tones independent while retaining an already-visible sibling', async () => {
    vi.useFakeTimers();
    const { breadcrumb, viewControls } = await mountContextBar({ surfaceWidth: 280, breadcrumbWidth: 400 });
    await expireTransientVisibility();

    document.querySelector<HTMLButtonElement>('[data-testid="show-breadcrumb"]')?.click();
    document.querySelector<HTMLButtonElement>('[data-testid="hold-view-controls"]')?.click();
    await tick();
    enter(viewControls);
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS);

    expect(breadcrumb.dataset.contextBarTone).toBe('transient');
    expect(viewControls.dataset.contextBarTone).toBe('engaged');

    await vi.advanceTimersByTimeAsync(1000);
    expect(breadcrumb.style.opacity).toBe('1');

    document.querySelector<HTMLButtonElement>('[data-testid="release-view-controls"]')?.click();
    leave(viewControls);
    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS);
    expect(breadcrumb.style.opacity).toBe('0');
    expect(viewControls.style.opacity).toBe('0');
  });

  it('paints a unified pair as one surface and keeps mixed tones explicit', async () => {
    vi.useFakeTimers();
    const { breadcrumb, viewControls } = await mountContextBar({ surfaceWidth: 280, breadcrumbWidth: 400 });
    await expireTransientVisibility();

    document.querySelector<HTMLButtonElement>('[data-testid="show-breadcrumb"]')?.click();
    document.querySelector<HTMLButtonElement>('[data-testid="hold-view-controls"]')?.click();
    await tick();

    const laneSurface = document.querySelector<HTMLElement>('[data-context-bar-lane-surface]');
    const blurLayer = document.querySelector<HTMLElement>('[data-context-bar-blur-layer]');
    expect(laneSurface?.dataset.contextBarFullLane).toBe('true');
    expect(document.querySelector<HTMLElement>('[data-context-bar-surface="leading"]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-context-bar-surface="view-controls"]')?.style.opacity).toBe('0');

    enter(viewControls);
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS);
    await vi.advanceTimersByTimeAsync(SURFACE_FADE_IN_MS);
    expect(breadcrumb.dataset.contextBarTone).toBe('transient');
    expect(blurLayer?.style.maskImage).not.toBe('linear-gradient(black, black)');
    expect(laneSurface?.style.maskImage).toContain('linear-gradient(black, black)');
  });

  it('unifies two visible segments across an open lane without a transparent gap', async () => {
    const { bar } = await mountContextBar({ surfaceWidth: 640, breadcrumbWidth: 120, viewControlsWidth: 124 });
    const surface = document.querySelector<HTMLElement>('[data-context-bar-lane-surface]');

    expect(bar.dataset.contextBarUnified).toBe('true');
    expect(surface?.dataset.contextBarFullLane).toBe('true');
  });

  it('reveals a pointer-following lane spot after intent and expands it without restarting on movement', async () => {
    vi.useFakeTimers();
    const { pane, breadcrumb, viewControls, bar } = await mountContextBar({ surfaceWidth: 640 });
    if (!pane) throw new Error('Missing pane fixture');
    enter(pane);
    await expireTransientVisibility();

    const breadcrumbRect = breadcrumb.getBoundingClientRect();
    const viewControlsRect = viewControls.getBoundingClientRect();
    const barRect = bar.getBoundingClientRect();
    const firstX = (breadcrumbRect.right + viewControlsRect.left) / 2;
    const firstY = barRect.top + barRect.height / 2;
    move(pane, firstX, firstY);
    await tick();

    expect(bar.dataset.contextBarLanePhase).toBe('pending');
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS - 1);
    expect(document.querySelector<HTMLElement>('[data-context-bar-lane-spot]')?.dataset.contextBarSpotVisible).toBe('false');

    await vi.advanceTimersByTimeAsync(1);
    const spot = document.querySelector<HTMLElement>('[data-context-bar-lane-spot]');
    expect(bar.dataset.contextBarLanePhase).toBe('spot');
    expect(spot?.dataset.contextBarSpotVisible).toBe('true');
    expect(spot?.dataset.contextBarSpotSurfaceStrength).toBe('0.7');
    expect(document.querySelector<HTMLElement>('[data-context-bar-blur-layer]')?.dataset.contextBarBlurRadius).toBe('1.5');
    expect(getComputedStyle(spot as HTMLElement).maskImage).not.toBe('none');
    expect(spot?.dataset.contextBarSpotX).toBe(String(firstX - barRect.left));
    expect(spot?.dataset.contextBarSpotY).toBe(String(firstY - barRect.top));

    move(pane, firstX + 40, firstY + 3);
    await tick();
    expect(spot?.dataset.contextBarSpotX).toBe(String(firstX + 40 - barRect.left));
    expect(spot?.dataset.contextBarSpotY).toBe(String(firstY + 3 - barRect.top));

    await vi.advanceTimersByTimeAsync(LANE_SPOT_DWELL_MS);
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');
    expect(spot?.dataset.contextBarSpotSurfaceStrength).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-context-bar-blur-layer]')?.dataset.contextBarBlurRadius).toBe('2');
    expect(breadcrumb.style.opacity).toBe('1');
    expect(viewControls.style.opacity).toBe('1');
    expect(breadcrumb.parentElement?.style.maskImage).toContain('radial-gradient');
    expect(viewControls.parentElement?.style.maskImage).toContain('radial-gradient');

    await vi.advanceTimersByTimeAsync(LANE_EXPANSION_TOTAL_MS);
    expect(bar.dataset.contextBarLanePhase).toBe('held');
    expect(breadcrumb.style.opacity).toBe('1');
    expect(viewControls.style.opacity).toBe('1');
    expect(bar.dataset.contextBarUnified).toBe('true');
    expect(breadcrumb.parentElement?.style.maskImage).toBe('none');
    expect(viewControls.parentElement?.style.maskImage).toBe('none');

    move(pane, firstX, barRect.bottom + 20);
    await tick();
    expect(bar.dataset.contextBarLanePhase).toBe('idle');

    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS - 1);
    expect(breadcrumb.style.opacity).toBe('1');
    expect(viewControls.style.opacity).toBe('1');

    await vi.advanceTimersByTimeAsync(1);
    expect(breadcrumb.style.opacity).toBe('0');
    expect(viewControls.style.opacity).toBe('0');
  });

  it('finishes a 900ms background wave as the 100ms-delayed foreground reaches the shared 1000ms endpoint', async () => {
    vi.useFakeTimers();
    const { breadcrumb, viewControls, bar } = await startHiddenLaneGapHover();
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS + LANE_SPOT_DWELL_MS);

    const breadcrumbReveal = breadcrumb.parentElement as HTMLElement;
    const viewControlsReveal = viewControls.parentElement as HTMLElement;
    const revealRadius = '--context-bar-segment-reveal-radius';
    const revealOpacity = '--context-bar-segment-reveal-opacity';
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');
    expect(bar.style.transition).toContain(`${LANE_BACKGROUND_EXPAND_MS}ms`);
    expect(breadcrumbReveal.style.transition).toContain(`${LANE_FOREGROUND_REVEAL_MS}ms`);
    expect(viewControlsReveal.style.transition).toContain(`${LANE_FOREGROUND_REVEAL_MS}ms`);
    expect(breadcrumbReveal.style.getPropertyValue(revealRadius)).toBe(`${LANE_SPOT_RADIUS}px`);
    expect(viewControlsReveal.style.getPropertyValue(revealRadius)).toBe(`${LANE_SPOT_RADIUS}px`);
    expect(breadcrumbReveal.style.getPropertyValue(revealOpacity)).toBe('0');
    expect(viewControlsReveal.style.getPropertyValue(revealOpacity)).toBe('0');

    await vi.advanceTimersByTimeAsync(LANE_FOREGROUND_REVEAL_DELAY_MS - 1);
    expect(breadcrumbReveal.style.getPropertyValue(revealRadius)).toBe(`${LANE_SPOT_RADIUS}px`);

    await vi.advanceTimersByTimeAsync(1);
    expect(breadcrumbReveal.style.getPropertyValue(revealRadius)).not.toBe(`${LANE_SPOT_RADIUS}px`);
    expect(viewControlsReveal.style.getPropertyValue(revealRadius)).not.toBe(`${LANE_SPOT_RADIUS}px`);
    expect(breadcrumbReveal.style.getPropertyValue(revealOpacity)).toBe('1');
    expect(viewControlsReveal.style.getPropertyValue(revealOpacity)).toBe('1');

    await vi.advanceTimersByTimeAsync(LANE_BACKGROUND_EXPAND_MS - LANE_FOREGROUND_REVEAL_DELAY_MS);
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');
    expect(breadcrumbReveal.style.maskImage).toContain('radial-gradient');

    await vi.advanceTimersByTimeAsync(LANE_EXPANSION_TOTAL_MS - LANE_BACKGROUND_EXPAND_MS - 1);
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');
    expect(breadcrumbReveal.style.maskImage).toContain('radial-gradient');

    await vi.advanceTimersByTimeAsync(1);
    expect(bar.dataset.contextBarLanePhase).toBe('held');
    expect(breadcrumbReveal.style.maskImage).toBe('none');
    expect(viewControlsReveal.style.maskImage).toBe('none');
  });

  it('latches the transient wave while the engaged spot follows the pointer across segments and survives expansion', async () => {
    vi.useFakeTimers();
    const { pane, breadcrumb, bar, breadcrumbRect, viewControlsRect, barRect, gapX, gapY } = await startHiddenLaneGapHover();
    const origin = {
      x: gapX - barRect.left,
      y: gapY - barRect.top,
    };
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS);

    const transientSurface = document.querySelector<HTMLElement>('[data-context-bar-lane-surface]');
    const engagedSpot = document.querySelector<HTMLElement>('[data-context-bar-engaged-spot]');
    const blur = document.querySelector<HTMLElement>('[data-context-bar-blur-layer]');
    if (!transientSurface || !engagedSpot || !blur) throw new Error('Missing context bar surface layers');
    expect(bar.dataset.contextBarLanePhase).toBe('spot');
    expect(bar.style.transition).toContain(`--context-bar-lane-spot-opacity 500ms ease-out`);
    expect(engagedSpot.style.transition).toContain(`opacity 500ms ease-out ${LANE_ENGAGED_SPOT_ENTER_DELAY_MS}ms`);
    expect(engagedSpot.style.maskImage).toContain(`rgba(0, 0, 0, ${LANE_ENGAGED_SPOT_STRENGTH}) 0px`);
    expect(engagedSpot.dataset.contextBarSpotVisible).toBe('true');
    expect(engagedSpot.style.backdropFilter).toBe('');
    expect(transientSurface.style.maskImage).toContain(`circle at ${origin.x}px ${origin.y}px`);
    expect(blur.nextElementSibling).toBe(transientSurface);
    expect(transientSurface.nextElementSibling).toBe(engagedSpot);
    const layers = [...bar.children];
    expect(layers.indexOf(engagedSpot)).toBeLessThan(layers.indexOf(breadcrumb.parentElement as HTMLElement));

    await vi.advanceTimersByTimeAsync(LANE_SPOT_DWELL_MS);
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');

    const viewControlsPoint = {
      x: viewControlsRect.left + viewControlsRect.width / 2,
      y: viewControlsRect.top + viewControlsRect.height / 2,
    };
    move(pane, viewControlsPoint.x, viewControlsPoint.y);
    await tick();

    expect(bar.dataset.contextBarLanePhase).toBe('expanding');
    expect(transientSurface?.style.maskImage).toContain(`circle at ${origin.x}px ${origin.y}px`);
    expect(engagedSpot?.dataset.contextBarSpotX).toBe(String(viewControlsPoint.x - barRect.left));
    expect(engagedSpot?.dataset.contextBarSpotY).toBe(String(viewControlsPoint.y - barRect.top));

    await vi.advanceTimersByTimeAsync(LANE_EXPANSION_TOTAL_MS);
    expect(bar.dataset.contextBarLanePhase).toBe('held');
    expect(engagedSpot?.dataset.contextBarSpotVisible).toBe('true');

    const breadcrumbPoint = {
      x: breadcrumbRect.left + breadcrumbRect.width / 2,
      y: breadcrumbRect.top + breadcrumbRect.height / 2,
    };
    move(pane, breadcrumbPoint.x, breadcrumbPoint.y);
    await tick();
    expect(bar.dataset.contextBarLanePhase).toBe('held');
    expect(engagedSpot?.dataset.contextBarSpotX).toBe(String(breadcrumbPoint.x - barRect.left));

    move(pane, breadcrumbPoint.x, barRect.bottom + 20);
    await tick();
    expect(engagedSpot?.dataset.contextBarSpotVisible).toBe('false');
    expect(engagedSpot?.style.transition).toContain(`${SURFACE_FADE_OUT_MS}ms`);
  });

  it('removes only the expansion masks whose segments become focused or pointer-engaged', async () => {
    vi.useFakeTimers();
    const { breadcrumb, viewControls, bar } = await startHiddenLaneGapHover();
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS + LANE_SPOT_DWELL_MS);

    const breadcrumbReveal = breadcrumb.parentElement as HTMLElement;
    const viewControlsReveal = viewControls.parentElement as HTMLElement;
    expect(breadcrumbReveal.style.maskImage).toContain('radial-gradient');
    expect(viewControlsReveal.style.maskImage).toContain('radial-gradient');

    document.querySelector<HTMLButtonElement>('[data-testid="show-breadcrumb"]')?.focus();
    await tick();
    expect(breadcrumb.dataset.contextBarTone).toBe('engaged');
    expect(breadcrumbReveal.style.maskImage).toBe('none');
    expect(viewControlsReveal.style.maskImage).toContain('radial-gradient');

    enter(viewControls);
    await tick();
    expect(viewControls.dataset.contextBarTone).toBe('engaged');
    expect(viewControlsReveal.style.maskImage).toBe('none');
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');
  });

  it('keeps the engaged spot absent for pointerless transient reveals', async () => {
    vi.useFakeTimers();
    const { breadcrumb } = await mountContextBar({ surfaceWidth: 640 });
    const engagedSpot = document.querySelector<HTMLElement>('[data-context-bar-engaged-spot]');

    expect(breadcrumb.style.opacity).toBe('1');
    expect(engagedSpot).not.toBeNull();
    expect(engagedSpot?.dataset.contextBarSpotVisible).toBe('false');

    await expireTransientVisibility();
    document.querySelector<HTMLButtonElement>('[data-testid="show-breadcrumb"]')?.click();
    await tick();
    expect(breadcrumb.style.opacity).toBe('1');
    expect(engagedSpot?.dataset.contextBarSpotVisible).toBe('false');
  });

  it('clears a staggered reveal when expansion is interrupted and restarts from the next pointer origin', async () => {
    vi.useFakeTimers();
    const { pane, breadcrumb, viewControls, bar, barRect, gapX: firstX, gapY } = await startHiddenLaneGapHover();
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS + LANE_SPOT_DWELL_MS + LANE_FOREGROUND_REVEAL_DELAY_MS / 2);
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');

    move(pane, firstX, barRect.bottom + 20);
    await tick();
    expect(bar.dataset.contextBarLanePhase).toBe('idle');
    expect(breadcrumb.parentElement?.style.maskImage).toBe('none');
    expect(viewControls.parentElement?.style.maskImage).toBe('none');

    const secondX = firstX + 48;
    move(pane, secondX, gapY);
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS + LANE_SPOT_DWELL_MS);
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');
    expect(document.querySelector<HTMLElement>('[data-context-bar-lane-surface]')?.style.maskImage).toContain(
      `circle at ${secondX - barRect.left}px`,
    );
    expect(breadcrumb.parentElement?.style.getPropertyValue('--context-bar-segment-reveal-radius')).toBe(`${LANE_SPOT_RADIUS}px`);
  });

  it('skips the expansion waves under initial reduced motion while retaining the static engaged spot', async () => {
    vi.useFakeTimers();
    reducedMotionPreference.set(true);
    const { breadcrumb, viewControls, bar } = await startHiddenLaneGapHover();
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS + LANE_SPOT_DWELL_MS);

    expect(bar.dataset.contextBarLanePhase).toBe('held');
    expect(bar.style.transition).toBe('none');
    expect(breadcrumb.parentElement?.style.maskImage).toBe('none');
    expect(viewControls.parentElement?.style.maskImage).toBe('none');
    expect(document.querySelector<HTMLElement>('[data-context-bar-engaged-spot]')?.dataset.contextBarSpotVisible).toBe('true');
  });

  it('finishes an active expansion immediately when reduced motion turns on and does not replay when it turns off', async () => {
    vi.useFakeTimers();
    const { breadcrumb, viewControls, bar } = await startHiddenLaneGapHover();
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS + LANE_SPOT_DWELL_MS + LANE_FOREGROUND_REVEAL_DELAY_MS / 2);
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');
    expect(breadcrumb.parentElement?.style.maskImage).toContain('radial-gradient');

    reducedMotionPreference.set(true);
    await tick();
    expect(bar.dataset.contextBarLanePhase).toBe('held');
    expect(breadcrumb.parentElement?.style.maskImage).toBe('none');
    expect(viewControls.parentElement?.style.maskImage).toBe('none');
    expect(document.querySelector<HTMLElement>('[data-context-bar-engaged-spot]')?.dataset.contextBarSpotVisible).toBe('true');

    reducedMotionPreference.set(false);
    await tick();
    await vi.advanceTimersByTimeAsync(LANE_EXPANSION_TOTAL_MS * 2);
    expect(bar.dataset.contextBarLanePhase).toBe('held');
    expect(breadcrumb.parentElement?.style.maskImage).toBe('none');
    expect(viewControls.parentElement?.style.maskImage).toBe('none');
  });

  it('shows the lane spot immediately when one segment is already visible', async () => {
    vi.useFakeTimers();
    const { pane, breadcrumb, viewControls, bar } = await mountContextBar({ surfaceWidth: 640 });
    if (!pane) throw new Error('Missing pane fixture');
    await expireTransientVisibility();
    document.querySelector<HTMLButtonElement>('[data-testid="show-breadcrumb"]')?.click();
    await tick();

    const breadcrumbRect = breadcrumb.getBoundingClientRect();
    const viewControlsRect = viewControls.getBoundingClientRect();
    const barRect = bar.getBoundingClientRect();
    move(pane, (breadcrumbRect.right + viewControlsRect.left) / 2, barRect.top + barRect.height / 2);
    await tick();

    expect(bar.dataset.contextBarLanePhase).toBe('spot');
    expect(document.querySelector<HTMLElement>('[data-context-bar-lane-spot]')?.dataset.contextBarSpotVisible).toBe('true');
    expect(document.querySelector<HTMLElement>('[data-context-bar-blur-layer]')?.dataset.contextBarBlurRadius).toBe('2');
    const activeBlurLayers = [...document.querySelectorAll<HTMLElement>('[data-context-bar-blur-layer]')].filter(
      (element) => element.style.opacity !== '0' && element.style.backdropFilter !== 'blur(0px)',
    );
    expect(activeBlurLayers).toHaveLength(1);
  });

  it('reveals a hidden segment immediately when its sibling is already visible', async () => {
    vi.useFakeTimers();
    const { pane, breadcrumb, viewControls } = await mountContextBar({ surfaceWidth: 640 });
    if (!pane) throw new Error('Missing pane fixture');
    await expireTransientVisibility();
    document.querySelector<HTMLButtonElement>('[data-testid="show-breadcrumb"]')?.click();
    await tick();
    expect(breadcrumb.style.opacity).toBe('1');
    expect(viewControls.style.opacity).toBe('0');

    const rect = viewControls.getBoundingClientRect();
    move(pane, rect.left + rect.width / 2, rect.top + rect.height / 2);
    await tick();

    expect(viewControls.style.opacity).toBe('1');
    expect(viewControls.dataset.contextBarTone).toBe('engaged');
  });

  it('previews the lane spot outside a lone engaged segment and starts expansion only after leaving it', async () => {
    vi.useFakeTimers();
    const { pane, breadcrumb, viewControls, bar } = await mountContextBar({ surfaceWidth: 640 });
    if (!pane) throw new Error('Missing pane fixture');
    await expireTransientVisibility();
    document.querySelector<HTMLButtonElement>('[data-testid="show-breadcrumb"]')?.click();
    await tick();

    enter(breadcrumb);
    const blur = document.querySelector<HTMLElement>('[data-context-bar-blur-layer]');
    const breadcrumbRect = breadcrumb.getBoundingClientRect();
    move(pane, breadcrumbRect.left + breadcrumbRect.width / 2, breadcrumbRect.top + breadcrumbRect.height / 2);
    await tick();

    const transientSurface = document.querySelector<HTMLElement>('[data-context-bar-lane-spot]');
    expect(bar.dataset.contextBarLanePhase).toBe('preview');
    expect(blur?.style.maskComposite).toBe('add');
    expect(blur?.dataset.contextBarBlurRadius).toBe('2');
    expect(transientSurface?.style.maskComposite).toBe('add');
    expect(blur?.style.maskImage).not.toBe(transientSurface?.style.maskImage);
    expect(viewControls.style.opacity).toBe('0');

    await vi.advanceTimersByTimeAsync(SURFACE_FADE_IN_MS);
    expect(transientSurface?.style.maskComposite).toBe('intersect');

    await vi.advanceTimersByTimeAsync(LANE_SPOT_DWELL_MS + LANE_EXPANSION_TOTAL_MS);
    expect(bar.dataset.contextBarLanePhase).toBe('preview');
    expect(viewControls.style.opacity).toBe('0');

    leave(breadcrumb);
    const viewControlsRect = viewControls.getBoundingClientRect();
    const barRect = bar.getBoundingClientRect();
    move(pane, (breadcrumbRect.right + viewControlsRect.left) / 2, barRect.top + barRect.height / 2);
    await tick();
    expect(bar.dataset.contextBarLanePhase).toBe('spot');

    await vi.advanceTimersByTimeAsync(LANE_SPOT_DWELL_MS);
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');
    expect(viewControls.style.opacity).toBe('1');
  });

  it('keeps transient blur through the engaged fade and removes it once engaged is stable', async () => {
    vi.useFakeTimers();
    const { breadcrumb } = await mountContextBar({ surfaceWidth: 640 });
    await expireTransientVisibility();
    document.querySelector<HTMLButtonElement>('[data-testid="show-breadcrumb"]')?.click();
    await tick();

    enter(breadcrumb);
    await tick();

    expect(breadcrumb.dataset.contextBarTone).toBe('engaged');
    expect(document.querySelector<HTMLElement>('[data-context-bar-blur-layer]')?.dataset.contextBarBlurRadius).toBe('2');

    await vi.advanceTimersByTimeAsync(SURFACE_FADE_IN_MS);
    expect(document.querySelector<HTMLElement>('[data-context-bar-blur-layer]')?.dataset.contextBarBlurRadius).toBe('0');
  });

  it('keeps the full-width lane and hidden breadcrumb pointer-inert', async () => {
    vi.useFakeTimers();
    const { breadcrumb, bar } = await mountContextBar();
    await expireTransientVisibility();

    expect(getComputedStyle(bar).pointerEvents).toBe('none');
    expect(breadcrumb.style.pointerEvents).toBe('none');
  });

  it('aligns the first breadcrumb layout to its trailing edge and exposes the shared scrollbar', async () => {
    const { breadcrumbViewport } = await mountContextBar({ surfaceWidth: 280, breadcrumbWidth: 400 });

    await vi.waitFor(() => expect(breadcrumbViewport.scrollWidth).toBeGreaterThan(breadcrumbViewport.clientWidth));
    expect(breadcrumbViewport.scrollLeft).toBe(breadcrumbViewport.scrollWidth - breadcrumbViewport.clientWidth);

    const scrollbar = document.querySelector<HTMLElement>('[role="scrollbar"][aria-label="문서 경로 가로 스크롤"]');
    expect(scrollbar?.getAttribute('aria-controls')).toBe(breadcrumbViewport.id);
  });

  it('pins content growth only while already at the trailing edge and realigns on path replacement', async () => {
    const { breadcrumbViewport, host } = await mountContextBar({ surfaceWidth: 320, breadcrumbWidth: 440 });
    const fixture = host as { setBreadcrumb: (next: { contentWidth?: number; pathIdentity?: string }) => void };
    await vi.waitFor(() => expect(breadcrumbViewport.scrollLeft).toBeGreaterThan(0));

    fixture.setBreadcrumb({ contentWidth: 520 });
    await vi.waitFor(() => {
      expect(breadcrumbViewport.scrollWidth).toBe(520);
      expect(breadcrumbViewport.scrollLeft).toBe(breadcrumbViewport.scrollWidth - breadcrumbViewport.clientWidth);
    });

    breadcrumbViewport.scrollLeft = 40;
    breadcrumbViewport.dispatchEvent(new Event('scroll'));
    fixture.setBreadcrumb({ contentWidth: 600 });
    await vi.waitFor(() => expect(breadcrumbViewport.scrollWidth).toBe(600));
    expect(breadcrumbViewport.scrollLeft).toBe(40);

    fixture.setBreadcrumb({ pathIdentity: 'other/document' });
    await vi.waitFor(() => expect(breadcrumbViewport.scrollLeft).toBe(breadcrumbViewport.scrollWidth - breadcrumbViewport.clientWidth));
  });

  it('reveals only the breadcrumb segment when its path identity changes', async () => {
    vi.useFakeTimers();
    const { breadcrumb, viewControls, host } = await mountContextBar();
    const fixture = host as { setBreadcrumb: (next: { contentWidth?: number; pathIdentity?: string }) => void };
    await expireTransientVisibility();

    fixture.setBreadcrumb({ contentWidth: 180 });
    await tick();
    expect(breadcrumb.style.opacity).toBe('0');

    fixture.setBreadcrumb({ pathIdentity: 'other/document' });
    await tick();

    expect(breadcrumb.style.opacity).toBe('1');
    expect(viewControls.style.opacity).toBe('0');
  });

  it('preserves and clamps physical scroll position when the available width changes', async () => {
    const { breadcrumbViewport, host } = await mountContextBar({ surfaceWidth: 420, breadcrumbWidth: 600 });
    const fixture = host as { setSurfaceWidth: (width: number) => void };
    await vi.waitFor(() => expect(breadcrumbViewport.scrollLeft).toBeGreaterThan(0));

    breadcrumbViewport.scrollLeft = 60;
    breadcrumbViewport.dispatchEvent(new Event('scroll'));
    fixture.setSurfaceWidth(320);
    await vi.waitFor(() => expect(breadcrumbViewport.clientWidth).toBeLessThan(300));
    expect(breadcrumbViewport.scrollLeft).toBe(60);

    fixture.setSurfaceWidth(900);
    await vi.waitFor(() => expect(breadcrumbViewport.scrollWidth - breadcrumbViewport.clientWidth).toBe(0));
    expect(breadcrumbViewport.scrollLeft).toBe(0);
  });

  it('keeps the fixed pin and divider outside the breadcrumb scroll fog', async () => {
    const { breadcrumbViewport } = await mountContextBar({ surfaceWidth: 320, breadcrumbWidth: 600 });
    await vi.waitFor(() => expect(breadcrumbViewport.scrollLeft).toBeGreaterThan(0));
    const pin = document.querySelector<HTMLElement>('[data-context-bar-pin]');
    const divider = document.querySelector<HTMLElement>('[data-context-bar-pin-divider]');

    expect(breadcrumbViewport.style.maskImage).not.toBe('');
    expect(breadcrumbViewport.contains(pin)).toBe(false);
    expect(breadcrumbViewport.contains(divider)).toBe(false);
  });
});

describe('public viewer floating zoom controls', () => {
  it('stays hidden on initial enable and reveals only after zoom activity', async () => {
    vi.useFakeTimers();
    const { overlay, contextBar, host } = await mountFloatingOverlay({ displayZoom: 1, landmark: 'unit' });

    expect(contextBar).toBeNull();
    expect(overlay).not.toBeNull();
    expect(overlay?.style.opacity).toBe('0');

    (
      host as typeof mounted & { setZoom: (displayZoom: number, indicatorZoom: number, landmark: DocumentZoomLandmark | null) => void }
    ).setZoom(1.1, 1.1, null);
    await tick();

    expect(overlay?.style.opacity).toBe('1');
    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS);
    expect(overlay?.style.opacity).toBe('0');
  });

  it('uses a fixed viewport anchor and does not reveal a hidden viewer control on hover', async () => {
    document.body.style.setProperty('--usersite-sticky-header-bottom', '52px');
    const { anchor, overlay } = await mountFloatingOverlay();

    expect(anchor).not.toBeNull();
    expect(getComputedStyle(anchor as HTMLElement).position).toBe('fixed');
    expect(getComputedStyle(anchor as HTMLElement).top).toBe('52px');
    expect(overlay?.style.pointerEvents).toBe('none');
    enter(overlay as HTMLElement);
    await tick();
    expect(overlay?.style.opacity).toBe('0');
  });

  it('anchors below pane chrome, shifts for focus-mode exit, and uses a rounded transient surface without an edge fade', async () => {
    const { anchor, overlay } = await mountFloatingOverlay({ fixed: false, rightInset: 36, topInset: 78 });
    const blur = document.querySelector<HTMLElement>('[data-floating-editor-zoom-blur]');
    const surface = document.querySelector<HTMLElement>('[data-floating-editor-zoom-surface]');

    expect(getComputedStyle(anchor as HTMLElement).position).toBe('absolute');
    expect(getComputedStyle(anchor as HTMLElement).top).toBe('78px');
    expect(getComputedStyle(anchor as HTMLElement).right).toBe('36px');
    expect(anchor?.dataset.floatingEditorZoomRightInset).toBe('36');
    expect(getComputedStyle(overlay as HTMLElement).borderRadius).toBe('8px');
    expect(overlay?.getBoundingClientRect().height).toBeGreaterThan(0);
    expect(blur?.getBoundingClientRect().height).toBe(overlay?.getBoundingClientRect().height);
    expect(surface?.getBoundingClientRect().height).toBe(overlay?.getBoundingClientRect().height);
    expect(blur?.style.maskImage).toBe('');
    expect(surface?.style.maskImage).toBe('');
    expect(document.querySelector('[data-floating-editor-zoom-edge]')).toBeNull();
  });

  it('reveals only when the hidden editor zoom footprint is hovered', async () => {
    const { overlay, pane } = await mountFloatingOverlay({ fixed: false, revealOnHover: true });

    expect(overlay?.style.opacity).toBe('0');
    enter(pane as HTMLElement);
    await tick();
    expect(overlay?.style.opacity).toBe('0');
    expect(overlay?.style.pointerEvents).toBe('auto');

    enter(overlay as HTMLElement);
    await tick();
    expect(overlay?.style.opacity).toBe('1');
  });

  it('does not reveal a standalone hidden zoom footprint from unrelated window pointer movement', async () => {
    const { overlay, pane } = await mountFloatingOverlay({ fixed: false, revealOnHover: true });
    if (!overlay || !pane) throw new Error('Missing floating zoom fixture');
    const rect = overlay.getBoundingClientRect();

    move(pane, rect.left + rect.width / 2, rect.top + rect.height / 2);
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await tick();

    expect(overlay.style.opacity).toBe('0');
  });

  it('moves a hidden zoom footprint to the new chrome edge immediately', async () => {
    const { anchor, overlay, host } = await mountFloatingOverlay({ fixed: false, revealOnHover: true });
    if (!anchor || !overlay) throw new Error('Missing floating zoom fixture');
    const initialTop = anchor.getBoundingClientRect().top;

    (host as typeof mounted & { setFloatingLayout(next: { surfaceTop: number; topInset: number }): void }).setFloatingLayout({
      surfaceTop: 0,
      topInset: 78,
    });
    await tick();

    const rect = anchor.getBoundingClientRect();
    const hit = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
    expect(rect.top).toBeCloseTo(initialTop + 78, 1);
    expect(overlay.contains(hit)).toBe(true);
  });

  it('keeps a visible zoom control at its viewport position while the pane layout origin changes', async () => {
    const { anchor, overlay, host } = await mountFloatingOverlay({ fixed: false, revealOnHover: true });
    if (!anchor || !overlay) throw new Error('Missing floating zoom fixture');
    const fixture = host as typeof mounted & {
      setFloatingLayout(next: { surfaceTop: number; topInset: number }): void;
      setZoom(displayZoom: number, indicatorZoom: number, landmark: DocumentZoomLandmark | null): void;
    };
    fixture.setFloatingLayout({ surfaceTop: 78, topInset: 0 });
    fixture.setZoom(1.1, 1.1, null);
    await tick();
    const initialTop = anchor.getBoundingClientRect().top;

    fixture.setFloatingLayout({ surfaceTop: 0, topInset: 36 });
    await tick();

    expect(overlay.style.opacity).toBe('1');
    expect(anchor.getBoundingClientRect().top).toBeCloseTo(initialTop, 1);
  });

  it('keeps focus-mode zoom unavailable until pane toolbar reveal has finished', async () => {
    const { overlay, host } = await mountFloatingOverlay({ fixed: false, revealOnHover: true, requiresChrome: true, topInset: 78 });

    expect(overlay?.style.opacity).toBe('0');
    expect(overlay?.style.pointerEvents).toBe('none');
    enter(overlay as HTMLElement);
    await tick();
    expect(overlay?.style.opacity).toBe('0');

    (host as typeof mounted & { setFloatingChromeReady: (ready: boolean) => void }).setFloatingChromeReady(true);
    await tick();
    expect(overlay?.style.pointerEvents).toBe('auto');

    enter(overlay as HTMLElement);
    await tick();
    expect(overlay?.style.opacity).toBe('1');
  });

  it('reveals when the toolbar makes a hidden zoom footprint available under a stationary pointer', async () => {
    const { overlay, pane, host } = await mountFloatingOverlay({
      fixed: false,
      revealOnHover: true,
      requiresChrome: true,
      topInset: 78,
    });
    if (!overlay || !pane) throw new Error('Missing floating zoom fixture');
    const rect = overlay.getBoundingClientRect();

    move(pane, rect.left + rect.width / 2, rect.top + rect.height / 2);
    (host as typeof mounted & { setFloatingChromeReady(ready: boolean): void }).setFloatingChromeReady(true);
    await tick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await tick();

    expect(overlay.style.opacity).toBe('1');
  });

  it('still presents zoom activity independently while focus-mode toolbar is hidden', async () => {
    const { anchor, overlay, host } = await mountFloatingOverlay({ fixed: false, revealOnHover: true, requiresChrome: true });

    (
      host as typeof mounted & { setZoom: (displayZoom: number, indicatorZoom: number, landmark: DocumentZoomLandmark | null) => void }
    ).setZoom(1.1, 1.1, null);
    await tick();

    expect(overlay?.style.opacity).toBe('1');
    expect(overlay?.dataset.floatingEditorZoomAttached).toBe('false');
    expect(getComputedStyle(anchor as HTMLElement).top).toBe('0px');
  });

  it('hands hover ownership to pane chrome when zoom is attached to header only', async () => {
    const { overlay, host } = await mountFloatingOverlay({ fixed: false, revealOnHover: true, requiresChrome: true });
    const fixture = host as typeof mounted & {
      setFloatingChromeState(next: { ready: boolean; attached: boolean }): void;
      setZoom(displayZoom: number, indicatorZoom: number, landmark: DocumentZoomLandmark | null): void;
    };
    fixture.setFloatingChromeState({ ready: false, attached: true });
    fixture.setZoom(1.1, 1.1, null);
    await tick();

    enter(overlay as HTMLElement);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-floating-chrome-request-count]')?.value).toBe('1');
  });

  it('remains keyboard discoverable and reveals before a hidden control receives interaction', async () => {
    const { overlay } = await mountFloatingOverlay();
    const zoomOut = overlay?.querySelector<HTMLButtonElement>('button');

    expect(zoomOut?.tabIndex).toBe(0);
    zoomOut?.focus();
    await tick();

    expect(overlay?.style.opacity).toBe('1');
    expect(overlay?.style.pointerEvents).toBe('auto');
  });
});

describe('context bar pin', () => {
  it('shows the pointer engaged spot without a reveal delay while pinned', async () => {
    const { pane, breadcrumb, viewControls, bar } = await mountContextBar({ surfaceWidth: 640, useAppPinned: true });
    if (!pane) throw new Error('Missing pane fixture');
    const breadcrumbRect = breadcrumb.getBoundingClientRect();
    const viewControlsRect = viewControls.getBoundingClientRect();
    const barRect = bar.getBoundingClientRect();

    move(pane, (breadcrumbRect.right + viewControlsRect.left) / 2, barRect.top + barRect.height / 2);
    await tick();

    const engagedSpot = document.querySelector<HTMLElement>('[data-context-bar-engaged-spot]');
    expect(bar.dataset.contextBarUnified).toBe('true');
    expect(bar.dataset.contextBarLanePhase).toBe('held');
    expect(engagedSpot?.dataset.contextBarSpotVisible).toBe('true');
    expect(engagedSpot?.style.transition).toBe('opacity 500ms ease-out');
  });

  it('defaults to pinned and restores an explicitly persisted unpinned preference', async () => {
    mounted = mount(EditorContextBarTestRoot, {
      target: document.body,
      props: { mode: 'context-bar', preferenceUserId: 'pin-default', useAppPinned: true },
    });
    await tick();

    expect(document.querySelector<HTMLButtonElement>('[data-context-bar-pin]')?.getAttribute('aria-pressed')).toBe('true');

    await unmount(mounted);
    mounted = undefined;
    document.body.replaceChildren();
    localStorage.setItem('typie:pref:pin-persisted', JSON.stringify({ contextBarPinned: false }));
    mounted = mount(EditorContextBarTestRoot, {
      target: document.body,
      props: { mode: 'context-bar', preferenceUserId: 'pin-persisted', useAppPinned: true },
    });
    await tick();

    expect(document.querySelector<HTMLButtonElement>('[data-context-bar-pin]')?.getAttribute('aria-pressed')).toBe('false');
  });

  it('updates every pane through one app preference', async () => {
    mounted = mount(EditorContextBarTestRoot, {
      target: document.body,
      props: { mode: 'context-bar', preferenceUserId: 'pin-shared', twoPanes: true, useAppPinned: true },
    });
    await tick();
    const pins = () => [...document.querySelectorAll<HTMLButtonElement>('[data-context-bar-pin]')];

    expect(pins()).toHaveLength(2);
    pins()[0]?.click();
    await tick();

    expect(pins().map((pin) => pin.getAttribute('aria-pressed'))).toEqual(['false', 'false']);
  });

  it('keeps the hidden pin keyboard reachable without exposing a pointer hit area', async () => {
    vi.useFakeTimers();
    localStorage.setItem('typie:pref:editor-context-bar-test', JSON.stringify({ contextBarPinned: false }));
    const { breadcrumb } = await mountContextBar({ useAppPinned: true });
    const pin = document.querySelector<HTMLButtonElement>('[data-context-bar-pin]');

    await expireTransientVisibility();
    expect(breadcrumb.style.opacity).toBe('0');
    expect(breadcrumb.style.pointerEvents).toBe('none');
    expect(pin?.tabIndex).toBe(0);

    pin?.focus();
    await tick();
    expect(breadcrumb.style.opacity).toBe('1');
  });

  it('settles an active lane expansion immediately when pinned', async () => {
    vi.useFakeTimers();
    localStorage.setItem('typie:pref:editor-context-bar-test', JSON.stringify({ contextBarPinned: false }));
    const { pane, bar, gapX, gapY } = await startHiddenLaneGapHover({ useAppPinned: true });

    move(pane, gapX, gapY);
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS + LANE_SPOT_DWELL_MS);
    expect(bar.dataset.contextBarLanePhase).toBe('expanding');

    (mounted as typeof mounted & { setPinned: (pinned: boolean) => void }).setPinned(true);
    await tick();

    expect(bar.dataset.contextBarLanePhase).toBe('idle');
    expect(bar.dataset.contextBarUnified).toBe('true');
  });
});

describe('editor view controls visibility', () => {
  it('reveals view controls when focus mode changes outside the control', async () => {
    vi.useFakeTimers();
    const { host, overlay } = await mountOverlay({ displayZoom: 1, indicatorZoom: 1, landmark: 'unit' });
    const focusModeControl = () => document.querySelector<HTMLButtonElement>('[data-editor-focus-mode-control]');

    expect(focusModeControl()?.ariaLabel).toBe('집중 모드 켜기');
    await expireTransientVisibility();
    expect(overlay.style.opacity).toBe('0');

    (host as { setFocusMode: (enabled: boolean) => void }).setFocusMode(true);
    await tick();

    expect(overlay.style.opacity).toBe('1');
    expect(focusModeControl()?.ariaLabel).toBe('집중 모드 끄기');
    expect(focusModeControl()?.tabIndex).toBe(0);
  });

  it('reveals an initial unit zoom for one transient window', async () => {
    vi.useFakeTimers();
    const unit = await mountOverlay({ displayZoom: 1, indicatorZoom: 1, landmark: 'unit' });
    expect(unit.overlay.style.opacity).toBe('1');

    await expireTransientVisibility();
    expect(unit.overlay.style.opacity).toBe('0');
  });

  it('waits to reveal the initial unit zoom until the controls become available', async () => {
    vi.useFakeTimers();
    const host = mount(EditorContextBarTestRoot, {
      target: document.body,
      props: { initialEnabled: false, initialZoom: 1, initialLandmark: null },
    });
    mounted = host;
    await tick();

    enter(document.querySelector<HTMLElement>('[data-pane-id="zoom-overlay-test-pane"]') as HTMLElement);
    host.setZoom(1, 1, 'unit');
    host.setEnabled(true);
    await tick();

    await vi.waitFor(() =>
      expect(document.querySelector<HTMLElement>('[data-context-bar-segment="view-controls"]')?.style.opacity).toBe('1'),
    );
  });

  it('uses the unzoomed editor surface coordinate system', async () => {
    const { overlay, editorViewSurface } = await mountOverlay();
    const anchor = overlay.closest<HTMLElement>('[data-editor-context-bar]');

    expect(anchor?.parentElement).toBe(editorViewSurface);
    expect(getComputedStyle(anchor as HTMLElement).position).toBe('absolute');
    expect(getComputedStyle(anchor as HTMLElement).top).toBe('0px');
    expect(getComputedStyle(anchor as HTMLElement).right).toBe('0px');
    expect(getComputedStyle(anchor as HTMLElement).zIndex).toBe('5');
    expect(anchor?.getBoundingClientRect().right).toBe(editorViewSurface.getBoundingClientRect().right);
  });

  it('suppresses editor entry at unit zoom but reveals a non-unit pane', async () => {
    vi.useFakeTimers();
    let fixture = await mountOverlay({ displayZoom: 1, indicatorZoom: 1, landmark: 'unit' });
    await expireTransientVisibility();

    enter(fixture.paneContainer);
    await tick();
    expect(fixture.overlay.style.opacity).toBe('0');

    await unmount(mounted as Record<string, unknown>);
    mounted = undefined;
    document.body.replaceChildren();
    fixture = await mountOverlay({ displayZoom: 1.25, indicatorZoom: 1.25 });
    await expireTransientVisibility();

    enter(fixture.paneContainer);
    await tick();
    expect(fixture.overlay.style.opacity).toBe('1');
  });

  it('does not restart pane entry visibility when moving from toolbar to editor surface', async () => {
    vi.useFakeTimers();
    const { overlay, paneContainer, editorViewSurface } = await mountOverlay({ displayZoom: 1.25, indicatorZoom: 1.25 });
    await expireTransientVisibility();

    enter(paneContainer);
    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS - 500);
    enter(editorViewSurface);
    await vi.advanceTimersByTimeAsync(500);

    expect(overlay.style.opacity).toBe('0');
  });

  it('falls back to the editor surface when no pane container exists', async () => {
    vi.useFakeTimers();
    const { overlay, editorViewSurface } = await mountOverlay({ displayZoom: 1.25, indicatorZoom: 1.25 }, { withinPane: false });
    await expireTransientVisibility();

    enter(editorViewSurface);
    await tick();

    expect(overlay.style.opacity).toBe('1');
  });

  it('stays hidden during an incidental pass that leaves before hover intent', async () => {
    vi.useFakeTimers();
    const { overlay } = await mountOverlay({ displayZoom: 1.25, indicatorZoom: 1.25 });
    await expireTransientVisibility();
    const hitRegion = overlay;

    enter(hitRegion);
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS - 1);
    expect(overlay.style.opacity).toBe('0');

    leave(hitRegion);
    await tick();
    expect(overlay.style.opacity).toBe('0');

    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS);

    expect(overlay.style.opacity).toBe('0');
  });

  it('stays visible after hover intent is established', async () => {
    vi.useFakeTimers();
    const { overlay } = await mountOverlay({ displayZoom: 1.25, indicatorZoom: 1.25 });
    await expireTransientVisibility();
    const hitRegion = overlay;

    enter(hitRegion);
    await vi.advanceTimersByTimeAsync(HOVER_INTENT_SETTLE_MS - 1);
    expect(overlay.style.opacity).toBe('0');

    await vi.advanceTimersByTimeAsync(1);
    expect(overlay.style.opacity).toBe('1');

    await vi.advanceTimersByTimeAsync(2000);
    expect(overlay.style.opacity).toBe('1');

    leave(hitRegion);
    await tick();
    expect(overlay.style.opacity).toBe('1');

    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS - 1);
    expect(overlay.style.opacity).toBe('1');

    await vi.advanceTimersByTimeAsync(1);
    expect(overlay.style.opacity).toBe('0');
  });

  it('lingers after keyboard focus leaves the indicator', async () => {
    vi.useFakeTimers();
    const { overlay } = await mountOverlay({ displayZoom: 1.25, indicatorZoom: 1.25 });
    await expireTransientVisibility();
    const value = document.querySelector<HTMLButtonElement>('[aria-label="원본 크기로 돌아가기"]');

    value?.focus();
    await tick();
    expect(overlay.style.opacity).toBe('1');

    value?.blur();
    await tick();
    expect(overlay.style.opacity).toBe('1');

    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS - 1);
    expect(overlay.style.opacity).toBe('1');

    await vi.advanceTimersByTimeAsync(1);
    expect(overlay.style.opacity).toBe('0');
  });
});

describe('editor zoom controls', () => {
  it('reveals on the first hidden touch and activates only on the next touch', async () => {
    vi.useFakeTimers();
    const onToggleZoom = vi.fn(async () => null);
    const { overlay } = await mountOverlay({ landmark: 'unit', onToggleZoom });
    const value = document.querySelector<HTMLButtonElement>('[aria-label="화면에 맞추기"]');
    await expireTransientVisibility();

    value?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, pointerType: 'touch' }));
    value?.click();
    await tick();
    expect(overlay.style.opacity).toBe('1');
    expect(onToggleZoom).not.toHaveBeenCalled();

    value?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, pointerType: 'touch' }));
    value?.click();
    await tick();
    expect(onToggleZoom).toHaveBeenCalledOnce();
  });

  it('does not present a landmark from an asynchronously completed toggle command', async () => {
    const toggle = Promise.withResolvers<DocumentZoomLandmark | null>();
    const onToggleZoom = vi.fn(() => toggle.promise);
    await mountOverlay({ landmark: 'unit', onToggleZoom });
    const value = document.querySelector<HTMLButtonElement>('[aria-label="화면에 맞추기"]');

    value?.click();
    toggle.resolve('fit-width');
    await tick();

    expect(value?.textContent).toContain('100%');
    expect(value?.textContent).not.toContain('맞춤');
  });

  it('keeps an at-boundary control actionable with explanatory semantics', async () => {
    const onZoomOut = vi.fn(() => false);
    await mountOverlay({
      displayZoom: 0.2,
      indicatorZoom: 0.2,
      landmark: 'minimum',
      atMinimum: true,
      onZoomOut,
    });
    const zoomOut = document.querySelector<HTMLButtonElement>('button[data-at-zoom-boundary="true"]');

    expect(zoomOut?.getAttribute('aria-label')).toBe('최소 배율입니다');
    zoomOut?.click();
    await tick();

    expect(onZoomOut).toHaveBeenCalledOnce();
  });

  it('shows the current landmark while the value is hovered and keeps unnamed percentages', async () => {
    await mountOverlay({ displayZoom: 1, indicatorZoom: 1, landmark: 'unit' });
    let value = document.querySelector<HTMLButtonElement>('[aria-label="화면에 맞추기"]');
    expect(value).not.toBeNull();
    enter(value as HTMLButtonElement);
    await tick();
    expect(value?.textContent).toContain('원본');

    await unmount(mounted as Record<string, unknown>);
    mounted = undefined;
    document.body.replaceChildren();
    const fixture = await mountOverlay({ displayZoom: 1.25, indicatorZoom: 1.25, landmark: null });
    value = document.querySelector<HTMLButtonElement>('[aria-label="원본 크기로 돌아가기"]');
    enter(value as HTMLButtonElement);
    await tick();
    expect(value?.textContent).toContain('125%');
    expect(fixture.overlay.style.opacity).toBe('1');
  });

  it('describes a clamped fit action by its actual maximum landmark', async () => {
    await mountOverlay({ landmark: 'unit', toggleTargetLandmark: 'maximum' });

    expect(document.querySelector('[aria-label="최대 배율로 확대"]')).not.toBeNull();
  });
});

describe('editor zoom snap feedback', () => {
  it('starts once on applied landmark entry and restarts only after leaving it', async () => {
    const host = mount(EditorContextBarTestRoot, { target: document.body });
    mounted = host;
    await tick();
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBeNull();

    host.setZoom(1, 1, 'unit');
    await tick();
    const firstFeedback = document.querySelector('[data-zoom-snap-feedback]');
    expect(firstFeedback).not.toBeNull();

    host.setZoom(1, 1, 'unit');
    await tick();
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBe(firstFeedback);

    host.setZoom(0.95, 0.95, null);
    await tick();
    host.setZoom(1, 1, 'unit');
    await tick();
    expect(document.querySelector('[data-zoom-snap-feedback]')).not.toBe(firstFeedback);
  });

  it('uses the applied bound landmark for snap feedback', async () => {
    const host = mount(EditorContextBarTestRoot, { target: document.body, props: { initialZoom: 1, initialLandmark: 'unit' } });
    mounted = host;
    await tick();
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBeNull();

    host.setZoom(2, 2, 'maximum');
    await tick();
    expect(document.querySelector<HTMLElement>('[data-zoom-snap-feedback]')?.dataset.zoomSnapLandmark).toBe('maximum');
  });

  it('restarts only the hidden overlay and boundary label for an unchanged minimum attempt', async () => {
    vi.useFakeTimers();
    const host = mount(EditorContextBarTestRoot, {
      target: document.body,
      props: { initialZoom: 0.2, initialIndicatorZoom: 0.2, initialLandmark: 'minimum' },
    });
    mounted = host;
    await tick();
    const overlay = document.querySelector<HTMLElement>('[data-context-bar-segment="view-controls"]');
    await expireTransientVisibility();
    expect(overlay?.style.opacity).toBe('0');

    host.requestBoundaryAttempt('minimum');
    await tick();

    expect(overlay?.style.opacity).toBe('1');
    expect(document.querySelector('[aria-live="polite"]')?.textContent).toContain('최소');
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBeNull();
  });

  it('announces overshoot once and restarts its label when it returns to range', async () => {
    vi.useFakeTimers();
    const host = mount(EditorContextBarTestRoot, { target: document.body });
    mounted = host;
    await tick();

    host.setZoom(0.15, 0.2, 'minimum');
    await tick();
    expect(document.querySelector('[aria-live="polite"]')?.textContent).toContain('최소');
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBeNull();

    await vi.advanceTimersByTimeAsync(2900);
    host.setZoom(0.14, 0.2, 'minimum');
    await vi.advanceTimersByTimeAsync(220);
    expect(document.querySelector<HTMLElement>('[data-context-bar-segment="view-controls"]')?.style.opacity).toBe('1');
    expect(document.querySelector('[aria-live="polite"]')?.textContent).toContain('최소');

    host.setZoom(0.2, 0.2, 'minimum');
    await tick();
    const recoveryFeedback = document.querySelector('[data-zoom-snap-feedback]');
    expect(recoveryFeedback).not.toBeNull();
    expect(document.querySelector('[aria-live="polite"]')?.textContent).toContain('최소');

    host.setZoom(0.15, 0.2, 'minimum');
    await tick();
    expect(document.querySelector('[aria-live="polite"]')?.textContent).toContain('최소');
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBe(recoveryFeedback);
  });

  it('keeps the recovered boundary landmark visible for one second', async () => {
    vi.useFakeTimers();
    const host = mount(EditorContextBarTestRoot, { target: document.body });
    mounted = host;
    await tick();

    host.setZoom(0.15, 0.2, 'minimum');
    await vi.advanceTimersByTimeAsync(500);
    host.setZoom(0.2, 0.2, 'minimum');
    await tick();

    const displayedValue = () => document.querySelector<HTMLElement>('[aria-live="polite"]');
    expect(displayedValue()?.textContent).toContain('최소');

    await vi.advanceTimersByTimeAsync(999);
    expect(displayedValue()?.dataset.zoomValueKind).toBe('landmark');

    await vi.advanceTimersByTimeAsync(1);
    expect(displayedValue()?.dataset.zoomValueKind).toBe('percentage');
  });

  it('prioritizes direct side-to-side overshoot over landmark snap feedback', async () => {
    const host = mount(EditorContextBarTestRoot, { target: document.body });
    mounted = host;
    await tick();

    host.setZoom(0.15, 0.2, 'minimum');
    await tick();
    host.setZoom(2.1, 2, 'maximum');
    await tick();

    expect(document.querySelector('[aria-live="polite"]')?.textContent).toContain('최대');
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBeNull();
  });

  it('uses recovery precedence when overshoot returns directly to another landmark', async () => {
    vi.useFakeTimers();
    const host = mount(EditorContextBarTestRoot, { target: document.body });
    mounted = host;
    await tick();

    host.setZoom(0.15, 0.2, 'minimum');
    await vi.advanceTimersByTimeAsync(1000);
    host.setZoom(1, 1, 'unit');
    await tick();

    const feedback = document.querySelector<HTMLElement>('[data-zoom-snap-feedback]');
    expect(feedback?.dataset.zoomSnapLandmark).toBe('minimum');
    expect(document.querySelector('[aria-live="polite"]')?.textContent).toContain('최소');
  });
});
