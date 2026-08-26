import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import { clearLoaders } from '#/context.ts';
import { db, firstOrThrow, TableCode, Users, validateDbId } from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { deleteEntityGoalCore, deleteUserGoalCore, upsertEntityGoalCore, upsertUserGoalCore } from '#/utils/goal-actions.ts';
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
      const dueAt = input.dueAt ? dayjs.kst(input.dueAt).startOf('day') : null;

      await upsertEntityGoalCore(db, {
        userId: ctx.session.userId,
        entityId: input.entityId,
        targetCharacterCount: input.targetCharacterCount,
        dueAt,
      });

      return input.entityId;
    },
  }),

  deleteEntityGoal: t.withAuth({ session: true }).fieldWithInput({
    type: Entity,
    input: {
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      await deleteEntityGoalCore(db, { userId: ctx.session.userId, entityId: input.entityId });

      return input.entityId;
    },
  }),

  updateUserGoal: t.withAuth({ session: true }).fieldWithInput({
    type: User,
    input: {
      targetCharacterCount: t.input.int(),
    },
    resolve: async (_, { input }, ctx) => {
      await upsertUserGoalCore(db, { userId: ctx.session.userId, targetCharacterCount: input.targetCharacterCount });

      return ctx.session.userId;
    },
  }),

  deleteUserGoal: t.withAuth({ session: true }).field({
    type: User,
    resolve: async (_, __, ctx) => {
      await deleteUserGoalCore(db, { userId: ctx.session.userId });

      return ctx.session.userId;
    },
  }),
}));

builder.subscriptionFields((t) => ({
  userGoalUpdateStream: t.withAuth({ session: true }).field({
    type: User,
    subscribe: async (_, __, ctx) => {
      const repeater = pubsub.subscribe('user:goal:update', ctx.session.userId);

      ctx.c.req.raw.signal.addEventListener('abort', () => {
        repeater.return();
      });

      return repeater;
    },
    resolve: async (_, __, ctx) => {
      clearLoaders(ctx);

      return await db.select().from(Users).where(eq(Users.id, ctx.session.userId)).then(firstOrThrow);
    },
  }),
}));
