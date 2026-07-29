import type { AnchorDraft, ItemDraft } from '../../../core/contracts.ts';

export type BuildItemsInput = {
  characterization: string;
  feedbacks: { category: string; layer: string; body: string; anchors: AnchorDraft[] }[];
  strengths: { body: string; quoteStart: string; quoteEnd: string; matchStart: number | null; matchEnd: number | null }[];
  cleared: { axis: string; note: string }[];
  patterns: { theme: string; body: string; feedbackIndexes: number[] }[];
  priority: { body: string; feedbackIndexes: number[] }[];
};

// 총평도 지적과 같은 항목이다. 구 구조에서는 지적만 행이고 총평은 JSON이라 섹션에 id가 없었고,
// 그래서 총평에 판정을 걸 수 없었으며 참조가 배열 순번이라 지적 하나가 빠지면 조용히 어긋났다.
export const buildItems = (input: BuildItemsInput): ItemDraft[] => {
  const items: ItemDraft[] = [];

  const push = (draft: Omit<ItemDraft, 'ord'>): number => {
    const ord = items.filter((i) => i.kind === draft.kind).length;
    items.push({ ...draft, ord });
    return items.length - 1;
  };

  if (input.characterization.trim()) {
    push({ kind: 'characterization', body: input.characterization.trim(), facets: {}, anchors: [], links: [] });
  }

  const findingIndexes = input.feedbacks.map((feedback) =>
    push({
      kind: 'finding',
      body: feedback.body,
      facets: { axis: feedback.category, layer: feedback.layer },
      anchors: feedback.anchors,
      links: [],
    }),
  );

  for (const strength of input.strengths) {
    if (!strength.body.trim()) continue;
    push({
      kind: 'strength',
      body: strength.body,
      facets: {},
      anchors: [
        { quoteStart: strength.quoteStart, quoteEnd: strength.quoteEnd, matchStart: strength.matchStart, matchEnd: strength.matchEnd },
      ],
      links: [],
    });
  }

  for (const entry of input.cleared) {
    if (!entry.note.trim()) continue;
    push({ kind: 'cleared', body: entry.note, facets: { axis: entry.axis }, anchors: [], links: [] });
  }

  // 총평이 가리키는 번호는 지적 목록 안의 순번이다. 이걸 items 배열 전체에서의 인덱스로 옮긴다.
  const toLinks = (indexes: number[]): number[] =>
    indexes.filter((i) => Number.isSafeInteger(i) && i >= 0 && i < findingIndexes.length).map((i) => findingIndexes[i]);

  for (const pattern of input.patterns) {
    if (!pattern.body.trim()) continue;
    push({ kind: 'pattern', body: pattern.body, facets: { theme: pattern.theme }, anchors: [], links: toLinks(pattern.feedbackIndexes) });
  }

  for (const entry of input.priority) {
    if (!entry.body.trim()) continue;
    push({ kind: 'priority', body: entry.body, facets: {}, anchors: [], links: toLinks(entry.feedbackIndexes) });
  }

  return items;
};
