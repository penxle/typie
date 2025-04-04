import { and, eq } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { db, Entities, firstOrThrow, Folders, Posts, TableCode } from '@/db';
import { EntityState, EntityType } from '@/enums';
import { builder } from '../builder';
import { Entity, EntityNode, EntityView, EntityViewNode, IEntity, isTypeOf } from '../objects';

/**
 * * Types
 */

IEntity.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    order: t.expose('order', { type: 'Binary' }),
  }),
});

Entity.implement({
  isTypeOf: isTypeOf(TableCode.ENTITIES),
  interfaces: [IEntity],
  fields: (t) => ({
    node: t.field({
      type: EntityNode,
      resolve: async (self) => {
        return match(self.type)
          .with(EntityType.FOLDER, () => db.select().from(Folders).where(eq(Folders.entityId, self.id)).then(firstOrThrow))
          .with(EntityType.POST, () => db.select().from(Posts).where(eq(Posts.entityId, self.id)).then(firstOrThrow))
          .exhaustive();
      },
    }),
  }),
});

EntityView.implement({
  isTypeOf: isTypeOf(TableCode.ENTITIES),
  interfaces: [IEntity],
  fields: (t) => ({
    node: t.field({
      type: EntityViewNode,
      resolve: async (self) => {
        return match(self.type)
          .with(EntityType.FOLDER, () => db.select().from(Folders).where(eq(Folders.entityId, self.id)).then(firstOrThrow))
          .with(EntityType.POST, () => db.select().from(Posts).where(eq(Posts.entityId, self.id)).then(firstOrThrow))
          .exhaustive();
      },
    }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  entityView: t.field({
    type: EntityView,
    args: { slug: t.arg.string() },
    resolve: async (_, args) => {
      return await db
        .select()
        .from(Entities)
        .where(and(eq(Entities.slug, args.slug), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);
    },
  }),
}));
