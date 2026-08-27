import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  AFFECTED_ROW_CAP,
  AT_MESSAGE,
  changedOf,
  documentIdOf,
  documentPath,
  EditDocumentInput,
  FULL_TOO_LARGE_MESSAGE,
  messageOf,
  opErrorMessage,
  OUTLINE_FULL_MAX_CHARS,
  OutlineDocumentInput,
  renderAffected,
  renderOutline,
  renderRow,
  SAVE_TOO_LARGE_MESSAGE,
  SaveDocumentInput,
  TOO_LARGE_MESSAGE,
  toRustOps,
  XML_DETAIL_TYPES,
  XML_MESSAGES,
} from './prism-document-edit-core.ts';
import type { XmlErrorInfo, XmlOutlineRow } from '@typie/editor-ffi/server';

test('경로는 documentId와 왕복한다', () => {
  assert.equal(documentPath('D0ABC'), 'documents/D0ABC.xml');
  assert.equal(documentIdOf('documents/D0ABC.xml'), 'D0ABC');
  assert.equal(documentIdOf('manuscript/v1.txt'), null);
  assert.equal(documentIdOf('documents/a/b.xml'), null);
});

test('저장 입력은 경로 규약과 빈 summary를 거른다', () => {
  assert.ok(SaveDocumentInput.safeParse({ path: 'documents/D0ABC.xml', summary: '오탈자 셋을 고쳤어요' }).success);
  assert.ok(!SaveDocumentInput.safeParse({ path: 'documents/D0ABC.txt', summary: 'x' }).success);
  assert.ok(!SaveDocumentInput.safeParse({ path: 'documents/D0ABC.xml', summary: ' '.repeat(3) }).success);
});

test('문면 표는 detail.type 73종을 빠짐없이 덮는다', () => {
  assert.equal(XML_DETAIL_TYPES.length, 73);
  const byName = (a: string, b: string) => a.localeCompare(b);
  assert.deepEqual(Object.keys(XML_MESSAGES).toSorted(byName), XML_DETAIL_TYPES.toSorted(byName));
});

test('상계 문면은 열기와 저장을 나눈다', () => {
  assert.notEqual(SAVE_TOO_LARGE_MESSAGE, TOO_LARGE_MESSAGE);
});

test('messageOf는 좌표와 dot을 접두·병기한다', () => {
  const info = {
    line: 12,
    column: 8,
    dot: undefined,
    detail: JSON.stringify({ type: 'close_without_open', name: 'bold', open: 'italic' }),
    message: 'x',
  };
  assert.equal(messageOf(info), '줄 12 열 8: `</bold>`가 닫을 요소가 없어요 — 지금 열린 요소는 <italic>이에요');

  const dotted = {
    line: 9,
    column: 1,
    dot: '1_5',
    detail: JSON.stringify({ type: 'opaque_id_changed', element: 'image', dot: '1_5' }),
    message: 'x',
  };
  assert.equal(messageOf(dotted), '줄 9 열 1: `<image>`의 `attr:id`는 바꿀 수 없어요 (dot 1_5)');

  const bare = {
    line: undefined,
    column: undefined,
    dot: undefined,
    detail: JSON.stringify({ type: 'base_not_in_history' }),
    message: 'x',
  };
  assert.equal(messageOf(bare), '이 파일은 지금 문서와 이어지지 않아요 — 문서를 다시 여세요');

  const filled = {
    line: 5,
    column: 5,
    dot: undefined,
    detail: JSON.stringify({ type: 'content_rule', parent: 'table_row', allowed: ['table_cell'], got: ['paragraph'], rule: 'TableCell+' }),
    message: 'x',
  };
  assert.equal(messageOf(filled), '줄 5 열 5: `<table_row>` 안에 올 수 있는 것은 <table_cell>이에요 — 지금은 <paragraph>이 있어요');

  const empty = {
    ...filled,
    detail: JSON.stringify({ type: 'content_rule', parent: 'table_row', allowed: ['table_cell'], got: [], rule: 'TableCell+' }),
  };
  assert.equal(messageOf(empty), '줄 5 열 5: `<table_row>` 안에 올 수 있는 것은 <table_cell>이에요 — 지금은 비어 있어요');

  const unknown = { line: undefined, column: undefined, dot: undefined, detail: '{"type":"???"}', message: 'fallback' };
  assert.equal(messageOf(unknown), '이 편집은 적용할 수 없어요 — 문서를 다시 열고 다시 시도하세요');

  const scalar = { line: undefined, column: undefined, dot: undefined, detail: 'null', message: 'fallback' };
  assert.equal(messageOf(scalar), '이 편집은 적용할 수 없어요 — 문서를 다시 열고 다시 시도하세요');
});

