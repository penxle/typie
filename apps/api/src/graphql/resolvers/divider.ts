import { db, TableCode, validateDbId } from '#/db/index.ts';
import { createDividerCore } from '#/utils/entity-actions.ts';
import { builder } from '../builder.ts';
import { Divider, Entity, IDivider, isTypeOf } from '../objects.ts';

/**
 * * Types
 */

IDivider.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
  }),
});

Divider.implement({
  isTypeOf: isTypeOf(TableCode.DIVIDERS),
  interfaces: [IDivider],
  fields: (t) => ({
    entity: t.expose('entityId', { type: Entity }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  createDivider: t.withAuth({ session: true }).fieldWithInput({
    type: Divider,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      parentEntityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) =>
      await createDividerCore(db, {
        userId: ctx.session.userId,
        siteId: input.siteId,
        parentEntityId: input.parentEntityId ?? null,
        lowerOrder: input.lowerOrder ?? null,
        upperOrder: input.upperOrder ?? null,
      }),
  }),
}));
