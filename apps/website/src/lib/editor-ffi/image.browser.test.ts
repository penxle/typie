import '../../app.css';

import { EDITOR_FFI_ROOT_ID } from '@typie/lib/const';
import { mount, tick, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { initWasm } from '$lib/wasm-ffi.svelte';
import { Editor } from './editor.svelte';
import EditorFrameSyncTestHost from './editor-frame-sync-test-host.svelte';
import type { PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/browser';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));

const entry = (node: PlainNode, children: PlainNodeEntry[] = []): PlainNodeEntry => ({
  node,
  modifiers: {} as never,
  carry: [],
  children,
});

let mounted: Record<string, unknown> | undefined;
let editor: Editor | undefined;

afterEach(async () => {
  if (mounted) await unmount(mounted);
  mounted = undefined;
  editor?.destroy();
  editor = undefined;
  window.scrollTo(0, 0);
  document.body.replaceChildren();
});

async function mountImage(proportion = 100, originalHeight = 3000, { useWindowScroll = false } = {}) {
  await initWasm();
  const doc: PlainDoc = {
    root: entry(
      {
        type: 'root',
        layout_mode: {
          type: 'paginated',
          page_width: 800,
          page_height: 1000,
          page_margin_top: 100,
          page_margin_bottom: 100,
          page_margin_left: 100,
          page_margin_right: 100,
        },
      },
      [
        entry({ type: 'paragraph' }, [entry({ type: 'text', text: 'Before the image' })]),
        entry({ type: 'image', id: 'asset', proportion }),
      ],
    ),
  };
  editor = await Editor.createFromDoc(doc, { width: 800, height: 1000, scale_factor: 1 });
  const url = `data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="600" height="${originalHeight}"/>`;
  editor.imageAssets.set('asset', { id: 'asset', url, originalUrl: url, width: 600, height: originalHeight, placeholder: '' });
  const target = document.createElement('div');
  document.body.append(target);
  let scrollRoot: HTMLDivElement | undefined;
  mounted = mount(EditorFrameSyncTestHost, {
    target,
    props: {
      editor,
      useWindowScroll,
      userId: `image-size-${crypto.randomUUID()}`,
      onReady: (harness) => {
        scrollRoot = harness.scrollRoot;
      },
    },
  });
  await tick();
  return { editor, scrollRoot };
}

it('fits a tall image on the next page and restores its requested size in continuous mode', async () => {
  const { editor, scrollRoot } = await mountImage();

  await vi.waitFor(() => {
    const elements = editor?.appliedSnapshot.externalElements;
    expect(elements).toHaveLength(1);
    expect(elements?.[0]?.page_idx).toBe(1);
    expect(elements?.[0]?.bounds.height).toBeCloseTo(800);
    expect(elements?.[0]?.data).toMatchObject({ type: 'image', proportion: 100, max_height: 800 });
  });
  scrollRoot?.scrollTo({ top: 1100 });
  await vi.waitFor(() => {
    const image = document.querySelector('img[alt="본문 이미지"]');
    expect(image?.getBoundingClientRect().width).toBeCloseTo(160);
    expect(image?.getBoundingClientRect().height).toBeCloseTo(800);
  });

  editor.updateNow((request) =>
    request.enqueue({
      type: 'node',
      op: {
        type: 'set_attrs',
        id: EDITOR_FFI_ROOT_ID,
        attrs: { type: 'root', layout_mode: { type: 'continuous', max_width: 800 } },
      },
    }),
  );
  scrollRoot?.scrollTo({ top: 0 });

  await vi.waitFor(() => {
    const element = editor?.appliedSnapshot.externalElements[0];
    expect(element?.bounds.height).toBeCloseTo(3000);
    expect(element?.data).toMatchObject({ type: 'image', proportion: 100 });
    expect(element?.data.type === 'image' && element.data.max_height).toBeUndefined();
    const image = document.querySelector('img[alt="본문 이미지"]');
    expect(image?.getBoundingClientRect().width).toBeCloseTo(600);
    expect(image?.getBoundingClientRect().height).toBeCloseTo(3000);
  });
});

it.each(['left', 'right'] as const)('keeps the active %s handle across page moves and compact sizing', async (side) => {
  const { editor } = await mountImage(50);
  await vi.waitFor(() => {
    expect(editor.published?.snapshot.externalElements[0]?.page_idx).toBe(0);
    expect(document.querySelector('img[alt="본문 이미지"]')?.getBoundingClientRect().height).toBeCloseTo(400);
  });
  const handle = document.querySelectorAll<HTMLElement>('[aria-label="이미지 크기 조절"]')[side === 'left' ? 0 : 1];
  expect(handle).toBeDefined();
  // Synthetic pointer events need a capture target; the real resize and page publication still run.
  handle.setPointerCapture = vi.fn();
  handle.hasPointerCapture = () => true;
  handle.releasePointerCapture = vi.fn();
  const pointer = (type: string, clientX: number, buttons = 1) => {
    handle.dispatchEvent(
      new PointerEvent(type, {
        pointerId: 1,
        pointerType: 'mouse',
        isPrimary: true,
        button: 0,
        buttons,
        clientX: side === 'left' ? 200 - clientX : clientX,
        clientY: 200,
        bubbles: true,
        cancelable: true,
      }),
    );
  };
  pointer('pointerdown', 100);
  pointer('pointermove', 140);
  await vi.waitFor(() => expect(editor.published?.snapshot.externalElements[0]?.page_idx).toBe(1));
  expect(handle.isConnected).toBe(true);
  pointer('pointermove', 120);
  await vi.waitFor(() => {
    expect(editor.published?.snapshot.externalElements[0]?.page_idx).toBe(0);
    expect(document.querySelector('img[alt="본문 이미지"]')?.getBoundingClientRect().height).toBeCloseTo(600);
  });
  pointer('pointermove', 68);
  await vi.waitFor(() => {
    expect(document.querySelector('img[alt="본문 이미지"]')?.getBoundingClientRect().width).toBeCloseTo(16);
    expect(document.querySelector('img[alt="본문 이미지"]')?.getBoundingClientRect().height).toBeCloseTo(80);
    expect(document.querySelectorAll('[aria-label="이미지 크기 조절"]')).toHaveLength(1);
  });
  expect(document.querySelector('[aria-label="이미지 크기 조절"]')).toBe(handle);
  const compactImage = document.querySelector('img[alt="본문 이미지"]')?.getBoundingClientRect();
  expect(handle.getBoundingClientRect()[side]).toBeCloseTo(compactImage?.[side] ?? NaN);
  pointer('pointerup', 68, 0);
  await vi.waitFor(() => expect(editor.published?.snapshot.externalElements[0]?.data).toMatchObject({ proportion: 10 }));
  const finalHandles = document.querySelectorAll('[aria-label="이미지 크기 조절"]');
  expect(finalHandles).toHaveLength(1);
  expect(finalHandles[0] === handle).toBe(side === 'right');
  expect(finalHandles[0].getBoundingClientRect().right).toBeCloseTo(compactImage?.right ?? NaN);
});

it('reports the actual height of images smaller than the upload placeholder', async () => {
  const { editor } = await mountImage(10, 60);
  await vi.waitFor(() => {
    expect(document.querySelector('img[alt="본문 이미지"]')?.getBoundingClientRect().height).toBeCloseTo(6);
    expect(editor.published?.snapshot.externalElements[0]?.bounds.height).toBeCloseTo(6);
  });
});

it('unmounts an idle image outside the rendered pages and mounts it again on return', async () => {
  const { editor } = await mountImage(50, 3000, { useWindowScroll: true });
  await vi.waitFor(() => expect(document.querySelector('img[alt="본문 이미지"]')).not.toBeNull());
  const image = document.querySelector('img[alt="본문 이미지"]');

  window.scrollTo(0, 3000);
  window.dispatchEvent(new Event('scroll'));
  await vi.waitFor(() => {
    expect(editor.published?.snapshot.pageData.has(0)).toBe(false);
    expect(image?.isConnected).toBe(false);
  });

  window.scrollTo(0, 0);
  window.dispatchEvent(new Event('scroll'));
  await vi.waitFor(() => {
    const remounted = document.querySelector('img[alt="본문 이미지"]');
    expect(remounted).not.toBe(image);
    expect(remounted?.getBoundingClientRect().height).toBeCloseTo(400);
  });
});

it.each(['pointerup', 'pointercancel'] as const)('keeps an offscreen resize until %s restores the final height', async (endEvent) => {
  const { editor } = await mountImage(50, 3000, { useWindowScroll: true });
  await vi.waitFor(() => expect(document.querySelector('img[alt="본문 이미지"]')?.getBoundingClientRect().height).toBeCloseTo(400));
  const handle = document.querySelectorAll<HTMLElement>('[aria-label="이미지 크기 조절"]')[1];
  handle.setPointerCapture = vi.fn();
  handle.hasPointerCapture = () => true;
  handle.releasePointerCapture = vi.fn();
  const pointer = (type: string, clientX: number, buttons = 1) => {
    handle.dispatchEvent(
      new PointerEvent(type, { pointerId: 1, pointerType: 'mouse', isPrimary: true, button: 0, buttons, clientX, bubbles: true }),
    );
  };
  pointer('pointerdown', 100);
  pointer('pointermove', 81);
  await vi.waitFor(() => expect(editor.published?.snapshot.externalElements[0]?.bounds.height).toBeCloseTo(210));

  window.scrollTo(0, 3000);
  window.dispatchEvent(new Event('scroll'));
  await vi.waitFor(() => expect(editor.published?.snapshot.pageData.has(0)).toBe(false));
  expect(handle.isConnected).toBe(true);

  pointer(endEvent, 81, 0);
  await vi.waitFor(() => {
    const element = editor.published?.snapshot.externalElements[0];
    expect(element?.data).toMatchObject({ proportion: endEvent === 'pointerup' ? 26 : 50 });
    expect(element?.bounds.height).toBeCloseTo(endEvent === 'pointerup' ? 208 : 400);
    expect(handle.isConnected).toBe(false);
  });
});

it('keeps an offscreen image while its enlarged view is open and unmounts it after closing', async () => {
  const { editor } = await mountImage(50, 3000, { useWindowScroll: true });
  await vi.waitFor(() => expect(document.querySelector('[aria-label="이미지 확대 보기"]')).not.toBeNull());
  const image = document.querySelector('img[alt="본문 이미지"]');
  document.querySelector<HTMLButtonElement>('[aria-label="이미지 확대 보기"]')?.click();
  await vi.waitFor(() => expect(document.querySelector('[aria-label="닫기"]')).not.toBeNull());

  window.scrollTo(0, 3000);
  window.dispatchEvent(new Event('scroll'));
  await vi.waitFor(() => expect(editor.published?.snapshot.pageData.has(0)).toBe(false));
  expect(image?.isConnected).toBe(true);
  expect(document.querySelector('[aria-label="닫기"]')).not.toBeNull();

  document.querySelector<HTMLButtonElement>('[aria-label="닫기"]')?.click();
  await vi.waitFor(() => {
    expect(document.querySelector('[aria-label="닫기"]')).toBeNull();
    expect(image?.isConnected).toBe(false);
  });
});
