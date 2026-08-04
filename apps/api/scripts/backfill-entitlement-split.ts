#!/usr/bin/env node

// 구독 권한/청구 분리 백필의 SQL 단계(0~5·7). IAP 스토어 조회 단계(6)는 backfill-iap-periods.ts 가 맡는다.
// 구 프로세스(API·워커) 정지 후 · 제약 생성 마이그레이션 전에 실행한다.
//
//   미리보기: doppler run -- node scripts/backfill-entitlement-split.ts --dry-run 2>&1 | tee backfill-dry.log
//   실행:     doppler run -- node scripts/backfill-entitlement-split.ts 2>&1 | tee backfill.log
//   원장 경로: --manifest=<path> (기본 ./backfill-entitlement-split-manifest.json)
//   최초 실행: 이미 백필된 DB 에서 원장이 없으면 중단한다 — 진짜 최초라면 --init-ledger 로 명시한다.
//
// 출력은 수동 목록이 본체라 반드시 파일로 남긴다(위 tee). 프로세스는 exitCode 만 세우고 자연 종료하므로
// 파이프 뒤의 tee 가 끊기지 않는다.
//
// 재실행 안전성의 근거는 원장(manifest)이다. ① 불변 증거(동결 인보이스의 원 주기·예약 페어·인보이스 경로 분류)는
// 최초 확정 후 재작성하지 않고, ② 보류 집합은 매 실행 현재 DB 로 재평가한다 — ①을 현재 DB 로 다시 계산하면
// 해소된 동결 인보이스를 종결 규칙으로 덮어쓰고, ②를 고정하면 해소돼도 그 구독이 영원히 교정되지 않는다.
// 원본 컬럼에서만 읽히는 단서(예약 페어의 expires_at 등식)는 어떤 UPDATE 보다 먼저 ①로 고정한다.

import '@typie/lib/dayjs';

import fs from 'node:fs';
import path from 'node:path';
import { PaymentInvoiceState, PlanAvailability, PlanInterval, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, eq, exists, inArray, isNotNull, ne, or, sql } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import { db, PaymentInvoices, PaymentRecords, pg, pgb, pgr, Plans, Subscriptions } from '#/db/index.ts';
import { computeNextPeriodEnd, floorToHourKst } from '#/utils/billing-period.ts';
import { ACTIVE_SUBSCRIPTION_STATES } from '#/utils/plan.ts';
import type { Transaction } from '#/db/index.ts';

const MANIFEST_VERSION = 1;
const TERMINAL_INVOICE_STATES = new Set<PaymentInvoiceState>([
  PaymentInvoiceState.PAID,
  PaymentInvoiceState.WAIVED,
  PaymentInvoiceState.CANCELED,
]);
const OPEN_INVOICE_STATES: PaymentInvoiceState[] = [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE];

type InvoicePath = 'RENEWAL' | 'TRANSITION' | 'AMBIGUOUS';

type FrozenInvoiceEntry = {
  invoiceId: string;
  subscriptionId: string;
  dueAtOriginal: string;
  interval: PlanInterval;
  servicePeriodStartsAt: string | null;
  servicePeriodEndsAt: string | null;
  frozenSource: 'DUE_AT' | 'PERSISTED';
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
  lastRun?: {
    at: string;
    dryRun: boolean;
    blockingSubscriptionIds: string[];
  };
};

type RawTimestamp = Date | string | null;

const dryRun = process.argv.includes('--dry-run') || !!process.env.DRY_RUN;
const initLedger = process.argv.includes('--init-ledger');
const manifestPath = path.resolve(
  process.argv.find((arg) => arg.startsWith('--manifest='))?.slice('--manifest='.length) ??
    process.env.BACKFILL_MANIFEST_PATH ??
    'backfill-entitlement-split-manifest.json',
);

const now = dayjs();

const manualLists: Record<string, string[]> = {};
const addManual = (bucket: string, entry: string) => {
  manualLists[bucket] ??= [];
  if (!manualLists[bucket].includes(entry)) {
    manualLists[bucket].push(entry);
  }
};

const section = (title: string) => {
  console.log(`\n=== ${title} ===`);
};

