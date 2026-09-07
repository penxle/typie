import { EntityState, PublicationState, SiteState, SpaceState } from '@typie/lib/enums';
import { and, asc, count, desc, eq, getTableColumns, gt, inArray, isNotNull, isNull, lt, sql } from 'drizzle-orm';
import { Collections, Documents, Entities, Publications, PublicationTags, Sites, Spaces } from '#/db/schemas/tables.ts';
import type { PgSelect } from 'drizzle-orm/pg-core';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

export const publishedPublicationScope = <T extends PgSelect>(qb: T) =>
  qb.innerJoin(Documents, eq(Publications.documentId, Documents.id)).innerJoin(Entities, eq(Documents.entityId, Entities.id));

export const publishedPublicationPredicate = () =>
  and(eq(Publications.state, PublicationState.PUBLISHED), eq(Entities.state, EntityState.ACTIVE));

export const publishedPublicationsBase = (executor: Executor) =>
  publishedPublicationScope(executor.select(getTableColumns(Publications)).from(Publications).$dynamic());

export const SPACE_PAGE_SIZE = 20;
export const SPACE_PAGE_SIZE_MAX = 50;

export const clampPageSize = (first: number | null | undefined) => Math.min(Math.max(first ?? SPACE_PAGE_SIZE, 1), SPACE_PAGE_SIZE_MAX);

export const toPublicationsPage = <T>(rows: T[], limit: number) => ({
  publications: rows.slice(0, limit),
  hasMore: rows.length > limit,
});

export const deriveExcerpt = (text: string, override: string | null): string => {
  if (override !== null && override.trim().length > 0) return override;
  const collapsed = text.replaceAll(/\s+/g, ' ').trim();
  return collapsed.length <= 200 ? collapsed : collapsed.slice(0, 200) + '...';
};

export const buildSpaceBySlugQuery = (executor: Executor, input: { slug: string }) =>
  executor
    .select(getTableColumns(Spaces))
    .from(Spaces)
    .innerJoin(Sites, eq(Spaces.siteId, Sites.id))
    .where(and(eq(Spaces.slug, input.slug), eq(Spaces.state, SpaceState.ACTIVE), eq(Sites.state, SiteState.ACTIVE)));

type PublishedQueryInput = {
  spaceId: string;
  collectionId?: string;
  tagName?: string;
  publicationId?: string;
  excludePinned?: boolean;
  after: string | null;
  limit: number;
};

export const buildPublishedPublicationsQuery = (executor: Executor, input: PublishedQueryInput) => {
  const base = publishedPublicationsBase(executor);

  const joined = input.tagName
    ? base.innerJoin(PublicationTags, and(eq(PublicationTags.publicationId, Publications.id), eq(PublicationTags.name, input.tagName)))
    : base;

  return joined
    .where(
      and(
        eq(Publications.spaceId, input.spaceId),
        publishedPublicationPredicate(),
        input.collectionId ? eq(Publications.collectionId, input.collectionId) : undefined,
        input.publicationId ? eq(Publications.id, input.publicationId) : undefined,
        input.excludePinned ? isNull(Publications.pinnedOrder) : undefined,
        input.after
          ? sql`(${Publications.publishedAt}, ${Publications.id}) < (select ${Publications.publishedAt}, ${Publications.id} from ${Publications} where ${Publications.id} = ${input.after})`
          : undefined,
      ),
    )
    .orderBy(...(input.collectionId ? [asc(Publications.collectionOrder)] : [desc(Publications.publishedAt), desc(Publications.id)]))
    .limit(input.limit);
};

export const buildCollectionNeighborQuery = (
  executor: Executor,
  input: { collectionId: string; collectionOrder: string; direction: 'prev' | 'next' },
) =>
  publishedPublicationsBase(executor)
    .where(
      and(
        eq(Publications.collectionId, input.collectionId),
        publishedPublicationPredicate(),
        input.direction === 'prev'
          ? lt(Publications.collectionOrder, input.collectionOrder)
          : gt(Publications.collectionOrder, input.collectionOrder),
      ),
    )
    .orderBy(input.direction === 'prev' ? desc(Publications.collectionOrder) : asc(Publications.collectionOrder))
    .limit(1);

const buildSpaceTagsScope = (executor: Executor, input: { spaceId: string; name?: string }) =>
  publishedPublicationScope(executor.select({ name: PublicationTags.name, count: count() }).from(Publications).$dynamic())
    .innerJoin(PublicationTags, eq(PublicationTags.publicationId, Publications.id))
    .where(
      and(
        eq(Publications.spaceId, input.spaceId),
        publishedPublicationPredicate(),
        input.name === undefined ? undefined : eq(PublicationTags.name, input.name),
      ),
    )
    .groupBy(PublicationTags.name)
    .orderBy(desc(count()), asc(PublicationTags.name));

export const buildSpaceTagsQuery = (executor: Executor, input: { spaceId: string }) => buildSpaceTagsScope(executor, input);

export const buildSpaceTagQuery = (executor: Executor, input: { spaceId: string; name: string }) => buildSpaceTagsScope(executor, input);

export const buildPublishedPublicationCountQuery = (executor: Executor, input: { spaceId: string }) =>
  publishedPublicationScope(executor.select({ count: count() }).from(Publications).$dynamic()).where(
    and(eq(Publications.spaceId, input.spaceId), publishedPublicationPredicate()),
  );

export const buildPublishedCollectionIdsQuery = (executor: Executor, input: { spaceId: string }) =>
  publishedPublicationScope(executor.selectDistinct({ collectionId: Publications.collectionId }).from(Publications).$dynamic()).where(
    and(eq(Publications.spaceId, input.spaceId), publishedPublicationPredicate(), isNotNull(Publications.collectionId)),
  );

export const buildPublishedPublicationByIdQuery = (executor: Executor, input: { publicationId: string }) =>
  publishedPublicationsBase(executor).where(and(eq(Publications.id, input.publicationId), publishedPublicationPredicate()));

export const buildCollectionsByIdsQuery = (executor: Executor, input: { collectionIds: string[] }) =>
  executor.select().from(Collections).where(inArray(Collections.id, input.collectionIds)).orderBy(asc(Collections.createdAt));

export const buildSitemapPaths = (input: { publicationIds: string[]; collectionIds: string[]; tagNames: string[] }) => [
  '/',
  ...input.publicationIds.map((id) => `/p/${id}`),
  ...input.collectionIds.map((id) => `/s/${id}`),
  ...input.tagNames.map((name) => `/t/${encodeURIComponent(name)}`),
];
