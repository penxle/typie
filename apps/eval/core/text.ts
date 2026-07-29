export type Match = { index: number; length: number };

const escape = (s: string) => s.replaceAll(/[|\\{}()[\]^$+*?.]/g, String.raw`\$&`).replaceAll('-', String.raw`\x2d`);

export const fuzzyFindMatch = (haystack: string, needle: string, fromIndex: number): Match | null => {
  const trimmed = needle.trim();
  if (!trimmed) return null;
  const pattern = escape(trimmed).replaceAll(/\s+/g, String.raw`\s+`);
  const subStart = Math.max(0, fromIndex);
  const match = new RegExp(pattern).exec(haystack.slice(subStart));
  if (!match) return null;
  return { index: subStart + match.index, length: match[0].length };
};

// 모델이 자주 일으키는 인용 변형(따옴표 날조·스타일 변경, 공백 소실)을 흡수하는 최후 폴백용 정규화.
const isMatchIgnored = (ch: string) => ch === '"' || ch === '“' || ch === '”' || ch === "'" || ch === '‘' || ch === '’' || /\s/.test(ch);

const buildNormalizedIndex = (text: string) => {
  const kept: string[] = [];
  const map: number[] = [];
  let i = 0;
  while (i < text.length) {
    if (!isMatchIgnored(text[i])) {
      kept.push(text[i]);
      map.push(i);
    }
    i++;
  }
  return { normalized: kept.join(''), map };
};

const normalizeForMatch = (s: string) => {
  let out = '';
  for (const ch of s) {
    if (!isMatchIgnored(ch)) out += ch;
  }
  return out;
};

// 모델이 인용을 옮기다 남기는 잡음을 걷어낸다. 실측된 두 가지를 겨냥한다:
// ① 개행 직후 첫 한글 음절이 음이 같은 가나로 바뀌어 나온다(에→エ, 하→は, 무→む).
//    개행이 잦은 원고에서 집중적으로 터진다 — 원문에 가나가 한 글자도 없어도 발생한다.
// ② 인용 앞뒤에 원문에 없는 문장부호(…, — 등)가 덧붙는다.
// 정상 경로가 전부 실패한 뒤에만 쓰므로, 원문에 실제로 가나가 있는 인용은 이미 매칭돼 여기 오지 않는다.
const KANA = /[\u{3040}-\u{30FF}]/gu;
const EDGE_NOISE = /^[\s…—·.,'"‘’“”]+|[\s…—·,'"‘’“”]+$/gu;

export const repairQuote = (s: string): string => s.replaceAll(KANA, '').replaceAll(EDGE_NOISE, '');

export const createFindRange = (text: string) => {
  const { normalized, map } = buildNormalizedIndex(text);

  return (startText: string, endText: string, searchStart: number) => {
    const exactFind = (needle: string, from: number): Match | null => {
      const idx = text.indexOf(needle, from);
      return idx === -1 ? null : { index: idx, length: needle.length };
    };

    const normalizedFind = (needle: string, from: number): Match | null => {
      const n = normalizeForMatch(needle.trim());
      if (!n) return null;
      let lo = 0;
      while (lo < map.length && map[lo] < from) lo++;
      const idx = normalized.indexOf(n, lo);
      if (idx === -1) return null;
      const first = map[idx];
      const last = map[idx + n.length - 1];
      return { index: first, length: last + 1 - first };
    };

    const tryFinders = (find: (needle: string, from: number) => Match | null, head: string, tail: string) => {
      const start = find(head, searchStart);
      if (!start) return null;
      // end가 start 인용 범위 안의 문장이어도 유효한 앵커다 — 겹침을 허용한다.
      const end = find(tail, start.index);
      if (end) {
        return { rangeStart: start.index, rangeEnd: Math.max(start.index + start.length, end.index + end.length) };
      }
      // 모델이 두 인용을 원문 순서와 반대로 준 경우 — 앞쪽에서 다시 찾아 둘을 잇는다.
      const earlier = find(tail, searchStart);
      if (!earlier || earlier.index >= start.index) return null;
      return { rangeStart: earlier.index, rangeEnd: start.index + start.length };
    };

    const finders = [exactFind, (n: string, from: number) => fuzzyFindMatch(text, n, from), normalizedFind];
    const attempt = (head: string, tail: string) => {
      for (const find of finders) {
        const range = tryFinders(find, head, tail);
        if (range) return range;
      }
      return null;
    };

    const repairedStart = repairQuote(startText);
    const repairedEnd = repairQuote(endText);
    const range =
      attempt(startText, endText) ?? (repairedStart === startText && repairedEnd === endText ? null : attempt(repairedStart, repairedEnd));
    if (!range) {
      return null;
    }
    return range;
  };
};
