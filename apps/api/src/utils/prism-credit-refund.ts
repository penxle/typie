import { TypieError } from '@typie/lib/errors';
import { and, asc, eq } from 'drizzle-orm';
import {
  createDbId,
  db,
  first,
  firstOrThrow,
  PrismCreditEntries,
  PrismCreditPurchases,
  PrismCreditRefunds,
  TableCode,
} from '#/db/index.ts';
import * as portone from '#/external/portone.ts';
import { pubsub } from '#/pubsub.ts';
import { opsAlert } from './ops-alert.ts';
import { lockUserPrismCredit } from './prism-credit.ts';
import { validateEntry } from './prism-credit-core.ts';
import { quoteRemainder, quoteWithdrawal } from './prism-credit-purchase-core.ts';
import type { PrismCreditRefundKind, PrismCreditRefundMethod, PrismCreditRefundState } from '@typie/lib/enums';
import type { Database, Transaction } from '#/db/index.ts';
import type { CancelPlan, LedgerRow, PurchaseRow, RefundQuote, RefundRow } from './prism-credit-purchase-core.ts';

export type RefundTarget = { userId: string; kind: PrismCreditRefundKind; purchaseId: string | null };

type RefundData = { cancels: CancelPlan[]; shortfall: number };

const loadContext = async (executor: Database | Transaction, userId: string) => {
  const entries: LedgerRow[] = await executor
    .select({
      kind: PrismCreditEntries.kind,
      paidDelta: PrismCreditEntries.paidDelta,
      freeDelta: PrismCreditEntries.freeDelta,
      key: PrismCreditEntries.key,
      createdAt: PrismCreditEntries.createdAt,
    })
    .from(PrismCreditEntries)
    .where(eq(PrismCreditEntries.userId, userId))
    .orderBy(asc(PrismCreditEntries.createdAt), asc(PrismCreditEntries.id))
    .then((rows) => rows.map((row) => ({ ...row, createdAt: row.createdAt.valueOf() })));

  const purchases: PurchaseRow[] = await executor
    .select({
      id: PrismCreditPurchases.id,
      paymentKey: PrismCreditPurchases.paymentKey,
      price: PrismCreditPurchases.price,
      credits: PrismCreditPurchases.credits,
      bonusCredits: PrismCreditPurchases.bonusCredits,
      state: PrismCreditPurchases.state,
      paidAt: PrismCreditPurchases.paidAt,
    })
    .from(PrismCreditPurchases)
    .where(eq(PrismCreditPurchases.userId, userId))
    .then((rows) => rows.map((row) => ({ ...row, paidAt: row.paidAt?.valueOf() ?? null })));

  const refunds: RefundRow[] = await executor
    .select({
      id: PrismCreditRefunds.id,
      kind: PrismCreditRefunds.kind,
      purchaseId: PrismCreditRefunds.purchaseId,
      state: PrismCreditRefunds.state,
      data: PrismCreditRefunds.data,
    })
    .from(PrismCreditRefunds)
    .where(eq(PrismCreditRefunds.userId, userId))
    .then((rows) => rows.map(({ data, ...row }) => ({ ...row, cancels: (data as RefundData).cancels })));

  return { entries, purchases, refunds };
};

const quoteWith = (context: Awaited<ReturnType<typeof loadContext>>, target: RefundTarget, now: number): RefundQuote => {
  if (target.kind === 'WITHDRAWAL') {
    const purchase = context.purchases.find((row) => row.id === target.purchaseId);
    if (!purchase) throw new TypieError({ code: 'not_found', status: 404 });
    return quoteWithdrawal({ purchase, entries: context.entries, refunds: context.refunds, now });
  }

  return quoteRemainder({ purchases: context.purchases, entries: context.entries, refunds: context.refunds });
};

export const quotePrismCreditRefund = async (
  executor: Database | Transaction,
  target: RefundTarget,
  now = Date.now(),
): Promise<RefundQuote> => {
  if (target.kind === 'WITHDRAWAL' && target.purchaseId === null) throw new TypieError({ code: 'invalid_input', status: 400 });
  return quoteWith(await loadContext(executor, target.userId), target, now);
};

const readRefundData = async (refundId: string): Promise<RefundData> => {
  const refund = await db
    .select({ data: PrismCreditRefunds.data })
    .from(PrismCreditRefunds)
    .where(eq(PrismCreditRefunds.id, refundId))
    .then(firstOrThrow);

  return refund.data as RefundData;
};

