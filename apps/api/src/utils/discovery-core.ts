import { EntityState, PublicationState, SiteState } from '@typie/lib/enums';
import { and, asc, count, desc, eq, getTableColumns, inArray, isNull, ne, sql } from 'drizzle-orm';
import { Documents, Entities, Publications, PublicationTags, Sites } from '#/db/schemas/tables.ts';
import type { PgSelect } from 'drizzle-orm/pg-core';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

export const DISCOVERY_TAG_LIMIT = 20;

export const discoverablePublicationScope = <T extends PgSelect>(qb: T) =>
  qb
    .innerJoin(Documents, eq(Publications.documentId, Documents.id))
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .innerJoin(Sites, eq(Publications.siteId, Sites.id));

export const discoverablePublicationPredicate = () =>
  and(
    eq(Publications.state, PublicationState.PUBLISHED),
    eq(Entities.state, EntityState.ACTIVE),
    eq(Sites.state, SiteState.ACTIVE),
    eq(Sites.allowIndexing, true),
    eq(Sites.allowDiscovery, true),
    isNull(Documents.password),
  );

const discoverablePublicationsBase = (executor: Executor) =>
  discoverablePublicationScope(executor.select(getTableColumns(Publications)).from(Publications).$dynamic());

export const buildDiscoveryPublicationsQuery = (executor: Executor, input: { tagName?: string; after: string | null; limit: number }) => {
  const base = discoverablePublicationsBase(executor);
  const joined = input.tagName
    ? base.innerJoin(PublicationTags, and(eq(PublicationTags.publicationId, Publications.id), eq(PublicationTags.name, input.tagName)))
    : base;

  return joined
    .where(
      and(
        discoverablePublicationPredicate(),
        input.after
          ? sql`(${Publications.publishedAt}, ${Publications.id}) < (select ${Publications.publishedAt}, ${Publications.id} from ${Publications} where ${Publications.id} = ${input.after})`
          : undefined,
      ),
    )
    .orderBy(desc(Publications.publishedAt), desc(Publications.id))
    .limit(input.limit);
};

export const buildDiscoveryFeedQuery = (executor: Executor, input: { after: string | null; limit: number }) => {
  const prevSiteId = sql<
    string | null
  >`lag(${Publications.siteId}) over (order by ${Publications.publishedAt} desc, ${Publications.id} desc)`.as('prev_site_id');
  const feed = discoverablePublicationScope(
    executor
      .select({ ...getTableColumns(Publications), prevSiteId })
      .from(Publications)
      .$dynamic(),
  )
    .where(
      and(
        discoverablePublicationPredicate(),
        input.after
          ? sql`(${Publications.publishedAt}, ${Publications.id}) <= (select ${Publications.publishedAt}, ${Publications.id} from ${Publications} where ${Publications.id} = ${input.after})`
          : undefined,
      ),
    )
    .as('feed_publications');

  const publicationKeys = Object.keys(getTableColumns(Publications)) as (keyof typeof Publications.$inferSelect)[];
  const selection = Object.fromEntries(publicationKeys.map((key) => [key, feed[key]])) as {
    [K in (typeof publicationKeys)[number]]: (typeof feed)[K];
  };

  return executor
    .select(selection)
    .from(feed)
    .where(and(sql`${feed.prevSiteId} is distinct from ${feed.siteId}`, input.after ? ne(feed.id, input.after) : undefined))
    .orderBy(desc(feed.publishedAt), desc(feed.id))
    .limit(input.limit);
};

export const DISCOVERY_RECENT_SCAN_LIMIT = 200;

export const buildDiscoveryRecentPublicationsQuery = (executor: Executor, input: { limit: number }) =>
  discoverablePublicationScope(
    executor
      .select({ id: Publications.id, siteId: Publications.siteId, documentId: Publications.documentId })
      .from(Publications)
      .$dynamic(),
  )
    .where(discoverablePublicationPredicate())
    .orderBy(desc(Publications.publishedAt), desc(Publications.id))
    .limit(input.limit);

export const pickFirstByKey = <T>(rows: readonly T[], key: (row: T) => string | null, limit: number): T[] => {
  const seen = new Set<string>();
  const picked: T[] = [];
  for (const row of rows) {
    const value = key(row);
    if (value === null || seen.has(value)) continue;
    seen.add(value);
    picked.push(row);
    if (picked.length === limit) break;
  }
  return picked;
};

export const buildDiscoveryTagsQuery = (executor: Executor, input: { name?: string; limit?: number }) => {
  const query = discoverablePublicationScope(executor.select({ name: PublicationTags.name, count: count() }).from(Publications).$dynamic())
    .innerJoin(PublicationTags, eq(PublicationTags.publicationId, Publications.id))
    .where(and(discoverablePublicationPredicate(), input.name === undefined ? undefined : eq(PublicationTags.name, input.name)))
    .groupBy(PublicationTags.name)
    .orderBy(desc(count()), asc(PublicationTags.name));

  return input.limit === undefined ? query : query.limit(input.limit);
};

export const buildDiscoverablePublicationsByIdsQuery = (executor: Executor, input: { publicationIds: string[] }) =>
  discoverablePublicationsBase(executor).where(and(inArray(Publications.id, input.publicationIds), discoverablePublicationPredicate()));

export const buildDiscoverableSitesByIdsQuery = (executor: Executor, input: { siteIds: string[] }) =>
  executor
    .select()
    .from(Sites)
    .where(
      and(
        inArray(Sites.id, input.siteIds),
        eq(Sites.state, SiteState.ACTIVE),
        eq(Sites.allowIndexing, true),
        eq(Sites.allowDiscovery, true),
      ),
    );

export const buildDiscoveryTagCountsQuery = (executor: Executor, input: { names: string[] }) =>
  discoverablePublicationScope(executor.select({ name: PublicationTags.name, count: count() }).from(Publications).$dynamic())
    .innerJoin(PublicationTags, eq(PublicationTags.publicationId, Publications.id))
    .where(and(discoverablePublicationPredicate(), inArray(PublicationTags.name, input.names)))
    .groupBy(PublicationTags.name);
