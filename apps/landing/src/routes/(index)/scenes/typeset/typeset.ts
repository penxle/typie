import { clamp } from '@typie/ui/utils';

export type TypesetState = {
  key: string;
  font: 'pretendard' | 'batang';
  maxWidth: number;
  indent: number;
  gap: number;
  paged: boolean;
};

// 실제 본문 설정의 페이지 프리셋 A4 (210mm × 297mm) — apps/website/src/lib/editor-ffi/values.ts
// zoom은 실제 앱의 displayZoom과 같은 자리 — A4 실제 크기로 조판하고 화면에 맞춰 줄인다
export const PAGE = { width: 794, height: 1123, margin: 94, zoom: 0.47 } as const;
export const PAGE_SLOT = { width: Math.round(PAGE.width * PAGE.zoom), height: Math.round(PAGE.height * PAGE.zoom) } as const;

// 설정 라벨 한 줄(12px × 1.4)과 아래 여백 14px, 그림자가 번지는 몫
export const SWEEP_RANGE = [0.12, 0.74] as const;

export const LABEL_HEIGHT = 48;
export const PAGE_TEXT_SIZE = 16;

export const zoomFor = (width: number, height: number): number =>
  width > 0 && height > 0 ? Math.min(PAGE.zoom, width / PAGE.width, Math.max(0, height - LABEL_HEIGHT) / PAGE.height) : PAGE.zoom;

export const TYPESET_SEQUENCE: readonly TypesetState[] = [
  { key: 'base', font: 'pretendard', maxWidth: 600, indent: 1, gap: 1, paged: false },
  { key: 'spacing', font: 'pretendard', maxWidth: 600, indent: 2, gap: 0, paged: false },
  { key: 'font', font: 'batang', maxWidth: 600, indent: 1, gap: 1, paged: false },
  { key: 'width', font: 'batang', maxWidth: 400, indent: 1, gap: 1, paged: false },
  { key: 'page', font: 'pretendard', maxWidth: PAGE_SLOT.width, indent: 1, gap: 1, paged: true },
];

export const pagedSequence = (slotWidth: number): readonly TypesetState[] =>
  TYPESET_SEQUENCE.map((state) => (state.paged ? { ...state, maxWidth: slotWidth } : state));

export const settingLineOf = (state: TypesetState): string =>
  [
    state.font === 'batang' ? '리디바탕' : '프리텐다드',
    state.paged ? '페이지' : `${state.maxWidth}px`,
    `들여쓰기 ${state.indent === 0 ? '없음' : `${state.indent}칸`}`,
    `문단 간격 ${state.gap === 0 ? '없음' : `${state.gap}줄`}`,
  ].join(' · ');

type Sweep = { prev: TypesetState; next: TypesetState; x: number };

export const sweepAt = (t: number, sequence: readonly TypesetState[] = TYPESET_SEQUENCE): Sweep => {
  const passes = sequence.length - 1;
  const scaled = clamp(t, 0, 1) * passes;
  const index = Math.min(passes - 1, Math.floor(scaled));
  return { next: sequence[index + 1], prev: sequence[index], x: Math.min(1, scaled - index) };
};
