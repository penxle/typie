import { EntityState, EntityType, EntityVisibility, PublicationState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, asc, desc, eq, inArray, ne, sql } from 'drizzle-orm';
import { first, firstOrThrow } from '#/db/index.ts';
import { Documents, Entities, Publications, PublicationTags, PublicationVersions } from '#/db/schemas/tables.ts';
import { getDurableHeads, readMergedGraph } from './changeset.ts';
import { extractAssetIdsFromPlainDoc, extractPlainDocLayoutMode } from './entity.ts';
import { generateFractionalOrder } from './order.ts';
import { assertSitePermission } from './permission.ts';
import { assertActiveSubscription } from './plan.ts';
import {
  buildDuePublicationsQuery,
  buildPublishedDocumentIdsQuery,
  hasUnpublishedChanges,
  normalizeTags,
  pickPublicationVersion,
  resolveBulkPublishPlan,
  resolvePublishedVisibilityBlock,
  resolvePublishTransition,
  resolveVisibilityRequestBlock,
  validateScheduledAt,
} from './publication-core.ts';
import { buildPublicationSnapshot } from './publication-snapshot.ts';
import type { Dayjs } from 'dayjs';
import type { Database, Transaction } from '#/db/index.ts';
import type { PublicationSnapshot } from './publication-snapshot.ts';

type Executor = Database | Transaction;

const flattenAssetIds = (grouped: ReturnType<typeof extractAssetIdsFromPlainDoc>) => [
  ...grouped.imageIds,
  ...grouped.fileIds,
  ...grouped.embedIds,
  ...grouped.archivedIds,
];

export const insertPublicationVersion = async (
  tx: Transaction,
  args: {
    publicationId: string;
    version: number;
    document: { title: string | null; subtitle: string | null };
    input: { thumbnailId: string | null; excerpt: string | null };
    snapshot: PublicationSnapshot;
  },
) =>
  await tx
    .insert(PublicationVersions)
    .values({
      publicationId: args.publicationId,
      version: args.version,
      title: args.document.title,
      subtitle: args.document.subtitle,
      graph: args.snapshot.graph,
      text: args.snapshot.text,
      characterCount: args.snapshot.characterCount,
      heads: args.snapshot.heads,
      thumbnailId: args.input.thumbnailId,
      excerpt: args.input.excerpt,
      assetIds: flattenAssetIds(extractAssetIdsFromPlainDoc(args.snapshot.plain)),
      layoutMode: extractPlainDocLayoutMode(args.snapshot.plain),
    })
    .returning()
    .then(firstOrThrow);

export const findPublicationByDocument = async (executor: Executor, documentId: string) =>
  await executor.select().from(Publications).where(eq(Publications.documentId, documentId)).then(first);

export const findLatestVersion = async (executor: Executor, publicationId: string) =>
  await executor
    .select()
    .from(PublicationVersions)
    .where(eq(PublicationVersions.publicationId, publicationId))
    .orderBy(desc(PublicationVersions.version))
    .limit(1)
    .then(first);

export const assertNoPublishedPublication = async (executor: Executor, args: { documentIds: string[]; visibility: EntityVisibility }) => {
  const code = resolvePublishedVisibilityBlock(args.visibility);
  if (!code) return;
  const published = await buildPublishedDocumentIdsQuery(executor, { documentIds: args.documentIds }).then(first);
  if (published) throw new TypieError({ code, status: 409 });
};

export const findPublishedDocumentIds = async (executor: Executor, args: { documentIds: string[]; visibility: EntityVisibility }) => {
  if (args.documentIds.length === 0 || !resolvePublishedVisibilityBlock(args.visibility)) return new Set<string>();
  const rows = await buildPublishedDocumentIdsQuery(executor, { documentIds: args.documentIds });
  return new Set(rows.map((row) => row.documentId));
};

export const assertVisibilityRequestable = (visibility: EntityVisibility) => {
  const code = resolveVisibilityRequestBlock(visibility);
  if (code) throw new TypieError({ code, status: 400 });
};

const snapshotDocumentForPublication = async (documentId: string) => {
  const graph = await readMergedGraph(documentId);
  if (graph.length === 0) {
    throw new TypieError({ code: 'publication_empty_document', status: 409 });
  }
  return await buildPublicationSnapshot(graph);
};

