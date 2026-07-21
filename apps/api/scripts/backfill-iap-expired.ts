#!/usr/bin/env node

// 구 renewal:cancel 크론이 로컬 상태만 보고 잘못 EXPIRED 처리한 IAP 구독을 스토어와 대조해 복구한다.
// 일상 재조정 크론은 (동시 환불의 부활 위험 때문에) EXPIRED 를 건드리지 않으므로, 롤아웃 시 이 백필을 1회 실행한다.
// 환불/철회된 구독은 스토어가 활성 상태를 반환하지 않으므로, 스토어가 실제 활성일 때만 복구한다(부활 위험 없음).
//
// 유저당 바인딩은 하나뿐이므로, 유저당 "가장 최근" EXPIRED IAP 구독 하나만 되살린다(여러 개 되살리면 ACTIVE 유니크 위반).
// 되살릴 때 플랜/만료일은 스토어의 현재 값을 그대로 반영한다.
//
// 미리보기: DRY_RUN=1 doppler run -- node scripts/backfill-iap-expired.ts
// 실제 적용: doppler run -- node scripts/backfill-iap-expired.ts

import { InAppPurchaseStore, PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, eq, inArray, ne, notExists, sql } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import { db, first, Plans, Subscriptions, UserInAppPurchases } from '#/db/index.ts';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';

const dryRun = !!process.env.DRY_RUN;

const LIVE_STATES = [
  SubscriptionState.ACTIVE,
  SubscriptionState.WILL_EXPIRE,
  SubscriptionState.IN_GRACE_PERIOD,
  SubscriptionState.WILL_ACTIVATE,
];

const other = alias(Subscriptions, 'other');

// 다른 live 구독이 없는(=현재 접근이 없는) EXPIRED IAP 구독을 모은다.
const rows = await db
  .select({
    id: Subscriptions.id,
    userId: Subscriptions.userId,
    expiresAt: Subscriptions.expiresAt,
    store: UserInAppPurchases.store,
    identifier: UserInAppPurchases.identifier,
  })
  .from(Subscriptions)
  .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
  .innerJoin(UserInAppPurchases, eq(UserInAppPurchases.userId, Subscriptions.userId))
  .where(
    and(
      eq(Subscriptions.state, SubscriptionState.EXPIRED),
      eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
      notExists(
        db
          .select({ one: sql`1` })
          .from(other)
          .where(and(eq(other.userId, Subscriptions.userId), ne(other.id, Subscriptions.id), inArray(other.state, LIVE_STATES))),
      ),
    ),
  );

// 유저당 가장 최근 만료 행 하나만 남긴다. (오름차순 정렬 후 Map 이 마지막 값을 유지 → 가장 최근)
const candidates = [
  ...new Map(rows.toSorted((a, b) => dayjs(a.expiresAt).valueOf() - dayjs(b.expiresAt).valueOf()).map((row) => [row.userId, row])).values(),
];

console.log(`대상 유저(EXPIRED IAP, 현재 접근 없음): ${candidates.length}명 (dryRun=${dryRun})`);

let reactivated = 0;
let untouched = 0;
const failed: string[] = [];

for (const candidate of candidates) {
  // 스토어가 실제 활성인지 + 현재 플랜/만료일 조회.
  // 일시적 조회 실패(타임아웃·레이트리밋·자격증명 등)를 "비활성"과 섞지 않는다 — 실패는 별도 추적해 재실행을 유도한다.
  let planId: string | null = null;
  let expiresAt: dayjs.Dayjs | null = null;
  let lookupFailed = false;

  if (candidate.store === InAppPurchaseStore.APP_STORE) {
    // getSubscriptionStatus 는 조회 실패를 'error'(전 환경 실패)로, 확인됐으나 활성 아님을 'unknown' 등으로 구분하고,
    // 후보 originalTransactionId 와 일치하는 시리즈만 본다(getSubscription 은 비활성·실패를 모두 throw 하고 첫 시리즈만 돌려줌).
    const status = await appstore.getSubscriptionStatus(candidate.identifier);
    if (status.kind === 'error') {
      lookupFailed = true;
    } else if (status.kind === 'active' && status.productId && status.expiresDate && dayjs(status.expiresDate).isAfter(dayjs())) {
      planId = status.productId.toUpperCase();
      expiresAt = dayjs(status.expiresDate);
    }
    // 그 외(unknown/grace/suspended/expired/revoked)는 활성 확인 불가 — 되살리지 않고 유지
  } else if (candidate.store === InAppPurchaseStore.GOOGLE_PLAY) {
    try {
      const subscription = await googleplay.getSubscription(candidate.identifier);
      const item = subscription.lineItems?.[0];
      const basePlanId = item?.offerDetails?.basePlanId;
      if (
        subscription.subscriptionState === 'SUBSCRIPTION_STATE_ACTIVE' &&
        basePlanId &&
        item?.expiryTime &&
        dayjs(item.expiryTime).isAfter(dayjs())
      ) {
        planId = basePlanId.toUpperCase();
        expiresAt = dayjs(item.expiryTime);
      }
      // 그 외(EXPIRED/CANCELED/ON_HOLD 등)는 스토어가 확인해 준 비활성 — 유지
    } catch (err) {
      // getSubscription 은 비활성은 정상 응답으로 상태를 돌려주므로, throw 는 일시적 조회 실패다.
      lookupFailed = true;
      console.log(`  ! ${candidate.id} (google): 조회 실패 — ${err instanceof Error ? err.message : String(err)}`);
    }
  }

  if (lookupFailed) {
    failed.push(candidate.id);
    continue;
  }

  if (!planId || !expiresAt) {
    untouched += 1;
    continue;
  }

  const nextExpiresAt = expiresAt;
  const nextPlanId = planId;

  const applied = await db.transaction(async (tx) => {
    // 되살릴 플랜이 실제 IAP 플랜인지 확인
    const plan = await tx
      .select({ id: Plans.id })
      .from(Plans)
      .where(and(eq(Plans.id, nextPlanId), eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)))
      .then(first);
    if (!plan) {
      return false;
    }

    // ACTIVE 유니크 위반 방지: 조회 이후 이 유저에게 live 구독이 생겼는지 재확인한다.
    const live = await tx
      .select({ id: Subscriptions.id })
      .from(Subscriptions)
      .where(and(eq(Subscriptions.userId, candidate.userId), inArray(Subscriptions.state, LIVE_STATES)))
      .for('update')
      .then(first);
    if (live) {
      return false;
    }

    if (!dryRun) {
      await tx
        .update(Subscriptions)
        .set({ state: SubscriptionState.ACTIVE, planId: plan.id, expiresAt: nextExpiresAt })
        .where(eq(Subscriptions.id, candidate.id));
    }

    return true;
  });

  if (applied) {
    reactivated += 1;
    console.log(
      `  ✓ ${candidate.id} (user ${candidate.userId}, ${candidate.store}) → ACTIVE plan=${nextPlanId} (~${nextExpiresAt.toISOString()})`,
    );
  } else {
    untouched += 1;
  }
}

console.log(`완료: 재활성 ${reactivated}, 유지 ${untouched}, 조회실패 ${failed.length}${dryRun ? ' (DRY_RUN — 실제 변경 없음)' : ''}`);

// 조회 실패가 남으면 잘못 만료된 유료 유저가 복구되지 않고 묻힐 수 있으므로, 실패 후보를 남기고 nonzero 로 종료해 재실행을 유도한다.
if (failed.length > 0) {
  console.log(`조회 실패(재실행 필요) 후보: ${failed.join(', ')}`);
  process.exit(1);
}

process.exit(0);
