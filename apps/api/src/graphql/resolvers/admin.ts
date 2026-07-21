import { DocumentType, EntityState, PaymentInvoiceState, PaymentOutcome, SubscriptionState, UserRole, UserState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { bootstrapSchema } from '@typie/lib/validation';
import { and, count, desc, eq, getTableColumns, ilike, inArray, ne, or, sql } from 'drizzle-orm';
import { fetchBootstrap, putBootstrap } from '#/bootstrap.ts';
import { redis } from '#/cache.ts';
import {
  db,
  Documents,
  Entities,
  first,
  firstOrThrow,
  PaymentInvoices,
  PaymentRecords,
  pgr,
  Subscriptions,
  TableCode,
  UserPaymentCredits,
  Users,
  UserSessions,
  validateDbId,
} from '#/db/index.ts';
import * as portone from '#/external/portone.ts';
import { assertAdminPermission } from '#/utils/permission.ts';
import { SYSTEM_USER_ID } from '#/utils/system-actor.ts';
import { builder } from '../builder.ts';
import { Document, User } from '../objects.ts';

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

  adminDocuments: t.withAuth({ session: true }).field({
    type: builder.simpleObject('AdminDocumentsResult', {
      fields: (t) => ({
        documents: t.field({ type: [Document] }),
        totalCount: t.int(),
      }),
    }),
    args: {
      search: t.arg.string({ required: false }),
      type: t.arg({ type: DocumentType, required: false }),
      state: t.arg({ type: EntityState, required: false }),
      offset: t.arg.int({ defaultValue: 0 }),
      limit: t.arg.int({ defaultValue: 20 }),
    },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      let list$ = db.select(getTableColumns(Documents)).from(Documents).innerJoin(Entities, eq(Documents.entityId, Entities.id)).$dynamic();
      let count$ = db.select({ totalCount: count() }).from(Documents).innerJoin(Entities, eq(Documents.entityId, Entities.id)).$dynamic();

      const conditions = [];

      if (args.type) {
        conditions.push(eq(Documents.type, args.type));
      }

      if (args.state) {
        conditions.push(eq(Entities.state, args.state));
      }

      if (args.search) {
        conditions.push(
          or(
            ilike(Documents.title, `%${args.search}%`),
            ilike(Documents.subtitle, `%${args.search}%`),
            eq(Documents.id, args.search),
            eq(Entities.slug, args.search),
            eq(Entities.permalink, args.search),
          ),
        );
      }

      if (conditions.length > 0) {
        list$ = list$.where(and(...conditions));
        count$ = count$.where(and(...conditions));
      }

      list$ = list$.orderBy(desc(Documents.createdAt)).limit(args.limit).offset(args.offset);

      const [documents, { totalCount }] = await Promise.all([list$, count$.then(firstOrThrow)]);

      return { documents, totalCount };
    },
  }),

  adminDocument: t.withAuth({ session: true }).field({
    type: Document,
    args: { documentId: t.arg.string({ validate: validateDbId(TableCode.DOCUMENTS) }) },
    resolve: async (_, { documentId }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      return documentId;
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

      const result = await pgr.begin('READ ONLY', async (sql) => {
        return await sql.unsafe(query, params ?? []);
      });

      return result;
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
          .select({ subscriptionId: PaymentInvoices.subscriptionId })
          .from(PaymentInvoices)
          .where(eq(PaymentInvoices.id, input.invoiceId))
          .then(firstOrThrow);

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
