import { EntityAvailability, EntityState, EntityType, NoteState } from '@typie/lib/enums';
import { NotFoundError, TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, asc, desc, eq, gt, inArray, isNull, ne, sql } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import {
  Dividers,
  DocumentBundles,
  Documents,
  DocumentStates,
  Entities,
  first,
  firstOrThrow,
  firstOrThrowWith,
  Folders,
  NoteEntities,
  Notes,
  UserPreferences,
} from '#/db/index.ts';
import { publishRecentDocumentUpdates, pubsub } from '#/pubsub.ts';
import { isPrivateVisibilityOnlyInput } from './documents-option-policy.ts';
import {
  buildFreshV2Content,
  calculateBlobSizeFromAssetIds,
  derivePlainRootFromPreset,
  extractAssetIdsFromPlainDoc,
  generatePermalink,
  generateSlug,
  insertFreshV2Content,
} from './entity.ts';
import { generateFractionalOrder } from './order.ts';
import { assertSitePermission } from './permission.ts';
import { assertActiveSubscription, hasActiveSubscription } from './plan.ts';
import { runAfterCommit } from './post-commit.ts';
import { enqueueSearchSyncForEntityIds } from './search-index.ts';
import { wasm as wasmFfi } from './wasm-ffi.ts';
import type { DocumentContentRating, EntityVisibility } from '@typie/lib/enums';
import type { Database, Transaction } from '#/db/index.ts';
import type { TemplatePreset } from './entity.ts';
import type { PostCommitRegistrar } from './post-commit.ts';

type CreateFolderCoreArgs = {
  userId: string;
  siteId: string;
  parentEntityId: string | null;
  name: string;
  lowerOrder?: string | null;
  upperOrder?: string | null;
};

export const createFolderCore = async (executor: Database | Transaction, args: CreateFolderCoreArgs, afterCommit?: PostCommitRegistrar) => {
  await assertSitePermission({
    userId: args.userId,
    siteId: args.siteId,
  });

  await assertActiveSubscription({ userId: args.userId });

  let depth = 0;
  if (args.parentEntityId) {
    const parentEntity = await executor
      .select({ id: Entities.id, depth: Entities.depth })
      .from(Entities)
      .where(
        and(
          eq(Entities.siteId, args.siteId),
          eq(Entities.id, args.parentEntityId),
          eq(Entities.type, EntityType.FOLDER),
          eq(Entities.state, EntityState.ACTIVE),
        ),
      )
      .then(firstOrThrow);

    depth = parentEntity.depth + 1;
  }

  let orderLower: string | null = args.lowerOrder ?? null;

  if (!args.lowerOrder) {
    const last = await executor
      .select({ order: Entities.order })
      .from(Entities)
      .where(
        and(eq(Entities.siteId, args.siteId), args.parentEntityId ? eq(Entities.parentId, args.parentEntityId) : isNull(Entities.parentId)),
      )
      .orderBy(desc(Entities.order))
      .limit(1)
      .then(first);

    orderLower = last?.order ?? null;
  }

  const folder = await executor.transaction(async (tx) => {
    const entity = await tx
      .insert(Entities)
      .values({
        userId: args.userId,
        siteId: args.siteId,
        parentId: args.parentEntityId,
        slug: generateSlug(),
        permalink: generatePermalink(),
        type: EntityType.FOLDER,
        icon: 'folder',
        order: generateFractionalOrder({ lower: orderLower, upper: args.upperOrder ?? null }),
        depth,
      })
      .returning({ id: Entities.id })
      .then(firstOrThrow);

    const folder = await tx
      .insert(Folders)
      .values({
        entityId: entity.id,
        name: args.name,
      })
      .returning()
      .then(firstOrThrow);

    return folder;
  });

  await runAfterCommit(afterCommit, async () => {
    if (args.parentEntityId) {
      pubsub.publish('site:update', args.siteId, { scope: 'entity', entityId: args.parentEntityId });
    } else {
      pubsub.publish('site:update', args.siteId, { scope: 'site' });
    }

    const { enqueueJob } = await import('#/mq/index.ts');
    await enqueueJob('search:index:folder', folder.id);
  });

  return folder;
};

type CreateDividerCoreArgs = {
  userId: string;
  siteId: string;
  parentEntityId: string | null;
  lowerOrder?: string | null;
  upperOrder?: string | null;
};

