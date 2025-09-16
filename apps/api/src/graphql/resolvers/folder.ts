import dayjs from 'dayjs';
import { and, desc, eq, getTableColumns, inArray, isNull, sql } from 'drizzle-orm';
import { Canvases, db, Entities, first, firstOrThrow, Folders, Notes, PostContents, Posts, TableCode, validateDbId } from '@/db';
import { EntityState, EntityType, EntityVisibility, NoteState } from '@/enums';
import { TypieError } from '@/errors';
import { enqueueJob } from '@/mq';
import { pubsub } from '@/pubsub';
import { generateFractionalOrder, generatePermalink, generateSlug } from '@/utils';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Entity, EntityView, Folder, FolderView, IFolder, isTypeOf } from '../objects';

/**
 * * Types
 */

IFolder.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
  }),
});

Folder.implement({
  isTypeOf: isTypeOf(TableCode.FOLDERS),
  interfaces: [IFolder],
  fields: (t) => ({
    view: t.expose('id', { type: FolderView }),

    entity: t.expose('entityId', { type: Entity }),

    maxDescendantFoldersDepth: t.int({
      resolve: async (self) => {
        const rows = await db.execute<{ depth: number }>(
          sql`
            WITH RECURSIVE sq AS (
              SELECT ${Entities.id}, ${Entities.depth}
              FROM ${Entities}
              WHERE ${eq(Entities.id, self.entityId)}
              UNION ALL
              SELECT ${Entities.id}, ${Entities.depth}
              FROM ${Entities}
              JOIN sq ON ${Entities.parentId} = sq.id
              WHERE ${and(eq(Entities.state, EntityState.ACTIVE), eq(Entities.type, EntityType.FOLDER))}
            )
            SELECT MAX(depth) AS depth FROM sq
          `,
        );
        return rows[0]?.depth ?? 0;
      },
    }),

    characterCount: t.int({
      resolve: async (self) => {
        const rows = await db.execute<{ total: number }>(
          sql`
            WITH RECURSIVE descendant_entities AS (
              SELECT id
              FROM ${Entities}
              WHERE id = ${self.entityId}
              UNION ALL
              SELECT e.id
              FROM ${Entities} e
              JOIN descendant_entities de ON e.parent_id = de.id
              WHERE e.state = ${EntityState.ACTIVE}
            )
            SELECT COALESCE(SUM(pc.character_count), 0) AS total
            FROM descendant_entities de
            JOIN ${Posts} p ON p.entity_id = de.id
            JOIN ${PostContents} pc ON pc.post_id = p.id
            JOIN ${Entities} e ON e.id = p.entity_id
            WHERE e.state = ${EntityState.ACTIVE}
          `,
        );
        return rows[0]?.total || 0;
      },
    }),

    folderCount: t.int({
      resolve: async (self) => {
        const rows = await db.execute<{ count: number }>(
          sql`
            WITH RECURSIVE descendant_entities AS (
              SELECT id, type
              FROM ${Entities}
              WHERE parent_id = ${self.entityId}
              AND state = ${EntityState.ACTIVE}
              UNION ALL
              SELECT e.id, e.type
              FROM ${Entities} e
              JOIN descendant_entities de ON e.parent_id = de.id
              WHERE e.state = ${EntityState.ACTIVE}
            )
            SELECT COUNT(*) AS count
            FROM descendant_entities
            WHERE type = ${EntityType.FOLDER}
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),

    postCount: t.int({
      resolve: async (self) => {
        const rows = await db.execute<{ count: number }>(
          sql`
            WITH RECURSIVE descendant_entities AS (
              SELECT id, type
              FROM ${Entities}
              WHERE parent_id = ${self.entityId}
              AND state = ${EntityState.ACTIVE}
              UNION ALL
              SELECT e.id, e.type
              FROM ${Entities} e
              JOIN descendant_entities de ON e.parent_id = de.id
              WHERE e.state = ${EntityState.ACTIVE}
            )
            SELECT COUNT(*) AS count
            FROM descendant_entities
            WHERE type = ${EntityType.POST}
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),

    canvasCount: t.int({
      resolve: async (self) => {
        const rows = await db.execute<{ count: number }>(
          sql`
            WITH RECURSIVE descendant_entities AS (
              SELECT id, type
              FROM ${Entities}
              WHERE parent_id = ${self.entityId}
              AND state = ${EntityState.ACTIVE}
              UNION ALL
              SELECT e.id, e.type
              FROM ${Entities} e
              JOIN descendant_entities de ON e.parent_id = de.id
              WHERE e.state = ${EntityState.ACTIVE}
            )
            SELECT COUNT(*) AS count
            FROM descendant_entities
            WHERE type = ${EntityType.CANVAS}
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),
  }),
});

FolderView.implement({
  isTypeOf: isTypeOf(TableCode.FOLDERS),
  interfaces: [IFolder],
  fields: (t) => ({
    entity: t.expose('entityId', { type: EntityView }),

    folderCount: t.int({
      resolve: async (self) => {
        const rows = await db.execute<{ count: number }>(
          sql`
            WITH RECURSIVE descendant_entities AS (
              SELECT id, type, visibility
              FROM ${Entities}
              WHERE parent_id = ${self.entityId}
              AND state = ${EntityState.ACTIVE}
              UNION ALL
              SELECT e.id, e.type, e.visibility
              FROM ${Entities} e
              JOIN descendant_entities de ON e.parent_id = de.id
              WHERE e.state = ${EntityState.ACTIVE}
            )
            SELECT COUNT(*) AS count
            FROM descendant_entities
            WHERE type = ${EntityType.FOLDER}
            AND visibility = ${EntityVisibility.UNLISTED}
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),

    postCount: t.int({
      resolve: async (self) => {
        const rows = await db.execute<{ count: number }>(
          sql`
            WITH RECURSIVE descendant_entities AS (
              SELECT id, type, visibility
              FROM ${Entities}
              WHERE parent_id = ${self.entityId}
              AND state = ${EntityState.ACTIVE}
              UNION ALL
              SELECT e.id, e.type, e.visibility
              FROM ${Entities} e
              JOIN descendant_entities de ON e.parent_id = de.id
              WHERE e.state = ${EntityState.ACTIVE}
            )
            SELECT COUNT(*) AS count
            FROM descendant_entities
            WHERE type = ${EntityType.POST}
            AND visibility = ${EntityVisibility.UNLISTED}
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  folder: t.withAuth({ session: true }).field({
    type: Folder,
    args: { id: t.arg.id({ validate: validateDbId(TableCode.FOLDERS) }) },
    resolve: async (_, args, ctx) => {
      const folder = await db
        .select({ siteId: Entities.siteId })
        .from(Folders)
        .innerJoin(Entities, eq(Folders.entityId, Entities.id))
        .where(eq(Folders.id, args.id))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: folder.siteId,
      });

      return args.id;
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  createFolder: t.withAuth({ session: true }).fieldWithInput({
    type: Folder,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      parentEntityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      name: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
      });

      let depth = 0;
      if (input.parentEntityId) {
        const parentEntity = await db
          .select({ id: Entities.id, depth: Entities.depth })
          .from(Entities)
          .where(
            and(
              eq(Entities.siteId, input.siteId),
              eq(Entities.id, input.parentEntityId),
              eq(Entities.type, EntityType.FOLDER),
              eq(Entities.state, EntityState.ACTIVE),
            ),
          )
          .then(firstOrThrow);

        depth = parentEntity.depth + 1;
      }

      const last = await db
        .select({ order: Entities.order })
        .from(Entities)
        .where(
          and(
            eq(Entities.siteId, input.siteId),
            input.parentEntityId ? eq(Entities.parentId, input.parentEntityId) : isNull(Entities.parentId),
          ),
        )
        .orderBy(desc(Entities.order))
        .limit(1)
        .then(first);

      const folder = await db.transaction(async (tx) => {
        const entity = await tx
          .insert(Entities)
          .values({
            userId: ctx.session.userId,
            siteId: input.siteId,
            parentId: input.parentEntityId,
            slug: generateSlug(),
            permalink: generatePermalink(),
            type: EntityType.FOLDER,
            order: generateFractionalOrder({ lower: last?.order, upper: null }),
            depth,
          })
          .returning({ id: Entities.id })
          .then(firstOrThrow);

        const folder = await tx
          .insert(Folders)
          .values({
            entityId: entity.id,
            name: input.name,
          })
          .returning()
          .then(firstOrThrow);

        return folder;
      });

      pubsub.publish('site:update', input.siteId, { scope: 'site' });

      return folder;
    },
  }),

  renameFolder: t.withAuth({ session: true }).fieldWithInput({
    type: Folder,
    input: {
      folderId: t.input.id({ validate: validateDbId(TableCode.FOLDERS) }),
      name: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const folder = await db
        .select({ siteId: Entities.siteId })
        .from(Folders)
        .innerJoin(Entities, eq(Folders.entityId, Entities.id))
        .where(eq(Folders.id, input.folderId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: folder.siteId,
      });

      const renamedFolder = await db
        .update(Folders)
        .set({ name: input.name })
        .where(eq(Folders.id, input.folderId))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('site:update', folder.siteId, { scope: 'site' });

      return renamedFolder;
    },
  }),

  deleteFolder: t.withAuth({ session: true }).fieldWithInput({
    type: Folder,
    input: { folderId: t.input.id({ validate: validateDbId(TableCode.FOLDERS) }) },
    resolve: async (_, { input }, ctx) => {
      const folder = await db
        .select({
          id: Folders.id,
          entityId: Entities.id,
          siteId: Entities.siteId,
        })
        .from(Folders)
        .innerJoin(Entities, eq(Folders.entityId, Entities.id))
        .where(eq(Folders.id, input.folderId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: folder.siteId,
      });

      const descendants = await db.execute<{ id: string; type: EntityType }>(
        sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id}, ${Entities.type} FROM ${Entities} WHERE ${eq(Entities.parentId, folder.entityId)}
            UNION ALL
            SELECT ${Entities.id}, ${Entities.type} FROM ${Entities}
            JOIN sq ON ${Entities.parentId} = sq.id
          )
          SELECT id, type FROM sq;
        `,
      );

      const entityIds = [folder.entityId, ...descendants.map(({ id }) => id)];

      await db.transaction(async (tx) => {
        await tx.update(Entities).set({ state: EntityState.DELETED, deletedAt: dayjs() }).where(inArray(Entities.id, entityIds));

        await tx
          .update(Notes)
          .set({ state: NoteState.DELETED_CASCADED })
          .where(and(inArray(Notes.entityId, entityIds), eq(Notes.state, NoteState.ACTIVE)));
      });

      pubsub.publish('site:update', folder.siteId, { scope: 'site' });
      for (const entityId of entityIds) {
        pubsub.publish('site:update', folder.siteId, { scope: 'entity', entityId });
      }

      const deletedPosts = await db
        .select({ id: Posts.id })
        .from(Posts)
        .where(
          inArray(
            Posts.entityId,
            descendants.filter(({ type }) => type === EntityType.POST).map(({ id }) => id),
          ),
        );

      const deletedCanvases = await db
        .select({ id: Canvases.id })
        .from(Canvases)
        .where(
          inArray(
            Canvases.entityId,
            descendants.filter(({ type }) => type === EntityType.CANVAS).map(({ id }) => id),
          ),
        );

      for (const post of deletedPosts) {
        await enqueueJob('post:index', post.id);
      }

      for (const canvas of deletedCanvases) {
        await enqueueJob('canvas:index', canvas.id);
      }

      return folder.id;
    },
  }),

  updateFolderOption: t.withAuth({ session: true }).fieldWithInput({
    type: Folder,
    input: {
      folderId: t.input.id({ validate: validateDbId(TableCode.FOLDERS) }),
      visibility: t.input.field({ type: EntityVisibility }),
      recursive: t.input.boolean({ required: false, defaultValue: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const { folder, siteId } = await db
        .select({ folder: Folders, siteId: Entities.siteId })
        .from(Folders)
        .innerJoin(Entities, eq(Folders.entityId, Entities.id))
        .where(and(eq(Folders.id, input.folderId)))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId,
      });

      await db.transaction(async (tx) => {
        await tx.update(Entities).set({ visibility: input.visibility }).where(eq(Entities.id, folder.entityId));

        if (input.recursive) {
          const descendantEntityIds = await tx
            .execute<{ id: string }>(
              sql`
                WITH RECURSIVE sq AS (
                  SELECT ${Entities.id} FROM ${Entities} WHERE ${eq(Entities.parentId, folder.entityId)}
                  UNION ALL
                  SELECT ${Entities.id} FROM ${Entities}
                  JOIN sq ON ${Entities.parentId} = sq.id
                )
                SELECT id FROM sq;
              `,
            )
            .then((rows) => rows.map(({ id }) => id));

          if (descendantEntityIds.length > 0) {
            await tx.update(Entities).set({ visibility: input.visibility }).where(inArray(Entities.id, descendantEntityIds));
          }
        }
      });

      return folder;
    },
  }),

  updateFoldersOption: t.withAuth({ session: true }).fieldWithInput({
    type: t.listRef(Folder),
    input: {
      folderIds: t.input.idList({ validate: { items: validateDbId(TableCode.FOLDERS) } }),
      visibility: t.input.field({ type: EntityVisibility, required: false }),
      recursive: t.input.boolean({ required: false, defaultValue: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const folders = await db
        .select({
          ...getTableColumns(Folders),
          siteId: Entities.siteId,
        })
        .from(Folders)
        .innerJoin(Entities, eq(Folders.entityId, Entities.id))
        .where(and(inArray(Folders.id, input.folderIds), eq(Entities.state, EntityState.ACTIVE)));

      if (folders.length === 0) {
        throw new TypieError({ code: 'invalid_argument' });
      }

      const siteId = folders[0].siteId;

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId,
      });

      if (folders.some((folder) => folder.siteId !== siteId)) {
        throw new TypieError({ code: 'site_mismatch' });
      }

      if (!input.visibility) {
        return folders;
      }

      const updatedEntities = await db.transaction(async (tx) => {
        const entityIds = folders.map((folder) => folder.entityId);

        const updatedEntities = await tx
          .update(Entities)
          .set({ visibility: input.visibility ?? undefined })
          .where(inArray(Entities.id, entityIds))
          .returning({ id: Entities.id });

        if (input.recursive) {
          const descendantEntityIds = await tx
            .execute<{ id: string }>(
              sql`
                WITH RECURSIVE sq AS (
                  SELECT ${Entities.id} FROM ${Entities} WHERE ${inArray(Entities.parentId, entityIds)} AND ${eq(Entities.state, EntityState.ACTIVE)}
                  UNION ALL
                  SELECT ${Entities.id} FROM ${Entities}
                  JOIN sq ON ${Entities.parentId} = sq.id
                  WHERE ${eq(Entities.state, EntityState.ACTIVE)}
                )
                SELECT id FROM sq;
              `,
            )
            .then((rows) => rows.map(({ id }) => id));

          if (descendantEntityIds.length > 0) {
            // visibility는 not null이므로 null이여도 undefined로 취급
            const updatedDescendantEntities = await tx
              .update(Entities)
              .set({ visibility: input.visibility ?? undefined })
              .where(inArray(Entities.id, descendantEntityIds))
              .returning({ id: Entities.id });

            updatedEntities.push(...updatedDescendantEntities);
          }
        }

        return updatedEntities;
      });

      pubsub.publish('site:update', siteId, { scope: 'site' });
      for (const entity of updatedEntities) {
        pubsub.publish('site:update', siteId, { scope: 'entity', entityId: entity.id });
      }

      return folders;
    },
  }),
}));
