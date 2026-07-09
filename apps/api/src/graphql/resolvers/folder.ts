import { EntityState, EntityType, EntityVisibility } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, desc, eq, getTableColumns, inArray, isNull, sql } from 'drizzle-orm';
import {
  db,
  DocumentContents,
  Documents,
  DocumentStates,
  Entities,
  first,
  firstOrThrow,
  Folders,
  TableCode,
  validateDbId,
} from '#/db/index.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import { generateFractionalOrder, generatePermalink, generateSlug } from '#/utils/index.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { builder } from '../builder.ts';
import { Entity, EntityView, Folder, FolderView, IFolder, Image, isTypeOf } from '../objects.ts';

/**
 * * Types
 */

IFolder.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    thumbnail: t.expose('thumbnailId', { type: Image, nullable: true }),
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
            SELECT COALESCE(SUM(COALESCE(ds.character_count, dc.character_count)), 0) AS total
            FROM descendant_entities de
            JOIN ${Documents} d ON d.entity_id = de.id
            JOIN ${DocumentContents} dc ON dc.document_id = d.id
            LEFT JOIN ${DocumentStates} ds ON ds.document_id = d.id
            JOIN ${Entities} e ON e.id = d.entity_id
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

    documentCount: t.int({
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
            WHERE type = ${EntityType.DOCUMENT}
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),

    postCount: t.int({
      resolve: async () => 0,
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
            AND visibility IN (${EntityVisibility.UNLISTED}, ${EntityVisibility.PUBLIC})
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),

    documentCount: t.int({
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
            WHERE type = ${EntityType.DOCUMENT}
            AND visibility IN (${EntityVisibility.UNLISTED}, ${EntityVisibility.PUBLIC})
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),

    postCount: t.int({
      resolve: async () => 0,
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
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
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

      let orderLower: string | null = input.lowerOrder ?? null;

      if (!input.lowerOrder) {
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

        orderLower = last?.order ?? null;
      }

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
            icon: 'folder',
            order: generateFractionalOrder({ lower: orderLower, upper: input.upperOrder ?? null }),
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

      if (input.parentEntityId) {
        pubsub.publish('site:update', input.siteId, { scope: 'entity', entityId: input.parentEntityId });
      } else {
        pubsub.publish('site:update', input.siteId, { scope: 'site' });
      }

      await enqueueJob('search:index:folder', folder.id);

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
        .select({ siteId: Entities.siteId, parentId: Entities.parentId })
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

      if (folder.parentId) {
        pubsub.publish('site:update', folder.siteId, { scope: 'entity', entityId: folder.parentId });
      } else {
        pubsub.publish('site:update', folder.siteId, { scope: 'site' });
      }

      await enqueueJob('search:index:folder', input.folderId);

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
          parentId: Entities.parentId,
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

      await db.update(Entities).set({ state: EntityState.DELETED, deletedAt: dayjs() }).where(inArray(Entities.id, entityIds));

      if (folder.parentId) {
        pubsub.publish('site:update', folder.siteId, { scope: 'entity', entityId: folder.parentId });
      } else {
        pubsub.publish('site:update', folder.siteId, { scope: 'site' });
      }
      for (const entityId of entityIds) {
        pubsub.publish('site:update', folder.siteId, { scope: 'entity', entityId });
      }
      pubsub.publish('user:usage:update', ctx.session.userId, null);

      const deletedDocuments = await db
        .select({ id: Documents.id })
        .from(Documents)
        .where(
          inArray(
            Documents.entityId,
            descendants.filter(({ type }) => type === EntityType.DOCUMENT).map(({ id }) => id),
          ),
        );

      for (const document of deletedDocuments) {
        await enqueueJob('search:index:document', document.id);
      }

      const deletedFolderIds = [
        folder.id,
        ...(await db
          .select({ id: Folders.id })
          .from(Folders)
          .where(
            inArray(
              Folders.entityId,
              descendants.filter(({ type }) => type === EntityType.FOLDER).map(({ id }) => id),
            ),
          )
          .then((rows) => rows.map(({ id }) => id))),
      ];

      for (const folderId of deletedFolderIds) {
        await enqueueJob('search:index:folder', folderId);
      }

      return folder.id;
    },
  }),

  updateFolderOption: t.withAuth({ session: true }).fieldWithInput({
    type: Folder,
    input: {
      folderId: t.input.id({ validate: validateDbId(TableCode.FOLDERS) }),
      visibility: t.input.field({ type: EntityVisibility }),
      thumbnailId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
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

        if (input.thumbnailId !== undefined) {
          await tx.update(Folders).set({ thumbnailId: input.thumbnailId }).where(eq(Folders.id, input.folderId));
        }

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

      return folder.id;
    },
  }),

  updateFoldersOption: t.withAuth({ session: true }).fieldWithInput({
    type: t.listRef(Folder),
    input: {
      folderIds: t.input.idList({ validate: { items: validateDbId(TableCode.FOLDERS) } }),
      visibility: t.input.field({ type: EntityVisibility, required: false }),
      thumbnailId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
      recursive: t.input.boolean({ required: false, defaultValue: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const folders = await db
        .select({
          ...getTableColumns(Folders),
          siteId: Entities.siteId,
          parentId: Entities.parentId,
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

      if (!input.visibility && input.thumbnailId === undefined) {
        return folders.map((folder) => folder.id);
      }

      const updatedEntities = await db.transaction(async (tx) => {
        const entityIds = folders.map((folder) => folder.entityId);
        const folderIds = folders.map((folder) => folder.id);

        let updatedEntities: { id: string }[] = [];

        if (input.visibility) {
          updatedEntities = await tx
            .update(Entities)
            .set({ visibility: input.visibility ?? undefined })
            .where(inArray(Entities.id, entityIds))
            .returning({ id: Entities.id });
        }

        if (input.thumbnailId !== undefined) {
          await tx.update(Folders).set({ thumbnailId: input.thumbnailId }).where(inArray(Folders.id, folderIds));
        }

        if (input.recursive && input.visibility) {
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

      for (const entity of updatedEntities) {
        pubsub.publish('site:update', siteId, { scope: 'entity', entityId: entity.id });
      }

      return folders.map((folder) => folder.id);
    },
  }),
}));