export const createDividerCore = async (
  executor: Database | Transaction,
  args: CreateDividerCoreArgs,
  afterCommit?: PostCommitRegistrar,
) => {
  await assertSitePermission({
    userId: args.userId,
    siteId: args.siteId,
  });

  await assertActiveSubscription({ userId: args.userId });

  let depth = 0;
  if (args.parentEntityId) {
    const parentEntity = await executor
      .select({ id: Entities.id, depth: Entities.depth })
      .from(Entities)
      .where(
        and(
          eq(Entities.siteId, args.siteId),
          eq(Entities.id, args.parentEntityId),
          eq(Entities.type, EntityType.FOLDER),
          eq(Entities.state, EntityState.ACTIVE),
        ),
      )
      .then(firstOrThrow);

    depth = parentEntity.depth + 1;
  }

  let orderLower: string | null = args.lowerOrder ?? null;

  if (!args.lowerOrder) {
    const last = await executor
      .select({ order: Entities.order })
      .from(Entities)
      .where(
        and(eq(Entities.siteId, args.siteId), args.parentEntityId ? eq(Entities.parentId, args.parentEntityId) : isNull(Entities.parentId)),
      )
      .orderBy(desc(Entities.order))
      .limit(1)
      .then(first);

    orderLower = last?.order ?? null;
  }

  const divider = await executor.transaction(async (tx) => {
    const entity = await tx
      .insert(Entities)
      .values({
        userId: args.userId,
        siteId: args.siteId,
        parentId: args.parentEntityId,
        slug: generateSlug(),
        permalink: generatePermalink(),
        type: EntityType.DIVIDER,
        icon: 'minus',
        order: generateFractionalOrder({ lower: orderLower, upper: args.upperOrder ?? null }),
        depth,
      })
      .returning({ id: Entities.id })
      .then(firstOrThrow);

    const divider = await tx
      .insert(Dividers)
      .values({
        entityId: entity.id,
      })
      .returning()
      .then(firstOrThrow);

    return divider;
  });

  await runAfterCommit(afterCommit, async () => {
    if (args.parentEntityId) {
      pubsub.publish('site:update', args.siteId, { scope: 'entity', entityId: args.parentEntityId });
    } else {
      pubsub.publish('site:update', args.siteId, { scope: 'site' });
    }
  });

  return divider;
};

type DeleteEntitiesCoreArgs = {
  userId: string;
  entityIds: string[];
};

export const deleteEntitiesCore = async (
  executor: Database | Transaction,
  args: DeleteEntitiesCoreArgs,
  afterCommit?: PostCommitRegistrar,
) => {
  const entities = await executor.execute<{ id: string; site_id: string; parent_id: string | null; type: EntityType }>(sql`
    WITH RECURSIVE sq AS (
      SELECT ${Entities.id}, ${Entities.parentId}, ${Entities.siteId}, ${Entities.type}
      FROM ${Entities}
      WHERE ${inArray(Entities.id, args.entityIds)}
      UNION ALL
      SELECT ${Entities.id}, ${Entities.parentId}, ${Entities.siteId}, ${Entities.type}
      FROM ${Entities}
      JOIN sq ON ${Entities.parentId} = sq.id
    )
    SELECT id, site_id, parent_id, type
    FROM sq
  `);

  if (entities.length === 0) {
    return [];
  }

  const siteId = entities[0].site_id;

  await assertSitePermission({
    userId: args.userId,
    siteId,
  });

  if (entities.some((entity) => entity.site_id !== siteId)) {
    throw new TypieError({ code: 'site_mismatch' });
  }

  const deletedEntities = await executor.transaction(async (tx) => {
    const deletedEntities = await tx
      .update(Entities)
      .set({
        state: EntityState.DELETED,
        deletedAt: dayjs(),
      })
      .where(
        inArray(
          Entities.id,
          entities.map(({ id }) => id),
        ),
      )
      .returning();

    const deletedEntityIds = deletedEntities.map(({ id }) => id);

    await tx.execute(sql`
      UPDATE ${Notes}
      SET state = ${NoteState.DELETED_CASCADED}, updated_at = NOW()
      WHERE ${Notes.id} IN (
        SELECT ${NoteEntities.noteId}
        FROM ${NoteEntities}
        WHERE ${inArray(NoteEntities.entityId, deletedEntityIds)}
      )
      AND ${eq(Notes.state, NoteState.ACTIVE)}
      AND NOT EXISTS (
        SELECT 1
        FROM ${NoteEntities} ne2
        INNER JOIN ${Entities} e ON e.id = ne2.entity_id
        WHERE ne2.note_id = ${Notes.id}
        AND ${eq(sql`e.state`, EntityState.ACTIVE)}
      )
    `);

    return deletedEntities;
  });

  const inputEntityIdSet = new Set(args.entityIds);
  const directEntities = entities.filter((entity) => inputEntityIdSet.has(entity.id));
  const parentIds = new Set(directEntities.map((entity) => entity.parent_id).filter((id): id is string => id !== null));

  await runAfterCommit(afterCommit, async () => {
    for (const parentId of parentIds) {
      pubsub.publish('site:update', siteId, { scope: 'entity', entityId: parentId });
    }
    if (directEntities.some((entity) => entity.parent_id === null)) {
      pubsub.publish('site:update', siteId, { scope: 'site' });
    }
    pubsub.publish('user:usage:update', args.userId, null);

    for (const entity of deletedEntities) {
      pubsub.publish('site:update', siteId, { scope: 'entity', entityId: entity.id });
    }
    if (entities.some((entity) => entity.type === EntityType.DOCUMENT)) {
      publishRecentDocumentUpdates(siteId, 'VIEWED_AT', 'UPDATED_AT');
    }

    await enqueueSearchSyncForEntityIds(deletedEntities.map(({ id }) => id));
  });

  return deletedEntities;
};

