import { PublicationState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, eq, isNotNull, sql } from 'drizzle-orm';
import { first, firstOrThrow } from '#/db/index.ts';
import { Collections, Publications, Spaces } from '#/db/schemas/tables.ts';
import { canPinMore } from './collection-core.ts';
import { generateFractionalOrder } from './order.ts';
import { assertSitePermission } from './permission.ts';
import { assertActiveSubscription } from './plan.ts';
import { findActiveSpace } from './space.ts';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

const loadSpaceForWrite = async (executor: Executor, userId: string, spaceId: string) => {
  const space = await findActiveSpace(executor, spaceId);
  if (!space) throw new TypieError({ code: 'space_not_found', status: 404 });
  await assertSitePermission({ userId, siteId: space.siteId });
  await assertActiveSubscription({ userId });
  return space;
};

const loadCollectionForWrite = async (executor: Executor, userId: string, collectionId: string) => {
  const collection = await executor.select().from(Collections).where(eq(Collections.id, collectionId)).then(first);
  if (!collection) throw new TypieError({ code: 'collection_not_found', status: 404 });
  await loadSpaceForWrite(executor, userId, collection.spaceId);
  return collection;
};

export const createCollectionCore = async (
  executor: Executor,
  args: { userId: string; spaceId: string; name: string; description?: string | null; coverId?: string | null },
) => {
  await loadSpaceForWrite(executor, args.userId, args.spaceId);
  return await executor
    .insert(Collections)
    .values({ spaceId: args.spaceId, name: args.name, description: args.description ?? null, coverId: args.coverId ?? null })
    .returning()
    .then(firstOrThrow);
};

export const updateCollectionCore = async (
  executor: Executor,
  args: { userId: string; collectionId: string; name?: string | null; description?: string | null; coverId?: string | null },
) => {
  const collection = await loadCollectionForWrite(executor, args.userId, args.collectionId);
  const set: Partial<typeof Collections.$inferInsert> = {};
  if (args.name != null) set.name = args.name;
  if (args.description !== undefined) set.description = args.description;
  if (args.coverId !== undefined) set.coverId = args.coverId;
  if (Object.keys(set).length === 0) return collection;
  return await executor.update(Collections).set(set).where(eq(Collections.id, collection.id)).returning().then(firstOrThrow);
};

export const deleteCollectionCore = async (executor: Executor, args: { userId: string; collectionId: string }) => {
  const collection = await loadCollectionForWrite(executor, args.userId, args.collectionId);
  return await executor.transaction(async (tx) => {
    await tx.update(Publications).set({ collectionId: null, collectionOrder: null }).where(eq(Publications.collectionId, collection.id));
    await tx.delete(Collections).where(eq(Collections.id, collection.id));
    return collection;
  });
};

const loadPublicationForWrite = async (executor: Executor, userId: string, publicationId: string) => {
  const publication = await executor.select().from(Publications).where(eq(Publications.id, publicationId)).then(first);
  if (!publication) throw new TypieError({ code: 'publication_not_found', status: 404 });
  await loadSpaceForWrite(executor, userId, publication.spaceId);
  return publication;
};

export const movePublicationInCollectionCore = async (
  executor: Executor,
  args: { userId: string; publicationId: string; lowerOrder?: string | null; upperOrder?: string | null },
) => {
  const publication = await loadPublicationForWrite(executor, args.userId, args.publicationId);
  if (!publication.collectionId) throw new TypieError({ code: 'publication_not_in_collection', status: 409 });
  return await executor
    .update(Publications)
    .set({ collectionOrder: generateFractionalOrder({ lower: args.lowerOrder ?? null, upper: args.upperOrder ?? null }) })
    .where(eq(Publications.id, publication.id))
    .returning()
    .then(firstOrThrow);
};

export const pinPublicationCore = async (
  executor: Executor,
  args: { userId: string; publicationId: string; lowerOrder?: string | null; upperOrder?: string | null },
) => {
  const publication = await loadPublicationForWrite(executor, args.userId, args.publicationId);
  if (publication.state !== PublicationState.PUBLISHED) throw new TypieError({ code: 'publication_not_published', status: 409 });
  return await executor.transaction(async (tx) => {
    await tx.select({ id: Spaces.id }).from(Spaces).where(eq(Spaces.id, publication.spaceId)).for('update');
    const [{ count }] = await tx
      .select({ count: sql<number>`count(*)::int` })
      .from(Publications)
      .where(
        and(
          eq(Publications.spaceId, publication.spaceId),
          isNotNull(Publications.pinnedOrder),
          sql`${Publications.id} <> ${publication.id}`,
        ),
      );
    if (!canPinMore(count)) throw new TypieError({ code: 'space_pin_limit', status: 409 });
    return await tx
      .update(Publications)
      .set({ pinnedOrder: generateFractionalOrder({ lower: args.lowerOrder ?? null, upper: args.upperOrder ?? null }) })
      .where(eq(Publications.id, publication.id))
      .returning()
      .then(firstOrThrow);
  });
};

export const unpinPublicationCore = async (executor: Executor, args: { userId: string; publicationId: string }) => {
  const publication = await loadPublicationForWrite(executor, args.userId, args.publicationId);
  return await executor
    .update(Publications)
    .set({ pinnedOrder: null })
    .where(eq(Publications.id, publication.id))
    .returning()
    .then(firstOrThrow);
};
