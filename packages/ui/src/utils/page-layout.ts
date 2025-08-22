export const MIN_CONTENT_SIZE_MM = 50;

export const INCOMPATIBLE_NODE_TYPES = new Set(['blockquote', 'callout', 'fold', 'table', 'code_block', 'html_block']);

export type PageLayoutPreset = 'a4' | 'a5' | 'b5' | 'b6';

export type PageLayout = {
  width: number;
  height: number;
  marginTop: number;
  marginBottom: number;
  marginLeft: number;
  marginRight: number;
};

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
  { label: '직접 지정', value: 'custom' as const },
];

export function createDefaultPageLayout(preset: PageLayoutPreset = 'a4'): PageLayout {
  return {
    ...PAGE_SIZE_MAP[preset],
    marginTop: DEFAULT_PAGE_MARGINS[preset].top,
    marginBottom: DEFAULT_PAGE_MARGINS[preset].bottom,
    marginLeft: DEFAULT_PAGE_MARGINS[preset].left,
    marginRight: DEFAULT_PAGE_MARGINS[preset].right,
  };
}

export function getMaxMargin(side: 'top' | 'bottom' | 'left' | 'right', pageLayoutSettings: PageLayout): number {
  if (side === 'left') {
    return pageLayoutSettings.width - pageLayoutSettings.marginRight - MIN_CONTENT_SIZE_MM;
  } else if (side === 'right') {
    return pageLayoutSettings.width - pageLayoutSettings.marginLeft - MIN_CONTENT_SIZE_MM;
  } else if (side === 'top') {
    return pageLayoutSettings.height - pageLayoutSettings.marginBottom - MIN_CONTENT_SIZE_MM;
  } else {
    return pageLayoutSettings.height - pageLayoutSettings.marginTop - MIN_CONTENT_SIZE_MM;
  }
}
