import { EntityState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { db, Entities, EntityGoals, firstOrThrow, TableCode, UserGoals, validateDbId } from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { builder } from '../builder.ts';
import { Entity, EntityGoal, User, UserGoal } from '../objects.ts';

EntityGoal.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    targetCharacterCount: t.exposeInt('targetCharacterCount'),
    dueAt: t.expose('dueAt', { type: 'DateTime', nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

UserGoal.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    targetCharacterCount: t.int({
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      resolve: (self) => self.targetCharacterCount!,
    }),
  }),
});

builder.mutationFields((t) => ({
  updateEntityGoal: t.withAuth({ session: true }).fieldWithInput({
    type: Entity,
    input: {
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
      targetCharacterCount: t.input.int(),
      dueAt: t.input.field({ type: 'DateTime', required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      if (input.targetCharacterCount <= 0) {
        throw new TypieError({ code: 'invalid_target_character_count' });
      }

      const entity = await db
        .select({ siteId: Entities.siteId })
        .from(Entities)
        .where(and(eq(Entities.id, input.entityId), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: entity.siteId });

      const dueAt = input.dueAt ? dayjs.kst(input.dueAt).startOf('day') : null;

      await db
        .insert(EntityGoals)
        .values({ entityId: input.entityId, targetCharacterCount: input.targetCharacterCount, dueAt })
        .onConflictDoUpdate({
          target: [EntityGoals.entityId],
          set: { targetCharacterCount: input.targetCharacterCount, dueAt, updatedAt: dayjs() },
        });

      pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: input.entityId });

      return input.entityId;
    },
  }),

  deleteEntityGoal: t.withAuth({ session: true }).fieldWithInput({
    type: Entity,
    input: {
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({ siteId: Entities.siteId })
        .from(Entities)
        .where(and(eq(Entities.id, input.entityId), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: entity.siteId });

      await db.delete(EntityGoals).where(eq(EntityGoals.entityId, input.entityId));

      pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: input.entityId });

      return input.entityId;
    },
  }),

  updateUserGoal: t.withAuth({ session: true }).fieldWithInput({
    type: User,
    input: {
      targetCharacterCount: t.input.int(),
    },
    resolve: async (_, { input }, ctx) => {
      if (input.targetCharacterCount <= 0) {
        throw new TypieError({ code: 'invalid_target_character_count' });
      }

      const today = dayjs.kst().startOf('day');

      await db
        .insert(UserGoals)
        .values({ userId: ctx.session.userId, targetCharacterCount: input.targetCharacterCount, effectiveAt: today })
        .onConflictDoUpdate({
          target: [UserGoals.userId, UserGoals.effectiveAt],
          set: { targetCharacterCount: input.targetCharacterCount },
        });

      return ctx.session.userId;
    },
  }),

  deleteUserGoal: t.withAuth({ session: true }).field({
    type: User,
    resolve: async (_, __, ctx) => {
      const today = dayjs.kst().startOf('day');

      await db
        .insert(UserGoals)
        .values({ userId: ctx.session.userId, targetCharacterCount: null, effectiveAt: today })
        .onConflictDoUpdate({
          target: [UserGoals.userId, UserGoals.effectiveAt],
          set: { targetCharacterCount: null },
        });

      return ctx.session.userId;
    },
  }),
}));
