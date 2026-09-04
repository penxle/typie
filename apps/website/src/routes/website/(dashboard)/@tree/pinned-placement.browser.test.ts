import { afterEach, describe, expect, it } from 'vitest';
import { resolvePinnedPlacementAt } from './pinned-placement';

const buildList = (options?: { footer?: boolean }) => {
  const list = document.createElement('ul');
  list.style.cssText =
    'position:fixed;top:100px;left:0;width:240px;display:flex;flex-direction:column;padding:4px 12px;margin:0;list-style:none';

  for (const id of ['A', 'B', 'C']) {
    const item = document.createElement('li');
    const row = document.createElement('a');
    row.dataset.id = id;
    row.style.cssText = 'display:flex;height:32px';
    item.append(row);
    list.append(item);
  }

  if (options?.footer) {
    const item = document.createElement('li');
    item.style.cssText = 'display:flex;height:28px';
    list.append(item);
  }

  document.body.append(list);
  return list;
};

const placementAt = (list: HTMLElement, x: number, y: number) => resolvePinnedPlacementAt(list, document.elementFromPoint(x, y), y);

afterEach(() => document.body.replaceChildren());

describe('pinned placement hit testing', () => {
  it('places before the first row from the padding above it', () => {
    const list = buildList();
    expect(placementAt(list, 120, list.getBoundingClientRect().top + 2)).toEqual({ targetId: 'A', position: 'before' });
  });

  it('splits a row at its midpoint', () => {
    const list = buildList();
    const rect = list.querySelector<HTMLElement>('[data-id="B"]')?.getBoundingClientRect();
    expect(rect).toBeDefined();
    expect(placementAt(list, 120, (rect?.top ?? 0) + 4)).toEqual({ targetId: 'B', position: 'before' });
    expect(placementAt(list, 120, (rect?.bottom ?? 0) - 4)).toEqual({ targetId: 'B', position: 'after' });
  });

  it('keeps the row beside the pointer when it strays into the horizontal padding', () => {
    const list = buildList();
    const rect = list.querySelector<HTMLElement>('[data-id="B"]')?.getBoundingClientRect();
    expect(placementAt(list, 4, (rect?.top ?? 0) + 4)).toEqual({ targetId: 'B', position: 'before' });
    expect(placementAt(list, 236, (rect?.bottom ?? 0) - 4)).toEqual({ targetId: 'B', position: 'after' });
  });

  it('places after the last row from the padding below it', () => {
    const list = buildList();
    expect(placementAt(list, 120, list.getBoundingClientRect().bottom - 2)).toEqual({ targetId: 'C', position: 'after' });
  });

  it('places after the last row from a trailing element without a row', () => {
    const list = buildList({ footer: true });
    expect(placementAt(list, 120, list.getBoundingClientRect().bottom - 14)).toEqual({ targetId: 'C', position: 'after' });
  });

  it('ignores pointers outside the list', () => {
    const list = buildList();
    expect(placementAt(list, 120, list.getBoundingClientRect().top - 20)).toBeNull();
  });
});