const listing = (label: string, entries: string[]) => {
  console.log(`  ${label}: ${entries.length}건`);
  for (const entry of entries) {
    console.log(`    - ${entry}`);
  }
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

const intervalUnit = (interval: PlanInterval) => {
  if (interval === PlanInterval.MONTHLY) {
    return 'month' as const;
  }

  if (interval === PlanInterval.YEARLY) {
    return 'year' as const;
  }

  return null;
};

class DryRunRollback extends Error {}

// 0단계 — 동결 스냅샷(어떤 UPDATE 보다 먼저 원본 컬럼에서만 읽는다).

const loadManifest = () => {
  if (!fs.existsSync(manifestPath)) {
    return null;
  }

  const parsed = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as Manifest;
  if (parsed.version !== MANIFEST_VERSION) {
    throw new Error(`manifest version mismatch: ${parsed.version} (expected ${MANIFEST_VERSION})`);
  }

  return parsed;
};

const saveManifest = (manifest: Manifest) => {
  if (dryRun) {
    return;
  }

  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
};

const snapshotFrozenInvoices = async (manifest: Manifest) => {
  const rows = await db
    .select({
      invoiceId: PaymentInvoices.id,
      subscriptionId: PaymentInvoices.subscriptionId,
      dueAt: PaymentInvoices.dueAt,
      servicePeriodStartsAt: sql<RawTimestamp>`${PaymentInvoices.servicePeriodStartsAt}`,
      servicePeriodEndsAt: sql<RawTimestamp>`${PaymentInvoices.servicePeriodEndsAt}`,
      interval: Plans.interval,
    })
    .from(PaymentInvoices)
    .innerJoin(Subscriptions, eq(Subscriptions.id, PaymentInvoices.subscriptionId))
    .innerJoin(Plans, eq(Plans.id, Subscriptions.planId))
    .where(
      and(
        inArray(PaymentInvoices.state, OPEN_INVOICE_STATES),
        exists(
          db
            .select({ one: sql`1` })
            .from(PaymentRecords)
            .where(eq(PaymentRecords.invoiceId, PaymentInvoices.id)),
        ),
      ),
    );

  const known = new Set(manifest.frozenInvoices.map((entry) => entry.invoiceId));
  const added: string[] = [];

  for (const row of rows) {
    if (known.has(row.invoiceId)) {
      continue;
    }

    const unit = intervalUnit(row.interval);
    if (unit === null) {
      addManual('frozenInvoiceUnsupportedInterval', `${row.invoiceId} (interval=${row.interval})`);
    }

    // 이전 실행이 이미 서비스 주기를 기록했다면 그 값으로 승인이 시도됐을 수 있다 — due_at 재파생보다 우선한다.
    const persistedStartsAt = toDayjs(row.servicePeriodStartsAt);
    const persistedEndsAt = toDayjs(row.servicePeriodEndsAt);
    const persisted = persistedStartsAt !== null && persistedEndsAt !== null;

    // 승인이 이 주기로 이뤄졌다 — 정렬·내림·앵커 재계산 없이 원본 due_at 그대로 고정한다.
    const derivedStartsAt = unit === null ? null : row.dueAt.toISOString();
    const derivedEndsAt = unit === null ? null : row.dueAt.kst().add(1, unit).toISOString();

    manifest.frozenInvoices.push({
      invoiceId: row.invoiceId,
      subscriptionId: row.subscriptionId,
      dueAtOriginal: row.dueAt.toISOString(),
      interval: row.interval,
      servicePeriodStartsAt: persisted ? persistedStartsAt.toISOString() : derivedStartsAt,
      servicePeriodEndsAt: persisted ? persistedEndsAt.toISOString() : derivedEndsAt,
      frozenSource: persisted ? 'PERSISTED' : 'DUE_AT',
      frozenAt: now.toISOString(),
    });

    added.push(`${row.invoiceId} (source=${persisted ? 'PERSISTED' : 'DUE_AT'})`);
  }

  return added;
};

const snapshotReservationPairs = async (manifest: Manifest) => {
  const reservations = await db
    .select({ id: Subscriptions.id, startsAt: Subscriptions.startsAt })
    .from(Subscriptions)
    .where(eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE));

  const predecessor = alias(Subscriptions, 'predecessor');
  const predecessorPlan = alias(Plans, 'predecessor_plan');

  const candidates = await db
    .select({
      reservationId: Subscriptions.id,
      predecessorId: predecessor.id,
    })
    .from(Subscriptions)
    .innerJoin(
      predecessor,
      and(
        eq(predecessor.userId, Subscriptions.userId),
        ne(predecessor.id, Subscriptions.id),
        // 1단계 복사 전이라 경계의 원값은 expires_at 뿐이다.
        eq(predecessor.expiresAt, Subscriptions.startsAt),
      ),
    )
    .innerJoin(predecessorPlan, eq(predecessorPlan.id, predecessor.planId))
    .where(
      and(
        eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE),
        inArray(predecessorPlan.availability, [PlanAvailability.BILLING_KEY, PlanAvailability.TRIAL]),
      ),
    );

  const grouped = new Map<string, string[]>();
  for (const candidate of candidates) {
    const bucket = grouped.get(candidate.reservationId) ?? [];
    bucket.push(candidate.predecessorId);
    grouped.set(candidate.reservationId, bucket);
  }

  for (const reservation of reservations) {
    const matched = grouped.get(reservation.id) ?? [];

    if (matched.length === 1) {
      manifest.reservationPairs.push({
        reservationId: reservation.id,
        predecessorId: matched[0],
        // 조인 등식이 성립한 시점의 경계 원값 — predecessor.expires_at 과 같다.
        oldBoundary: reservation.startsAt.toISOString(),
        candidates: 1,
      });
      continue;
    }

    manifest.reservationPairs.push({
      reservationId: reservation.id,
      predecessorId: null,
      oldBoundary: null,
      candidates: matched.length,
    });
  }
};

