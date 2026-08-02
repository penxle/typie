import * as Sentry from '@sentry/node';
import { sql } from 'drizzle-orm';
import { dbr, PaymentInvoices, Plans, Subscriptions, UserBillingKeys, UserInAppPurchases } from '#/db/index.ts';
import { opsAlert } from '#/utils/ops-alert.ts';
import { queue } from '../bullmq.ts';
import { defineCron } from '../types.ts';
import type { SQL } from 'drizzle-orm';

const SAMPLE_LIMIT = 10;
const QUEUE_STUCK_THRESHOLD_MS = 30 * 60 * 1000;

type InvariantCheck = { key: string; violations: SQL };

// 권한이 상태에서만 나오므로 크론이 죽으면 유저는 잠기지 않고 무료로 더 쓴다(fail-open, 의도된 방향).
// 상태 고착은 조용히 새므로 이 13개가 그 대가로 서는 상시 감시다 — 각 행은 규범 목록을 그대로 옮긴 것이다.
const INVARIANT_CHECKS: InvariantCheck[] = [
  {
    key: 'billing-overdue-scan',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE ${Subscriptions.state} = 'ACTIVE'
        AND ${Plans.availability} = 'BILLING_KEY'
        AND ${Subscriptions.currentPeriodEndsAt} < now() - interval '14 hours'
    `,
  },
  {
    key: 'iap-sync-stalled',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE ${Subscriptions.state} = 'ACTIVE'
        AND ${Plans.availability} = 'IN_APP_PURCHASE'
        AND ${Subscriptions.currentPeriodEndsAt} < now() - interval '2 days'
    `,
  },
  {
    key: 'iap-orphan-live-row',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE ${Plans.availability} = 'IN_APP_PURCHASE'
        AND ${Subscriptions.state} IN ('ACTIVE', 'WILL_EXPIRE', 'IN_GRACE_PERIOD')
        AND NOT EXISTS (
          SELECT 1 FROM ${UserInAppPurchases} WHERE ${UserInAppPurchases.subscriptionId} = ${Subscriptions.id}
        )
    `,
  },
  {
    // WILL_EXPIRE 는 제외한다 — 전이 크론이 기간 종료와 함께 종결시키므로 다시 청구하지 않는다. 빌링키가 없는 것이
    // 정상이며, 되살아나면 ACTIVE 로 돌아와 다시 이 체크에 걸린다.
    key: 'billing-key-missing',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE ${Plans.availability} = 'BILLING_KEY'
        AND ${Subscriptions.state} IN ('ACTIVE', 'WILL_ACTIVATE', 'IN_GRACE_PERIOD')
        AND NOT EXISTS (
          SELECT 1 FROM ${UserBillingKeys} WHERE ${UserBillingKeys.userId} = ${Subscriptions.userId}
        )
    `,
  },
  {
    // 유예 마감 파생식은 통계 재투영(graphql/resolvers/stats.ts entitledOnDate)과 동일하다 — 채널별 fallback을
    // 빠뜨리면 NULL 비교로 위반 행이 조용히 스캔에서 빠진다.
    key: 'grace-deadline-passed',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE ${Subscriptions.state} = 'IN_GRACE_PERIOD'
        AND CASE
              WHEN ${Plans.availability} = 'IN_APP_PURCHASE' THEN ${Subscriptions.currentPeriodEndsAt} + interval '31 days'
              ELSE COALESCE(GREATEST(
                CASE WHEN ${Subscriptions.currentPeriodStartsAt} <= now() THEN ${Subscriptions.currentPeriodStartsAt} END,
                CASE WHEN ${Subscriptions.currentPeriodEndsAt} <= now() THEN ${Subscriptions.currentPeriodEndsAt} END
              ), ${Subscriptions.currentPeriodStartsAt}) + interval '7 days'
            END < now() - interval '1 day'
    `,
  },
  {
    key: 'will-expire-stuck',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE ${Subscriptions.state} = 'WILL_EXPIRE'
        AND ${Plans.availability} != 'IN_APP_PURCHASE'
        AND ${Subscriptions.currentPeriodEndsAt} < now() - interval '1 hour'
    `,
  },
  {
    key: 'overdue-invoice-orphan',
    violations: sql`
      SELECT ${PaymentInvoices.id} AS id
      FROM ${PaymentInvoices}
      INNER JOIN ${Subscriptions} ON ${PaymentInvoices.subscriptionId} = ${Subscriptions.id}
      WHERE ${PaymentInvoices.state} = 'OVERDUE'
        AND ${Subscriptions.state} != 'IN_GRACE_PERIOD'
    `,
  },
  {
    // 배포 후에는 부분 유니크(payment_invoices_open_subscription_unique)가 DB 레벨에서 막는다 — 이 체크는
    // 백필 구간(§7 7단계, 제약 생성 전)과 그 제약을 우회하는 회귀의 방어망이다.
    key: 'open-invoice-duplicate',
    violations: sql`
      SELECT ${PaymentInvoices.subscriptionId} AS id
      FROM ${PaymentInvoices}
      WHERE ${PaymentInvoices.state} IN ('UPCOMING', 'OVERDUE')
      GROUP BY ${PaymentInvoices.subscriptionId}
      HAVING COUNT(*) >= 2
    `,
  },
  {
    key: 'grace-without-open-invoice',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE ${Subscriptions.state} = 'IN_GRACE_PERIOD'
        AND ${Plans.availability} = 'BILLING_KEY'
        AND NOT EXISTS (
          SELECT 1 FROM ${PaymentInvoices}
          WHERE ${PaymentInvoices.subscriptionId} = ${Subscriptions.id}
            AND ${PaymentInvoices.state} IN ('UPCOMING', 'OVERDUE')
        )
    `,
  },
  {
    key: 'transition-missed',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      WHERE ${Subscriptions.state} = 'WILL_ACTIVATE'
        AND ${Subscriptions.startsAt} < now() - interval '1 hour'
    `,
  },
  {
    // 브리프 표 그대로 상태 무관 전체 대상이다(§7 "게이트·상시 모니터에 anchor NULL 0건" — EXPIRED 도 포함) —
    // 앵커 없는 행은 언제 되살아나도 주기 계산이 불능이므로 상태로 좁히지 않는다.
    key: 'anchor-missing',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE ${Plans.availability} = 'BILLING_KEY'
        AND ${Plans.interval} IN ('MONTHLY', 'YEARLY')
        AND ${Subscriptions.billingAnchorAt} IS NULL
    `,
  },
  {
    // LIFETIME sentinel(9999-12-31)·MANUAL 플랜은 기간 검증 대상이 아니다.
    key: 'period-order-violation',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE ${Subscriptions.currentPeriodStartsAt} >= ${Subscriptions.currentPeriodEndsAt}
        AND ${Plans.interval} != 'LIFETIME'
        AND ${Plans.availability} != 'MANUAL'
    `,
  },
  {
    key: 'service-period-order-violation',
    violations: sql`
      SELECT ${PaymentInvoices.id} AS id
      FROM ${PaymentInvoices}
      WHERE ${PaymentInvoices.servicePeriodStartsAt} >= ${PaymentInvoices.servicePeriodEndsAt}
    `,
  },
];

