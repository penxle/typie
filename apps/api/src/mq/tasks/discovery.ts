import { indexPublications, indexSites } from '#/utils/discovery-indexer.ts';
import { defineJob } from '../types.ts';

export const PublicationIndexJob = defineJob('search:index:publication', async (publicationId: string) => {
  await indexPublications([publicationId]);
});

export const SiteIndexJob = defineJob('search:index:site', async (siteId: string) => {
  await indexSites([siteId]);
});
