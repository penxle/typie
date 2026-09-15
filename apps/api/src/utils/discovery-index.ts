import { inArray } from 'drizzle-orm';

type DiscoveryIndexSyncDeps = {
  findPublicationIdsByDocumentIds: (documentIds: string[]) => Promise<string[]>;
  enqueuePublicationIndexJob: (publicationId: string) => Promise<void>;
  enqueueSiteIndexJob: (siteId: string) => Promise<void>;
};

const unique = (ids: string[]) => [...new Set(ids)];

const defaultDeps: DiscoveryIndexSyncDeps = {
  findPublicationIdsByDocumentIds: async (documentIds) => {
    if (documentIds.length === 0) {
      return [];
    }

    const { db, Publications } = await import('#/db/index.ts');

    return await db
      .select({ id: Publications.id })
      .from(Publications)
      .where(inArray(Publications.documentId, documentIds))
      .then((rows) => rows.map(({ id }) => id));
  },
  enqueuePublicationIndexJob: async (publicationId) => {
    const { enqueueJob } = await import('#/mq/index.ts');
    await enqueueJob('search:index:publication', publicationId);
  },
  enqueueSiteIndexJob: async (siteId) => {
    const { enqueueJob } = await import('#/mq/index.ts');
    await enqueueJob('search:index:site', siteId);
  },
};

export const enqueueDiscoveryPublicationSync = async (
  publicationIds: string[],
  deps: DiscoveryIndexSyncDeps = defaultDeps,
): Promise<void> => {
  for (const id of unique(publicationIds)) {
    await deps.enqueuePublicationIndexJob(id);
  }
};

export const enqueueDiscoverySiteSync = async (siteIds: string[], deps: DiscoveryIndexSyncDeps = defaultDeps): Promise<void> => {
  for (const id of unique(siteIds)) {
    await deps.enqueueSiteIndexJob(id);
  }
};

export const enqueueDiscoverySyncForDocumentIds = async (
  documentIds: string[],
  deps: DiscoveryIndexSyncDeps = defaultDeps,
): Promise<void> => {
  const ids = unique(documentIds);

  if (ids.length === 0) {
    return;
  }

  await enqueueDiscoveryPublicationSync(await deps.findPublicationIdsByDocumentIds(ids), deps);
};