const runCancels = async (refundId: string, note: string): Promise<{ done: boolean; failed: CancelPlan[] }> => {
  const data = await readRefundData(refundId);
  const cancels = data.cancels;

  for (const [index, cancel] of cancels.entries()) {
    if (cancel.status === 'succeeded') continue;

    const result = await portone.cancelPayment({ paymentId: cancel.paymentKey, amount: cancel.amount, reason: note });
    cancels[index] = { ...cancel, status: result.status === 'failed' ? 'failed' : 'succeeded' };

    await db
      .update(PrismCreditRefunds)
      .set({ data: { ...data, cancels } })
      .where(eq(PrismCreditRefunds.id, refundId));
  }

  const failed = cancels.filter((cancel) => cancel.status !== 'succeeded');
  if (failed.length === 0) {
    await db
      .update(PrismCreditRefunds)
      .set({ state: 'DONE' })
      .where(and(eq(PrismCreditRefunds.id, refundId), eq(PrismCreditRefunds.state, 'PENDING')));
    return { done: true, failed: [] };
  }

  await opsAlert('prism-credit-refund-incomplete', { refundId, failed: failed.map((cancel) => cancel.purchaseId) });
  return { done: false, failed };
};

const settle = async (
  refundId: string,
  method: PrismCreditRefundMethod,
  note: string,
): Promise<{ refundId: string; state: PrismCreditRefundState }> => {
  if (method === 'MANUAL') {
    const data = await readRefundData(refundId);
    const cancels = data.cancels.map((cancel) => (cancel.status === 'succeeded' ? cancel : { ...cancel, status: 'manual' as const }));

    await db
      .update(PrismCreditRefunds)
      .set({ method: 'MANUAL', state: 'DONE', data: { ...data, cancels } })
      .where(and(eq(PrismCreditRefunds.id, refundId), eq(PrismCreditRefunds.state, 'PENDING')));
    return { refundId, state: 'DONE' };
  }

  const { done } = await runCancels(refundId, note);
  if (!done) throw new TypieError({ code: 'refund_partially_failed', status: 502 });

  return { refundId, state: 'DONE' };
};

export const executePrismCreditRefund = async ({
  userId,
  kind,
  purchaseId,
  expectedAmount,
  method,
  note,
  actorId,
}: RefundTarget & { expectedAmount: number; method: PrismCreditRefundMethod; note: string; actorId: string }) => {
  if (kind === 'WITHDRAWAL' && purchaseId === null) throw new TypieError({ code: 'invalid_input', status: 400 });

  const refundId = await db.transaction(async (tx) => {
    await lockUserPrismCredit(tx, userId);

    const quote = quoteWith(await loadContext(tx, userId), { userId, kind, purchaseId }, Date.now());
    if (!quote.eligible) throw new TypieError({ code: 'refund_not_eligible', status: 400, message: quote.reason });
    if (quote.amount !== expectedAmount) throw new TypieError({ code: 'refund_quote_changed', status: 409 });
    if (method === 'PG_CANCEL' && quote.shortfall > 0) throw new TypieError({ code: 'refund_shortfall', status: 400 });

    const id = createDbId(TableCode.PRISM_CREDIT_REFUNDS);
    await tx.insert(PrismCreditRefunds).values({
      id,
      userId,
      kind,
      purchaseId,
      amount: quote.amount,
      method,
      state: 'PENDING',
      actorId,
      note,
      data: { cancels: quote.cancels, shortfall: quote.shortfall } satisfies RefundData,
    });

    validateEntry('REFUND_OUT', quote.delta);
    await tx.insert(PrismCreditEntries).values({ userId, kind: 'REFUND_OUT', key: id, note, actorId, ...quote.delta });

    return id;
  });

  pubsub.publish('prism:credit', userId, {});

  return await settle(refundId, method, note);
};

export const retryPrismCreditRefund = async ({ refundId, method }: { refundId: string; method: PrismCreditRefundMethod }) => {
  const refund = await db
    .select({ state: PrismCreditRefunds.state, note: PrismCreditRefunds.note })
    .from(PrismCreditRefunds)
    .where(eq(PrismCreditRefunds.id, refundId))
    .then(first);
  if (!refund) throw new TypieError({ code: 'not_found', status: 404 });
  if (refund.state !== 'PENDING') throw new TypieError({ code: 'refund_not_pending', status: 409 });

  return await settle(refundId, method, refund.note);
};
