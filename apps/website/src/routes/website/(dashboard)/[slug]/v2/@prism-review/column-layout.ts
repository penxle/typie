import { CURSOR_VISIBLE_MARGIN } from '$lib/editor-ffi/constants';

export type CardEntry = { id: string; desired: number; height: number };

export const CARD_GAP = 12;
export const CARD_BOTTOM_GAP = CURSOR_VISIBLE_MARGIN;

export const layoutBottomWithin = (element: HTMLElement, ancestor: HTMLElement): number | null => {
  let bottom = element.offsetHeight;
  let current: HTMLElement | null = element;
  while (current !== ancestor) {
    bottom += current.offsetTop;
    current = current.offsetParent as HTMLElement | null;
    if (current === null) return null;
  }
  return bottom;
};

export const layoutCards = (entries: readonly CardEntry[], activeId: string | null): { tops: Record<string, number>; spacer: number } => {
  const sorted = entries.toSorted((a, b) => a.desired - b.desired);
  const tops: Record<string, number> = {};

  const layoutDown = (from: number, start: number) => {
    let cursor = start;
    for (let i = from; i < sorted.length; i++) {
      const entry = sorted[i];
      const top = Math.max(Number.isFinite(entry.desired) ? entry.desired : 0, cursor);
      tops[entry.id] = top;
      cursor = top + entry.height + CARD_GAP;
    }
  };

  const activeAt = sorted.findIndex((entry) => entry.id === activeId);

  // 상향 양보는 앵커 높이가 잡힌 활성 카드만의 규칙이다 — 앵커 없는 카드를 기준으로 위를 물리면
  // 기준점이 컬럼 상단이 되어 위쪽 전부가 top 0에 포개진다.
  if (activeAt === -1 || !Number.isFinite(sorted[activeAt].desired)) {
    layoutDown(0, 0);
  } else {
    const active = sorted[activeAt];
    tops[active.id] = active.desired;
    let ceiling = active.desired;
    for (let i = activeAt - 1; i >= 0; i--) {
      const entry = sorted[i];
      tops[entry.id] = Math.max(0, Math.min(entry.desired, ceiling - CARD_GAP - entry.height));
      ceiling = tops[entry.id];
    }
    layoutDown(activeAt + 1, active.desired + active.height + CARD_GAP);
  }

  const bottoms = sorted.map((entry) => tops[entry.id] + entry.height);
  return { tops, spacer: (bottoms.length > 0 ? Math.max(...bottoms) : 0) + CARD_BOTTOM_GAP };
};
