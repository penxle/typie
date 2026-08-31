export type UndoTargetRow = { beforeHeads: Uint8Array; afterHeads: Uint8Array; undone: boolean };

export const headsEqual = (a: Uint8Array | null, b: Uint8Array | null): boolean => {
  if (a === null || b === null) return false;
  if (a.length !== b.length) return false;
  for (const [i, byte] of a.entries()) {
    if (byte !== b[i]) return false;
  }
  return true;
};

// 캐시 미스(live === null)는 "변경 있음"으로 떨어진다 — 틀려도 한 번 더 묻는 쪽으로만 틀린다.
export const changedAfter = (live: Uint8Array | null, checkpoint: Uint8Array): boolean => !headsEqual(live, checkpoint);

export const undoTarget = (row: UndoTargetRow): Uint8Array => (row.undone ? row.afterHeads : row.beforeHeads);
