import type { Anchor } from './review.ts';

export type ReanchorRange = { start: number; end: number };

type Match = { index: number; length: number };
type Find = (needle: string, from: number) => Match | null;
type Walk = (needle: string, origin: number) => Match | null;
type Tier = { find: Find; walk: Walk };

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

// 동점이면 뒤쪽을 취한다 — 이 규칙의 유일한 정의다.
const nearer = (behind: Match | null, ahead: Match | null, origin: number): Match | null => {
  if (!behind) return ahead;
  if (!ahead) return behind;
  return origin - behind.index <= ahead.index - origin ? behind : ahead;
};

const nearestOutward = (forward: (p: number) => Match | null, backward: (p: number) => Match | null, origin: number): Match | null => {
  const from = Math.max(0, origin);
  return nearer(from > 0 ? backward(from - 1) : null, forward(from), origin);
};

const buildTiers = (text: string): Tier[] => {
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

  const exactWalk: Walk = (needle, origin) => {
    if (!needle.trim()) return null;
    const backward = (p: number) => {
      const idx = text.lastIndexOf(needle, p);
      return idx === -1 ? null : { index: idx, length: needle.length };
    };
    return nearestOutward((p) => exactFind(needle, p), backward, origin);
  };

  const fuzzyPattern = (needle: string) => {
    const trimmed = needle.trim();
    return trimmed ? escape(trimmed).replaceAll(/\s+/g, String.raw`\s+`) : null;
  };

  const fuzzyFind: Find = (needle, from) => {
    const pattern = fuzzyPattern(needle);
    if (!pattern) return null;
    const at = Math.max(0, from);
    const match = new RegExp(pattern).exec(text.slice(at));
    return match ? { index: at + match.index, length: match[0].length } : null;
  };

  // 정규식은 뒤로 훑지 못한다 — 전역 순회로 origin 직전 일치를 계속 덮어써 두고, origin을 넘는 첫 일치에서 끊는다.
  const fuzzyWalk: Walk = (needle, origin) => {
    const pattern = fuzzyPattern(needle);
    if (!pattern) return null;
    const from = Math.max(0, origin);
    let behind: Match | null = null;
    for (const match of text.matchAll(new RegExp(pattern, 'g'))) {
      const found = { index: match.index, length: match[0].length };
      if (found.index >= from) return nearer(behind, found, origin);
      behind = found;
    }
    return behind;
  };

  const spanAt = (idx: number, length: number): Match => {
    const first = map[idx];
    const last = map[idx + length - 1];
    return { index: first, length: last + 1 - first };
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
    return idx === -1 ? null : spanAt(idx, n.length);
  };

  const looseWalk: Walk = (needle, origin) => {
    const n = normalizeForMatch(needle.trim());
    if (!n) return null;
    const backward = (p: number) => {
      const hi = keptFrom(p + 1) - 1;
      if (hi < 0) return null;
      const idx = normalized.lastIndexOf(n, hi);
      return idx === -1 ? null : spanAt(idx, n.length);
    };
    return nearestOutward((p) => looseFind(needle, p), backward, origin);
  };

  return [
    { find: exactFind, walk: exactWalk },
    { find: fuzzyFind, walk: fuzzyWalk },
    { find: looseFind, walk: looseWalk },
  ];
};

const closeRange = (find: Find, head: Match, tail: string): ReanchorRange | null => {
  // 꼬리가 머리 인용 범위 안의 문장이어도 유효한 앵커다 — 겹침을 허용한다.
  const after = find(tail, head.index);
  if (after) return { start: head.index, end: Math.max(head.index + head.length, after.index + after.length) };
  // 모델이 머리와 꼬리를 원문 순서와 반대로 준 경우 — 앞쪽에서 다시 찾아 둘을 잇는다.
  const earlier = find(tail, 0);
  return earlier && earlier.index < head.index ? { start: earlier.index, end: head.index + head.length } : null;
};

const reanchorOne = (tiers: readonly Tier[], anchor: Anchor): ReanchorRange | null => {
  // 인용이 없으면 찾을 근거가 없다 — 리뷰 시점 좌표는 그 뒤의 편집만큼 어긋나 있으므로 쓰지 않는다.
  // 머리만·꼬리만 빈 경우와 같은 판정이다.
  if (!anchor.head.trim() && !anchor.tail.trim()) return null;

  const attempt = (head: string, tail: string): ReanchorRange | null => {
    for (const tier of tiers) {
      // 티어마다 최근접 후보 하나만 본다. 이 후보가 닫히지 않았다면 이 티어의 파인더가 꼬리를 원고 어디에서도
      // 찾지 못한다는 뜻이다 — 파인더는 언제나 "from 이후 첫 일치"라 머리 뒤에 꼬리가 없으면 모든 꼬리가 머리
      // 앞에 있고, 그때는 역순 경로가 반드시 닫는다. 닫힘 여부가 머리 위치와 무관하므로 더 먼 후보를 넣어도
      // 결과가 같다.
      const nearest = tier.walk(head, anchor.start);
      if (!nearest) continue;
      const range = closeRange(tier.find, nearest, tail);
      if (range) return range;
    }
    return null;
  };

  const head = repairQuote(anchor.head);
  const tail = repairQuote(anchor.tail);
  return attempt(anchor.head, anchor.tail) ?? (head === anchor.head && tail === anchor.tail ? null : attempt(head, tail));
};

export const reanchorAll = (text: string, anchors: readonly Anchor[]): (ReanchorRange | null)[] => {
  const tiers = buildTiers(text);
  return anchors.map((anchor) => reanchorOne(tiers, anchor));
};
