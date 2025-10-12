import { and, count, desc, eq, getTableColumns, ilike, or, sql } from 'drizzle-orm';
import { redis } from '@/cache';
import { db, Entities, first, firstOrThrow, pg, Posts, TableCode, UserPaymentCredits, Users, UserSessions, validateDbId } from '@/db';
import { EntityState, UserRole, UserState } from '@/enums';
import { TypieError } from '@/errors';
import { enqueueJob } from '@/mq';
import { assertAdminPermission } from '@/utils/permission';
import { builder } from '../builder';
import { Post, User } from '../objects';

builder.queryFields((t) => ({
  adminUsers: t.withAuth({ session: true }).field({
    type: builder.simpleObject('AdminUsersResult', {
      fields: (t) => ({
        users: t.field({ type: [User] }),
        totalCount: t.int(),
      }),
    }),
    args: {
      search: t.arg.string({ required: false }),
      state: t.arg({ type: UserState, required: false }),
      role: t.arg({ type: UserRole, required: false }),
      offset: t.arg.int({ defaultValue: 0 }),
      limit: t.arg.int({ defaultValue: 20 }),
    },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      let list$ = db.select().from(Users).$dynamic();
      let count$ = db.select({ totalCount: count() }).from(Users).$dynamic();

      const conditions = [];

      if (args.state) {
        conditions.push(eq(Users.state, args.state));
      }

      if (args.role) {
        conditions.push(eq(Users.role, args.role));
      }

      if (args.search) {
        conditions.push(or(ilike(Users.name, `%${args.search}%`), ilike(Users.email, `%${args.search}%`), eq(Users.id, args.search)));
      }

      if (conditions.length > 0) {
        list$ = list$.where(and(...conditions));
        count$ = count$.where(and(...conditions));
      }

      list$ = list$.orderBy(desc(Users.createdAt)).limit(args.limit).offset(args.offset);

      const [users, { totalCount }] = await Promise.all([list$, count$.then(firstOrThrow)]);

      return { users, totalCount };
    },
  }),

  adminUser: t.withAuth({ session: true }).field({
    type: User,
    args: { userId: t.arg.string({ validate: validateDbId(TableCode.USERS) }) },
    resolve: async (_, { userId }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      return userId;
    },
  }),

  adminPosts: t.withAuth({ session: true }).field({
    type: builder.simpleObject('AdminPostsResult', {
      fields: (t) => ({
        posts: t.field({ type: [Post] }),
        totalCount: t.int(),
      }),
    }),
    args: {
      search: t.arg.string({ required: false }),
      state: t.arg({ type: EntityState, required: false }),
      offset: t.arg.int({ defaultValue: 0 }),
      limit: t.arg.int({ defaultValue: 20 }),
    },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      let list$ = db.select(getTableColumns(Posts)).from(Posts).innerJoin(Entities, eq(Posts.entityId, Entities.id)).$dynamic();
      let count$ = db.select({ totalCount: count() }).from(Posts).innerJoin(Entities, eq(Posts.entityId, Entities.id)).$dynamic();

      const conditions = [];

      if (args.state) {
        conditions.push(eq(Entities.state, args.state));
      }

      if (args.search) {
        conditions.push(
          or(
            ilike(Posts.title, `%${args.search}%`),
            ilike(Posts.subtitle, `%${args.search}%`),
            eq(Posts.id, args.search),
            eq(Entities.slug, args.search),
            eq(Entities.permalink, args.search),
          ),
        );
      }

      if (conditions.length > 0) {
        list$ = list$.where(and(...conditions));
        count$ = count$.where(and(...conditions));
      }

      list$ = list$.orderBy(desc(Posts.createdAt)).limit(args.limit).offset(args.offset);

      const [posts, { totalCount }] = await Promise.all([list$, count$.then(firstOrThrow)]);

      return { posts, totalCount };
    },
  }),

  adminPost: t.withAuth({ session: true }).field({
    type: Post,
    args: { postId: t.arg.string({ validate: validateDbId(TableCode.POSTS) }) },
    resolve: async (_, { postId }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      return postId;
    },
  }),

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

  adminRawQuery: t.withAuth({ session: true }).field({
    type: ['JSON'],
    args: {
      query: t.arg.string(),
      params: t.arg({ type: ['JSON'], required: false }),
    },
    resolve: async (_, { query, params }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const result = await pg.begin('READ ONLY', async (sql) => {
        return await sql.unsafe(query, params ?? []);
      });

      return result;
    },
  }),
}));

builder.mutationFields((t) => ({
  adminImpersonate: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: { userId: t.input.string({ validate: validateDbId(TableCode.USERS) }) },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

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
      await assertAdminPermission({ sessionId: ctx.session.id });

      await redis.del(`admin:impersonate:${ctx.session.id}`);

      return true;
    },
  }),

  adminEnqueuePostCompact: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: { postId: t.input.string({ validate: validateDbId(TableCode.POSTS) }) },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      await enqueueJob('post:compact', input.postId);

      return true;
    },
  }),

  adminGiveCredit: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: { userId: t.input.string({ validate: validateDbId(TableCode.USERS) }), amount: t.input.int() },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      await db
        .insert(UserPaymentCredits)
        .values({
          userId: input.userId,
          amount: input.amount,
        })
        .onConflictDoUpdate({
          target: [UserPaymentCredits.userId],
          set: {
            amount: sql`${UserPaymentCredits.amount} + ${input.amount}`,
          },
        });

      return true;
    },
  }),
}));
