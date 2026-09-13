import { defaultValues } from '@typie/lib/const';
import { CONTINUOUS_MIN_WIDTH, CONTINUOUS_VIEW_PADDING } from '$lib/editor-ffi/constants';

export type DocumentViewLayoutMode =
  | { type: 'continuous'; maxWidth: number }
  | {
      type: 'paginated';
      pageWidth: number;
      pageHeight: number;
      pageMarginTop: number;
      pageMarginBottom: number;
      pageMarginLeft: number;
      pageMarginRight: number;
    };

export type DocumentViewColumn = {
  paginated: boolean;
  width: string;
  minWidth: string;
  paddingLeft: string;
  paddingRight: string;
};

const isPositive = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value) && value > 0;
const isNonNegative = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value) && value >= 0;

export const defaultDocumentViewLayoutMode = (): DocumentViewLayoutMode => ({ type: 'continuous', maxWidth: defaultValues.maxWidth });

export const parseDocumentViewLayoutMode = (value: unknown): DocumentViewLayoutMode => {
  if (typeof value !== 'object' || value === null) return defaultDocumentViewLayoutMode();
  const raw = value as Record<string, unknown>;

  if (raw.type === 'continuous' && isPositive(raw.maxWidth)) {
    return { type: 'continuous', maxWidth: raw.maxWidth };
  }

  if (
    raw.type === 'paginated' &&
    isPositive(raw.pageWidth) &&
    isPositive(raw.pageHeight) &&
    isNonNegative(raw.pageMarginTop) &&
    isNonNegative(raw.pageMarginBottom) &&
    isNonNegative(raw.pageMarginLeft) &&
    isNonNegative(raw.pageMarginRight)
  ) {
    return {
      type: 'paginated',
      pageWidth: raw.pageWidth,
      pageHeight: raw.pageHeight,
      pageMarginTop: raw.pageMarginTop,
      pageMarginBottom: raw.pageMarginBottom,
      pageMarginLeft: raw.pageMarginLeft,
      pageMarginRight: raw.pageMarginRight,
    };
  }

  return defaultDocumentViewLayoutMode();
};

export const resolveDocumentViewColumn = (layoutMode: DocumentViewLayoutMode): DocumentViewColumn => {
  if (layoutMode.type === 'paginated') {
    const { pageWidth, pageMarginLeft, pageMarginRight } = layoutMode;
    return {
      paginated: true,
      width: `min(${pageWidth}px, 100%)`,
      minWidth: '0px',
      paddingLeft: `min(${pageMarginLeft}px, calc(100% * ${pageMarginLeft} / ${pageWidth}))`,
      paddingRight: `min(${pageMarginRight}px, calc(100% * ${pageMarginRight} / ${pageWidth}))`,
    };
  }

  return {
    paginated: false,
    width: `min(${layoutMode.maxWidth + CONTINUOUS_VIEW_PADDING * 2}px, 100%)`,
    minWidth: `${CONTINUOUS_MIN_WIDTH}px`,
    paddingLeft: `${CONTINUOUS_VIEW_PADDING}px`,
    paddingRight: `${CONTINUOUS_VIEW_PADDING}px`,
  };
};

export const documentViewColumnStyle = (column: DocumentViewColumn): string =>
  `width:${column.width};min-width:${column.minWidth};padding-left:${column.paddingLeft};padding-right:${column.paddingRight}`;
