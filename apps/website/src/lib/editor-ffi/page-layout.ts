import { values } from '$lib/editor-ffi/values';

export const mmToPx = (mm: number) => Math.round((mm * 96) / 25.4);
export const pxToMm = (px: number) => Math.round((px * 25.4) / 96);

export type PageLayoutPreset = (typeof values.pageLayout)[number]['value'];
export type PageLayout = {
  pageWidth: number;
  pageHeight: number;
  pageMarginTop: number;
  pageMarginBottom: number;
  pageMarginLeft: number;
  pageMarginRight: number;
};

export type DocumentLayoutMode = ({ type: 'paginated' } & PageLayout) | { type: 'continuous'; maxWidth: number };

export type PageMarginSide = 'top' | 'bottom' | 'left' | 'right';

export const createPaginatedLayout = (preset: PageLayoutPreset = 'a4'): PageLayout => {
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  return { ...values.pageLayout.find((p) => p.value === preset)!.layout };
};

const MIN_CONTENT_SIZE_PX = mmToPx(50);
export const MIN_PAGE_SIZE_MM = 100;

const clampNumber = (value: number, min: number, max: number): number => Math.min(max, Math.max(min, value));

const PAGE_MARGIN_KEYS = {
  top: 'pageMarginTop',
  bottom: 'pageMarginBottom',
  left: 'pageMarginLeft',
  right: 'pageMarginRight',
} as const satisfies Record<PageMarginSide, keyof PageLayout>;

const OPPOSITE_PAGE_MARGIN_KEYS = {
  top: 'pageMarginBottom',
  bottom: 'pageMarginTop',
  left: 'pageMarginRight',
  right: 'pageMarginLeft',
} as const satisfies Record<PageMarginSide, keyof PageLayout>;

const PAGE_SIZE_KEYS = {
  width: 'pageWidth',
  height: 'pageHeight',
} as const;

const CONTENT_AXIS_SIZE_KEYS = {
  top: 'pageHeight',
  bottom: 'pageHeight',
  left: 'pageWidth',
  right: 'pageWidth',
} as const satisfies Record<PageMarginSide, keyof PageLayout>;

export const getMaxMargin = (side: PageMarginSide, layout: PageLayout): number => {
  const axisSize = layout[CONTENT_AXIS_SIZE_KEYS[side]];
  const oppositeMargin = layout[OPPOSITE_PAGE_MARGIN_KEYS[side]];
  return Math.max(0, axisSize - oppositeMargin - MIN_CONTENT_SIZE_PX);
};

export const getPageMargin = (side: PageMarginSide, layout: PageLayout): number => layout[PAGE_MARGIN_KEYS[side]];

export type PageUnit = 'width' | 'height' | PageMarginSide;

// width/height: 페이지 크기를 바꾼 뒤 줄어든 축에 맞춰 인접 여백을 다시 clamp
// margin: 0 ~ getMaxMargin 범위로 clamp
export const resizePageUnit = (layout: PageLayout, unit: PageUnit, valueMm: number): PageLayout => {
  if (unit === 'width' || unit === 'height') {
    const nextLayout = { ...layout, [PAGE_SIZE_KEYS[unit]]: mmToPx(Math.max(MIN_PAGE_SIZE_MM, valueMm)) };
    const sides = unit === 'width' ? (['left', 'right'] as const) : (['top', 'bottom'] as const);
    return {
      ...nextLayout,
      [PAGE_MARGIN_KEYS[sides[0]]]: Math.min(nextLayout[PAGE_MARGIN_KEYS[sides[0]]], getMaxMargin(sides[0], nextLayout)),
      [PAGE_MARGIN_KEYS[sides[1]]]: Math.min(nextLayout[PAGE_MARGIN_KEYS[sides[1]]], getMaxMargin(sides[1], nextLayout)),
    };
  }

  const marginPx = clampNumber(mmToPx(valueMm), 0, getMaxMargin(unit, layout));
  return { ...layout, [PAGE_MARGIN_KEYS[unit]]: marginPx };
};
