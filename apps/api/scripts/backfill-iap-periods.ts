#!/usr/bin/env node

// 구독 권한/청구 분리 백필의 IAP 스토어 조회 단계. SQL 단계는 backfill-entitlement-split.ts 가 먼저 끝낸다
// (주기 종료 복사가 이 스크립트의 입력이다). 구 프로세스(API·워커) 정지 후 · 제약 생성 마이그레이션 전에 실행한다.
//
//   미리보기: doppler run -- node scripts/backfill-iap-periods.ts --dry-run 2>&1 | tee backfill-iap-dry.log
//   실행:     doppler run -- node scripts/backfill-iap-periods.ts 2>&1 | tee backfill-iap.log
//   원장 경로: --ledger=<path> (기본 ./backfill-iap-periods-ledger.json)
//   최초 실행: 바인딩에 이미 백필 흔적이 있는데 원장이 없으면 중단한다 — 진짜 최초라면 --init-ledger 로 명시한다.
//   시뮬레이션: --fixture=<path> 는 스토어 조회를 파일 응답으로 대체한다(실계정 없이 분기를 재현하는 용도).
//
// 출력은 수동 목록이 본체라 반드시 파일로 남긴다(위 tee). 프로세스는 exitCode 만 세우고 자연 종료하므로
// 파이프 뒤의 tee 가 끊기지 않는다.
//
// 원값 원장: 첫 UPDATE 이전에 canonical 이 확정된 구글 바인딩 전건의 current_period_starts_at 원값을 파일로
// 고정하고, 모든 재실행이 그 값을 previousBoundaryAt(비교 경계)과 prior 시작으로 읽는다. 교정된 컬럼을 경계로
// 다시 읽으면 전진 게이트가 "동일 주기 연장"으로 판정해 startTime 오염이 start < end 감사도 통과하는 채로
// 영구 보존된다. 원장은 append-only 다 — 한 번 고정한 항목은 재작성하지 않는다.
//
// 쓰기는 조회·정규화·게이트 검산을 메모리에서 전부 끝낸 뒤 단일 트랜잭션으로만 한다. 쓰기 전 실패는 무해하고
// (재실행이 같은 원값에서 다시 시작한다) 쓰기 실패는 전량 롤백이라 부분 갱신 상태가 존재하지 않는다.

import '@typie/lib/dayjs';

import fs from 'node:fs';
import path from 'node:path';
import { InAppPurchaseStore, PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, eq, exists, inArray, isNotNull, or, sql } from 'drizzle-orm';
import { db, pg, pgb, pgr, Plans, Subscriptions, UserInAppPurchases } from '#/db/index.ts';
import { getSubscriptionV2 } from '#/external/googleplay.ts';
import { isSubscriptionEntitled } from '#/utils/entitlement.ts';
import { normalizeGoogle } from '#/utils/iap-normalize.ts';
import { ACTIVE_SUBSCRIPTION_STATES } from '#/utils/plan.ts';
import type { PlanInterval } from '@typie/lib/enums';
import type { Transaction } from '#/db/index.ts';
import type { GoogleSubscriptionResult } from '#/external/googleplay.ts';
import type { IapPriorPeriod } from '#/utils/iap-normalize.ts';

const LEDGER_VERSION = 1;

type RawTimestamp = Date | string | null;

type LedgerEntry = {
  bindingId: string;
  subscriptionId: string;
  renewedAtOriginal: string;
  recordedAt: string;
};

type Ledger = {
  version: number;
  createdAt: string;
  originalPeriodStarts: LedgerEntry[];
  lastRun?: {
    at: string;
    dryRun: boolean;
    googleTotal: number;
    corrected: number;
    quarantined: number;
  };
};

const argValue = (name: string) => process.argv.find((arg) => arg.startsWith(`${name}=`))?.slice(name.length + 1);

const dryRun = process.argv.includes('--dry-run') || !!process.env.DRY_RUN;
const initLedger = process.argv.includes('--init-ledger');
const ledgerPath = path.resolve(argValue('--ledger') ?? process.env.BACKFILL_IAP_LEDGER_PATH ?? 'backfill-iap-periods-ledger.json');
const fixturePath = argValue('--fixture');

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

const toDayjs = (value: RawTimestamp) => (value === null ? null : dayjs(value));

const iso = (value: dayjs.Dayjs | null) => (value === null ? 'null' : value.toISOString());