const loadDocumentsForPublishing = async (executor: Executor, documentIds: string[]) =>
  await executor
    .select({
      id: Documents.id,
      title: Documents.title,
      subtitle: Documents.subtitle,
      thumbnailId: Documents.thumbnailId,
      entityId: Entities.id,
      siteId: Entities.siteId,
    })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(and(inArray(Documents.id, documentIds), eq(Entities.state, EntityState.ACTIVE)));

const loadDocumentForPublishing = async (executor: Executor, documentId: string) =>
  await loadDocumentsForPublishing(executor, [documentId]).then(first);

type PublishingDocument = NonNullable<Awaited<ReturnType<typeof loadDocumentForPublishing>>>;

const withDocumentId = (err: unknown, documentId: string): unknown =>
  err instanceof TypieError && err.extra === undefined
    ? new TypieError({ code: err.code, message: err.message, status: err.status, extra: { documentId } })
    : err;

const replaceTags = async (tx: Transaction, publicationId: string, siteId: string, tags: string[]) => {
  await tx.delete(PublicationTags).where(eq(PublicationTags.publicationId, publicationId));
  const rows: { publicationId: string; siteId: string; name: string; order: string }[] = [];
  let lower: string | null = null;
  for (const name of tags) {
    const order = generateFractionalOrder({ lower, upper: null });
    rows.push({ publicationId, siteId, name, order });
    lower = order;
  }
  if (rows.length > 0) {
    await tx.insert(PublicationTags).values(rows);
  }
};

const writeVersion = async (
  tx: Transaction,
  publicationId: string,
  document: { id: string; title: string | null; subtitle: string | null },
  input: { thumbnailId: string | null; excerpt: string | null },
  snapshot: PublicationSnapshot,
) => {
  const latest = await findLatestVersion(tx, publicationId);
  const snap = {
    title: document.title,
    subtitle: document.subtitle,
    heads: snapshot.heads,
    thumbnailId: input.thumbnailId,
    excerpt: input.excerpt,
  };
  const picked = pickPublicationVersion(latest ?? null, snap);
  if (latest && picked.reuse) return { version: latest, created: false };
  const version = await insertPublicationVersion(tx, { publicationId, version: picked.version, document, input, snapshot });
  return { version, created: true };
};

const applyPublish = async (
  tx: Transaction,
  args: {
    document: PublishingDocument;
    tags: string[];
    scheduledAt: Dayjs | null;
    version: { thumbnailId: string | null; excerpt: string | null };
    snapshot: PublicationSnapshot;
    now: Dayjs;
  },
) => {
  const existing = await findPublicationByDocument(tx, args.document.id);
  const transition = resolvePublishTransition({ currentState: existing?.state ?? null, scheduledAt: args.scheduledAt, now: args.now });

  const publication = existing
    ? await tx
        .update(Publications)
        .set({
          siteId: args.document.siteId,
          state: transition.state,
          publishedAt: transition.publishedAt,
          scheduledAt: transition.scheduledAt,
          unpublishedAt: null,
          updatedAt: args.now,
        })
        .where(eq(Publications.id, existing.id))
        .returning()
        .then(firstOrThrow)
    : await tx
        .insert(Publications)
        .values({
          documentId: args.document.id,
          siteId: args.document.siteId,
          state: transition.state,
          publishedAt: transition.publishedAt,
          scheduledAt: transition.scheduledAt,
          updatedAt: args.now,
        })
        .returning()
        .then(firstOrThrow);

  await writeVersion(tx, publication.id, args.document, args.version, args.snapshot);
  await replaceTags(tx, publication.id, args.document.siteId, args.tags);

  if (transition.state === PublicationState.PUBLISHED) {
    await tx.update(Entities).set({ visibility: EntityVisibility.PUBLIC }).where(eq(Entities.id, args.document.entityId));
  }

  return publication;
};

const applyRepublish = async (
  tx: Transaction,
  args: {
    document: PublishingDocument;
    tags: string[];
    version: { thumbnailId: string | null; excerpt: string | null };
    snapshot: PublicationSnapshot;
    now: Dayjs;
  },
) => {
  const current = await findPublicationByDocument(tx, args.document.id);
  if (!current || current.state !== PublicationState.PUBLISHED) {
    throw new TypieError({ code: 'publication_not_published', status: 409 });
  }

  const { created } = await writeVersion(tx, current.id, args.document, args.version, args.snapshot);
  await replaceTags(tx, current.id, args.document.siteId, args.tags);

  return await tx
    .update(Publications)
    .set({ updatedAt: created ? args.now : current.updatedAt })
    .where(eq(Publications.id, current.id))
    .returning()
    .then(firstOrThrow);
};

