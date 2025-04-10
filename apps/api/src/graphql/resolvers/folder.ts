import { faker } from '@faker-js/faker';
import { and, desc, eq, isNull } from 'drizzle-orm';
import { generateJitteredKeyBetween } from 'fractional-indexing-jittered';
import { db, Entities, first, firstOrThrow, Folders, TableCode } from '@/db';
import { EntityState, EntityType } from '@/enums';
import { pubsub } from '@/pubsub';
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
    entity: t.field({ type: Entity, resolve: (self) => self.entityId }),
  }),
});

FolderView.implement({
  isTypeOf: isTypeOf(TableCode.FOLDERS),
  interfaces: [IFolder],
  fields: (t) => ({
    entity: t.field({ type: EntityView, resolve: (self) => self.entityId }),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  createFolder: t.withAuth({ session: true }).fieldWithInput({
    type: Folder,
    input: {
      siteId: t.input.id(),
      parentEntityId: t.input.id({ required: false }),
      name: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
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
            order: encoder.encode(generateJitteredKeyBetween(last ? decoder.decode(last.order) : null, null)),
          })
          .returning({ id: Entities.id })
          .then(firstOrThrow);

        return await tx
          .insert(Folders)
          .values({
            entityId: entity.id,
            name: input.name,
          })
          .returning()
          .then(firstOrThrow);
      });

      pubsub.publish('site:update', input.siteId, { scope: 'site' });

      return folder;
    },
  }),

  renameFolder: t.withAuth({ session: true }).fieldWithInput({
    type: Folder,
    input: {
      id: t.input.id(),
      name: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const folder = await db
        .select({
          id: Folders.id,
          siteId: Entities.siteId,
        })
        .from(Folders)
        .innerJoin(Entities, eq(Folders.entityId, Entities.id))
        .where(and(eq(Folders.id, input.id), eq(Entities.userId, ctx.session.userId)))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: folder.siteId, ctx });

      const renamedFolder = await db
        .update(Folders)
        .set({
          name: input.name,
        })
        .where(eq(Folders.id, folder.id))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('site:update', folder.siteId, { scope: 'site' });

      return renamedFolder;
    },
  }),
}));

/**
 * * Utils
 */

const encoder = new TextEncoder();
const decoder = new TextDecoder();
