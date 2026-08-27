import { z } from 'zod';
import type { XmlEditResult, XmlErrorInfo } from '@typie/editor-ffi/server';

const PATH = /^documents\/([^/]+)\.xml$/;

export const OpenDocumentInput = z.object({ id: z.string() });
export const SaveDocumentInput = z.object({
  path: z.string().regex(PATH),
  summary: z.string().trim().min(1).regex(/\S/),
});

export const documentPath = (documentId: string): string => `documents/${documentId}.xml`;
export const documentIdOf = (path: string): string | null => PATH.exec(path)?.[1] ?? null;

export const NO_FILE_MESSAGE = '먼저 open-document로 문서를 여세요.';
export const TOO_LARGE_MESSAGE = '이 문서는 너무 커서 열 수 없어요.';
export const SAVE_TOO_LARGE_MESSAGE = '이 문서는 너무 커서 저장할 수 없어요.';
export const REWRITE_FAILED_MESSAGE = '저장하지 못했어요 — 잠시 뒤 다시 저장하세요.';
const INTERNAL_MESSAGE = '이 편집은 적용할 수 없어요 — 문서를 다시 열고 다시 시도하세요';

type Detail = Record<string, unknown>;

const s = (v: unknown): string => String(v ?? '');
const list = (v: unknown): string => (Array.isArray(v) ? v.map((x) => `<${String(x)}>`).join('·') : s(v));
const hex = (v: unknown): string => {
  const codepoint = Number(v);
  return Number.isFinite(codepoint) ? codepoint.toString(16).toUpperCase().padStart(4, '0') : '?';
};

export const XML_DETAIL_TYPES: readonly string[] = [
  'declaration',
  'comment_or_dtd',
  'close_without_open',
  'close_tag_unterminated',
  'self_close_unterminated',
  'attr_missing_equals',
  'attr_unquoted',
  'attr_duplicate',
  'illegal_char_in_tag',
  'tag_unterminated',
  'name_expected',
  'unterminated_quote',
  'lt_in_attr_value',
  'forbidden_control_char',
  'unknown_entity',
  'bad_numeric_reference',
  'element_unclosed',
  'root_missing',
  'root_not_root',
  'trailing_content',
  'multiple_roots',
  'unknown_element',
  'unknown_attribute',
  'base_on_non_root',
  'unknown_modifier',
  'modifier_not_carry_kind',
  'carry_on_non_textblock',
  'value_not_integer',
  'value_out_of_range',
  'enum_value_unknown',
  'node_attr_missing',
  'node_attr_unknown',
  'node_attr_not_unsigned_integer',
  'layout_mode_invalid',
  'atom_attr_not_allowed',
  'atom_has_content',
  'inline_modifier_attr_not_allowed',
  'inline_modifier_attr_missing',
  'text_in_container',
  'block_inside_textblock',
  'content_rule',
  'context_not_allowed',
  'trailing_page_break',
  'table_not_rectangular',
  'block_modifier_not_allowed',
  'inline_modifier_not_allowed',
  'newline_in_text',
  'tab_in_text',
  'forbidden_char_in_document',
  'dot_invalid',
  'dot_duplicate',
  'dot_not_in_document',
  'dot_type_incompatible',
  'root_dot_mismatch',
  'opaque_needs_dot',
  'opaque_has_children',
  'opaque_id_changed',
  'base_missing',
  'base_undecodable',
  'base_not_in_history',
  'projection_degraded',
  'internal',
];

