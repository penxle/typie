import dayjs from 'dayjs';
import { db } from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { enqueueDiscoveryPublicationSync } from '#/utils/discovery-index.ts';
import { promoteDuePublicationsCore } from '#/utils/publication.ts';
import { defineCron } from '../types.ts';

export const PublicationPublishScheduledCron = defineCron('publication:publish-scheduled', '* * * * *', async () => {
  const promoted = await db.transaction(async (tx) => await promoteDuePublicationsCore(tx, { now: dayjs() }));
  await enqueueDiscoveryPublicationSync(promoted.map((row) => row.publicationId));
  for (const siteId of new Set(promoted.map((row) => row.siteId))) {
    pubsub.publish('site:update', siteId, { scope: 'site' });
  }
});
