import { EntityVisibility, PublicationState } from '@typie/lib/enums';
import { and, eq, inArray } from 'drizzle-orm';
import {
  Collections,
  db,
  Documents,
  Entities,
  first,
  firstOrThrow,
  Publications,
  PublicationTags,
  PublicationVersions,
  Spaces,
} from '#/db/index.ts';
import { readMergedGraph } from './changeset.ts';
import {
  buildLegacyPublicationValues,
  buildLegacyPublicDocumentsQuery,
  buildLegacySpaceValues,
  resolveLegacySpaceSlot,
  toLegacyPublicDocument,
} from './legacy-publication-core.ts';
import { insertPublicationVersion } from './publication.ts';
import { generateNumericPermalink } from './publication-core.ts';
import { buildPublicationSnapshot } from './publication-snapshot.ts';
import type { LegacyPublicDocument, LegacyPublicDocumentRow, LegacySite } from './legacy-publication-core.ts';

const CHUNK = 500;

export type LegacyMigrationOutcome =
  | { outcome: 'migrated'; publicationId: string; versionId: string; permalink: string; updatedAt: string; graphBytes: number }
  | { outcome: 'planned'; graphBytes: number }
  | { outcome: 'empty' }
  | { outcome: 'already_published' }
  | { outcome: 'failed'; message: string };

export type LegacySpaceResolution = { kind: 'created' | 'reused' | 'planned' | 'taken'; spaceId: string | null };

export type LegacyMigrationManifest = {
  spaces: { id: string; siteId: string; slug: string }[];
  publications: { id: string; versionId: string; documentId: string; spaceId: string; permalink: string; updatedAt: string }[];
  demoted: { entityId: string; from: 'PUBLIC' }[];
};

const chunks = <T>(items: readonly T[]): T[][] => {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += CHUNK) out.push(items.slice(i, i + CHUNK));
  return out;
};

export const loadLegacyPublicDocuments = async (): Promise<LegacyPublicDocument[]> => {
  const rows = await db.execute<LegacyPublicDocumentRow>(buildLegacyPublicDocumentsQuery());
  return rows.map(toLegacyPublicDocument);
};

export const loadPublishedDocumentIds = async (documentIds: string[]): Promise<Set<string>> => {
  const published = new Set<string>();
  for (const chunk of chunks(documentIds)) {
    const rows = await db.select({ documentId: Publications.documentId }).from(Publications).where(inArray(Publications.documentId, chunk));
    for (const row of rows) published.add(row.documentId);
  }
  return published;
};

export const ensureLegacySpaces = async (
  sites: LegacySite[],
  options: { dryRun: boolean },
): Promise<Map<string, LegacySpaceResolution>> => {
  const resolutions = new Map<string, LegacySpaceResolution>();
  const existingBySlug = new Map<string, { id: string; siteId: string; state: (typeof Spaces.$inferSelect)['state'] }>();
  for (const chunk of chunks(sites.map((site) => site.siteSlug))) {
    const rows = await db
      .select({ id: Spaces.id, slug: Spaces.slug, siteId: Spaces.siteId, state: Spaces.state })
      .from(Spaces)
      .where(inArray(Spaces.slug, chunk));
    for (const row of rows) existingBySlug.set(row.slug, row);
  }

  for (const site of sites) {
    const slot = resolveLegacySpaceSlot(site, existingBySlug.get(site.siteSlug) ?? null);
    if (slot.kind === 'taken') {
      resolutions.set(site.siteId, { kind: 'taken', spaceId: null });
    } else if (slot.kind === 'reuse') {
      resolutions.set(site.siteId, { kind: 'reused', spaceId: slot.spaceId });
    } else if (options.dryRun) {
      resolutions.set(site.siteId, { kind: 'planned', spaceId: null });
    } else {
      const space = await db.insert(Spaces).values(buildLegacySpaceValues(site)).returning({ id: Spaces.id }).then(firstOrThrow);
      resolutions.set(site.siteId, { kind: 'created', spaceId: space.id });
    }
  }
  return resolutions;
};

