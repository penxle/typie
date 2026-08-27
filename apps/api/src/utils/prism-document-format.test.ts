import assert from 'node:assert/strict';
import { test } from 'node:test';
import { EditDocumentInput, toRustOps } from './prism-document-edit-core.ts';
import { DOCUMENT_FORMAT_GUIDE, DOCUMENT_FORMAT_PATH } from './prism-document-format.ts';
import { wasm } from './wasm-ffi.ts';

const ELEMENTS = [
  'root',
  'paragraph',
  'blockquote',
  'callout',
  'bullet_list',
  'ordered_list',
  'list_item',
  'fold',
  'fold_title',
  'fold_content',
  'table',
  'table_row',
  'table_cell',
  'image',
  'file',
  'embed',
  'archived',
  'hard_break',
  'horizontal_rule',
  'page_break',
  'tab',
  'unknown',
];

const MODIFIERS = [
  'bold',
  'italic',
  'underline',
  'strikethrough',
  'font_size',
  'font_family',
  'font_weight',
  'text_color',
  'background_color',
  'letter_spacing',
  'link',
  'ruby',
  'line_height',
  'block_gap',
  'paragraph_indent',
  'alignment',
];

const NODE_ATTRS = [
  'layout_mode',
  'max_width',
  'page_width',
  'page_height',
  'page_margin_top',
  'page_margin_bottom',
  'page_margin_left',
  'page_margin_right',
  'variant',
  'border_style',
  'proportion',
  'col_width',
  'id',
];

const ENUM_VALUES = [
  'left_line',
  'left_quote',
  'message_sent',
  'message_received',
  'info',
  'success',
  'warning',
  'danger',
  'solid',
  'dashed',
  'dotted',
  'none',
  'line',
  'dashed_line',
  'circle_line',
  'diamond_line',
  'circle',
  'diamond',
  'three_circles',
  'three_diamonds',
  'zigzag',
  'left',
  'center',
  'right',
  'justify',
  'continuous',
  'paginated',
];

const xmlExamples = (guide: string): string[] => [...guide.matchAll(/```xml\n([\s\S]*?)```/g)].map((m) => m[1]);

test('안내 파일 경로는 문서 파일이 아니다', () => {
  assert.equal(DOCUMENT_FORMAT_PATH, 'documents/README.md');
  assert.doesNotMatch(DOCUMENT_FORMAT_PATH, /\.xml$/);
});

test('안내는 요소·서식·속성·열거값 어휘를 전부 담는다', () => {
  for (const name of [...ELEMENTS, ...MODIFIERS, ...NODE_ATTRS, ...ENUM_VALUES]) {
    assert.ok(DOCUMENT_FORMAT_GUIDE.includes(`\`${name}\``), name);
  }
});

test('안내의 xml 예시는 전부 실제 검증기를 통과한다', async () => {
  const examples = xmlExamples(DOCUMENT_FORMAT_GUIDE);
  assert.ok(examples.length >= 7);

  const rootTag = await wasm.use((host) => {
    const plain = host.default_doc_with_preset({ layout_mode: { type: 'continuous', max_width: 600 } }, []);
    const graph = host.to_graph(plain);
    const rendered = host.to_xml(graph, []);
    assert.equal(rendered.error, undefined);
    return rendered.xml.slice(0, rendered.xml.indexOf('>') + 1);
  });
  assert.match(rootTag, /^<root dot="[^"]+" base="[^"]+"/);

  for (const example of examples) {
    const verdict = await wasm.verify_xml(`${rootTag}\n${example}<paragraph/>\n</root>`);
    assert.equal(verdict.error, undefined, `${example}\n${JSON.stringify(verdict.error)}`);
  }
});

test('안내에 있는 잘못된 형태는 실제로 거절된다', async () => {
  const rootTag = await wasm.use((host) => {
    const plain = host.default_doc_with_preset({ layout_mode: { type: 'continuous', max_width: 600 } }, []);
    const rendered = host.to_xml(host.to_graph(plain), []);
    return rendered.xml.slice(0, rendered.xml.indexOf('>') + 1);
  });
  const rejected = async (inner: string) => {
    const verdict = await wasm.verify_xml(`${rootTag}${inner}</root>`);
    assert.notEqual(verdict.error, undefined, inner);
  };

  await rejected('<paragraph><b>x</b></paragraph><paragraph/>');
  await rejected('<paragraph>a\nb</paragraph><paragraph/>');
  await rejected('<table><table_row><table_cell><paragraph/></table_cell></table_row></table>');
  await rejected(
    '<table><table_row><table_cell><paragraph/></table_cell><table_cell><paragraph/></table_cell></table_row><table_row><table_cell><paragraph/></table_cell></table_row></table><paragraph/>',
  );
  await rejected('<fold><fold_title><bold>x</bold></fold_title><fold_content><paragraph/></fold_content></fold><paragraph/>');
  await rejected('<paragraph>끝<page_break/></paragraph>');
  await rejected('<paragraph><font_size value="99">x</font_size></paragraph><paragraph/>');
  await rejected('<paragraph><background_color value="none">x</background_color></paragraph><paragraph/>');
  await rejected('<image/><paragraph/>');
});

const jsonExamples = (guide: string): string[] => [...guide.matchAll(/```json\n([\s\S]*?)```/g)].map((m) => m[1]);

test('안내의 연산 예시는 빈 문서에 차례로 적용되고 결과가 검증기를 통과한다', async () => {
  const examples = jsonExamples(DOCUMENT_FORMAT_GUIDE);
  assert.equal(examples.length, 5);

  let xml = await wasm.use((host) => {
    const plain = host.default_doc_with_preset({ layout_mode: { type: 'continuous', max_width: 600 } }, []);
    const rendered = host.to_xml(host.to_graph(plain), []);
    assert.equal(rendered.error, undefined);
    return rendered.xml;
  });
  for (const example of examples) {
    const parsed = EditDocumentInput.safeParse({ path: 'documents/x.xml', ops: JSON.parse(example) });
    assert.ok(parsed.success, example);
    const result = await wasm.edit_xml(xml, JSON.stringify(toRustOps(parsed.data.ops)));
    assert.equal(result.error, undefined, `${example}\n${JSON.stringify(result.error)}`);
    xml = result.xml;
  }
  const outline = await wasm.outline_xml(xml, 'root', 8, 0, 500, false);
  assert.deepEqual(
    outline.rows.map((r) => `${r.path} ${r.name}`),
    [
      '1 table',
      '1.1 table_row',
      '1.1.1 table_cell',
      '1.1.1.1 paragraph',
      '1.1.2 table_cell',
      '1.1.2.1 paragraph',
      '2 blockquote',
      '2.1 paragraph',
      '3 paragraph',
    ],
  );
  const preview = (path: string) => outline.rows.find((r) => r.path === path)?.preview;
  assert.equal(preview('2.1'), '셋째');
  assert.equal(preview('1.1.1.1'), '왼쪽');
  assert.equal(preview('1.1.2.1'), '오른쪽');
  const verdict = await wasm.verify_xml(xml);
  assert.equal(verdict.error, undefined);
});

test('안내는 도구 이름·주소 문법·연산 다섯을 말한다', () => {
  for (const word of [
    'outline-document',
    'edit-document',
    '`root`',
    '서수 경로',
    'first_child',
    'last_child',
    '`insert`',
    '`delete`',
    '`move`',
    '`replace`',
    '`set`',
  ]) {
    assert.ok(DOCUMENT_FORMAT_GUIDE.includes(word), word);
  }
});
