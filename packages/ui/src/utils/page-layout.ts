export const MIN_CONTENT_SIZE_MM = 50;

export const INCOMPATIBLE_NODE_TYPES = new Set(['blockquote', 'callout', 'fold', 'table', 'code_block', 'html_block']);

export type PageLayoutSettings = {
  size: 'a4' | 'a5' | 'b5' | 'b6';
  margins: {
    top: number;
    bottom: number;
    left: number;
    right: number;
  };
};

export type PageLayoutSize = PageLayoutSettings['size'];

export const PAGE_SIZE_MAP = {
  a4: { width: 210, height: 297 },
  a5: { width: 148, height: 210 },
  b5: { width: 176, height: 250 },
  b6: { width: 125, height: 176 },
} as const;

export const DEFAULT_PAGE_MARGINS = {
  a4: { top: 25, bottom: 25, left: 25, right: 25 },
  a5: { top: 20, bottom: 20, left: 20, right: 20 },
  b5: { top: 15, bottom: 15, left: 15, right: 15 },
  b6: { top: 10, bottom: 10, left: 10, right: 10 },
} as const;

export const PAGE_LAYOUT_OPTIONS = [
  { label: 'A4 (210mm × 297mm)', value: 'a4' as const },
  { label: 'A5 (148mm × 210mm)', value: 'a5' as const },
  { label: 'B5 (176mm × 250mm)', value: 'b5' as const },
  { label: 'B6 (125mm × 176mm)', value: 'b6' as const },
];

export function getPageLayoutDimensions(settings: PageLayoutSettings) {
  const size = PAGE_SIZE_MAP[settings.size];
  const margins = settings.margins;

  return {
    width: size.width,
    height: size.height,
    marginTop: margins.top,
    marginBottom: margins.bottom,
    marginLeft: margins.left,
    marginRight: margins.right,
  };
}

export function createDefaultPageLayout(size: PageLayoutSize = 'a4'): PageLayoutSettings {
  return {
    size,
    margins: DEFAULT_PAGE_MARGINS[size],
  };
}

export function getMaxMargin(
  side: 'top' | 'bottom' | 'left' | 'right',
  pageSize: PageLayoutSize,
  margins: PageLayoutSettings['margins'],
): number {
  const pageDimensions = PAGE_SIZE_MAP[pageSize];

  if (side === 'left') {
    return pageDimensions.width - margins.right - MIN_CONTENT_SIZE_MM;
  } else if (side === 'right') {
    return pageDimensions.width - margins.left - MIN_CONTENT_SIZE_MM;
  } else if (side === 'top') {
    return pageDimensions.height - margins.bottom - MIN_CONTENT_SIZE_MM;
  } else {
    return pageDimensions.height - margins.top - MIN_CONTENT_SIZE_MM;
  }
}
