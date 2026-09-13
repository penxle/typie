const LEAD_LETTER_GROUPS = 'ㄱㄱㄴㄷㄷㄹㅁㅂㅂㅅㅅㅇㅈㅈㅊㅋㅌㅍㅎ';
const TAIL_LETTER_GROUPS = 'ㄱㄱㄱㄴㄴㄴㄷㄹㄹㄹㄹㄹㄹㄹㄹㅁㅂㅂㅅㅅㅇㅈㅊㅋㅌㅍㅎ';
const CLUSTER_LEAD_GROUPS: ReadonlyMap<number, TagInitial> = new Map([
  [0x11_1a, 'ㄹ'],
  [0x11_21, 'ㅂ'],
]);

export const TAG_INITIAL_ORDER = [
  'ㄱ',
  'ㄴ',
  'ㄷ',
  'ㄹ',
  'ㅁ',
  'ㅂ',
  'ㅅ',
  'ㅇ',
  'ㅈ',
  'ㅊ',
  'ㅋ',
  'ㅌ',
  'ㅍ',
  'ㅎ',
  'A-Z',
  '기타',
] as const;

export type TagInitial = (typeof TAG_INITIAL_ORDER)[number];

const LEAD_LETTER_START = 0x11_00;
const TAIL_LETTER_START = 0x11_a8;

const groupOf = (groups: string, offset: number) => (offset >= 0 && offset < groups.length ? (groups[offset] as TagInitial) : null);

export const tagInitial = (name: string): TagInitial => {
  const first = name.normalize('NFKD').codePointAt(0);
  if (first === undefined) return '기타';

  const hangul =
    groupOf(LEAD_LETTER_GROUPS, first - LEAD_LETTER_START) ??
    groupOf(TAIL_LETTER_GROUPS, first - TAIL_LETTER_START) ??
    CLUSTER_LEAD_GROUPS.get(first);
  if (hangul) return hangul;

  return /[a-z]/i.test(String.fromCodePoint(first)) ? 'A-Z' : '기타';
};

export const groupTagsByInitial = <T extends { name: string }>(tags: readonly T[]): { initial: TagInitial; tags: T[] }[] => {
  const byInitial = Map.groupBy(tags, (tag) => tagInitial(tag.name));
  return TAG_INITIAL_ORDER.flatMap((initial) => {
    const group = byInitial.get(initial);
    return group ? [{ initial, tags: group.toSorted((a, b) => a.name.localeCompare(b.name, 'ko')) }] : [];
  });
};
