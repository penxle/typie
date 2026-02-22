import dayjs from 'dayjs';
import { and, asc, desc, eq, gt, isNull, sql } from 'drizzle-orm';
import { LoroDoc } from 'loro-crdt';
import {
  db,
  DocumentArchivedNodes,
  DocumentContents,
  Documents,
  DocumentVersionContributors,
  DocumentVersions,
  Entities,
  first,
  firstOrThrow,
  Notes,
  PostAnchors,
  PostContents,
  Posts,
} from '@/db';
import { EntityState, EntityType, NoteState } from '@/enums';
import { Lock } from '@/lock';
import { extractLoroDocContents, generateFractionalOrder, generatePermalink, generateSlug } from '@/utils';
import { compressZstd } from '@/utils/compression';
import { convertPostToDocumentJson } from '@/utils/convert';
import { wasm } from '@/utils/wasm';
import { enqueueJob } from '../index';
import { defineJob } from '../types';

export const PostToDocumentMigrationJob = defineJob('post:migrate-to-document', async (postId: string) => {
  const lock = new Lock(`post:migration:${postId}`);

  const acquired = await lock.tryAcquire();
  if (!acquired) {
    return;
  }

  try {
    const post = await db
      .select({
        id: Posts.id,
        entityId: Posts.entityId,
        title: Posts.title,
        subtitle: Posts.subtitle,
        maxWidth: Posts.maxWidth,
        password: Posts.password,
        contentRating: Posts.contentRating,
        allowReaction: Posts.allowReaction,
        protectContent: Posts.protectContent,
        thumbnailId: Posts.thumbnailId,
        documentId: Posts.documentId,
      })
      .from(Posts)
      .where(eq(Posts.id, postId))
      .then(first);

    if (!post || post.documentId) {
      return;
    }

    const entity = await db
      .select({
        id: Entities.id,
        userId: Entities.userId,
        siteId: Entities.siteId,
        parentId: Entities.parentId,
        order: Entities.order,
        depth: Entities.depth,
        state: Entities.state,
        visibility: Entities.visibility,
        availability: Entities.availability,
        deletedAt: Entities.deletedAt,
        purgedAt: Entities.purgedAt,
      })
      .from(Entities)
      .where(eq(Entities.id, post.entityId))
      .then(first);

    if (!entity) {
      return;
    }

    const postContents = await db
      .select({
        body: PostContents.body,
        layoutMode: PostContents.layoutMode,
        pageLayout: PostContents.pageLayout,
      })
      .from(PostContents)
      .where(eq(PostContents.postId, postId))
      .then(first);

    if (!postContents) {
      return;
    }

    const anchors = await db
      .select({
        nodeId: PostAnchors.nodeId,
        name: PostAnchors.name,
        createdAt: PostAnchors.createdAt,
      })
      .from(PostAnchors)
      .where(eq(PostAnchors.postId, postId));

    const { json, archivedNodes } = await convertPostToDocumentJson(postContents.body, {
      maxWidth: post.maxWidth,
      layoutMode: postContents.layoutMode,
      pageLayout: postContents.pageLayout,
      anchors,
      userId: entity.userId,
    });

    await wasm.validateDocumentJson(json);

    const snapshot = await wasm.jsonToSnapshot(json);
    const doc = new LoroDoc();
    doc.import(snapshot);
    const version = doc.version().encode();
    const { json: contentJson, text, characterCount, blobSize } = await extractLoroDocContents(doc);

    const nextEntity = await db
      .select({ order: Entities.order })
      .from(Entities)
      .where(
        and(
          eq(Entities.siteId, entity.siteId),
          entity.parentId ? eq(Entities.parentId, entity.parentId) : isNull(Entities.parentId),
          gt(Entities.order, entity.order),
        ),
      )
      .orderBy(asc(Entities.order))
      .limit(1)
      .then(first);

    const notes = await db
      .select({
        content: Notes.content,
        color: Notes.color,
        order: Notes.order,
      })
      .from(Notes)
      .where(and(eq(Notes.entityId, entity.id), eq(Notes.state, NoteState.ACTIVE)))
      .orderBy(asc(Notes.order));

    const newDocument = await db.transaction(async (tx) => {
      let lastOrder: string | null = null;
      if (notes.length > 0) {
        await tx.execute(sql`SELECT pg_advisory_xact_lock(hashtext(${entity.userId}))`);

        const lastUserNote = await tx
          .select({ order: Notes.order })
          .from(Notes)
          .where(and(eq(Notes.userId, entity.userId), eq(Notes.state, NoteState.ACTIVE)))
          .orderBy(desc(Notes.order))
          .limit(1)
          .then(first);

        lastOrder = lastUserNote?.order ?? null;
      }

      if (archivedNodes.length > 0) {
        await tx.insert(DocumentArchivedNodes).values(
          archivedNodes.map((node) => ({
            id: node.id,
            content: node.content,
          })),
        );
      }

      const newEntity = await tx
        .insert(Entities)
        .values({
          userId: entity.userId,
          siteId: entity.siteId,
          parentId: entity.parentId,
          slug: generateSlug(),
          permalink: generatePermalink(),
          type: EntityType.DOCUMENT,
          order: generateFractionalOrder({ lower: entity.order, upper: nextEntity?.order }),
          depth: entity.depth,
          state: entity.state,
          visibility: entity.visibility,
          availability: entity.availability,
          ...(entity.deletedAt ? { deletedAt: entity.deletedAt } : {}),
          ...(entity.purgedAt ? { purgedAt: entity.purgedAt } : {}),
        })
        .returning({ id: Entities.id })
        .then(firstOrThrow);

      const newDocument = await tx
        .insert(Documents)
        .values({
          entityId: newEntity.id,
          title: post.title,
          subtitle: post.subtitle,
          contentRating: post.contentRating,
          allowReaction: post.allowReaction,
          protectContent: post.protectContent,
          thumbnailId: post.thumbnailId,
          password: post.password,
        })
        .returning()
        .then(firstOrThrow);

      await tx.insert(DocumentContents).values({
        documentId: newDocument.id,
        json: contentJson,
        text,
        characterCount,
        blobSize,
        snapshot,
        version,
      });

      const documentVersion = await tx
        .insert(DocumentVersions)
        .values({
          documentId: newDocument.id,
          version: await compressZstd(version),
        })
        .returning({ id: DocumentVersions.id })
        .then(firstOrThrow);

      await tx.insert(DocumentVersionContributors).values({
        versionId: documentVersion.id,
        userId: entity.userId,
      });

      if (notes.length > 0) {
        const notesWithNewOrder = notes.map((note) => {
          const newOrder = generateFractionalOrder({
            lower: lastOrder,
            upper: null,
          });
          lastOrder = newOrder;

          return {
            userId: entity.userId,
            entityId: newEntity.id,
            content: note.content,
            color: note.color,
            order: newOrder,
            createdAt: dayjs(),
            updatedAt: dayjs(),
          };
        });

        await tx.insert(Notes).values(notesWithNewOrder);
      }

      await tx.update(Posts).set({ documentId: newDocument.id }).where(eq(Posts.id, postId));

      return newDocument;
    });

    if (entity.state === EntityState.ACTIVE) {
      await enqueueJob('document:index', newDocument.id);
    }
  } finally {
    await lock.release();
  }
});
