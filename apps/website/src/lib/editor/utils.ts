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

export const createPaginatedLayout = (preset: PageLayoutPreset = 'a4'): PageLayout => {
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  return { ...values.pageLayout.find((p) => p.value === preset)!.layout };
};

const MIN_CONTENT_SIZE_PX = mmToPx(50);

export const getMaxMargin = (side: 'top' | 'bottom' | 'left' | 'right', layout: PageLayout): number => {
  if (side === 'left') {
    return Math.max(0, layout.pageWidth - layout.pageMarginRight - MIN_CONTENT_SIZE_PX);
  } else if (side === 'right') {
    return Math.max(0, layout.pageWidth - layout.pageMarginLeft - MIN_CONTENT_SIZE_PX);
  } else if (side === 'top') {
    return Math.max(0, layout.pageHeight - layout.pageMarginBottom - MIN_CONTENT_SIZE_PX);
  } else {
    return Math.max(0, layout.pageHeight - layout.pageMarginTop - MIN_CONTENT_SIZE_PX);
  }
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
    setTimeout(callback);
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
  let minDistance = Number.POSITIVE_INFINITY;

  for (const [i, pageEl] of pageElements.entries()) {
    if (!pageEl) continue;

    const pageRect = pageEl.getBoundingClientRect();
    const pageTop = pageRect.top;
    const pageBottom = pageRect.bottom;

    if (eventY >= pageTop && eventY <= pageBottom) {
      nearestPageIdx = i;
      nearestPageEl = pageEl;
      minDistance = 0;
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