export const migrateLegacyDocument = async (
  document: LegacyPublicDocument,
  options: { dryRun: true } | { dryRun: false; spaceId: string },
): Promise<LegacyMigrationOutcome> => {
  try {
    const graph = await readMergedGraph(document.documentId);
    if (graph.length === 0) return { outcome: 'empty' };
    const snapshot = await buildPublicationSnapshot(graph);

    if (options.dryRun) {
      const existing = await db
        .select({ id: Publications.id })
        .from(Publications)
        .where(eq(Publications.documentId, document.documentId))
        .then(first);
      return existing ? { outcome: 'already_published' } : { outcome: 'planned', graphBytes: snapshot.graph.length };
    }

    return await db.transaction(async (tx) => {
      await tx.select({ id: Documents.id }).from(Documents).where(eq(Documents.id, document.documentId)).for('update');
      const existing = await tx
        .select({ id: Publications.id })
        .from(Publications)
        .where(eq(Publications.documentId, document.documentId))
        .then(first);
      if (existing) return { outcome: 'already_published' as const };

      const publication = await tx
        .insert(Publications)
        .values(
          buildLegacyPublicationValues({
            documentId: document.documentId,
            spaceId: options.spaceId,
            permalink: generateNumericPermalink(),
            createdAt: document.createdAt,
            updatedAt: document.updatedAt,
          }),
        )
        .returning()
        .then(firstOrThrow);

      const version = await insertPublicationVersion(tx, {
        publicationId: publication.id,
        version: 1,
        document: { title: document.title, subtitle: document.subtitle },
        input: { thumbnailId: document.thumbnailId, excerpt: null },
        snapshot,
      });

      return {
        outcome: 'migrated' as const,
        publicationId: publication.id,
        versionId: version.id,
        permalink: publication.permalink,
        updatedAt: publication.updatedAt.toISOString(),
        graphBytes: snapshot.graph.length,
      };
    });
  } catch (err) {
    return { outcome: 'failed', message: err instanceof Error ? err.message : String(err) };
  }
};

export const demoteLegacyEntities = async (entityIds: string[]): Promise<string[]> => {
  const demoted: string[] = [];
  for (const chunk of chunks(entityIds)) {
    const rows = await db
      .update(Entities)
      .set({ visibility: EntityVisibility.UNLISTED })
      .where(and(inArray(Entities.id, chunk), eq(Entities.visibility, EntityVisibility.PUBLIC)))
      .returning({ id: Entities.id });
    for (const row of rows) demoted.push(row.id);
  }
  return demoted;
};

export const restoreLegacyMigration = async (manifest: LegacyMigrationManifest) => {
  const skipped: { kind: 'publication' | 'space'; id: string; reason: string }[] = [];

  const restoredEntityIds: string[] = [];
  for (const chunk of chunks(manifest.demoted.map((entry) => entry.entityId))) {
    const rows = await db
      .update(Entities)
      .set({ visibility: EntityVisibility.PUBLIC })
      .where(and(inArray(Entities.id, chunk), eq(Entities.visibility, EntityVisibility.UNLISTED)))
      .returning({ id: Entities.id });
    for (const row of rows) restoredEntityIds.push(row.id);
  }

  for (const entry of manifest.publications) {
    try {
      const result = await db.transaction(async (tx) => {
        const publication = await tx
          .select({
            id: Publications.id,
            state: Publications.state,
            collectionId: Publications.collectionId,
            pinnedOrder: Publications.pinnedOrder,
            updatedAt: Publications.updatedAt,
          })
          .from(Publications)
          .where(eq(Publications.id, entry.id))
          .then(first);
        if (!publication) return 'missing';

        const versions = await tx
          .select({ id: PublicationVersions.id })
          .from(PublicationVersions)
          .where(eq(PublicationVersions.publicationId, entry.id));
        const tags = await tx.select({ id: PublicationTags.id }).from(PublicationTags).where(eq(PublicationTags.publicationId, entry.id));
        const untouched =
          publication.state === PublicationState.PUBLISHED &&
          publication.collectionId === null &&
          publication.pinnedOrder === null &&
          tags.length === 0 &&
          versions.length === 1 &&
          versions[0].id === entry.versionId &&
          publication.updatedAt.toISOString() === entry.updatedAt;
        if (!untouched) return 'changed';

        await tx.delete(PublicationVersions).where(eq(PublicationVersions.publicationId, entry.id));
        await tx.delete(Publications).where(eq(Publications.id, entry.id));
        return 'deleted';
      });
      if (result !== 'deleted') skipped.push({ kind: 'publication', id: entry.id, reason: result });
    } catch (err) {
      skipped.push({ kind: 'publication', id: entry.id, reason: err instanceof Error ? err.message : String(err) });
    }
  }

  for (const space of manifest.spaces) {
    const publications = await db.$count(Publications, eq(Publications.spaceId, space.id));
    const collections = await db.$count(Collections, eq(Collections.spaceId, space.id));
    if (publications > 0 || collections > 0) {
      skipped.push({ kind: 'space', id: space.id, reason: `publications ${publications}, collections ${collections}` });
      continue;
    }
    try {
      await db.delete(Spaces).where(eq(Spaces.id, space.id));
    } catch (err) {
      skipped.push({ kind: 'space', id: space.id, reason: err instanceof Error ? err.message : String(err) });
    }
  }

  return { skipped, restoredEntityIds };
};