const classifyInvoicePaths = async (manifest: Manifest) => {
  const frozen = new Set(manifest.frozenInvoices.map((entry) => entry.invoiceId));

  const rows = await db
    .select({
      invoiceId: PaymentInvoices.id,
      subscriptionId: PaymentInvoices.subscriptionId,
      subscriptionState: Subscriptions.state,
      // 2·3·7단계가 경계를 옮기면 이 등식은 소멸한다 — 최초 판별을 원장에 고정한다.
      transitionShaped: sql<boolean>`${PaymentInvoices.dueAt} = ${Subscriptions.startsAt}`,
    })
    .from(PaymentInvoices)
    .innerJoin(Subscriptions, eq(Subscriptions.id, PaymentInvoices.subscriptionId))
    .where(inArray(PaymentInvoices.state, OPEN_INVOICE_STATES));

  const added: string[] = [];

  for (const row of rows) {
    if (frozen.has(row.invoiceId) || Object.hasOwn(manifest.invoicePaths, row.invoiceId)) {
      continue;
    }

    const reserved = row.subscriptionState === SubscriptionState.WILL_ACTIVATE;
    const live = row.subscriptionState === SubscriptionState.ACTIVE || row.subscriptionState === SubscriptionState.WILL_EXPIRE;

    let resolved: InvoicePath = row.transitionShaped ? 'TRANSITION' : 'RENEWAL';
    if ((live && row.transitionShaped) || (reserved && !row.transitionShaped)) {
      resolved = 'AMBIGUOUS';
    }

    manifest.invoicePaths[row.invoiceId] = resolved;
    added.push(`${row.invoiceId} → ${resolved} (subscription=${row.subscriptionId} state=${row.subscriptionState})`);

    if (resolved === 'AMBIGUOUS') {
      addManual('invoicePathAmbiguous', `${row.invoiceId} (subscription=${row.subscriptionId} state=${row.subscriptionState})`);
    }
  }

  return added;
};

// 인보이스 서비스 주기 계산(5단계 규칙) — 7단계 재정렬이 같은 계산을 재사용한다.

type InvoiceProjectionRow = {
  invoiceId: string;
  invoiceState: PaymentInvoiceState;
  dueAt: dayjs.Dayjs;
  servicePeriodStartsAt: dayjs.Dayjs | null;
  servicePeriodEndsAt: dayjs.Dayjs | null;
  subscriptionId: string;
  periodStartsAt: dayjs.Dayjs;
  periodEndsAt: dayjs.Dayjs | null;
  billingAnchorAt: dayjs.Dayjs | null;
  interval: PlanInterval;
};

const selectInvoiceProjection = async (tx: Transaction): Promise<InvoiceProjectionRow[]> => {
  const rows = await tx
    .select({
      invoiceId: PaymentInvoices.id,
      invoiceState: PaymentInvoices.state,
      dueAt: PaymentInvoices.dueAt,
      servicePeriodStartsAt: sql<RawTimestamp>`${PaymentInvoices.servicePeriodStartsAt}`,
      servicePeriodEndsAt: sql<RawTimestamp>`${PaymentInvoices.servicePeriodEndsAt}`,
      subscriptionId: Subscriptions.id,
      periodStartsAt: Subscriptions.currentPeriodStartsAt,
      periodEndsAt: sql<RawTimestamp>`${Subscriptions.currentPeriodEndsAt}`,
      billingAnchorAt: sql<RawTimestamp>`${Subscriptions.billingAnchorAt}`,
      interval: Plans.interval,
    })
    .from(PaymentInvoices)
    .innerJoin(Subscriptions, eq(Subscriptions.id, PaymentInvoices.subscriptionId))
    .innerJoin(Plans, eq(Plans.id, Subscriptions.planId));

  return rows.map((row) => ({
    ...row,
    servicePeriodStartsAt: toDayjs(row.servicePeriodStartsAt),
    servicePeriodEndsAt: toDayjs(row.servicePeriodEndsAt),
    periodEndsAt: toDayjs(row.periodEndsAt),
    billingAnchorAt: toDayjs(row.billingAnchorAt),
  }));
};

type ServicePeriod = { startsAt: dayjs.Dayjs; endsAt: dayjs.Dayjs };

const frozenIndex = (manifest: Manifest) => {
  return new Map(manifest.frozenInvoices.map((entry) => [entry.invoiceId, entry]));
};

const resolveServicePeriod = (
  row: InvoiceProjectionRow,
  frozenById: Map<string, FrozenInvoiceEntry>,
  invoicePaths: Record<string, InvoicePath>,
): ServicePeriod | { manual: string } => {
  const frozen = frozenById.get(row.invoiceId);
  if (frozen) {
    // 동결 값은 상태가 어떻게 바뀌어도 재작성하지 않는다(해소된 인보이스를 종결 규칙으로 덮어쓰면 승인 주기가 사라진다).
    if (frozen.servicePeriodStartsAt === null || frozen.servicePeriodEndsAt === null) {
      return { manual: `동결 인보이스 interval 미지원 (interval=${frozen.interval})` };
    }

    return { startsAt: dayjs(frozen.servicePeriodStartsAt), endsAt: dayjs(frozen.servicePeriodEndsAt) };
  }

  const unit = intervalUnit(row.interval);
  if (unit === null) {
    return { manual: `interval=${row.interval}` };
  }

  if (TERMINAL_INVOICE_STATES.has(row.invoiceState)) {
    // 과거 주기는 당시 실제 청구 경계의 기록이다 — 앵커 복원식을 소급하지 않고 시간 경계만 맞춘다.
    const startsAt = floorToHourKst(row.dueAt);
    return { startsAt, endsAt: startsAt.add(1, unit) };
  }

  const invoicePath = invoicePaths[row.invoiceId];
  if (invoicePath === undefined || invoicePath === 'AMBIGUOUS') {
    return { manual: `경로 판별 불가 (${invoicePath ?? 'MISSING'})` };
  }

  if (row.periodEndsAt === null) {
    return { manual: '연결 구독의 주기 종료 없음' };
  }

  if (invoicePath === 'TRANSITION') {
    return { startsAt: row.periodStartsAt, endsAt: row.periodEndsAt };
  }

  if (row.billingAnchorAt === null) {
    return { manual: '연결 구독의 앵커 없음' };
  }

  // finalizePaymentSuccess 가 이 값을 구독에 되쓰므로 단순 가산이 아니라 앵커 투영과 일치해야 한다.
  const startsAt = row.periodEndsAt;
  return {
    startsAt,
    endsAt: computeNextPeriodEnd({ periodStartsAt: startsAt, interval: row.interval, billingAnchorAt: row.billingAnchorAt }),
  };
};