const same = (left: dayjs.Dayjs | null, right: dayjs.Dayjs | null) => {
  if (left === null || right === null) {
    return left === right;
  }

  return left.valueOf() === right.valueOf();
};

class DryRunRollback extends Error {}

// 스토어 조회. 실계정 없이 분기를 재현해야 하는 시뮬레이션만 파일 응답으로 대체한다 — 형식은 조회 결과 그대로다.
type StoreLookup = (identifier: string) => Promise<GoogleSubscriptionResult>;

const loadFixtureLookup = (filePath: string): StoreLookup => {
  const fixtures = JSON.parse(fs.readFileSync(filePath, 'utf8')) as Record<string, GoogleSubscriptionResult>;

  return (identifier) => Promise.resolve(fixtures[identifier] ?? { kind: 'error' });
};

const lookupGoogleSubscription: StoreLookup = fixturePath ? loadFixtureLookup(path.resolve(fixturePath)) : getSubscriptionV2;

// 원장.

const loadLedger = () => {
  if (!fs.existsSync(ledgerPath)) {
    return null;
  }

  const parsed = JSON.parse(fs.readFileSync(ledgerPath, 'utf8')) as Ledger;
  if (parsed.version !== LEDGER_VERSION) {
    throw new Error(`ledger version mismatch: ${parsed.version} (expected ${LEDGER_VERSION})`);
  }

  return parsed;
};

// 손상된 원장은 복구 수단이 없다 — 1회차가 커밋되고 나면 원값은 어디에도 남아 있지 않다.
// 임시 파일에 쓰고 rename 으로 갈아끼워 부분 기록된 파일이 원장 자리에 앉지 못하게 한다.
const saveLedger = (ledger: Ledger) => {
  if (dryRun) {
    return;
  }

  const tempPath = `${ledgerPath}.tmp`;
  fs.writeFileSync(tempPath, `${JSON.stringify(ledger, null, 2)}\n`);
  fs.renameSync(tempPath, ledgerPath);
};

// 데이터.

type BindingRow = {
  id: string;
  userId: string;
  store: InAppPurchaseStore;
  identifier: string;
  subscriptionId: string | null;
  reconcileSuspendedAt: dayjs.Dayjs | null;
};

type SubscriptionRow = {
  id: string;
  userId: string;
  planId: string;
  state: SubscriptionState;
  planAvailability: PlanAvailability;
  startsAt: dayjs.Dayjs;
  createdAt: dayjs.Dayjs;
  currentPeriodStartsAt: dayjs.Dayjs;
  currentPeriodEndsAt: dayjs.Dayjs | null;
};

// current_period_ends_at 은 스키마상 NOT NULL 이지만 제약 생성 마이그레이션 전이라 실제로는 비어 있을 수 있다.
// starts_at·created_at 은 권한 판정(isSubscriptionEntitled)의 입력이다.
const subscriptionColumns = {
  id: Subscriptions.id,
  userId: Subscriptions.userId,
  planId: Subscriptions.planId,
  state: Subscriptions.state,
  planAvailability: Plans.availability,
  startsAt: Subscriptions.startsAt,
  createdAt: Subscriptions.createdAt,
  currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
  currentPeriodEndsAt: sql<RawTimestamp>`${Subscriptions.currentPeriodEndsAt}`,
};

type SelectedSubscription = Omit<SubscriptionRow, 'currentPeriodEndsAt'> & { currentPeriodEndsAt: RawTimestamp };

const toSubscriptionRow = (row: SelectedSubscription): SubscriptionRow => ({
  ...row,
  currentPeriodEndsAt: toDayjs(row.currentPeriodEndsAt),
});

// canonical 채움.

type CanonicalResolution =
  | { kind: 'suspended'; at: dayjs.Dayjs }
  | { kind: 'resolved'; subscription: SubscriptionRow; assigned: boolean }
  | { kind: 'manual'; reason: string };

