export type ScrollViewport = {
  target: EventTarget;
  getRect(): { top: number; bottom: number; left: number; right: number };
  getScrollTop(): number;
  scrollBy(x: number, y: number): void;
};

export function elementScrollViewport(el: HTMLElement): ScrollViewport {
  return {
    target: el,
    getRect: () => el.getBoundingClientRect(),
    getScrollTop: () => el.scrollTop,
    scrollBy: (x, y) => el.scrollBy(x, y),
  };
}

export function windowScrollViewport(): ScrollViewport {
  return {
    target: window,
    getRect: () => ({ top: 0, bottom: window.innerHeight, left: 0, right: window.innerWidth }),
    getScrollTop: () => window.scrollY,
    scrollBy: (x, y) => window.scrollBy(x, y),
  };
}
