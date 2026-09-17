import { elementScrollViewport, windowScrollViewport } from '@typie/ui/utils';
import { afterEach, describe, expect, it, vi } from 'vitest';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('elementScrollViewport', () => {
  it('scrollTo와 scroll extent를 요소에 위임한다', () => {
    const el = document.createElement('div');
    el.scrollTo = vi.fn();
    Object.defineProperties(el, {
      scrollHeight: { value: 1234 },
      scrollWidth: { value: 2345 },
    });

    const viewport = elementScrollViewport(el);
    viewport.scrollTo({ top: 100, behavior: 'smooth' });

    expect(el.scrollTo).toHaveBeenCalledWith({ top: 100, behavior: 'smooth' });
    expect(viewport.getScrollHeight()).toBe(1234);
    expect(viewport.getScrollWidth()).toBe(2345);
  });
});

describe('windowScrollViewport', () => {
  it.each([
    { browser: 'Chrome', scrollX: 0, scrollY: 1000 },
    { browser: 'Safari', scrollX: 180, scrollY: 1450 },
  ])('keeps layout scrolling separate from browser pinch panning in $browser', ({ scrollX, scrollY }) => {
    vi.stubGlobal('visualViewport', { pageLeft: 180, pageTop: 1450, offsetLeft: 180, offsetTop: 450 });
    vi.stubGlobal('scrollX', scrollX);
    vi.stubGlobal('scrollY', scrollY);
    vi.spyOn(document.documentElement, 'clientWidth', 'get').mockReturnValue(390);
    vi.spyOn(document.documentElement, 'clientHeight', 'get').mockReturnValue(800);
    const scrollTo = vi.spyOn(window, 'scrollTo').mockImplementation(vi.fn());
    const viewport = windowScrollViewport();

    expect(viewport.getScrollLeft()).toBe(0);
    expect(viewport.getScrollTop()).toBe(1000);
    expect(viewport.getRect()).toEqual({ left: 0 - scrollX, right: 390 - scrollX, top: 1000 - scrollY, bottom: 1800 - scrollY });
    viewport.scrollTo({ left: 0, top: 1200, behavior: 'instant' });
    expect(scrollTo).toHaveBeenLastCalledWith({ left: scrollX, top: scrollY + 200, behavior: 'instant' });
    viewport.scrollTo({ top: 1200, behavior: 'instant' });
    expect(scrollTo).toHaveBeenLastCalledWith({ top: scrollY + 200, behavior: 'instant' });
  });

  it('scrollTo는 window에, scroll extent는 scrollingElement에 위임한다', () => {
    const scrollToSpy = vi.spyOn(window, 'scrollTo').mockImplementation(vi.fn());
    const scrollingElement = document.createElement('div');
    Object.defineProperties(scrollingElement, {
      scrollHeight: { value: 5678 },
      scrollWidth: { value: 6789 },
    });
    Object.defineProperty(document, 'scrollingElement', { value: scrollingElement, configurable: true });

    const viewport = windowScrollViewport();
    viewport.scrollTo({ top: 50, behavior: 'instant' });

    expect(scrollToSpy).toHaveBeenCalledWith({ top: 50, behavior: 'instant' });
    expect(viewport.getScrollHeight()).toBe(5678);
    expect(viewport.getScrollWidth()).toBe(6789);

    scrollToSpy.mockRestore();
    Reflect.deleteProperty(document, 'scrollingElement');
  });
});