const resolveCanonical = (
  binding: BindingRow,
  liveByUser: Map<string, SubscriptionRow[]>,
  canonicalById: Map<string, SubscriptionRow>,
): CanonicalResolution => {
  // 마커는 사람이 확정한 명시 격리이거나 이전 실행의 410 표식이다. 여기서 재선정하면 수동 지정이 매 실행
  // 목록으로 되돌아와 고정점에 도달하지 못한다.
  if (binding.reconcileSuspendedAt) {
    return { kind: 'suspended', at: binding.reconcileSuspendedAt };
  }

  if (binding.subscriptionId) {
    const canonical = canonicalById.get(binding.subscriptionId);

    if (!canonical) {
      return { kind: 'manual', reason: `설정된 canonical(${binding.subscriptionId}) 구독 행 부재` };
    }
    if (canonical.userId !== binding.userId) {
      return { kind: 'manual', reason: `설정된 canonical(${binding.subscriptionId})이 타 유저(${canonical.userId}) 구독` };
    }
    if (canonical.planAvailability !== PlanAvailability.IN_APP_PURCHASE) {
      return { kind: 'manual', reason: `설정된 canonical(${binding.subscriptionId})이 IAP 플랜 아님(${canonical.planAvailability})` };
    }

    // 상태는 보지 않는다 — 사람이 지정한 EXPIRED canonical 이 목록으로 회귀하면 고정점에 도달하지 못한다.
    return { kind: 'resolved', subscription: canonical, assigned: false };
  }

  const candidates = liveByUser.get(binding.userId) ?? [];
  if (candidates.length !== 1) {
    return { kind: 'manual', reason: `살아있는 IAP 구독 ${candidates.length}건 (${candidates.map((row) => row.id).join(', ') || '없음'})` };
  }

  return { kind: 'resolved', subscription: candidates[0], assigned: true };
};

// 구글 주기 교정.

type PeriodUpdate = {
  subscriptionId: string;
  periodStartsAt: dayjs.Dayjs;
  periodEndsAt: dayjs.Dayjs;
  planId: string | null;
};

type GoogleVerdict =
  | { kind: 'corrected'; detail: string; update: PeriodUpdate | null }
  | { kind: 'quarantined'; reason: string; suspend?: boolean; goneLive?: boolean };

const buildPeriodUpdate = (
  canonical: SubscriptionRow,
  periodStartsAt: dayjs.Dayjs,
  periodEndsAt: dayjs.Dayjs,
  planId: string | null,
): PeriodUpdate | null => {
  const planSettled = planId === null || planId === canonical.planId;
  if (same(canonical.currentPeriodStartsAt, periodStartsAt) && same(canonical.currentPeriodEndsAt, periodEndsAt) && planSettled) {
    return null;
  }

  return { subscriptionId: canonical.id, periodStartsAt, periodEndsAt, planId };
};

