#!/usr/bin/env node

// 구독 권한/청구 분리 백필의 최종 감사. 0100(제약·인덱스) 적용 전 게이트이며 읽기 전용이다 — UPDATE 를 하지 않는다.
//
//   감사:    doppler run -- node scripts/audit-entitlement-split.ts --manifest=<백필 원장> 2>&1 | tee audit.log
//   EXPLAIN: node scripts/audit-entitlement-split.ts --explain   (쿼리 출력만 — 실행은 오너가 실분포에서)
//
// 전 항목 0건이어야 0100 을 적용할 수 있다(exit 0 = 통과, exit 1 = 위반 또는 원장 부재).
// 백필 원장(backfill-entitlement-split.ts 의 산출물)은 필수 입력이다 — 동결 인보이스와 인보이스 경로 분류가
// 여기에만 남아 있고, 둘 다 현재 DB 로는 재계산할 수 없다.
//
// 동결 인보이스는 시간 경계 정렬 게이트에서 제외한다 — 동결은 원본 due_at(또는 이미 승인 시도에 쓰인 값)을
// 보존하는 의도된 비정렬이라, 위반으로 띄우면 승인된 주기를 되돌리는 수리를 유도한다. 같은 이유로 종결 인보이스
// 연속성에서 동결 행이 낀 체인은 참고 항목으로만 표기하고 게이트를 막지 않는다.

import '@typie/lib/dayjs';

