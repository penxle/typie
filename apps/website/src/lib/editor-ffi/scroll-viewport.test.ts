import { elementScrollViewport, windowScrollViewport } from '@typie/ui/utils';
import { describe, expect, it, vi } from 'vitest';

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
