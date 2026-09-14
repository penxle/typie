import { EntityState, EntityVisibility, PublicationState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, desc, eq } from 'drizzle-orm';
import { first, firstOrThrow } from '#/db/index.ts';
import { Collections, Documents, Entities, Publications, PublicationTags, PublicationVersions } from '#/db/schemas/tables.ts';
import { getDurableHeads, readMergedGraph } from './changeset.ts';
import { extractAssetIdsFromPlainDoc } from './entity.ts';
import { generateFractionalOrder } from './order.ts';
import { assertSitePermission } from './permission.ts';
import { assertActiveSubscription } from './plan.ts';
import {
  buildDuePublicationsQuery,
  buildPublishedDocumentIdsQuery,
  hasUnpublishedChanges,
  normalizeTags,
  pickPublicationVersion,
  resolvePublishedVisibilityBlock,
  resolvePublishTransition,
  resolveVisibilityRequestBlock,
  validateScheduledAt,
} from './publication-core.ts';
import { buildPublicationSnapshot } from './publication-snapshot.ts';
import { findActiveSpace } from './space.ts';
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

const loadDocumentForPublishing = async (executor: Executor, documentId: string) =>
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
    .where(and(eq(Documents.id, documentId), eq(Entities.state, EntityState.ACTIVE)))
    .then(first);

const assertCollectionInSpace = async (executor: Executor, collectionId: string, spaceId: string) => {
  const collection = await executor
    .select({ id: Collections.id, spaceId: Collections.spaceId })
    .from(Collections)
    .where(eq(Collections.id, collectionId))
    .then(first);
  if (!collection || collection.spaceId !== spaceId) throw new TypieError({ code: 'collection_space_mismatch', status: 400 });
};

const replaceTags = async (tx: Transaction, publicationId: string, spaceId: string, tags: string[]) => {
  await tx.delete(PublicationTags).where(eq(PublicationTags.publicationId, publicationId));
  const rows: { publicationId: string; spaceId: string; name: string; order: string }[] = [];
  let lower: string | null = null;
  for (const name of tags) {
    const order = generateFractionalOrder({ lower, upper: null });
    rows.push({ publicationId, spaceId, name, order });
    lower = order;
  }
  if (rows.length > 0) {
    await tx.insert(PublicationTags).values(rows);
  }
};

const nextCollectionOrder = async (tx: Transaction, collectionId: string) => {
  const last = await tx
    .select({ order: Publications.collectionOrder })
    .from(Publications)
    .where(eq(Publications.collectionId, collectionId))
    .orderBy(desc(Publications.collectionOrder))
    .limit(1)
    .then(first);
  return generateFractionalOrder({ lower: last?.order ?? null, upper: null });
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
  const version = await tx
    .insert(PublicationVersions)
    .values({
      publicationId,
      version: picked.version,
      title: document.title,
      subtitle: document.subtitle,
      graph: snapshot.graph,
      text: snapshot.text,
      characterCount: snapshot.characterCount,
      heads: snapshot.heads,
      thumbnailId: input.thumbnailId,
      excerpt: input.excerpt,
      assetIds: flattenAssetIds(extractAssetIdsFromPlainDoc(snapshot.plain)),
    })
    .returning()
    .then(firstOrThrow);
  return { version, created: true };
};

