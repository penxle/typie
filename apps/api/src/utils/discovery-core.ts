import { EntityState, PublicationState, SiteState, SpaceState } from '@typie/lib/enums';
import { and, asc, count, desc, eq, getTableColumns, inArray, isNull, sql } from 'drizzle-orm';
import { Documents, Entities, Publications, PublicationTags, Sites, Spaces } from '#/db/schemas/tables.ts';
import type { PgSelect } from 'drizzle-orm/pg-core';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

export const DISCOVERY_TAG_LIMIT = 20;

export const discoverablePublicationScope = <T extends PgSelect>(qb: T) =>
  qb
    .innerJoin(Documents, eq(Publications.documentId, Documents.id))
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .innerJoin(Spaces, eq(Publications.spaceId, Spaces.id))
    .innerJoin(Sites, eq(Spaces.siteId, Sites.id));

export const discoverablePublicationPredicate = () =>
  and(
    eq(Publications.state, PublicationState.PUBLISHED),
    eq(Entities.state, EntityState.ACTIVE),
    eq(Spaces.state, SpaceState.ACTIVE),
    eq(Sites.state, SiteState.ACTIVE),
    eq(Spaces.allowIndexing, true),
    eq(Spaces.allowDiscovery, true),
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

export const buildDiscoverableSpacesByIdsQuery = (executor: Executor, input: { spaceIds: string[] }) =>
  executor
    .select(getTableColumns(Spaces))
    .from(Spaces)
    .innerJoin(Sites, eq(Spaces.siteId, Sites.id))
    .where(
      and(
        inArray(Spaces.id, input.spaceIds),
        eq(Spaces.state, SpaceState.ACTIVE),
        eq(Sites.state, SiteState.ACTIVE),
        eq(Spaces.allowIndexing, true),
        eq(Spaces.allowDiscovery, true),
      ),
    );

export const buildDiscoveryTagCountsQuery = (executor: Executor, input: { names: string[] }) =>
  discoverablePublicationScope(executor.select({ name: PublicationTags.name, count: count() }).from(Publications).$dynamic())
    .innerJoin(PublicationTags, eq(PublicationTags.publicationId, Publications.id))
    .where(and(discoverablePublicationPredicate(), inArray(PublicationTags.name, input.names)))
    .groupBy(PublicationTags.name);