test('코드포인트는 생산자와 같은 네 자리 16진으로 적는다', () => {
  const control = {
    line: undefined,
    column: undefined,
    dot: undefined,
    detail: JSON.stringify({ type: 'forbidden_control_char', codepoint: 11 }),
    message: 'x',
  };
  assert.equal(messageOf(control), '쓸 수 없는 제어 문자(U+000B)가 있어요');

  const missing = { ...control, detail: JSON.stringify({ type: 'forbidden_control_char' }) };
  assert.equal(messageOf(missing), '쓸 수 없는 제어 문자(U+?)가 있어요');
});

test('changedOf는 집계 6필드를 두 묶음으로 사영한다', () => {
  assert.deepEqual(
    changedOf({
      error: undefined,
      bundle: new Uint8Array(),
      xml: '',
      blocks_inserted: 1,
      blocks_deleted: 2,
      blocks_moved: 3,
      blocks_updated: 4,
      chars_inserted: 5,
      chars_deleted: 6,
    }),
    { blocks: { inserted: 1, deleted: 2, moved: 3, updated: 4 }, chars: { inserted: 5, deleted: 6 } },
  );
});

const REPRESENTATIVE: Record<string, Record<string, unknown>> = {
  declaration: {},
  comment_or_dtd: {},
  close_without_open: { name: 'bold', open: 'italic' },
  close_tag_unterminated: { name: 'bold' },
  self_close_unterminated: {},
  attr_missing_equals: { attr: 'value' },
  attr_unquoted: { attr: 'value' },
  attr_duplicate: { attr: 'value' },
  illegal_char_in_tag: {},
  tag_unterminated: { name: 'paragraph' },
  name_expected: {},
  unterminated_quote: {},
  lt_in_attr_value: {},
  forbidden_control_char: { codepoint: 11 },
  unknown_entity: {},
  bad_numeric_reference: {},
  element_unclosed: { name: 'paragraph' },
  root_missing: {},
  root_not_root: { name: 'paragraph' },
  trailing_content: {},
  multiple_roots: {},
  unknown_element: { name: 'em', hint: 'italic' },
  unknown_attribute: { element: 'paragraph', attr: 'class' },
  base_on_non_root: {},
  unknown_modifier: { prefix: 'mod', name: 'font_color' },
  modifier_not_carry_kind: { name: 'alignment' },
  carry_on_non_textblock: { element: 'blockquote' },
  value_not_integer: { value: '1.5' },
  value_out_of_range: { modifier: 'font_weight', value: '150' },
  enum_value_unknown: { value: 'middle' },
  node_attr_missing: { element: 'image', field: 'id' },
  node_attr_unknown: { element: 'paragraph', field: 'id' },
  node_attr_not_unsigned_integer: { element: 'table', field: 'proportion' },
  layout_mode_invalid: { value: 'fixed' },
  atom_attr_not_allowed: { element: 'hard_break', attr: 'value' },
  atom_has_content: { element: 'hard_break' },
  inline_modifier_attr_not_allowed: { element: 'bold', attr: 'value' },
  inline_modifier_attr_missing: { element: 'link', attr: 'href' },
  text_in_container: { element: 'blockquote' },
  block_inside_textblock: { parent: 'paragraph', child: 'blockquote' },
  content_rule: { parent: 'table_row', allowed: ['table_cell'], got: ['paragraph'], rule: 'TableCell+' },
  context_not_allowed: { element: 'page_break' },
  trailing_page_break: {},
  table_not_rectangular: { expected: 3, got: 2 },
  block_modifier_not_allowed: { modifier: 'alignment', element: 'blockquote' },
  inline_modifier_not_allowed: { modifier: 'link', leaf: 'hard_break' },
  newline_in_text: {},
  tab_in_text: {},
  forbidden_char_in_document: { codepoint: 0xff_fe },
  dot_invalid: { value: 'zz' },
  dot_duplicate: { dot: '1_5' },
  dot_not_in_document: { dot: '1_5' },
  dot_type_incompatible: { dot: '1_5', new_type: 'blockquote' },
  root_dot_mismatch: {},
  opaque_needs_dot: { element: 'image' },
  opaque_has_children: { element: 'image' },
  opaque_id_changed: { element: 'image', dot: '1_5' },
  base_missing: {},
  base_undecodable: {},
  base_not_in_history: {},
  projection_degraded: {},
  address_invalid: { value: 'nope' },
  address_unresolved: { value: '9_9' },
  root_not_editable: {},
  root_has_no_siblings: {},
  target_not_container: { element: 'paragraph' },
  move_into_self: { target: '2.1' },
  targets_nested: { outer: '2', inner: '2.1' },
  fragment_empty: {},
  fragment_not_block: {},
  fragment_not_single: { count: 2 },
  set_key_unknown: { key: 'dot' },
  internal: { message: 'text is not an element' },
};