type CreateDocumentCoreArgs = {
  userId: string;
  siteId: string;
  parentEntityId: string | null;
  lowerOrder?: string | null;
  upperOrder?: string | null;
};

export const createDocumentCore = async (
  executor: Database | Transaction,
  args: CreateDocumentCoreArgs,
  afterCommit?: PostCommitRegistrar,
) => {
  await assertSitePermission({
    userId: args.userId,
    siteId: args.siteId,
  });

  await assertActiveSubscription({ userId: args.userId });

  let depth = 0;
  if (args.parentEntityId) {
    const parentEntity = await executor
      .select({ id: Entities.id, depth: Entities.depth })
      .from(Entities)
      .where(
        and(
          eq(Entities.siteId, args.siteId),
          eq(Entities.id, args.parentEntityId),
          eq(Entities.type, EntityType.FOLDER),
          eq(Entities.state, EntityState.ACTIVE),
        ),
      )
      .then(firstOrThrow);

    depth = parentEntity.depth + 1;
  }

  let orderLower: string | null = args.lowerOrder ?? null;

  if (!args.lowerOrder) {
    const last = await executor
      .select({ order: Entities.order })
      .from(Entities)
      .where(
        and(eq(Entities.siteId, args.siteId), args.parentEntityId ? eq(Entities.parentId, args.parentEntityId) : isNull(Entities.parentId)),
      )
      .orderBy(desc(Entities.order))
      .limit(1)
      .then(first);

    orderLower = last?.order ?? null;
  }

  const preference = await executor
    .select({ value: UserPreferences.value })
    .from(UserPreferences)
    .where(eq(UserPreferences.userId, args.userId))
    .then(first);

  const preset = (preference?.value as Record<string, unknown> | undefined)?.template as TemplatePreset | undefined;

  const document = await executor.transaction(async (tx) => {
    const entity = await tx
      .insert(Entities)
      .values({
        userId: args.userId,
        siteId: args.siteId,
        parentId: args.parentEntityId,
        slug: generateSlug(),
        permalink: generatePermalink(),
        type: EntityType.DOCUMENT,
        icon: 'file',
        order: generateFractionalOrder({ lower: orderLower, upper: args.upperOrder ?? null }),
        depth,
      })
      .returning({ id: Entities.id })
      .then(firstOrThrow);

    const document = await tx
      .insert(Documents)
      .values({
        entityId: entity.id,
        title: null,
      })
      .returning()
      .then(firstOrThrow);

    const { root, modifiers } = derivePlainRootFromPreset(preset);
    await wasmFfi.use(async (host) => {
      const plain = host.default_doc_with_preset(root, modifiers);
      const graph = host.to_graph(plain);
      const heads = host.heads(graph);
      const text = host.extract_text(plain);
      const { imageIds, fileIds } = extractAssetIdsFromPlainDoc(plain);
      const blobSize = await calculateBlobSizeFromAssetIds(imageIds, fileIds);
      const characterCount = host.count_characters(text);

      await tx.insert(DocumentBundles).values({ documentId: document.id, seq: 1, payload: graph });
      await tx.insert(DocumentStates).values({
        documentId: document.id,
        json: plain,
        text,
        characterCount,
        blobSize,
        heads,
        lastBundleSeq: 1,
      });
    });

    return document;
  });

  await runAfterCommit(afterCommit, async () => {
    if (args.parentEntityId) {
      pubsub.publish('site:update', args.siteId, { scope: 'entity', entityId: args.parentEntityId });
    } else {
      pubsub.publish('site:update', args.siteId, { scope: 'site' });
    }
    publishRecentDocumentUpdates(args.siteId, 'UPDATED_AT');

    pubsub.publish('user:usage:update', args.userId, null);

    const { enqueueJob } = await import('#/mq/index.ts');
    await enqueueJob('search:index:document', document.id);
  });

  return document;
};

type DuplicateDocumentCoreArgs = {
  userId: string;
  documentId: string;
};