const applyServicePeriods = async (tx: Transaction, manifest: Manifest) => {
  const rows = await selectInvoiceProjection(tx);
  const frozenById = frozenIndex(manifest);
  let updated = 0;
  let processed = 0;

  for (const row of rows) {
    processed += 1;
    if (processed % 500 === 0) {
      console.log(`  진행: ${processed}/${rows.length}건 (기록 ${updated}건)`);
    }

    const resolved = resolveServicePeriod(row, frozenById, manifest.invoicePaths);

    if ('manual' in resolved) {
      addManual('manualInvoiceIds', `${row.invoiceId} (${resolved.manual})`);
      continue;
    }

    if (same(row.servicePeriodStartsAt, resolved.startsAt) && same(row.servicePeriodEndsAt, resolved.endsAt)) {
      continue;
    }

    await tx
      .update(PaymentInvoices)
      .set({ servicePeriodStartsAt: resolved.startsAt, servicePeriodEndsAt: resolved.endsAt })
      .where(eq(PaymentInvoices.id, row.invoiceId));

    updated += 1;
  }

  return updated;
};

// 정합 게이트 — 위반은 목록 출력만 한다(제약은 마지막 마이그레이션이 만든다).

const runGates = async (tx: Transaction, manifest: Manifest, blocking: Set<string>, stage: string, includeReservations: boolean) => {
  const rows = await selectInvoiceProjection(tx);
  const frozen = new Set(manifest.frozenInvoices.map((entry) => entry.invoiceId));

  const violations: string[] = [];
  const deferred: string[] = [];

  const record = (subscriptionId: string, message: string) => {
    if (blocking.has(subscriptionId)) {
      deferred.push(message);
    } else {
      violations.push(message);
    }
  };

  for (const row of rows) {
    if (!OPEN_INVOICE_STATES.includes(row.invoiceState) || frozen.has(row.invoiceId)) {
      continue;
    }

    const invoicePath = manifest.invoicePaths[row.invoiceId];

    if (invoicePath === 'RENEWAL' && !same(row.servicePeriodStartsAt, row.periodEndsAt)) {
      record(
        row.subscriptionId,
        `[정기갱신] ${row.invoiceId} service_start=${iso(row.servicePeriodStartsAt)} ≠ subscription.current_period_ends_at=${iso(row.periodEndsAt)}`,
      );
    }

    if (
      invoicePath === 'TRANSITION' &&
      (!same(row.servicePeriodStartsAt, row.periodStartsAt) || !same(row.servicePeriodEndsAt, row.periodEndsAt))
    ) {
      record(
        row.subscriptionId,
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

      if (!same(previous.endsAt, current.startsAt)) {
        const overlapping = current.startsAt.isBefore(previous.endsAt);
        record(
          subscriptionId,
          `[종결 ${overlapping ? '중첩' : '불연속'}] ${previous.invoiceId}(~${iso(previous.endsAt)}) → ${current.invoiceId}(${iso(current.startsAt)}~)`,
        );
      }
    }
  }

  // 예약 게이트는 7단계 재구성 이후에만 의미가 있다 — 5단계에서 돌리면 재구성 전 경계가 전건 위반으로 나온다.
  const reservationRows = includeReservations
    ? await tx
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
        .where(eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE))
    : [];

  const predecessorBoundaries = new Map<string, dayjs.Dayjs | null>();
  const predecessorIds = manifest.reservationPairs.map((pair) => pair.predecessorId).filter((id) => id !== null);
  if (includeReservations && predecessorIds.length > 0) {
    const boundaries = await tx
      .select({ id: Subscriptions.id, periodEndsAt: sql<RawTimestamp>`${Subscriptions.currentPeriodEndsAt}` })
      .from(Subscriptions)
      .where(inArray(Subscriptions.id, predecessorIds));

    for (const boundary of boundaries) {
      predecessorBoundaries.set(boundary.id, toDayjs(boundary.periodEndsAt));
    }
  }

  for (const reservation of reservationRows) {
    const pair = manifest.reservationPairs.find((entry) => entry.reservationId === reservation.id);
    const periodEndsAt = toDayjs(reservation.periodEndsAt);
    const billingAnchorAt = toDayjs(reservation.billingAnchorAt);

    // predecessor 가 보류면 그 경계가 아직 교정되지 않았다 — 예약의 불일치는 위반이 아니라 보류 산물이다.
    const recordReservation = (message: string) => {
      if (pair?.predecessorId !== undefined && pair.predecessorId !== null && blocking.has(pair.predecessorId)) {
        deferred.push(message);
        return;
      }

      record(reservation.id, message);
    };

    if (pair?.predecessorId) {
      const boundary = predecessorBoundaries.get(pair.predecessorId) ?? null;
      if (!same(reservation.startsAt, boundary)) {
        recordReservation(
          `[예약 경계] ${reservation.id} starts_at=${iso(reservation.startsAt)} ≠ predecessor(${pair.predecessorId}).current_period_ends_at=${iso(boundary)}`,
        );
      }
    }

    if (billingAnchorAt === null || periodEndsAt === null) {
      recordReservation(`[예약 주기] ${reservation.id} 앵커·주기 종료 미확정 (anchor=${iso(billingAnchorAt)} end=${iso(periodEndsAt)})`);
      continue;
    }

    if (intervalUnit(reservation.interval) === null) {
      recordReservation(`[예약 주기] ${reservation.id} interval=${reservation.interval}`);
      continue;
    }

    const expected = computeNextPeriodEnd({
      periodStartsAt: reservation.periodStartsAt,
      interval: reservation.interval,
      billingAnchorAt,
    });

    if (!same(periodEndsAt, expected)) {
      recordReservation(`[예약 주기] ${reservation.id} current_period_ends_at=${iso(periodEndsAt)} ≠ plan interval 투영=${iso(expected)}`);
    }
  }

  listing(`${stage} 게이트 위반`, violations);
  listing(`${stage} 게이트 위반(보류 — 해소 후 재실행)`, deferred);
};

