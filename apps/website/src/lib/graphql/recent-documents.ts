import { writable } from 'svelte/store';

export type RecentDocumentSort = 'VIEWED_AT' | 'UPDATED_AT';

type RecentDocumentInvalidationVersions = Record<RecentDocumentSort, number>;

const RECENT_DOCUMENT_SORTS: readonly RecentDocumentSort[] = ['VIEWED_AT', 'UPDATED_AT'];
const EMPTY_VERSIONS: Readonly<RecentDocumentInvalidationVersions> = { VIEWED_AT: 0, UPDATED_AT: 0 };

export const recentDocumentInvalidationVersions = writable<ReadonlyMap<string, Readonly<RecentDocumentInvalidationVersions>>>(new Map());

const invalidate = (sorts: readonly RecentDocumentSort[], siteIds: (string | null | undefined)[]) => {
  const ids = new Set(siteIds.filter((id): id is string => Boolean(id)));
  if (ids.size === 0) return;

  recentDocumentInvalidationVersions.update((current) => {
    const next = new Map(current);
    for (const siteId of ids) {
      const previous = next.get(siteId) ?? EMPTY_VERSIONS;
      const versions = { ...previous };
      for (const sort of sorts) versions[sort] += 1;
      next.set(siteId, versions);
    }
    return next;
  });
};

export const invalidateRecentDocuments = (...siteIds: (string | null | undefined)[]) => {
  invalidate(RECENT_DOCUMENT_SORTS, siteIds);
};

export const invalidateRecentDocumentsForSort = (sort: RecentDocumentSort, ...siteIds: (string | null | undefined)[]) => {
  invalidate([sort], siteIds);
};
