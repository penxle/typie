import assert from 'node:assert/strict';
import test from 'node:test';
import { handleFontDataMissing, manifestEscalationKey, registerFonts } from './fonts.ts';
import type { Editor, EditorHost, ResourceUpdate } from '@typie/editor-ffi/server';
import type { EditorFontFamily } from './font-families.ts';

test('manifestEscalationKey escalates when a required manifest failed to register', () => {
  const key = manifestEscalationKey({ family: 'Pretendard', weight: 400, required: [{ type: 'manifest' }] }, new Set(['Pretendard:400']));
  assert.equal(key, 'Pretendard:400');
});

test('manifestEscalationKey does not escalate when the required manifest registered successfully', () => {
  const key = manifestEscalationKey({ family: 'Pretendard', weight: 400, required: [{ type: 'manifest' }] }, new Set());
  assert.equal(key, null);
});

test('manifestEscalationKey does not escalate when the manifest is only in prefetch', () => {
  const key = manifestEscalationKey({ family: 'Pretendard', weight: 400, required: [{ type: 'base' }] }, new Set(['Pretendard:400']));
  assert.equal(key, null);
});

test('manifestEscalationKey does not escalate for base/chunk requirements without a manifest', () => {
  const key = manifestEscalationKey(
    { family: 'Pretendard', weight: 400, required: [{ type: 'base' }, { type: 'chunk', id: 0 }] },
    new Set(['Pretendard:400']),
  );
  assert.equal(key, null);
});

test('registerFonts marks manifests that fail to register without touching families that succeed', async (t) => {
  t.mock.method(globalThis, 'fetch', async (input: Parameters<typeof fetch>[0]) => {
    const url = String(input);
    return url.includes('/good/') ? new Response(new Uint8Array([1])) : new Response(null, { status: 404 });
  });
  // eslint-disable-next-line @typescript-eslint/no-empty-function -- host mock only needs to satisfy the call shape
  const host = { set_fonts: () => {}, add_font_manifest: () => {} } as unknown as EditorHost;

  const families: EditorFontFamily[] = [
    { name: 'Good', source: 'DEFAULT', weights: [{ value: 400, hash: 'h1', chunks: [[0x41, 0x42]], baseUrl: 'https://cdn/good' }] },
    { name: 'Bad', source: 'DEFAULT', weights: [{ value: 400, hash: 'h2', chunks: [[0x41]], baseUrl: 'https://cdn/bad' }] },
  ];

  const reg = await registerFonts(host, families);

  assert.equal(reg.failedManifests.has('Bad:400'), true);
  assert.equal(reg.failedManifests.has('Good:400'), false);
  assert.equal(reg.baseUrlOf('Good', 400), 'https://cdn/good');
});

test('registerFonts frees resource updates created before the editor exists', async (t) => {
  t.mock.method(globalThis, 'fetch', async () => new Response(new Uint8Array([1])));
  let freeCount = 0;
  const fontsUpdate = { free: () => freeCount++ } as unknown as ResourceUpdate;
  const manifestUpdate = { free: () => freeCount++ } as unknown as ResourceUpdate;
  const host = {
    set_fonts: () => fontsUpdate,
    add_font_manifest: () => manifestUpdate,
  } as unknown as EditorHost;
  const families: EditorFontFamily[] = [
    { name: 'Good', source: 'DEFAULT', weights: [{ value: 400, hash: 'h1', chunks: [[0x41, 0x42]], baseUrl: 'https://cdn/good' }] },
  ];

  await registerFonts(host, families);

  assert.equal(freeCount, 2);
});

test('registerFonts requests v2 first and falls back to persisted v1', async (t) => {
  const requested: string[] = [];
  t.mock.method(globalThis, 'fetch', async (input: Parameters<typeof fetch>[0]) => {
    const url = String(input);
    requested.push(url);
    return url.endsWith('/manifest.v2') ? new Response(null, { status: 404 }) : new Response(new Uint8Array([1]));
  });
  let applied = 0;
  let setFontsCalls = 0;
  const host = {
    set_fonts: () => {
      setFontsCalls++;
    },
    add_font_manifest: () => {
      applied++;
    },
  } as unknown as EditorHost;
  const families: EditorFontFamily[] = [
    { name: 'Good', source: 'DEFAULT', weights: [{ value: 400, hash: 'h1', chunks: [], baseUrl: 'https://cdn/fallback' }] },
  ];

  await registerFonts(host, families);

  assert.deepEqual(requested, ['https://cdn/fallback/manifest.v2', 'https://cdn/fallback/manifest.v1']);
  assert.equal(setFontsCalls, 1);
  assert.equal(applied, 1);
});

