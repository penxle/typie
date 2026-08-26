import { TypieError } from '@typie/lib/errors';
import { findPrismCreditPack } from '@typie/prism';
import dayjs from 'dayjs';
import { and, asc, eq, lt } from 'drizzle-orm';
import {
  createDbId,
  db,
  first,
  firstOrThrow,
  PrismCreditEntries,
  PrismCreditPurchases,
  TableCode,
  UserBillingKeys,
  Users,
} from '#/db/index.ts';
import * as portone from '#/external/portone.ts';
import { pubsub } from '#/pubsub.ts';
import { opsAlert } from './ops-alert.ts';
import { lockUserPrismCredit } from './prism-credit.ts';
import { toMilli, validateEntry } from './prism-credit-core.ts';
import { classifyReconcile, RECONCILE_MIN_AGE_MS } from './prism-credit-purchase-core.ts';
import { lockUserSubscriptionState } from './subscription-lock.ts';
import type { PrismCreditPack } from '@typie/lib/enums';
import type { Transaction } from '#/db/index.ts';

export type PurchaseOutcome = { kind: 'paid' | 'pending' | 'failed'; purchaseId: string };

const RECONCILE_BATCH = 50;

const markFailed = async (tx: Transaction, purchaseId: string, failure: Record<string, unknown>) => {
  const row = await tx
    .select({ data: PrismCreditPurchases.data })
    .from(PrismCreditPurchases)
    .where(eq(PrismCreditPurchases.id, purchaseId))
    .then(firstOrThrow);

  await tx
    .update(PrismCreditPurchases)
    .set({ state: 'FAILED', data: { ...(row.data as Record<string, unknown>), failure } })
    .where(and(eq(PrismCreditPurchases.id, purchaseId), eq(PrismCreditPurchases.state, 'PENDING')));
};

export const finalizePurchase = async (
  tx: Transaction,
  { purchaseId, evidence }: { purchaseId: string; evidence: Record<string, unknown> },
): Promise<boolean> => {
  const pending = await tx
    .select({
      userId: PrismCreditPurchases.userId,
      credits: PrismCreditPurchases.credits,
      bonusCredits: PrismCreditPurchases.bonusCredits,
      data: PrismCreditPurchases.data,
    })
    .from(PrismCreditPurchases)
    .where(and(eq(PrismCreditPurchases.id, purchaseId), eq(PrismCreditPurchases.state, 'PENDING')))
    .then(first);
  if (!pending) return false;

  await lockUserPrismCredit(tx, pending.userId);

  const cas = await tx
    .update(PrismCreditPurchases)
    .set({ state: 'PAID', paidAt: dayjs(), data: { ...(pending.data as Record<string, unknown>), ...evidence } })
    .where(and(eq(PrismCreditPurchases.id, purchaseId), eq(PrismCreditPurchases.state, 'PENDING')))
    .returning({ id: PrismCreditPurchases.id })
    .then(first);
  if (!cas) return false;

  const purchaseDelta = { paidDelta: toMilli(pending.credits), freeDelta: 0 };
  validateEntry('PURCHASE', purchaseDelta);
  await tx
    .insert(PrismCreditEntries)
    .values({ userId: pending.userId, kind: 'PURCHASE', key: purchaseId, ...purchaseDelta })
    .onConflictDoNothing({ target: [PrismCreditEntries.kind, PrismCreditEntries.key] });

  if (pending.bonusCredits > 0) {
    const bonusDelta = { paidDelta: 0, freeDelta: toMilli(pending.bonusCredits) };
    validateEntry('BONUS', bonusDelta);
    await tx
      .insert(PrismCreditEntries)
      .values({ userId: pending.userId, kind: 'BONUS', key: purchaseId, ...bonusDelta })
      .onConflictDoNothing({ target: [PrismCreditEntries.kind, PrismCreditEntries.key] });
  }

  return true;
};

const enrichPurchaseReceipt = async (purchaseId: string) => {
  try {
    const receipt = await portone.getPaymentReceipt({ paymentId: purchaseId });
    if (!receipt) return;

    const row = await db
      .select({ data: PrismCreditPurchases.data })
      .from(PrismCreditPurchases)
      .where(eq(PrismCreditPurchases.id, purchaseId))
      .then(first);
    if (!row) return;

    await db
      .update(PrismCreditPurchases)
      .set({ data: { ...(row.data as Record<string, unknown>), receipt } })
      .where(eq(PrismCreditPurchases.id, purchaseId));
  } catch {
    return;
  }
};

const afterPaid = async (purchaseId: string, userId: string) => {
  pubsub.publish('prism:credit', userId, {});
  await enrichPurchaseReceipt(purchaseId);
};

