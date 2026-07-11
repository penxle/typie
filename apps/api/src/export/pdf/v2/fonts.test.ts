import assert from 'node:assert/strict';
import test from 'node:test';
import { manifestEscalationKey, registerFonts } from './fonts.ts';
import type { EditorHost } from '@typie/editor-ffi/server';
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

test('registerFonts marks manifests that fail to build without touching families that succeed', async () => {
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