test('registerFonts fetches manifests for different weights concurrently', async (t) => {
  let activeFetches = 0;
  let maxActiveFetches = 0;
  t.mock.method(globalThis, 'fetch', async () => {
    activeFetches++;
    maxActiveFetches = Math.max(maxActiveFetches, activeFetches);
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
    activeFetches--;
    return new Response(new Uint8Array([1]));
  });
  let setFontsCalls = 0;
  let manifestCalls = 0;
  const host = {
    set_fonts: () => {
      setFontsCalls++;
    },
    add_font_manifest: () => {
      manifestCalls++;
    },
  } as unknown as EditorHost;
  const families: EditorFontFamily[] = [
    { name: 'First', source: 'DEFAULT', weights: [{ value: 400, hash: 'parallel-1', chunks: [], baseUrl: 'https://cdn/parallel-1' }] },
    { name: 'Second', source: 'DEFAULT', weights: [{ value: 400, hash: 'parallel-2', chunks: [], baseUrl: 'https://cdn/parallel-2' }] },
  ];

  await registerFonts(host, families);

  assert.equal(maxActiveFetches, 2);
  assert.equal(setFontsCalls, 1);
  assert.equal(manifestCalls, 2);
});

test('handleFontDataMissing applies and frees resource updates returned by the host', async () => {
  let freeCount = 0;
  const baseUpdate = { free: () => freeCount++ } as unknown as ResourceUpdate;
  const chunkUpdate = { free: () => freeCount++ } as unknown as ResourceUpdate;
  const host = {
    add_font_base: () => baseUpdate,
    add_font_chunk: () => chunkUpdate,
  } as unknown as EditorHost;
  const received: ResourceUpdate[] = [];
  const editor = {
    receive_resource_update: (update: ResourceUpdate) => {
      received.push(update);
    },
  } as unknown as Editor;

  await handleFontDataMissing(
    host,
    editor,
    { baseUrlOf: () => 'data:application/octet-stream,resource-update', failedManifests: new Set() },
    {
      family: 'Pretendard',
      weight: 400,
      required: [{ type: 'base' }, { type: 'chunk', id: 1 }],
      prefetch: [],
    },
  );

  assert.deepEqual(new Set(received), new Set([baseUpdate, chunkUpdate]));
  assert.equal(freeCount, 2);
});

test('handleFontDataMissing frees a resource update and fails when the editor rejects it', async () => {
  let freeCount = 0;
  const update = { free: () => freeCount++ } as unknown as ResourceUpdate;
  let added = false;
  const host = {
    add_font_base: () => {
      if (added) return;
      added = true;
      return update;
    },
  } as unknown as EditorHost;
  let receiveCount = 0;
  const editor = {
    receive_resource_update: () => {
      receiveCount++;
      throw new Error('rejected');
    },
  } as unknown as Editor;

  await assert.rejects(
    handleFontDataMissing(
      host,
      editor,
      { baseUrlOf: () => 'data:application/octet-stream,rejected-resource-update', failedManifests: new Set() },
      { family: 'Pretendard', weight: 400, required: [{ type: 'base' }], prefetch: [] },
    ),
    /failed to deliver font resource/,
  );

  assert.equal(receiveCount, 1);
  assert.equal(freeCount, 1);
});

test('handleFontDataMissing fails when the host cannot produce the requested update', async () => {
  let addBaseCount = 0;
  const host = {
    add_font_base: () => {
      addBaseCount++;
      return;
    },
  } as unknown as EditorHost;
  const editor = {
    receive_resource_update: () => assert.fail('no update should be delivered'),
  } as unknown as Editor;

  await assert.rejects(
    handleFontDataMissing(
      host,
      editor,
      { baseUrlOf: () => 'data:application/octet-stream,missing-resource-update', failedManifests: new Set() },
      { family: 'Pretendard', weight: 400, required: [{ type: 'base' }], prefetch: [] },
    ),
    /produced no resource update/,
  );
  assert.equal(addBaseCount, 1);
});