const applyUnpublish = async (tx: Transaction, document: PublishingDocument, now: Dayjs) => {
  const existing = await findPublicationByDocument(tx, document.id);
  if (!existing || existing.state !== PublicationState.PUBLISHED) {
    throw new TypieError({ code: 'publication_not_published', status: 409 });
  }

  const publication = await tx
    .update(Publications)
    .set({ state: PublicationState.UNPUBLISHED, unpublishedAt: now, scheduledAt: null, pinnedOrder: null })
    .where(eq(Publications.id, existing.id))
    .returning()
    .then(firstOrThrow);

  await tx.update(Entities).set({ visibility: EntityVisibility.PRIVATE }).where(eq(Entities.id, document.entityId));

  return publication;
};

const applyCancelSchedule = async (tx: Transaction, document: PublishingDocument, now: Dayjs) => {
  const existing = await findPublicationByDocument(tx, document.id);
  if (!existing || existing.state !== PublicationState.SCHEDULED) {
    throw new TypieError({ code: 'publication_not_scheduled', status: 409 });
  }

  return await tx
    .update(Publications)
    .set({ state: PublicationState.UNPUBLISHED, scheduledAt: null, unpublishedAt: now })
    .where(eq(Publications.id, existing.id))
    .returning()
    .then(firstOrThrow);
};

export const publishDocumentCore = async (
  executor: Executor,
  args: {
    userId: string;
    documentId: string;
    tags: string[];
    excerpt?: string | null;
    thumbnailId?: string | null;
    scheduledAt?: Dayjs | null;
    now: Dayjs;
  },
) => {
  const document = await loadDocumentForPublishing(executor, args.documentId);
  if (!document) throw new TypieError({ code: 'document_not_found', status: 404 });
  await assertSitePermission({ userId: args.userId, siteId: document.siteId });
  await assertActiveSubscription({ userId: args.userId });

  const scheduledAt = validateScheduledAt(args.scheduledAt ?? null, args.now);
  const tags = normalizeTags(args.tags);
  const version = { thumbnailId: args.thumbnailId === undefined ? document.thumbnailId : args.thumbnailId, excerpt: args.excerpt ?? null };
  const snapshot = await snapshotDocumentForPublication(document.id);

  return await executor.transaction(async (tx) => {
    await tx.select({ id: Documents.id }).from(Documents).where(eq(Documents.id, document.id)).for('update');
    return await applyPublish(tx, { document, tags, scheduledAt, version, snapshot, now: args.now });
  });
};

export const updatePublicationCore = async (
  executor: Executor,
  args: {
    userId: string;
    publicationId: string;
    tags: string[];
    excerpt?: string | null;
    thumbnailId?: string | null;
    now: Dayjs;
  },
) => {
  const existing = await executor.select().from(Publications).where(eq(Publications.id, args.publicationId)).then(first);
  if (!existing || existing.state !== PublicationState.PUBLISHED) throw new TypieError({ code: 'publication_not_published', status: 409 });
  const document = await loadDocumentForPublishing(executor, existing.documentId);
  if (!document) throw new TypieError({ code: 'document_not_found', status: 404 });
  await assertSitePermission({ userId: args.userId, siteId: document.siteId });
  await assertActiveSubscription({ userId: args.userId });

  const tags = normalizeTags(args.tags);
  const version = { thumbnailId: args.thumbnailId === undefined ? document.thumbnailId : args.thumbnailId, excerpt: args.excerpt ?? null };
  const snapshot = await snapshotDocumentForPublication(document.id);

  return await executor.transaction(async (tx) => {
    await tx.select({ id: Documents.id }).from(Documents).where(eq(Documents.id, document.id)).for('update');
    return await applyRepublish(tx, { document, tags, version, snapshot, now: args.now });
  });
};

export const unpublishDocumentCore = async (executor: Executor, args: { userId: string; documentId: string; now: Dayjs }) => {
  const document = await loadDocumentForPublishing(executor, args.documentId);
  if (!document) throw new TypieError({ code: 'document_not_found', status: 404 });
  await assertSitePermission({ userId: args.userId, siteId: document.siteId });
  return await executor.transaction(async (tx) => await applyUnpublish(tx, document, args.now));
};

