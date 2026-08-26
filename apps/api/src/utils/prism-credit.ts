import { TypieError } from '@typie/lib/errors';
import { and, eq, sql } from 'drizzle-orm';
import { first, PrismCreditEntries } from '#/db/index.ts';
import { invertCharge, splitCharge, validateEntry } from './prism-credit-core.ts';
import type { Database, Transaction } from '#/db/index.ts';

export type PrismCreditBalance = { paid: number; free: number; total: number };

export const lockUserPrismCredit = async (tx: Transaction, userId: string) => {
  await tx.execute(sql`SET LOCAL lock_timeout = '15s'`);
  await tx.execute(sql`SELECT pg_advisory_xact_lock(hashtextextended(${userId}, 1))`);
};

export const readPrismCreditBalance = async (executor: Database | Transaction, userId: string): Promise<PrismCreditBalance> => {
  const row = await executor
    .select({
      paid: sql<string>`coalesce(sum(${PrismCreditEntries.paidDelta}), 0)`.mapWith(Number),
      free: sql<string>`coalesce(sum(${PrismCreditEntries.freeDelta}), 0)`.mapWith(Number),
    })
    .from(PrismCreditEntries)
    .where(eq(PrismCreditEntries.userId, userId))
    .then(first);

  const paid = row?.paid ?? 0;
  const free = row?.free ?? 0;

  return { paid, free, total: paid + free };
};

type ChargeParams = { userId: string; kind: 'REVIEW_CHARGE' | 'CHAT_CHARGE'; key: string; amount: number };
export const chargePrismCredit = async (tx: Transaction, { userId, kind, key, amount }: ChargeParams): Promise<{ applied: boolean }> => {
  await lockUserPrismCredit(tx, userId);

  const { free } = await readPrismCreditBalance(tx, userId);
  const delta = splitCharge({ free, amount });
  validateEntry(kind, delta);

  const inserted = await tx
    .insert(PrismCreditEntries)
    .values({ userId, kind, key, ...delta })
    .onConflictDoNothing({ target: [PrismCreditEntries.kind, PrismCreditEntries.key] })
    .returning({ id: PrismCreditEntries.id });

  return { applied: inserted.length > 0 };
};

export type RefundResult = { applied: true } | { applied: false; reason: 'no_charge' | 'already_refunded' };
export const refundPrismReview = async (tx: Transaction, { roundId }: { roundId: string }): Promise<RefundResult> => {
  const charge = await tx
    .select({ userId: PrismCreditEntries.userId, paidDelta: PrismCreditEntries.paidDelta, freeDelta: PrismCreditEntries.freeDelta })
    .from(PrismCreditEntries)
    .where(and(eq(PrismCreditEntries.kind, 'REVIEW_CHARGE'), eq(PrismCreditEntries.key, roundId)))
    .then(first);

  if (!charge) {
    return { applied: false, reason: 'no_charge' };
  }

  await lockUserPrismCredit(tx, charge.userId);

  const delta = invertCharge(charge);
  validateEntry('REVIEW_REFUND', delta);

  const inserted = await tx
    .insert(PrismCreditEntries)
    .values({ userId: charge.userId, kind: 'REVIEW_REFUND', key: roundId, ...delta })
    .onConflictDoNothing({ target: [PrismCreditEntries.kind, PrismCreditEntries.key] })
    .returning({ id: PrismCreditEntries.id });

  if (inserted.length === 0) {
    return { applied: false, reason: 'already_refunded' };
  }

  return { applied: true };
};

type GrantParams = { userId: string; kind: 'GRANT' | 'TRIAL'; amount: number; note?: string; actorId?: string };
export const grantPrismCredit = async (
  tx: Transaction,
  { userId, kind, amount, note, actorId }: GrantParams,
): Promise<{ applied: boolean }> => {
  const delta = { paidDelta: 0, freeDelta: amount };
  validateEntry(kind, delta);

  await lockUserPrismCredit(tx, userId);

  const inserted = await tx
    .insert(PrismCreditEntries)
    .values({ userId, kind, key: kind === 'TRIAL' ? userId : null, note: note ?? null, actorId: actorId ?? null, ...delta })
    .onConflictDoNothing({ target: [PrismCreditEntries.kind, PrismCreditEntries.key] })
    .returning({ id: PrismCreditEntries.id });

  return { applied: inserted.length > 0 };
};

type AdjustParams = { userId: string; paidDelta: number; freeDelta: number; note: string; actorId: string };
export const adjustPrismCredit = async (tx: Transaction, { userId, paidDelta, freeDelta, note, actorId }: AdjustParams): Promise<void> => {
  const delta = { paidDelta, freeDelta };
  validateEntry('ADJUSTMENT', delta);

  await lockUserPrismCredit(tx, userId);

  const { free } = await readPrismCreditBalance(tx, userId);
  if (free + freeDelta < 0) {
    throw new TypieError({ code: 'prism_credit_free_negative', status: 400 });
  }

  await tx.insert(PrismCreditEntries).values({ userId, kind: 'ADJUSTMENT', key: null, note, actorId, ...delta });
};
