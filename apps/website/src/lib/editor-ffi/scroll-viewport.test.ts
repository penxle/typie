import { elementScrollViewport, windowScrollViewport } from '@typie/ui/utils';
import { describe, expect, it, vi } from 'vitest';

describe('elementScrollViewport', () => {
  it('scrollTo와 getScrollHeight를 요소에 위임한다', () => {
    const el = document.createElement('div');
    el.scrollTo = vi.fn();
    Object.defineProperty(el, 'scrollHeight', { value: 1234 });

    const viewport = elementScrollViewport(el);
    viewport.scrollTo({ top: 100, behavior: 'smooth' });

    expect(el.scrollTo).toHaveBeenCalledWith({ top: 100, behavior: 'smooth' });
    expect(viewport.getScrollHeight()).toBe(1234);
  });
});

describe('windowScrollViewport', () => {
  it('scrollTo는 window에, getScrollHeight는 scrollingElement에 위임한다', () => {
    const scrollToSpy = vi.spyOn(window, 'scrollTo').mockImplementation(vi.fn());
    const scrollingElement = document.createElement('div');
    Object.defineProperty(scrollingElement, 'scrollHeight', { value: 5678 });
    Object.defineProperty(document, 'scrollingElement', { value: scrollingElement, configurable: true });

    const viewport = windowScrollViewport();
    viewport.scrollTo({ top: 50, behavior: 'instant' });

    expect(scrollToSpy).toHaveBeenCalledWith({ top: 50, behavior: 'instant' });
    expect(viewport.getScrollHeight()).toBe(5678);

    scrollToSpy.mockRestore();
    Reflect.deleteProperty(document, 'scrollingElement');
  });
});
