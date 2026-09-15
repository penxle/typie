import { EntityState, PublicationState, SiteState } from '@typie/lib/enums';
import { and, asc, count, desc, eq, exists, getTableColumns, inArray, isNotNull, isNull, sql } from 'drizzle-orm';
import { DocumentReactions, Documents, Entities, Publications, PublicationTags, Sites } from '#/db/schemas/tables.ts';
import type { PgSelect } from 'drizzle-orm/pg-core';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

export const publishedPublicationScope = <T extends PgSelect>(qb: T) =>
  qb.innerJoin(Documents, eq(Publications.documentId, Documents.id)).innerJoin(Entities, eq(Documents.entityId, Entities.id));

export const publishedPublicationPredicate = () =>
  and(eq(Publications.state, PublicationState.PUBLISHED), eq(Entities.state, EntityState.ACTIVE));

export const publishedPublicationsBase = (executor: Executor) =>
  publishedPublicationScope(executor.select(getTableColumns(Publications)).from(Publications).$dynamic());

export const SITE_PAGE_SIZE = 20;
export const SITE_PAGE_SIZE_MAX = 50;

export const clampPageSize = (first: number | null | undefined) => Math.min(Math.max(first ?? SITE_PAGE_SIZE, 1), SITE_PAGE_SIZE_MAX);

export const toPublicationsPage = <T>(rows: T[], limit: number) => ({
  publications: rows.slice(0, limit),
  hasMore: rows.length > limit,
});

export const deriveExcerpt = (text: string, override: string | null): string => {
  if (override !== null && override.trim().length > 0) return override;
  const collapsed = text.replaceAll(/\s+/g, ' ').trim();
  return collapsed.length <= 200 ? collapsed : collapsed.slice(0, 200) + '...';
};

export const buildSiteBySlugQuery = (executor: Executor, input: { slug: string }) =>
  executor
    .select()
    .from(Sites)
    .where(and(eq(Sites.slug, input.slug), eq(Sites.state, SiteState.ACTIVE)));

const publishedInSite = (executor: Executor) =>
  executor
    .select({ id: Publications.id })
    .from(Publications)
    .where(and(eq(Publications.siteId, Sites.id), eq(Publications.state, PublicationState.PUBLISHED)));

export const buildIndexableSiteSlugsQuery = (executor: Executor) =>
  executor
    .select({ slug: Sites.slug })
    .from(Sites)
    .where(and(eq(Sites.state, SiteState.ACTIVE), eq(Sites.allowIndexing, true), exists(publishedInSite(executor))))
    .orderBy(Sites.createdAt);

type PublishedQueryInput = {
  siteId: string;
  tagName?: string;
  number?: string;
  entityIds?: string[];
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
        eq(Publications.siteId, input.siteId),
        publishedPublicationPredicate(),
        input.number === undefined ? undefined : eq(Entities.number, input.number),
        input.entityIds === undefined ? undefined : inArray(Entities.id, input.entityIds),
        input.excludePinned ? isNull(Publications.pinnedOrder) : undefined,
        input.after
          ? sql`(${Publications.publishedAt}, ${Publications.id}) < (select ${Publications.publishedAt}, ${Publications.id} from ${Publications} where ${Publications.id} = ${input.after})`
          : undefined,
      ),
    )
    .orderBy(desc(Publications.publishedAt), desc(Publications.id))
    .limit(input.limit);
};

export const buildPublishedPublicationNumbersQuery = (executor: Executor, input: { siteId: string; limit: number }) =>
  publishedPublicationScope(executor.select({ number: Entities.number }).from(Publications).$dynamic())
    .where(and(eq(Publications.siteId, input.siteId), publishedPublicationPredicate()))
    .orderBy(desc(Publications.publishedAt), desc(Publications.id))
    .limit(input.limit);

export const buildPublishedPublicationsByEntityIdsQuery = (executor: Executor, input: { entityIds: string[] }) =>
  publishedPublicationScope(
    executor
      .select({ ...getTableColumns(Publications), entityId: Entities.id })
      .from(Publications)
      .$dynamic(),
  ).where(and(inArray(Entities.id, input.entityIds), publishedPublicationPredicate()));

const buildSiteTagsScope = (executor: Executor, input: { siteId: string; name?: string }) =>
  publishedPublicationScope(executor.select({ name: PublicationTags.name, count: count() }).from(Publications).$dynamic())
    .innerJoin(PublicationTags, eq(PublicationTags.publicationId, Publications.id))
    .where(
      and(
        eq(Publications.siteId, input.siteId),
        publishedPublicationPredicate(),
        input.name === undefined ? undefined : eq(PublicationTags.name, input.name),
      ),
    )
    .groupBy(PublicationTags.name)
    .orderBy(desc(count()), asc(PublicationTags.name));

export const buildSiteTagsQuery = (executor: Executor, input: { siteId: string }) => buildSiteTagsScope(executor, input);

export const buildSiteTagQuery = (executor: Executor, input: { siteId: string; name: string }) => buildSiteTagsScope(executor, input);

export const buildPublishedPublicationCountQuery = (executor: Executor, input: { siteId: string }) =>
  publishedPublicationScope(executor.select({ count: count() }).from(Publications).$dynamic()).where(
    and(eq(Publications.siteId, input.siteId), publishedPublicationPredicate()),
  );

export const buildPinnedPublicationsQuery = (executor: Executor, input: { siteIds: string[] }) =>
  publishedPublicationsBase(executor)
    .where(and(inArray(Publications.siteId, input.siteIds), publishedPublicationPredicate(), isNotNull(Publications.pinnedOrder)))
    .orderBy(asc(Publications.siteId), asc(Publications.pinnedOrder));

export const buildReactionCountsQuery = (executor: Executor, input: { documentIds: string[] }) =>
  executor
    .select({ documentId: DocumentReactions.documentId, count: count() })
    .from(DocumentReactions)
    .where(inArray(DocumentReactions.documentId, input.documentIds))
    .groupBy(DocumentReactions.documentId);

export const buildPublishedPublicationByIdQuery = (executor: Executor, input: { publicationId: string }) =>
  publishedPublicationsBase(executor).where(and(eq(Publications.id, input.publicationId), publishedPublicationPredicate()));

export const buildPublishedPublicationByNumberQuery = (executor: Executor, input: { number: string }) =>
  publishedPublicationsBase(executor).where(and(eq(Entities.number, input.number), publishedPublicationPredicate()));

export const buildSitemapPaths = (input: { publicationNumbers: string[]; folderNumbers: string[]; tagNames: string[] }) => [
  '/',
  ...input.folderNumbers.map((number) => `/f/${number}`),
  ...input.publicationNumbers.map((number) => `/p/${number}`),
  ...input.tagNames.map((name) => `/t/${encodeURIComponent(name)}`),
];

export const PIN_LIMIT = 3;
export const canPinMore = (currentCount: number) => currentCount < PIN_LIMIT;