const judgeGoogleBinding = async ({
  binding,
  canonical,
  boundaryAt,
  planIntervals,
  iapPlanIds,
}: {
  binding: BindingRow;
  canonical: SubscriptionRow;
  boundaryAt: dayjs.Dayjs;
  planIntervals: Record<string, PlanInterval>;
  iapPlanIds: Set<string>;
}): Promise<GoogleVerdict> => {
  const currentPeriodEndsAt = canonical.currentPeriodEndsAt;
  if (currentPeriodEndsAt === null) {
    return { kind: 'quarantined', reason: 'canonical 주기 종료 미확정(SQL 단계 미완)' };
  }

  // 깨진 경계는 비교가 전부 false 로 떨어져 전진 게이트를 무조건 통과시킨다 — 순수 역산이 조용히 값을 덮는다.
  if (!boundaryAt.isValid()) {
    return { kind: 'quarantined', reason: '원장 원값이 유효한 시각이 아님' };
  }

  const subscription = await lookupGoogleSubscription(binding.identifier);

  if (subscription.kind === 'gone') {
    // 살아있음의 기준은 상태가 아니라 권한이다(런타임 재조정과 동일) — 기간이 지난 WILL_EXPIRE 를
    // 살아있다고 보면 410 의 지배적 다수가 사람 목록을 점유해 실제 확인 대상이 묻힌다.
    // 바인딩은 지우지 않는다 — (store, identifier) 가 앱 밖 재가입의 유일한 연결키다.
    if (!isSubscriptionEntitled({ ...canonical, currentPeriodEndsAt }, now)) {
      return { kind: 'quarantined', reason: `410 gone (state=${canonical.state}, 권한 없음) — 재조정 비활성 마커 설정`, suspend: true };
    }

    return { kind: 'quarantined', reason: `410 gone 인데 canonical 이 권한 보유(state=${canonical.state})`, goneLive: true };
  }

  if (subscription.kind === 'not-found') {
    return { kind: 'quarantined', reason: '404 not-found — 설정 오류일 수 있어 마커를 붙이지 않는다' };
  }

  if (subscription.kind === 'error') {
    return { kind: 'quarantined', reason: '스토어 조회 실패' };
  }

  // 보존용 현재 주기(prior)와 비교 경계(previousBoundaryAt)를 분리한다 — 합치면 유예 행의 prior 보존이
  // 0길이 주기가 되고, 1단계 복사값을 경계로 쓰면 전진 게이트가 오염된 시작을 영구 보존한다.
  const prior: IapPriorPeriod = {
    state: canonical.state,
    currentPeriodStartsAt: boundaryAt,
    currentPeriodEndsAt,
  };

  const normalized = normalizeGoogle({ purchase: subscription.purchase, prior, planIntervals, previousBoundaryAt: boundaryAt, now });

  if (normalized.kind === 'tracked') {
    if (!iapPlanIds.has(normalized.planKey)) {
      return { kind: 'quarantined', reason: `정규화 플랜(${normalized.planKey})이 IAP 플랜 목록에 없음` };
    }

    return {
      kind: 'corrected',
      detail: `tracked ${iso(normalized.periodStartsAt)}~${iso(normalized.periodEndsAt)} plan=${normalized.planKey}`,
      update: buildPeriodUpdate(canonical, normalized.periodStartsAt, normalized.periodEndsAt, normalized.planKey),
    };
  }

  if (normalized.kind === 'expired') {
    // 복권·회수 판정은 배포 후 재조정 몫이라 상태는 건드리지 않는다 — 관측된 주기만 교정한다.
    if (normalized.observed) {
      return {
        kind: 'corrected',
        detail: `expired(observed) ${iso(normalized.observed.periodStartsAt)}~${iso(normalized.observed.periodEndsAt)} state=${canonical.state} 유지`,
        update: buildPeriodUpdate(canonical, normalized.observed.periodStartsAt, normalized.observed.periodEndsAt, null),
      };
    }

    return { kind: 'quarantined', reason: 'expired 인데 관측 주기 없음' };
  }

  if (normalized.kind === 'defer') {
    return { kind: 'quarantined', reason: `판정 보류(${normalized.reason})` };
  }

  if (normalized.kind === 'untracked') {
    return { kind: 'quarantined', reason: `추적 대상 아님(${normalized.reason})` };
  }

  if (normalized.kind === 'unknown') {
    return { kind: 'quarantined', reason: `판정 불가(${normalized.reason})` };
  }

  // 정규화에 새 kind 가 생기면 컴파일에서 걸린다 — 기본 분기로 흡수하면 미지 결과가 조용히 격리로 섞인다.
  const exhaustive: never = normalized;

  return { kind: 'quarantined', reason: `정규화 결과 미지(${JSON.stringify(exhaustive)})` };
};

// 실행.

type ApplyPlan = {
  canonicalUpdates: { bindingId: string; subscriptionId: string }[];
  suspendMarkers: string[];
  periodUpdates: PeriodUpdate[];
};

const applyPlan = async (tx: Transaction, plan: ApplyPlan) => {
  for (const update of plan.canonicalUpdates) {
    await tx.update(UserInAppPurchases).set({ subscriptionId: update.subscriptionId }).where(eq(UserInAppPurchases.id, update.bindingId));
  }

  for (const bindingId of plan.suspendMarkers) {
    await tx.update(UserInAppPurchases).set({ reconcileSuspendedAt: now }).where(eq(UserInAppPurchases.id, bindingId));
  }

  for (const update of plan.periodUpdates) {
    await tx
      .update(Subscriptions)
      .set({
        currentPeriodStartsAt: update.periodStartsAt,
        currentPeriodEndsAt: update.periodEndsAt,
        ...(update.planId && { planId: update.planId }),
      })
      .where(eq(Subscriptions.id, update.subscriptionId));
  }
};

