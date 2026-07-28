import {
  EntityState,
  EntityType,
  EntityVisibility,
  PaymentInvoiceState,
  PaymentOutcome,
  SubscriptionState,
  UserRole,
  UserState,
} from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { bootstrapSchema } from '@typie/lib/validation';
import { and, asc, count, desc, eq, exists, getTableColumns, gte, ilike, inArray, lte, ne, or, sql } from 'drizzle-orm';
import { fetchBootstrap, putBootstrap } from '#/bootstrap.ts';
import { redis } from '#/cache.ts';
import {
  db,
  Documents,
  Entities,
  first,
  firstOrThrow,
  Folders,
  PaymentInvoices,
  PaymentRecords,
  Sites,
  Subscriptions,
  TableCode,
  UserPaymentCredits,
  Users,
  UserSessions,
  validateDbId,
} from '#/db/index.ts';
import * as portone from '#/external/portone.ts';
import { assertAdminPermission } from '#/utils/permission.ts';
import { lockUserSubscriptionState } from '#/utils/subscription-lock.ts';
import { SYSTEM_USER_ID } from '#/utils/system-actor.ts';
import { builder } from '../builder.ts';
import { Entity, PaymentInvoice, Site, Subscription, User } from '../objects.ts';
import type { SQL } from 'drizzle-orm';

const AdminSearchResult = builder.unionType('AdminSearchResult', {
  types: [User, Entity, PaymentInvoice, Subscription, Site],
});

