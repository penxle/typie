import { and, desc, eq, inArray, isNull, sql } from 'drizzle-orm';
import { db, Entities, first, firstOrThrow, Folders, TableCode, validateDbId } from '@/db';
import { EntityState, EntityType, EntityVisibility } from '@/enums';
import { pubsub } from '@/pubsub';
import { generateEntityOrder, generatePermalink, generateSlug } from '@/utils';
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
        return rows[0].depth;
      },
    }),
  }),
});

FolderView.implement({
  isTypeOf: isTypeOf(TableCode.FOLDERS),
  interfaces: [IFolder],
  fields: (t) => ({
    entity: t.expose('entityId', { type: EntityView }),
  }),
});

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
            order: generateEntityOrder({ lower: last?.order, upper: null }),
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

      const descendants = await db.execute<{ id: string }>(
        sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id} FROM ${Entities} WHERE ${eq(Entities.parentId, folder.entityId)}
            UNION ALL
            SELECT ${Entities.id} FROM ${Entities}
            JOIN sq ON ${Entities.parentId} = sq.id
          )
          SELECT id FROM sq;
        `,
      );

      const entityIds = [folder.entityId, ...descendants.map(({ id }) => id)];

      await db.update(Entities).set({ state: EntityState.DELETED }).where(inArray(Entities.id, entityIds));

      pubsub.publish('site:update', folder.siteId, { scope: 'site' });
      for (const entityId of entityIds) {
        pubsub.publish('site:update', folder.siteId, { scope: 'entity', entityId });
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
}));
