import '../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import EntityName from './EntityName.svelte';

let component: Record<string, unknown> | undefined;

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

const frames = async (count = 2) => {
  for (let index = 0; index < count; index++) await frame();
};

const wait = (duration: number) => new Promise<void>((resolve) => setTimeout(resolve, duration));
const maskAlpha = (element: HTMLElement, edge: 'leading' | 'trailing') =>
  Number(getComputedStyle(element).getPropertyValue(`--entity-name-${edge}-mask-alpha`));

const mockReducedMotion = (reducedMotion: boolean) => {
  const eventTarget = new EventTarget();
  const query = '(prefers-reduced-motion: reduce)';
  const mediaQuery = {
    get matches() {
      return reducedMotion;
    },
    media: query,
    onchange: null,
    addEventListener: eventTarget.addEventListener.bind(eventTarget),
    removeEventListener: eventTarget.removeEventListener.bind(eventTarget),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: eventTarget.dispatchEvent.bind(eventTarget),
  } satisfies MediaQueryList;

  vi.spyOn(window, 'matchMedia').mockReturnValue(mediaQuery);

  return (next: boolean) => {
    reducedMotion = next;
    mediaQuery.dispatchEvent(new MediaQueryListEvent('change', { matches: next, media: query }));
  };
};

const mountName = async (name: string, width: number) => {
  const target = document.createElement('div');
  target.className = 'group';
  target.setAttribute('role', 'treeitem');
  Object.assign(target.style, { display: 'flex', width: `${width}px` });
  document.body.append(target);

  component = mount(EntityName, { target, props: { name } });
  await tick();
  await document.fonts.ready;
  await frames();

  const viewport = target.firstElementChild;
  const content = viewport?.firstElementChild;
  expect(viewport).toBeInstanceOf(HTMLSpanElement);
  expect(content).toBeInstanceOf(HTMLSpanElement);
  if (!(viewport instanceof HTMLSpanElement) || !(content instanceof HTMLSpanElement)) {
    throw new TypeError('EntityName did not render the expected viewport and content spans');
  }

  return { target, viewport, content };
};

const enter = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerenter'));
const leave = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerleave'));