export const duplicateDocumentCore = async (
  executor: Database | Transaction,
  args: DuplicateDocumentCoreArgs,
  afterCommit?: PostCommitRegistrar,
) => {
  const entity = await executor
    .select({
      id: Entities.id,
      siteId: Entities.siteId,
      parentEntityId: Entities.parentId,
      order: Entities.order,
      depth: Entities.depth,
      icon: Entities.icon,
      iconColor: Entities.iconColor,
    })
    .from(Entities)
    .innerJoin(Documents, eq(Entities.id, Documents.entityId))
    .where(eq(Documents.id, args.documentId))
    .then(firstOrThrow);

  await assertSitePermission({
    userId: args.userId,
    siteId: entity.siteId,
  });

  const nextEntity = await executor
    .select({ order: Entities.order })
    .from(Entities)
    .where(
      and(
        eq(Entities.siteId, entity.siteId),
        entity.parentEntityId ? eq(Entities.parentId, entity.parentEntityId) : isNull(Entities.parentId),
        gt(Entities.order, entity.order),
      ),
    )
    .orderBy(asc(Entities.order))
    .limit(1)
    .then(first);

  const document = await executor
    .select({
      title: Documents.title,
      subtitle: Documents.subtitle,
    })
    .from(Documents)
    .where(eq(Documents.id, args.documentId))
    .then(firstOrThrow);

  await assertActiveSubscription({ userId: args.userId });

  // TODO: anchors

  const noteRows = await executor
    .select({
      content: Notes.content,
      color: Notes.color,
      status: Notes.status,
    })
    .from(NoteEntities)
    .innerJoin(Notes, eq(NoteEntities.noteId, Notes.id))
    .where(and(eq(NoteEntities.entityId, entity.id), eq(Notes.state, NoteState.ACTIVE)));

  const v2Content = await buildFreshV2Content(args.documentId);
  if (!v2Content) {
    throw new TypieError({ code: 'document_projection_degraded', message: '문서를 복사할 수 없는 상태예요.', status: 409 });
  }

  const title = `(사본) ${document.title ?? '(제목 없음)'}`;

  const newDocument = await executor.transaction(async (tx) => {
    const newEntity = await tx
      .insert(Entities)
      .values({
        userId: args.userId,
        siteId: entity.siteId,
        parentId: entity.parentEntityId,
        slug: generateSlug(),
        permalink: generatePermalink(),
        type: EntityType.DOCUMENT,
        order: generateFractionalOrder({ lower: entity.order, upper: nextEntity?.order }),
        depth: entity.depth,
        icon: entity.icon,
        iconColor: entity.iconColor,
      })
      .returning({ id: Entities.id })
      .then(firstOrThrow);

    const newDocument = await tx
      .insert(Documents)
      .values({
        entityId: newEntity.id,
        title,
        subtitle: document.subtitle,
      })
      .returning()
      .then(firstOrThrow);

    // TODO: anchors

    await insertFreshV2Content(tx, newDocument.id, v2Content);

    if (noteRows.length > 0) {
      let prevOrder: string | null = null;

      for (const row of noteRows) {
        const order = generateFractionalOrder({ lower: prevOrder, upper: null });

        const newNote = await tx
          .insert(Notes)
          .values({
            userId: args.userId,
            siteId: entity.siteId,
            content: row.content,
            color: row.color,
            status: row.status,
            order,
          })
          .returning({ id: Notes.id })
          .then(firstOrThrow);

        await tx.insert(NoteEntities).values({
          noteId: newNote.id,
          entityId: newEntity.id,
        });

        prevOrder = order;
      }
    }

    return newDocument;
  });

  await runAfterCommit(afterCommit, async () => {
    if (entity.parentEntityId) {
      pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: entity.parentEntityId });
    } else {
      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
    }
    publishRecentDocumentUpdates(entity.siteId, 'UPDATED_AT');
    pubsub.publish('user:usage:update', args.userId, null);

    const { enqueueJob } = await import('#/mq/index.ts');
    await enqueueJob('search:index:document', newDocument.id);
  });

  return newDocument;
};

type RenameFolderCoreArgs = {
  userId: string;
  folderId: string;
  name: string;
};

export const renameFolderCore = async (executor: Database | Transaction, args: RenameFolderCoreArgs, afterCommit?: PostCommitRegistrar) => {
  const folder = await executor
    .select({ siteId: Entities.siteId, parentId: Entities.parentId })
    .from(Folders)
    .innerJoin(Entities, eq(Folders.entityId, Entities.id))
    .where(eq(Folders.id, args.folderId))
    .then(firstOrThrow);

  await assertSitePermission({
    userId: args.userId,
    siteId: folder.siteId,
  });

  await assertActiveSubscription({ userId: args.userId });

  const renamedFolder = await executor
    .update(Folders)
    .set({ name: args.name })
    .where(eq(Folders.id, args.folderId))
    .returning()
    .then(firstOrThrow);

  await runAfterCommit(afterCommit, async () => {
    if (folder.parentId) {
      pubsub.publish('site:update', folder.siteId, { scope: 'entity', entityId: folder.parentId });
    } else {
      pubsub.publish('site:update', folder.siteId, { scope: 'site' });
    }

    const { enqueueJob } = await import('#/mq/index.ts');
    await enqueueJob('search:index:folder', args.folderId);
  });

  return renamedFolder;
};

