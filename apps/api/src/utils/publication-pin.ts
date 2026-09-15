import { PublicationState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, eq, isNotNull, sql } from 'drizzle-orm';
import { first, firstOrThrow } from '#/db/index.ts';
import { Publications, Sites } from '#/db/schemas/tables.ts';
import { generateFractionalOrder } from './order.ts';
import { assertSitePermission } from './permission.ts';
import { assertActiveSubscription } from './plan.ts';
import { canPinMore } from './publication-view-core.ts';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

const loadPublicationForWrite = async (executor: Executor, userId: string, publicationId: string) => {
  const publication = await executor.select().from(Publications).where(eq(Publications.id, publicationId)).then(first);
  if (!publication) throw new TypieError({ code: 'publication_not_found', status: 404 });
  await assertSitePermission({ userId, siteId: publication.siteId });
  await assertActiveSubscription({ userId });
  return publication;
};

export const pinPublicationCore = async (
  executor: Executor,
  args: { userId: string; publicationId: string; lowerOrder?: string | null; upperOrder?: string | null },
) => {
  const publication = await loadPublicationForWrite(executor, args.userId, args.publicationId);
  if (publication.state !== PublicationState.PUBLISHED) throw new TypieError({ code: 'publication_not_published', status: 409 });
  return await executor.transaction(async (tx) => {
    await tx.select({ id: Sites.id }).from(Sites).where(eq(Sites.id, publication.siteId)).for('update');
    const [{ count }] = await tx
      .select({ count: sql<number>`count(*)::int` })
      .from(Publications)
      .where(
        and(eq(Publications.siteId, publication.siteId), isNotNull(Publications.pinnedOrder), sql`${Publications.id} <> ${publication.id}`),
      );
    if (!canPinMore(count)) throw new TypieError({ code: 'site_pin_limit', status: 409 });
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
