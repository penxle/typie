import { and, eq } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { db, Entities, firstOrThrow, Folders, Posts } from '@/db';
import { EntityState, EntityType } from '@/enums';
import { builder } from '../builder';
import { Entity, EntityViewUnion } from '../objects';

/**
 * * Types
 */

Entity.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    order: t.expose('order', { type: 'Binary' }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  entityView: t.field({
    type: EntityViewUnion,
    args: { slug: t.arg.string() },
    resolve: async (_, args) => {
      const entity = await db
        .select({ id: Entities.id, type: Entities.type })
        .from(Entities)
        .where(and(eq(Entities.slug, args.slug), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);

      return match(entity.type)
        .with(EntityType.FOLDER, () => db.select().from(Folders).where(eq(Folders.entityId, entity.id)).then(firstOrThrow))
        .with(EntityType.POST, () => db.select().from(Posts).where(eq(Posts.entityId, entity.id)).then(firstOrThrow))
        .exhaustive();
    },
  }),
}));
