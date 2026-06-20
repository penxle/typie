import { values } from '$lib/editor/values';

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

export const getPageElement = (element: HTMLElement): HTMLElement | null => {
  let currentElement = element;
  while (true) {
    if (currentElement.dataset.pageIndex) {
      return currentElement;
    }
    if (!currentElement.parentElement || !(currentElement.parentElement instanceof HTMLElement)) {
      break;
    }
    currentElement = currentElement.parentElement;
  }
  return null;
};

export const getPageIndex = (element: HTMLElement): number => {
  const pageElement = getPageElement(element);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  return pageElement ? Number.parseInt(pageElement.dataset.pageIndex!) : -1;
};

export const calculateRelativePosition = (element: HTMLElement, e: MouseEvent | PointerEvent) => {
  const rect = element.getBoundingClientRect();
  return {
    x: e.clientX - rect.left,
    y: e.clientY - rect.top,
  };
};

export const idleCallback = (callback: () => void): void => {
  if (typeof requestIdleCallback === 'undefined') {
    setTimeout(callback, 0);
  } else {
    requestIdleCallback(callback);
  }
};

export type ExtensionAreaCoordinate = {
  pageIdx: number;
  x: number;
  y: number;
  pageElement: HTMLElement;
};

export const findNearestPageCoordinate = (
  e: MouseEvent | PointerEvent,
  pageElements: HTMLElement[],
  pageWidth: number,
  zoom = 1,
): ExtensionAreaCoordinate | null => {
  if (pageElements.length === 0) return null;

  const safeZoom = Number.isFinite(zoom) && zoom > 0 ? zoom : 1;

  const eventY = e.clientY;

  let nearestPageIdx = 0;
  let nearestPageEl = pageElements[0];
  let minDistance = Infinity;

  for (const [i, pageEl] of pageElements.entries()) {
    if (!pageEl) continue;

    const pageRect = pageEl.getBoundingClientRect();
    const pageTop = pageRect.top;
    const pageBottom = pageRect.bottom;

    if (eventY >= pageTop && eventY <= pageBottom) {
      nearestPageIdx = i;
      nearestPageEl = pageEl;
      break;
    }

    const distanceToTop = Math.abs(eventY - pageTop);
    const distanceToBottom = Math.abs(eventY - pageBottom);
    const distance = Math.min(distanceToTop, distanceToBottom);

    if (distance < minDistance) {
      minDistance = distance;
      nearestPageIdx = i;
      nearestPageEl = pageEl;
    }
  }

  const pageRect = nearestPageEl.getBoundingClientRect();

  const relativeX = Math.max(0, Math.min(pageWidth, (e.clientX - pageRect.left) / safeZoom));

  let relativeY: number;
  if (eventY < pageRect.top) {
    relativeY = 0;
  } else if (eventY > pageRect.bottom) {
    relativeY = pageRect.height / safeZoom;
  } else {
    relativeY = (eventY - pageRect.top) / safeZoom;
  }

  return {
    pageIdx: nearestPageIdx,
    x: relativeX,
    y: relativeY,
    pageElement: nearestPageEl,
  };
};

export function calculateImageDisplaySize(
  bounds: { width: number; height: number },
  originalWidth: number,
  originalHeight: number,
): { displayWidth: number; xOffset: number } {
  if (originalWidth > 0 && originalHeight > 0) {
    const aspectRatio = originalWidth / originalHeight;
    let displayWidth = bounds.height * aspectRatio;
    if (displayWidth > bounds.width) displayWidth = bounds.width;
    const xOffset = (bounds.width - displayWidth) / 2;
    return { displayWidth, xOffset };
  }
  return { displayWidth: bounds.width, xOffset: 0 };
}
