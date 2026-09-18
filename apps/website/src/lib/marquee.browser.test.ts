import '../app.css';

import { Marquee } from '@typie/ui/components';
import { mount, tick, unmount } from 'svelte';
import { fromStore, writable } from 'svelte/store';
import { afterEach, describe, expect, it } from 'vitest';
import { userEvent } from 'vitest/browser';

let component: Record<string, unknown> | undefined;

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

const frames = async (count = 2) => {
  for (let index = 0; index < count; index++) await frame();
};

const pointer = (element: HTMLElement, type: 'pointerenter' | 'pointerleave') => {
  element.dispatchEvent(new PointerEvent(type, { pointerType: 'mouse' }));
};

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

describe('shared marquee', () => {
  it('preserves its scroll position across parent snapshots unless the displayed text changes', async () => {
    const target = document.createElement('div');
    target.style.width = '240px';
    document.body.append(target);
    const text = '긴 문서 제목을 읽는 동안 다른 상태가 바뀌더라도 읽던 위치가 처음으로 돌아가면 안 돼요';
    const snapshot = fromStore(writable({ text, revision: 0 }));

    component = mount(Marquee, {
      target,
      props: {
        get text() {
          return snapshot.current.text;
        },
      },
    });
    await tick();
    await document.fonts.ready;
    await frames();

    const viewport = target.firstElementChild;
    expect(viewport).toBeInstanceOf(HTMLSpanElement);
    if (!(viewport instanceof HTMLSpanElement)) throw new Error('Marquee viewport missing');

    pointer(viewport, 'pointerenter');
    await expect.poll(() => viewport.scrollLeft, { timeout: 3000 }).toBeGreaterThan(20);
    const position = viewport.scrollLeft;

    snapshot.current = { text, revision: 1 };
    await tick();
    expect(viewport.isConnected).toBe(true);
    expect(viewport.scrollLeft).toBeGreaterThanOrEqual(position);

    snapshot.current = { text: `${text} — 수정된 제목`, revision: 2 };
    await tick();
    expect(viewport.textContent).toBe(`${text} — 수정된 제목`);
    expect(viewport.scrollLeft).toBe(0);
  });

  it('reveals overflowing text from keyboard focus or parent hover and resets afterward', async () => {
    const before = document.createElement('button');
    const button = document.createElement('button');
    const after = document.createElement('button');
    Object.assign(button.style, { display: 'block', padding: '0', width: '240px' });
    document.body.append(before, button, after);

    component = mount(Marquee, {
      target: button,
      props: {
        getTrigger: (element) => element.parentElement,
        text: 'Reorder mutation 동기화 구조 검토 및 후속 작업 정리',
      },
    });
    await tick();
    await document.fonts.ready;
    await frames();

    const viewport = button.firstElementChild;
    expect(viewport).toBeInstanceOf(HTMLSpanElement);
    if (!(viewport instanceof HTMLSpanElement)) return;

    before.focus();
    await userEvent.keyboard('{Tab}');
    expect(document.activeElement).toBe(button);
    expect(button.matches(':focus-visible')).toBe(true);
    await expect.poll(() => viewport.scrollLeft, { interval: 16, timeout: 350 }).toBeGreaterThan(0);

    await userEvent.keyboard('{Tab}');
    expect(document.activeElement).toBe(after);
    expect(viewport.scrollLeft).toBe(0);

    pointer(button, 'pointerenter');
    await expect.poll(() => viewport.scrollLeft, { timeout: 2500 }).toBeGreaterThan(0);

    pointer(button, 'pointerleave');
    expect(viewport.scrollLeft).toBe(0);
  });

  it('bleeds its viewport without moving the resting text', async () => {
    const button = document.createElement('button');
    Object.assign(button.style, { border: '0', boxSizing: 'border-box', display: 'flex', padding: '0 8px', width: '240px' });
    document.body.append(button);

    component = mount(Marquee, {
      target: button,
      props: {
        bleed: { start: 8, end: 4 },
        text: 'Reorder mutation 동기화 구조 검토 및 후속 작업 정리',
      },
    });
    await tick();
    await document.fonts.ready;
    await frames();

    const viewport = button.firstElementChild;
    const content = viewport?.firstElementChild;
    expect(viewport).toBeInstanceOf(HTMLSpanElement);
    expect(content).toBeInstanceOf(HTMLSpanElement);
    if (!(viewport instanceof HTMLSpanElement) || !(content instanceof HTMLSpanElement)) return;

    const buttonRect = button.getBoundingClientRect();
    const viewportRect = viewport.getBoundingClientRect();
    const contentRect = content.getBoundingClientRect();
    expect(viewportRect.left).toBeCloseTo(buttonRect.left, 0);
    expect(viewportRect.right).toBeCloseTo(buttonRect.right - 4, 0);
    expect(contentRect.left).toBeCloseTo(buttonRect.left + 8, 0);
  });

  it('clips without rendering a legacy ellipsis', async () => {
    const style = document.createElement('style');
    style.textContent = '.legacy-ellipsis { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }';
    const target = document.createElement('div');
    target.style.width = '240px';
    document.body.append(style, target);

    component = mount(Marquee, {
      target,
      props: {
        class: 'legacy-ellipsis',
        text: 'Reorder mutation 동기화 구조 검토 및 후속 작업 정리',
      },
    });
    await tick();
    await frames();

    const viewport = target.firstElementChild;
    expect(viewport).toBeInstanceOf(HTMLSpanElement);
    if (!(viewport instanceof HTMLSpanElement)) return;

    expect(getComputedStyle(viewport).textOverflow).toBe('clip');
  });
});