export const cancelScheduledPublicationCore = async (executor: Executor, args: { userId: string; publicationId: string; now: Dayjs }) => {
  const existing = await executor.select().from(Publications).where(eq(Publications.id, args.publicationId)).then(first);
  if (!existing || existing.state !== PublicationState.SCHEDULED) throw new TypieError({ code: 'publication_not_scheduled', status: 409 });
  const document = await loadDocumentForPublishing(executor, existing.documentId);
  if (!document) throw new TypieError({ code: 'document_not_found', status: 404 });
  await assertSitePermission({ userId: args.userId, siteId: document.siteId });
  return await executor.transaction(async (tx) => await applyCancelSchedule(tx, document, args.now));
};

const loadDocumentsOrThrow = async (executor: Executor, args: { userId: string; documentIds: string[] }) => {
  const documentIds = [...new Set(args.documentIds)].toSorted((a, b) => a.localeCompare(b));
  const documents = await loadDocumentsForPublishing(executor, documentIds);
  const byId = new Map(documents.map((document) => [document.id, document]));

  for (const documentId of documentIds) {
    if (!byId.has(documentId)) throw new TypieError({ code: 'document_not_found', status: 404, extra: { documentId } });
  }

  for (const siteId of new Set(documents.map((document) => document.siteId))) {
    await assertSitePermission({ userId: args.userId, siteId });
  }

  return { documentIds, documents: documentIds.map((id) => byId.get(id) as PublishingDocument) };
};

const lockDocuments = async (tx: Transaction, documentIds: string[]) => {
  await tx.select({ id: Documents.id }).from(Documents).where(inArray(Documents.id, documentIds)).orderBy(asc(Documents.id)).for('update');
};

export const findFolderDocumentIds = async (executor: Executor, args: { userId: string; folderIds: string[] }) => {
  const folders = await executor
    .select({ entityId: Entities.id, siteId: Entities.siteId })
    .from(Entities)
    .where(and(inArray(Entities.id, args.folderIds), eq(Entities.type, EntityType.FOLDER), eq(Entities.state, EntityState.ACTIVE)));

  if (folders.length === 0) throw new TypieError({ code: 'invalid_argument', status: 400 });

  for (const siteId of new Set(folders.map((folder) => folder.siteId))) {
    await assertSitePermission({ userId: args.userId, siteId });
  }

  const rows = await executor.execute<{ id: string }>(sql`
    WITH RECURSIVE sq AS (
      SELECT ${Entities.id}, ${Entities.type} FROM ${Entities}
      WHERE ${inArray(
        Entities.parentId,
        folders.map((folder) => folder.entityId),
      )}
        AND ${eq(Entities.state, EntityState.ACTIVE)} AND ${ne(Entities.type, EntityType.DIVIDER)}
      UNION ALL
      SELECT ${Entities.id}, ${Entities.type} FROM ${Entities}
      JOIN sq ON ${Entities.parentId} = sq.id
      WHERE ${eq(Entities.state, EntityState.ACTIVE)} AND ${ne(Entities.type, EntityType.DIVIDER)}
    )
    SELECT ${Documents.id} FROM sq
    INNER JOIN ${Documents} ON ${Documents.entityId} = sq.id
  `);

  return rows.map(({ id }) => id);
};

