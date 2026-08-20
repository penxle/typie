import '../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { initWasm } from '$lib/wasm-ffi.svelte';
import { Editor } from './editor.svelte';
import EditorFrameSyncTestHost from './editor-frame-sync-test-host.svelte';
import { handle } from './handlers';
import { handleClick } from './handlers/pointer';
import type { PlainDoc, PlainNode, PlainNodeEntry, StableSelection } from '@typie/editor-ffi/browser';

const { captureException } = vi.hoisted(() => ({ captureException: vi.fn() }));

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('@sentry/sveltekit', () => ({ captureException }));

const entry = (node: PlainNode, children: PlainNodeEntry[] = []): PlainNodeEntry => ({
  node,
  modifiers: {} as never,
  carry: [],
  children,
});

const doc: PlainDoc = {
  root: entry({ type: 'root', layout_mode: { type: 'continuous', max_width: 320 } }, [
    entry({ type: 'paragraph' }, [entry({ type: 'text', text: 'hello' })]),
  ]),
};

const imageDoc: PlainDoc = {
  root: entry({ type: 'root', layout_mode: { type: 'continuous', max_width: 320 } }, [
    entry({ type: 'image', id: 'asset', proportion: 100 }),
  ]),
};

describe('frozen comment registration', () => {
  let mounted: Record<string, unknown> | undefined;
  let editor: Editor | undefined;

  afterEach(async () => {
    if (mounted) await unmount(mounted);
    mounted = undefined;
    editor?.destroy();
    editor = undefined;
    captureException.mockReset();
    document.body.replaceChildren();
  });

  it('rejects a malformed selection without failing the editor or blocking valid siblings', async () => {
    await initWasm();
    editor = await Editor.createFromDoc(doc, { width: 320, height: 180, scale_factor: 1 });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } }));
    const selection = editor.appliedSnapshot.selection;
    if (!selection) throw new Error('Expected the initial editor selection');
    const valid = editor.freezeSelection(selection);
    if (!valid) throw new Error('Expected a stable initial selection');
    const malformed = { ...valid, version: 'invalid' } as unknown as StableSelection;

    expect(() => editor?.addFrozenComment('malformed', malformed)).not.toThrow();
    editor.addFrozenComment('valid', valid);

    expect(editor.terminal).toBe(false);
    expect(editor.registeredCommentIds()).toEqual(['valid']);
    expect(captureException).toHaveBeenCalledOnce();
  });

  it('finds a comment for the selected image atom at the clicked geometry', async () => {
    await initWasm();
    editor = await Editor.createFromDoc(imageDoc, { width: 320, height: 180, scale_factor: 1 });
    const initialImage = editor.appliedSnapshot.externalElements[0];
    if (!initialImage) throw new Error('Expected an image external element');

    editor.setExternalElementHeight(initialImage.node, 120);
    editor.updateNow((request) =>
      request.enqueue({
        type: 'selection',
        op: {
          type: 'set_at',
          page: initialImage.page_idx,
          x: initialImage.bounds.x + initialImage.bounds.width / 2,
          y: initialImage.bounds.y + initialImage.bounds.height / 2,
        },
      }),
    );
    const selection = editor.appliedSnapshot.selection;
    if (!selection) throw new Error('Expected the image selection');
    expect(selection.anchor).not.toEqual(selection.head);

    const frozen = editor.freezeSelection(selection);
    if (!frozen) throw new Error('Expected a stable image selection');
    editor.addFrozenComment('image-comment', frozen);
    await vi.waitFor(() => expect(editor?.appliedSnapshot.trackedRanges.map(({ id }) => id)).toContain('image-comment'));

    const image = editor.appliedSnapshot.externalElements[0];
    if (!image) throw new Error('Expected the updated image external element');
    expect(editor.commentIdAt(image.page_idx, image.bounds.x + image.bounds.width / 2, image.bounds.y + image.bounds.height / 2)).toBe(
      'image-comment',
    );
  });

  it('enlarges a read-only image without opening its comment', async () => {
    await initWasm();
    editor = await Editor.createFromDoc(imageDoc, { width: 320, height: 180, scale_factor: 1 });
    editor.imageAssets.set('asset', {
      id: 'asset',
      url: 'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"/>',
      originalUrl: 'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"/>',
      width: 100,
      height: 100,
      placeholder: '',
    });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_flat', start: 0, end: 1 } }));
    const selection = editor.appliedSnapshot.selection;
    if (!selection) throw new Error('Expected the image selection');
    const frozen = editor.freezeSelection(selection);
    if (!frozen) throw new Error('Expected a stable image selection');
    editor.addFrozenComment('image-comment', frozen);
    await vi.waitFor(() => expect(editor?.appliedSnapshot.trackedRanges.map(({ id }) => id)).toContain('image-comment'));
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } }));

    const commentClickHandler = vi.fn();
    editor.commentClickHandler = commentClickHandler;
    const parentClickHandler = vi.fn(handle(editor, handleClick));
    const target = document.createElement('div');
    document.body.append(target);
    mounted = mount(EditorFrameSyncTestHost, {
      target,
      props: {
        editor,
        onclick: parentClickHandler,
        readOnly: true,
        userId: `image-comment-${crypto.randomUUID()}`,
      },
    });
    await tick();

    await vi.waitFor(() => expect(document.querySelector('[aria-label="이미지 확대 보기"]')).not.toBeNull());
    const image = document.querySelector<HTMLElement>('[aria-label="이미지 확대 보기"]');
    if (!image) throw new Error('Expected the read-only image');
    await vi.waitFor(() => expect(editor?.appliedSnapshot.externalElements[0]?.bounds.height).toBeGreaterThan(1));
    const rect = image.getBoundingClientRect();
    expect(editor.readOnly).toBe(true);
    expect(rect.width).toBeGreaterThan(0);
    await userEvent.click(image);

    expect(commentClickHandler).not.toHaveBeenCalled();
    await vi.waitFor(() => expect(document.querySelector('[aria-label="닫기"]')).not.toBeNull());
    expect(parentClickHandler).not.toHaveBeenCalled();
  });
});
