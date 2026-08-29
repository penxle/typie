import '../app.css';

import { css } from '@typie/styled-system/css';
import { Scrollbar } from '@typie/ui/components';
import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

type Orientation = 'horizontal' | 'vertical';
type Size = 'sm' | 'md';

let components: ReturnType<typeof mount>[] = [];

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

const pointer = (target: HTMLElement, type: string, init: PointerEventInit) => {
  target.dispatchEvent(
    new PointerEvent(type, {
      pointerId: 1,
      pointerType: 'mouse',
      isPrimary: true,
      bubbles: true,
      cancelable: true,
      ...init,
    }),
  );
};

const drag = (target: HTMLElement, deltaX: number, deltaY: number) => {
  const bounds = target.getBoundingClientRect();
  const startX = bounds.left + bounds.width / 2;
  const startY = bounds.top + bounds.height / 2;
  pointer(target, 'pointerdown', { button: 0, buttons: 1, clientX: startX, clientY: startY });
  pointer(target, 'pointermove', { buttons: 1, clientX: startX + deltaX, clientY: startY + deltaY });
  pointer(target, 'pointerup', { button: 0, clientX: startX + deltaX, clientY: startY + deltaY });
};

const mountFixture = async (orientation: Orientation, size: Size) => {
  const layout = {
    horizontal: {
      wrapper: 'width:240px;height:64px',
      overflow: 'overflow-x:auto;overflow-y:hidden',
      content: 'width:720px;height:100%',
    },
    vertical: {
      wrapper: 'width:120px;height:240px',
      overflow: 'overflow-x:hidden;overflow-y:auto',
      content: 'width:100%;height:720px',
    },
  }[orientation];

  const wrapper = document.createElement('div');
  wrapper.className = css({
    transitionProperty: '[border, border-radius, box-shadow]',
    transitionDuration: '150ms',
    transitionTimingFunction: 'ease',
  });
  wrapper.style.cssText = `position:relative;${layout.wrapper}`;
  wrapper.innerHTML = `<div id="${orientation}-scroll-container" style="width:100%;height:100%;scrollbar-width:none;${layout.overflow}"><div style="${layout.content}"></div></div>`;
  document.body.append(wrapper);

  const container = wrapper.firstElementChild;
  const content = container?.firstElementChild;
  if (!(container instanceof HTMLElement) || !(content instanceof HTMLElement)) throw new Error(`Missing ${orientation} fixture`);

  components.push(
    mount(Scrollbar, {
      target: wrapper,
      props: {
        controls: container.id,
        label: `${orientation} test scrollbar`,
        orientation,
        scrollContainer: container,
        size,
      },
    }),
  );
  await tick();

  await expect.poll(() => wrapper.querySelector('[role="scrollbar"]')).toBeInstanceOf(HTMLElement);
  const track = wrapper.querySelector('[role="scrollbar"]');
  if (!(track instanceof HTMLElement)) throw new Error(`Missing ${orientation} scrollbar`);

  const thumb = track.firstElementChild;
  const paint = thumb?.firstElementChild;
  if (!(thumb instanceof HTMLElement) || !(paint instanceof HTMLElement)) throw new Error(`Missing ${orientation} thumb`);

  return { container, content, paint, thumb, track, wrapper };
};

afterEach(async () => {
  for (const component of components) await unmount(component);
  components = [];
  document.body.replaceChildren();
});

describe('shared custom scrollbar', () => {
  it('maps sm horizontal and md vertical interactions to the correct axis', async () => {
    const horizontal = await mountFixture('horizontal', 'sm');
    const vertical = await mountFixture('vertical', 'md');

    expect(horizontal.track.getBoundingClientRect().height).toBeCloseTo(8, 0);
    expect(horizontal.paint.getBoundingClientRect().height).toBeCloseTo(3, 0);
    expect(vertical.track.getBoundingClientRect().width).toBeCloseTo(12, 0);
    expect(vertical.paint.getBoundingClientRect().width).toBeCloseTo(6, 0);

    expect(getComputedStyle(vertical.track).transitionProperty).toBe('opacity');
    expect(getComputedStyle(vertical.track).transitionTimingFunction).toBe('cubic-bezier(0.4, 0, 0.2, 1)');

    expect(horizontal.track.style.opacity).toBe('0');
    pointer(horizontal.container, 'pointerenter', {});
    await tick();
    expect(horizontal.track.style.opacity).toBe('1');
    pointer(horizontal.container, 'pointerleave', {});
    await tick();
    expect(horizontal.track.style.opacity).toBe('0');

    const horizontalTrackBounds = horizontal.track.getBoundingClientRect();
    pointer(horizontal.track, 'pointerdown', {
      button: 0,
      buttons: 1,
      clientX: horizontalTrackBounds.left + horizontalTrackBounds.width * 0.75,
      clientY: horizontalTrackBounds.top + horizontalTrackBounds.height / 2,
    });
    await frame();
    expect(horizontal.container.scrollLeft).toBeGreaterThan(0);
    expect(horizontal.container.scrollTop).toBe(0);

    drag(vertical.thumb, 0, 40);
    await frame();
    expect(vertical.container.scrollTop).toBeGreaterThan(0);
    expect(vertical.container.scrollLeft).toBe(0);
  });

  it('removes the control when its container no longer overflows', async () => {
    const fixture = await mountFixture('vertical', 'md');

    fixture.content.style.height = '100%';
    await expect.poll(() => fixture.wrapper.querySelector('[role="scrollbar"]')).toBeNull();
  });
});
