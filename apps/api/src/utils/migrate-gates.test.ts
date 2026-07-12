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

test('구식 스키마 데이터: 비직사각형 테이블·컨테이너 직속 인라인·인접 동종 리스트가 선정규화되어 게이트를 통과한다', async () => {
  const json: LegacyDocumentJson = await (async () => {
    const fixture: LegacyDocumentJson = {
      settings: baseSettings,
      nodes: {
        [ROOT_ID]: { type: 'root', children: [id(1), id(20), id(29), id(40)] },
        [id(29)]: { type: 'bullet_list', parent: ROOT_ID, children: [id(30)] },
        [id(1)]: { type: 'table', border_style: 'solid', align: 'left', proportion: 1, parent: ROOT_ID, children: [id(2), id(3)] },
        [id(2)]: { type: 'table_row', parent: id(1), children: [id(4), id(5), id(6)] },
        [id(3)]: { type: 'table_row', parent: id(1), children: [id(7)] },
        [id(4)]: { type: 'table_cell', parent: id(2), children: [id(8)] },
        [id(5)]: { type: 'table_cell', parent: id(2), children: [id(9)] },
        [id(6)]: { type: 'table_cell', parent: id(2), children: [id(10)] },
        [id(7)]: { type: 'table_cell', parent: id(3), children: [id(11)] },
        [id(8)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(4), children: [] },
        [id(9)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(5), children: [] },
        [id(10)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(6), children: [] },
        [id(11)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(7), children: [] },
        [id(20)]: { type: 'bullet_list', parent: ROOT_ID, children: [id(21)] },
        [id(21)]: { type: 'list_item', parent: id(20), children: [id(22), id(23), id(24)] },
        [id(22)]: { type: 'text', parent: id(21), text: [{ text: '항목 ' }, { text: '강조', styles: [{ type: 'bold' }] }] },
        [id(23)]: { type: 'hard_break', parent: id(21) },
        [id(24)]: { type: 'bullet_list', parent: id(21), children: [id(25)] },
        [id(25)]: { type: 'list_item', parent: id(24), children: [id(26)] },
        [id(26)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(25), children: [] },
        [id(30)]: { type: 'list_item', parent: id(29), children: [id(31), id(32), id(33)] },
        [id(31)]: { type: 'bullet_list', parent: id(30), children: [id(34)] },
        [id(32)]: { type: 'bullet_list', parent: id(30), children: [id(35)] },
        [id(33)]: { type: 'bullet_list', parent: id(30), children: [id(36)] },
        [id(34)]: { type: 'list_item', parent: id(31), children: [id(37)] },
        [id(35)]: { type: 'list_item', parent: id(32), children: [id(38)] },
        [id(36)]: { type: 'list_item', parent: id(33), children: [id(39)] },
        [id(37)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(34), children: [] },
        [id(38)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(35), children: [] },
        [id(39)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(36), children: [] },
        [id(40)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
      },
    };
    const snapshot = await wasm.jsonToSnapshot(fixture);
    return await wasm.snapshotToJson(snapshot);
  })();

  const { plain, remarkAnchors } = convertLegacyDocumentJson(json);

  const [table, outerList] = plain.root.children;
  assert.equal(plain.root.children.length, 3);
  assert.equal(table.node.type, 'table');
  for (const row of table.children) {
    assert.equal(row.children.length, 3);
  }
  const [, secondRow] = table.children;
  const [, padCell1, padCell2] = secondRow.children;
  for (const cell of [padCell1, padCell2]) {
    assert.equal(cell.node.type, 'table_cell');
    assert.deepEqual(
      cell.children.map((c) => c.node.type),
      ['paragraph'],
    );
  }

  assert.equal(outerList.node.type, 'bullet_list');
  assert.equal(outerList.children.length, 2);
  const [item, mergeItem] = outerList.children;
  assert.deepEqual(
    item.children.map((c) => c.node.type),
    ['paragraph', 'bullet_list'],
  );
  const [wrapped] = item.children;
  assert.deepEqual(
    wrapped.children.map((c) => c.node.type),
    ['text', 'text', 'hard_break'],
  );

  assert.equal(mergeItem.node.type, 'list_item');
  assert.deepEqual(
    mergeItem.children.map((c) => c.node.type),
    ['paragraph', 'bullet_list'],
  );
  const mergedList = mergeItem.children.at(1);
  assert.ok(mergedList);
  assert.equal(mergedList.children.length, 3);

  const { text, roundtrip } = await wasmFfi.use((host) => {
    host.verify_plain(plain);
    const result = host.to_graph_with_anchors(plain, { paths: remarkAnchors.map((anchor) => anchor.path) });
    return { text: host.extract_text(plain), roundtrip: host.to_plain(result.graph) };
  });

  assert.equal(text, deriveExpectedTextFromPlain(plain));
  assert.deepEqual(plainStructureDiff(plain, roundtrip), []);
});

test('구식 스키마 데이터: 다문단 list_item·행 직속 블록·다중 fold_title/fold_content·고아 list_item이 선정규화되어 게이트를 통과한다', async () => {
  const json: LegacyDocumentJson = await (async () => {
    const fixture: LegacyDocumentJson = {
      settings: baseSettings,
      nodes: {
        [ROOT_ID]: { type: 'root', children: [id(1), id(10), id(20), id(30), id(40)] },
        [id(1)]: { type: 'bullet_list', parent: ROOT_ID, children: [id(2)] },
        [id(2)]: {
          type: 'list_item',
          parent: id(1),
          children: [id(3), id(4), id(5), id(8)],
          remarks: { [id(100)]: { user_id: 'U1', text: '항목 remark', created_at: 1000 } },
        },
        [id(3)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(2), children: [id(9)] },
        [id(9)]: { type: 'text', parent: id(3), text: [{ text: '첫 문단' }] },
        [id(4)]: {
          type: 'paragraph',
          align: 'left',
          line_height: 160,
          parent: id(2),
          children: [id(50)],
          remarks: { [id(101)]: { user_id: 'U2', text: '둘째 문단 remark', created_at: 2000 } },
        },
        [id(50)]: { type: 'text', parent: id(4), text: [{ text: '둘째 문단' }] },
        [id(5)]: { type: 'bullet_list', parent: id(2), children: [id(6)] },
        [id(6)]: { type: 'list_item', parent: id(5), children: [id(7)] },
        [id(7)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(6), children: [id(51)] },
        [id(51)]: { type: 'text', parent: id(7), text: [{ text: '중첩' }] },
        [id(8)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(2), children: [id(52)] },
        [id(52)]: { type: 'text', parent: id(8), text: [{ text: '셋째' }] },
        [id(10)]: { type: 'table', border_style: 'solid', align: 'left', proportion: 1, parent: ROOT_ID, children: [id(11), id(14)] },
        [id(11)]: { type: 'table_row', parent: id(10), children: [id(12), id(13)] },
        [id(12)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(11), children: [] },
        [id(13)]: { type: 'text', parent: id(11), text: [{ text: '행 직속 텍스트' }] },
        [id(14)]: { type: 'table_row', parent: id(10), children: [id(15)] },
        [id(15)]: { type: 'table_cell', parent: id(14), children: [id(16)] },
        [id(16)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(15), children: [] },
        [id(20)]: { type: 'fold', parent: ROOT_ID, children: [id(21), id(22), id(23), id(25)] },
        [id(21)]: { type: 'fold_title', parent: id(20), children: [id(53)] },
        [id(53)]: { type: 'text', parent: id(21), text: [{ text: '제목1' }] },
        [id(22)]: { type: 'fold_title', parent: id(20), children: [id(54)] },
        [id(54)]: { type: 'text', parent: id(22), text: [{ text: '제목2' }] },
        [id(23)]: { type: 'fold_content', parent: id(20), children: [id(24)] },
        [id(24)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(23), children: [id(55)] },
        [id(55)]: { type: 'text', parent: id(24), text: [{ text: '내용1' }] },
        [id(25)]: { type: 'fold_content', parent: id(20), children: [id(26)] },
        [id(26)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(25), children: [id(56)] },
        [id(56)]: { type: 'text', parent: id(26), text: [{ text: '내용2' }] },
        [id(30)]: { type: 'list_item', parent: ROOT_ID, children: [id(31)] },
        [id(31)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(30), children: [id(57)] },
        [id(57)]: { type: 'text', parent: id(31), text: [{ text: '고아' }] },
        [id(40)]: { type: 'image', id: 'IMG1', proportion: 0.8, parent: ROOT_ID },
      },
    };
    const snapshot = await wasm.jsonToSnapshot(fixture);
    return await wasm.snapshotToJson(snapshot);
  })();

  const { plain, remarkAnchors } = convertLegacyDocumentJson(json);

  assert.equal(plain.root.children.length, 6);
  const [outerList, table, fold, orphanList, image, trailing] = plain.root.children;

  assert.equal(outerList.node.type, 'bullet_list');
  assert.equal(outerList.children.length, 2);
  const [firstItem, secondItem] = outerList.children;
  assert.deepEqual(
    firstItem.children.map((c) => c.node.type),
    ['paragraph', 'bullet_list'],
  );
  const [mergedPara] = firstItem.children;
  assert.deepEqual(
    mergedPara.children.map((c) => c.node.type),
    ['text', 'hard_break', 'text'],
  );
  assert.deepEqual(
    secondItem.children.map((c) => c.node.type),
    ['paragraph'],
  );

  assert.equal(remarkAnchors.length, 2);
  const [itemAnchor, paraAnchor] = remarkAnchors;
  assert.equal(itemAnchor.remarks[0].text, '항목 remark');
  assert.deepEqual(itemAnchor.path, [0, 0]);
  assert.equal(paraAnchor.remarks[0].text, '둘째 문단 remark');
  assert.deepEqual(paraAnchor.path, [0, 0, 0]);

  assert.equal(table.node.type, 'table');
  for (const row of table.children) {
    assert.equal(row.children.length, 2);
    for (const cell of row.children) {
      assert.equal(cell.node.type, 'table_cell');
    }
  }

  assert.equal(fold.node.type, 'fold');
  assert.deepEqual(
    fold.children.map((c) => c.node.type),
    ['fold_title', 'fold_content'],
  );
  const [foldTitle, foldContent] = fold.children;
  assert.equal(foldTitle.children.length, 1);
  const [titleRun] = foldTitle.children;
  assert.equal(titleRun.node.type === 'text' && titleRun.node.text, '제목1제목2');
  assert.deepEqual(
    foldContent.children.map((c) => c.node.type),
    ['paragraph', 'paragraph'],
  );

  assert.equal(orphanList.node.type, 'bullet_list');
  assert.equal(orphanList.children.at(0)?.node.type, 'list_item');

  assert.equal(image.node.type, 'image');
  assert.equal(trailing.node.type, 'paragraph');

  const { text, roundtrip } = await wasmFfi.use((host) => {
    host.verify_plain(plain);
    const result = host.to_graph_with_anchors(plain, { paths: remarkAnchors.map((anchor) => anchor.path) });
    return { text: host.extract_text(plain), roundtrip: host.to_plain(result.graph) };
  });

  assert.equal(text, deriveExpectedTextFromPlain(plain));
  assert.equal(collectPlainTextChars(plain), collectLegacyTextChars(json));
  assert.deepEqual(plainStructureDiff(plain, roundtrip), []);
});

test('구식 스키마 데이터: 다중/중간 page_break·table 직속 문단·fold_title 내 tab이 선정규화되어 게이트를 통과한다', async () => {
  const json: LegacyDocumentJson = await (async () => {
    const fixture: LegacyDocumentJson = {
      settings: baseSettings,
      nodes: {
        [ROOT_ID]: { type: 'root', children: [id(1), id(5), id(10), id(20), id(30)] },
        [id(1)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [id(2), id(3)] },
        [id(2)]: { type: 'page_break', parent: id(1) },
        [id(3)]: { type: 'page_break', parent: id(1) },
        [id(5)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [id(6), id(7), id(8)] },
        [id(6)]: { type: 'text', parent: id(5), text: [{ text: '앞' }] },
        [id(7)]: { type: 'page_break', parent: id(5) },
        [id(8)]: { type: 'text', parent: id(5), text: [{ text: '뒤' }] },
        [id(10)]: { type: 'table', border_style: 'solid', align: 'left', proportion: 1, parent: ROOT_ID, children: [id(11), id(13)] },
        [id(11)]: { type: 'paragraph', align: 'center', line_height: 160, parent: id(10), children: [id(12)] },
        [id(12)]: { type: 'text', parent: id(11), text: [{ text: '표 직속 문단' }] },
        [id(13)]: { type: 'table_row', parent: id(10), children: [id(14)] },
        [id(14)]: { type: 'table_cell', parent: id(13), children: [id(15)] },
        [id(15)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(14), children: [] },
        [id(20)]: { type: 'fold', parent: ROOT_ID, children: [id(21), id(23)] },
        [id(21)]: { type: 'fold_title', parent: id(20), children: [id(22)] },
        [id(22)]: { type: 'text', parent: id(21), text: [{ text: '제목\t이어짐' }] },
        [id(23)]: { type: 'fold_content', parent: id(20), children: [id(24)] },
        [id(24)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(23), children: [] },
        [id(30)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
      },
    };
    const snapshot = await wasm.jsonToSnapshot(fixture);
    return await wasm.snapshotToJson(snapshot);
  })();

  const { plain, remarkAnchors, warnings } = convertLegacyDocumentJson(json);

  assert.equal(plain.root.children.length, 7);
  const [pb1, pb2, front, back, table, fold] = plain.root.children;
  for (const para of [pb1, pb2]) {
    assert.equal(para.node.type, 'paragraph');
    assert.deepEqual(
      para.children.map((c) => c.node.type),
      ['page_break'],
    );
  }
  assert.deepEqual(
    front.children.map((c) => c.node.type),
    ['text', 'page_break'],
  );
  assert.deepEqual(
    back.children.map((c) => c.node.type),
    ['text'],
  );

  assert.equal(table.node.type, 'table');
  assert.equal(table.children.length, 2);
  for (const row of table.children) {
    assert.equal(row.node.type, 'table_row');
    assert.equal(row.children.length, 1);
    assert.equal(row.children.at(0)?.node.type, 'table_cell');
  }
  const wrappedPara = table.children.at(0)?.children.at(0)?.children.at(0);
  assert.ok(wrappedPara);
  assert.equal(wrappedPara.node.type, 'paragraph');
  assert.deepEqual(wrappedPara.modifiers, { alignment: { type: 'alignment', value: 'center' } });

  assert.equal(fold.node.type, 'fold');
  const [foldTitle] = fold.children;
  assert.deepEqual(
    foldTitle.children.map((c) => c.node.type),
    ['text'],
  );
  const [titleRun] = foldTitle.children;
  assert.equal(titleRun.node.type === 'text' && titleRun.node.text, '제목이어짐');
  assert.ok(warnings.some((w) => w.includes('fold_title children other than text dropped')));

  const { text, roundtrip } = await wasmFfi.use((host) => {
    host.verify_plain(plain);
    const result = host.to_graph_with_anchors(plain, { paths: remarkAnchors.map((anchor) => anchor.path) });
    return { text: host.extract_text(plain), roundtrip: host.to_plain(result.graph) };
  });

  assert.equal(text, deriveExpectedTextFromPlain(plain));
  assert.equal(collectPlainTextChars(plain), collectLegacyTextChars(json));
  assert.deepEqual(plainStructureDiff(plain, roundtrip), []);
});