type UpdateDocumentCoreArgs = {
  userId: string;
  documentId: string;
  title?: string | null;
  subtitle?: string | null;
};

export const updateDocumentCore = async (
  executor: Database | Transaction,
  args: UpdateDocumentCoreArgs,
  afterCommit?: PostCommitRegistrar,
) => {
  const document = await executor
    .select({ entityId: Documents.entityId, siteId: Entities.siteId, availability: Entities.availability })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(eq(Documents.id, args.documentId))
    .then(firstOrThrow);

  if (document.availability === EntityAvailability.PRIVATE) {
    await assertSitePermission({ userId: args.userId, siteId: document.siteId });
  }

  await assertActiveSubscription({ userId: args.userId });

  const updatedDocument = await executor
    .update(Documents)
    .set({
      ...(args.title !== undefined && { title: args.title }),
      ...(args.subtitle !== undefined && { subtitle: args.subtitle }),
      updatedAt: dayjs(),
    })
    .where(eq(Documents.id, args.documentId))
    .returning()
    .then(firstOrThrow);

  await runAfterCommit(afterCommit, async () => {
    pubsub.publish('site:update', document.siteId, { scope: 'entity', entityId: document.entityId });
    publishRecentDocumentUpdates(document.siteId, 'UPDATED_AT');

    const { enqueueJob } = await import('#/mq/index.ts');
    await enqueueJob('search:index:document', args.documentId);
  });

  return updatedDocument;
};

type MoveEntitiesCoreArgs = {
  userId: string;
  entityIds: string[];
  parentEntityId: string | null;
  lowerOrder: string | null;
  upperOrder: string | null;
  targetSiteId: string | null;
};