import fs from 'node:fs';
import path from 'node:path';
import { PaymentInvoiceState, PlanInterval, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { eq, sql } from 'drizzle-orm';
import { PgDialect } from 'drizzle-orm/pg-core';
import { db, PaymentInvoices, PaymentRecords, pg, pgb, pgr, Plans, Subscriptions, UserInAppPurchases } from '#/db/index.ts';
import { computeNextPeriodEnd } from '#/utils/billing-period.ts';
import type { SQL } from 'drizzle-orm';

const MANIFEST_VERSION = 1;
const DEFAULT_SAMPLE_LIMIT = 20;
const PARAM_PREVIEW_LIMIT = 10;

const TERMINAL_INVOICE_STATES = new Set<PaymentInvoiceState>([
  PaymentInvoiceState.PAID,
  PaymentInvoiceState.WAIVED,
  PaymentInvoiceState.CANCELED,
]);
const OPEN_INVOICE_STATES = new Set<PaymentInvoiceState>([PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE]);

type InvoicePath = 'RENEWAL' | 'TRANSITION' | 'AMBIGUOUS';

type FrozenInvoiceEntry = {
  invoiceId: string;
  subscriptionId: string;
  dueAtOriginal: string;
  interval: PlanInterval;
  servicePeriodStartsAt: string | null;
  servicePeriodEndsAt: string | null;
  frozenSource?: 'DUE_AT' | 'PERSISTED';
  frozenAt: string;
};

type ReservationPairEntry = {
  reservationId: string;
  predecessorId: string | null;
  oldBoundary: string | null;
  candidates: number;
};

type Manifest = {
  version: number;
  createdAt: string;
  frozenInvoices: FrozenInvoiceEntry[];
  reservationPairs: ReservationPairEntry[];
  invoicePaths: Record<string, InvoicePath>;
};

type RawTimestamp = Date | string | null;

const explainOnly = process.argv.includes('--explain');
const manifestPath = path.resolve(
  process.argv.find((arg) => arg.startsWith('--manifest='))?.slice('--manifest='.length) ??
    process.env.BACKFILL_MANIFEST_PATH ??
    'backfill-entitlement-split-manifest.json',
);
const sampleLimit = Number(process.argv.find((arg) => arg.startsWith('--samples='))?.slice('--samples='.length) ?? DEFAULT_SAMPLE_LIMIT);

const dialect = new PgDialect();

const section = (title: string) => {
  console.log(`\n=== ${title} ===`);
};

const indent = (text: string, prefix: string) => {
  const lines = text.replaceAll(/^\n+|\s+$/g, '').split('\n');
  const common = Math.min(...lines.filter((line) => line.trim().length > 0).map((line) => line.length - line.trimStart().length));

  return lines.map((line) => `${prefix}${line.slice(common)}`).join('\n');
};

const toDayjs = (value: RawTimestamp) => {
  return value === null ? null : dayjs(value);
};

const iso = (value: dayjs.Dayjs | null) => {
  return value === null ? 'null' : value.toISOString();
};

const same = (left: dayjs.Dayjs | null, right: dayjs.Dayjs | null) => {
  if (left === null || right === null) {
    return left === right;
  }

  return left.valueOf() === right.valueOf();
};

// 배열 파라미터는 드라이버가 개별 바인딩으로 펼쳐 버린다 — ARRAY 리터럴로 넘긴다.
const textArray = (values: string[]) => {
  if (values.length === 0) {
    return sql`ARRAY[]::text[]`;
  }

  return sql`ARRAY[${sql.join(
    values.map((value) => sql`${value}`),
    sql`, `,
  )}]::text[]`;
};

// 원장 없이 돌리면 동결 제외·경로 분류가 통째로 빠져 감사가 조용히 약해진다 — 부재는 통과가 아니라 중단이다.
const loadManifest = (): Manifest | null => {
  if (!fs.existsSync(manifestPath)) {
    console.error(`원장(${manifestPath})이 없다. 백필이 만든 manifest 경로를 --manifest= 로 지정한다.`);

    return null;
  }

  const parsed = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as Manifest;

  if (parsed.version !== MANIFEST_VERSION) {
    console.error(`원장 version=${parsed.version} 은 이 스크립트(version=${MANIFEST_VERSION})가 읽을 수 없다.`);

    return null;
  }

  return parsed;
};

// SQL 감사 항목.

type Check = {
  key: string;
  title: string;
  violations: SQL;
};

const buildChecks = (frozenInvoiceIds: string[]): Check[] => [
  {
    key: 'payment-key-null',
    title: '인보이스 payment_key NULL',
    violations: sql`
      SELECT ${PaymentInvoices.id} AS id
      FROM ${PaymentInvoices}
      WHERE ${PaymentInvoices.paymentKey} IS NULL
    `,
  },
  {
    key: 'payment-key-duplicate',
    title: '인보이스 payment_key 중복',
    violations: sql`
      SELECT ${PaymentInvoices.paymentKey} AS id
      FROM ${PaymentInvoices}
      WHERE ${PaymentInvoices.paymentKey} IS NOT NULL
      GROUP BY ${PaymentInvoices.paymentKey}
      HAVING COUNT(*) >= 2
    `,
  },
  {
    key: 'service-period-null',
    title: '인보이스 service_period_* NULL',
    violations: sql`
      SELECT ${PaymentInvoices.id} AS id
      FROM ${PaymentInvoices}
      WHERE ${PaymentInvoices.servicePeriodStartsAt} IS NULL
         OR ${PaymentInvoices.servicePeriodEndsAt} IS NULL
    `,
  },
  {
    key: 'service-period-duplicate',
    title: '서비스 주기 중복 (subscription_id, service_period_starts_at)',
    violations: sql`
      SELECT ${PaymentInvoices.subscriptionId} || '@' || ${PaymentInvoices.servicePeriodStartsAt} AS id
      FROM ${PaymentInvoices}
      WHERE ${PaymentInvoices.servicePeriodStartsAt} IS NOT NULL
      GROUP BY ${PaymentInvoices.subscriptionId}, ${PaymentInvoices.servicePeriodStartsAt}
      HAVING COUNT(*) >= 2
    `,
  },
  {
    key: 'open-invoice-duplicate',
    title: '구독당 열린 인보이스 2건 이상',
    violations: sql`
      SELECT ${PaymentInvoices.subscriptionId} AS id
      FROM ${PaymentInvoices}
      WHERE ${PaymentInvoices.state} IN ('UPCOMING', 'OVERDUE')
      GROUP BY ${PaymentInvoices.subscriptionId}
      HAVING COUNT(*) >= 2
    `,
  },
  {
    key: 'period-end-null',
    title: '구독 current_period_ends_at NULL',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      WHERE ${Subscriptions.currentPeriodEndsAt} IS NULL
    `,
  },
  {
    // 앵커 없는 행은 주기 계산 불능으로 무기한 fail-open 이 된다 — 상태로 좁히지 않는다(EXPIRED 도 되살아난다).
    key: 'anchor-missing',
    title: '빌링키 × MONTHLY/YEARLY 의 billing_anchor_at NULL',
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
    // IAP 는 주기를 스토어가 준다 — 스토어가 0길이 창을 보고하는 행이 실재하므로 등호는 위반이 아니다.
    // 역전(시작 > 종료)은 채널과 무관하게 위반이다.
    key: 'period-order-violation',
    title: '구독 주기 역전 (IAP 0길이는 스토어 보고값이므로 제외)',
    violations: sql`
      SELECT ${Subscriptions.id} AS id
      FROM ${Subscriptions}
      INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
      WHERE (
              ${Subscriptions.currentPeriodStartsAt} > ${Subscriptions.currentPeriodEndsAt}
              OR (${Subscriptions.currentPeriodStartsAt} = ${Subscriptions.currentPeriodEndsAt}
                  AND ${Plans.availability} != 'IN_APP_PURCHASE')
            )
        AND ${Plans.interval} != 'LIFETIME'
        AND ${Plans.availability} != 'MANUAL'
    `,
  },
  {
    key: 'service-period-order-violation',
    title: '인보이스 service_period_starts_at >= service_period_ends_at',
    violations: sql`
      SELECT ${PaymentInvoices.id} AS id
      FROM ${PaymentInvoices}
      WHERE ${PaymentInvoices.servicePeriodStartsAt} >= ${PaymentInvoices.servicePeriodEndsAt}
    `,
  },
  {
    key: 'payment-record-success-duplicate',
    title: '인보이스당 SUCCESS 레코드 2건 이상',
    violations: sql`
      SELECT ${PaymentRecords.invoiceId} AS id
      FROM ${PaymentRecords}
      WHERE ${PaymentRecords.outcome} = 'SUCCESS'
      GROUP BY ${PaymentRecords.invoiceId}
      HAVING COUNT(*) >= 2
    `,
  },
  {
    key: 'iap-binding-subscription-duplicate',
    title: '바인딩 canonical subscription_id 중복',
    violations: sql`
      SELECT ${UserInAppPurchases.subscriptionId} AS id
      FROM ${UserInAppPurchases}
      WHERE ${UserInAppPurchases.subscriptionId} IS NOT NULL
      GROUP BY ${UserInAppPurchases.subscriptionId}
      HAVING COUNT(*) >= 2
    `,
  },
  {
    // composite FK 가 막을 두 경우를 한 항목으로 본다 — 타 유저 참조와 존재하지 않는 구독 참조.
    key: 'iap-binding-subscription-foreign',
    title: '바인딩 subscription_id 가 타 유저 구독·부재 구독 참조',
    violations: sql`
      SELECT ${UserInAppPurchases.id} AS id
      FROM ${UserInAppPurchases}
      WHERE ${UserInAppPurchases.subscriptionId} IS NOT NULL
        AND NOT EXISTS (
          SELECT 1 FROM ${Subscriptions}
          WHERE ${Subscriptions.id} = ${UserInAppPurchases.subscriptionId}
            AND ${Subscriptions.userId} = ${UserInAppPurchases.userId}
        )
    `,
  },
  {
    // NULL 자체는 DDL 로 막지 않는다 — 해소 불가능한 바인딩은 terminated_at 으로 명시 격리해야 무음 join 탈락과 구분된다.
    key: 'iap-binding-unresolved',
    title: '바인딩 subscription_id NULL ∧ terminated_at 없음',
    violations: sql`
      SELECT ${UserInAppPurchases.id} AS id
      FROM ${UserInAppPurchases}
      WHERE ${UserInAppPurchases.subscriptionId} IS NULL
        AND ${UserInAppPurchases.terminatedAt} IS NULL
    `,
  },
  {
    key: 'invoice-hour-alignment',
    title: '인보이스 서비스 시작의 KST 시간 경계 정렬 (동결 인보이스 제외)',
    violations: sql`
      SELECT ${PaymentInvoices.id} AS id
      FROM ${PaymentInvoices}
      WHERE ${PaymentInvoices.servicePeriodStartsAt} IS NOT NULL
        AND ${PaymentInvoices.id} <> ALL(${textArray(frozenInvoiceIds)})
        AND ${PaymentInvoices.servicePeriodStartsAt} <>
            date_trunc('hour', ${PaymentInvoices.servicePeriodStartsAt} AT TIME ZONE 'Asia/Seoul') AT TIME ZONE 'Asia/Seoul'
    `,
  },
];

const runCheck = async (check: Check, index: number, total: number) => {
  const query = dialect.sqlToQuery(check.violations);

  console.log(`\n[${index}/${total}] ${check.key} — ${check.title}`);
  console.log(indent(query.sql, ' '.repeat(4)));
  if (query.params.length > 0) {
    const preview = query.params.slice(0, PARAM_PREVIEW_LIMIT).map(String);
    const suffix = query.params.length > PARAM_PREVIEW_LIMIT ? ` …외 ${query.params.length - PARAM_PREVIEW_LIMIT}건` : '';
    console.log(`    -- params: ${preview.join(', ')}${suffix}`);
  }

  const rows = await db.execute<{ count: number; sample_ids: string[] | null }>(sql`
    WITH violations AS (${check.violations})
    SELECT
      COUNT(*)::int AS count,
      COALESCE(
        (SELECT array_agg(v.id) FROM (SELECT id FROM violations ORDER BY id LIMIT ${sampleLimit}) v),
        ARRAY[]::text[]
      ) AS sample_ids
    FROM violations
  `);

  const count = rows[0]?.count ?? 0;
  const samples = rows[0]?.sample_ids ?? [];

  console.log(`  결과: ${count}건`);
  for (const sample of samples) {
    console.log(`    - ${sample}`);
  }
  if (count > samples.length) {
    console.log(`    … 외 ${count - samples.length}건 (--samples= 로 확장)`);
  }

  return count;
};

// Task 23 5단계 정합 게이트 재현 — 경로별 동일성·종결 연속성·예약 재구성.

type ProjectionRow = {
  invoiceId: string;
  invoiceState: PaymentInvoiceState;
  servicePeriodStartsAt: dayjs.Dayjs | null;
  servicePeriodEndsAt: dayjs.Dayjs | null;
  subscriptionId: string;
  periodStartsAt: dayjs.Dayjs;
  periodEndsAt: dayjs.Dayjs | null;
};

const runConsistencyGates = async (manifest: Manifest) => {
  const frozen = new Set(manifest.frozenInvoices.map((entry) => entry.invoiceId));
  const violations: string[] = [];
  const frozenNotes: string[] = [];

  const raw = await db
    .select({
      invoiceId: PaymentInvoices.id,
      invoiceState: PaymentInvoices.state,
      servicePeriodStartsAt: sql<RawTimestamp>`${PaymentInvoices.servicePeriodStartsAt}`,
      servicePeriodEndsAt: sql<RawTimestamp>`${PaymentInvoices.servicePeriodEndsAt}`,
      subscriptionId: Subscriptions.id,
      periodStartsAt: Subscriptions.currentPeriodStartsAt,
      periodEndsAt: sql<RawTimestamp>`${Subscriptions.currentPeriodEndsAt}`,
    })
    .from(PaymentInvoices)
    .innerJoin(Subscriptions, eq(Subscriptions.id, PaymentInvoices.subscriptionId));

  const rows: ProjectionRow[] = raw.map((row) => ({
    ...row,
    servicePeriodStartsAt: toDayjs(row.servicePeriodStartsAt),
    servicePeriodEndsAt: toDayjs(row.servicePeriodEndsAt),
    periodEndsAt: toDayjs(row.periodEndsAt),
  }));

  for (const row of rows) {
    if (!OPEN_INVOICE_STATES.has(row.invoiceState) || frozen.has(row.invoiceId)) {
      continue;
    }

    const invoicePath = manifest.invoicePaths[row.invoiceId];

    if (invoicePath === undefined || invoicePath === 'AMBIGUOUS') {
      violations.push(`[경로 미분류] ${row.invoiceId} (subscription=${row.subscriptionId} path=${invoicePath ?? 'MISSING'})`);
      continue;
    }

    if (invoicePath === 'RENEWAL' && !same(row.servicePeriodStartsAt, row.periodEndsAt)) {
      violations.push(
        `[정기갱신] ${row.invoiceId} service_start=${iso(row.servicePeriodStartsAt)} ≠ subscription.current_period_ends_at=${iso(row.periodEndsAt)}`,
      );
    }

    if (
      invoicePath === 'TRANSITION' &&
      (!same(row.servicePeriodStartsAt, row.periodStartsAt) || !same(row.servicePeriodEndsAt, row.periodEndsAt))
    ) {
      violations.push(
        `[전환] ${row.invoiceId} service=${iso(row.servicePeriodStartsAt)}~${iso(row.servicePeriodEndsAt)} ≠ subscription=${iso(row.periodStartsAt)}~${iso(row.periodEndsAt)}`,
      );
    }
  }

  const terminalBySubscription = new Map<string, { invoiceId: string; startsAt: dayjs.Dayjs; endsAt: dayjs.Dayjs }[]>();
  for (const row of rows) {
    if (!TERMINAL_INVOICE_STATES.has(row.invoiceState) || row.servicePeriodStartsAt === null || row.servicePeriodEndsAt === null) {
      continue;
    }

    const bucket = terminalBySubscription.get(row.subscriptionId) ?? [];
    bucket.push({ invoiceId: row.invoiceId, startsAt: row.servicePeriodStartsAt, endsAt: row.servicePeriodEndsAt });
    terminalBySubscription.set(row.subscriptionId, bucket);
  }

  for (const [subscriptionId, bucket] of terminalBySubscription) {
    const sorted = bucket.toSorted((a, b) => a.startsAt.valueOf() - b.startsAt.valueOf());

    for (let index = 1; index < sorted.length; index++) {
      const previous = sorted[index - 1];
      const current = sorted[index];

      if (same(previous.endsAt, current.startsAt)) {
        continue;
      }

      const overlapping = current.startsAt.isBefore(previous.endsAt);
      const message = `[종결 ${overlapping ? '중첩' : '불연속'}] ${subscriptionId}: ${previous.invoiceId}(~${iso(previous.endsAt)}) → ${current.invoiceId}(${iso(current.startsAt)}~)`;

      if (frozen.has(previous.invoiceId) || frozen.has(current.invoiceId)) {
        frozenNotes.push(message);
      } else {
        violations.push(message);
      }
    }
  }

  const reservations = await db
    .select({
      id: Subscriptions.id,
      startsAt: Subscriptions.startsAt,
      periodStartsAt: Subscriptions.currentPeriodStartsAt,
      periodEndsAt: sql<RawTimestamp>`${Subscriptions.currentPeriodEndsAt}`,
      billingAnchorAt: sql<RawTimestamp>`${Subscriptions.billingAnchorAt}`,
      interval: Plans.interval,
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Plans.id, Subscriptions.planId))
    .where(eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE));

  const predecessorIds = manifest.reservationPairs.map((pair) => pair.predecessorId).filter((id) => id !== null);
  const predecessorBoundaries = new Map<string, dayjs.Dayjs | null>();
  if (predecessorIds.length > 0) {
    const boundaries = await db
      .select({ id: Subscriptions.id, periodEndsAt: sql<RawTimestamp>`${Subscriptions.currentPeriodEndsAt}` })
      .from(Subscriptions)
      .where(sql`${Subscriptions.id} = ANY(${textArray(predecessorIds)})`);

    for (const boundary of boundaries) {
      predecessorBoundaries.set(boundary.id, toDayjs(boundary.periodEndsAt));
    }
  }

  for (const reservation of reservations) {
    const pair = manifest.reservationPairs.find((entry) => entry.reservationId === reservation.id);
    const periodEndsAt = toDayjs(reservation.periodEndsAt);
    const billingAnchorAt = toDayjs(reservation.billingAnchorAt);

    if (pair === undefined) {
      violations.push(`[예약 원장] ${reservation.id} 이 원장에 없다 (백필 이후 생성 — 수동 확인 대상)`);
    } else if (pair.predecessorId !== null) {
      const boundary = predecessorBoundaries.get(pair.predecessorId) ?? null;
      if (!same(reservation.startsAt, boundary)) {
        violations.push(
          `[예약 경계] ${reservation.id} starts_at=${iso(reservation.startsAt)} ≠ predecessor(${pair.predecessorId}).current_period_ends_at=${iso(boundary)}`,
        );
      }
    }

    if (billingAnchorAt === null || periodEndsAt === null) {
      violations.push(`[예약 주기] ${reservation.id} 앵커·주기 종료 미확정 (anchor=${iso(billingAnchorAt)} end=${iso(periodEndsAt)})`);
      continue;
    }

    if (reservation.interval !== PlanInterval.MONTHLY && reservation.interval !== PlanInterval.YEARLY) {
      violations.push(`[예약 주기] ${reservation.id} interval=${reservation.interval}`);
      continue;
    }

    const expected = computeNextPeriodEnd({
      periodStartsAt: reservation.periodStartsAt,
      interval: reservation.interval,
      billingAnchorAt,
    });

    if (!same(periodEndsAt, expected)) {
      violations.push(`[예약 주기] ${reservation.id} current_period_ends_at=${iso(periodEndsAt)} ≠ plan interval 투영=${iso(expected)}`);
    }
  }

  return { violations, frozenNotes };
};

// EXPLAIN 3종 — 실분포가 필요하므로 실행은 오너 몫이다(런북 배포 후 검증 단계).

const EXPLAIN_QUERIES = [
  {
    title: '매분 갱신 스캔 (subscription:billing-scan)',
    criteria: 'subscriptions_active_period_ends_index 로 경계 도달 행만 집는지 — 64k 행 전체 스캔이 매분 반복되면 실패',
    sql: `EXPLAIN (ANALYZE, BUFFERS)
SELECT s.id
FROM subscriptions s
INNER JOIN plans p ON p.id = s.plan_id
WHERE s.state = 'ACTIVE'
  AND s.current_period_ends_at <= now()
  AND p.availability = 'BILLING_KEY';`,
  },
  {
    title: 'OVERDUE 재시도 스캔 (subscription:renewal)',
    criteria: 'payment_invoices_overdue_service_start_index 로 OVERDUE 부분집합만 보는지',
    sql: `EXPLAIN (ANALYZE, BUFFERS)
SELECT id
FROM payment_invoices
WHERE state = 'OVERDUE'
  AND service_period_starts_at <= now();`,
  },
  {
    title: '유저 단위 권한 집계 로더 (User.entitlementRows)',
    criteria: 'subscriptions_user_id_state_index 로 유저 배치만 훑는지 — ARRAY 는 실제 유저 id 로 채운다',
    sql: `EXPLAIN (ANALYZE, BUFFERS)
SELECT s.*, p.availability
FROM subscriptions s
INNER JOIN plans p ON p.id = s.plan_id
WHERE s.user_id = ANY(ARRAY['<userId1>', '<userId2>']::text[])
  AND s.state <> 'EXPIRED'
ORDER BY s.created_at ASC, s.id ASC;`,
  },
];

const printExplainQueries = () => {
  section('EXPLAIN 3종 (실행은 오너 — 백필 후 실분포에서)');
  console.log('  판정 기준: seq scan 존재 자체는 실패가 아니다(소규모 테이블에서는 정당한 선택).');
  console.log('  실제 rows·buffers 가 대상 규모에 비례하고, 64k 행 전체 스캔이 매분 반복되는 형태가 아닐 것.');

  for (const [index, query] of EXPLAIN_QUERIES.entries()) {
    console.log(`\n[${index + 1}/${EXPLAIN_QUERIES.length}] ${query.title}`);
    console.log(`  기준: ${query.criteria}`);
    console.log(indent(query.sql, ' '.repeat(4)));
  }
};

const main = async () => {
  if (explainOnly) {
    printExplainQueries();

    return;
  }

  console.log(`감사 시작 — now=${dayjs().toISOString()}`);
  console.log(`원장: ${manifestPath}`);

  const manifest = loadManifest();
  if (manifest === null) {
    process.exitCode = 1;

    return;
  }

  const frozenInvoiceIds = manifest.frozenInvoices.map((entry) => entry.invoiceId);
  console.log(`  동결 인보이스: ${frozenInvoiceIds.length}건 / 경로 분류: ${Object.keys(manifest.invoicePaths).length}건`);
  console.log(`  예약 페어: ${manifest.reservationPairs.length}건`);

  const checks = buildChecks(frozenInvoiceIds);
  const failed: string[] = [];

  section('SQL 감사 항목');
  for (const [index, check] of checks.entries()) {
    const count = await runCheck(check, index + 1, checks.length);
    if (count > 0) {
      failed.push(`${check.key} ${count}건`);
    }
  }

  section('정합 게이트 (백필 5단계 재현 — 경로별 동일성·종결 연속성·예약 재구성)');
  const { violations, frozenNotes } = await runConsistencyGates(manifest);
  console.log(`  위반: ${violations.length}건`);
  for (const violation of violations) {
    console.log(`    - ${violation}`);
  }
  console.log(`  동결 행이 낀 종결 체인(참고 — 의도된 비정렬, 게이트 막지 않음): ${frozenNotes.length}건`);
  for (const note of frozenNotes) {
    console.log(`    - ${note}`);
  }

  if (violations.length > 0) {
    failed.push(`정합 게이트 ${violations.length}건`);
  }

  section('요약');
  if (failed.length === 0) {
    console.log('  전 항목 0건 — 0100 적용 가능');
  } else {
    console.log(`  위반 ${failed.length}항목 — 0100 적용 불가`);
    for (const entry of failed) {
      console.log(`    - ${entry}`);
    }
    process.exitCode = 1;
  }

  printExplainQueries();
};

// process.exit 는 파이프(tee)로 흘려보낸 출력을 자르므로 exitCode 만 세우고 커넥션을 닫아 자연 종료한다.
await main();
await pg.end();
await pgr.end();
await pgb.end();