// 실행.

const runBackfill = async (tx: Transaction, manifest: Manifest) => {
  section('0단계 — 보류 집합(② 매 실행 재평가) · payment_key');

  const frozenIds = manifest.frozenInvoices.map((entry) => entry.invoiceId);
  const stillOpen =
    frozenIds.length === 0
      ? []
      : await tx
          .select({ invoiceId: PaymentInvoices.id, subscriptionId: PaymentInvoices.subscriptionId, state: PaymentInvoices.state })
          .from(PaymentInvoices)
          .where(and(inArray(PaymentInvoices.id, frozenIds), inArray(PaymentInvoices.state, OPEN_INVOICE_STATES)));

  const blocking = new Set(stillOpen.map((row) => row.subscriptionId));
  listing(
    '보류 구독(동결 인보이스가 지금 열려 있음)',
    stillOpen.map((row) => `subscription=${row.subscriptionId} invoice=${row.invoiceId} state=${row.state}`),
  );
  for (const row of stillOpen) {
    addManual(
      'blockedSubscriptionIds',
      `${row.subscriptionId} (invoice=${row.invoiceId} state=${row.state} — 성공 확정 또는 CANCELED 로 해소 후 재실행)`,
    );
  }

  const blockingParam = [...blocking];

  const paymentKeyBackfilled = await tx.execute(sql`UPDATE payment_invoices SET payment_key = id WHERE payment_key IS NULL RETURNING id`);
  console.log(`  payment_key 백필: ${paymentKeyBackfilled.length}건`);

  section('1단계 — current_period_ends_at');

  const copied = await tx.execute(
    sql`UPDATE subscriptions SET current_period_ends_at = expires_at WHERE current_period_ends_at IS NULL AND expires_at IS NOT NULL RETURNING id`,
  );
  console.log(`  expires_at 복사: ${copied.length}건`);

  const missing = await tx.execute(sql`SELECT id FROM subscriptions WHERE current_period_ends_at IS NULL`);
  for (const row of missing) {
    addManual('missingPeriodEndSubscriptionIds', String(row.id));
  }
  console.log(`  주기 종료 미확정: ${missing.length}건`);

  const zeroLength = await tx
    .select({
      id: Subscriptions.id,
      state: Subscriptions.state,
      startsAt: Subscriptions.startsAt,
      periodEndsAt: sql<RawTimestamp>`${Subscriptions.currentPeriodEndsAt}`,
      interval: Plans.interval,
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Plans.id, Subscriptions.planId))
    .where(
      and(
        sql`${Subscriptions.currentPeriodEndsAt} = ${Subscriptions.startsAt}`,
        or(
          eq(Subscriptions.state, SubscriptionState.IN_GRACE_PERIOD),
          and(
            eq(Subscriptions.state, SubscriptionState.EXPIRED),
            exists(
              tx
                .select({ one: sql`1` })
                .from(PaymentInvoices)
                .where(eq(PaymentInvoices.subscriptionId, Subscriptions.id)),
            ),
          ),
        ),
      ),
    );

  listing(
    '0길이 주기 재구성 대상(전건 — 드라이런에서 사람이 확인)',
    zeroLength.map(
      (row) => `${row.id} state=${row.state} starts_at=${iso(row.startsAt)} current_period_ends_at=${iso(toDayjs(row.periodEndsAt))}`,
    ),
  );

  let reconstructed = 0;
  for (const row of zeroLength) {
    if (blocking.has(row.id)) {
      addManual('blockedZeroLengthSubscriptionIds', row.id);
      continue;
    }

    const unit = intervalUnit(row.interval);
    if (unit === null) {
      addManual('zeroLengthUnsupportedInterval', `${row.id} (interval=${row.interval})`);
      continue;
    }

    // SQL interval 가산은 세션 시간대(UTC) 달력으로 더해져 KST 월 경계에서 하루 어긋난다.
    await tx
      .update(Subscriptions)
      .set({ currentPeriodEndsAt: row.startsAt.kst().add(1, unit) })
      .where(eq(Subscriptions.id, row.id));

    reconstructed += 1;
  }
  console.log(`  0길이 주기 재구성: ${reconstructed}건`);

  section('2단계 — 앵커·시간 경계 내림');

  const anchored = await tx.execute(sql`
    UPDATE subscriptions s
    SET billing_anchor_at = date_trunc('hour', s.starts_at),
        current_period_starts_at = date_trunc('hour', s.current_period_starts_at),
        current_period_ends_at = date_trunc('hour', s.current_period_ends_at)
    FROM plans p
    WHERE p.id = s.plan_id
      AND p.availability = 'BILLING_KEY'
      AND p.interval IN ('MONTHLY', 'YEARLY')
      AND s.id <> ALL(${textArray(blockingParam)})
      AND (
        s.billing_anchor_at IS DISTINCT FROM date_trunc('hour', s.starts_at)
        OR s.current_period_starts_at IS DISTINCT FROM date_trunc('hour', s.current_period_starts_at)
        OR s.current_period_ends_at IS DISTINCT FROM date_trunc('hour', s.current_period_ends_at)
      )
    RETURNING s.id
  `);
  console.log(`  앵커·시간 내림: ${anchored.length}건`);

  section('3단계 — 밀린 결제일');

  const billed = await tx
    .select({
      id: Subscriptions.id,
      state: Subscriptions.state,
      periodEndsAt: sql<RawTimestamp>`${Subscriptions.currentPeriodEndsAt}`,
      billingAnchorAt: sql<RawTimestamp>`${Subscriptions.billingAnchorAt}`,
      interval: Plans.interval,
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Plans.id, Subscriptions.planId))
    .where(
      and(
        eq(Plans.availability, PlanAvailability.BILLING_KEY),
        inArray(Plans.interval, [PlanInterval.MONTHLY, PlanInterval.YEARLY]),
        inArray(Subscriptions.state, ACTIVE_SUBSCRIPTION_STATES),
      ),
    );

  const lagging: string[] = [];
  let realigned = 0;

  for (const row of billed) {
    if (blocking.has(row.id)) {
      addManual('blockedLagCheckSubscriptionIds', row.id);
      continue;
    }

    const periodEndsAt = toDayjs(row.periodEndsAt);
    const billingAnchorAt = toDayjs(row.billingAnchorAt);
    const unit = intervalUnit(row.interval);

    if (periodEndsAt === null || unit === null) {
      addManual('lagCheckUnresolvable', `${row.id} (end=${iso(periodEndsAt)} interval=${row.interval})`);
      continue;
    }

    if (billingAnchorAt === null) {
      addManual('missingAnchorSubscriptionIds', `${row.id} (state=${row.state})`);
      continue;
    }

    // 같은 달 앵커 정렬식은 한 주기 이상 밀린 행을 잡지 못한다 — 판정을 둘로 나눈다.
    if (!periodEndsAt.isAfter(now.kst().subtract(1, unit))) {
      lagging.push(`${row.id} state=${row.state} current_period_ends_at=${iso(periodEndsAt)} interval=${row.interval}`);
      addManual('laggingSubscriptionIds', `${row.id} (current_period_ends_at=${iso(periodEndsAt)} — 신 워커 기동 전 해소 필요)`);
      continue;
    }

    const anchor = billingAnchorAt.kst();
    const base = periodEndsAt.kst();
    const expected = base.date(Math.min(anchor.date(), base.daysInMonth())).hour(anchor.hour()).startOf('hour');

    if (expected.isAfter(periodEndsAt)) {
      await tx.update(Subscriptions).set({ currentPeriodEndsAt: expected }).where(eq(Subscriptions.id, row.id));
      realigned += 1;
    }
  }

  listing('여러 주기 밀림(교정 없음 — 배포 조건)', lagging);
  console.log(`  같은 달 앵커 정렬: ${realigned}건`);

  section('4단계 — EXPIRED expires_at clip');

  const clipped = await tx.execute(
    sql`UPDATE subscriptions SET expires_at = now() WHERE state = 'EXPIRED' AND expires_at > now() RETURNING id`,
  );
  console.log(`  clip: ${clipped.length}건`);

  section('5단계 — 인보이스 서비스 주기');

  const projected = await applyServicePeriods(tx, manifest);
  console.log(`  서비스 주기 기록: ${projected}건`);

  await runGates(tx, manifest, blocking, '5단계', false);

  section('7단계 — 열린 인보이스 감사·재정렬·예약 재구성');

  const openInvoices = await tx
    .select({ id: PaymentInvoices.id, subscriptionId: PaymentInvoices.subscriptionId, state: PaymentInvoices.state })
    .from(PaymentInvoices)
    .where(inArray(PaymentInvoices.state, OPEN_INVOICE_STATES));

  const openBySubscription = new Map<string, string[]>();
  for (const invoice of openInvoices) {
    const bucket = openBySubscription.get(invoice.subscriptionId) ?? [];
    bucket.push(`${invoice.id}(${invoice.state})`);
    openBySubscription.set(invoice.subscriptionId, bucket);
  }

  const duplicates: string[] = [];
  for (const [subscriptionId, invoices] of openBySubscription) {
    if (invoices.length < 2) {
      continue;
    }

    duplicates.push(`${subscriptionId}: ${invoices.join(', ')}`);
    addManual('duplicateOpenInvoiceIds', `${subscriptionId}: ${invoices.join(', ')}`);
  }
  listing('구독당 열린 인보이스 2건 이상', duplicates);

  let reservationsRebuilt = 0;
  for (const pair of manifest.reservationPairs) {
    if (pair.predecessorId === null) {
      addManual('manualReservationIds', `${pair.reservationId} (predecessor 후보 ${pair.candidates}건)`);
      continue;
    }

    const rows = await tx
      .select({
        id: Subscriptions.id,
        state: Subscriptions.state,
        startsAt: Subscriptions.startsAt,
        periodStartsAt: Subscriptions.currentPeriodStartsAt,
        periodEndsAt: sql<RawTimestamp>`${Subscriptions.currentPeriodEndsAt}`,
        billingAnchorAt: sql<RawTimestamp>`${Subscriptions.billingAnchorAt}`,
        interval: Plans.interval,
      })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Plans.id, Subscriptions.planId))
      .where(inArray(Subscriptions.id, [pair.reservationId, pair.predecessorId]));

    const reservation = rows.find((row) => row.id === pair.reservationId);
    const predecessor = rows.find((row) => row.id === pair.predecessorId);

    if (!reservation || !predecessor) {
      addManual('manualReservationIds', `${pair.reservationId} (예약·predecessor 행 부재 — 취소로 삭제됐을 수 있다)`);
      continue;
    }

    if (reservation.state !== SubscriptionState.WILL_ACTIVATE) {
      addManual('manualReservationIds', `${pair.reservationId} (예약이 이미 ${reservation.state} — 재구성 대상 아님)`);
      continue;
    }

    if (blocking.has(reservation.id) || blocking.has(predecessor.id)) {
      addManual('blockedReservationIds', `${pair.reservationId} (predecessor=${pair.predecessorId})`);
      continue;
    }

    const boundary = toDayjs(predecessor.periodEndsAt);
    const unit = intervalUnit(reservation.interval);

    if (boundary === null || unit === null) {
      addManual('manualReservationIds', `${pair.reservationId} (boundary=${iso(boundary)} interval=${reservation.interval})`);
      continue;
    }

    const startsAt = boundary;
    const periodStartsAt = floorToHourKst(startsAt);
    const billingAnchorAt = floorToHourKst(startsAt);
    const periodEndsAt = computeNextPeriodEnd({ periodStartsAt, interval: reservation.interval, billingAnchorAt });

    if (
      same(reservation.startsAt, startsAt) &&
      same(reservation.periodStartsAt, periodStartsAt) &&
      same(toDayjs(reservation.billingAnchorAt), billingAnchorAt) &&
      same(toDayjs(reservation.periodEndsAt), periodEndsAt)
    ) {
      continue;
    }

    // 시작만 옮기면 종료·앵커가 옛 날짜에 남아 첫 주기가 짧아진다 — 한 계산으로 함께 재구성한다.
    await tx
      .update(Subscriptions)
      .set({ startsAt, currentPeriodStartsAt: periodStartsAt, billingAnchorAt, currentPeriodEndsAt: periodEndsAt })
      .where(eq(Subscriptions.id, reservation.id));

    reservationsRebuilt += 1;
  }
  console.log(`  예약 재구성: ${reservationsRebuilt}건`);

  const realignedInvoices = await applyServicePeriods(tx, manifest);
  console.log(`  인보이스 재정렬: ${realignedInvoices}건`);

  // 열린 인보이스 중복이 종결(CANCELED)로 해소돼도 종결 규칙이 같은 서비스 시작을 재기입해 유니크 충돌이 남는다
  // — 열린 상태만 보는 duplicateOpenInvoiceIds 로는 잡히지 않으므로 최종값 전건을 따로 검사한다.
  const finalPeriods = await tx
    .select({
      id: PaymentInvoices.id,
      state: PaymentInvoices.state,
      subscriptionId: PaymentInvoices.subscriptionId,
      servicePeriodStartsAt: sql<RawTimestamp>`${PaymentInvoices.servicePeriodStartsAt}`,
    })
    .from(PaymentInvoices);

  const periodGroups = new Map<string, string[]>();
  for (const row of finalPeriods) {
    const startsAt = toDayjs(row.servicePeriodStartsAt);
    if (startsAt === null) {
      continue;
    }

    const key = `${row.subscriptionId}@${startsAt.toISOString()}`;
    const bucket = periodGroups.get(key) ?? [];
    bucket.push(`${row.id}(${row.state})`);
    periodGroups.set(key, bucket);
  }

  const periodDuplicates: string[] = [];
  for (const [key, invoices] of periodGroups) {
    if (invoices.length < 2) {
      continue;
    }

    periodDuplicates.push(`${key}: ${invoices.join(', ')}`);
    addManual('servicePeriodDuplicateInvoiceIds', `${key}: ${invoices.join(', ')}`);
  }
  listing('서비스 주기 중복(subscription_id, service_period_starts_at)', periodDuplicates);

  await runGates(tx, manifest, blocking, '7단계', true);

  return blockingParam;
};