const BOUNDARY_ROWS: { types?: string[]; detail: Record<string, unknown> }[] = [
  { detail: {} },
  { types: ['close_without_open'], detail: { open: null } },
  { types: ['unknown_element'], detail: { hint: null } },
  { types: ['content_rule'], detail: { got: [] } },
  { types: ['content_rule'], detail: { allowed: [] } },
  { types: ['content_rule'], detail: { allowed: ['paragraph', 'table'], got: ['paragraph', 'table'] } },
  { types: ['table_not_rectangular'], detail: { expected: 1, got: 0 } },
  { types: ['forbidden_control_char', 'forbidden_char_in_document'], detail: { codepoint: 0 } },
  { types: ['forbidden_control_char', 'forbidden_char_in_document'], detail: { codepoint: 0x10_ff_ff } },
  { types: ['value_not_integer', 'enum_value_unknown', 'layout_mode_invalid', 'dot_invalid'], detail: { value: '' } },
  { types: ['value_out_of_range'], detail: { value: '' } },
  { types: ['node_attr_unknown'], detail: { field: '' } },
  { types: ['unknown_modifier'], detail: { name: '' } },
];

const DEFECT_PATTERNS = [/undefined/, /\bnull\b/, /NaN/, /<>/, / {2}/, / 이 있어요/, /은 이에요/, /^\s|\s$/, /`<>`/, /``/];

const COORDS = [
  { line: undefined, column: undefined, dot: undefined },
  { line: 3, column: 7, dot: '1_5' },
];

test('73종 문면은 파서가 내는 경계 입력에서도 깨지지 않는다', () => {
  const byName = (a: string, b: string) => a.localeCompare(b);
  assert.deepEqual(Object.keys(REPRESENTATIVE).toSorted(byName), XML_DETAIL_TYPES.toSorted(byName));
  const defects: string[] = [];
  for (const type of XML_DETAIL_TYPES) {
    for (const row of BOUNDARY_ROWS) {
      if (row.types && !row.types.includes(type)) {
        continue;
      }
      for (const coords of COORDS) {
        const detail = JSON.stringify({ type, ...REPRESENTATIVE[type], ...row.detail });
        const rendered = messageOf({ ...coords, detail, message: 'x' });
        for (const pattern of DEFECT_PATTERNS) {
          if (pattern.test(rendered)) {
            defects.push(`${detail} ${String(pattern)} => ${rendered}`);
          }
        }
      }
    }
  }
  assert.deepEqual(defects, []);
});

test('detail 필드가 통째로 빠져도 문면은 예외 없이 나온다', () => {
  for (const type of XML_DETAIL_TYPES) {
    const rendered = messageOf({ line: undefined, column: undefined, dot: undefined, detail: JSON.stringify({ type }), message: 'x' });
    assert.ok(rendered.length > 0, type);
  }
});

test('renderRow: 경로·요소·dot·속성·미리보기·글자 수·접힌 자식을 한 줄로 만든다', () => {
  const row = (over: Partial<XmlOutlineRow>): XmlOutlineRow => ({
    path: '3.1',
    name: 'paragraph',
    dot: 'Ab1_9y',
    attrs: [],
    preview: undefined,
    chars: undefined,
    children: 0,
    ...over,
  });
  assert.equal(renderRow(row({ preview: '인용 안 문단', chars: 412 })), '3.1 <paragraph dot=Ab1_9y> 인용 안 문단 (412자)');
  assert.equal(
    renderRow(row({ name: 'table', attrs: [{ key: 'attr:proportion', value: '80' }], children: 3 })),
    '3.1 <table dot=Ab1_9y attr:proportion="80"> 자식 3',
  );
  assert.equal(renderRow(row({ dot: undefined, preview: '', chars: 0 })), '3.1 <paragraph> (0자)');
  assert.equal(
    renderRow(row({ name: 'horizontal_rule', attrs: [{ key: 'attr:variant', value: 'plain' }] })),
    '3.1 <horizontal_rule dot=Ab1_9y attr:variant="plain">',
  );
});

