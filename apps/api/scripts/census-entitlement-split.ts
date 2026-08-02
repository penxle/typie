#!/usr/bin/env node

// 구독 권한/청구 분리 백필의 수동 목록 규모를 사전 조사한다(순수 SELECT — readonly URL 로 실행 가능, 정지 창 불필요).
// backfill-entitlement-split.ts / backfill-iap-periods.ts 의 분류 술어를 구 스키마(0099 이전) 컬럼으로 근사한다.
// 인보이스 경로·서비스 주기 중복 판정은 근사이므로, 확정 목록은 스냅샷 복제본에서의 드라이런으로만 얻는다.
//
//   doppler run -- node scripts/census-entitlement-split.ts 2>&1 | tee census.log   (DATABASE_RO_URL 우선 사용)
//   또는 node scripts/census-entitlement-split.ts --url='<readonly url>' [--samples=20]

import postgres from 'postgres';

const url =
  process.argv.find((arg) => arg.startsWith('--url='))?.slice('--url='.length) ??
  process.env.READONLY_DATABASE_URL ??
  process.env.DATABASE_RO_URL ??
  process.env.DATABASE_URL;

if (!url) {
  console.error('DATABASE_URL(또는 --url=)이 필요하다.');
  process.exit(1);
}

const samples = Number(process.argv.find((arg) => arg.startsWith('--samples='))?.slice('--samples='.length) ?? 20);

// ssl 은 api 본체(db/index.ts)와 동일 — RDS Proxy 가 TLS 를 요구한다.
const sql = postgres(url, { max: 1, prepare: false, ssl: 'prefer' });

type Row = Record<string, unknown>;

const section = (title: string) => {
  console.log(`\n=== ${title} ===`);
};

const listing = (label: string, entries: string[]) => {
  console.log(`  ${label}: ${entries.length}건`);
  for (const entry of entries.slice(0, samples)) {
    console.log(`    - ${entry}`);
  }
  if (entries.length > samples) {
    console.log(`    … 외 ${entries.length - samples}건 (--samples= 로 조정)`);
  }
};

const iso = (value: unknown) => (value instanceof Date ? value.toISOString() : String(value));

