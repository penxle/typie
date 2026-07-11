import assert from 'node:assert/strict';
import { test } from 'node:test';
import { wasm } from '#/utils/wasm.ts';
import { collectLegacyTextChars, convertLegacyDocumentJson, deriveExpectedTextFromPlain } from './legacy-convert.ts';
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

test('빈 문단 하나짜리 문서가 v2 트리로 변환된다', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: {
        type: 'root',
        children: [id(1)],
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
      [id(1)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
    },
  });

  const { plain, remarkAnchors, warnings } = convertLegacyDocumentJson(json);
  const [paragraph] = plain.root.children;

  assert.equal(plain.root.node.type, 'root');
  assert.deepEqual(plain.root.node.type === 'root' ? plain.root.node.layout_mode : null, {
    type: 'continuous',
    max_width: 800,
  });
  assert.equal(plain.root.children.length, 1);
  assert.equal(paragraph.node.type, 'paragraph');
  assert.equal(paragraph.children.length, 0);
  assert.equal(plain.root.modifiers.font_family?.type === 'font_family' && plain.root.modifiers.font_family.value, 'Pretendard');
  assert.equal(plain.root.modifiers.font_size?.type === 'font_size' && plain.root.modifiers.font_size.value, 1600);
  assert.equal(plain.root.modifiers.line_height?.type === 'line_height' && plain.root.modifiers.line_height.value, 160);
  assert.equal(plain.root.modifiers.block_gap?.type === 'block_gap' && plain.root.modifiers.block_gap.value, 100);
  assert.equal(plain.root.modifiers.paragraph_indent?.type === 'paragraph_indent' && plain.root.modifiers.paragraph_indent.value, 100);
  assert.equal('background_color' in plain.root.modifiers, false);
  assert.equal('text_color' in plain.root.modifiers, false);
  assert.deepEqual(remarkAnchors, []);
  assert.deepEqual(warnings, []);
});

test('블록 구조가 보존된다 (blockquote/callout/list/fold/table/image)', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: { type: 'root', children: [id(1), id(4), id(7), id(10), id(12)] },
      [id(12)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
      [id(1)]: { type: 'blockquote', variant: 'left_quote', parent: ROOT_ID, children: [id(2)] },
      [id(2)]: { type: 'paragraph', align: 'center', line_height: 160, parent: id(1), children: [id(3)] },
      [id(3)]: { type: 'text', parent: id(2), text: [{ text: 'quoted' }] },
      [id(4)]: { type: 'bullet_list', parent: ROOT_ID, children: [id(5)] },
      [id(5)]: { type: 'list_item', parent: id(4), children: [id(6)] },
      [id(6)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(5), children: [] },
      [id(7)]: { type: 'table', border_style: 'solid', align: 'center', proportion: 1, parent: ROOT_ID, children: [id(8)] },
      [id(8)]: { type: 'table_row', parent: id(7), children: [id(9)] },
      [id(9)]: { type: 'table_cell', col_width: 0.5, parent: id(8), children: [id(11)] },
      [id(11)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(9), children: [] },
      [id(10)]: { type: 'image', id: 'IMG123', proportion: 0.8, parent: ROOT_ID },
    },
  });

  const { plain } = convertLegacyDocumentJson(json);
  const [bq, list, table, image] = plain.root.children;
  const [bqParagraph] = bq.children;
  const [listItem] = list.children;
  const [tableRow] = table.children;
  const [tableCell] = tableRow.children;

  assert.equal(bq.node.type, 'blockquote');
  assert.equal(bq.node.type === 'blockquote' && bq.node.variant, 'left_quote');
  assert.equal(bqParagraph.node.type, 'paragraph');
  assert.equal(bqParagraph.modifiers.alignment?.type === 'alignment' && bqParagraph.modifiers.alignment.value, 'center');

  assert.equal(list.node.type, 'bullet_list');
  assert.equal(listItem.node.type, 'list_item');

  assert.equal(table.node.type, 'table');
  assert.equal(table.node.type === 'table' && table.node.border_style, 'solid');
  assert.equal(table.node.type === 'table' && table.node.proportion, 100);
  assert.deepEqual(table.modifiers.alignment, { type: 'alignment', value: 'center' });
  assert.equal(tableCell.node.type, 'table_cell');
  assert.equal(tableCell.node.type === 'table_cell' && tableCell.node.col_width, 50);

  assert.equal(image.node.type, 'image');
  assert.equal(image.node.type === 'image' && image.node.id, 'IMG123');
  assert.equal(image.node.type === 'image' && image.node.proportion, 80);
});

test('빈 문단의 텍스트 런 cascade는 carry로 이관된다', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: { type: 'root', children: [id(1)] },
      [id(1)]: {
        type: 'paragraph',
        align: 'left',
        line_height: 160,
        parent: ROOT_ID,
        children: [],
        cascade_attrs: { 'style:bold': true, 'style:text_color': '#ff0000', 'paragraph:line_height': 200 },
      },
    },
  });

  const { plain, warnings } = convertLegacyDocumentJson(json);
  const [para] = plain.root.children;

  assert.deepEqual(
    para.carry?.toSorted((a, b) => a.type.localeCompare(b.type)),
    [{ type: 'bold' }, { type: 'text_color', value: '#ff0000' }],
  );
  assert.deepEqual(para.modifiers.line_height, { type: 'line_height', value: 200 });
  assert.deepEqual(warnings, []);
});

