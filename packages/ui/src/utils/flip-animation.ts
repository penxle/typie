import { tick } from 'svelte';

const ANIMATION_DURATION = 300;

const animationStates = new WeakMap<HTMLElement, { timerId?: number; cancelled: boolean }>();

// NOTE: $effect.pre()에서 사용하세요
export const animateFlip = async (selector: string, idAttribute = 'id', container: Document | HTMLElement = document): Promise<void> => {
  const elements = container.querySelectorAll(selector);
  const firstPositions: Record<string, DOMRect> = {};

  elements.forEach((el) => {
    if (!(el instanceof HTMLElement)) return;
    const id = el.dataset[idAttribute];
    if (id) {
      firstPositions[id] = el.getBoundingClientRect();
    }
  });

  const containerElement = container instanceof Document ? null : container;
  const firstContainerHeight = containerElement?.getBoundingClientRect().height;

  await tick();

  const elementsAfter = container.querySelectorAll(selector);
  if (Object.keys(firstPositions).length === 0) return;

  if (containerElement && firstContainerHeight !== undefined) {
    const lastContainerHeight = containerElement.getBoundingClientRect().height;
    const deltaHeight = firstContainerHeight - lastContainerHeight;

    if (Math.abs(deltaHeight) > 0) {
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

    const existingState = animationStates.get(el);
    if (existingState) {
      existingState.cancelled = true;
      if (existingState.timerId) {
        clearTimeout(existingState.timerId);
      }
    }

    const state = { cancelled: false };
    animationStates.set(el, state);

    el.style.transform = `translate(${deltaX}px, ${deltaY}px)`;
    el.style.transition = 'none';

    const elRef = new WeakRef(el);
    requestAnimationFrame(() => {
      const element = elRef.deref();
      if (!element) return;

      const currentState = animationStates.get(element);
      if (currentState?.cancelled) return;

      element.style.transition = `transform ${ANIMATION_DURATION}ms cubic-bezier(0.4, 0, 0.2, 1)`;
      element.style.transform = '';
      element.style.pointerEvents = 'none';

      const timerId = setTimeout(() => {
        const el = elRef.deref();
        if (!el) return;

        const finalState = animationStates.get(el);
        if (finalState?.cancelled) return;

        el.style.transition = 'none';
        el.style.pointerEvents = 'auto';
        animationStates.delete(el);
      }, ANIMATION_DURATION);

      if (currentState) {
        currentState.timerId = timerId as unknown as number;
      }
    });
  }
};