export const XML_MESSAGES: Partial<Record<string, (d: Detail) => string>> = {
  declaration: () => 'xml 선언은 쓸 수 없어요 — 파일은 `<root>`로 바로 시작해요',
  comment_or_dtd: () => '주석·CDATA·DTD는 쓸 수 없어요',
  close_without_open: (d) => `\`</${s(d.name)}>\`가 닫을 요소가 없어요${d.open ? ` — 지금 열린 요소는 <${s(d.open)}>이에요` : ''}`,
  close_tag_unterminated: (d) => `\`</${s(d.name)}\`가 \`>\`로 닫히지 않았어요`,
  self_close_unterminated: () => '`/` 다음에는 `>`가 와야 해요',
  attr_missing_equals: (d) => `\`${s(d.attr)}\`에 \`="값"\`이 없어요`,
  attr_unquoted: (d) => `\`${s(d.attr)}\` 값은 따옴표로 감싸야 해요`,
  attr_duplicate: (d) => `\`${s(d.attr)}\`가 두 번 나와요`,
  illegal_char_in_tag: () => '태그 안에 쓸 수 없는 문자가 있어요',
  tag_unterminated: (d) => `\`<${s(d.name)}\`가 \`>\`로 닫히지 않았어요`,
  name_expected: () => '요소 이름이 있어야 해요',
  unterminated_quote: () => '속성 값의 따옴표가 닫히지 않았어요',
  lt_in_attr_value: () => '속성 값 안의 `<`는 `&lt;`로 써야 해요',
  forbidden_control_char: (d) => `쓸 수 없는 제어 문자(U+${hex(d.codepoint)})가 있어요`,
  unknown_entity: () => '그 문자 참조는 쓸 수 없어요 — `&lt;` `&gt;` `&amp;` `&quot;` `&apos;`와 숫자 참조만 돼요',
  bad_numeric_reference: () => '숫자 문자 참조가 올바르지 않아요',
  element_unclosed: (d) => `\`<${s(d.name)}>\`가 닫히지 않았어요`,
  root_missing: () => '파일은 `<root>` 하나로 시작해야 해요',
  root_not_root: (d) => `파일의 바깥은 \`<${s(d.name)}>\`이 아니라 \`<root>\`여야 해요`,
  trailing_content: () => '`</root>` 뒤에 내용이 있어요',
  multiple_roots: () => '`<root>`는 한 번만 쓸 수 있어요',
  unknown_element: (d) => `\`<${s(d.name)}>\`은 쓸 수 없어요${d.hint ? ` — ${s(d.hint)}` : ''}`,
  unknown_attribute: (d) => `\`<${s(d.element)}>\`에는 \`${s(d.attr)}\` 속성을 쓸 수 없어요`,
  base_on_non_root: () => '`base`는 `<root>`에만 쓸 수 있어요',
  unknown_modifier: (d) => `\`${s(d.prefix)}:${s(d.name)}\`는 없는 수정자예요`,
  modifier_not_carry_kind: (d) => `\`${s(d.name)}\`은 \`carry:\`로 쓸 수 없어요`,
  carry_on_non_textblock: (d) => `\`carry:*\`는 문단과 접기 제목에만 쓸 수 있어요(\`<${s(d.element)}>\` 불가)`,
  value_not_integer: (d) => (s(d.value) === '' ? '빈 값은 정수가 아니에요' : `\`${s(d.value)}\`는 정수가 아니에요`),
  value_out_of_range: (d) =>
    s(d.value) === '' ? `\`${s(d.modifier)}\` 값은 비어 있을 수 없어요` : `\`${s(d.modifier)}\` 값 \`${s(d.value)}\`는 허용 범위 밖이에요`,
  enum_value_unknown: (d) => (s(d.value) === '' ? '빈 값은 쓸 수 없어요' : `\`${s(d.value)}\`는 쓸 수 없는 값이에요`),
  node_attr_missing: (d) => `\`<${s(d.element)}>\`에는 \`attr:${s(d.field)}\`가 있어야 해요`,
  node_attr_unknown: (d) => `\`<${s(d.element)}>\`에는 \`attr:${s(d.field)}\`가 없어요`,
  node_attr_not_unsigned_integer: (d) => `\`<${s(d.element)}>\`의 \`attr:${s(d.field)}\`는 0 이상의 정수여야 해요`,
  layout_mode_invalid: (d) =>
    `\`attr:layout_mode\`는 \`continuous\` 또는 \`paginated\`여야 해요(${s(d.value) === '' ? '빈 값' : `\`${s(d.value)}\``} 불가)`,
  atom_attr_not_allowed: (d) => `\`<${s(d.element)}>\`에는 \`${s(d.attr)}\`를 쓸 수 없어요`,
  atom_has_content: (d) => `\`<${s(d.element)}>\`는 비어 있어야 해요 — \`<${s(d.element)}/>\`로 쓰세요`,
  inline_modifier_attr_not_allowed: (d) => `\`<${s(d.element)}>\`에는 \`${s(d.attr)}\`를 쓸 수 없어요`,
  inline_modifier_attr_missing: (d) => `\`<${s(d.element)}>\`에는 \`${s(d.attr)}="…"\`가 있어야 해요`,
  text_in_container: (d) => `\`<${s(d.element)}>\` 안의 글자는 \`<paragraph>\`로 감싸야 해요`,
  block_inside_textblock: (d) => `\`<${s(d.parent)}>\` 안에는 \`<${s(d.child)}>\`를 넣을 수 없어요`,
  content_rule: (d) => {
    const allowed =
      Array.isArray(d.allowed) && d.allowed.length === 0
        ? `\`<${s(d.parent)}>\` 안에는 내용을 넣을 수 없어요`
        : `\`<${s(d.parent)}>\` 안에 올 수 있는 것은 ${list(d.allowed)}이에요`;
    const got = Array.isArray(d.got) && d.got.length === 0 ? '비어 있어요' : `${list(d.got)}이 있어요`;
    return `${allowed} — 지금은 ${got}`;
  },
  context_not_allowed: (d) => `\`<${s(d.element)}>\`는 이 자리에 올 수 없어요`,
  trailing_page_break: () => '문서의 마지막 문단은 쪽 나눔으로 끝날 수 없어요 — 뒤에 문단을 하나 두세요',
  table_not_rectangular: (d) => `표의 모든 행은 셀 수가 같아야 해요 — 이 행은 ${s(d.expected)}개 중 ${s(d.got)}개예요`,
  block_modifier_not_allowed: (d) => `\`mod:${s(d.modifier)}\`는 \`<${s(d.element)}>\`에 쓸 수 없어요`,
  inline_modifier_not_allowed: (d) => `\`<${s(d.modifier)}>\`는 \`<${s(d.leaf)}>\`를 감쌀 수 없어요`,
  newline_in_text: () => '문단 안에 줄바꿈 문자가 있어요 — 줄을 나누려면 `<hard_break/>`를 쓰세요',
  tab_in_text: () => '문단 안에 탭 문자가 있어요 — `<tab/>`를 쓰세요',
  forbidden_char_in_document: (d) => `문서에 xml이 담을 수 없는 문자(U+${hex(d.codepoint)})가 있어요`,
  dot_invalid: (d) => `\`dot="${s(d.value)}"\`는 올바른 dot이 아니에요`,
  dot_duplicate: (d) => `dot \`${s(d.dot)}\`이 두 번 나와요`,
  dot_not_in_document: (d) => `dot \`${s(d.dot)}\`은 이 문서에 없어요 — 새 블록이면 dot을 지우세요`,
  dot_type_incompatible: (d) =>
    `dot \`${s(d.dot)}\`을 \`<${s(d.new_type)}>\`으로 바꿀 수 없어요(안의 내용이 맞지 않아요) — dot을 지우고 새 블록으로 쓰세요`,
  root_dot_mismatch: () => '`<root>`의 dot이 이 문서와 달라요 — 문서를 다시 여세요',
  opaque_needs_dot: (d) => `\`<${s(d.element)}>\`는 새로 넣을 수 없어요 — 이미 있는 것만 dot으로 가리킬 수 있어요`,
  opaque_has_children: (d) => `\`<${s(d.element)}>\` 안에는 내용을 넣을 수 없어요`,
  opaque_id_changed: (d) => `\`<${s(d.element)}>\`의 \`attr:id\`는 바꿀 수 없어요`,
  base_missing: () => '`base`가 없어요 — 문서를 다시 여세요',
  base_undecodable: () => '`base`를 읽을 수 없어요 — 문서를 다시 여세요',
  base_not_in_history: () => '이 파일은 지금 문서와 이어지지 않아요 — 문서를 다시 여세요',
  projection_degraded: () => '지금은 이 문서를 파일로 고칠 수 없어요 — 작가에게 화면에서 직접 고쳐 달라고 전하세요',
  internal: () => INTERNAL_MESSAGE,
};

const parseDetail = (raw: string): Detail => {
  try {
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === 'object' && parsed !== null ? (parsed as Detail) : {};
  } catch {
    return {};
  }
};

export const messageOf = (info: XmlErrorInfo): string => {
  const detail = parseDetail(info.detail);
  const type = typeof detail.type === 'string' ? detail.type : '';
  const body = XML_MESSAGES[type]?.(detail) ?? INTERNAL_MESSAGE;
  const pos = info.line != null && info.column != null ? `줄 ${info.line} 열 ${info.column}: ` : '';
  const dot = info.dot != null && !body.includes(info.dot) ? ` (dot ${info.dot})` : '';
  return `${pos}${body}${dot}`;
};

export const changedOf = (
  result: XmlEditResult,
): { blocks: { inserted: number; deleted: number; moved: number; updated: number }; chars: { inserted: number; deleted: number } } => ({
  blocks: {
    inserted: result.blocks_inserted,
    deleted: result.blocks_deleted,
    moved: result.blocks_moved,
    updated: result.blocks_updated,
  },
  chars: { inserted: result.chars_inserted, deleted: result.chars_deleted },
});