export const purchasePrismCreditPack = async ({ userId, pack }: { userId: string; pack: PrismCreditPack }): Promise<PurchaseOutcome> => {
  const grid = findPrismCreditPack(pack);
  if (!grid) throw new TypieError({ code: 'invalid_pack', status: 400 });

  const purchaseId = await db.transaction(async (tx) => {
    await lockUserSubscriptionState(tx, userId);

    const billingKey = await tx
      .select({ type: UserBillingKeys.type })
      .from(UserBillingKeys)
      .where(eq(UserBillingKeys.userId, userId))
      .for('no key update')
      .then(first);
    if (!billingKey) throw new TypieError({ code: 'billing_key_required', status: 400 });

    const id = createDbId(TableCode.PRISM_CREDIT_PURCHASES);
    await tx.insert(PrismCreditPurchases).values({
      id,
      userId,
      pack,
      price: grid.price,
      credits: grid.credits,
      bonusCredits: grid.bonus,
      channel: 'BILLING_KEY',
      billingKeyType: billingKey.type,
      paymentKey: id,
      state: 'PENDING',
      data: {},
    });

    return id;
  });

  const outcome = await db.transaction(async (tx): Promise<PurchaseOutcome> => {
    await lockUserSubscriptionState(tx, userId);

    const billingKey = await tx
      .select({ billingKey: UserBillingKeys.billingKey })
      .from(UserBillingKeys)
      .where(eq(UserBillingKeys.userId, userId))
      .for('no key update')
      .then(first);
    if (!billingKey) return { kind: 'pending', purchaseId };

    const user = await tx.select({ name: Users.name, email: Users.email }).from(Users).where(eq(Users.id, userId)).then(firstOrThrow);

    const result = await portone.payWithBillingKey({
      paymentId: purchaseId,
      billingKey: billingKey.billingKey,
      customerName: user.name,
      customerEmail: user.email,
      orderName: `타이피 프리즘 ${grid.credits + grid.bonus} 크레딧`,
      amount: grid.price,
    });

    if (result.status === 'succeeded') {
      await finalizePurchase(tx, { purchaseId, evidence: { pgTxId: result.pgTxId, paidAt: result.paidAt } });
      return { kind: 'paid', purchaseId };
    }

    const lookup = await portone.lookupPayment({ paymentId: purchaseId });
    // 인라인 not-found는 결제가 아직 PortOne에 기록되기 전일 수 있다 — 확정은 2분 유예를 가진 크론에만 맡긴다
    switch (lookup.kind === 'not-found' ? 'defer' : classifyReconcile(lookup, grid.price)) {
      case 'finalize': {
        await finalizePurchase(tx, { purchaseId, evidence: { recoveredFromLookup: true } });
        return { kind: 'paid', purchaseId };
      }
      case 'mismatch': {
        await markFailed(tx, purchaseId, { code: 'amount_mismatch' });
        await opsAlert('already-paid-amount-mismatch', { purchaseId });
        return { kind: 'failed', purchaseId };
      }
      case 'fail': {
        await markFailed(tx, purchaseId, { code: result.code, message: result.message });
        return { kind: 'failed', purchaseId };
      }
      case 'defer': {
        return { kind: 'pending', purchaseId };
      }
    }
  });

  if (outcome.kind === 'paid') await afterPaid(outcome.purchaseId, userId);

  return outcome;
};

export const reconcilePendingPurchases = async () => {
  const stale = await db
    .select({
      id: PrismCreditPurchases.id,
      paymentKey: PrismCreditPurchases.paymentKey,
      userId: PrismCreditPurchases.userId,
      price: PrismCreditPurchases.price,
    })
    .from(PrismCreditPurchases)
    .where(
      and(
        eq(PrismCreditPurchases.state, 'PENDING'),
        eq(PrismCreditPurchases.channel, 'BILLING_KEY'),
        lt(PrismCreditPurchases.createdAt, dayjs().subtract(RECONCILE_MIN_AGE_MS, 'ms')),
      ),
    )
    .orderBy(asc(PrismCreditPurchases.createdAt))
    .limit(RECONCILE_BATCH);

  for (const purchase of stale) {
    const lookup = await portone.lookupPayment({ paymentId: purchase.paymentKey });

    switch (classifyReconcile(lookup, purchase.price)) {
      case 'finalize': {
        const finalized = await db.transaction((tx) => finalizePurchase(tx, { purchaseId: purchase.id, evidence: { reconciled: true } }));
        if (finalized) await afterPaid(purchase.id, purchase.userId);
        break;
      }
      case 'mismatch': {
        await db.transaction((tx) => markFailed(tx, purchase.id, { code: 'amount_mismatch', reconciled: true }));
        await opsAlert('already-paid-amount-mismatch', { purchaseId: purchase.id });
        break;
      }
      case 'fail': {
        await db.transaction((tx) =>
          markFailed(tx, purchase.id, {
            code: 'not_paid',
            status: lookup.kind === 'not-paid' ? lookup.paymentStatus : null,
            reconciled: true,
          }),
        );
        break;
      }
      case 'defer': {
        break;
      }
    }
  }
};