export const publishDocumentsCore = async (
  executor: Executor,
  args: {
    userId: string;
    documentIds: string[];
    addTags?: string[];
    removeTags?: string[];
    scheduledAt?: Dayjs | null;
    now: Dayjs;
  },
) => {
  if (args.documentIds.length === 0) throw new TypieError({ code: 'publication_no_documents', status: 400 });

  const { documentIds, documents } = await loadDocumentsOrThrow(executor, args);
  await assertActiveSubscription({ userId: args.userId });

  const scheduledAt = args.scheduledAt === undefined ? undefined : validateScheduledAt(args.scheduledAt, args.now);

  const publications = await executor.select().from(Publications).where(inArray(Publications.documentId, documentIds));
  const publicationByDocument = new Map(publications.map((publication) => [publication.documentId, publication]));

  const tagRows =
    publications.length > 0
      ? await executor
          .select({ publicationId: PublicationTags.publicationId, name: PublicationTags.name })
          .from(PublicationTags)
          .where(
            inArray(
              PublicationTags.publicationId,
              publications.map((publication) => publication.id),
            ),
          )
          .orderBy(asc(PublicationTags.order))
      : [];
  const tagsByPublication = new Map<string, string[]>();
  for (const row of tagRows) {
    tagsByPublication.set(row.publicationId, [...(tagsByPublication.get(row.publicationId) ?? []), row.name]);
  }

  const plan = resolveBulkPublishPlan(
    documentIds.map((documentId) => {
      const existing = publicationByDocument.get(documentId);
      return {
        documentId,
        existing: existing
          ? { state: existing.state, scheduledAt: existing.scheduledAt, tags: tagsByPublication.get(existing.id) ?? [] }
          : null,
      };
    }),
    { addTags: args.addTags, removeTags: args.removeTags, scheduledAt },
  );

  const snapshots = new Map<string, PublicationSnapshot>();
  for (const documentId of documentIds) {
    try {
      snapshots.set(documentId, await snapshotDocumentForPublication(documentId));
    } catch (err) {
      throw withDocumentId(err, documentId);
    }
  }

  return await executor.transaction(async (tx) => {
    await lockDocuments(tx, documentIds);

    const results: (typeof publications)[number][] = [];
    for (const [index, step] of plan.entries()) {
      const document = documents[index];
      const snapshot = snapshots.get(step.documentId);
      if (!snapshot) throw new TypieError({ code: 'document_not_found', status: 404, extra: { documentId: step.documentId } });

      const existing = publicationByDocument.get(step.documentId);
      const latest = existing ? await findLatestVersion(tx, existing.id) : null;
      const version = latest
        ? { thumbnailId: latest.thumbnailId, excerpt: latest.excerpt }
        : { thumbnailId: document.thumbnailId, excerpt: null };

      try {
        results.push(
          step.kind === 'republish'
            ? await applyRepublish(tx, { document, tags: step.tags, version, snapshot, now: args.now })
            : await applyPublish(tx, { document, tags: step.tags, scheduledAt: step.scheduledAt, version, snapshot, now: args.now }),
        );
      } catch (err) {
        throw withDocumentId(err, step.documentId);
      }
    }

    return results;
  });
};

export const unpublishDocumentsCore = async (executor: Executor, args: { userId: string; documentIds: string[]; now: Dayjs }) => {
  if (args.documentIds.length === 0) throw new TypieError({ code: 'publication_no_documents', status: 400 });

  const { documentIds, documents } = await loadDocumentsOrThrow(executor, args);
  const publications = await executor.select().from(Publications).where(inArray(Publications.documentId, documentIds));
  const stateByDocument = new Map(publications.map((publication) => [publication.documentId, publication.state]));

  return await executor.transaction(async (tx) => {
    await lockDocuments(tx, documentIds);

    const results: Awaited<ReturnType<typeof applyUnpublish>>[] = [];
    for (const document of documents) {
      const state = stateByDocument.get(document.id);
      if (state !== PublicationState.PUBLISHED && state !== PublicationState.SCHEDULED) continue;
      try {
        results.push(
          state === PublicationState.SCHEDULED
            ? await applyCancelSchedule(tx, document, args.now)
            : await applyUnpublish(tx, document, args.now),
        );
      } catch (err) {
        throw withDocumentId(err, document.id);
      }
    }

    return results;
  });
};

export const promoteDuePublicationsCore = async (tx: Transaction, args: { now: Dayjs }) => {
  const due = await buildDuePublicationsQuery(tx, { now: args.now });
  const promoted: { publicationId: string; siteId: string }[] = [];
  for (const row of due) {
    await tx
      .update(Publications)
      .set({
        state: PublicationState.PUBLISHED,
        publishedAt: row.scheduledAt ?? args.now,
        scheduledAt: null,
        unpublishedAt: null,
      })
      .where(eq(Publications.id, row.id));
    const document = await tx
      .select({ entityId: Documents.entityId, siteId: Entities.siteId })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .where(eq(Documents.id, row.documentId))
      .then(firstOrThrow);
    await tx.update(Entities).set({ visibility: EntityVisibility.PUBLIC }).where(eq(Entities.id, document.entityId));
    promoted.push({ publicationId: row.id, siteId: document.siteId });
  }
  return promoted;
};

export const computeHasUnpublishedChanges = async (
  document: { id: string; title: string | null; subtitle: string | null },
  version: { heads: Uint8Array; title: string | null; subtitle: string | null },
  liveHeads: Uint8Array | null,
) => {
  const heads = liveHeads ?? (await getDurableHeads(document.id));
  return hasUnpublishedChanges(version, { heads, title: document.title, subtitle: document.subtitle });
};