export const publishDocumentCore = async (
  executor: Executor,
  args: {
    userId: string;
    documentId: string;
    spaceId: string;
    tags: string[];
    collectionId?: string | null;
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

  const space = await findActiveSpace(executor, args.spaceId);
  if (!space) throw new TypieError({ code: 'space_not_found', status: 404 });
  if (space.siteId !== document.siteId) throw new TypieError({ code: 'space_site_mismatch', status: 400 });
  if (args.collectionId) await assertCollectionInSpace(executor, args.collectionId, space.id);

  const scheduledAt = validateScheduledAt(args.scheduledAt ?? null, args.now);
  const tags = normalizeTags(args.tags);
  const thumbnailId = args.thumbnailId === undefined ? document.thumbnailId : args.thumbnailId;
  const excerpt = args.excerpt ?? null;
  const snapshot = await snapshotDocumentForPublication(document.id);

  return await executor.transaction(async (tx) => {
    await tx.select({ id: Documents.id }).from(Documents).where(eq(Documents.id, document.id)).for('update');
    const existing = await findPublicationByDocument(tx, document.id);
    const transition = resolvePublishTransition({ currentState: existing?.state ?? null, scheduledAt, now: args.now });

    const publication = existing
      ? await tx
          .update(Publications)
          .set({
            spaceId: space.id,
            state: transition.state,
            collectionId: args.collectionId ?? null,
            collectionOrder: args.collectionId
              ? existing.collectionId === args.collectionId
                ? existing.collectionOrder
                : await nextCollectionOrder(tx, args.collectionId)
              : null,
            publishedAt: existing.publishedAt ?? transition.publishedAt,
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
            documentId: document.id,
            spaceId: space.id,
            state: transition.state,
            collectionId: args.collectionId ?? null,
            collectionOrder: args.collectionId ? await nextCollectionOrder(tx, args.collectionId) : null,
            publishedAt: transition.publishedAt,
            scheduledAt: transition.scheduledAt,
            updatedAt: args.now,
          })
          .returning()
          .then(firstOrThrow);

    await writeVersion(tx, publication.id, document, { thumbnailId, excerpt }, snapshot);
    await replaceTags(tx, publication.id, space.id, tags);

    if (transition.state === PublicationState.PUBLISHED) {
      await tx.update(Entities).set({ visibility: EntityVisibility.PUBLIC }).where(eq(Entities.id, document.entityId));
    }

    return publication;
  });
};

export const updatePublicationCore = async (
  executor: Executor,
  args: {
    userId: string;
    publicationId: string;
    tags: string[];
    collectionId?: string | null;
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
  if (args.collectionId) await assertCollectionInSpace(executor, args.collectionId, existing.spaceId);

  const tags = normalizeTags(args.tags);
  const thumbnailId = args.thumbnailId === undefined ? document.thumbnailId : args.thumbnailId;
  const excerpt = args.excerpt ?? null;
  const snapshot = await snapshotDocumentForPublication(document.id);

  return await executor.transaction(async (tx) => {
    await tx.select({ id: Documents.id }).from(Documents).where(eq(Documents.id, document.id)).for('update');
    const current = await findPublicationByDocument(tx, document.id);
    if (!current || current.state !== PublicationState.PUBLISHED) {
      throw new TypieError({ code: 'publication_not_published', status: 409 });
    }

    const { created } = await writeVersion(tx, current.id, document, { thumbnailId, excerpt }, snapshot);
    await replaceTags(tx, current.id, current.spaceId, tags);
    return await tx
      .update(Publications)
      .set({
        collectionId: args.collectionId ?? null,
        collectionOrder: args.collectionId
          ? current.collectionId === args.collectionId
            ? current.collectionOrder
            : await nextCollectionOrder(tx, args.collectionId)
          : null,
        updatedAt: created ? args.now : current.updatedAt,
      })
      .where(eq(Publications.id, current.id))
      .returning()
      .then(firstOrThrow);
  });
};

export const unpublishDocumentCore = async (executor: Executor, args: { userId: string; documentId: string; now: Dayjs }) => {
  const document = await loadDocumentForPublishing(executor, args.documentId);
  if (!document) throw new TypieError({ code: 'document_not_found', status: 404 });
  await assertSitePermission({ userId: args.userId, siteId: document.siteId });
  return await executor.transaction(async (tx) => {
    const existing = await findPublicationByDocument(tx, document.id);
    if (!existing || existing.state !== PublicationState.PUBLISHED)
      throw new TypieError({ code: 'publication_not_published', status: 409 });
    const publication = await tx
      .update(Publications)
      .set({ state: PublicationState.UNPUBLISHED, unpublishedAt: args.now, scheduledAt: null, pinnedOrder: null })
      .where(eq(Publications.id, existing.id))
      .returning()
      .then(firstOrThrow);
    await tx.update(Entities).set({ visibility: EntityVisibility.PRIVATE }).where(eq(Entities.id, document.entityId));
    return publication;
  });
};

export const cancelScheduledPublicationCore = async (executor: Executor, args: { userId: string; publicationId: string; now: Dayjs }) => {
  const existing = await executor.select().from(Publications).where(eq(Publications.id, args.publicationId)).then(first);
  if (!existing || existing.state !== PublicationState.SCHEDULED) throw new TypieError({ code: 'publication_not_scheduled', status: 409 });
  const document = await loadDocumentForPublishing(executor, existing.documentId);
  if (!document) throw new TypieError({ code: 'document_not_found', status: 404 });
  await assertSitePermission({ userId: args.userId, siteId: document.siteId });
  return await executor
    .update(Publications)
    .set({ state: PublicationState.UNPUBLISHED, scheduledAt: null, unpublishedAt: args.now })
    .where(eq(Publications.id, existing.id))
    .returning()
    .then(firstOrThrow);
};

export const promoteDuePublicationsCore = async (tx: Transaction, args: { now: Dayjs }) => {
  const due = await buildDuePublicationsQuery(tx, { now: args.now });
  const promoted: { publicationId: string; siteId: string }[] = [];
  for (const row of due) {
    await tx
      .update(Publications)
      .set({
        state: PublicationState.PUBLISHED,
        publishedAt: row.publishedAt ?? row.scheduledAt ?? args.now,
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
