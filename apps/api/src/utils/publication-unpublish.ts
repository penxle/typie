import { EntityVisibility, PublicationState } from '@typie/lib/enums';
import { and, eq, inArray } from 'drizzle-orm';
import { Documents, Entities, Publications } from '#/db/schemas/tables.ts';
import { buildPublishingEntityIdsBySiteQuery } from './publication-core.ts';
import type { Dayjs } from 'dayjs';
import type { Transaction } from '#/db/index.ts';

export const unpublishByEntityIdsCore = async (tx: Transaction, args: { entityIds: string[]; now: Dayjs }) => {
  if (args.entityIds.length === 0) return [];
  const affected = await tx
    .select({ publicationId: Publications.id, entityId: Documents.entityId, state: Publications.state })
    .from(Publications)
    .innerJoin(Documents, eq(Publications.documentId, Documents.id))
    .where(
      and(
        inArray(Documents.entityId, args.entityIds),
        inArray(Publications.state, [PublicationState.PUBLISHED, PublicationState.SCHEDULED]),
      ),
    );
  if (affected.length === 0) return [];
  await tx
    .update(Publications)
    .set({ state: PublicationState.UNPUBLISHED, scheduledAt: null, unpublishedAt: args.now, pinnedOrder: null })
    .where(
      inArray(
        Publications.id,
        affected.map((row) => row.publicationId),
      ),
    );
  const publishedEntityIds = affected.filter((row) => row.state === PublicationState.PUBLISHED).map((row) => row.entityId);
  if (publishedEntityIds.length > 0) {
    await tx.update(Entities).set({ visibility: EntityVisibility.PRIVATE }).where(inArray(Entities.id, publishedEntityIds));
  }
  return affected.map((row) => row.publicationId);
};

export const unpublishBySpaceIdCore = async (tx: Transaction, args: { spaceId: string; now: Dayjs }) => {
  const rows = await tx
    .select({ entityId: Documents.entityId })
    .from(Publications)
    .innerJoin(Documents, eq(Publications.documentId, Documents.id))
    .where(
      and(eq(Publications.spaceId, args.spaceId), inArray(Publications.state, [PublicationState.PUBLISHED, PublicationState.SCHEDULED])),
    );
  return await unpublishByEntityIdsCore(tx, { entityIds: rows.map((row) => row.entityId), now: args.now });
};

export const unpublishBySiteIdsCore = async (tx: Transaction, args: { siteIds: string[]; now: Dayjs }) => {
  if (args.siteIds.length === 0) return [];
  const rows = await buildPublishingEntityIdsBySiteQuery(tx, { siteIds: args.siteIds });
  return await unpublishByEntityIdsCore(tx, { entityIds: rows.map((row) => row.entityId), now: args.now });
};
