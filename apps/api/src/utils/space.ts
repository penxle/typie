import { SpaceState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, eq, inArray, ne } from 'drizzle-orm';
import { first, firstOrThrow } from '#/db/index.ts';
import { Spaces } from '#/db/schemas/tables.ts';
import { assertSitePermission } from './permission.ts';
import { assertActiveSubscription } from './plan.ts';
import { unpublishBySiteIdsCore, unpublishBySpaceIdCore } from './publication-unpublish.ts';
import { normalizeSpaceLinks } from './space-core.ts';
import type { SpaceDateDisplay } from '@typie/lib/enums';
import type { Dayjs } from 'dayjs';
import type { Database, Transaction } from '#/db/index.ts';
import type { SpaceLink } from './space-core.ts';

export const findActiveSpace = async (executor: Database | Transaction, spaceId: string) =>
  await executor
    .select()
    .from(Spaces)
    .where(and(eq(Spaces.id, spaceId), eq(Spaces.state, SpaceState.ACTIVE)))
    .then(first);

const assertSlugAvailable = async (executor: Database | Transaction, slug: string, exceptSpaceId?: string) => {
  const taken = await executor
    .select({ id: Spaces.id })
    .from(Spaces)
    .where(exceptSpaceId ? and(eq(Spaces.slug, slug), ne(Spaces.id, exceptSpaceId)) : eq(Spaces.slug, slug))
    .then(first);
  if (taken) throw new TypieError({ code: 'space_slug_already_exists', status: 409 });
};

export const createSpaceCore = async (
  executor: Database | Transaction,
  args: { userId: string; siteId: string; name: string; slug: string; logoId?: string | null },
) => {
  await assertSitePermission({ userId: args.userId, siteId: args.siteId });
  await assertActiveSubscription({ userId: args.userId });
  await assertSlugAvailable(executor, args.slug);
  return await executor
    .insert(Spaces)
    .values({ siteId: args.siteId, name: args.name, slug: args.slug, logoId: args.logoId ?? null })
    .returning()
    .then(firstOrThrow);
};

export const updateSpaceCore = async (
  executor: Database | Transaction,
  args: {
    userId: string;
    spaceId: string;
    name?: string | null;
    slug?: string | null;
    logoId?: string | null;
    description?: string | null;
    links?: SpaceLink[] | null;
    allowIndexing?: boolean | null;
    dateDisplay?: SpaceDateDisplay | null;
  },
) => {
  const space = await findActiveSpace(executor, args.spaceId);
  if (!space) throw new TypieError({ code: 'space_not_found', status: 404 });
  await assertSitePermission({ userId: args.userId, siteId: space.siteId });
  await assertActiveSubscription({ userId: args.userId });
  if (args.slug != null && args.slug !== space.slug) await assertSlugAvailable(executor, args.slug, space.id);

  const set: Partial<typeof Spaces.$inferInsert> = {};
  if (args.name != null) set.name = args.name;
  if (args.slug != null) set.slug = args.slug;
  if (args.logoId !== undefined) set.logoId = args.logoId;
  if (args.description !== undefined) set.description = args.description;
  if (args.links != null) set.links = normalizeSpaceLinks(args.links);
  if (args.allowIndexing != null) set.allowIndexing = args.allowIndexing;
  if (args.dateDisplay != null) set.dateDisplay = args.dateDisplay;
  if (Object.keys(set).length === 0) return space;

  return await executor.update(Spaces).set(set).where(eq(Spaces.id, space.id)).returning().then(firstOrThrow);
};

export const deleteSpaceCore = async (executor: Database | Transaction, args: { userId: string; spaceId: string; now: Dayjs }) => {
  const space = await findActiveSpace(executor, args.spaceId);
  if (!space) throw new TypieError({ code: 'space_not_found', status: 404 });
  await assertSitePermission({ userId: args.userId, siteId: space.siteId });
  return await executor.transaction(async (tx) => {
    await unpublishBySpaceIdCore(tx, { spaceId: space.id, now: args.now });
    return await tx.update(Spaces).set({ state: SpaceState.DELETED }).where(eq(Spaces.id, space.id)).returning().then(firstOrThrow);
  });
};

export const deleteSpacesBySiteIdsCore = async (tx: Transaction, args: { siteIds: string[]; now: Dayjs }) => {
  if (args.siteIds.length === 0) return;
  await unpublishBySiteIdsCore(tx, { siteIds: args.siteIds, now: args.now });
  await tx
    .update(Spaces)
    .set({ state: SpaceState.DELETED })
    .where(and(inArray(Spaces.siteId, args.siteIds), eq(Spaces.state, SpaceState.ACTIVE)));
};
