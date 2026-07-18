export type SweepReportEntry = { documentId: string; reason: 'failed' | 'deferred'; message?: string };
export type SweepCommentHit = { documentId: string; threadId: string; hitDots: string[] | null };

export const shardOf = (documentId: string, workers: number): number => {
  let hash = 0x81_1c_9d_c5;
  for (const char of documentId) {
    hash ^= char.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 0x01_00_01_93);
  }
  return (hash >>> 0) % workers;
};

export const mergeReportEntries = (existing: readonly SweepReportEntry[], incoming: readonly SweepReportEntry[]): SweepReportEntry[] => {
  const byDocument = new Map<string, SweepReportEntry>();
  for (const entry of existing) {
    byDocument.set(entry.documentId, entry);
  }
  for (const entry of incoming) {
    byDocument.set(entry.documentId, entry);
  }
  return [...byDocument.values()];
};

// documentId/threadId are id strings that never contain a space, so a space separator keeps
// (documentId, threadId) pairs with a shared prefix from colliding onto one key.
const hitKey = (hit: SweepCommentHit): string => `${hit.documentId} ${hit.threadId}`;

export const mergeCommentHits = (existing: readonly SweepCommentHit[], incoming: readonly SweepCommentHit[]): SweepCommentHit[] => {
  const byThread = new Map<string, SweepCommentHit>();
  for (const hit of existing) {
    byThread.set(hitKey(hit), hit);
  }
  for (const hit of incoming) {
    byThread.set(hitKey(hit), hit);
  }
  return [...byThread.values()];
};
