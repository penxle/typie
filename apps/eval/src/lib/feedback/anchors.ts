import { computeSegments } from './highlight.ts';
import type { Anchor } from './types.ts';

export type MarkKind = 'critique' | 'proofread' | 'closed' | 'strength';

// 마크 파생에 필요한 최소 형태 — load가 내려주는 스레드 행이 그대로 들어맞는다. 잘 작동하는 대목(strength)도
// 같은 파이프라인을 탄다 — id만 'strength.N' 네임스페이스로 갈라지고 나머지 형상·동작은 지적과 동일하다.
export type MarkThread = { id: string; pass: 'critique' | 'proofread' | 'strength'; state: string; anchors: Anchor[] };

// 본문 표기는 레일로 통일됐다(오너 결정) — 세그먼트는 앵커 길이를 가리지 않고 구간 소속만 싣는다.
// 스팬은 레일 측정 좌표와 활성 하이라이트만 담당하고, 평시 본문은 무표기다.
export type MarkSegment = { text: string; threadIds: string[] };

export const kindOf = (thread: MarkThread): MarkKind => (thread.state === 'closed' ? 'closed' : thread.pass);

// 긴 앵커 판정 — 카드의 인용 접기(clipQuote)가 쓴다: head·tail이 구간을 다 덮지 못하면 접는다.
export const isLongAnchor = (content: string, anchor: Anchor): boolean => {
  if (!isValid(anchor, content.length)) return false;
  const text = content.slice(anchor.start, anchor.end).trim();
  const head = anchor.head.trim();
  const tail = anchor.tail.trim();
  return head.length > 0 && tail.length > 0 && text.length > head.length + tail.length;
};

export const markSegments = (content: string, threads: MarkThread[]): MarkSegment[] => {
  const known = new Set(threads.map((thread) => thread.id));
  const anchors = threads.flatMap((thread) =>
    thread.anchors.map((anchor) => ({ start: anchor.start, end: anchor.end, feedbackId: thread.id })),
  );

  return computeSegments(content, anchors).map((segment) => ({
    text: segment.text,
    // 한 스레드가 겹치는 앵커 여러 개로 같은 구간을 덮으면 id가 중복으로 온다.
    threadIds: [...new Set(segment.feedbackIds).intersection(known)],
  }));
};

const trimEdges = (pieces: MarkSegment[]): MarkSegment[] => {
  const trimmed = pieces.map((piece, index) => {
    let { text } = piece;
    if (index === 0) text = text.trimStart();
    if (index === pieces.length - 1) text = text.trimEnd();
    return { ...piece, text };
  });
  return trimmed.filter((piece) => piece.text.length > 0);
};

// 앵커는 원문 전체 기준 오프셋이라 문단 분할을 먼저 하면 좌표가 어긋난다 — 세그먼트를 먼저 만들고 개행에서 가른다.
export const markParagraphs = (content: string, threads: MarkThread[]): MarkSegment[][] => {
  const paragraphs: MarkSegment[][] = [];
  let current: MarkSegment[] = [];

  for (const segment of markSegments(content, threads)) {
    const lines = segment.text.split('\n');
    for (const [index, line] of lines.entries()) {
      if (index > 0) {
        paragraphs.push(current);
        current = [];
      }
      if (line.length > 0) current.push({ ...segment, text: line });
    }
  }
  paragraphs.push(current);

  return paragraphs.map((pieces) => trimEdges(pieces)).filter((pieces) => pieces.length > 0);
};

// 인용이 볼 앵커의 판정 — limit은 원문 길이다.
const isValid = (anchor: Anchor, limit: number): boolean => anchor.start >= 0 && anchor.end <= limit && anchor.start < anchor.end;

// 인용 블록 — 긴 구간은 전문 대신 머리·꼬리만 잇는다(수천 자 앵커가 카드를 삼키지 않게). head·tail이
// 구간을 이미 다 덮는 짧은 앵커는 원문 그대로 쓴다 — 생략할 중간이 없는데 이어 붙이면 글자가 겹친다.
const clipQuote = (content: string, anchor: Anchor): string =>
  isLongAnchor(content, anchor) ? `${anchor.head.trim()} ⋯ ${anchor.tail.trim()}` : content.slice(anchor.start, anchor.end).trim();

// 여러 앵커도 같은 줄임표로 잇는다 — 한 인용 안에서 "구간 사이 생략"과 "구간 안 생략"을 구분하지 않는다.
export const anchorQuote = (content: string, anchors: Anchor[]): string =>
  anchors
    .filter((anchor) => isValid(anchor, content.length))
    .map((anchor) => clipQuote(content, anchor))
    .filter((text) => text.length > 0)
    .join(' ⋯ ');
