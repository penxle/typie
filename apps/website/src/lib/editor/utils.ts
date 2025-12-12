export const getPageIndex = (element: HTMLElement): number => {
  let currentElement = element;
  while (true) {
    if (currentElement.dataset.pageIndex) {
      return Number.parseInt(currentElement.dataset.pageIndex);
    }
    if (!currentElement.parentElement || !(currentElement.parentElement instanceof HTMLElement)) {
      break;
    }
    currentElement = currentElement.parentElement;
  }
  return -1;
};

export const calculateRelativePosition = (element: HTMLElement, e: MouseEvent | PointerEvent) => {
  const rect = element.getBoundingClientRect();
  return {
    x: e.clientX - rect.left,
    y: e.clientY - rect.top,
  };
};

export const findScroller = (element: HTMLElement): HTMLElement => {
  let currentElement = element;
  while (true) {
    const style = getComputedStyle(currentElement);
    const overflowY = style.overflowY;
    const isScrollable = overflowY === 'auto' || overflowY === 'scroll' || overflowY === 'overlay';

    if (isScrollable && currentElement.scrollHeight > currentElement.clientHeight) {
      return currentElement;
    }
    if (!currentElement.parentElement || !(currentElement.parentElement instanceof HTMLElement)) {
      break;
    }
    currentElement = currentElement.parentElement;
  }
  return currentElement;
};

export const idleCallback = (callback: () => void): void => {
  if (typeof requestIdleCallback === 'undefined') {
    setTimeout(callback);
  } else {
    requestIdleCallback(callback);
  }
};
