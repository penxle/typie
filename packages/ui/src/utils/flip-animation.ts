import { tick } from 'svelte';

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

  await tick();

  const elementsAfter = container.querySelectorAll(selector);
  if (Object.keys(firstPositions).length === 0) return;

  for (const el of elementsAfter) {
    if (!(el instanceof HTMLElement)) continue;
    const id = el.dataset[idAttribute];
    if (!id || !firstPositions[id]) continue;

    const prevPos = firstPositions[id];
    const lastPos = el.getBoundingClientRect();
    const deltaX = prevPos.left - lastPos.left;
    const deltaY = prevPos.top - lastPos.top;

    if (Math.abs(deltaX) === 0 && Math.abs(deltaY) === 0) continue;

    el.style.transform = `translate(${deltaX}px, ${deltaY}px)`;
    el.style.transition = 'none';

    const elRef = new WeakRef(el);
    requestAnimationFrame(() => {
      const element = elRef.deref();
      if (!element) return;

      element.style.transition = 'transform 300ms cubic-bezier(0.4, 0, 0.2, 1)';
      element.style.transform = '';
      element.style.pointerEvents = 'none';
      setTimeout(() => {
        const el = elRef.deref();
        if (!el) return;

        el.style.transition = 'none';
        el.style.pointerEvents = 'auto';
      }, 300);
    });
  }
};
