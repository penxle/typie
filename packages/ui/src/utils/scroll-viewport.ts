export type ScrollViewport = {
  target: EventTarget;
  getRect(): { top: number; bottom: number; left: number; right: number };
  getScrollTop(): number;
  getScrollLeft(): number;
  getScrollWidth(): number;
  getScrollHeight(): number;
  scrollBy(x: number, y: number): void;
  scrollTo(options: ScrollToOptions): void;
};

const DEFAULT_LINE_HEIGHT_PX = 16;

function resolveWheelDeltas(element: HTMLElement, event: WheelEvent): { x: number; y: number } {
  const shiftHorizontal = event.shiftKey && event.deltaX === 0;
  const deltaX = shiftHorizontal ? event.deltaY : event.deltaX;
  const deltaY = shiftHorizontal ? 0 : event.deltaY;
  if (event.deltaMode === WheelEvent.DOM_DELTA_LINE) {
    const computedLineHeight = Number.parseFloat(getComputedStyle(element).lineHeight);
    const lineHeight = Number.isFinite(computedLineHeight) ? computedLineHeight : DEFAULT_LINE_HEIGHT_PX;
    return { x: deltaX * lineHeight, y: deltaY * lineHeight };
  }
  if (event.deltaMode === WheelEvent.DOM_DELTA_PAGE) {
    return { x: deltaX * element.clientWidth, y: deltaY * element.clientHeight };
  }
  return { x: deltaX, y: deltaY };
}

export function scrollElementFromWheel(element: HTMLElement, event: WheelEvent): boolean {
  if (event.defaultPrevented || event.metaKey || event.ctrlKey) return false;

  const delta = resolveWheelDeltas(element, event);
  if (delta.x === 0 && delta.y === 0) return false;

  const previousScrollLeft = element.scrollLeft;
  const previousScrollTop = element.scrollTop;
  element.scrollBy(delta.x, delta.y);

  const consumed = element.scrollLeft !== previousScrollLeft || element.scrollTop !== previousScrollTop;
  if (consumed && event.cancelable) event.preventDefault();
  return consumed;
}

export function elementScrollViewport(el: HTMLElement): ScrollViewport {
  return {
    target: el,
    getRect: () => el.getBoundingClientRect(),
    getScrollTop: () => el.scrollTop,
    getScrollLeft: () => el.scrollLeft,
    getScrollWidth: () => el.scrollWidth,
    getScrollHeight: () => el.scrollHeight,
    scrollBy: (x, y) => el.scrollBy(x, y),
    scrollTo: (options) => el.scrollTo(options),
  };
}

export function windowScrollViewport(): ScrollViewport {
  return {
    target: window,
    getRect: () => ({ top: 0, bottom: window.innerHeight, left: 0, right: window.innerWidth }),
    getScrollTop: () => window.scrollY,
    getScrollLeft: () => window.scrollX,
    getScrollWidth: () => document.scrollingElement?.scrollWidth ?? 0,
    getScrollHeight: () => document.scrollingElement?.scrollHeight ?? 0,
    scrollBy: (x, y) => window.scrollBy(x, y),
    scrollTo: (options) => window.scrollTo(options),
  };
}
