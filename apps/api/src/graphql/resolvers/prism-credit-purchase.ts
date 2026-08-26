import {
  PrismCreditPack,
  PrismCreditPurchaseState,
  PrismCreditRefundKind,
  PrismCreditRefundMethod,
  PrismCreditRefundState,
} from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, desc, eq } from 'drizzle-orm';
import { db, dbr, PrismCreditPurchases, PrismCreditRefunds, TableCode, validateDbId } from '#/db/index.ts';
import { assertAdminPermission } from '#/utils/permission.ts';
import { assertPrismAccess } from '#/utils/prism-access.ts';
import { toDisplayCredits } from '#/utils/prism-credit-core.ts';
import { purchasePrismCreditPack } from '#/utils/prism-credit-purchase.ts';
import { executePrismCreditRefund, quotePrismCreditRefund, retryPrismCreditRefund } from '#/utils/prism-credit-refund.ts';
import { builder } from '../builder.ts';
import { isTypeOf, PrismCreditPurchase, User } from '../objects.ts';
import { assertSelf } from './prism-credit.ts';
import type { CancelPlan } from '#/utils/prism-credit-purchase-core.ts';

type PurchaseData = { receipt?: { receiptUrl: string | null } | null; failure?: Record<string, unknown> };
type RefundData = { cancels: CancelPlan[]; shortfall: number };

const resolveRefundedAmount = async (purchase: { id: string; userId: string }) => {
  const refunds = await dbr
    .select({ data: PrismCreditRefunds.data })
    .from(PrismCreditRefunds)
    .where(eq(PrismCreditRefunds.userId, purchase.userId));
  return refunds
    .flatMap((refund) => (refund.data as RefundData).cancels)
    .filter((cancel) => cancel.purchaseId === purchase.id && (cancel.status === 'succeeded' || cancel.status === 'manual'))
    .reduce((acc, cancel) => acc + cancel.amount, 0);
};

PrismCreditPurchase.implement({
  isTypeOf: isTypeOf(TableCode.PRISM_CREDIT_PURCHASES),
  fields: (t) => ({
    id: t.exposeID('id'),
    pack: t.expose('pack', { type: PrismCreditPack }),
    price: t.exposeInt('price'),
    credits: t.exposeInt('credits'),
    bonusCredits: t.exposeInt('bonusCredits'),
    paidAt: t.expose('paidAt', { type: 'DateTime', nullable: true }),
    receiptUrl: t.string({ nullable: true, resolve: (self) => (self.data as PurchaseData).receipt?.receiptUrl ?? null }),
    refundedAmount: t.int({ resolve: (self) => resolveRefundedAmount(self) }),
  }),
});

builder.objectFields(User, (t) => ({
  prismCreditPurchases: t.field({
    type: [PrismCreditPurchase],
    resolve: async (self, _, ctx) => {
      assertSelf(self, ctx);
      const rows = await db
        .select({ id: PrismCreditPurchases.id })
        .from(PrismCreditPurchases)
        .where(and(eq(PrismCreditPurchases.userId, self.id), eq(PrismCreditPurchases.state, 'PAID')))
        .orderBy(desc(PrismCreditPurchases.paidAt), desc(PrismCreditPurchases.id));
      return rows.map((row) => row.id);
    },
  }),
}));

builder.mutationFields((t) => ({
  purchasePrismCreditPack: t.withAuth({ session: true }).fieldWithInput({
    type: PrismCreditPurchase,
    input: { pack: t.input.field({ type: PrismCreditPack }) },
    resolve: async (_, { input }, ctx) => {
      await assertPrismAccess({ userId: ctx.session.userId });

      const outcome = await purchasePrismCreditPack({ userId: ctx.session.userId, pack: input.pack });
      if (outcome.kind === 'failed') throw new TypieError({ code: 'payment_failed', status: 402 });
      if (outcome.kind === 'pending') throw new TypieError({ code: 'payment_pending', status: 409 });

      return outcome.purchaseId;
    },
  }),
}));

const AdminPrismCreditPurchase = builder.objectRef<typeof PrismCreditPurchases.$inferSelect>('AdminPrismCreditPurchase').implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    pack: t.expose('pack', { type: PrismCreditPack }),
    price: t.exposeInt('price'),
    credits: t.exposeInt('credits'),
    bonusCredits: t.exposeInt('bonusCredits'),
    state: t.expose('state', { type: PrismCreditPurchaseState }),
    paymentKey: t.exposeString('paymentKey'),
    paidAt: t.expose('paidAt', { type: 'DateTime', nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    data: t.expose('data', { type: 'JSON' }),
    refundedAmount: t.int({ resolve: (self) => resolveRefundedAmount(self) }),
  }),
});

