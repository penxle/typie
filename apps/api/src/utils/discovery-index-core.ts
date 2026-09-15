import { EntityState, PublicationState, SiteState } from '@typie/lib/enums';
import { asc, eq, inArray } from 'drizzle-orm';
import { Documents, Entities, Publications, PublicationTags, Sites } from '#/db/schemas/tables.ts';
import { decompose } from './text.ts';
import type { Dayjs } from 'dayjs';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

export type PublicationIndexRow = {
  id: string;
  siteId: string;
  state: PublicationState;
  publishedAt: Dayjs | null;
  entityState: EntityState;
  siteState: SiteState;
  allowDiscovery: boolean;
  password: string | null;
};

export type PublicationIndexDocument = {
  site_id: string;
  discoverable: boolean;
  title: string | null;
  title_decomposed: string | null;
  subtitle: string | null;
  subtitle_decomposed: string | null;
  text: string;
  tags: string[];
  tags_text: string;
  published_at: Dayjs | null;
};

export type SiteIndexRow = {
  id: string;
  name: string;
  description: string | null;
  state: SiteState;
  allowDiscovery: boolean;
};

export type SiteIndexDocument = {
  name: string;
  name_decomposed: string | null;
  description: string | null;
};

export type TagIndexDocument = {
  name: string;
  name_decomposed: string | null;
  count: number;
};

export const buildPublicationIndexRowsQuery = (executor: Executor, input: { publicationIds: string[] }) =>
  executor
    .select({
      id: Publications.id,
      siteId: Publications.siteId,
      state: Publications.state,
      publishedAt: Publications.publishedAt,
      entityState: Entities.state,
      siteState: Sites.state,
      allowDiscovery: Sites.allowDiscovery,
      password: Documents.password,
    })
    .from(Publications)
    .innerJoin(Documents, eq(Publications.documentId, Documents.id))
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .innerJoin(Sites, eq(Publications.siteId, Sites.id))
    .where(inArray(Publications.id, input.publicationIds));

export const buildPublicationTagNamesQuery = (executor: Executor, input: { publicationIds: string[] }) =>
  executor
    .select({ publicationId: PublicationTags.publicationId, name: PublicationTags.name, order: PublicationTags.order })
    .from(PublicationTags)
    .where(inArray(PublicationTags.publicationId, input.publicationIds))
    .orderBy(asc(PublicationTags.publicationId), asc(PublicationTags.order));

export const buildSiteIndexRowsQuery = (executor: Executor, input: { siteIds: string[] }) =>
  executor
    .select({
      id: Sites.id,
      name: Sites.name,
      description: Sites.description,
      state: Sites.state,
      allowDiscovery: Sites.allowDiscovery,
    })
    .from(Sites)
    .where(inArray(Sites.id, input.siteIds));

export const buildSiteTagNamesQuery = (executor: Executor, input: { siteIds: string[] }) =>
  executor.selectDistinct({ name: PublicationTags.name }).from(PublicationTags).where(inArray(PublicationTags.siteId, input.siteIds));

export const toPublicationIndexDocument = (input: {
  row: PublicationIndexRow;
  version: { title: string | null; subtitle: string | null; text: string } | undefined;
  tagNames: string[];
}): PublicationIndexDocument | null => {
  const { row, version } = input;
  if (!version) return null;
  if (row.state !== PublicationState.PUBLISHED) return null;
  if (row.entityState !== EntityState.ACTIVE) return null;
  if (row.siteState !== SiteState.ACTIVE) return null;
  if (row.password !== null) return null;

  return {
    site_id: row.siteId,
    discoverable: row.allowDiscovery,
    title: version.title,
    title_decomposed: decompose(version.title),
    subtitle: version.subtitle,
    subtitle_decomposed: decompose(version.subtitle),
    text: version.text,
    tags: input.tagNames,
    tags_text: input.tagNames.join(' '),
    published_at: row.publishedAt,
  };
};

export const toSiteIndexDocument = (row: SiteIndexRow): SiteIndexDocument | null => {
  if (row.state !== SiteState.ACTIVE) return null;
  if (!row.allowDiscovery) return null;

  return { name: row.name, name_decomposed: decompose(row.name), description: row.description };
};

export const toTagIndexDocument = (input: { name: string; count: number }): TagIndexDocument => ({
  name: input.name,
  name_decomposed: decompose(input.name),
  count: input.count,
});

export const collectTagNames = (...lists: (readonly string[] | undefined)[]): string[] => [...new Set(lists.flatMap((list) => list ?? []))];
