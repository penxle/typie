import '../../app.css';

import { afterEach, describe, expect, it, vi } from 'vitest';
import { initWasm } from '$lib/wasm-ffi.svelte';
import { createDocumentDomMirror } from './document-dom-mirror';
import { Editor } from './editor.svelte';
import type { PlainNodeEntry } from '@typie/editor-ffi/browser';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));

const SHALLOW_GRAPH_BASE64 =
  'wgEAAAAAAasCtQIozdMW5czWiSi1L/0EOPUIAFwPAAIAAAAAAAAAAAGAgICAgICAgIABAAEAFwwBAAcIAAAABAEC2AQOAQEBAAkAAgYBDgECAQAJAQsBAwEABgIAA+yKpAsBBAEABgMAA+2MjAsBBQEABgQAA+ydtAsBBgEABgUAA+2BrAkBBwEABAYAASALAQgBAAYHAAPssqsJAQkBAAQIAAEgCwEKAQAGCQAD66y4CwELAQAGCgAD64uoDgEMAQAJCwsBDQEABgwOAQAGDQAPAQAGDgADEAEABg8AEQEABBAAASALARIBAAYRAAPrkZgLARMBAAYSAAPsp7gJARQBAAQTAAEgCwEVAQAGFBYBAAYVCqgQTrBd6s9wEBRCyAcQeuIHhkEYHPXqTT16kt/ET+Vejc5mBCTMZYV1';
const EXPECTED_PARAGRAPHS = ['스파이크 첫 문단', '스파이크 둘째 문단'];

const paragraphTexts = (entry: PlainNodeEntry): string[] =>
  entry.node.type === 'paragraph'
    ? [entry.children.map((child) => (child.node.type === 'text' ? child.node.text : '')).join('')]
    : entry.children.flatMap((child) => paragraphTexts(child));

describe('publication shallow graph', () => {
  let editor: Editor | undefined;

  afterEach(() => {
    editor?.destroy();
    editor = undefined;
  });

  it('loads a server-made shallow graph through Editor.create and projects the same paragraphs', async () => {
    await initWasm();
    editor = await Editor.create(Uint8Array.fromBase64(SHALLOW_GRAPH_BASE64), { width: 640, height: 480, scale_factor: 1 });
    expect(editor.failure).toBeUndefined();

    const mirror = createDocumentDomMirror(editor.documentDomProjection());
    const texts = paragraphTexts(mirror.project().doc.root).filter((text) => text.length > 0);
    expect(texts).toEqual(EXPECTED_PARAGRAPHS);
  });
});
