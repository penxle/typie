import type { Anchor } from './review.ts';

// 원고 텍스트 좌표(UTF-16 코드 유닛) — 이 모듈이 원고 포맷에 종속된 유일한 텍스트층이다
export type ResolvedRange = { start: number; end: number };

type Match = { index: number; length: number };
type Find = (needle: string, from: number) => Match | null;

const escape = (s: string) => s.replaceAll(/[|\\{}()[\]^$+*?.]/g, String.raw`\$&`).replaceAll('-', String.raw`\x2d`);

const isMatchIgnored = (ch: string) => ch === '"' || ch === '“' || ch === '”' || ch === "'" || ch === '‘' || ch === '’' || /\s/.test(ch);

const KANA = /[\u{3040}-\u{30FF}]/gu;
const EDGE_NOISE = /^[\s…—·.,'"‘’“”]+|[\s…—·,'"‘’“”]+$/gu;

// 모델이 인용을 옮기며 남기는 잡음: 개행 직후 음절이 가나로 바뀌거나, 앞뒤에 없던 문장부호가 붙는다.
const repairQuote = (s: string) => s.replaceAll(KANA, '').replaceAll(EDGE_NOISE, '');

const normalizeForMatch = (s: string) => {
  let out = '';
  for (const ch of s) {
    if (!isMatchIgnored(ch)) out += ch;
  }
  return out;
};

const buildFinders = (text: string): Find[] => {
  const kept: string[] = [];
  const map: number[] = [];
  // normalized의 인덱스가 kept의 인덱스와 1:1이어야 하므로 코드 포인트가 아니라 코드 유닛 단위로 훑는다.
  let index = 0;
  while (index < text.length) {
    const ch = text[index];
    if (!isMatchIgnored(ch)) {
      kept.push(ch);
      map.push(index);
    }
    index++;
  }
  const normalized = kept.join('');

  const exactFind: Find = (needle, from) => {
    // 빈 인용은 indexOf가 from에서 성공시켜 길이 0짜리 앵커를 만든다 — 세 티어 같은 자리에서 막는다.
    if (!needle.trim()) return null;
    const idx = text.indexOf(needle, from);
    return idx === -1 ? null : { index: idx, length: needle.length };
  };

  const fuzzyFind: Find = (needle, from) => {
    const trimmed = needle.trim();
    if (!trimmed) return null;
    const pattern = escape(trimmed).replaceAll(/\s+/g, String.raw`\s+`);
    const at = Math.max(0, from);
    const match = new RegExp(pattern).exec(text.slice(at));
    return match ? { index: at + match.index, length: match[0].length } : null;
  };

  const keptFrom = (p: number) => {
    let lo = 0;
    let hi = map.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (map[mid] < p) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  };

  const looseFind: Find = (needle, from) => {
    const n = normalizeForMatch(needle.trim());
    if (!n) return null;
    const idx = normalized.indexOf(n, keptFrom(from));
    if (idx === -1) return null;
    const first = map[idx];
    const last = map[idx + n.length - 1];
    return { index: first, length: last + 1 - first };
  };

  return [exactFind, fuzzyFind, looseFind];
};

const closeRange = (find: Find, head: Match, tail: string): ResolvedRange | null => {
  // 꼬리가 머리 인용 범위 안의 문장이어도 유효한 앵커다 — 겹침을 허용한다.
  const after = find(tail, head.index);
  if (after) return { start: head.index, end: Math.max(head.index + head.length, after.index + after.length) };
  // 모델이 머리와 꼬리를 원문 순서와 반대로 준 경우 — 앞쪽에서 다시 찾아 둘을 잇는다.
  const earlier = find(tail, 0);
  return earlier && earlier.index < head.index ? { start: earlier.index, end: head.index + head.length } : null;
};

const resolveOne = (finders: readonly Find[], anchor: Anchor): ResolvedRange | null => {
  if (!anchor.head.trim() && !anchor.tail.trim()) return null;

  const attempt = (head: string, tail: string): ResolvedRange | null => {
    for (const find of finders) {
      const first = find(head, 0);
      if (!first) continue;
      const range = closeRange(find, first, tail);
      if (range) return range;
    }
    return null;
  };

  const head = repairQuote(anchor.head);
  const tail = repairQuote(anchor.tail);
  return attempt(anchor.head, anchor.tail) ?? (head === anchor.head && tail === anchor.tail ? null : attempt(head, tail));
};

export const resolveAnchors = (text: string, anchors: readonly Anchor[]): (ResolvedRange | null)[] => {
  const finders = buildFinders(text);
  return anchors.map((anchor) => resolveOne(finders, anchor));
};
