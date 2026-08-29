import '../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ZoomOverlayTestRoot from './zoom-overlay-test-root.svelte';
import ZoomOverlay from './ZoomOverlay.svelte';
import type { Component } from 'svelte';
import type { DocumentZoomLandmark } from '$lib/editor-ffi/zoom';

type TestProps = {
  enabled: boolean;
  displayZoom: number;
  indicatorZoom: number;
  landmark: DocumentZoomLandmark | null;
  atMinimum: boolean;
  atMaximum: boolean;
  toggleTargetLandmark: DocumentZoomLandmark | null;
  scrollContainer: HTMLElement;
  editorViewSurface: HTMLElement;
  onZoomOut: () => void;
  onZoomIn: () => void;
  onToggleZoom: () => Promise<unknown>;
};

const TestableZoomOverlay = ZoomOverlay as unknown as Component<TestProps>;
const TRANSIENT_VISIBLE_MS = 1000;
const HOVER_INTENT_SETTLE_MS = 100;
let mounted: Record<string, unknown> | undefined;

const enter = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerenter'));
const leave = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerleave'));

async function mountOverlay(
  {
    displayZoom = 1,
    indicatorZoom = displayZoom,
    landmark = null,
    atMinimum = false,
    atMaximum = false,
    toggleTargetLandmark = landmark === 'unit' ? 'fit-width' : 'unit',
    onZoomOut = () => null,
    onZoomIn = () => null,
    onToggleZoom = async () => null,
  }: Partial<TestProps> = {},
  { withinPane = true }: { withinPane?: boolean } = {},
) {
  const scrollContainer = document.createElement('div');
  scrollContainer.dataset.testid = 'editor-pane';
  scrollContainer.getBoundingClientRect = () => new DOMRect(40, 30, 300, 200);
  const header = document.createElement('div');
  header.dataset.testid = 'editor-header';
  const editorViewSurface = document.createElement('div');
  editorViewSurface.style.position = 'relative';
  editorViewSurface.style.width = '300px';
  editorViewSurface.append(header, scrollContainer);
  const paneContainer = document.createElement('div');
  paneContainer.dataset.paneId = 'pane-1';
  const toolbar = document.createElement('div');
  toolbar.dataset.testid = 'editor-toolbar';
  if (withinPane) {
    paneContainer.append(toolbar, editorViewSurface);
    document.body.append(paneContainer);
  } else {
    document.body.append(editorViewSurface);
  }
  mounted = mount(TestableZoomOverlay, {
    target: scrollContainer,
    props: {
      enabled: true,
      displayZoom,
      indicatorZoom,
      landmark,
      atMinimum,
      atMaximum,
      toggleTargetLandmark,
      scrollContainer,
      editorViewSurface,
      onZoomOut,
      onZoomIn,
      onToggleZoom,
    },
  });
  await tick();
  const overlay = document.querySelector<HTMLElement>('[role="group"][aria-label="페이지 배율"]');
  if (!overlay) throw new Error('Missing zoom overlay');
  return { overlay, paneContainer, scrollContainer, editorViewSurface, toolbar };
}

async function expireTransientVisibility() {
  await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS);
  await tick();
}

afterEach(async () => {
  if (mounted) await unmount(mounted);
  mounted = undefined;
  vi.useRealTimers();
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

describe('zoom overlay visibility', () => {
  it('keeps an initial unit zoom hidden while revealing an initial non-unit zoom', async () => {
    const unit = await mountOverlay({ displayZoom: 1, indicatorZoom: 1, landmark: 'unit' });
    expect(unit.overlay.style.opacity).toBe('0');

    await unmount(mounted as Record<string, unknown>);
    mounted = undefined;
    document.body.replaceChildren();

    const nonUnit = await mountOverlay({ displayZoom: 1.25, indicatorZoom: 1.25, landmark: null });
    expect(nonUnit.overlay.style.opacity).toBe('1');
  });

  it('keeps unit zoom hidden when the editor enables after its initial layout arrives', async () => {
    const host = mount(ZoomOverlayTestRoot, {
      target: document.body,
      props: { initialEnabled: false, initialZoom: 1, initialLandmark: null },
    });
    mounted = host;
    await tick();

    enter(document.querySelector<HTMLElement>('[data-pane-id="zoom-overlay-test-pane"]') as HTMLElement);
    host.setZoom(1, 1, 'unit');
    host.setEnabled(true);
    await tick();

    expect(document.querySelector<HTMLElement>('[role="group"][aria-label="페이지 배율"]')?.style.opacity).toBe('0');
  });

  it('uses the unzoomed editor surface coordinate system', async () => {
    const { overlay, editorViewSurface } = await mountOverlay();
    const anchor = overlay.parentElement;

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
    const hitRegion = overlay.parentElement as HTMLElement;

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
    const hitRegion = overlay.parentElement as HTMLElement;

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

describe('zoom overlay controls', () => {
  it('reveals on the first hidden touch and activates only on the next touch', async () => {
    const onToggleZoom = vi.fn(async () => null);
    const { overlay } = await mountOverlay({ landmark: 'unit', onToggleZoom });
    const value = document.querySelector<HTMLButtonElement>('[aria-label="화면에 맞추기"]');

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

describe('zoom overlay snap feedback', () => {
  it('starts once on applied landmark entry and restarts only after leaving it', async () => {
    const host = mount(ZoomOverlayTestRoot, { target: document.body });
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
    const host = mount(ZoomOverlayTestRoot, { target: document.body, props: { initialZoom: 1, initialLandmark: 'unit' } });
    mounted = host;
    await tick();
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBeNull();

    host.setZoom(2, 2, 'maximum');
    await tick();
    expect(document.querySelector<HTMLElement>('[data-zoom-snap-feedback]')?.dataset.zoomSnapLandmark).toBe('maximum');
  });

  it('restarts only the hidden overlay and boundary label for an unchanged minimum attempt', async () => {
    vi.useFakeTimers();
    const host = mount(ZoomOverlayTestRoot, {
      target: document.body,
      props: { initialZoom: 0.2, initialIndicatorZoom: 0.2, initialLandmark: 'minimum' },
    });
    mounted = host;
    await tick();
    const overlay = document.querySelector<HTMLElement>('[role="group"][aria-label="페이지 배율"]');
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
    const host = mount(ZoomOverlayTestRoot, { target: document.body });
    mounted = host;
    await tick();

    host.setZoom(0.15, 0.2, 'minimum');
    await tick();
    expect(document.querySelector('[aria-live="polite"]')?.textContent).toContain('최소');
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBeNull();

    await vi.advanceTimersByTimeAsync(2900);
    host.setZoom(0.14, 0.2, 'minimum');
    await vi.advanceTimersByTimeAsync(220);
    expect(document.querySelector<HTMLElement>('[role="group"][aria-label="페이지 배율"]')?.style.opacity).toBe('1');
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
    const host = mount(ZoomOverlayTestRoot, { target: document.body });
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
    const host = mount(ZoomOverlayTestRoot, { target: document.body });
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
    const host = mount(ZoomOverlayTestRoot, { target: document.body });
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
