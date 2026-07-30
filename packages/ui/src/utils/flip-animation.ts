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

  for (const el of elements) {
    if (!(el instanceof HTMLElement)) continue;
    const id = el.dataset[idAttribute];
    if (id) {
      firstPositions[id] = el.getBoundingClientRect();
    }
  }

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
  const pendingTransformFinishes = new Set<() => void>();
  const finishTransforms = () => {
    for (const finish of pendingTransformFinishes) {
      finish();
    }
  };
  const handlePointerDown = (event: Event) => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const item = target.closest(selector);
    if (!item || !container.contains(item)) return;

    finishTransforms();
  };

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
    if (!id || !Object.hasOwn(firstPositions, id)) continue;

    const prevPos = firstPositions[id];
    const lastPos = el.getBoundingClientRect();
    const deltaX = prevPos.left - lastPos.left;
    const deltaY = prevPos.top - lastPos.top;

    if (Math.abs(deltaX) === 0 && Math.abs(deltaY) === 0) continue;

    hasAnimation = true;
    const originalTransition = el.style.transition;
    el.style.transform = `translate(${deltaX}px, ${deltaY}px)`;
    el.style.transition = 'none';

    const elRef = new WeakRef(el);
    requestAnimationFrame(() => {
      const element = elRef.deref();
      if (!element) return;

      let finished = false;
      const finish = () => {
        if (finished) return;
        finished = true;

        element.removeEventListener('transitionend', handleTransitionFinished);
        element.removeEventListener('transitioncancel', handleTransitionFinished);
        pendingTransformFinishes.delete(finish);
        if (pendingTransformFinishes.size === 0) {
          container.removeEventListener('pointerdown', handlePointerDown, { capture: true });
        }
        clearTimeout(fallbackTimeout);
        element.style.transition = originalTransition;
      };
      const handleTransitionFinished = (event: TransitionEvent) => {
        if (event.target === element && event.propertyName === 'transform') {
          finish();
        }
      };

      if (pendingTransformFinishes.size === 0) {
        container.addEventListener('pointerdown', handlePointerDown, { capture: true });
      }
      pendingTransformFinishes.add(finish);
      element.addEventListener('transitionend', handleTransitionFinished);
      element.addEventListener('transitioncancel', handleTransitionFinished);
      element.style.transition = `transform ${ANIMATION_DURATION}ms cubic-bezier(0.4, 0, 0.2, 1)`;
      element.style.transform = '';
      const fallbackTimeout = setTimeout(finish, ANIMATION_DURATION + 100);
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
