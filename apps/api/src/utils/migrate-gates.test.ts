import assert from 'node:assert/strict';
import { test } from 'node:test';
import { wasm } from '#/utils/wasm.ts';
import { wasm as wasmFfi } from '#/utils/wasm-ffi.ts';
import {
  collectLegacyTextChars,
  collectPlainTextChars,
  convertLegacyDocumentJson,
  deriveExpectedTextFromPlain,
  plainStructureDiff,
  plainStructureEquals,
} from './legacy-convert.ts';
import type { PlainNodeEntry } from '@typie/editor-ffi/server';
import type { LegacyDocumentJson } from './legacy-convert.ts';

const ROOT_ID = '0'.repeat(32);
const id = (n: number) => n.toString(16).padStart(32, '0');

const canonicalize = async (json: LegacyDocumentJson): Promise<LegacyDocumentJson> => {
  await wasm.validateDocumentJson(json);
  const snapshot = await wasm.jsonToSnapshot(json);
  return await wasm.snapshotToJson(snapshot);
};

const baseSettings = {
  block_gap: 100,
  paragraph_indent: 100,
  layout_mode: { type: 'continuous', max_width: 800 },
} satisfies LegacyDocumentJson['settings'];

const buildFixture = (): LegacyDocumentJson => ({
  settings: baseSettings,
  nodes: {
    [ROOT_ID]: {
      type: 'root',
      children: [id(1), id(10), id(20), id(25), id(30), id(50), id(40)],
      cascade_attrs: {
        'style:font_family': 'Pretendard',
        'style:font_size': 1600,
        'style:font_weight': 400,
        'style:text_color': '#000000',
        'style:background_color': 'none',
        'style:letter_spacing': 0,
        'paragraph:line_height': 160,
      },
    },
    [id(1)]: {
      type: 'paragraph',
      align: 'left',
      line_height: 160,
      parent: ROOT_ID,
      children: [id(2)],
      remarks: {
        [id(100)]: { user_id: 'U1', text: '첫번째 remark', created_at: 1000 },
        [id(101)]: { user_id: 'U2', text: '두번째 remark', created_at: 2000 },
      },
    },
    [id(2)]: {
      type: 'text',
      parent: id(1),
      text: [
        { text: 'plain ' },
        { text: 'bold', styles: [{ type: 'bold' }, { type: 'font_size', size: 2000 }] },
        { text: '\tafter', annotations: [{ type: 'link', href: 'https://typie.co' }] },
      ],
    },
    [id(10)]: { type: 'table', border_style: 'solid', align: 'center', proportion: 1, parent: ROOT_ID, children: [id(11)] },
    [id(11)]: { type: 'table_row', parent: id(10), children: [id(12)] },
    [id(12)]: { type: 'table_cell', col_width: 0.5, parent: id(11), children: [id(13)] },
    [id(13)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(12), children: [id(14)] },
    [id(14)]: { type: 'text', parent: id(13), text: [{ text: 'cell text' }] },
    [id(20)]: { type: 'image', id: 'IMG1', proportion: 0.8, parent: ROOT_ID },
    [id(25)]: { type: 'horizontal_rule', variant: 'zigzag', parent: ROOT_ID },
    [id(30)]: { type: 'fold', parent: ROOT_ID, children: [id(31), id(32)], cascade_attrs: { 'style:bold': true } },
    [id(31)]: {
      type: 'fold_title',
      parent: id(30),
      children: [id(33)],
      remarks: {
        [id(102)]: { user_id: 'U3', text: '폴드 remark', created_at: 3000 },
      },
    },
    [id(33)]: { type: 'text', parent: id(31), text: [{ text: 'fold title text' }] },
    [id(32)]: { type: 'fold_content', parent: id(30), children: [id(34)] },
    [id(34)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(32), children: [] },
    [id(40)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
    [id(50)]: {
      type: 'blockquote',
      variant: 'left_line',
      parent: ROOT_ID,
      children: [id(51), id(53)],
      cascade_attrs: { 'style:font_size': 2400, 'style:text_color': '#ff0000', 'paragraph:line_height': 200 },
    },
    [id(51)]: { type: 'paragraph', align: 'left', line_height: 200, parent: id(50), children: [id(52)] },
    [id(52)]: { type: 'text', parent: id(51), text: [{ text: 'styled ' }, { text: 'sized', styles: [{ type: 'font_size', size: 1400 }] }] },
    [id(53)]: { type: 'paragraph', align: 'left', line_height: 200, parent: id(50), children: [] },
  },
});

const findFirstText = (entry: PlainNodeEntry): PlainNodeEntry | null => {
  if (entry.node.type === 'text') return entry;
  for (const child of entry.children) {
    const found = findFirstText(child);
    if (found) return found;
  }
  return null;
};

test('마이그레이션 게이트 체인: convert -> verify_plain -> to_graph_with_anchors -> heads/extract_text -> to_plain 라운드트립', async () => {
  const json = await canonicalize(buildFixture());
  const { plain, remarkAnchors, warnings } = convertLegacyDocumentJson(json);

  assert.deepEqual(warnings, ['link/ruby dropped from tab: v2 schema does not allow them on tab nodes']);
  assert.equal(remarkAnchors.length, 2);

  const fold = plain.root.children.at(4);
  const blockquote = plain.root.children.at(5);
  assert.ok(fold && blockquote);
  assert.equal(blockquote.node.type, 'blockquote');
  assert.deepEqual(blockquote.modifiers, {});
  const [styledPara, emptyPara] = blockquote.children;
  assert.deepEqual(styledPara.modifiers, {});
  const [run1, run2] = styledPara.children;
  assert.deepEqual(run1.modifiers, { font_size: { type: 'font_size', value: 2400 }, text_color: { type: 'text_color', value: '#ff0000' } });
  assert.deepEqual(run2.modifiers, { font_size: { type: 'font_size', value: 1400 }, text_color: { type: 'text_color', value: '#ff0000' } });
  assert.deepEqual(emptyPara.carry, [
    { type: 'font_size', value: 2400 },
    { type: 'text_color', value: '#ff0000' },
  ]);

  assert.equal(fold.node.type, 'fold');
  const [foldTitle] = fold.children;
  for (const run of foldTitle.children) {
    assert.deepEqual(run.modifiers, {});
  }

  const { anchors, heads, text, roundtrip } = await wasmFfi.use((host) => {
    host.verify_plain(plain);
    const result = host.to_graph_with_anchors(plain, { paths: remarkAnchors.map((anchor) => anchor.path) });
    return {
      anchors: result.anchors,
      heads: host.heads(result.graph),
      text: host.extract_text(plain),
      roundtrip: host.to_plain(result.graph),
    };
  });

  assert.equal(anchors.length, remarkAnchors.length);
  assert.ok(heads.length > 0);
  assert.equal(text, deriveExpectedTextFromPlain(plain));
  assert.equal(collectPlainTextChars(plain), collectLegacyTextChars(json));
  assert.equal(plainStructureEquals(plain, roundtrip), true);

  const mutated = structuredClone(roundtrip);
  const target = findFirstText(mutated.root);
  if (!target || target.node.type !== 'text') throw new Error('expected a text node in the roundtripped doc');
  target.node.text += '_mutated';

  assert.equal(plainStructureEquals(roundtrip, mutated), false);

  const diffs = plainStructureDiff(roundtrip, mutated);
  assert.ok(diffs.length > 0);
  assert.ok(diffs[0].includes('_mutated'));
  assert.deepEqual(plainStructureDiff(plain, roundtrip), []);
});

test('validate를 통과하지 못하는 과거 데이터: fold_title 내 스타일 마크도 게이트를 통과한다', async () => {
  const json: LegacyDocumentJson = await (async () => {
    const fixture: LegacyDocumentJson = {
      settings: baseSettings,
      nodes: {
        [ROOT_ID]: { type: 'root', children: [id(1), id(4)] },
        [id(1)]: { type: 'fold', parent: ROOT_ID, children: [id(2), id(3)] },
        [id(2)]: {
          type: 'text',
          parent: id(1),
          text: [],
        },
        [id(3)]: { type: 'fold_content', parent: id(1), children: [id(5)] },
        [id(5)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(3), children: [] },
        [id(4)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
      },
    };
    fixture.nodes[id(2)] = {
      type: 'fold_title',
      parent: id(1),
      children: [id(6)],
    };
    fixture.nodes[id(6)] = {
      type: 'text',
      parent: id(2),
      text: [
        { text: 'plain ' },
        {
          text: 'legacy styled',
          styles: [
            { type: 'font_family', family: 'Iropke Batang OTF' },
            { type: 'font_size', size: 1000 },
            { type: 'text_color', color: 'black' },
          ],
        },
      ],
    };
    const snapshot = await wasm.jsonToSnapshot(fixture);
    return await wasm.snapshotToJson(snapshot);
  })();

  const { plain, remarkAnchors, warnings } = convertLegacyDocumentJson(json);

  assert.equal(
    warnings.some((w) => w.includes('fold_title')),
    false,
  );

  const [foldNode] = plain.root.children;
  const [foldTitle] = foldNode.children;
  assert.equal(foldTitle.node.type, 'fold_title');
  for (const run of foldTitle.children) {
    assert.deepEqual(run.modifiers, {});
  }

  const { text, roundtrip } = await wasmFfi.use((host) => {
    host.verify_plain(plain);
    const result = host.to_graph_with_anchors(plain, { paths: remarkAnchors.map((anchor) => anchor.path) });
    return { text: host.extract_text(plain), roundtrip: host.to_plain(result.graph) };
  });

  assert.equal(text, deriveExpectedTextFromPlain(plain));
  assert.deepEqual(plainStructureDiff(plain, roundtrip), []);
});
