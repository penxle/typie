import type { StableSelection } from '@typie/editor-ffi/server';

export type ResolvedAnchor = { head: string; tail: string; selection: StableSelection | null; text: string | null };

// 회차의 총평 앵커 — result.conclusion.strengths[i].anchors·result.elevations[i].anchors와 평행 배열
export type ConclusionAnchors = { strengths: ResolvedAnchor[][]; elevations: ResolvedAnchor[][] };

// head·tail이 구간을 이미 다 덮는 짧은 앵커는 원문 그대로 쓴다 — 생략할 중간이 없는데 이어 붙이면 글자가 겹친다.
// 원문(text)이 없는 앵커는 자리를 못 찾은 것이다 — 인용은 앵커가 살아 있든 잃었든 보여야 하므로 머리·꼬리로 선다.
const clipQuote = (anchor: ResolvedAnchor): string => {
  const head = anchor.head.trim();
  const tail = anchor.tail.trim();
  const text = anchor.text?.trim() ?? '';
  if (text.length === 0) return head === tail ? head : [head, tail].filter((part) => part.length > 0).join(' ⋯ ');
  return head.length > 0 && tail.length > 0 && text.length > head.length + tail.length ? `${head} ⋯ ${tail}` : text;
};

// 여러 앵커도 같은 줄임표로 잇는다 — 한 인용 안에서 "구간 사이 생략"과 "구간 안 생략"을 구분하지 않는다.
export const anchorQuote = (anchors: readonly ResolvedAnchor[]): string =>
  anchors
    .map((anchor) => clipQuote(anchor))
    .filter((quote) => quote.length > 0)
    .join(' ⋯ ');
