import { and, eq } from 'drizzle-orm';
import { redis } from '@/cache';
import { db, first, firstOrThrow, TableCode, Users, UserSessions, validateDbId } from '@/db';
import { UserState } from '@/enums';
import { TypieError } from '@/errors';
import { assertAdminPermission } from '@/utils/permission';
import { builder } from '../builder';
import { User } from '../objects';

builder.queryFields((t) => ({
  impersonation: t.field({
    type: builder.simpleObject('Impersonation', {
      fields: (t) => ({
        user: t.field({ type: User }),
        admin: t.field({ type: User }),
      }),
    }),
    nullable: true,
    resolve: async (_, __, ctx) => {
      if (!ctx.session) {
        return null;
      }

      const impersonatedUserId = await redis.get(`admin:impersonate:${ctx.session.id}`);
      if (!impersonatedUserId) {
        return null;
      }

      const session = await db
        .select({ userId: UserSessions.userId })
        .from(UserSessions)
        .where(eq(UserSessions.id, ctx.session.id))
        .then(firstOrThrow);

      return {
        admin: session.userId,
        user: impersonatedUserId,
      };
    },
  }),
}));

builder.mutationFields((t) => ({
  adminImpersonate: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: { userId: t.input.string({ validate: validateDbId(TableCode.USERS) }) },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ userId: ctx.session.userId });

      if (ctx.session.userId === input.userId) {
        throw new TypieError({ code: 'cannot_impersonate_self' });
      }

      const targetUser = await db
        .select({ id: Users.id })
        .from(Users)
        .where(and(eq(Users.id, input.userId), eq(Users.state, UserState.ACTIVE)))
        .then(first);

      if (!targetUser) {
        throw new TypieError({ code: 'user_not_found' });
      }

      await redis.setex(`admin:impersonate:${ctx.session.id}`, 24 * 60 * 60, input.userId);

      return true;
    },
  }),

  adminStopImpersonation: t.withAuth({ session: true }).field({
    type: 'Boolean',
    resolve: async (_, __, ctx) => {
      await assertAdminPermission({ userId: ctx.session.userId });

      await redis.del(`admin:impersonate:${ctx.session.id}`);

      return true;
    },
  }),
}));
