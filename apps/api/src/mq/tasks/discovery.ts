import { indexPublications, indexSpaces } from '#/utils/discovery-indexer.ts';
import { defineJob } from '../types.ts';

export const PublicationIndexJob = defineJob('search:index:publication', async (publicationId: string) => {
  await indexPublications([publicationId]);
});

export const SpaceIndexJob = defineJob('search:index:space', async (spaceId: string) => {
  await indexSpaces([spaceId]);
});
