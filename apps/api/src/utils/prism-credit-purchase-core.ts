import { MILLI_PER_CREDIT } from './prism-credit-core.ts';
import type { PrismCreditEntryKind, PrismCreditPurchaseState, PrismCreditRefundKind, PrismCreditRefundState } from '@typie/lib/enums';
import type { LookupPaymentResult } from '#/external/portone.ts';

export const WITHDRAWAL_WINDOW_MS = 7 * 24 * 60 * 60 * 1000;
export const REMAINDER_REFUND_RATE = 0.9;
export const RECONCILE_MIN_AGE_MS = 2 * 60 * 1000;

export type LedgerRow = { kind: PrismCreditEntryKind; paidDelta: number; freeDelta: number; key: string | null; createdAt: number };
export type PurchaseRow = {
  id: string;
  paymentKey: string;
  price: number;
  credits: number;
  bonusCredits: number;
  state: PrismCreditPurchaseState;
  paidAt: number | null;
};
export type CancelPlan = { purchaseId: string; paymentKey: string; amount: number; status: 'planned' | 'succeeded' | 'failed' | 'manual' };
export type RefundRow = {
  id: string;
  kind: PrismCreditRefundKind;
  purchaseId: string | null;
  state: PrismCreditRefundState;
  cancels: CancelPlan[];
};

export type RefundIneligibleReason = 'not_paid' | 'already_refunded' | 'window_expired' | 'used' | 'no_paid_balance';
export type RefundQuote =
  | { eligible: false; reason: RefundIneligibleReason }
  | { eligible: true; amount: number; delta: { paidDelta: number; freeDelta: number }; cancels: CancelPlan[]; shortfall: number };

const CONSUMPTION_KINDS = new Set<PrismCreditEntryKind>(['REVIEW_CHARGE', 'CHAT_CHARGE', 'REVIEW_REFUND']);
const FINAL_NOT_PAID_STATUSES = new Set(['FAILED', 'CANCELLED', 'PARTIAL_CANCELLED']);

const sumBy = (rows: LedgerRow[], pick: (row: LedgerRow) => number) => rows.reduce((acc, row) => acc + pick(row), 0);

const cancelledAmount = (purchaseId: string, refunds: RefundRow[]) =>
  refunds.reduce(
    (acc, refund) =>
      acc +
      refund.cancels
        .filter((cancel) => cancel.purchaseId === purchaseId && (cancel.status === 'succeeded' || cancel.status === 'manual'))
        .reduce((s, c) => s + c.amount, 0),
    0,
  );

export const allocateCancels = (purchases: PurchaseRow[], refunds: RefundRow[], amount: number): CancelPlan[] => {
  const plans: CancelPlan[] = [];
  let remaining = amount;

  const ordered = purchases.filter((p) => p.state === 'PAID').toSorted((a, b) => (b.paidAt ?? 0) - (a.paidAt ?? 0));
  for (const purchase of ordered) {
    if (remaining <= 0) break;
    const available = purchase.price - cancelledAmount(purchase.id, refunds);
    const take = Math.min(available, remaining);
    if (take <= 0) continue;
    plans.push({ purchaseId: purchase.id, paymentKey: purchase.paymentKey, amount: take, status: 'planned' });
    remaining -= take;
  }

  return plans;
};

export const quoteWithdrawal = ({
  purchase,
  entries,
  refunds,
  now,
}: {
  purchase: PurchaseRow;
  entries: LedgerRow[];
  refunds: RefundRow[];
  now: number;
}): RefundQuote => {
  if (purchase.state !== 'PAID' || purchase.paidAt === null) return { eligible: false, reason: 'not_paid' };
  const touched = refunds.some(
    (refund) => refund.purchaseId === purchase.id || refund.cancels.some((cancel) => cancel.purchaseId === purchase.id),
  );
  if (touched) return { eligible: false, reason: 'already_refunded' };
  if (now - purchase.paidAt > WITHDRAWAL_WINDOW_MS) return { eligible: false, reason: 'window_expired' };

  const purchaseEntry = entries.find((row) => row.kind === 'PURCHASE' && row.key === purchase.id);
  if (!purchaseEntry) return { eligible: false, reason: 'not_paid' };

  const consumedPaid =
    0 -
    sumBy(
      entries.filter((row) => CONSUMPTION_KINDS.has(row.kind) && row.createdAt > purchaseEntry.createdAt),
      (row) => row.paidDelta,
    );
  if (consumedPaid > 0) return { eligible: false, reason: 'used' };

  const freeBalance = Math.max(
    sumBy(entries, (row) => row.freeDelta),
    0,
  );
  const freeDelta = 0 - Math.min(freeBalance, purchase.bonusCredits * MILLI_PER_CREDIT);

  return {
    eligible: true,
    amount: purchase.price,
    delta: { paidDelta: 0 - purchase.credits * MILLI_PER_CREDIT, freeDelta },
    cancels: [{ purchaseId: purchase.id, paymentKey: purchase.paymentKey, amount: purchase.price, status: 'planned' }],
    shortfall: 0,
  };
};

export const quoteRemainder = ({
  purchases,
  entries,
  refunds,
}: {
  purchases: PurchaseRow[];
  entries: LedgerRow[];
  refunds: RefundRow[];
}): RefundQuote => {
  const paidBalance = sumBy(entries, (row) => row.paidDelta);
  if (paidBalance <= 0) return { eligible: false, reason: 'no_paid_balance' };

  const withdrawn = new Set(refunds.filter((refund) => refund.kind === 'WITHDRAWAL').map((refund) => refund.purchaseId));
  const basis = purchases.filter((purchase) => purchase.state === 'PAID' && !withdrawn.has(purchase.id));
  const totalCredits = basis.reduce((acc, purchase) => acc + purchase.credits, 0);
  if (totalCredits === 0 || basis.length === 0) return { eligible: false, reason: 'no_paid_balance' };

  const unit = basis.reduce((acc, purchase) => acc + purchase.price, 0) / totalCredits;
  const amount = Math.floor((paidBalance / MILLI_PER_CREDIT) * unit * REMAINDER_REFUND_RATE);
  if (amount <= 0) return { eligible: false, reason: 'no_paid_balance' };

  const freeBalance = Math.max(
    sumBy(entries, (row) => row.freeDelta),
    0,
  );
  const cancels = allocateCancels(basis, refunds, amount);
  const allocated = cancels.reduce((acc, cancel) => acc + cancel.amount, 0);

  return {
    eligible: true,
    amount,
    delta: { paidDelta: 0 - paidBalance, freeDelta: 0 - freeBalance },
    cancels,
    shortfall: amount - allocated,
  };
};

export type ReconcileClassification = 'finalize' | 'mismatch' | 'fail' | 'defer';

export const classifyReconcile = (lookup: LookupPaymentResult, price: number): ReconcileClassification => {
  if (lookup.kind === 'paid') return lookup.amount === price ? 'finalize' : 'mismatch';
  if (lookup.kind === 'not-found') return 'fail';
  if (lookup.kind === 'not-paid' && FINAL_NOT_PAID_STATUSES.has(lookup.paymentStatus)) return 'fail';
  return 'defer';
};