test('renderOutline: 창 머리와 잘림 안내, 머리 행, 빈 구간', () => {
  const rows = [
    { path: '1', name: 'paragraph', dot: 'A_1', attrs: [], preview: 'a', chars: 1, children: 0 },
    { path: '2', name: 'blockquote', dot: 'A_2', attrs: [], preview: undefined, chars: undefined, children: 2 },
  ];
  const paged = renderOutline({ error: undefined, head: undefined, rows, total: 5, xml: undefined }, 0);
  assert.equal(paged.split('\n')[0], '[0~1 / 전체 5행] (상한으로 잘림 — offset=2로 이어 읽으세요)');
  assert.equal(paged.split('\n').length, 3);
  const whole = renderOutline({ error: undefined, head: rows[1], rows: [rows[0]], total: 1, xml: undefined }, 0);
  assert.deepEqual(whole.split('\n'), ['[0~0 / 전체 1행]', '2 <blockquote dot=A_2> 자식 2', '1 <paragraph dot=A_1> a (1자)']);
  assert.equal(
    renderOutline({ error: undefined, head: undefined, rows: [], total: 0, xml: undefined }, 0),
    '[전체 0행] 이 아래에는 블록이 없어요',
  );
  assert.equal(
    renderOutline({ error: undefined, head: undefined, rows: [], total: 3, xml: undefined }, 7),
    '[past end: offset 7, 전체 3행]',
  );
  assert.equal(renderOutline({ error: undefined, head: undefined, rows: [], total: 0, xml: '<root/>\n' }, 0), '<root/>\n');
});

test('full 상한은 핸들러가 재고, renderOutline은 긴 xml도 그대로 돌려준다', () => {
  const xml = `<root>${'x'.repeat(OUTLINE_FULL_MAX_CHARS)}</root>`;
  assert.ok(xml.length > OUTLINE_FULL_MAX_CHARS);
  assert.equal(renderOutline({ error: undefined, head: undefined, rows: [], total: 0, xml }, 0), xml);
  assert.notEqual(FULL_TOO_LARGE_MESSAGE, TOO_LARGE_MESSAGE);
});

test('renderAffected: 적용 수와 부모별 직계 자식, 합계 200행 상한', () => {
  const row = (i: number): XmlOutlineRow => ({
    path: String(i),
    name: 'paragraph',
    dot: undefined,
    attrs: [],
    preview: 'x',
    chars: 1,
    children: 0,
  });
  const many = Array.from({ length: 150 }, (_, i) => row(i + 1));
  const one = { error: undefined, head: undefined, rows: many, total: 150, xml: undefined };
  const text = renderAffected(2, [one, one]);
  const lines = text.split('\n');
  assert.equal(lines[0], '2개 연산을 적용했어요.');
  assert.equal(lines.filter((l) => /^\d+ <paragraph>/.test(l)).length, 200);
  assert.equal(lines.at(-1), '(행이 많아 200행에서 줄였어요 — 나머지는 outline-document로 보세요)');
  assert.equal(
    renderAffected(1, [
      { error: undefined, head: { ...row(2), name: 'blockquote', children: 0 }, rows: [row(1)], total: 1, xml: undefined },
    ]),
    '1개 연산을 적용했어요.\n\n2 <blockquote> x (1자)\n1 <paragraph> x (1자)',
  );

  const brimmed = {
    error: undefined,
    head: undefined,
    rows: Array.from({ length: AFFECTED_ROW_CAP }, (_, i) => row(i + 1)),
    total: AFFECTED_ROW_CAP,
    xml: undefined,
  };
  const next = { error: undefined, head: row(999), rows: [row(1000)], total: 1, xml: undefined };
  const boundary = renderAffected(2, [brimmed, next]).split('\n');
  assert.equal(boundary.at(-1), '(행이 많아 200행에서 줄였어요 — 나머지는 outline-document로 보세요)');
  assert.equal(boundary.at(-2), renderRow(row(AFFECTED_ROW_CAP)));
});