const main = async (tx: postgres.TransactionSql) => {
  section('0. 컨텍스트');

  const byState = await tx`SELECT state, count(*)::int AS cnt FROM subscriptions GROUP BY state ORDER BY state`;
  console.log(`  구독 상태별: ${byState.map((row) => `${row.state}=${row.cnt}`).join(' / ')}`);

  const byPlan = await tx`
    SELECT p.availability, p.interval, count(*)::int AS cnt
    FROM subscriptions s JOIN plans p ON p.id = s.plan_id
    WHERE s.state IN ('ACTIVE', 'WILL_EXPIRE', 'IN_GRACE_PERIOD')
    GROUP BY p.availability, p.interval ORDER BY p.availability, p.interval`;
  console.log(`  살아있는 구독(플랜별): ${byPlan.map((row) => `${row.availability}/${row.interval}=${row.cnt}`).join(' / ')}`);

  const byInvoice = await tx`SELECT state, count(*)::int AS cnt FROM payment_invoices GROUP BY state ORDER BY state`;
  console.log(`  인보이스 상태별: ${byInvoice.map((row) => `${row.state}=${row.cnt}`).join(' / ')}`);

  const boundaryPassed = await tx`
    SELECT count(*)::int AS cnt
    FROM subscriptions s JOIN plans p ON p.id = s.plan_id
    WHERE p.availability = 'BILLING_KEY' AND p.interval IN ('MONTHLY', 'YEARLY')
      AND s.state IN ('ACTIVE', 'WILL_EXPIRE', 'IN_GRACE_PERIOD') AND s.expires_at <= now()`;
  console.log(`  경계 경과(만료 지남·살아있음, 참고): ${boundaryPassed[0].cnt}건`);

  section('1. 동결 인보이스 — blockedSubscriptionIds 원천');

  // 열린 인보이스 + 결제 레코드 = 동결. 이 구독들은 주기·앵커 교정 전체가 보류된다(blocked* 4목록의 공통 원천).
  const frozen: Row[] = await tx`
    SELECT pi.id AS invoice_id, pi.subscription_id, pi.state, pi.due_at, p.interval
    FROM payment_invoices pi
    JOIN subscriptions s ON s.id = pi.subscription_id
    JOIN plans p ON p.id = s.plan_id
    WHERE pi.state IN ('UPCOMING', 'OVERDUE')
      AND EXISTS (SELECT 1 FROM payment_records pr WHERE pr.invoice_id = pi.id)
    ORDER BY pi.due_at`;

  const blockingSubscriptionIds = new Set(frozen.map((row) => String(row.subscription_id)));
  listing(
    '동결 인보이스(열린 상태 + 결제 레코드)',
    frozen.map((row) => `invoice=${row.invoice_id} subscription=${row.subscription_id} state=${row.state} due_at=${iso(row.due_at)}`),
  );
  console.log(`  → 보류 구독(blockedSubscriptionIds): ${blockingSubscriptionIds.size}건`);
  listing(
    'frozenInvoiceUnsupportedInterval 예상(동결 중 interval 미지원)',
    frozen
      .filter((row) => row.interval !== 'MONTHLY' && row.interval !== 'YEARLY')
      .map((row) => `invoice=${row.invoice_id} (interval=${row.interval})`),
  );

  section('2. 0길이 주기 재구성 대상');

  const zeroLength: Row[] = await tx`
    SELECT s.id, s.state, s.starts_at, p.interval
    FROM subscriptions s JOIN plans p ON p.id = s.plan_id
    WHERE s.expires_at = s.starts_at
      AND (s.state = 'IN_GRACE_PERIOD'
        OR (s.state = 'EXPIRED' AND EXISTS (SELECT 1 FROM payment_invoices pi WHERE pi.subscription_id = s.id)))
    ORDER BY s.starts_at`;

  listing(
    '0길이 주기(expires_at = starts_at)',
    zeroLength.map(
      (row) =>
        `${row.id} state=${row.state} starts_at=${iso(row.starts_at)} interval=${row.interval}${blockingSubscriptionIds.has(String(row.id)) ? ' [동결 보류 → blockedZeroLengthSubscriptionIds]' : ''}`,
    ),
  );

  section('3. 밀린 결제일 — laggingSubscriptionIds (신 워커 기동 전 전건 해소 필수)');

  // 백필 판정: current_period_ends_at(≒ expires_at) <= now - 1 interval. 0길이 주기는 밀림 판정 전에
  // starts_at + 1 interval 로 재구성되므로 같은 값으로 평가한다. 세션 timezone 이 Asia/Seoul 이라 달력 가산이 KST 로 맞는다.
  const lagging: Row[] = await tx`
    SELECT s.id, s.state, s.expires_at, p.interval
    FROM subscriptions s JOIN plans p ON p.id = s.plan_id
    WHERE p.availability = 'BILLING_KEY' AND p.interval IN ('MONTHLY', 'YEARLY')
      AND s.state IN ('ACTIVE', 'WILL_EXPIRE', 'IN_GRACE_PERIOD')
      AND (CASE WHEN s.expires_at = s.starts_at THEN s.starts_at + (CASE p.interval WHEN 'MONTHLY' THEN interval '1 month' ELSE interval '1 year' END) ELSE s.expires_at END)
        <= now() - (CASE p.interval WHEN 'MONTHLY' THEN interval '1 month' ELSE interval '1 year' END)
    ORDER BY s.expires_at`;

  listing(
    '여러 주기 밀림(1 interval 이상 경과)',
    lagging.map(
      (row) =>
        `${row.id} state=${row.state} expires_at=${iso(row.expires_at)} interval=${row.interval}${blockingSubscriptionIds.has(String(row.id)) ? ' [동결 보류 → blockedLagCheckSubscriptionIds]' : ''}`,
    ),
  );

  section('4. EXPIRED 인데 만료 시각이 미래 — clip 대상(자동 교정, 참고)');

  const clip = await tx`SELECT count(*)::int AS cnt FROM subscriptions WHERE state = 'EXPIRED' AND expires_at > now()`;
  console.log(`  clip 대상: ${clip[0].cnt}건`);

  section('5. 열린 인보이스 경로 분류 — invoicePathAmbiguous / manualInvoiceIds');

  // TRANSITION 꼴 = due_at 이 구독 starts_at 과 일치. 꼴과 구독 상태가 어긋나면 AMBIGUOUS(수동).
  const openPaths: Row[] = await tx`
    SELECT pi.id AS invoice_id, pi.subscription_id, s.state AS subscription_state, p.interval,
           (pi.due_at = s.starts_at) AS transition_shaped
    FROM payment_invoices pi
    JOIN subscriptions s ON s.id = pi.subscription_id
    JOIN plans p ON p.id = s.plan_id
    WHERE pi.state IN ('UPCOMING', 'OVERDUE')
      AND NOT EXISTS (SELECT 1 FROM payment_records pr WHERE pr.invoice_id = pi.id)
    ORDER BY pi.id`;

  const pathOf = (row: Row) => {
    const shaped = row.transition_shaped === true;
    const reserved = row.subscription_state === 'WILL_ACTIVATE';
    const live = row.subscription_state === 'ACTIVE' || row.subscription_state === 'WILL_EXPIRE';
    if ((shaped && live) || (!shaped && reserved)) {
      return 'AMBIGUOUS';
    }
    return shaped ? 'TRANSITION' : 'RENEWAL';
  };

  const pathCounts = { RENEWAL: 0, TRANSITION: 0, AMBIGUOUS: 0 };
  for (const row of openPaths) {
    pathCounts[pathOf(row)] += 1;
  }
  console.log(`  RENEWAL=${pathCounts.RENEWAL} / TRANSITION=${pathCounts.TRANSITION} / AMBIGUOUS=${pathCounts.AMBIGUOUS}`);
  listing(
    'invoicePathAmbiguous 예상',
    openPaths
      .filter((row) => pathOf(row) === 'AMBIGUOUS')
      .map((row) => `invoice=${row.invoice_id} subscription=${row.subscription_id} state=${row.subscription_state}`),
  );
  listing(
    'manualInvoiceIds 예상(열린 인보이스인데 구독 interval 미지원)',
    openPaths
      .filter((row) => row.interval !== 'MONTHLY' && row.interval !== 'YEARLY')
      .map((row) => `invoice=${row.invoice_id} (interval=${row.interval})`),
  );

  section('6. 구독당 열린 인보이스 2건 이상 — duplicateOpenInvoiceIds');

  const openDuplicates: Row[] = await tx`
    SELECT subscription_id, array_agg(id || '(' || state || ')' ORDER BY id) AS invoices
    FROM payment_invoices
    WHERE state IN ('UPCOMING', 'OVERDUE')
    GROUP BY subscription_id HAVING count(*) >= 2`;
  listing(
    '중복 열린 인보이스',
    openDuplicates.map((row) => `${row.subscription_id}: ${(row.invoices as string[]).join(', ')}`),
  );

  section('7. 서비스 주기 중복 근사 — servicePeriodDuplicateInvoiceIds');

  // 백필의 서비스 주기 시작 투영을 근사한다: 동결=원본 due_at / 종결=hour 내림 due_at /
  // 열린 TRANSITION=hour 내림 renewed_at / 열린 RENEWAL=hour 내림 expires_at. AMBIGUOUS·미지원 interval 은 어차피 수동이라 제외.
  const periodDuplicates: Row[] = await tx`
    WITH projected AS (
      SELECT pi.id, pi.state, pi.subscription_id,
        CASE
          WHEN EXISTS (SELECT 1 FROM payment_records pr WHERE pr.invoice_id = pi.id) AND pi.state IN ('UPCOMING', 'OVERDUE')
            THEN pi.due_at
          WHEN pi.state IN ('PAID', 'CANCELED', 'WAIVED')
            THEN date_trunc('hour', pi.due_at)
          WHEN pi.due_at = s.starts_at AND s.state = 'WILL_ACTIVATE'
            THEN date_trunc('hour', s.renewed_at)
          WHEN pi.due_at <> s.starts_at AND s.state NOT IN ('WILL_ACTIVATE')
            THEN date_trunc('hour', s.expires_at)
        END AS service_starts_at
      FROM payment_invoices pi
      JOIN subscriptions s ON s.id = pi.subscription_id
      JOIN plans p ON p.id = s.plan_id
      WHERE p.interval IN ('MONTHLY', 'YEARLY')
    )
    SELECT subscription_id, service_starts_at, array_agg(id || '(' || state || ')' ORDER BY id) AS invoices
    FROM projected
    WHERE service_starts_at IS NOT NULL
    GROUP BY subscription_id, service_starts_at HAVING count(*) >= 2`;
  listing(
    '서비스 주기 시작 중복 그룹(근사)',
    periodDuplicates.map((row) => `${row.subscription_id}@${iso(row.service_starts_at)}: ${(row.invoices as string[]).join(', ')}`),
  );

  section('8. 예약(WILL_ACTIVATE) 페어 — manualReservationIds / blockedReservationIds');

  const reservations: Row[] = await tx`
    SELECT r.id, r.starts_at,
      (SELECT array_agg(pred.id)
       FROM subscriptions pred JOIN plans pp ON pp.id = pred.plan_id
       WHERE pred.user_id = r.user_id AND pred.id <> r.id AND pred.expires_at = r.starts_at
         AND pp.availability IN ('BILLING_KEY', 'TRIAL')) AS predecessors
    FROM subscriptions r
    WHERE r.state = 'WILL_ACTIVATE'
    ORDER BY r.starts_at`;

  const paired = reservations.filter((row) => ((row.predecessors as string[] | null) ?? []).length === 1);
  console.log(`  예약 전체: ${reservations.length}건 / predecessor 확정(후보 1): ${paired.length}건`);
  listing(
    'manualReservationIds 예상(후보 0 또는 2+)',
    reservations
      .filter((row) => ((row.predecessors as string[] | null) ?? []).length !== 1)
      .map((row) => `${row.id} (후보 ${((row.predecessors as string[] | null) ?? []).length}건, starts_at=${iso(row.starts_at)})`),
  );
  listing(
    'blockedReservationIds 예상(예약 또는 predecessor 가 동결 보류)',
    paired
      .filter(
        (row) => blockingSubscriptionIds.has(String(row.id)) || blockingSubscriptionIds.has(String((row.predecessors as string[])[0])),
      )
      .map((row) => `${row.id} (predecessor=${(row.predecessors as string[])[0]})`),
  );

  section('9. IAP 바인딩 — manualCanonicalBindingIds (스토어 조회 결과 목록은 드라이런에서만)');

  const bindings: Row[] = await tx`
    SELECT b.id, b.store, b.user_id,
      (SELECT array_agg(s.id)
       FROM subscriptions s JOIN plans p ON p.id = s.plan_id
       WHERE s.user_id = b.user_id AND p.availability = 'IN_APP_PURCHASE'
         AND s.state IN ('ACTIVE', 'WILL_EXPIRE', 'IN_GRACE_PERIOD')) AS live_subscriptions
    FROM user_in_app_purchases b
    ORDER BY b.id`;

  const byStore = new Map<string, number>();
  for (const row of bindings) {
    byStore.set(String(row.store), (byStore.get(String(row.store)) ?? 0) + 1);
  }
  console.log(`  바인딩 전체: ${bindings.length}건 (${[...byStore].map(([store, cnt]) => `${store}=${cnt}`).join(' / ')})`);
  listing(
    'manualCanonicalBindingIds 예상(살아있는 IAP 구독 0 또는 2+)',
    bindings
      .filter((row) => ((row.live_subscriptions as string[] | null) ?? []).length !== 1)
      .map(
        (row) =>
          `${row.id} store=${row.store} (살아있는 IAP 구독 ${((row.live_subscriptions as string[] | null) ?? []).length}건${((row.live_subscriptions as string[] | null) ?? []).length > 0 ? `: ${(row.live_subscriptions as string[]).join(', ')}` : ''})`,
      ),
  );
};

console.log(`조사 시작 — 읽기 전용 / samples=${samples}`);

await sql.unsafe(`SET default_transaction_read_only = on`);
await sql.unsafe(`SET statement_timeout = '120s'`);
await sql.unsafe(`SET timezone = 'Asia/Seoul'`);

try {
  // 스냅샷 하나에서 전 질의를 읽어 목록 간 정합을 유지한다.
  await sql.begin('isolation level repeatable read read only', main);
  console.log('\n완료 — 이 결과는 규모 파악용 근사다. 확정 목록은 스냅샷 복제본 드라이런으로 얻는다.');
} finally {
  await sql.end();
}