const AdminPrismCreditRefund = builder.objectRef<typeof PrismCreditRefunds.$inferSelect>('AdminPrismCreditRefund').implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    kind: t.expose('kind', { type: PrismCreditRefundKind }),
    purchaseId: t.exposeID('purchaseId', { nullable: true }),
    amount: t.exposeInt('amount'),
    method: t.expose('method', { type: PrismCreditRefundMethod }),
    state: t.expose('state', { type: PrismCreditRefundState }),
    note: t.exposeString('note'),
    actor: t.field({ type: User, resolve: (self) => self.actorId }),
    data: t.expose('data', { type: 'JSON' }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

const AdminPrismCreditRefundCancel = builder.simpleObject('AdminPrismCreditRefundCancel', {
  fields: (t) => ({ purchaseId: t.id(), paymentKey: t.string(), amount: t.int() }),
});

const AdminPrismCreditRefundQuote = builder.simpleObject('AdminPrismCreditRefundQuote', {
  fields: (t) => ({
    eligible: t.boolean(),
    reason: t.string({ nullable: true }),
    amount: t.int(),
    paidCredits: t.int(),
    freeCredits: t.int(),
    shortfall: t.int(),
    cancels: t.field({ type: [AdminPrismCreditRefundCancel] }),
  }),
});

builder.queryFields((t) => ({
  adminPrismCreditPurchases: t.withAuth({ session: true }).field({
    type: [AdminPrismCreditPurchase],
    args: { userId: t.arg.string({ validate: validateDbId(TableCode.USERS) }) },
    resolve: async (_, { userId }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });
      return await db
        .select()
        .from(PrismCreditPurchases)
        .where(eq(PrismCreditPurchases.userId, userId))
        .orderBy(desc(PrismCreditPurchases.createdAt));
    },
  }),

  adminPrismCreditRefunds: t.withAuth({ session: true }).field({
    type: [AdminPrismCreditRefund],
    args: { userId: t.arg.string({ validate: validateDbId(TableCode.USERS) }) },
    resolve: async (_, { userId }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });
      return await db
        .select()
        .from(PrismCreditRefunds)
        .where(eq(PrismCreditRefunds.userId, userId))
        .orderBy(desc(PrismCreditRefunds.createdAt));
    },
  }),

  adminPrismCreditRefundQuote: t.withAuth({ session: true }).field({
    type: AdminPrismCreditRefundQuote,
    args: {
      userId: t.arg.string({ validate: validateDbId(TableCode.USERS) }),
      kind: t.arg({ type: PrismCreditRefundKind }),
      purchaseId: t.arg.string({ required: false, validate: validateDbId(TableCode.PRISM_CREDIT_PURCHASES) }),
    },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const quote = await quotePrismCreditRefund(dbr, { userId: args.userId, kind: args.kind, purchaseId: args.purchaseId ?? null });
      if (!quote.eligible) {
        return { eligible: false, reason: quote.reason, amount: 0, paidCredits: 0, freeCredits: 0, shortfall: 0, cancels: [] };
      }

      return {
        eligible: true,
        reason: null,
        amount: quote.amount,
        paidCredits: toDisplayCredits(0 - quote.delta.paidDelta),
        freeCredits: toDisplayCredits(0 - quote.delta.freeDelta),
        shortfall: quote.shortfall,
        cancels: quote.cancels.map(({ purchaseId, paymentKey, amount }) => ({ purchaseId, paymentKey, amount })),
      };
    },
  }),
}));

builder.mutationFields((t) => ({
  adminRefundPrismCredit: t.withAuth({ session: true }).fieldWithInput({
    type: AdminPrismCreditRefund,
    input: {
      userId: t.input.string({ validate: validateDbId(TableCode.USERS) }),
      kind: t.input.field({ type: PrismCreditRefundKind }),
      purchaseId: t.input.string({ required: false, validate: validateDbId(TableCode.PRISM_CREDIT_PURCHASES) }),
      expectedAmount: t.input.int(),
      method: t.input.field({ type: PrismCreditRefundMethod }),
      note: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const note = input.note.trim();
      if (note.length === 0) throw new TypieError({ code: 'invalid_input', status: 400 });

      const { refundId } = await executePrismCreditRefund({
        userId: input.userId,
        kind: input.kind,
        purchaseId: input.purchaseId ?? null,
        expectedAmount: input.expectedAmount,
        method: input.method,
        note,
        actorId: ctx.session.userId,
      });

      return await db
        .select()
        .from(PrismCreditRefunds)
        .where(eq(PrismCreditRefunds.id, refundId))
        .then((rows) => rows[0]);
    },
  }),

  adminRetryPrismCreditRefund: t.withAuth({ session: true }).fieldWithInput({
    type: AdminPrismCreditRefund,
    input: {
      refundId: t.input.string({ validate: validateDbId(TableCode.PRISM_CREDIT_REFUNDS) }),
      method: t.input.field({ type: PrismCreditRefundMethod }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      const { refundId } = await retryPrismCreditRefund({ refundId: input.refundId, method: input.method });

      return await db
        .select()
        .from(PrismCreditRefunds)
        .where(eq(PrismCreditRefunds.id, refundId))
        .then((rows) => rows[0]);
    },
  }),
}));
