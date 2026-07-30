import { animateFlip } from '@typie/ui/utils';
import { afterEach, describe, expect, it, vi } from 'vitest';

const prepareMovingItem = async () => {
  vi.useFakeTimers();
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    callback(0);
    return 1;
  });

  const container = document.createElement('div');
  const item = document.createElement('div');
  item.dataset.noteId = 'note-1';
  const sibling = document.createElement('div');
  sibling.dataset.noteId = 'note-2';
  const stationary = document.createElement('div');
  stationary.dataset.noteId = 'note-3';
  container.append(item, sibling, stationary);
  document.body.append(container);

  vi.spyOn(container, 'getBoundingClientRect').mockReturnValue(new DOMRect(0, 0, 100, 100));
  vi.spyOn(item, 'getBoundingClientRect')
    .mockReturnValueOnce(new DOMRect(0, 100, 100, 50))
    .mockReturnValueOnce(new DOMRect(0, 0, 100, 50));
  vi.spyOn(sibling, 'getBoundingClientRect')
    .mockReturnValueOnce(new DOMRect(0, 0, 100, 50))
    .mockReturnValueOnce(new DOMRect(0, 100, 100, 50));
  vi.spyOn(stationary, 'getBoundingClientRect').mockReturnValue(new DOMRect(0, 200, 100, 50));

  await animateFlip('[data-note-id]', 'noteId', container);
  return { item, sibling, stationary };
};

afterEach(() => {
  vi.runOnlyPendingTimers();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

describe('animateFlip', () => {
  it.each(['transitionend', 'transitioncancel'])('finishes the animation on %s', async (eventType) => {
    const { item } = await prepareMovingItem();
    expect(item.style.transition).toContain('transform 300ms');

    const transitionEvent = new Event(eventType);
    Object.defineProperty(transitionEvent, 'propertyName', { value: 'transform' });
    item.dispatchEvent(transitionEvent);

    expect(item.style.transition).toBe('');
  });

  it('keeps pointer input enabled and finishes the transition on pointerdown', async () => {
    const { item, sibling, stationary } = await prepareMovingItem();
    const dragHandle = document.createElement('div');
    stationary.append(dragHandle);
    let transitionAtDragHandler = '';
    let siblingTransitionAtDragHandler = '';
    dragHandle.addEventListener('pointerdown', () => {
      transitionAtDragHandler = item.style.transition;
      siblingTransitionAtDragHandler = sibling.style.transition;
    });

    expect(item.style.pointerEvents).toBe('');
    expect(item.style.transition).toContain('transform 300ms');
    expect(sibling.style.transition).toContain('transform 300ms');

    dragHandle.dispatchEvent(new Event('pointerdown', { bubbles: true }));

    expect(transitionAtDragHandler).toBe('');
    expect(siblingTransitionAtDragHandler).toBe('');
    expect(item.style.transition).toBe('');
    expect(sibling.style.transition).toBe('');
  });
});