const AdminSearchTableCodes = new Set<string>([
  TableCode.USERS,
  TableCode.ENTITIES,
  TableCode.DOCUMENTS,
  TableCode.FOLDERS,
  TableCode.PAYMENT_INVOICES,
  TableCode.SUBSCRIPTIONS,
  TableCode.SITES,
]);

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

      list$ = list$.where(and(ne(Users.id, SYSTEM_USER_ID), ...conditions));
      count$ = count$.where(and(ne(Users.id, SYSTEM_USER_ID), ...conditions));

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

  adminEntities: t.withAuth({ session: true }).field({
    type: builder.simpleObject('AdminEntitiesResult', {
      fields: (t) => ({
        entities: t.field({ type: [Entity] }),
        totalCount: t.int(),
      }),
    }),
    args: {
      search: t.arg.string({ required: false }),
      type: t.arg({ type: EntityType, required: false }),
      state: t.arg({ type: EntityState, required: false }),
      visibility: t.arg({ type: EntityVisibility, required: false }),
      offset: t.arg.int({ defaultValue: 0 }),
      limit: t.arg.int({ defaultValue: 20 }),
    },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const conditions: (SQL | undefined)[] = [ne(Entities.type, EntityType.POST)];

      if (args.type) {
        conditions.push(eq(Entities.type, args.type));
      }

      if (args.state) {
        conditions.push(eq(Entities.state, args.state));
      }

      if (args.visibility) {
        conditions.push(eq(Entities.visibility, args.visibility));
      }

      if (args.search) {
        conditions.push(
          or(
            eq(Entities.id, args.search),
            eq(Entities.slug, args.search),
            eq(Entities.permalink, args.search),
            exists(
              db
                .select({ one: sql`1` })
                .from(Documents)
                .where(
                  and(
                    eq(Documents.entityId, Entities.id),
                    or(ilike(Documents.title, `%${args.search}%`), ilike(Documents.subtitle, `%${args.search}%`)),
                  ),
                ),
            ),
            exists(
              db
                .select({ one: sql`1` })
                .from(Folders)
                .where(and(eq(Folders.entityId, Entities.id), ilike(Folders.name, `%${args.search}%`))),
            ),
          ),
        );
      }

      const list$ = db
        .select(getTableColumns(Entities))
        .from(Entities)
        .where(and(...conditions))
        .orderBy(desc(Entities.createdAt))
        .limit(args.limit)
        .offset(args.offset);

      const count$ = db
        .select({ totalCount: count() })
        .from(Entities)
        .where(and(...conditions));

      const [entities, { totalCount }] = await Promise.all([list$, count$.then(firstOrThrow)]);

      return { entities, totalCount };
    },
  }),

  adminEntity: t.withAuth({ session: true }).field({
    type: Entity,
    args: { entityId: t.arg.string({ validate: validateDbId(TableCode.ENTITIES) }) },
    resolve: async (_, { entityId }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      return entityId;
    },
  }),

  adminSiteEntities: t.withAuth({ session: true }).field({
    type: [Entity],
    args: {
      siteId: t.arg.string({ validate: validateDbId(TableCode.SITES) }),
      includeDeleted: t.arg.boolean({ defaultValue: false }),
    },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const conditions = [eq(Entities.siteId, args.siteId), ne(Entities.type, EntityType.POST)];

      if (!args.includeDeleted) {
        conditions.push(eq(Entities.state, EntityState.ACTIVE));
      }

      return await db
        .select()
        .from(Entities)
        .where(and(...conditions))
        .orderBy(asc(Entities.depth), asc(Entities.order));
    },
  }),

  adminSubscriptions: t.withAuth({ session: true }).field({
    type: builder.simpleObject('AdminSubscriptionsResult', {
      fields: (t) => ({
        subscriptions: t.field({ type: [Subscription] }),
        totalCount: t.int(),
      }),
    }),
    args: {
      state: t.arg({ type: SubscriptionState, required: false }),
      planId: t.arg.string({ required: false }),
      offset: t.arg.int({ defaultValue: 0 }),
      limit: t.arg.int({ defaultValue: 20 }),
    },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const conditions = [];

      if (args.state) {
        conditions.push(eq(Subscriptions.state, args.state));
      }

      if (args.planId) {
        conditions.push(eq(Subscriptions.planId, args.planId));
      }

      const where = conditions.length > 0 ? and(...conditions) : undefined;

      const [subscriptions, { totalCount }] = await Promise.all([
        db.select().from(Subscriptions).where(where).orderBy(desc(Subscriptions.createdAt)).limit(args.limit).offset(args.offset),
        db.select({ totalCount: count() }).from(Subscriptions).where(where).then(firstOrThrow),
      ]);

      return { subscriptions, totalCount };
    },
  }),

  adminInvoices: t.withAuth({ session: true }).field({
    type: builder.simpleObject('AdminInvoicesResult', {
      fields: (t) => ({
        invoices: t.field({ type: [PaymentInvoice] }),
        totalCount: t.int(),
      }),
    }),
    args: {
      state: t.arg({ type: PaymentInvoiceState, required: false }),
      from: t.arg({ type: 'DateTime', required: false }),
      until: t.arg({ type: 'DateTime', required: false }),
      offset: t.arg.int({ defaultValue: 0 }),
      limit: t.arg.int({ defaultValue: 20 }),
    },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const conditions = [];

      if (args.state) {
        conditions.push(eq(PaymentInvoices.state, args.state));
      }

      if (args.from) {
        conditions.push(gte(PaymentInvoices.createdAt, args.from));
      }

      if (args.until) {
        conditions.push(lte(PaymentInvoices.createdAt, args.until));
      }

      const where = conditions.length > 0 ? and(...conditions) : undefined;

      const [invoices, { totalCount }] = await Promise.all([
        db.select().from(PaymentInvoices).where(where).orderBy(desc(PaymentInvoices.createdAt)).limit(args.limit).offset(args.offset),
        db.select({ totalCount: count() }).from(PaymentInvoices).where(where).then(firstOrThrow),
      ]);

      return { invoices, totalCount };
    },
  }),

  adminSearch: t.withAuth({ session: true }).field({
    type: [AdminSearchResult],
    args: { query: t.arg.string() },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const query = args.query.trim();
      if (query.length === 0) {
        return [];
      }

      const tableCode = /^([A-Z]{1,4})0[A-Z0-9]+$/.exec(query)?.[1];

      if (tableCode && AdminSearchTableCodes.has(tableCode)) {
        if (tableCode === TableCode.USERS) {
          const user = await db.select().from(Users).where(eq(Users.id, query)).then(first);
          return user ? [user] : [];
        }

        if (tableCode === TableCode.ENTITIES) {
          const entity = await db.select().from(Entities).where(eq(Entities.id, query)).then(first);
          return entity ? [entity] : [];
        }

        if (tableCode === TableCode.DOCUMENTS) {
          const entity = await db
            .select(getTableColumns(Entities))
            .from(Entities)
            .innerJoin(Documents, eq(Documents.entityId, Entities.id))
            .where(eq(Documents.id, query))
            .then(first);
          return entity ? [entity] : [];
        }

        if (tableCode === TableCode.FOLDERS) {
          const entity = await db
            .select(getTableColumns(Entities))
            .from(Entities)
            .innerJoin(Folders, eq(Folders.entityId, Entities.id))
            .where(eq(Folders.id, query))
            .then(first);
          return entity ? [entity] : [];
        }

        if (tableCode === TableCode.PAYMENT_INVOICES) {
          const invoice = await db.select().from(PaymentInvoices).where(eq(PaymentInvoices.id, query)).then(first);
          return invoice ? [invoice] : [];
        }

        if (tableCode === TableCode.SUBSCRIPTIONS) {
          const subscription = await db.select().from(Subscriptions).where(eq(Subscriptions.id, query)).then(first);
          return subscription ? [subscription] : [];
        }

        if (tableCode === TableCode.SITES) {
          const site = await db.select().from(Sites).where(eq(Sites.id, query)).then(first);
          return site ? [site] : [];
        }
      }

      if (query.includes('@')) {
        return await db
          .select()
          .from(Users)
          .where(and(ne(Users.id, SYSTEM_USER_ID), ilike(Users.email, `%${query}%`)))
          .orderBy(desc(Users.createdAt))
          .limit(10);
      }

      const [users, entities] = await Promise.all([
        db
          .select()
          .from(Users)
          .where(and(ne(Users.id, SYSTEM_USER_ID), ilike(Users.name, `%${query}%`)))
          .orderBy(desc(Users.createdAt))
          .limit(10),
        db
          .select()
          .from(Entities)
          .where(
            and(
              ne(Entities.type, EntityType.POST),
              or(
                exists(
                  db
                    .select({ one: sql`1` })
                    .from(Documents)
                    .where(and(eq(Documents.entityId, Entities.id), ilike(Documents.title, `%${query}%`))),
                ),
                exists(
                  db
                    .select({ one: sql`1` })
                    .from(Folders)
                    .where(and(eq(Folders.entityId, Entities.id), ilike(Folders.name, `%${query}%`))),
                ),
                eq(Entities.slug, query),
                eq(Entities.permalink, query),
              ),
            ),
          )
          .orderBy(desc(Entities.createdAt))
          .limit(10),
      ]);

      return [...users, ...entities];
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

  getBootstrap: t.withAuth({ session: true }).field({
    type: 'JSON',
    resolve: async (_, __, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      return fetchBootstrap();
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

  adminRefundPayment: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      invoiceId: t.input.string({ validate: validateDbId(TableCode.PAYMENT_INVOICES) }),
      reason: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      return await db.transaction(async (tx) => {
        // 갱신 잡과 직렬화한다. 갱신 잡도 구독 행을 잠그므로, 여기서 먼저 잠그면 환불 처리(외부 호출 포함) 중에
        // 갱신 잡이 새 인보이스를 청구·커밋해 만료된 구독에 결제가 남는 경합을 막는다.
        // 교착 방지: 모든 갱신·환불 경로는 구독 → 인보이스 순으로 잠근다. subscriptionId 는 불변 컬럼이라
        // 무락 조회가 안전하고, 인보이스 상태(PAID)는 아래 잠금 조회에서 재검증한다.
        const invoiceRef = await tx
          .select({ subscriptionId: PaymentInvoices.subscriptionId, userId: PaymentInvoices.userId })
          .from(PaymentInvoices)
          .where(eq(PaymentInvoices.id, input.invoiceId))
          .then(firstOrThrow);

        await lockUserSubscriptionState(tx, invoiceRef.userId);

        // 환불은 미래 예약도 함께 취소한다 — 예약이 남으면 환불 직후 전환 크론이 재과금한다.
        await tx
          .delete(Subscriptions)
          .where(and(eq(Subscriptions.userId, invoiceRef.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)));

        await tx
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .where(eq(Subscriptions.id, invoiceRef.subscriptionId))
          .for('no key update')
          .then(firstOrThrow);

        const invoice = await tx
          .select()
          .from(PaymentInvoices)
          .where(and(eq(PaymentInvoices.id, input.invoiceId), eq(PaymentInvoices.state, PaymentInvoiceState.PAID)))
          .for('no key update')
          .then(firstOrThrow);

        const record = await tx
          .select()
          .from(PaymentRecords)
          .where(and(eq(PaymentRecords.invoiceId, invoice.id), eq(PaymentRecords.outcome, PaymentOutcome.SUCCESS)))
          .then(first);

        if (record && record.billingAmount > 0) {
          const result = await portone.cancelPayment({
            paymentId: invoice.id,
            reason: input.reason ?? '관리자 환불',
          });
          if (result.status === 'failed') {
            throw new TypieError({ code: 'refund_failed', message: `[${result.code}] ${result.message}` });
          }
        }

        await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.CANCELED }).where(eq(PaymentInvoices.id, invoice.id));

        await tx
          .update(PaymentInvoices)
          .set({ state: PaymentInvoiceState.CANCELED })
          .where(
            and(
              eq(PaymentInvoices.subscriptionId, invoice.subscriptionId),
              inArray(PaymentInvoices.state, [PaymentInvoiceState.OVERDUE, PaymentInvoiceState.UPCOMING]),
            ),
          );

        await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
          .where(eq(Subscriptions.id, invoice.subscriptionId));

        return true;
      });
    },
  }),

  updateBootstrap: t.withAuth({ session: true }).fieldWithInput({
    type: 'JSON',
    input: {
      bootstrap: t.input.field({ type: 'JSON' }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const parsed = bootstrapSchema.omit({ version: true, updatedAt: true }).parse(input.bootstrap);

      return putBootstrap(parsed);
    },
  }),
}));
