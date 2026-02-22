import dayjs from 'dayjs';
import { and, asc, desc, eq, getTableColumns, gt, isNull, sql } from 'drizzle-orm';
import { LoroDoc } from 'loro-crdt';
import * as Y from 'yjs';
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
  Folders,
  Notes,
  PostAnchors,
  PostContents,
  Posts,
  TableCode,
  validateDbId,
} from '@/db';
import { EntityState, EntityType, EntityVisibility, NoteState } from '@/enums';
import { enqueueJob } from '@/mq';
import { schema } from '@/pm';
import { pubsub } from '@/pubsub';
import {
  extractLoroDocContents,
  generateActivityImage,
  generateFractionalOrder,
  generatePermalink,
  generateRandomName,
  generateSlug,
  makeYDoc,
} from '@/utils';
import { compressZstd } from '@/utils/compression';
import { convertPostToDocumentJson } from '@/utils/convert';
import { assertSitePermission } from '@/utils/permission';
import { assertPlanRule } from '@/utils/plan';
import { wasm } from '@/utils/wasm';
import { builder } from '../builder';
import { Document, PostView } from '../objects';

/**
 * * Queries
 */

builder.queryFields((t) => ({
  seed: t.field({
    type: 'Float',
    resolve: () => {
      return Math.random();
    },
  }),

  randomName: t.field({
    type: 'String',
    resolve: () => {
      return generateRandomName();
    },
  }),

  welcome: t.field({
    type: builder.simpleObject('Welcome', {
      fields: (t) => ({
        body: t.field({ type: 'JSON' }),
        update: t.field({ type: 'Binary' }),
        name: t.string(),
        bodyMobile: t.field({ type: 'JSON' }),
        updateMobile: t.field({ type: 'Binary' }),
      }),
    }),
    resolve: async () => {
      const content = await db
        .select({ body: PostContents.body })
        .from(PostContents)
        .where(eq(PostContents.postId, 'P0WELCOME'))
        .then(first);

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const body = content?.body ?? schema.topNodeType.createAndFill()!.toJSON();

      const yDoc = makeYDoc({ body });
      const update = Y.encodeStateAsUpdateV2(yDoc);

      const name = generateRandomName();

      const contentMobile = await db
        .select({ body: PostContents.body })
        .from(PostContents)
        .where(eq(PostContents.postId, 'P0WELCOMEMOBILE'))
        .then(first);

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const bodyMobile = contentMobile?.body ?? schema.topNodeType.createAndFill()!.toJSON();

      const yDocMobile = makeYDoc({ body: bodyMobile });
      const updateMobile = Y.encodeStateAsUpdateV2(yDocMobile);

      return {
        body,
        update,
        name,
        bodyMobile,
        updateMobile,
      };
    },
  }),

  announcements: t.field({
    type: [PostView],
    resolve: async () => {
      const folder = await db.select({ entityId: Folders.entityId }).from(Folders).where(eq(Folders.id, 'F0ANNOUNCEMENTS')).then(first);
      if (!folder) {
        return [];
      }

      return await db
        .select(getTableColumns(Posts))
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(
          and(
            eq(Entities.parentId, folder.entityId),
            eq(Entities.state, EntityState.ACTIVE),
            eq(Entities.visibility, EntityVisibility.UNLISTED),
          ),
        )
        .orderBy(asc(Entities.order));
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  generateRandomName: t.field({
    type: 'String',
    resolve: () => {
      return generateRandomName();
    },
  }),

  generateActivityImage: t.withAuth({ session: true }).field({
    type: 'Binary',
    resolve: async (_, __, ctx) => {
      return await generateActivityImage(ctx.session.userId);
    },
  }),

  convertPostToDocument: t.withAuth({ session: true }).fieldWithInput({
    type: Document,
    input: { postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }) },
    resolve: async (_, { input }, ctx) => {
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
        })
        .from(Posts)
        .where(eq(Posts.id, input.postId))
        .then(firstOrThrow);

      const postContents = await db
        .select({
          body: PostContents.body,
          layoutMode: PostContents.layoutMode,
          pageLayout: PostContents.pageLayout,
        })
        .from(PostContents)
        .where(eq(PostContents.postId, post.id))
        .then(firstOrThrow);

      const entity = await db
        .select({
          id: Entities.id,
          userId: Entities.userId,
          siteId: Entities.siteId,
          parentId: Entities.parentId,
          order: Entities.order,
          depth: Entities.depth,
          visibility: Entities.visibility,
          availability: Entities.availability,
        })
        .from(Entities)
        .where(eq(Entities.id, post.entityId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

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

      await assertPlanRule({ userId: ctx.session.userId, rule: 'maxTotalCharacterCount' });
      await assertPlanRule({ userId: ctx.session.userId, rule: 'maxTotalBlobSize' });

      const anchors = await db
        .select({
          nodeId: PostAnchors.nodeId,
          name: PostAnchors.name,
          createdAt: PostAnchors.createdAt,
        })
        .from(PostAnchors)
        .where(eq(PostAnchors.postId, post.id));

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

      const notes = await db
        .select({
          content: Notes.content,
          color: Notes.color,
          order: Notes.order,
        })
        .from(Notes)
        .where(and(eq(Notes.entityId, entity.id), eq(Notes.state, NoteState.ACTIVE)))
        .orderBy(asc(Notes.order));

      const document = await db.transaction(async (tx) => {
        let lastOrder: string | null = null;
        if (notes.length > 0) {
          await tx.execute(sql`SELECT pg_advisory_xact_lock(hashtext(${ctx.session.userId}))`);

          const lastUserNote = await tx
            .select({ order: Notes.order })
            .from(Notes)
            .where(and(eq(Notes.userId, ctx.session.userId), eq(Notes.state, NoteState.ACTIVE)))
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
            userId: ctx.session.userId,
            siteId: entity.siteId,
            parentId: entity.parentId,
            slug: generateSlug(),
            permalink: generatePermalink(),
            type: EntityType.DOCUMENT,
            order: generateFractionalOrder({ lower: entity.order, upper: nextEntity?.order }),
            depth: entity.depth,
            visibility: entity.visibility,
            availability: entity.availability,
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
          userId: ctx.session.userId,
        });

        if (notes.length > 0) {
          const notesWithNewOrder = notes.map((note) => {
            const newOrder = generateFractionalOrder({
              lower: lastOrder,
              upper: null,
            });
            lastOrder = newOrder;

            return {
              userId: ctx.session.userId,
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

        return newDocument;
      });

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
      pubsub.publish('site:usage:update', entity.siteId, null);

      await enqueueJob('document:index', document.id);

      return document;
    },
  }),
}));
