import type { Anchor } from './review.ts';

const isValid = (anchor: Anchor, limit: number): boolean => anchor.start >= 0 && anchor.end <= limit && anchor.start < anchor.end;

const isLongAnchor = (content: string, anchor: Anchor): boolean => {
  if (!isValid(anchor, content.length)) return false;
  const text = content.slice(anchor.start, anchor.end).trim();
  const head = anchor.head.trim();
  const tail = anchor.tail.trim();
  return head.length > 0 && tail.length > 0 && text.length > head.length + tail.length;
};

// head·tail이 구간을 이미 다 덮는 짧은 앵커는 원문 그대로 쓴다 — 생략할 중간이 없는데 이어 붙이면 글자가 겹친다.
const clipQuote = (content: string, anchor: Anchor): string =>
  isLongAnchor(content, anchor) ? `${anchor.head.trim()} ⋯ ${anchor.tail.trim()}` : content.slice(anchor.start, anchor.end).trim();

// 여러 앵커도 같은 줄임표로 잇는다 — 한 인용 안에서 "구간 사이 생략"과 "구간 안 생략"을 구분하지 않는다.
export const anchorQuote = (content: string, anchors: Anchor[]): string =>
  anchors
    .filter((anchor) => isValid(anchor, content.length))
    .map((anchor) => clipQuote(content, anchor))
    .filter((text) => text.length > 0)
    .join(' ⋯ ');
