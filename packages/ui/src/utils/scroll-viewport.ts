export type ScrollViewport = {
  target: EventTarget;
  getRect(): { top: number; bottom: number; left: number; right: number };
  getScrollTop(): number;
  getScrollLeft(): number;
  getScrollHeight(): number;
  scrollBy(x: number, y: number): void;
  scrollTo(options: ScrollToOptions): void;
};

export function elementScrollViewport(el: HTMLElement): ScrollViewport {
  return {
    target: el,
    getRect: () => el.getBoundingClientRect(),
    getScrollTop: () => el.scrollTop,
    getScrollLeft: () => el.scrollLeft,
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
    getScrollHeight: () => document.scrollingElement?.scrollHeight ?? 0,
    scrollBy: (x, y) => window.scrollBy(x, y),
    scrollTo: (options) => window.scrollTo(options),
  };
}