test('세그먼트가 스타일별 run으로 쪼개지고 탭은 tab 노드가 된다', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: { type: 'root', children: [id(1)] },
      [id(1)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [id(2)] },
      [id(2)]: {
        type: 'text',
        parent: id(1),
        text: [
          { text: 'plain ' },
          { text: 'bold', styles: [{ type: 'bold' }, { type: 'font_size', size: 2000 }] },
          { text: '\tafter', annotations: [{ type: 'link', href: 'https://typie.co' }] },
        ],
      },
    },
  });

  const { plain, warnings } = convertLegacyDocumentJson(json);
  const [paragraph] = plain.root.children;
  const runs = paragraph.children;

  assert.deepEqual(
    runs.map((r) => r.node.type),
    ['text', 'text', 'tab', 'text'],
  );
  assert.equal(runs[0].node.type === 'text' && runs[0].node.text, 'plain ');
  assert.deepEqual(Object.keys(runs[0].modifiers), []);
  assert.equal(runs[1].node.type === 'text' && runs[1].node.text, 'bold');
  assert.deepEqual(runs[1].modifiers.bold, { type: 'bold' });
  assert.deepEqual(runs[1].modifiers.font_size, { type: 'font_size', value: 2000 });
  assert.equal(runs[3].node.type === 'text' && runs[3].node.text, 'after');
  assert.deepEqual(runs[3].modifiers.link, { type: 'link', href: 'https://typie.co' });
  assert.equal(runs[2].modifiers.link, undefined);
  assert.deepEqual(warnings, ['link/ruby dropped from tab: v2 schema does not allow them on tab nodes']);
});

test('동일 스타일의 인접 세그먼트와 인접 text 노드는 하나의 run으로 병합된다', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: { type: 'root', children: [id(1)] },
      [id(1)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [id(2), id(3)] },
      [id(2)]: { type: 'text', parent: id(1), text: [{ text: 'ab' }, { text: 'cd' }, { text: 'ef', styles: [{ type: 'bold' }] }] },
      [id(3)]: { type: 'text', parent: id(1), text: [{ text: 'gh', styles: [{ type: 'bold' }] }] },
    },
  });

  const { plain } = convertLegacyDocumentJson(json);
  const [paragraph] = plain.root.children;
  const runs = paragraph.children;

  assert.deepEqual(
    runs.map((r) => [r.node.type === 'text' ? r.node.text : r.node.type, Object.keys(r.modifiers)]),
    [
      ['abcd', []],
      ['efgh', ['bold']],
    ],
  );
});

test('범위 밖 스타일 값은 클램프되고 none 색상은 드롭된다', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: { type: 'root', children: [id(1)] },
      [id(1)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [id(2)] },
      [id(2)]: {
        type: 'text',
        parent: id(1),
        text: [
          {
            text: 'x',
            styles: [
              { type: 'font_size', size: 99_999 },
              { type: 'font_weight', weight: 450 },
              { type: 'background_color', color: 'none' },
              { type: 'letter_spacing', spacing: 999 },
            ],
          },
        ],
      },
    },
  });

  const { plain, warnings } = convertLegacyDocumentJson(json);
  const [paragraph] = plain.root.children;
  const [run] = paragraph.children;

  assert.deepEqual(run.modifiers.font_size, { type: 'font_size', value: 12_800 });
  assert.deepEqual(run.modifiers.font_weight, { type: 'font_weight', value: 500 });
  assert.deepEqual(run.modifiers.letter_spacing, { type: 'letter_spacing', value: 200 });
  assert.equal('background_color' in run.modifiers, false);
  assert.ok(warnings.some((w) => w.includes('font_size')));
  assert.ok(warnings.some((w) => w.includes('letter_spacing')));
});

test('remark가 문서 순서대로 경로와 함께 수집되고 시간순 정렬된다', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: { type: 'root', children: [id(1), id(2)] },
      [id(1)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
      [id(2)]: {
        type: 'paragraph',
        align: 'left',
        line_height: 160,
        parent: ROOT_ID,
        children: [],
        remarks: {
          [id(100)]: { user_id: 'U2', text: '두번째', created_at: 2000 },
          [id(101)]: { user_id: 'U1', text: '첫번째', created_at: 1000 },
        },
      },
    },
  });

  const { remarkAnchors } = convertLegacyDocumentJson(json);

  assert.equal(remarkAnchors.length, 1);
  assert.deepEqual(remarkAnchors[0].path, [1]);
  assert.deepEqual(
    remarkAnchors[0].remarks.map((r) => [r.user_id, r.text, r.created_at]),
    [
      ['U1', '첫번째', 1000],
      ['U2', '두번째', 2000],
    ],
  );
});

