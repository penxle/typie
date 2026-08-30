import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, eq, sql } from 'drizzle-orm';
import { first, PrismCreditEntries } from '#/db/index.ts';
import { clampExpiringMilli, computeTrialRemainder, invertCharge, splitCharge, validateEntry } from './prism-credit-core.ts';
import type { Dayjs } from 'dayjs';
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

type GrantParams = { userId: string; kind: 'GRANT' | 'TRIAL'; amount: number; note?: string; actorId?: string; expiresAt?: Dayjs };
export const grantPrismCredit = async (
  tx: Transaction,
  { userId, kind, amount, note, actorId, expiresAt }: GrantParams,
): Promise<{ applied: boolean }> => {
  const delta = { paidDelta: 0, freeDelta: amount };
  validateEntry(kind, delta);

  await lockUserPrismCredit(tx, userId);

  const inserted = await tx
    .insert(PrismCreditEntries)
    .values({
      userId,
      kind,
      key: kind === 'TRIAL' ? userId : null,
      note: note ?? null,
      actorId: actorId ?? null,
      expiresAt: expiresAt ?? null,
      ...delta,
    })
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

export type TrialGrant = { id: string; userId: string; granted: number; consumedNet: number; expiresAt: Dayjs };

export const CONSUMED_NET_LATERAL = sql`
  CROSS JOIN LATERAL (
    SELECT coalesce(sum(e.free_delta), 0) AS net
    FROM prism_credit_entries e
    WHERE e.user_id = t.user_id
      AND e.kind NOT IN ('GRANT', 'TRIAL', 'BONUS')
      AND (e.created_at, e.id) > (t.created_at, t.id)
  ) s
`;

export const NOT_EXPIRED = sql`
  NOT EXISTS (SELECT 1 FROM prism_credit_entries x WHERE x.kind = 'EXPIRE' AND x.key = t.id)
`;

type TrialGrantRow = { id: string; user_id: string; granted: string; consumed_net: string; expires_at: string };

const toTrialGrant = (row: TrialGrantRow): TrialGrant => ({
  id: row.id,
  userId: row.user_id,
  granted: Number(row.granted),
  consumedNet: Number(row.consumed_net),
  expiresAt: dayjs(row.expires_at),
});

export const listExpirableTrialGrants = async (executor: Database | Transaction): Promise<TrialGrant[]> => {
  const rows = await executor.execute<TrialGrantRow>(sql`
    SELECT t.id, t.user_id, t.free_delta AS granted, s.net AS consumed_net, t.expires_at
    FROM prism_credit_entries t
    ${CONSUMED_NET_LATERAL}
    WHERE t.kind = 'TRIAL'
      AND t.expires_at IS NOT NULL
      AND t.expires_at <= now()
      AND ${NOT_EXPIRED}
      AND t.free_delta + s.net > 0
    ORDER BY t.expires_at, t.id
  `);

  return rows.map((row) => toTrialGrant(row));
};

export const readTrialGrant = async (executor: Database | Transaction, trialId: string): Promise<TrialGrant | null> => {
  const rows = await executor.execute<TrialGrantRow>(sql`
    SELECT t.id, t.user_id, t.free_delta AS granted, s.net AS consumed_net, t.expires_at
    FROM prism_credit_entries t
    ${CONSUMED_NET_LATERAL}
    WHERE t.kind = 'TRIAL'
      AND t.id = ${trialId}
      AND t.expires_at IS NOT NULL
      AND t.expires_at <= now()
      AND ${NOT_EXPIRED}
  `);

  const row = rows[0];

  return row ? toTrialGrant(row) : null;
};

export const readActiveTrialExpiry = async (
  executor: Database | Transaction,
  userId: string,
  total: number,
): Promise<{ milli: number; expiresAt: Dayjs } | null> => {
  const rows = await executor.execute<TrialGrantRow>(sql`
    SELECT t.id, t.user_id, t.free_delta AS granted, s.net AS consumed_net, t.expires_at
    FROM prism_credit_entries t
    ${CONSUMED_NET_LATERAL}
    WHERE t.kind = 'TRIAL'
      AND t.user_id = ${userId}
      AND t.expires_at IS NOT NULL
      AND t.expires_at > now()
      AND ${NOT_EXPIRED}
  `);

  const row = rows[0];
  if (!row) {
    return null;
  }

  const grant = toTrialGrant(row);
  const remainder = computeTrialRemainder({ granted: grant.granted, consumedNet: grant.consumedNet });
  if (remainder <= 0) {
    return null;
  }

  return { milli: clampExpiringMilli({ remainder, total }), expiresAt: grant.expiresAt };
};

export const expireTrialGrant = async (tx: Transaction, grant: TrialGrant): Promise<{ applied: boolean }> => {
  await lockUserPrismCredit(tx, grant.userId);

  const fresh = await readTrialGrant(tx, grant.id);
  if (!fresh) {
    return { applied: false };
  }

  const remainder = computeTrialRemainder({ granted: fresh.granted, consumedNet: fresh.consumedNet });
  if (remainder <= 0) {
    return { applied: false };
  }

  const delta = { paidDelta: 0, freeDelta: 0 - remainder };
  validateEntry('EXPIRE', delta);

  const inserted = await tx
    .insert(PrismCreditEntries)
    .values({ userId: fresh.userId, kind: 'EXPIRE', key: fresh.id, ...delta })
    .onConflictDoNothing({ target: [PrismCreditEntries.kind, PrismCreditEntries.key] })
    .returning({ id: PrismCreditEntries.id });

  return { applied: inserted.length > 0 };
};