export const moveEntitiesCore = async (executor: Database | Transaction, args: MoveEntitiesCoreArgs, afterCommit?: PostCommitRegistrar) => {
  const entities = await executor.execute<{ id: string; site_id: string; depth: number; parent_id: string | null }>(sql`
    WITH RECURSIVE descendants AS (
      SELECT ${Entities.id}
      FROM ${Entities}
      WHERE ${inArray(Entities.parentId, args.entityIds)}
      UNION ALL
      SELECT ${Entities.id}
      FROM ${Entities}
      JOIN descendants ON ${Entities.parentId} = descendants.id
    )
    SELECT ${Entities.id}, ${Entities.siteId}, ${Entities.depth}, ${Entities.parentId}
    FROM ${Entities}
    WHERE ${inArray(Entities.id, args.entityIds)}
    AND ${eq(Entities.state, EntityState.ACTIVE)}
    AND ${Entities.id} NOT IN (SELECT id FROM descendants)
    ORDER BY ${Entities.order} ASC
  `);

  if (entities.length === 0) {
    return [];
  }

  const siteId = entities[0].site_id;

  await assertSitePermission({
    userId: args.userId,
    siteId,
  });

  await assertActiveSubscription({ userId: args.userId });

  if (entities.some((entity) => entity.site_id !== siteId)) {
    throw new TypieError({ code: 'site_mismatch' });
  }

  const isCrossSite = !!(args.targetSiteId && args.targetSiteId !== siteId);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const targetSiteId = isCrossSite ? args.targetSiteId! : siteId;

  if (isCrossSite) {
    await assertSitePermission({
      userId: args.userId,
      siteId: targetSiteId,
    });
  }

  const targetParentId: string | null = args.parentEntityId ?? null;
  let targetDepth = 0;

  if (targetParentId) {
    const parentEntity = await executor
      .select({ depth: Entities.depth, siteId: Entities.siteId })
      .from(Entities)
      .where(and(eq(Entities.id, targetParentId), eq(Entities.state, EntityState.ACTIVE)))
      .then(firstOrThrowWith(new NotFoundError()));

    if (parentEntity.siteId !== targetSiteId) {
      throw new TypieError({ code: 'site_mismatch' });
    }

    if (args.entityIds.includes(targetParentId)) {
      throw new TypieError({ code: 'circular_reference' });
    }

    if (!isCrossSite) {
      const [hasCycle] = await executor.execute<{ exists: boolean }>(
        sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id}, ${Entities.parentId}
            FROM ${Entities}
            WHERE ${eq(Entities.id, targetParentId)}
            UNION ALL
            SELECT ${Entities.id}, ${Entities.parentId}
            FROM ${Entities}
            JOIN sq ON ${Entities.id} = sq.parent_id
          )
          SELECT EXISTS (
            SELECT 1 FROM sq WHERE ${inArray(sql`id`, args.entityIds)}
          ) as exists
        `,
      );

      if (hasCycle.exists) {
        throw new TypieError({ code: 'circular_reference' });
      }
    }

    targetDepth = parentEntity.depth + 1;
  }

  const sourceParentIds = new Set(
    entities.map((entity) => entity.parent_id).filter((id): id is string => id !== null && id !== targetParentId),
  );
  const hasRootSourceEntity = entities.some((entity) => entity.parent_id === null);

  const movedEntities = await executor.transaction(async (tx) => {
    const movedEntities: (typeof Entities.$inferSelect | string)[] = [];
    let lastOrder = args.lowerOrder ?? null;

    for (const entity of entities) {
      const depthDelta = targetDepth - entity.depth;

      const order = generateFractionalOrder({
        lower: lastOrder,
        upper: args.upperOrder ?? null,
      });

      const movedEntity = await tx
        .update(Entities)
        .set({
          ...(isCrossSite && { siteId: targetSiteId, pinnedOrder: null }),
          parentId: targetParentId,
          depth: targetDepth,
          order,
        })
        .where(eq(Entities.id, entity.id))
        .returning()
        .then(firstOrThrow);

      movedEntities.push(movedEntity);
      lastOrder = order;

      // 자손 업데이트 (depth 변경 또는 cross-site siteId 변경)
      if (depthDelta !== 0 || isCrossSite) {
        movedEntities.push(
          ...(await tx
            .execute<{ id: string }>(
              sql`
                WITH RECURSIVE sq AS (
                  SELECT ${Entities.id}
                  FROM ${Entities}
                  WHERE ${eq(Entities.parentId, entity.id)}
                  UNION ALL
                  SELECT ${Entities.id}
                  FROM ${Entities}
                  JOIN sq ON ${Entities.parentId} = sq.id
                )
                UPDATE ${Entities}
                SET ${sql.raw(
                  [
                    isCrossSite ? `site_id = '${targetSiteId}'` : null,
                    isCrossSite ? 'pinned_order = NULL' : null,
                    depthDelta === 0 ? null : `depth = depth + ${depthDelta}`,
                  ]
                    .filter(Boolean)
                    .join(', '),
                )}
                WHERE id IN (SELECT id FROM sq)
                RETURNING ${Entities.id}
              `,
            )
            .then((result) => result.map(({ id }) => id))),
        );
      }
    }

    for (const parentId of sourceParentIds) {
      movedEntities.push(parentId);
    }

    return movedEntities;
  });

  await runAfterCommit(afterCommit, () => {
    if (targetParentId) {
      pubsub.publish('site:update', targetSiteId, { scope: 'entity', entityId: targetParentId });
    } else {
      pubsub.publish('site:update', targetSiteId, { scope: 'site' });
    }

    for (const parentId of sourceParentIds) {
      pubsub.publish('site:update', siteId, { scope: 'entity', entityId: parentId });
    }

    if (hasRootSourceEntity && (targetParentId || isCrossSite)) {
      pubsub.publish('site:update', siteId, { scope: 'site' });
    }

    for (const entity of entities) {
      pubsub.publish('site:update', targetSiteId, { scope: 'entity', entityId: entity.id });
    }
    if (isCrossSite) {
      publishRecentDocumentUpdates(siteId, 'VIEWED_AT', 'UPDATED_AT');
      publishRecentDocumentUpdates(targetSiteId, 'VIEWED_AT', 'UPDATED_AT');
    }
  });

  return movedEntities;
};

type UpdateEntityIconCoreArgs = {
  userId: string;
  entityId: string;
  icon?: string;
  iconColor?: string;
};

export const updateEntityIconCore = async (
  executor: Database | Transaction,
  args: UpdateEntityIconCoreArgs,
  afterCommit?: PostCommitRegistrar,
) => {
  const entity = await executor
    .select({ id: Entities.id, siteId: Entities.siteId, parentId: Entities.parentId })
    .from(Entities)
    .where(and(eq(Entities.id, args.entityId), eq(Entities.state, EntityState.ACTIVE)))
    .then(firstOrThrow);

  await assertSitePermission({
    userId: args.userId,
    siteId: entity.siteId,
  });

  await assertActiveSubscription({ userId: args.userId });

  const updatedEntity = await executor
    .update(Entities)
    .set({
      ...(args.icon !== undefined && { icon: args.icon }),
      ...(args.iconColor !== undefined && { iconColor: args.iconColor }),
    })
    .where(eq(Entities.id, args.entityId))
    .returning()
    .then(firstOrThrow);

  await runAfterCommit(afterCommit, () => {
    if (entity.parentId) {
      pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: entity.parentId });
    } else {
      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
    }
  });

  return updatedEntity;
};

type RecoverEntityCoreArgs = {
  userId: string;
  entityId: string;
};

export const recoverEntityCore = async (
  executor: Database | Transaction,
  args: RecoverEntityCoreArgs,
  afterCommit?: PostCommitRegistrar,
) => {
  const ParentEntities = alias(Entities, 'parent_entities');

  const entity = await executor
    .select({
      id: Entities.id,
      siteId: Entities.siteId,
      order: Entities.order,
      depth: Entities.depth,
      parentEntity: {
        id: ParentEntities.id,
        state: ParentEntities.state,
        depth: ParentEntities.depth,
      },
    })
    .from(Entities)
    .leftJoin(ParentEntities, eq(Entities.parentId, ParentEntities.id))
    .where(and(eq(Entities.id, args.entityId), eq(Entities.state, EntityState.DELETED)))
    .then(firstOrThrow);

  await assertSitePermission({
    userId: args.userId,
    siteId: entity.siteId,
  });

  await assertActiveSubscription({ userId: args.userId });

  const hasParent = entity.parentEntity?.id !== null && entity.parentEntity?.id !== undefined;
  const isParentActive = hasParent && entity.parentEntity?.state === EntityState.ACTIVE;
  const shouldReattachToRoot = hasParent && !isParentActive;

  const rootLastChildOrder = shouldReattachToRoot
    ? await executor
        .select({ order: Entities.order })
        .from(Entities)
        .where(and(eq(Entities.siteId, entity.siteId), eq(Entities.state, EntityState.ACTIVE), isNull(Entities.parentId)))
        .orderBy(desc(Entities.order))
        .limit(1)
        .then(first)
        .then((result) => result?.order ?? null)
    : null;

  const depthDelta = isParentActive
    ? // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      entity.parentEntity!.depth + 1 - entity.depth
    : shouldReattachToRoot
      ? -entity.depth
      : 0;

  const { recoveredEntity, recoveredEntityIds, hasRecoveredDocuments } = await executor.transaction(async (tx) => {
    if (shouldReattachToRoot) {
      await tx
        .update(Entities)
        .set({
          parentId: null,
          order: generateFractionalOrder({ lower: rootLastChildOrder, upper: null }),
        })
        .where(eq(Entities.id, entity.id));
    }

    const recoveredEntities = await tx.execute<{ id: string; type: EntityType }>(
      sql`
        WITH RECURSIVE sq AS (
          SELECT ${Entities.id}
          FROM ${Entities}
          WHERE ${eq(Entities.id, entity.id)}
          UNION ALL
          SELECT ${Entities.id}
          FROM ${Entities}
          JOIN sq ON ${Entities.parentId} = sq.id
        )
        UPDATE ${Entities}
        SET state = ${EntityState.ACTIVE},
        deleted_at = null,
        depth = depth + ${depthDelta}
        WHERE id IN (SELECT id FROM sq) AND ${gt(Entities.deletedAt, dayjs().subtract(30, 'days'))}
        RETURNING ${Entities.id}, ${Entities.type}
      `,
    );

    const recoveredEntityIds = recoveredEntities.map(({ id }) => id);
    if (recoveredEntityIds.length > 0) {
      await tx
        .update(Notes)
        .set({ state: NoteState.ACTIVE, updatedAt: dayjs() })
        .where(
          and(
            eq(Notes.state, NoteState.DELETED_CASCADED),
            inArray(
              Notes.id,
              tx.select({ noteId: NoteEntities.noteId }).from(NoteEntities).where(inArray(NoteEntities.entityId, recoveredEntityIds)),
            ),
          ),
        );
    }

    const recoveredEntity = await tx.select().from(Entities).where(eq(Entities.id, entity.id)).then(firstOrThrow);

    return {
      recoveredEntity,
      recoveredEntityIds,
      hasRecoveredDocuments: recoveredEntities.some(({ type }) => type === EntityType.DOCUMENT),
    };
  });

  await runAfterCommit(afterCommit, async () => {
    if (isParentActive && entity.parentEntity?.id) {
      pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: entity.parentEntity.id });
    } else {
      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
    }
    if (hasRecoveredDocuments) {
      publishRecentDocumentUpdates(entity.siteId, 'VIEWED_AT', 'UPDATED_AT');
    }
    pubsub.publish('user:usage:update', args.userId, null);

    await enqueueSearchSyncForEntityIds(recoveredEntityIds);
  });

  return recoveredEntity;
};

type UpdateDocumentsOptionCoreArgs = {
  userId: string;
  documentIds: string[];
  availability?: EntityAvailability | null;
  visibility?: EntityVisibility | null;
  password?: string | null;
  thumbnailId?: string | null;
  contentRating?: DocumentContentRating | null;
  allowReaction?: boolean | null;
  protectContent?: boolean | null;
};

export const updateDocumentsOptionCore = async (
  executor: Database | Transaction,
  args: UpdateDocumentsOptionCoreArgs,
  afterCommit?: PostCommitRegistrar,
) => {
  const documents = await executor
    .select({
      id: Documents.id,
      siteId: Entities.siteId,
      entityId: Entities.id,
      parentId: Entities.parentId,
    })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(and(eq(Entities.state, EntityState.ACTIVE), inArray(Documents.id, args.documentIds)));

  if (documents.length === 0) {
    throw new TypieError({ code: 'invalid_argument' });
  }

  const siteId = documents[0].siteId;

  await assertSitePermission({
    userId: args.userId,
    siteId,
  });

  if (documents.some((doc) => doc.siteId !== siteId)) {
    throw new TypieError({ code: 'site_mismatch' });
  }

  if (!isPrivateVisibilityOnlyInput(args) && !(await hasActiveSubscription({ userId: args.userId }))) {
    throw new TypieError({ code: 'subscription_required', status: 403 });
  }

  await executor.transaction(async (tx) => {
    if (args.availability || args.visibility) {
      await tx
        .update(Entities)
        .set({
          availability: args.availability ?? undefined,
          visibility: args.visibility ?? undefined,
        })
        .where(
          inArray(
            Entities.id,
            documents.map((doc) => doc.entityId),
          ),
        );
    }

    if (
      args.contentRating ||
      typeof args.allowReaction === 'boolean' ||
      typeof args.protectContent === 'boolean' ||
      args.password !== undefined ||
      args.thumbnailId !== undefined
    ) {
      await tx
        .update(Documents)
        .set({
          contentRating: args.contentRating ?? undefined,
          allowReaction: args.allowReaction ?? undefined,
          protectContent: args.protectContent ?? undefined,
          password: args.password,
          thumbnailId: args.thumbnailId,
        })
        .where(
          inArray(
            Documents.id,
            documents.map((doc) => doc.id),
          ),
        );
    }
  });

  await runAfterCommit(afterCommit, () => {
    for (const doc of documents) {
      pubsub.publish('site:update', siteId, { scope: 'entity', entityId: doc.entityId });
    }
  });

  return documents.map((doc) => doc.id);
};

type UpdateFolderOptionCoreArgs = {
  userId: string;
  folderId: string;
  visibility: EntityVisibility;
  thumbnailId?: string | null;
  recursive: boolean;
};

export const updateFolderOptionCore = async (
  executor: Database | Transaction,
  args: UpdateFolderOptionCoreArgs,
  afterCommit?: PostCommitRegistrar,
) => {
  const DescendantEntities = alias(Entities, 'descendant_entities');

  const { folder, siteId } = await executor
    .select({ folder: Folders, siteId: Entities.siteId })
    .from(Folders)
    .innerJoin(Entities, eq(Folders.entityId, Entities.id))
    .where(and(eq(Folders.id, args.folderId)))
    .then(firstOrThrow);

  await assertSitePermission({
    userId: args.userId,
    siteId,
  });

  await assertActiveSubscription({ userId: args.userId });

  const changedFolderEntityIds = await executor.transaction(async (tx) => {
    const changedFolderEntityIds = [folder.entityId];

    await tx.update(Entities).set({ visibility: args.visibility }).where(eq(Entities.id, folder.entityId));

    if (args.thumbnailId !== undefined) {
      await tx.update(Folders).set({ thumbnailId: args.thumbnailId }).where(eq(Folders.id, args.folderId));
    }

    if (args.recursive) {
      const descendantEntities = await tx.execute<{ id: string; type: EntityType; state: EntityState }>(
        sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id} FROM ${Entities}
            WHERE ${eq(Entities.parentId, folder.entityId)} AND ${ne(Entities.type, EntityType.DIVIDER)}
            UNION ALL
            SELECT ${Entities.id} FROM ${Entities}
            JOIN sq ON ${Entities.parentId} = sq.id
            WHERE ${ne(Entities.type, EntityType.DIVIDER)}
          )
          SELECT ${DescendantEntities.id}, ${DescendantEntities.type}, ${DescendantEntities.state}
          FROM sq
          INNER JOIN ${DescendantEntities} ON ${DescendantEntities.id} = sq.id;
        `,
      );
      const descendantEntityIds = descendantEntities.map(({ id }) => id);

      if (descendantEntityIds.length > 0) {
        await tx.update(Entities).set({ visibility: args.visibility }).where(inArray(Entities.id, descendantEntityIds));
      }

      changedFolderEntityIds.push(
        ...descendantEntities.filter(({ type, state }) => type === EntityType.FOLDER && state === EntityState.ACTIVE).map(({ id }) => id),
      );
    }

    return changedFolderEntityIds;
  });

  await runAfterCommit(afterCommit, () => {
    for (const entityId of changedFolderEntityIds) {
      pubsub.publish('site:update', siteId, { scope: 'entity', entityId });
    }
  });

  return folder.id;
};