// 체크 1개의 쿼리 오류가 나머지 12개·큐 계측을 막지 않는다 — 격리 없이 하나가 던지면 루프가 끊겨 그 뒤 체크가
// 통째로 30분(다음 스케줄)을 건너뛴다.
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

// 실패 수는 removeOnFail: true(bullmq.ts) 라 큐에서 셀 수 없다 — worker 의 failed 훅(Sentry 캡처)이 그 몫이다.
// 여기서는 세 상태 전부의 최고령 잡을 본다 — active 를 빼면 외부 호출 중 고착된 워커가 waiting·delayed
// 조회에 잡히지 않고, 실패 훅도 실패 전에는 침묵한다. 세 상태를 독립 try/catch 로 감싼다 — 하나(예: redis
// 순단)가 실패해도 나머지 상태 계측은 이번 사이클에서 살아남는다.
const checkQueueHealth = async () => {
  const now = Date.now();

  try {
    const [oldestWaiting] = await queue.getWaiting(0, 0);
    if (oldestWaiting && now - oldestWaiting.timestamp > QUEUE_STUCK_THRESHOLD_MS) {
      await opsAlert('invariant-violation', {
        check: 'queue-waiting-stuck',
        count: 1,
        sampleIds: oldestWaiting.id ? [oldestWaiting.id] : [],
      });
    }
  } catch (err) {
    Sentry.captureException(err, { extra: { check: 'queue-waiting-stuck' } });
  }

  try {
    // delayed 의 정렬 키는 실행 예정 시각(score)이라 생성 시각 최고령이 아니다 — 예정 초과분이 집행 지연을 잰다.
    const [nextDelayed] = await queue.getDelayed(0, 0);
    if (nextDelayed) {
      const scheduledAt = nextDelayed.timestamp + nextDelayed.delay;
      if (now - scheduledAt > QUEUE_STUCK_THRESHOLD_MS) {
        await opsAlert('invariant-violation', {
          check: 'queue-delayed-stuck',
          count: 1,
          sampleIds: nextDelayed.id ? [nextDelayed.id] : [],
        });
      }
    }
  } catch (err) {
    Sentry.captureException(err, { extra: { check: 'queue-delayed-stuck' } });
  }

  try {
    const activeJobs = await queue.getActive(0, -1);
    // 오름차순 정렬 후 임계 초과분만 남긴다 — 정렬한 배열의 앞쪽이 곧 최고령이라 그대로 표본이 된다.
    // count 는 전체 active 수가 아니라 실제 고착(임계 초과) 수다 — 정상 처리 중인 잡까지 세면 알람이 과장된다.
    const stuck = activeJobs.filter((job) => now - job.timestamp > QUEUE_STUCK_THRESHOLD_MS).toSorted((a, b) => a.timestamp - b.timestamp);
    if (stuck.length > 0) {
      await opsAlert('invariant-violation', {
        check: 'queue-active-stuck',
        count: stuck.length,
        sampleIds: stuck
          .slice(0, SAMPLE_LIMIT)
          .map((job) => job.id)
          .filter((id): id is string => !!id),
      });
    }
  } catch (err) {
    Sentry.captureException(err, { extra: { check: 'queue-active-stuck' } });
  }
};

export const SubscriptionInvariantsCron = defineCron('subscription:invariants', '*/30 * * * *', async () => {
  for (const check of INVARIANT_CHECKS) {
    await runInvariantCheck(check);
  }

  await checkQueueHealth();
});
