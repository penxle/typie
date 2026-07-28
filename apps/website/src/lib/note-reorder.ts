export type NoteReorderGeometry = {
  top: number;
  bottom: number;
};

export type NoteReorderDirection = -1 | 0 | 1;

export const reorderedNoteIdsForDrag = (
  noteIds: readonly string[],
  draggedNoteId: string,
  direction: NoteReorderDirection,
  noteGeometries: ReadonlyMap<string, NoteReorderGeometry>,
): string[] | null => {
  if (noteIds.length < 2) return null;

  const currentIndex = noteIds.indexOf(draggedNoteId);
  if (currentIndex === -1) return null;

  const draggedGeometry = noteGeometries.get(draggedNoteId);
  if (!draggedGeometry) return null;

  let insertionIndex = currentIndex;

  if (direction < 0) {
    while (insertionIndex > 0) {
      const previousNoteId = noteIds[insertionIndex - 1];
      const previousGeometry = noteGeometries.get(previousNoteId);
      if (!previousGeometry || !shouldSwapTowardsPrevious(draggedGeometry, previousGeometry)) break;
      insertionIndex -= 1;
    }
  } else if (direction > 0) {
    while (insertionIndex < noteIds.length - 1) {
      const nextNoteId = noteIds[insertionIndex + 1];
      const nextGeometry = noteGeometries.get(nextNoteId);
      if (!nextGeometry || !shouldSwapTowardsNext(draggedGeometry, nextGeometry)) break;
      insertionIndex += 1;
    }
  }

  const reorderedNoteIds = [...noteIds];
  reorderedNoteIds.splice(currentIndex, 1);
  reorderedNoteIds.splice(insertionIndex, 0, draggedNoteId);
  return reorderedNoteIds;
};

const shouldSwapTowardsPrevious = (dragged: NoteReorderGeometry, previous: NoteReorderGeometry): boolean => {
  return verticalOverlap(dragged, previous) > requiredOverlap(dragged, previous) || dragged.bottom <= previous.top;
};

const shouldSwapTowardsNext = (dragged: NoteReorderGeometry, next: NoteReorderGeometry): boolean => {
  return verticalOverlap(dragged, next) > requiredOverlap(dragged, next) || dragged.top >= next.bottom;
};

const requiredOverlap = (first: NoteReorderGeometry, second: NoteReorderGeometry): number => {
  return Math.min(first.bottom - first.top, second.bottom - second.top) / 2;
};

const verticalOverlap = (first: NoteReorderGeometry, second: NoteReorderGeometry): number => {
  return Math.max(0, Math.min(first.bottom, second.bottom) - Math.max(first.top, second.top));
};
