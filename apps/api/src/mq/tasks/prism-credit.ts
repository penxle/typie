import * as Sentry from '@sentry/node';
import { sql } from 'drizzle-orm';
import { dbr, PrismCreditEntries, PrismCreditPurchases, PrismCreditRefunds } from '#/db/index.ts';
import { opsAlert } from '#/utils/ops-alert.ts';
import { reconcilePendingPurchases } from '#/utils/prism-credit-purchase.ts';
import { defineCron } from '../types.ts';
import type { SQL } from 'drizzle-orm';

const SAMPLE_LIMIT = 10;

type InvariantCheck = { key: string; violations: SQL };

const INVARIANT_CHECKS: InvariantCheck[] = [
  {
    key: 'prism-credit-free-negative',
    violations: sql`
      SELECT ${PrismCreditEntries.userId} AS id
      FROM ${PrismCreditEntries}
      GROUP BY ${PrismCreditEntries.userId}
      HAVING SUM(${PrismCreditEntries.freeDelta}) < 0
    `,
  },
  {
    key: 'prism-credit-refund-mismatch',
    violations: sql`
      SELECT r.id AS id
      FROM ${PrismCreditEntries} r
      LEFT JOIN ${PrismCreditEntries} c
        ON c.kind = 'REVIEW_CHARGE' AND c.key = r.key
      WHERE r.kind = 'REVIEW_REFUND'
        AND (c.id IS NULL OR c.paid_delta <> -r.paid_delta OR c.free_delta <> -r.free_delta OR c.user_id <> r.user_id)
    `,
  },
  {
    key: 'prism-credit-purchase-ledger-mismatch',
    violations: sql`
      SELECT p.id AS id
      FROM ${PrismCreditPurchases} p
      LEFT JOIN ${PrismCreditEntries} pe ON pe.kind = 'PURCHASE' AND pe.key = p.id
      LEFT JOIN ${PrismCreditEntries} be ON be.kind = 'BONUS' AND be.key = p.id
      WHERE p.state = 'PAID'
        AND (
          pe.id IS NULL OR pe.user_id <> p.user_id OR pe.paid_delta <> p.credits * 1000
          OR (p.bonus_credits > 0 AND (be.id IS NULL OR be.free_delta <> p.bonus_credits * 1000))
          OR (p.bonus_credits = 0 AND be.id IS NOT NULL)
        )
      UNION
      SELECT e.id AS id
      FROM ${PrismCreditEntries} e
      LEFT JOIN ${PrismCreditPurchases} p ON p.id = e.key AND p.state = 'PAID'
      WHERE e.kind IN ('PURCHASE', 'BONUS') AND p.id IS NULL
    `,
  },
  {
    key: 'prism-credit-refund-ledger-mismatch',
    violations: sql`
      SELECT r.id AS id
      FROM ${PrismCreditRefunds} r
      LEFT JOIN ${PrismCreditEntries} e ON e.kind = 'REFUND_OUT' AND e.key = r.id
      WHERE e.id IS NULL OR e.user_id <> r.user_id
      UNION
      SELECT e.id AS id
      FROM ${PrismCreditEntries} e
      LEFT JOIN ${PrismCreditRefunds} r ON r.id = e.key
      WHERE e.kind = 'REFUND_OUT' AND r.id IS NULL
    `,
  },
  {
    key: 'prism-credit-purchase-stuck',
    violations: sql`
      SELECT ${PrismCreditPurchases.id} AS id
      FROM ${PrismCreditPurchases}
      WHERE ${PrismCreditPurchases.state} = 'PENDING'
        AND ${PrismCreditPurchases.createdAt} < now() - interval '30 minutes'
    `,
  },
  {
    key: 'prism-credit-refund-stuck',
    violations: sql`
      SELECT ${PrismCreditRefunds.id} AS id
      FROM ${PrismCreditRefunds}
      WHERE ${PrismCreditRefunds.state} = 'PENDING'
        AND ${PrismCreditRefunds.createdAt} < now() - interval '30 minutes'
    `,
  },
];

const runInvariantCheck = async (check: InvariantCheck) => {
  try {
    const rows = await dbr.execute<{ count: number; sample_ids: string[] | null }>(sql`
      WITH violations AS (${check.violations})
      SELECT
        COUNT(*)::int AS count,
        COALESCE(
          (SELECT array_agg(v.id) FROM (SELECT id FROM violations ORDER BY id LIMIT ${SAMPLE_LIMIT}) v),
          ARRAY[]::text[]
        ) AS sample_ids
      FROM violations
    `);

    const row = rows[0];
    if (row && row.count > 0) {
      await opsAlert('invariant-violation', { check: check.key, count: row.count, sampleIds: row.sample_ids ?? [] });
    }
  } catch (err) {
    Sentry.captureException(err, { extra: { check: check.key } });
  }
};

export const PrismCreditInvariantsCron = defineCron('prism:credit-invariants', '*/30 * * * *', async () => {
  for (const check of INVARIANT_CHECKS) {
    await runInvariantCheck(check);
  }
});

export const PrismCreditPurchaseReconcileCron = defineCron('prism:credit-purchase-reconcile', '*/5 * * * *', async () => {
  await reconcilePendingPurchases();
});
