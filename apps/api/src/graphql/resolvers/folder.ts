import { faker } from '@faker-js/faker';
import { and, desc, eq, inArray, isNull, sql } from 'drizzle-orm';
import { db, Entities, first, firstOrThrow, FolderOptions, Folders, TableCode, validateDbId } from '@/db';
import { EntityState, EntityType, FolderVisibility } from '@/enums';
import { pubsub } from '@/pubsub';
import { generateEntityOrder } from '@/utils';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Entity, EntityView, Folder, FolderOption, FolderOptionView, FolderView, IFolder, IFolderOption, isTypeOf } from '../objects';

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

    option: t.field({
      type: FolderOption,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Folder.option',
          load: async (ids) => {
            return await db.select().from(FolderOptions).where(inArray(FolderOptions.folderId, ids));
          },
          key: ({ folderId }) => folderId,
        });

        return await loader.load(self.id);
      },
    }),
  }),
});

FolderView.implement({
  isTypeOf: isTypeOf(TableCode.FOLDERS),
  interfaces: [IFolder],
  fields: (t) => ({
    entity: t.expose('entityId', { type: EntityView }),

    option: t.field({
      type: FolderOptionView,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'FolderView.option',
          load: async (ids) => {
            return await db.select().from(FolderOptions).where(inArray(FolderOptions.folderId, ids));
          },
          key: ({ folderId }) => folderId,
        });

        return await loader.load(self.id);
      },
    }),
  }),
});

IFolderOption.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    visibility: t.expose('visibility', { type: FolderVisibility }),
  }),
});

FolderOption.implement({
  isTypeOf: isTypeOf(TableCode.FOLDER_OPTIONS),
  interfaces: [IFolderOption],
});

FolderOptionView.implement({
  isTypeOf: isTypeOf(TableCode.FOLDER_OPTIONS),
  interfaces: [IFolderOption],
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

      if (input.parentEntityId) {
        await db
          .select({ id: Entities.id })
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
            slug: faker.string.hexadecimal({ length: 32, casing: 'lower', prefix: '' }),
            permalink: faker.string.alphanumeric({ length: 6, casing: 'mixed' }),
            type: EntityType.FOLDER,
            order: generateEntityOrder({ lower: last?.order, upper: null }),
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

        await tx.insert(FolderOptions).values({
          folderId: folder.id,
        });

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

      await db.transaction(async (tx) => {
        await tx
          .update(Entities)
          .set({ state: EntityState.DELETED })
          .where(inArray(Entities.id, [folder.entityId, ...descendants.map(({ id }) => id)]));
      });

      pubsub.publish('site:update', folder.siteId, { scope: 'site' });

      return folder.id;
    },
  }),

  updateFolderOption: t.withAuth({ session: true }).fieldWithInput({
    type: FolderOption,
    input: {
      folderId: t.input.id({ validate: validateDbId(TableCode.FOLDERS) }),
      visibility: t.input.field({ type: FolderVisibility }),
    },
    resolve: async (_, { input }, ctx) => {
      const folder = await db
        .select({ siteId: Entities.siteId })
        .from(Folders)
        .innerJoin(Entities, eq(Folders.entityId, Entities.id))
        .where(and(eq(Folders.id, input.folderId)))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: folder.siteId,
      });

      return await db
        .update(FolderOptions)
        .set({ visibility: input.visibility })
        .where(eq(FolderOptions.folderId, input.folderId))
        .returning()
        .then(firstOrThrow);
    },
  }),
}));
