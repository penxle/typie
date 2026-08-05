import '../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Editor } from './editor.svelte';
import EditorFrameSyncTestHost from './editor-frame-sync-test-host.svelte';
import type { PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/browser';
import type { EditorFrameSyncTestHarness } from './editor-frame-sync-test-host.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('./components/ExternalElement.svelte', async () => {
  const stub = await import('./embed-keep-alive-test-stub.svelte');
  return { default: stub.default };
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

const entry = (node: PlainNode, children: PlainNodeEntry[] = []): PlainNodeEntry => ({
  node,
  modifiers: {} as never,
  carry: [],
  children,
});

const documentWithEmbed = (includeEmbed: boolean): PlainDoc => ({
  root: entry(
    { type: 'root', layout_mode: { type: 'continuous', max_width: 320 } },
    includeEmbed ? [entry({ type: 'embed', id: 'embed-1' })] : [entry({ type: 'paragraph' })],
  ),
});

const paginatedDocumentWithEmbed = (): PlainDoc => ({
  root: entry(
    {
      type: 'root',
      layout_mode: {
        type: 'paginated',
        page_width: 320,
        page_height: 220,
        page_margin_top: 24,
        page_margin_bottom: 24,
        page_margin_left: 24,
        page_margin_right: 24,
      },
    },
    [entry({ type: 'paragraph' }), entry({ type: 'embed', id: 'embed-1' })],
  ),
});

async function mountViewer(plain = documentWithEmbed(true)): Promise<EditorFrameSyncTestHarness> {
  editor = await Editor.createFromDoc(plain, { width: 360, height: 180, scale_factor: 1 });
  const target = document.createElement('div');
  document.body.append(target);
  const harness = Promise.withResolvers<EditorFrameSyncTestHarness>();
  mounted = mount(EditorFrameSyncTestHost, {
    target,
    props: {
      editor,
      onReady: harness.resolve,
      readOnly: true,
      useWindowScroll: true,
      userId: `embed-keep-alive-${crypto.randomUUID()}`,
    },
  });
  const result = await harness.promise;
  await tick();
  await vi.waitFor(() => expect(editor?.published?.frames.get(0)).toBeDefined());
  return result;
}

describe('viewer embed keep-alive', () => {
  it('keeps the same iframe across hidden surface release until document deletion', async () => {
    await mountViewer();

    const initialIframe = document.querySelector<HTMLIFrameElement>('[data-keep-alive-embed]');
    expect(initialIframe).not.toBeNull();
    const parkingElement = initialIframe?.closest<HTMLElement>('[data-document-embed]');
    expect(parkingElement).not.toBeNull();

    window.scrollTo(0, 3000);
    window.dispatchEvent(new Event('scroll'));
    await vi.waitFor(() => expect(editor?.published?.snapshot.pageData.has(0)).toBe(false));
    expect(document.querySelector('[data-page-canvas="0"]')).toBeNull();
    expect(document.querySelector('[data-keep-alive-embed]')).toBe(initialIframe);
    expect(initialIframe?.isConnected).toBe(true);
    expect(parkingElement?.inert).toBe(true);
    expect(getComputedStyle(parkingElement as HTMLElement).visibility).toBe('hidden');
    expect(getComputedStyle(parkingElement as HTMLElement).pointerEvents).toBe('none');

    window.scrollTo(0, 0);
    window.dispatchEvent(new Event('scroll'));
    await vi.waitFor(() => expect(editor?.published?.snapshot.pageData.has(0)).toBe(true));
    expect(document.querySelector('[data-keep-alive-embed]')).toBe(initialIframe);
    expect(parkingElement?.inert).toBe(false);
    expect(getComputedStyle(parkingElement as HTMLElement).visibility).toBe('visible');

    editor?.setDoc(documentWithEmbed(false));
    await vi.waitFor(() => expect(document.querySelector('[data-keep-alive-embed]')).toBeNull());
  });

  it('keeps the same iframe when the embed moves to another page', async () => {
    await mountViewer(paginatedDocumentWithEmbed());

    const initialIframe = document.querySelector<HTMLIFrameElement>('[data-keep-alive-embed]');
    expect(initialIframe).not.toBeNull();

    if (!editor) throw new Error('Expected an editor');
    editor.readOnly = false;
    editor.updateNow((request) => {
      request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: 24, y: 24 } });
      request.enqueue({ type: 'insertion', op: { type: 'break', kind: 'page' } });
    });

    await vi.waitFor(() => expect(editor?.published?.snapshot.externalElements[0]?.page_idx).toBe(1));
    expect(document.querySelector('[data-keep-alive-embed]')).toBe(initialIframe);
  });

  it('requires a reinserted embed node to materialize again after document deletion', async () => {
    await mountViewer();
    expect(document.querySelector('[data-keep-alive-embed]')).not.toBeNull();

    window.scrollTo(0, 3000);
    window.dispatchEvent(new Event('scroll'));
    await vi.waitFor(() => expect(editor?.published?.snapshot.pageData.has(0)).toBe(false));

    editor?.setDoc(documentWithEmbed(false));
    await vi.waitFor(() => expect(document.querySelector('[data-keep-alive-embed]')).toBeNull());

    editor?.setDoc(documentWithEmbed(true));
    await vi.waitFor(() => expect(editor?.published?.snapshot.externalElements).toHaveLength(1));
    expect(document.querySelector('[data-keep-alive-embed]')).toBeNull();

    window.scrollTo(0, 0);
    window.dispatchEvent(new Event('scroll'));
    await vi.waitFor(() => expect(document.querySelector('[data-keep-alive-embed]')).not.toBeNull());
  });
});