afterEach(async () => {
  vi.useRealTimers();
  if (component) await unmount(component);
  component = undefined;
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

describe('entity tree name overflow', () => {
  it('bleeds into both adjacent gaps without shifting the resting text', async () => {
    mockReducedMotion(false);
    const { target, viewport, content } = await mountName('간격까지 이어지는 이름', 320);

    const targetBounds = target.getBoundingClientRect();
    const viewportBounds = viewport.getBoundingClientRect();
    const contentBounds = content.getBoundingClientRect();
    const style = getComputedStyle(viewport);

    expect(style.marginLeft).toBe('-6px');
    expect(style.marginRight).toBe('-6px');
    expect(style.paddingLeft).toBe('6px');
    expect(style.paddingRight).toBe('6px');
    expect(viewportBounds.left).toBeCloseTo(targetBounds.left - 6, 0);
    expect(viewportBounds.right).toBeCloseTo(targetBounds.right + 6, 0);
    expect(contentBounds.left).toBeCloseTo(targetBounds.left, 0);
  });

  it('leaves a fitting name still and unfogged', async () => {
    mockReducedMotion(false);
    const { target, viewport } = await mountName('짧은 이름', 320);

    expect(viewport.scrollWidth).toBeLessThanOrEqual(viewport.clientWidth);
    expect(viewport.style.maskImage).toContain('24px');
    expect(maskAlpha(viewport, 'leading')).toBe(1);
    expect(maskAlpha(viewport, 'trailing')).toBe(1);

    enter(target);
    await wait(400);

    expect(viewport.scrollLeft).toBe(0);
    expect(maskAlpha(viewport, 'leading')).toBe(1);
    expect(maskAlpha(viewport, 'trailing')).toBe(1);
  });

  it('waits for the first usable measurement before enabling fog transitions', async () => {
    mockReducedMotion(false);
    const target = document.createElement('div');
    target.className = 'group';
    target.setAttribute('role', 'treeitem');
    Object.assign(target.style, { display: 'none', width: '120px' });
    document.body.append(target);

    component = mount(EntityName, { target, props: { name: '처음 열릴 때부터 긴 폴더 이름' } });
    await tick();
    await frames(3);

    const viewport = target.firstElementChild;
    expect(viewport).toBeInstanceOf(HTMLSpanElement);
    if (!(viewport instanceof HTMLSpanElement)) throw new Error('EntityName did not render its viewport span');

    expect(viewport.clientWidth).toBe(0);
    expect(viewport.style.transition).toBe('none');

    target.style.display = 'flex';
    await vi.waitFor(() => expect(viewport.clientWidth).toBeGreaterThan(0));
    await frames(3);

    expect(viewport.style.transition).toContain('--entity-name-leading-mask-alpha');
  });

  it('scrolls an overflowing name to the end once and resets when the pointer leaves', async () => {
    mockReducedMotion(false);
    const { target, viewport, content } = await mountName('Reorder mutation 동기화 구조 검토 및 후속 작업 정리', 360);

    target.style.width = `${content.scrollWidth - 24}px`;
    await frames();

    const initialMaximumScrollLeft = viewport.scrollWidth - viewport.clientWidth;
    expect(initialMaximumScrollLeft).toBeGreaterThanOrEqual(20);
    expect(initialMaximumScrollLeft).toBeLessThanOrEqual(28);
    expect(viewport.scrollLeft).toBe(0);
    expect(viewport.style.maskImage).toContain('24px');
    expect(maskAlpha(viewport, 'leading')).toBe(1);
    await vi.waitFor(() => expect(maskAlpha(viewport, 'trailing')).toBeCloseTo(0, 1));

    enter(target);
    await wait(100);
    target.style.width = `${content.scrollWidth - 28}px`;
    await frames();
    const maximumScrollLeft = viewport.scrollWidth - viewport.clientWidth;
    expect(maximumScrollLeft).toBeGreaterThanOrEqual(24);
    expect(maximumScrollLeft).toBeLessThanOrEqual(32);

    await wait(100);
    expect(viewport.scrollLeft).toBe(0);

    await vi.waitFor(() => expect(viewport.scrollLeft).toBeCloseTo(maximumScrollLeft, 0), { timeout: 2500 });
    await vi.waitFor(() => expect(maskAlpha(viewport, 'leading')).toBeCloseTo(0, 1));
    await vi.waitFor(() => expect(maskAlpha(viewport, 'trailing')).toBeCloseTo(1, 1));

    const settledScrollLeft = viewport.scrollLeft;
    await wait(150);
    expect(viewport.scrollLeft).toBe(settledScrollLeft);

    leave(target);
    expect(viewport.scrollLeft).toBe(0);
    await vi.waitFor(() => expect(maskAlpha(viewport, 'leading')).toBeCloseTo(1, 1));
    await vi.waitFor(() => expect(maskAlpha(viewport, 'trailing')).toBeCloseTo(0, 1));
  });

  it('keeps the original hover-intent deadline when its width changes', async () => {
    mockReducedMotion(true);
    const { target, viewport, content } = await mountName('Reorder mutation 동기화 구조 검토 및 후속 작업 정리', 360);

    target.style.width = `${content.scrollWidth - 24}px`;
    await frames();

    enter(target);
    await wait(250);

    target.style.width = `${content.scrollWidth - 40}px`;
    await frames();
    const resizedMaximumScrollLeft = viewport.scrollWidth - viewport.clientWidth;

    await wait(220);
    expect(viewport.scrollLeft).toBeCloseTo(resizedMaximumScrollLeft, 0);
  });

  it('preserves active progress and follows a resized endpoint without another delay', async () => {
    mockReducedMotion(false);
    const { target, viewport, content } = await mountName('Reorder mutation 동기화 구조 검토 및 후속 작업 정리', 360);

    target.style.width = `${content.scrollWidth - 80}px`;
    await frames();

    enter(target);
    await vi.waitFor(() => expect(viewport.scrollLeft).toBeGreaterThan(5), { timeout: 1500 });
    const positionBeforeResize = viewport.scrollLeft;

    target.style.width = `${content.scrollWidth - 110}px`;
    await frames(3);
    expect(viewport.scrollLeft).toBeGreaterThanOrEqual(positionBeforeResize - 0.5);

    let resizedMaximumScrollLeft = viewport.scrollWidth - viewport.clientWidth;
    await vi.waitFor(() => expect(viewport.scrollLeft).toBeCloseTo(resizedMaximumScrollLeft, 0), { timeout: 4000 });

    const settledPosition = viewport.scrollLeft;
    target.style.width = `${content.scrollWidth - 125}px`;
    await frames(3);
    resizedMaximumScrollLeft = viewport.scrollWidth - viewport.clientWidth;

    await wait(200);
    expect(viewport.scrollLeft).toBeGreaterThan(settledPosition);
    await vi.waitFor(() => expect(viewport.scrollLeft).toBeCloseTo(resizedMaximumScrollLeft, 0), { timeout: 1500 });
  });

  it('transitions the fixed-width fog as content enters and leaves each edge', async () => {
    mockReducedMotion(false);
    const { target, viewport, content } = await mountName('Reorder mutation 동기화 구조 검토 및 후속 작업 정리', 360);

    target.style.width = `${content.scrollWidth - 40}px`;
    await frames();
    await wait(220);

    expect(maskAlpha(viewport, 'leading')).toBe(1);
    expect(maskAlpha(viewport, 'trailing')).toBe(0);

    viewport.scrollLeft = 12;
    viewport.dispatchEvent(new Event('scroll'));
    await tick();
    await wait(80);
    expect(maskAlpha(viewport, 'leading')).toBeGreaterThan(0);
    expect(maskAlpha(viewport, 'leading')).toBeLessThan(1);
    await wait(120);
    expect(maskAlpha(viewport, 'leading')).toBeCloseTo(0, 1);

    viewport.scrollLeft = viewport.scrollWidth - viewport.clientWidth;
    viewport.dispatchEvent(new Event('scroll'));
    await tick();
    await wait(80);
    expect(maskAlpha(viewport, 'trailing')).toBeGreaterThan(0);
    expect(maskAlpha(viewport, 'trailing')).toBeLessThan(1);
    await wait(120);
    expect(maskAlpha(viewport, 'trailing')).toBeCloseTo(1, 1);
  });

  it('reveals the end directly when reduced motion is requested', async () => {
    mockReducedMotion(true);
    const { target, viewport, content } = await mountName('Reorder mutation 동기화 구조 검토 및 후속 작업 정리', 360);

    target.style.width = `${content.scrollWidth - 24}px`;
    await frames();

    const maximumScrollLeft = viewport.scrollWidth - viewport.clientWidth;
    expect(viewport.style.transition).toBe('none');

    vi.useFakeTimers();
    enter(target);
    await vi.advanceTimersByTimeAsync(399);
    expect(viewport.scrollLeft).toBe(0);

    await vi.advanceTimersByTimeAsync(1);
    expect(viewport.scrollLeft).toBeCloseTo(maximumScrollLeft, 0);
    expect(maskAlpha(viewport, 'leading')).toBe(0);
    expect(maskAlpha(viewport, 'trailing')).toBe(1);
  });

  it('finishes an active pass immediately when reduced motion is enabled', async () => {
    const setReducedMotion = mockReducedMotion(false);
    const { target, viewport, content } = await mountName('Reorder mutation 동기화 구조 검토 및 후속 작업 정리', 360);

    target.style.width = `${content.scrollWidth - 80}px`;
    await frames();

    const maximumScrollLeft = viewport.scrollWidth - viewport.clientWidth;
    enter(target);
    await vi.waitFor(() => {
      expect(viewport.scrollLeft).toBeGreaterThan(0);
      expect(viewport.scrollLeft).toBeLessThan(maximumScrollLeft);
    });

    setReducedMotion(true);
    await tick();
    await frames();

    expect(viewport.scrollLeft).toBeCloseTo(maximumScrollLeft, 0);
    expect(viewport.style.transition).toBe('none');
    expect(maskAlpha(viewport, 'leading')).toBe(0);
    expect(maskAlpha(viewport, 'trailing')).toBe(1);
  });
});
