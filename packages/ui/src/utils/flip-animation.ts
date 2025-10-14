import { tick } from 'svelte';

const ANIMATION_DURATION = 300;
const ANIMATING_ATTR = 'data-flip-animating';

// NOTE: $effect.pre()에서 사용하세요
export const animateFlip = async (selector: string, idAttribute = 'id', container: Document | HTMLElement = document): Promise<void> => {
  const containerElement = container instanceof Document ? null : container;

  if (containerElement) {
    let parent = containerElement.parentElement;
    while (parent) {
      if (parent.hasAttribute(ANIMATING_ATTR)) {
        return;
      }
      parent = parent.parentElement;
    }

    containerElement.setAttribute(ANIMATING_ATTR, 'true');
  }

  const elements = container.querySelectorAll(selector);
  const firstPositions: Record<string, DOMRect> = {};

  elements.forEach((el) => {
    if (!(el instanceof HTMLElement)) return;
    const id = el.dataset[idAttribute];
    if (id) {
      firstPositions[id] = el.getBoundingClientRect();
    }
  });

  const firstContainerHeight = containerElement?.getBoundingClientRect().height;

  await tick();

  const elementsAfter = container.querySelectorAll(selector);
  if (Object.keys(firstPositions).length === 0) {
    if (containerElement) {
      containerElement.removeAttribute(ANIMATING_ATTR);
    }
    return;
  }

  let hasAnimation = false;

  if (containerElement && firstContainerHeight !== undefined) {
    const lastContainerHeight = containerElement.getBoundingClientRect().height;
    const deltaHeight = firstContainerHeight - lastContainerHeight;

    if (Math.abs(deltaHeight) > 0) {
      hasAnimation = true;
      containerElement.style.height = `${firstContainerHeight}px`;
      containerElement.style.transition = 'none';

      const containerRef = new WeakRef(containerElement);
      requestAnimationFrame(() => {
        const element = containerRef.deref();
        if (!element) return;

        element.style.transition = `height ${ANIMATION_DURATION}ms cubic-bezier(0.4, 0, 0.2, 1)`;
        element.style.height = `${lastContainerHeight}px`;

        setTimeout(() => {
          const el = containerRef.deref();
          if (!el) return;

          el.style.height = '';
          el.style.transition = '';
        }, ANIMATION_DURATION);
      });
    }
  }

  for (const el of elementsAfter) {
    if (!(el instanceof HTMLElement)) continue;
    const id = el.dataset[idAttribute];
    if (!id || !firstPositions[id]) continue;

    const prevPos = firstPositions[id];
    const lastPos = el.getBoundingClientRect();
    const deltaX = prevPos.left - lastPos.left;
    const deltaY = prevPos.top - lastPos.top;

    if (Math.abs(deltaX) === 0 && Math.abs(deltaY) === 0) continue;

    hasAnimation = true;
    el.style.transform = `translate(${deltaX}px, ${deltaY}px)`;
    el.style.transition = 'none';

    const elRef = new WeakRef(el);
    requestAnimationFrame(() => {
      const element = elRef.deref();
      if (!element) return;

      element.style.transition = `transform ${ANIMATION_DURATION}ms cubic-bezier(0.4, 0, 0.2, 1)`;
      element.style.transform = '';
      element.style.pointerEvents = 'none';
      setTimeout(() => {
        const el = elRef.deref();
        if (!el) return;

        el.style.transition = 'none';
        el.style.pointerEvents = 'auto';
      }, ANIMATION_DURATION);
    });
  }

  if (!hasAnimation && containerElement) {
    containerElement.removeAttribute(ANIMATING_ATTR);
  } else if (hasAnimation && containerElement) {
    setTimeout(() => {
      containerElement.removeAttribute(ANIMATING_ATTR);
    }, ANIMATION_DURATION);
  }
};