test('remark가 구조 노드(root/fold_title/table_row/image)에서도 수집된다', async () => {
  const remark = (n: number) => ({ [id(200 + n)]: { user_id: `U${n}`, text: `r${n}`, created_at: 1000 * n } });
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: { type: 'root', children: [id(1), id(5), id(8), id(9)], remarks: remark(0) },
      [id(1)]: { type: 'fold', parent: ROOT_ID, children: [id(2), id(3)] },
      [id(2)]: { type: 'fold_title', parent: id(1), children: [], remarks: remark(1) },
      [id(3)]: { type: 'fold_content', parent: id(1), children: [id(4)] },
      [id(4)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(3), children: [] },
      [id(5)]: { type: 'table', border_style: 'solid', align: 'left', proportion: 1, parent: ROOT_ID, children: [id(6)] },
      [id(6)]: { type: 'table_row', parent: id(5), children: [id(7)], remarks: remark(2) },
      [id(7)]: { type: 'table_cell', parent: id(6), children: [id(10)] },
      [id(10)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(7), children: [] },
      [id(8)]: { type: 'image', id: 'IMG1', proportion: 1, parent: ROOT_ID, remarks: remark(3) },
      [id(9)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
    },
  });

  const { remarkAnchors } = convertLegacyDocumentJson(json);

  assert.deepEqual(
    remarkAnchors.map((a) => a.path),
    [[], [0, 0], [1, 0], [2]],
  );
});

test('expectedText가 extract_text 계약을 미러링한다', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: { type: 'root', children: [id(1), id(2)] },
      [id(1)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [id(3)] },
      [id(3)]: { type: 'text', parent: id(1), text: [{ text: 'a\tb' }] },
      [id(2)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [id(4), id(5), id(6)] },
      [id(4)]: { type: 'text', parent: id(2), text: [{ text: 'c' }] },
      [id(5)]: { type: 'hard_break', parent: id(2) },
      [id(6)]: { type: 'text', parent: id(2), text: [{ text: 'd' }] },
    },
  });

  const { plain } = convertLegacyDocumentJson(json);
  assert.equal(deriveExpectedTextFromPlain(plain), 'ab\ncd');
  assert.equal(collectLegacyTextChars(json), 'abcd');
});

test('상속 동일값 스타일은 런과 carry에 주입되지 않는다 (v2 상속 정규형)', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: {
        type: 'root',
        children: [id(1), id(4), id(6)],
        cascade_attrs: { 'style:font_family': 'Pretendard', 'style:font_size': 1600, 'style:font_weight': 400, 'style:letter_spacing': 0 },
      },
      [id(1)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [id(2)] },
      [id(2)]: {
        type: 'text',
        parent: id(1),
        text: [
          {
            text: 'pinned-but-equal',
            styles: [
              { type: 'font_size', size: 1600 },
              { type: 'font_weight', weight: 400 },
              { type: 'font_family', family: 'Iropke Batang OTF' },
              { type: 'bold' },
              { type: 'text_color', color: '#000000' },
            ],
          },
        ],
      },
      [id(4)]: {
        type: 'blockquote',
        variant: 'left_line',
        parent: ROOT_ID,
        children: [id(5), id(7)],
        cascade_attrs: { 'style:font_size': 2400 },
      },
      [id(5)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(4), children: [id(8)] },
      [id(8)]: { type: 'text', parent: id(5), text: [{ text: 'back to root size', styles: [{ type: 'font_size', size: 1600 }] }] },
      [id(7)]: { type: 'paragraph', align: 'left', line_height: 160, parent: id(4), children: [] },
      [id(6)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
    },
  });

  const { plain } = convertLegacyDocumentJson(json);

  const [para, blockquote] = plain.root.children;
  const [run] = para.children;
  assert.deepEqual(run.modifiers, {
    font_family: { type: 'font_family', value: 'Iropke Batang OTF' },
    bold: { type: 'bold' },
  });

  const [bqPara, bqEmpty] = blockquote.children;
  const [bqRun] = bqPara.children;
  assert.deepEqual(bqRun.modifiers, {});
  assert.deepEqual(bqEmpty.carry, [{ type: 'font_size', value: 2400 }]);
});

test('문서 기본 색은 v2에 없음: root 색상 cascade는 배치하지 않고 비기본값은 경고와 함께 유실된다', async () => {
  const json = await canonicalize({
    settings: baseSettings,
    nodes: {
      [ROOT_ID]: {
        type: 'root',
        children: [id(1)],
        cascade_attrs: { 'style:text_color': '#333333', 'style:background_color': '#fff8dc' },
      },
      [id(1)]: { type: 'paragraph', align: 'left', line_height: 160, parent: ROOT_ID, children: [] },
    },
  });

  const { plain, warnings } = convertLegacyDocumentJson(json);

  assert.equal('text_color' in plain.root.modifiers, false);
  assert.equal('background_color' in plain.root.modifiers, false);
  assert.equal(warnings.filter((w) => w.includes('document default')).length, 2);

  const [paragraph] = plain.root.children;
  assert.deepEqual(paragraph.carry, []);
});
