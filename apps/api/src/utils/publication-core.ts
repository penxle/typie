import { EntityState, EntityVisibility, PublicationState, SiteState, SpaceState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, asc, desc, eq, inArray, lte } from 'drizzle-orm';
import { Documents, Entities, Publications, PublicationVersions, Sites, Spaces } from '#/db/schemas/tables.ts';
import type { Dayjs } from 'dayjs';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

export type VersionSnapshot = {
  title: string | null;
  subtitle: string | null;
  heads: Uint8Array;
  thumbnailId: string | null;
  excerpt: string | null;
};

const bytesEqual = (a: Uint8Array, b: Uint8Array) => a.length === b.length && a.every((v, i) => v === b[i]);

export const normalizeTags = (tags: readonly string[]): string[] => {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of tags) {
    const name = raw.normalize('NFC').trim();
    if (name.length === 0 || seen.has(name)) continue;
    seen.add(name);
    out.push(name);
  }
  return out;
};

export const validateScheduledAt = (scheduledAt: Dayjs | null | undefined, now: Dayjs): Dayjs | null => {
  if (!scheduledAt) return null;
  if (!scheduledAt.isAfter(now)) {
    throw new TypieError({ code: 'publication_scheduled_in_past', status: 400 });
  }
  return scheduledAt;
};

export const pickPublicationVersion = (
  latest: (VersionSnapshot & { version: number }) | null,
  snap: VersionSnapshot,
): { reuse: boolean; version: number } => {
  if (
    latest &&
    bytesEqual(latest.heads, snap.heads) &&
    latest.title === snap.title &&
    latest.subtitle === snap.subtitle &&
    latest.thumbnailId === snap.thumbnailId &&
    latest.excerpt === snap.excerpt
  ) {
    return { reuse: true, version: latest.version };
  }
  return { reuse: false, version: (latest?.version ?? 0) + 1 };
};

export const resolvePublishTransition = (input: {
  currentState: PublicationState | null;
  scheduledAt: Dayjs | null;
  now: Dayjs;
}): { state: PublicationState; publishedAt: Dayjs | null; scheduledAt: Dayjs | null } => {
  if (input.currentState === PublicationState.PUBLISHED) {
    throw new TypieError({ code: 'publication_already_published', status: 409 });
  }
  if (input.scheduledAt) {
    return { state: PublicationState.SCHEDULED, publishedAt: null, scheduledAt: input.scheduledAt };
  }
  return { state: PublicationState.PUBLISHED, publishedAt: input.now, scheduledAt: null };
};

export const hasUnpublishedChanges = (
  version: { heads: Uint8Array; title: string | null; subtitle: string | null },
  current: { heads: Uint8Array | null; title: string | null; subtitle: string | null },
): boolean => {
  if (current.heads === null) return true;
  return !bytesEqual(version.heads, current.heads) || version.title !== current.title || version.subtitle !== current.subtitle;
};

export const buildLatestVersionMetadataQuery = (executor: Executor, input: { publicationIds: string[] }) =>
  executor
    .selectDistinctOn([PublicationVersions.publicationId], {
      id: PublicationVersions.id,
      publicationId: PublicationVersions.publicationId,
      version: PublicationVersions.version,
      title: PublicationVersions.title,
      subtitle: PublicationVersions.subtitle,
      text: PublicationVersions.text,
      characterCount: PublicationVersions.characterCount,
      heads: PublicationVersions.heads,
      thumbnailId: PublicationVersions.thumbnailId,
      excerpt: PublicationVersions.excerpt,
      assetIds: PublicationVersions.assetIds,
      createdAt: PublicationVersions.createdAt,
    })
    .from(PublicationVersions)
    .where(inArray(PublicationVersions.publicationId, input.publicationIds))
    .orderBy(asc(PublicationVersions.publicationId), desc(PublicationVersions.version));

export const buildLatestVersionGraphsQuery = (executor: Executor, input: { publicationIds: string[] }) =>
  executor
    .selectDistinctOn([PublicationVersions.publicationId], {
      publicationId: PublicationVersions.publicationId,
      version: PublicationVersions.version,
      graph: PublicationVersions.graph,
    })
    .from(PublicationVersions)
    .where(inArray(PublicationVersions.publicationId, input.publicationIds))
    .orderBy(asc(PublicationVersions.publicationId), desc(PublicationVersions.version));

export const buildPublicationsBySpaceQuery = (executor: Executor, input: { spaceIds: string[]; states: PublicationState[] }) =>
  executor
    .select()
    .from(Publications)
    .where(and(inArray(Publications.spaceId, input.spaceIds), inArray(Publications.state, input.states)))
    .orderBy(desc(Publications.publishedAt), desc(Publications.id));

export const buildPublishedDocumentIdsQuery = (executor: Executor, input: { documentIds: string[] }) =>
  executor
    .select({ documentId: Publications.documentId })
    .from(Publications)
    .where(and(inArray(Publications.documentId, input.documentIds), eq(Publications.state, PublicationState.PUBLISHED)));

export const buildPublishingEntityIdsBySiteQuery = (executor: Executor, input: { siteIds: string[] }) =>
  executor
    .select({ entityId: Documents.entityId })
    .from(Publications)
    .innerJoin(Documents, eq(Publications.documentId, Documents.id))
    .innerJoin(Spaces, eq(Publications.spaceId, Spaces.id))
    .where(
      and(inArray(Spaces.siteId, input.siteIds), inArray(Publications.state, [PublicationState.PUBLISHED, PublicationState.SCHEDULED])),
    );

export const resolvePublishedVisibilityBlock = (requested: EntityVisibility): string | null => {
  if (requested === EntityVisibility.UNLISTED) return 'publication_link_share_blocked';
  if (requested === EntityVisibility.PRIVATE) return 'publication_unpublish_required';
  return null;
};

export const resolveVisibilityRequestBlock = (requested: EntityVisibility): string | null =>
  requested === EntityVisibility.PUBLIC ? 'visibility_public_reserved' : null;

export const buildDuePublicationsQuery = (executor: Executor, input: { now: Dayjs }) =>
  executor
    .select({
      id: Publications.id,
      documentId: Publications.documentId,
      spaceId: Publications.spaceId,
      scheduledAt: Publications.scheduledAt,
      publishedAt: Publications.publishedAt,
    })
    .from(Publications)
    .innerJoin(Documents, eq(Publications.documentId, Documents.id))
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .innerJoin(Sites, eq(Entities.siteId, Sites.id))
    .innerJoin(Spaces, eq(Publications.spaceId, Spaces.id))
    .where(
      and(
        eq(Publications.state, PublicationState.SCHEDULED),
        lte(Publications.scheduledAt, input.now),
        eq(Entities.state, EntityState.ACTIVE),
        eq(Sites.state, SiteState.ACTIVE),
        eq(Spaces.state, SpaceState.ACTIVE),
      ),
    )
    .for('update', { of: [Publications], skipLocked: true });