const main = async () => {
  console.log(`백필 시작 — ${dryRun ? 'DRY RUN(트랜잭션 롤백)' : '실행'} / now=${now.toISOString()}`);
  console.log(`원장: ${manifestPath}`);

  const loaded = loadManifest();

  if (loaded === null && !initLedger) {
    // 경로 오타 한 번이 원장을 "신규 생성"으로 만들면 동결 값·예약 페어가 조용히 소실되고 실행은 성공처럼 끝난다.
    // 이미 백필이 돌아간 DB 에서는 원장 부재를 사고로 간주한다.
    const backfilled = await db
      .select({ id: PaymentInvoices.id })
      .from(PaymentInvoices)
      .where(isNotNull(PaymentInvoices.paymentKey))
      .limit(1);

    if (backfilled.length > 0) {
      console.error(`원장(${manifestPath})이 없는데 DB 에는 이미 백필 흔적이 있다(payment_key 기록됨).`);
      console.error('이전 실행의 원장 경로를 --manifest= 로 지정하거나, 진짜 최초 실행이면 --init-ledger 를 명시한다.');
      process.exitCode = 1;

      return;
    }
  }

  const manifest: Manifest = loaded ?? {
    version: MANIFEST_VERSION,
    createdAt: now.toISOString(),
    frozenInvoices: [],
    reservationPairs: [],
    invoicePaths: {},
  };

  section('0단계 — 동결 스냅샷(① 불변 증거, 첫 UPDATE 이전)');
  console.log(`  원장 상태: ${loaded ? '기존 로드' : '신규 생성'}`);

  const addedFrozen = await snapshotFrozenInvoices(manifest);
  console.log(`  동결 인보이스: ${manifest.frozenInvoices.length}건 (이번 실행 신규 ${addedFrozen.length}건)`);
  if (loaded && addedFrozen.length > 0) {
    listing('재실행에서 새로 동결된 인보이스(원장 추가만 — 기존 항목은 재작성하지 않는다)', addedFrozen);
  }

  if (loaded === null) {
    await snapshotReservationPairs(manifest);
  } else {
    const known = new Set(manifest.reservationPairs.map((pair) => pair.reservationId));
    const current = await db
      .select({ id: Subscriptions.id })
      .from(Subscriptions)
      .where(eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE));

    for (const reservation of current) {
      if (!known.has(reservation.id)) {
        // 경계 교정 후에는 predecessor 등식 단서가 남아 있지 않다 — 추정하지 않고 사람에게 넘긴다.
        addManual('manualReservationIds', `${reservation.id} (원장에 없는 예약 — 최초 실행 이후 생성)`);
      }
    }
  }

  const pairedCount = manifest.reservationPairs.filter((pair) => pair.predecessorId !== null).length;
  console.log(
    `  예약 페어: ${manifest.reservationPairs.length}건 (확정 ${pairedCount} / 미해결 ${manifest.reservationPairs.length - pairedCount})`,
  );
  for (const pair of manifest.reservationPairs) {
    if (pair.predecessorId === null) {
      continue;
    }

    console.log(`    - ${pair.reservationId} ← ${pair.predecessorId} (boundary=${pair.oldBoundary})`);
  }

  const addedPaths = await classifyInvoicePaths(manifest);
  console.log(`  경로 분류: ${Object.keys(manifest.invoicePaths).length}건 (이번 실행 신규 ${addedPaths.length}건)`);
  for (const entry of addedPaths) {
    console.log(`    - ${entry}`);
  }

  // 원장은 첫 UPDATE 이전에 디스크로 확정한다 — 커밋 후에 쓰면 파일 쓰기 실패가 원값 증거를 잃는다.
  saveManifest(manifest);
  if (dryRun) {
    console.log('  DRY RUN — 원장 파일은 기록하지 않는다');
  }

  let blockingSubscriptionIds: string[] = [];

  try {
    await db.transaction(async (tx) => {
      blockingSubscriptionIds = await runBackfill(tx, manifest);

      if (dryRun) {
        throw new DryRunRollback('dry-run');
      }
    });
  } catch (err) {
    if (!(err instanceof DryRunRollback)) {
      throw err;
    }

    console.log('\nDRY RUN — 트랜잭션 롤백 완료(DB 반영 없음)');
  }

  section('수동 처리 목록');
  const buckets = Object.keys(manualLists).toSorted((a, b) => a.localeCompare(b));
  if (buckets.length === 0) {
    console.log('  없음');
  }
  for (const bucket of buckets) {
    listing(bucket, manualLists[bucket]);
  }

  manifest.lastRun = { at: now.toISOString(), dryRun, blockingSubscriptionIds };
  saveManifest(manifest);

  console.log(`\n완료 — ${dryRun ? 'DRY RUN' : '적용'} / 보류 구독 ${blockingSubscriptionIds.length}건`);
};

// process.exit 는 파이프(tee)로 흘려보낸 출력을 자르므로 exitCode 만 세우고 커넥션을 닫아 자연 종료한다.
await main();
await pg.end();
await pgr.end();
await pgb.end();
