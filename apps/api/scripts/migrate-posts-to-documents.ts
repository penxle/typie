#!/usr/bin/env tsx

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
  pg,
  PostAnchors,
  PostContents,
  Posts,
} from '@/db';
import { EntityType, NoteState } from '@/enums';
import { extractLoroDocContents, generateFractionalOrder, generatePermalink, generateSlug } from '@/utils';
import { compressZstd } from '@/utils/compression';
import { convertPostToDocumentJson } from '@/utils/convert';
import { wasm } from '@/utils/wasm';

process.env.SCRIPT = 'true';

const CONCURRENCY = 10;

await (async () => {
  console.log(`Starting post → document migration... (concurrency: ${CONCURRENCY})`);

  const posts = await db
    .select({
      id: Posts.id,
      entityId: Posts.entityId,
    })
    .from(Posts)
    .where(isNull(Posts.documentId));

  console.log(`Found ${posts.length} posts to migrate`);

  let migrated = 0;
  let skipped = 0;
  let errors = 0;
  let processed = 0;
  const startTime = Date.now();

  function formatEta() {
    if (processed === 0) return '';
    const elapsed = Date.now() - startTime;
    const avgMs = elapsed / processed;
    const remaining = (posts.length - processed) * avgMs;
    const remainingSec = Math.ceil(remaining / 1000);
    if (remainingSec < 60) return `${remainingSec}s remaining`;
    if (remainingSec < 3600) {
      const min = Math.floor(remainingSec / 60);
      const sec = remainingSec % 60;
      return `${min}m${sec}s remaining`;
    }
    const hr = Math.floor(remainingSec / 3600);
    const min = Math.floor((remainingSec % 3600) / 60);
    const sec = remainingSec % 60;
    return `${hr}h${min}m${sec}s remaining`;
  }

  async function migratePost({ id: postId, entityId: postEntityId }: { id: string; entityId: string }) {
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

      if (!post) {
        skipped++;
        return;
      }

      // Double-check idempotency (another worker may have processed this)
      if (post.documentId) {
        skipped++;
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
        .where(eq(Entities.id, postEntityId))
        .then(first);

      if (!entity) {
        console.error(`[${postId}] Entity not found: ${postEntityId}`);
        errors++;
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
        console.error(`[${postId}] PostContents not found`);
        errors++;
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

      await db.transaction(async (tx) => {
        // Lock per-user notes ordering inside the transaction to prevent race conditions
        // when concurrent workers process posts owned by the same user
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
      });

      migrated++;
    } catch (err) {
      console.error(`Error migrating post ${postId}:`, err);
      errors++;
    } finally {
      processed++;
      if (processed % 100 === 0 || processed === posts.length) {
        console.log(`[${processed}/${posts.length}] migrated=${migrated} skipped=${skipped} errors=${errors} (${formatEta()})`);
      }
    }
  }

  const pool = new Set<Promise<void>>();
  for (const item of posts) {
    const promise: Promise<void> = migratePost(item).then(() => {
      pool.delete(promise);
    });
    pool.add(promise);
    if (pool.size >= CONCURRENCY) {
      await Promise.race(pool);
    }
  }
  await Promise.all(pool);

  console.log(`Migration complete. Migrated: ${migrated}, Skipped: ${skipped}, Errors: ${errors}`);

  await pg.end();
  process.exit(0);
})();