test('opErrorMessage: op 귀속이면 ops[k], 아니면 주소, 둘 다 없으면 줄·열을 붙인 본문', () => {
  const info = (type: string, extra: Record<string, unknown> = {}): XmlErrorInfo => ({
    line: 5,
    column: 5,
    dot: undefined,
    detail: JSON.stringify({ type, ...extra }),
    message: '',
  });
  assert.equal(
    opErrorMessage({ op: 1, address: undefined, info: info('address_unresolved', { value: '9_9' }) }),
    'ops[1]: `9_9`에 해당하는 블록이 없어요 — outline-document로 지금 주소를 확인하세요',
  );
  assert.equal(
    opErrorMessage({ op: undefined, address: '1_2', info: info('table_not_rectangular', { expected: 2, got: 1 }) }),
    '1_2: 표의 모든 행은 셀 수가 같아야 해요 — 이 행은 2개 중 1개예요',
  );
  assert.equal(
    opErrorMessage({ op: undefined, address: undefined, info: info('base_missing') }),
    '줄 5 열 5: `base`가 없어요 — 문서를 다시 여세요',
  );
});

test('EditDocumentInput: op별 필수 필드와 at의 정확히 하나, ops 1~100', () => {
  const ok = EditDocumentInput.safeParse({
    path: 'documents/d1.xml',
    ops: [
      { op: 'insert', xml: '<paragraph/>', at: { after: '1' } },
      { op: 'delete', targets: ['1'] },
      { op: 'move', targets: ['1'], at: { first_child: 'root' } },
      { op: 'replace', target: '1', xml: '<paragraph/>' },
      {
        op: 'set',
        targets: ['1'],
        attrs: [{ key: 'mod:alignment', value: 'center' }, { key: 'carry:font_size', value: null }, { key: 'carry:bold' }],
      },
    ],
  });
  assert.ok(ok.success);
  const bad = (ops: unknown[]) => assert.equal(EditDocumentInput.safeParse({ path: 'documents/d1.xml', ops }).success, false);
  const badAt = (ops: unknown[]) => {
    const result = EditDocumentInput.safeParse({ path: 'documents/d1.xml', ops });
    assert.ok(!result.success);
    assert.ok(result.error.issues.some((issue) => issue.message === AT_MESSAGE || issue.path.includes('at')));
  };
  bad([]);
  bad(Array.from({ length: 101 }, () => ({ op: 'delete', targets: ['1'] })));
  badAt([{ op: 'insert', xml: '<paragraph/>' }]);
  bad([{ op: 'insert', xml: '<paragraph/>', at: {} }]);
  badAt([{ op: 'insert', xml: '<paragraph/>', at: { before: '1', after: '2' } }]);
  badAt([{ op: 'insert', xml: '<paragraph/>', at: { before: 123 } }]);
  bad([{ op: 'delete', targets: [] }]);
  bad([{ op: 'set', targets: ['1'], attrs: [] }]);
  bad([{ op: 'nope' }]);
  assert.deepEqual(toRustOps(ok.data.ops)[4], {
    op: 'set',
    targets: ['1'],
    attrs: [
      { key: 'mod:alignment', value: 'center' },
      { key: 'carry:font_size', value: null },
      { key: 'carry:bold', value: null },
    ],
  });
});

test('OutlineDocumentInput: 범위와 기본값 없음(기본값은 핸들러가 채운다)', () => {
  assert.ok(OutlineDocumentInput.safeParse({ path: 'documents/d1.xml' }).success);
  assert.ok(
    OutlineDocumentInput.safeParse({ path: 'documents/d1.xml', under: '2.1', depth: 8, offset: 0, limit: 500, full: true }).success,
  );
  assert.equal(OutlineDocumentInput.safeParse({ path: 'documents/d1.xml', depth: 0 }).success, false);
  assert.equal(OutlineDocumentInput.safeParse({ path: 'documents/d1.xml', depth: 9 }).success, false);
  assert.equal(OutlineDocumentInput.safeParse({ path: 'documents/d1.xml', limit: 501 }).success, false);
  assert.equal(OutlineDocumentInput.safeParse({ path: 'notes/x.xml' }).success, false);
});
