import type { DragIndicatorState } from './DragIndicator.svelte';

export type PinnedPlacementItem = { id: string; pinnedOrder: string };

export type PinnedPlacement = { targetId: string; position: 'before' | 'after' };

const placementOn = (row: HTMLElement, clientY: number): PinnedPlacement | null => {
  if (!row.dataset.id) return null;

  const rect = row.getBoundingClientRect();
  return { targetId: row.dataset.id, position: clientY < rect.top + rect.height / 2 ? 'before' : 'after' };
};

export const resolvePinnedPlacementAt = (listElement: HTMLElement, hit: Element | null, clientY: number): PinnedPlacement | null => {
  if (!hit || !listElement.contains(hit)) return null;

  const row = hit.closest<HTMLElement>('[data-id]');
  if (row && listElement.contains(row)) {
    const placement = placementOn(row, clientY);
    if (placement) return placement;
  }

  const rows = [...listElement.querySelectorAll<HTMLElement>('[data-id]')];

  const covering = rows.find((candidate) => {
    const rect = candidate.getBoundingClientRect();
    return clientY >= rect.top && clientY < rect.bottom;
  });
  if (covering) return placementOn(covering, clientY);

  const first = rows[0];
  if (first && clientY < first.getBoundingClientRect().top) {
    return first.dataset.id ? { targetId: first.dataset.id, position: 'before' } : null;
  }

  const last = rows.at(-1);
  return last?.dataset.id ? { targetId: last.dataset.id, position: 'after' } : null;
};

export const resolvePinnedOrders = (
  items: readonly PinnedPlacementItem[],
  draggedIds: readonly string[],
  placement: PinnedPlacement,
): { lowerOrder: string | null; upperOrder: string | null } | null => {
  const dragged = new Set(draggedIds);
  if (dragged.size === 0 || dragged.has(placement.targetId)) return null;

  const rest = items.filter((item) => !dragged.has(item.id));
  const targetIndex = rest.findIndex((item) => item.id === placement.targetId);
  if (targetIndex === -1) return null;

  const insertAt = placement.position === 'before' ? targetIndex : targetIndex + 1;

  const moved = items.filter((item) => dragged.has(item.id));
  if (moved.length === dragged.size) {
    const next = [...rest.slice(0, insertAt), ...moved, ...rest.slice(insertAt)];
    if (next.every((item, index) => item.id === items[index].id)) return null;
  }

  return {
    lowerOrder: rest[insertAt - 1]?.pinnedOrder ?? null,
    upperOrder: rest[insertAt]?.pinnedOrder ?? null,
  };
};

export const pinnedPlacementIndicator = (listElement: HTMLElement, placement: PinnedPlacement): DragIndicatorState => {
  const row = listElement.querySelector<HTMLElement>(`[data-id="${placement.targetId}"]`);
  if (!row) return {};

  const rect = row.getBoundingClientRect();
  return {
    top: placement.position === 'before' ? rect.top : rect.bottom,
    left: rect.left,
    width: rect.width,
    height: 4,
    opacity: 1,
    transform: 'translateY(-50%)',
  };
};
