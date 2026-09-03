import '../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import EditorZoomControlsTestRoot from './editor-zoom-controls-test-root.svelte';
import type { DocumentZoomLandmark } from '$lib/editor-ffi/zoom';

type TestProps = {
  displayZoom: number;
  indicatorZoom: number;
  landmark: DocumentZoomLandmark | null;
  onZoomOut: () => unknown;
  onToggleZoom: () => unknown | Promise<unknown>;
  fixed: boolean;
  revealOnHover: boolean;
  requiresChrome: boolean;
  topInset: number;
};

const TRANSIENT_VISIBLE_MS = 1500;
let mounted: Record<string, unknown> | undefined;

const enter = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerenter'));
const leave = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerleave'));
const move = (element: HTMLElement, clientX: number, clientY: number) =>
  element.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX, clientY, pointerType: 'mouse' }));

async function mountControls({
  displayZoom = 1,
  indicatorZoom = displayZoom,
  landmark = null,
  onZoomOut = () => null,
  onToggleZoom = () => null,
  fixed = true,
  revealOnHover = false,
  requiresChrome = false,
  topInset = 0,
}: Partial<TestProps> = {}) {
  mounted = mount(EditorZoomControlsTestRoot, {
    target: document.body,
    props: {
      initialZoom: displayZoom,
      initialIndicatorZoom: indicatorZoom,
      initialLandmark: landmark,
      onZoomOut,
      onToggleZoom,
      fixed,
      revealOnHover,
      requiresChrome,
      topInset,
    },
  });
  await tick();

  const anchor = document.querySelector<HTMLElement>('[data-floating-editor-zoom-anchor]');
  const controls = document.querySelector<HTMLElement>('[data-floating-editor-zoom-controls]');
  const pane = document.querySelector<HTMLElement>('[data-pane-id="zoom-controls-test-pane"]');
  if (!anchor || !controls || !pane) throw new Error('Missing floating editor zoom fixture');
  return { anchor, controls, pane, host: mounted };
}

afterEach(async () => {
  if (mounted) await unmount(mounted);
  mounted = undefined;
  vi.useRealTimers();
  vi.restoreAllMocks();
  document.body.style.removeProperty('--usersite-sticky-header-bottom');
  document.body.replaceChildren();
});

describe('floating editor zoom controls', () => {
  it('reveals after zoom activity and hides after the transient window', async () => {
    vi.useFakeTimers();
    const { controls, host } = await mountControls({ landmark: 'unit' });
    expect(controls.style.opacity).toBe('0');

    (host as typeof mounted & { setZoom(displayZoom: number, indicatorZoom: number, landmark: DocumentZoomLandmark | null): void }).setZoom(
      1.1,
      1.1,
      null,
    );
    await tick();
    expect(controls.style.opacity).toBe('1');

    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS);
    expect(controls.style.opacity).toBe('0');
  });

  it('remains visible while hovered and lingers after the pointer leaves', async () => {
    vi.useFakeTimers();
    const { controls } = await mountControls({ fixed: false, revealOnHover: true });

    enter(controls);
    await tick();
    expect(controls.style.opacity).toBe('1');

    leave(controls);
    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS - 1);
    expect(controls.style.opacity).toBe('1');

    await vi.advanceTimersByTimeAsync(1);
    expect(controls.style.opacity).toBe('0');
  });

  it('is keyboard discoverable while hidden and lingers after focus leaves', async () => {
    vi.useFakeTimers();
    const { controls } = await mountControls();
    const zoomOut = controls.querySelector<HTMLButtonElement>('[aria-label="페이지 축소"]');

    expect(zoomOut?.tabIndex).toBe(0);
    zoomOut?.focus();
    await tick();
    expect(controls.style.opacity).toBe('1');

    zoomOut?.blur();
    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS);
    expect(controls.style.opacity).toBe('0');
  });

  it('becomes hoverable when pane chrome exposes its footprint under a stationary pointer', async () => {
    const { controls, pane, host } = await mountControls({ fixed: false, revealOnHover: true, requiresChrome: true, topInset: 78 });
    const rect = controls.getBoundingClientRect();

    expect(controls.style.pointerEvents).toBe('none');
    move(pane, rect.left + rect.width / 2, rect.top + rect.height / 2);
    (host as typeof mounted & { setChromeReady(ready: boolean): void }).setChromeReady(true);
    await tick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await tick();

    expect(controls.style.opacity).toBe('1');
    expect(controls.style.pointerEvents).toBe('auto');
  });

  it('uses the viewer header offset without enabling hover reveal', async () => {
    document.body.style.setProperty('--usersite-sticky-header-bottom', '52px');
    const { anchor, controls } = await mountControls();

    expect(getComputedStyle(anchor).position).toBe('fixed');
    expect(getComputedStyle(anchor).top).toBe('52px');
    enter(controls);
    await tick();
    expect(controls.style.opacity).toBe('0');
  });
});

