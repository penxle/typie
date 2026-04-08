import { EntityState } from '@typie/lib/enums';
import { and, eq, inArray } from 'drizzle-orm';

type SearchHitKind = 'document' | 'folder';

export type SearchHitCandidate<T> = {
  kind: SearchHitKind;
  id: string;
  payload: T;
};

type SearchHitVisibilityDeps = {
  findActiveDocumentIds: (ids: string[]) => Promise<string[]>;
  findActiveFolderIds: (ids: string[]) => Promise<string[]>;
};

type SearchIndexSyncDeps = {
  findDocumentIdsByEntityIds: (entityIds: string[]) => Promise<string[]>;
  findFolderIdsByEntityIds: (entityIds: string[]) => Promise<string[]>;
  enqueueDocumentIndexJob: (id: string) => Promise<void>;
  enqueueFolderIndexJob: (id: string) => Promise<void>;
};

const unique = (ids: string[]) => [...new Set(ids)];

const defaultSearchHitVisibilityDeps: SearchHitVisibilityDeps = {
  findActiveDocumentIds: async (ids) => {
    if (ids.length === 0) {
      return [];
    }

    const { db, Documents, Entities } = await import('#/db/index.ts');

    return await db
      .select({ id: Documents.id })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .where(and(inArray(Documents.id, ids), eq(Entities.state, EntityState.ACTIVE)))
      .then((rows) => rows.map(({ id }) => id));
  },
  findActiveFolderIds: async (ids) => {
    if (ids.length === 0) {
      return [];
    }

    const { db, Entities, Folders } = await import('#/db/index.ts');

    return await db
      .select({ id: Folders.id })
      .from(Folders)
      .innerJoin(Entities, eq(Folders.entityId, Entities.id))
      .where(and(inArray(Folders.id, ids), eq(Entities.state, EntityState.ACTIVE)))
      .then((rows) => rows.map(({ id }) => id));
  },
};

const defaultSearchIndexSyncDeps: SearchIndexSyncDeps = {
  findDocumentIdsByEntityIds: async (entityIds) => {
    if (entityIds.length === 0) {
      return [];
    }

    const { db, Documents } = await import('#/db/index.ts');

    return await db
      .select({ id: Documents.id })
      .from(Documents)
      .where(inArray(Documents.entityId, entityIds))
      .then((rows) => rows.map(({ id }) => id));
  },
  findFolderIdsByEntityIds: async (entityIds) => {
    if (entityIds.length === 0) {
      return [];
    }

    const { db, Folders } = await import('#/db/index.ts');

    return await db
      .select({ id: Folders.id })
      .from(Folders)
      .where(inArray(Folders.entityId, entityIds))
      .then((rows) => rows.map(({ id }) => id));
  },
  enqueueDocumentIndexJob: async (id) => {
    const { enqueueJob } = await import('#/mq/index.ts');
    await enqueueJob('search:index:document', id);
  },
  enqueueFolderIndexJob: async (id) => {
    const { enqueueJob } = await import('#/mq/index.ts');
    await enqueueJob('search:index:folder', id);
  },
};

export const filterVisibleSearchHits = async <T>(
  hits: SearchHitCandidate<T>[],
  deps: SearchHitVisibilityDeps = defaultSearchHitVisibilityDeps,
): Promise<SearchHitCandidate<T>[]> => {
  const documentIds = unique(hits.filter((hit) => hit.kind === 'document').map((hit) => hit.id));
  const folderIds = unique(hits.filter((hit) => hit.kind === 'folder').map((hit) => hit.id));

  const [activeDocumentIds, activeFolderIds] = await Promise.all([
    deps.findActiveDocumentIds(documentIds),
    deps.findActiveFolderIds(folderIds),
  ]);

  const visibleDocumentIds = new Set(activeDocumentIds);
  const visibleFolderIds = new Set(activeFolderIds);

  return hits.filter((hit) => {
    if (hit.kind === 'document') {
      return visibleDocumentIds.has(hit.id);
    }

    return visibleFolderIds.has(hit.id);
  });
};

export const enqueueSearchSyncForEntityIds = async (
  entityIds: string[],
  deps: SearchIndexSyncDeps = defaultSearchIndexSyncDeps,
): Promise<void> => {
  const uniqueEntityIds = unique(entityIds);

  if (uniqueEntityIds.length === 0) {
    return;
  }

  const [documentIds, folderIds] = await Promise.all([
    deps.findDocumentIdsByEntityIds(uniqueEntityIds),
    deps.findFolderIdsByEntityIds(uniqueEntityIds),
  ]);

  for (const documentId of unique(documentIds)) {
    await deps.enqueueDocumentIndexJob(documentId);
  }

  for (const folderId of unique(folderIds)) {
    await deps.enqueueFolderIndexJob(folderId);
  }
};
