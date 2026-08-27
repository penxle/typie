import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  changedOf,
  documentIdOf,
  documentPath,
  messageOf,
  SAVE_TOO_LARGE_MESSAGE,
  SaveDocumentInput,
  TOO_LARGE_MESSAGE,
  XML_DETAIL_TYPES,
  XML_MESSAGES,
} from './prism-document-edit-core.ts';

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

test('문면 표는 detail.type 62종을 빠짐없이 덮는다', () => {
  assert.equal(XML_DETAIL_TYPES.length, 62);
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

test('62종 문면은 파서가 내는 경계 입력에서도 깨지지 않는다', () => {
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
