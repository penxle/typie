import {
  DocumentType,
  EntityState,
  EntityType,
  EntityVisibility,
  PublicationState,
  SiteDateDisplay,
  SiteState,
  SpaceDateDisplay,
  SpaceState,
} from '@typie/lib/enums';
import dayjs from 'dayjs';
import { sql } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { Documents, DocumentStates, Entities, Sites } from '#/db/schemas/tables.ts';
import type { Publications, Spaces } from '#/db/schemas/tables.ts';

export type LegacyPublicDocumentRow = {
  document_id: string;
  entity_id: string;
  site_id: string;
  site_slug: string;
  site_name: string;
  site_logo_id: string;
  site_date_display: SiteDateDisplay;
  document_type: DocumentType;
  character_count: number;
  chain_public: boolean;
  title: string | null;
  subtitle: string | null;
  thumbnail_id: string | null;
  created_at: string;
  updated_at: string;
};

export type LegacyPublicDocument = {
  documentId: string;
  entityId: string;
  siteId: string;
  siteSlug: string;
  siteName: string;
  siteLogoId: string;
  siteDateDisplay: SiteDateDisplay;
  documentType: DocumentType;
  characterCount: number;
  chainPublic: boolean;
  title: string | null;
  subtitle: string | null;
  thumbnailId: string | null;
  createdAt: string;
  updatedAt: string;
};

export type LegacySite = Pick<LegacyPublicDocument, 'siteId' | 'siteSlug' | 'siteName' | 'siteLogoId' | 'siteDateDisplay'>;

export type LegacyClassification = 'template' | 'empty' | 'ancestor' | 'slug_taken' | 'already_published' | 'migrated';

export type LegacySpaceSlot = { kind: 'create' } | { kind: 'reuse'; spaceId: string } | { kind: 'taken' };

export const buildLegacyPublicDocumentsQuery = () => sql`
  with recursive chain as (
    select e.id as doc_entity_id, e.parent_id, true as ok
    from ${Entities} e
    join ${Sites} s on s.id = e.site_id
    where e.state = ${EntityState.ACTIVE} and s.state = ${SiteState.ACTIVE} and e.type = ${EntityType.DOCUMENT} and e.visibility = ${EntityVisibility.PUBLIC}
    union all
    select c.doc_entity_id, p.parent_id, (c.ok and p.visibility = ${EntityVisibility.PUBLIC} and p.state = ${EntityState.ACTIVE})
    from chain c
    join ${Entities} p on p.id = c.parent_id
  ),
  chains as (
    select doc_entity_id, bool_and(ok) as chain_public from chain group by doc_entity_id
  )
  select
    d.id as document_id, e.id as entity_id, s.id as site_id, s.slug as site_slug, s.name as site_name, s.logo_id as site_logo_id,
    s.date_display as site_date_display, d.type as document_type, coalesce(ds.character_count, 0) as character_count,
    ch.chain_public, d.title, d.subtitle, d.thumbnail_id, d.created_at, d.updated_at
  from chains ch
  join ${Entities} e on e.id = ch.doc_entity_id
  join ${Sites} s on s.id = e.site_id
  join ${Documents} d on d.entity_id = e.id
  left join ${DocumentStates} ds on ds.document_id = d.id
  order by s.id, d.id
`;

export const toLegacyPublicDocument = (row: LegacyPublicDocumentRow): LegacyPublicDocument => ({
  documentId: row.document_id,
  entityId: row.entity_id,
  siteId: row.site_id,
  siteSlug: row.site_slug,
  siteName: row.site_name,
  siteLogoId: row.site_logo_id,
  siteDateDisplay: row.site_date_display,
  documentType: row.document_type,
  characterCount: Number(row.character_count),
  chainPublic: row.chain_public,
  title: row.title,
  subtitle: row.subtitle,
  thumbnailId: row.thumbnail_id,
  createdAt: row.created_at,
  updatedAt: row.updated_at,
});

export const classifyLegacyDocument = (
  document: Pick<LegacyPublicDocument, 'documentId' | 'siteId' | 'documentType' | 'characterCount' | 'chainPublic'>,
  context: { slugTakenSiteIds: ReadonlySet<string>; publishedDocumentIds: ReadonlySet<string> },
): LegacyClassification => {
  if (document.documentType === DocumentType.TEMPLATE) return 'template';
  if (document.characterCount === 0) return 'empty';
  if (!document.chainPublic) return 'ancestor';
  if (context.slugTakenSiteIds.has(document.siteId)) return 'slug_taken';
  if (context.publishedDocumentIds.has(document.documentId)) return 'already_published';
  return 'migrated';
};

export const isDemotedClassification = (classification: LegacyClassification) =>
  classification === 'template' || classification === 'empty' || classification === 'ancestor';

export const mapSiteDateDisplay = (value: SiteDateDisplay): SpaceDateDisplay =>
  match(value)
    .with(SiteDateDisplay.NONE, () => SpaceDateDisplay.NONE)
    .with(SiteDateDisplay.UPDATED_AT, () => SpaceDateDisplay.UPDATED_AT)
    .with(SiteDateDisplay.CREATED_AT, () => SpaceDateDisplay.PUBLISHED_AT)
    .exhaustive();

export const resolveLegacySpaceSlot = (
  site: { siteId: string },
  existing: { id: string; siteId: string; state: SpaceState } | null,
): LegacySpaceSlot => {
  if (!existing) return { kind: 'create' };
  if (existing.siteId === site.siteId && existing.state === SpaceState.ACTIVE) return { kind: 'reuse', spaceId: existing.id };
  return { kind: 'taken' };
};

export const buildLegacySpaceValues = (site: LegacySite): typeof Spaces.$inferInsert => ({
  siteId: site.siteId,
  slug: site.siteSlug,
  name: site.siteName,
  logoId: site.siteLogoId,
  allowIndexing: true,
  allowDiscovery: true,
  dateDisplay: mapSiteDateDisplay(site.siteDateDisplay),
});

export const buildLegacyPublicationValues = (input: {
  documentId: string;
  spaceId: string;
  permalink: string;
  createdAt: string;
  updatedAt: string;
}): typeof Publications.$inferInsert => ({
  documentId: input.documentId,
  spaceId: input.spaceId,
  permalink: input.permalink,
  state: PublicationState.PUBLISHED,
  publishedAt: dayjs(input.createdAt),
  updatedAt: dayjs(input.updatedAt),
});