const main = async () => {
  console.log(`IAP 주기 백필 시작 — ${dryRun ? 'DRY RUN(트랜잭션 롤백)' : '실행'} / now=${now.toISOString()}`);
  console.log(`원장: ${ledgerPath}`);
  if (fixturePath) {
    console.log(`스토어 조회 대체(FIXTURE): ${path.resolve(fixturePath)}`);
  }

  const loaded = loadLedger();

  if (loaded === null && !initLedger) {
    // 경로 오타 한 번이 원장을 "신규 생성"으로 만들면 교정된 컬럼이 원값으로 승격되어 오염이 그대로 굳는다.
    const traces = await db
      .select({ id: UserInAppPurchases.id })
      .from(UserInAppPurchases)
      .where(or(isNotNull(UserInAppPurchases.subscriptionId), isNotNull(UserInAppPurchases.reconcileSuspendedAt)))
      .limit(1);

    if (traces.length > 0) {
      console.error(`원장(${ledgerPath})이 없는데 바인딩에는 이미 백필 흔적이 있다(canonical 또는 마커 기록됨).`);
      console.error('이전 실행의 원장 경로를 --ledger= 로 지정하거나, 진짜 최초 실행이면 --init-ledger 를 명시한다.');
      process.exitCode = 1;

      return;
    }
  }

  const ledger: Ledger = loaded ?? {
    version: LEDGER_VERSION,
    createdAt: now.toISOString(),
    originalPeriodStarts: [],
  };

  section('1단계 — canonical subscription_id 채움');
  console.log(`  원장 상태: ${loaded ? '기존 로드' : '신규 생성'}`);

  const bindings: BindingRow[] = await db
    .select({
      id: UserInAppPurchases.id,
      userId: UserInAppPurchases.userId,
      store: UserInAppPurchases.store,
      identifier: UserInAppPurchases.identifier,
      subscriptionId: UserInAppPurchases.subscriptionId,
      reconcileSuspendedAt: UserInAppPurchases.reconcileSuspendedAt,
    })
    .from(UserInAppPurchases)
    // 출력이 결과물이라 재실행 로그가 행 물리 순서로 흔들리면 실행 간 diff 가 신호를 잃는다.
    .orderBy(UserInAppPurchases.id);

  const liveRows = await db
    .select(subscriptionColumns)
    .from(Subscriptions)
    .innerJoin(Plans, eq(Plans.id, Subscriptions.planId))
    .where(
      and(
        eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
        inArray(Subscriptions.state, ACTIVE_SUBSCRIPTION_STATES),
        exists(
          db
            .select({ one: sql`1` })
            .from(UserInAppPurchases)
            .where(eq(UserInAppPurchases.userId, Subscriptions.userId)),
        ),
      ),
    );

  const liveByUser = new Map<string, SubscriptionRow[]>();
  for (const live of liveRows) {
    const row = toSubscriptionRow(live);
    const bucket = liveByUser.get(row.userId) ?? [];
    bucket.push(row);
    liveByUser.set(row.userId, bucket);
  }

  const assignedIds = bindings.map((binding) => binding.subscriptionId).filter((id): id is string => id !== null);
  const assignedRows =
    assignedIds.length === 0
      ? []
      : await db
          .select(subscriptionColumns)
          .from(Subscriptions)
          .innerJoin(Plans, eq(Plans.id, Subscriptions.planId))
          .where(inArray(Subscriptions.id, assignedIds));

  const canonicalById = new Map(assignedRows.map(toSubscriptionRow).map((row) => [row.id, row]));

  const canonicalUpdates: { bindingId: string; subscriptionId: string }[] = [];
  const resolutions = new Map<string, CanonicalResolution>();

  for (const binding of bindings) {
    const resolution = resolveCanonical(binding, liveByUser, canonicalById);
    resolutions.set(binding.id, resolution);

    if (resolution.kind === 'manual') {
      addManual('manualCanonicalBindingIds', `${binding.id} (store=${binding.store} user=${binding.userId} — ${resolution.reason})`);
      continue;
    }

    if (resolution.kind === 'resolved' && resolution.assigned) {
      canonicalUpdates.push({ bindingId: binding.id, subscriptionId: resolution.subscription.id });
    }
  }

  const countBy = (store: InAppPurchaseStore, predicate: (resolution: CanonicalResolution) => boolean) =>
    bindings.filter((binding) => binding.store === store && predicate(resolutions.get(binding.id) as CanonicalResolution)).length;

  for (const store of [InAppPurchaseStore.GOOGLE_PLAY, InAppPurchaseStore.APP_STORE]) {
    const total = bindings.filter((binding) => binding.store === store).length;
    const resolved = countBy(store, (resolution) => resolution.kind === 'resolved');
    const suspended = countBy(store, (resolution) => resolution.kind === 'suspended');
    const manual = countBy(store, (resolution) => resolution.kind === 'manual');
    console.log(`  ${store}: 총 ${total}건 / canonical 확정 ${resolved} (신규 지정 포함) / 마커 격리 ${suspended} / 수동 ${manual}`);
  }
  console.log(`  canonical 신규 기록 예정: ${canonicalUpdates.length}건`);

  section('원값 원장 — 구 renewed_at 스냅샷(첫 UPDATE 이전)');

  const ledgerByBinding = new Map(ledger.originalPeriodStarts.map((entry) => [entry.bindingId, entry]));
  const addedLedger: string[] = [];

  for (const binding of bindings) {
    if (binding.store !== InAppPurchaseStore.GOOGLE_PLAY) {
      continue;
    }

    const resolution = resolutions.get(binding.id);
    if (resolution?.kind !== 'resolved' || ledgerByBinding.has(binding.id)) {
      continue;
    }

    const entry: LedgerEntry = {
      bindingId: binding.id,
      subscriptionId: resolution.subscription.id,
      renewedAtOriginal: resolution.subscription.currentPeriodStartsAt.toISOString(),
      recordedAt: now.toISOString(),
    };

    ledger.originalPeriodStarts.push(entry);
    ledgerByBinding.set(entry.bindingId, entry);
    addedLedger.push(`${entry.bindingId} → ${entry.subscriptionId} renewed_at=${entry.renewedAtOriginal}`);
  }

  console.log(`  원장 항목: ${ledger.originalPeriodStarts.length}건 (이번 실행 신규 ${addedLedger.length}건)`);
  if (loaded && addedLedger.length > 0) {
    listing('재실행에서 새로 고정된 원값(원장 추가만 — 기존 항목은 재작성하지 않는다)', addedLedger);
  }

  // 원장은 첫 UPDATE 이전에 디스크로 확정한다 — 커밋 후에 쓰면 파일 쓰기 실패가 원값 증거를 잃는다.
  saveLedger(ledger);
  if (dryRun) {
    console.log('  DRY RUN — 원장 파일은 기록하지 않는다');
  }

  section('2단계 — Google 행 주기 교정(스토어 조회)');

  const plans = await db
    .select({ id: Plans.id, interval: Plans.interval })
    .from(Plans)
    .where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE));
  const planIntervals: Record<string, PlanInterval> = Object.fromEntries(plans.map((plan) => [plan.id, plan.interval]));
  const iapPlanIds = new Set(plans.map((plan) => plan.id));

  const googleBindings = bindings.filter((binding) => binding.store === InAppPurchaseStore.GOOGLE_PLAY);
  const periodUpdates: PeriodUpdate[] = [];
  const suspendMarkers: string[] = [];
  const corrections: string[] = [];
  const goneLiveIds: string[] = [];

  let corrected = 0;
  let quarantined = 0;

  for (const binding of googleBindings) {
    const resolution = resolutions.get(binding.id) as CanonicalResolution;

    if (resolution.kind === 'suspended') {
      quarantined += 1;
      addManual('quarantinedBindingIds', `${binding.id} (재조정 비활성 마커 ${iso(resolution.at)} — 명시 격리 유지)`);
      continue;
    }

    if (resolution.kind === 'manual') {
      quarantined += 1;
      addManual('quarantinedBindingIds', `${binding.id} (canonical 미확정 — ${resolution.reason})`);
      continue;
    }

    const canonical = resolution.subscription;
    const entry = ledgerByBinding.get(binding.id);

    if (!entry) {
      quarantined += 1;
      addManual('quarantinedBindingIds', `${binding.id} (원값 원장 항목 부재)`);
      continue;
    }

    // 원장 항목이 다른 구독의 원값이면 경계가 남의 주기다 — 추정하지 않고 사람에게 넘긴다.
    if (entry.subscriptionId !== canonical.id) {
      quarantined += 1;
      addManual(
        'quarantinedBindingIds',
        `${binding.id} (원장 원값이 다른 구독 ${entry.subscriptionId} 것 — 현재 canonical ${canonical.id})`,
      );
      continue;
    }

    const verdict = await judgeGoogleBinding({
      binding,
      canonical,
      boundaryAt: dayjs(entry.renewedAtOriginal),
      planIntervals,
      iapPlanIds,
    });

    if (verdict.kind === 'quarantined') {
      quarantined += 1;
      addManual('quarantinedBindingIds', `${binding.id} (${verdict.reason})`);

      if (verdict.suspend) {
        suspendMarkers.push(binding.id);
      }
      if (verdict.goneLive) {
        goneLiveIds.push(`${binding.id} (subscription=${canonical.id} state=${canonical.state} end=${iso(canonical.currentPeriodEndsAt)})`);
        addManual('goneLiveIds', `${binding.id} (subscription=${canonical.id} state=${canonical.state})`);
      }

      continue;
    }

    corrected += 1;

    if (verdict.update) {
      periodUpdates.push(verdict.update);
    }

    // 교정 성공은 전건 출력한다 — 값이 그대로인 행도 "스토어가 이 주기를 확인해 줬다"는 기록이라
    // 목록에서 빠지면 무엇이 검증되지 않았는지 사후에 분간할 수 없다.
    corrections.push(
      `${verdict.update ? '[변경]' : '[유지]'} ${binding.id} subscription=${canonical.id} 원값=${entry.renewedAtOriginal} 현재=${iso(canonical.currentPeriodStartsAt)}~${iso(canonical.currentPeriodEndsAt)} → ${verdict.detail}`,
    );
  }

  listing('주기 교정 성공(전건)', corrections);
  console.log(`  값 변경 ${periodUpdates.length}건 / 이미 정합 ${corrected - periodUpdates.length}건`);
  listing('410 gone — 재조정 비활성 마커 설정', suspendMarkers);
  listing('410 gone 인데 canonical 이 살아있음(사람 확인)', goneLiveIds);

  section('3단계 — 완료 게이트');

  const googleTotal = googleBindings.length;
  const balanced = googleTotal === corrected + quarantined;
  console.log(`  Google 대상 총수 ${googleTotal} = 교정 성공 ${corrected} + 명시적 격리 ${quarantined} → ${balanced ? 'OK' : '불일치'}`);

  // 두 카운터만 맞추면 같은 루프가 세는 구조적 항등식이라 검출력이 없다. 사람이 실제로 받아 보는
  // 산출물(교정 목록·격리 목록)의 길이와 대조해야 "센 것"과 "출력된 것"의 어긋남이 잡힌다.
  const listed = manualLists.quarantinedBindingIds?.length ?? 0;
  const crossChecked = corrections.length === corrected && listed === quarantined;
  console.log(
    `  교차 검증: 교정 목록 ${corrections.length} = ${corrected} ∧ 격리 목록 ${listed} = ${quarantined} → ${crossChecked ? 'OK' : '불일치'}`,
  );

  if (!balanced || !crossChecked) {
    console.error('완료 게이트 불일치 — 어떤 바인딩도 무음으로 통과시키지 않기 위해 쓰기 없이 중단한다.');
    process.exitCode = 1;

    return;
  }

  section('적용');

  const plan: ApplyPlan = { canonicalUpdates, suspendMarkers, periodUpdates };
  console.log(`  canonical ${plan.canonicalUpdates.length}건 / 마커 ${plan.suspendMarkers.length}건 / 주기 ${plan.periodUpdates.length}건`);

  try {
    await db.transaction(async (tx) => {
      await applyPlan(tx, plan);

      if (dryRun) {
        throw new DryRunRollback('dry-run');
      }
    });
  } catch (err) {
    if (!(err instanceof DryRunRollback)) {
      throw err;
    }

    console.log('  DRY RUN — 트랜잭션 롤백 완료(DB 반영 없음)');
  }

  section('수동 처리 목록');
  const buckets = Object.keys(manualLists).toSorted((a, b) => a.localeCompare(b));
  if (buckets.length === 0) {
    console.log('  없음');
  }
  for (const bucket of buckets) {
    listing(bucket, manualLists[bucket]);
  }

  ledger.lastRun = { at: now.toISOString(), dryRun, googleTotal, corrected, quarantined };
  saveLedger(ledger);

  console.log(`\n완료 — ${dryRun ? 'DRY RUN' : '적용'} / 교정 ${corrected} / 격리 ${quarantined}`);
};

// process.exit 는 파이프(tee)로 흘려보낸 출력을 자르므로 exitCode 만 세우고 커넥션을 닫아 자연 종료한다.
await main();
await pg.end();
await pgr.end();
await pgb.end();
