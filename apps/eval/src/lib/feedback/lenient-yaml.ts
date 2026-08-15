// prism의 관용 전처리 이식 — 원본: prism src/yaml.ts `lenient`(오너 결정 2026-08-13). 에이전트가 쓴 산출물은 prism이 이 전처리를
// 거쳐 검증·저장하므로, 파일에는 표준 YAML이 깨지는 plain 값이 그대로 남아 있을 수 있다(값 선두의 따옴표, 값 속 ': '·' #').
// eval이 같은 전처리 없이 표준 파서로 읽으면 prism이 접수한 파일을 형식 불일치로 세운다 — 그래서 같은 규칙으로 읽는다.
//
// 규칙(원본과 동형): 스키마가 string으로 선언한 자리의 plain 꼬리를 원문 그대로의 큰따옴표 스칼라(JSON 문자열)로 감싼다.
// 블록 헤더(|·>)는 그 스팬을 건너뛰고, 꼬리 전체가 온전한 단일 인용 스칼라면 YAML 인용으로 존중하며, 그 밖의 꼬리는 주석(#)까지
// 전부 값이다. array·number·boolean 자리와 스키마 밖 키는 표준 규칙 그대로. plain 연속 줄(더 깊은 들여쓰기)은 YAML 폴딩과 같게
// 공백 결합으로 흡수하고 원 줄은 빈 줄로 두어 행 수를 보존한다. 구조를 못 알아본 줄은 손대지 않는다 — 실패의 방향은 언제나
// "현행 그대로(표준 파서의 판정)"다.

import { isScalar, parseDocument, Scalar } from 'yaml';

export type ShapeSchema = {
  type?: string;
  properties?: Record<string, ShapeSchema>;
  items?: ShapeSchema;
};

const BLOCK_HEADER = /^[|>][0-9+-]{0,2}[ \t]*(?:#.*)?$/;
const KEY_LINE = /^(?<key>[A-Za-z_][\w-]*):(?<rest>.*)$/;

// 꼬리 전체가 온전한 단일 인용 스칼라인가 — 이때만 YAML 인용 의미론(이스케이프 해석)을 존중한다.
// 인용 뒤에 공백 외 무엇이라도(주석 포함) 남으면 온전하지 않다 — 꼬리 전체가 값이 된다.
const wholeQuoted = (tail: string): boolean => {
  if (tail[0] !== '"' && tail[0] !== "'") return false;
  const doc = parseDocument(tail);
  const node = doc.contents;
  if (doc.errors.length > 0 || !isScalar(node)) return false;
  if (node.type !== Scalar.QUOTE_DOUBLE && node.type !== Scalar.QUOTE_SINGLE) return false;
  return node.range !== null && tail.slice(node.range[1]).trim() === '';
};

// entry는 이 스코프의 키(또는 대시)가 서는 열 — 첫 줄에서 고정되고, 안 맞는 줄은 표준 파서의 몫.
type LenientScope = { kind: 'object' | 'array'; entry: number | null; min: number; schema: ShapeSchema };

// 감싸기로 결정된 꼬리. plain 연속 줄(over보다 깊은 줄)은 공백 결합으로 흡수하고 원 줄은 빈 줄로 둔다. 빈 줄이 나오면 흡수를 끝낸다.
type LenientPending = { line: number; prefix: string; parts: string[]; over: number };

type LenientSettled = { block: { over: number } } | { pending: LenientPending } | null;

export const lenient = (source: string, root: ShapeSchema): string => {
  if (!root.properties) return source;
  const lines = source.split('\n');
  const out = [...lines];
  const rootScope: LenientScope = { kind: 'object', entry: 0, min: 0, schema: root };
  const scopes: LenientScope[] = [rootScope];
  // 스코프 스택은 비지 않는다(루트가 남는다) — 폴백은 타입을 위한 것.
  const topOf = (): LenientScope => scopes.at(-1) ?? rootScope;
  let block: { over: number } | null = null;
  let pending: LenientPending | null = null;

  const flush = (p: LenientPending): void => {
    out[p.line] = p.prefix + JSON.stringify(p.parts.join(' '));
  };

  const settle = (i: number, tailCol: number, tail: string, schema: ShapeSchema, over: number): LenientSettled => {
    if (BLOCK_HEADER.test(tail)) return { block: { over } };
    if (schema.type !== 'string' || wholeQuoted(tail)) return null;
    return { pending: { line: i, prefix: lines[i].slice(0, tailCol), parts: [tail], over } };
  };

  const handleKey = (i: number, col: number, content: string): LenientSettled => {
    const top = topOf();
    const m = KEY_LINE.exec(content);
    if (!m?.groups) return null;
    if (top.entry === null) top.entry = col;
    else if (col !== top.entry) return null;
    const child = top.schema.properties?.[m.groups.key];
    if (child === undefined) return null;
    const rest = m.groups.rest;
    if (rest.trim() === '') {
      if (child.type === 'object' || child.type === 'array') {
        scopes.push({ kind: child.type, entry: null, min: col + 1, schema: child });
      }
      return null;
    }
    if (!rest.startsWith(' ')) return null;
    const tail = rest.trimStart();
    return settle(i, col + m.groups.key.length + 1 + (rest.length - tail.length), tail.trimEnd(), child, col);
  };

  for (const [i, line] of lines.entries()) {
    const indent = line.search(/\S/);
    if (block !== null) {
      if (indent === -1 || indent > block.over) continue;
      block = null;
    }
    if (pending !== null) {
      if (indent > pending.over) {
        pending.parts.push(line.trim());
        out[i] = '';
        continue;
      }
      flush(pending);
      pending = null;
    }
    if (indent === -1) continue;
    while (scopes.length > 1 && indent < (topOf().entry ?? topOf().min)) scopes.pop();
    const top = topOf();
    const text = line.slice(indent).trimEnd();
    let settled: LenientSettled = null;
    if (top.kind === 'array' && (text === '-' || text.startsWith('- '))) {
      if (top.entry === null) top.entry = indent;
      if (indent === top.entry) {
        const after = text.slice(1);
        const content = after.trimStart();
        if (content !== '') {
          const contentCol = indent + 1 + (after.length - content.length);
          const items = top.schema.items ?? {};
          if (items.type === 'object') {
            scopes.push({ kind: 'object', entry: contentCol, min: contentCol, schema: items });
            settled = handleKey(i, contentCol, content);
          } else {
            settled = settle(i, contentCol, content, items, indent);
          }
        }
      }
    } else if (top.kind === 'object') {
      settled = handleKey(i, indent, text);
    }
    if (settled !== null) {
      if ('block' in settled) block = settled.block;
      else pending = settled.pending;
    }
  }
  if (pending !== null) flush(pending);
  return out.join('\n');
};