describe('editor zoom controls', () => {
  it('reveals on the first hidden touch and activates only on the next touch', async () => {
    const onToggleZoom = vi.fn(() => null);
    const { controls } = await mountControls({ landmark: 'unit', onToggleZoom });
    const value = controls.querySelector<HTMLButtonElement>('[aria-label="화면에 맞추기"]');

    value?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, pointerType: 'touch' }));
    value?.click();
    await tick();
    expect(controls.style.opacity).toBe('1');
    expect(onToggleZoom).not.toHaveBeenCalled();

    value?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, pointerType: 'touch' }));
    value?.click();
    await tick();
    expect(onToggleZoom).toHaveBeenCalledOnce();
  });

  it('keeps a boundary control actionable with explanatory semantics', async () => {
    const onZoomOut = vi.fn(() => null);
    const { controls } = await mountControls({ displayZoom: 0.2, indicatorZoom: 0.2, landmark: 'minimum', onZoomOut });
    const zoomOut = controls.querySelector<HTMLButtonElement>('[data-at-zoom-boundary="true"]');

    expect(zoomOut?.ariaLabel).toBe('최소 배율입니다');
    zoomOut?.click();
    await tick();
    expect(onZoomOut).toHaveBeenCalledOnce();
  });

  it('shows the current landmark while the value is hovered', async () => {
    const { controls } = await mountControls({ landmark: 'unit' });
    const value = controls.querySelector<HTMLButtonElement>('[aria-label="화면에 맞추기"]');

    enter(value as HTMLButtonElement);
    await tick();
    expect(value?.textContent).toContain('원본');
  });

  it('uses the applied landmark for snap feedback', async () => {
    const { host } = await mountControls({ landmark: 'unit' });
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBeNull();

    (host as typeof mounted & { setZoom(displayZoom: number, indicatorZoom: number, landmark: DocumentZoomLandmark | null): void }).setZoom(
      2,
      2,
      'maximum',
    );
    await tick();

    expect(document.querySelector<HTMLElement>('[data-zoom-snap-feedback]')?.dataset.zoomSnapLandmark).toBe('maximum');
  });

  it('holds overshoot feedback until recovery and then uses the extended transient window', async () => {
    vi.useFakeTimers();
    const { controls, host } = await mountControls();
    const fixture = host as typeof mounted & {
      setZoom(displayZoom: number, indicatorZoom: number, landmark: DocumentZoomLandmark | null): void;
    };

    fixture.setZoom(0.15, 0.2, 'minimum');
    await tick();
    await vi.advanceTimersByTimeAsync(5000);
    expect(controls.style.opacity).toBe('1');

    fixture.setZoom(0.2, 0.2, 'minimum');
    await tick();
    await vi.advanceTimersByTimeAsync(TRANSIENT_VISIBLE_MS + 999);
    expect(controls.style.opacity).toBe('1');

    await vi.advanceTimersByTimeAsync(1);
    expect(controls.style.opacity).toBe('0');
  });

  it('reveals an unchanged boundary attempt without replaying snap feedback', async () => {
    const { controls, host } = await mountControls({ displayZoom: 0.2, indicatorZoom: 0.2, landmark: 'minimum' });

    (host as typeof mounted & { requestBoundaryAttempt(landmark: DocumentZoomLandmark): void }).requestBoundaryAttempt('minimum');
    await tick();

    expect(controls.style.opacity).toBe('1');
    expect(document.querySelector('[aria-live="polite"]')?.textContent).toContain('최소');
    expect(document.querySelector('[data-zoom-snap-feedback]')).toBeNull();
  });
});
