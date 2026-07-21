import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { SUBSCRIPTION_GRACE_DAYS } from '@typie/lib/const';
import { InAppPurchaseStore, PaymentInvoiceState, PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, desc, eq, gt, inArray, lte, ne, or, sql } from 'drizzle-orm';
import { db, first, firstOrThrow, PaymentInvoices, Plans, Subscriptions, UserBillingKeys, UserInAppPurchases } from '#/db/index.ts';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';
import * as portone from '#/external/portone.ts';
import { getSubscriptionExpiresAt, hasBillableUsageDuring, payInvoiceWithBillingKey } from '#/utils/index.ts';
import { lockUserSubscriptionState } from '#/utils/subscription-lock.ts';
import { enqueueJob } from '../index.ts';
import { defineCron, defineJob } from '../types.ts';
import { resolveReconcileAction } from './subscription-reconcile.ts';
import type { ReconcileStoreState } from './subscription-reconcile.ts';

const log = logger.getChild('subscription');

export const SubscriptionRenewalCron = defineCron('subscription:renewal', '0 10 * * *', async () => {
  const now = dayjs();

  await db.transaction(
    async (tx) => {
      const overdueInvoices = await tx
        .select({ id: PaymentInvoices.id })
        .from(PaymentInvoices)
        .where(and(eq(PaymentInvoices.state, PaymentInvoiceState.OVERDUE)));

      for (const invoice of overdueInvoices) {
        await enqueueJob('subscription:renewal:retry', invoice.id);
      }

      const initialSubscriptions = await tx
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
        .where(
          and(
            eq(Subscriptions.state, SubscriptionState.ACTIVE),
            lte(Subscriptions.expiresAt, now),
            eq(Plans.availability, PlanAvailability.BILLING_KEY),
          ),
        );

      for (const subscription of initialSubscriptions) {
        await enqueueJob('subscription:renewal:initial', subscription.id);
      }

      const planChangeSubscriptions = await tx
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .where(and(eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE), lte(Subscriptions.startsAt, now)));

      for (const subscription of planChangeSubscriptions) {
        await enqueueJob('subscription:renewal:plan-change', subscription.id, { jobId: `plan-change-${subscription.id}` });
      }

      // IAP 는 스토어 웹훅/재조정 크론이 만료를 담당한다. 여기서 처리하면 스토어가 갱신·재개한 구독을
      // 잘못 만료시킬 수 있으므로 제외한다(빌링키·트라이얼만 처리).
      const cancelSubscriptions = await tx
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
        .where(
          and(
            eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE),
            lte(Subscriptions.expiresAt, now),
            ne(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
          ),
        );

      for (const subscription of cancelSubscriptions) {
        await enqueueJob('subscription:renewal:cancel', subscription.id, { jobId: `renewal-cancel-${subscription.id}` });
      }
    },
    { isolationLevel: 'repeatable read' },
  );
});

// 일일 크론은 백스톱으로 남긴다. 결제 재시도(OVERDUE)·정기 갱신은 하루 1회가 정책이므로 여기에 넣지 않는다.
// 트랜잭션 없이 조회한다 — 잡이 상태를 재검증하므로 스냅샷이 불필요하고, Redis enqueue 를 DB 트랜잭션
// 안에서 하면 지연 시 커넥션을 붙든다.
export const SubscriptionTransitionCron = defineCron('subscription:transition', '* * * * *', async () => {
  const now = dayjs();

  const planChangeSubscriptions = await db
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .where(and(eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE), lte(Subscriptions.startsAt, now)));

  for (const subscription of planChangeSubscriptions) {
    await enqueueJob('subscription:renewal:plan-change', subscription.id, { jobId: `plan-change-${subscription.id}` });
  }

  // IAP 는 스토어 웹훅/재조정 크론이 만료를 담당한다. 여기서 처리하면 스토어가 갱신·재개한 구독을
  // 잘못 만료시킬 수 있으므로 제외한다(빌링키·트라이얼만 처리).
  const cancelSubscriptions = await db
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE),
        lte(Subscriptions.expiresAt, now),
        ne(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
      ),
    );

  for (const subscription of cancelSubscriptions) {
    await enqueueJob('subscription:renewal:cancel', subscription.id, { jobId: `renewal-cancel-${subscription.id}` });
  }
});

export const SubscriptionRenewalInitialJob = defineJob('subscription:renewal:initial', async (subscriptionId: string) => {
  await db.transaction(async (tx) => {
    // userId 는 불변 컬럼이라 무락 조회가 안전하다 — advisory 를 행 잠금보다 먼저 잡기 위한 사전 조회.
    const subscriptionRef = await tx
      .select({ userId: Subscriptions.userId })
      .from(Subscriptions)
      .where(eq(Subscriptions.id, subscriptionId))
      .then(first);

    if (!subscriptionRef) {
      return;
    }

    await lockUserSubscriptionState(tx, subscriptionRef.userId);

    const subscription = await tx
      .select({
        id: Subscriptions.id,
        userId: Subscriptions.userId,
        state: Subscriptions.state,
        renewedAt: Subscriptions.renewedAt,
        expiresAt: Subscriptions.expiresAt,
        plan: { fee: Plans.fee, interval: Plans.interval },
      })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(Subscriptions.id, subscriptionId))
      .for('no key update', { of: Subscriptions })
      .then(first);

    if (!subscription || subscription.state !== SubscriptionState.ACTIVE || dayjs(subscription.expiresAt).isAfter(dayjs())) {
      return;
    }

    const hasUsage = await hasBillableUsageDuring(tx, subscription.userId, subscription.renewedAt, subscription.expiresAt);

    if (!hasUsage) {
      // 미사용 면제
      const waivedInvoice = await tx
        .insert(PaymentInvoices)
        .values({
          userId: subscription.userId,
          subscriptionId: subscription.id,
          amount: 0,
          state: PaymentInvoiceState.WAIVED,
          dueAt: subscription.expiresAt,
        })
        .returning({ id: PaymentInvoices.id })
        .then(firstOrThrow);

      const newExpiresAt = getSubscriptionExpiresAt(subscription.expiresAt, subscription.plan.interval);
      await tx
        .update(Subscriptions)
        .set({ renewedAt: subscription.expiresAt, expiresAt: newExpiresAt })
        .where(eq(Subscriptions.id, subscriptionId));

      // 연속 면제 여부 확인 — 직전 invoice가 WAIVED가 아니면 첫 면제
      const previousInvoice = await tx
        .select({ state: PaymentInvoices.state })
        .from(PaymentInvoices)
        .where(and(eq(PaymentInvoices.subscriptionId, subscriptionId), ne(PaymentInvoices.id, waivedInvoice.id)))
        .orderBy(desc(PaymentInvoices.createdAt))
        .limit(1)
        .then(first);

      if (!previousInvoice || previousInvoice.state !== PaymentInvoiceState.WAIVED) {
        await enqueueJob('email:subscription-waived', subscriptionId, { delay: 5 * 60 * 1000 });
      }

      return;
    }

    // 기존 결제 플로우
    const invoice = await tx
      .insert(PaymentInvoices)
      .values({
        userId: subscription.userId,
        subscriptionId: subscription.id,
        amount: subscription.plan.fee,
        state: PaymentInvoiceState.UPCOMING,
        dueAt: subscription.expiresAt,
      })
      .returning({ id: PaymentInvoices.id })
      .then(firstOrThrow);

    const success = await payInvoiceWithBillingKey(tx, invoice.id);
    if (success) {
      const newExpiresAt = getSubscriptionExpiresAt(subscription.expiresAt, subscription.plan.interval);
      await tx
        .update(Subscriptions)
        .set({ renewedAt: subscription.expiresAt, expiresAt: newExpiresAt })
        .where(eq(Subscriptions.id, subscriptionId));
      await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.PAID }).where(eq(PaymentInvoices.id, invoice.id));
    } else {
      await tx.update(Subscriptions).set({ state: SubscriptionState.IN_GRACE_PERIOD }).where(eq(Subscriptions.id, subscriptionId));
      await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.OVERDUE }).where(eq(PaymentInvoices.id, invoice.id));

      await enqueueJob('email:subscription-grace-period', subscription.id, { delay: 5 * 60 * 1000 });
    }
  });
});

export const SubscriptionRenewalRetryJob = defineJob('subscription:renewal:retry', async (invoiceId: string) => {
  await db.transaction(async (tx) => {
    // 교착 방지: 모든 갱신·환불 경로는 구독 → 인보이스 순으로 잠근다(환불은 구독을 잠근 채 인보이스를 갱신한다).
    // subscriptionId/userId 는 불변 컬럼이라 무락 조회가 안전하고, 상태는 아래 잠금 조회에서 재검증한다.
    const invoiceRef = await tx
      .select({ subscriptionId: PaymentInvoices.subscriptionId, userId: PaymentInvoices.userId })
      .from(PaymentInvoices)
      .where(eq(PaymentInvoices.id, invoiceId))
      .then(first);

    if (!invoiceRef) {
      return;
    }

    await lockUserSubscriptionState(tx, invoiceRef.userId);

    // 락 대기 중 대상이 사라졌으면 조용한 no-op — throw 는 불필요한 큐 재시도·Sentry 노이즈다.
    const lockedSubscription = await tx
      .select({ id: Subscriptions.id })
      .from(Subscriptions)
      .where(eq(Subscriptions.id, invoiceRef.subscriptionId))
      .for('no key update')
      .then(first);

    if (!lockedSubscription) {
      return;
    }

    const invoice = await tx
      .select({
        id: PaymentInvoices.id,
        state: PaymentInvoices.state,
        subscription: {
          id: Subscriptions.id,
          userId: Subscriptions.userId,
          state: Subscriptions.state,
          expiresAt: Subscriptions.expiresAt,
        },
        plan: { interval: Plans.interval },
      })
      .from(PaymentInvoices)
      .innerJoin(Subscriptions, eq(PaymentInvoices.subscriptionId, Subscriptions.id))
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(PaymentInvoices.id, invoiceId))
      .for('no key update', { of: PaymentInvoices })
      .then(first);

    if (!invoice || invoice.state !== PaymentInvoiceState.OVERDUE || invoice.subscription.state !== SubscriptionState.IN_GRACE_PERIOD) {
      return;
    }

    // 유예 중 다른 채널(IAP 등)로 유효 구독이 생겼으면 이 구독·인보이스는 낡은 청구다 — 결제 없이 거둔다.
    // 슬롯 선점이 매번 유니크 충돌로 abort 되어 유예 종료 판정에 영원히 못 가는 정지 상태도 이 분기가 푼다.
    const conflicting = await tx
      .select({ id: Subscriptions.id })
      .from(Subscriptions)
      .where(
        and(
          eq(Subscriptions.userId, invoice.subscription.userId),
          ne(Subscriptions.id, invoice.subscription.id),
          or(
            inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.IN_GRACE_PERIOD]),
            and(eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE), gt(Subscriptions.expiresAt, dayjs())),
          ),
        ),
      )
      .then(first);

    if (conflicting) {
      log.info('renewal retry superseded by another live subscription {*}', {
        subscriptionId: invoice.subscription.id,
        userId: invoice.subscription.userId,
        conflictingSubscriptionId: conflicting.id,
      });
      await tx
        .update(Subscriptions)
        .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
        .where(eq(Subscriptions.id, invoice.subscription.id));
      await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.CANCELED }).where(eq(PaymentInvoices.id, invoice.id));
      return;
    }

    const billingKey = await tx
      .select({ id: UserBillingKeys.id })
      .from(UserBillingKeys)
      .where(eq(UserBillingKeys.userId, invoice.subscription.userId))
      .then(first);

    // 슬롯 선점: ACTIVE 전이를 결제보다 먼저 둔다 — 동시 다른 채널의 ACTIVE 와의 유니크 충돌이
    // PG 호출 전에 발생해, 승인 후 롤백(재과금) 경로가 구조적으로 사라진다. 실패 시 아래에서 되돌린다.
    await tx.update(Subscriptions).set({ state: SubscriptionState.ACTIVE }).where(eq(Subscriptions.id, invoice.subscription.id));

    // 빌링키 부재를 payInvoiceWithBillingKey 에 넘기면 try 밖 firstOrThrow 가 throw → 롤백 → 유예 종료 판정에
    // 영원히 도달하지 못해 무기한 무료 유예가 된다. 결제 시도 없이 실패로 진행해 종료 판정을 반드시 태운다.
    const success = billingKey ? await payInvoiceWithBillingKey(tx, invoice.id) : false;
    if (success) {
      const newExpiresAt = getSubscriptionExpiresAt(invoice.subscription.expiresAt, invoice.plan.interval);
      await tx
        .update(Subscriptions)
        .set({ expiresAt: newExpiresAt, renewedAt: invoice.subscription.expiresAt })
        .where(eq(Subscriptions.id, invoice.subscription.id));
      await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.PAID }).where(eq(PaymentInvoices.id, invoice.id));
    } else {
      await tx.update(Subscriptions).set({ state: SubscriptionState.IN_GRACE_PERIOD }).where(eq(Subscriptions.id, invoice.subscription.id));

      const gracePeriodEndsAt = invoice.subscription.expiresAt.add(SUBSCRIPTION_GRACE_DAYS, 'days').kst();

      if (gracePeriodEndsAt.subtract(1, 'day').isSame(dayjs.kst(), 'day')) {
        await enqueueJob('email:subscription-expiring', invoice.subscription.id, { delay: 5 * 60 * 1000 });
      }

      if (gracePeriodEndsAt.isBefore(dayjs())) {
        await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
          .where(eq(Subscriptions.id, invoice.subscription.id));
        await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.CANCELED }).where(eq(PaymentInvoices.id, invoice.id));

        await enqueueJob('email:subscription-expired', invoice.subscription.id, { delay: 5 * 60 * 1000 });
      }
    }
  });
});

export const SubscriptionRenewalPlanChangeJob = defineJob('subscription:renewal:plan-change', async (subscriptionId: string) => {
  await db.transaction(async (tx) => {
    // userId 는 불변 컬럼이라 무락 조회가 안전하다 — advisory 를 행 잠금보다 먼저 잡기 위한 사전 조회.
    const subscriptionRef = await tx
      .select({ userId: Subscriptions.userId })
      .from(Subscriptions)
      .where(eq(Subscriptions.id, subscriptionId))
      .then(first);

    if (!subscriptionRef) {
      return;
    }

    await lockUserSubscriptionState(tx, subscriptionRef.userId);

    // 락 대기 중 취소가 예약을 지웠으면 조용한 no-op — throw 는 불필요한 큐 재시도·Sentry 노이즈다.
    const subscription = await tx
      .select({
        id: Subscriptions.id,
        userId: Subscriptions.userId,
        state: Subscriptions.state,
        startsAt: Subscriptions.startsAt,
        plan: { fee: Plans.fee },
      })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(Subscriptions.id, subscriptionId))
      .for('no key update', { of: Subscriptions })
      .then(first);

    if (!subscription || subscription.state !== SubscriptionState.WILL_ACTIVATE || dayjs(subscription.startsAt).isAfter(dayjs())) {
      return;
    }

    // 예약 뒤 다른 채널(IAP 등)로 유효 구독이 생겼으면 예약은 낡은 의사다 — 결제 없이 거둔다.
    const conflicting = await tx
      .select({ id: Subscriptions.id })
      .from(Subscriptions)
      .where(
        and(
          eq(Subscriptions.userId, subscription.userId),
          ne(Subscriptions.id, subscriptionId),
          or(
            inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.IN_GRACE_PERIOD]),
            and(eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE), gt(Subscriptions.expiresAt, dayjs())),
          ),
        ),
      )
      .then(first);

    if (conflicting) {
      log.info('plan-change superseded by another live subscription {*}', {
        subscriptionId,
        userId: subscription.userId,
        conflictingSubscriptionId: conflicting.id,
      });
      await tx.delete(Subscriptions).where(eq(Subscriptions.id, subscriptionId));
      return;
    }

    const billingKey = await tx
      .select({ id: UserBillingKeys.id })
      .from(UserBillingKeys)
      .where(eq(UserBillingKeys.userId, subscription.userId))
      .then(first);

    // 슬롯 선점: ACTIVE 전이를 결제보다 먼저 둔다 — 동시 다른 채널의 ACTIVE 와의 유니크 충돌이
    // PG 호출 전에 발생해, 승인 후 롤백(재과금) 경로가 구조적으로 사라진다. 실패 시 아래에서 유예로 전이한다.
    await tx.update(Subscriptions).set({ state: SubscriptionState.ACTIVE }).where(eq(Subscriptions.id, subscriptionId));

    const invoice = await tx
      .insert(PaymentInvoices)
      .values({
        userId: subscription.userId,
        subscriptionId: subscription.id,
        amount: subscription.plan.fee,
        state: PaymentInvoiceState.UPCOMING,
        dueAt: subscription.startsAt,
      })
      .returning({ id: PaymentInvoices.id })
      .then(firstOrThrow);

    // 빌링키 부재를 payInvoiceWithBillingKey 에 넘기면 try 밖 firstOrThrow 가 throw → 롤백 → 매 분 재실행 루프가 된다.
    const success = billingKey ? await payInvoiceWithBillingKey(tx, invoice.id) : false;
    if (success) {
      await tx.update(Subscriptions).set({ renewedAt: subscription.startsAt }).where(eq(Subscriptions.id, subscriptionId));
      await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.PAID }).where(eq(PaymentInvoices.id, invoice.id));
    } else {
      await tx
        .update(Subscriptions)
        .set({ expiresAt: subscription.startsAt, state: SubscriptionState.IN_GRACE_PERIOD })
        .where(eq(Subscriptions.id, subscriptionId));
      await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.OVERDUE }).where(eq(PaymentInvoices.id, invoice.id));

      await enqueueJob('email:subscription-grace-period', subscription.id, { delay: 5 * 60 * 1000 });
    }
  });
});

export const SubscriptionRenewalCancelJob = defineJob('subscription:renewal:cancel', async (subscriptionId: string) => {
  const billingKey = await db.transaction(async (tx) => {
    const subscriptionRef = await tx
      .select({ userId: Subscriptions.userId })
      .from(Subscriptions)
      .where(eq(Subscriptions.id, subscriptionId))
      .then(first);

    if (!subscriptionRef) {
      return null;
    }

    await lockUserSubscriptionState(tx, subscriptionRef.userId);

    const subscription = await tx
      .select({
        userId: Subscriptions.userId,
        state: Subscriptions.state,
        expiresAt: Subscriptions.expiresAt,
        availability: Plans.availability,
      })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(Subscriptions.id, subscriptionId))
      .for('no key update', { of: Subscriptions })
      .then(first);

    if (!subscription) {
      return null;
    }

    // IAP 배제를 상태 변경보다 먼저 한다 — 구버전이 큐에 넣었거나 롤백된 워커가 만든 IAP 잡이 배포 중 재시도돼도
    // IAP 구독을 EXPIRED(재조정으로 복구 불가)로 만들지 않도록 한다. IAP 만료는 스토어 웹훅/재조정이 담당한다.
    if (
      subscription.availability === PlanAvailability.IN_APP_PURCHASE ||
      subscription.state !== SubscriptionState.WILL_EXPIRE ||
      dayjs(subscription.expiresAt).isAfter(dayjs())
    ) {
      return null;
    }

    // 빌링키 취소 확정 또는 트라이얼 만료 — 둘 다 EXPIRED 로 전이한다.
    await tx
      .update(Subscriptions)
      .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
      .where(eq(Subscriptions.id, subscriptionId));

    // 빌링키 정리는 BILLING_KEY 플랜에만 해당한다(트라이얼은 빌링키가 없음).
    if (subscription.availability !== PlanAvailability.BILLING_KEY) {
      return null;
    }

    const remainingBillingKeySubscription = await tx
      .select({ id: Subscriptions.id })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(
        and(
          eq(Subscriptions.userId, subscription.userId),
          ne(Subscriptions.id, subscriptionId),
          inArray(Subscriptions.state, [
            SubscriptionState.ACTIVE,
            SubscriptionState.WILL_ACTIVATE,
            SubscriptionState.WILL_EXPIRE,
            SubscriptionState.IN_GRACE_PERIOD,
          ]),
          eq(Plans.availability, PlanAvailability.BILLING_KEY),
        ),
      )
      .then(first);

    if (remainingBillingKeySubscription) {
      return null;
    }

    const billingKey = await tx
      .delete(UserBillingKeys)
      .where(eq(UserBillingKeys.userId, subscription.userId))
      .returning({ billingKey: UserBillingKeys.billingKey })
      .then(first);

    return billingKey;
  });

  if (billingKey) {
    try {
      await portone.deleteBillingKey({ billingKey: billingKey.billingKey });
    } catch (err) {
      Sentry.captureException(err);
    }
  }
});

export const SubscriptionReconcileInAppPurchaseCron = defineCron('subscription:reconcile-iap', '0 4 * * *', async () => {
  const subscriptions = await db
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
        inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD]),
      ),
    );

  for (const subscription of subscriptions) {
    await enqueueJob('subscription:reconcile-iap:sync', subscription.id);
  }
});

export const SubscriptionReconcileInAppPurchaseJob = defineJob('subscription:reconcile-iap:sync', async (subscriptionId: string) => {
  const subscription = await db
    .select({ id: Subscriptions.id, userId: Subscriptions.userId, state: Subscriptions.state, expiresAt: Subscriptions.expiresAt })
    .from(Subscriptions)
    .where(eq(Subscriptions.id, subscriptionId))
    .then(first);

  if (!subscription) {
    return;
  }

  const binding = await db
    .select({ store: UserInAppPurchases.store, identifier: UserInAppPurchases.identifier })
    .from(UserInAppPurchases)
    .where(eq(UserInAppPurchases.userId, subscription.userId))
    .then(first);

  if (!binding) {
    return;
  }

  // 스토어가 명시적으로 상태를 확인해 준 경우에만 판정한다.
  // 조회 실패(네트워크·타임아웃·5xx)나 모호 상태는 unknown 으로 두어, 실결제 구독을 잘못 만료시키지 않는다.
  // suspended = 보류·재청구·일시중지(권한 없음이나 복구 가능), expired = 일반 만료(하루 여유 후), revoked = 환불·철회(즉시).
  let storeState: ReconcileStoreState = 'unknown';
  let storeExpiresAt: dayjs.Dayjs | null = null;
  let storePlanId: string | null = null;

  if (binding.store === InAppPurchaseStore.APP_STORE) {
    const status = await appstore.getSubscriptionStatus(binding.identifier);
    if (status.kind === 'error') {
      // 전 환경 조회 실패. 조용히 스킵하면 재시도·알림이 없으므로 던져서 큐 재시도와 Sentry 캡처를 발동시킨다.
      throw new Error('appstore reconcile status lookup failed');
    }

    if (status.kind === 'active' || status.kind === 'grace') {
      storeState = status.kind;
      storeExpiresAt = status.expiresDate ? dayjs(status.expiresDate) : null;
      if (status.kind === 'active') {
        storePlanId = status.productId?.toUpperCase() ?? null;
      }
    } else if (status.kind === 'suspended' || status.kind === 'expired' || status.kind === 'revoked') {
      storeState = status.kind;
    }
    // status.kind === 'unknown' 은 storeState = 'unknown' 유지(안전 스킵)
  } else if (binding.store === InAppPurchaseStore.GOOGLE_PLAY) {
    // getSubscription 은 조회 실패 시 throw 한다. 삼키지 않고 전파해 큐 재시도·Sentry 를 발동시킨다.
    // 단 404/410(만료 60일 경과 등으로 토큰 제공 영구 중단)은 재시도가 무의미한 확정 응답이므로,
    // 로컬 만료가 이미 지난 행에 한해 스토어 확인 만료로 취급한다 — 영구 재시도 루프로 stale ACTIVE 가 방치되는 것을 막는다.
    const googlePlaySubscription = await googleplay.getSubscription(binding.identifier).catch((err: unknown) => {
      if (googleplay.isPurchaseTokenGoneError(err) && dayjs(subscription.expiresAt).isBefore(dayjs())) {
        return null;
      }
      throw err;
    });

    if (googlePlaySubscription === null) {
      storeState = 'expired';
    } else {
      const expiryTime = googlePlaySubscription.lineItems?.[0]?.expiryTime;
      const googlePlayState = googlePlaySubscription.subscriptionState;
      if (googlePlayState === 'SUBSCRIPTION_STATE_ACTIVE') {
        storeState = 'active';
        storeExpiresAt = expiryTime ? dayjs(expiryTime) : null;
        storePlanId = googlePlaySubscription.lineItems?.[0]?.offerDetails?.basePlanId?.toUpperCase() ?? null;
      } else if (googlePlayState === 'SUBSCRIPTION_STATE_IN_GRACE_PERIOD') {
        storeState = 'grace';
        storeExpiresAt = expiryTime ? dayjs(expiryTime) : null;
      } else if (googlePlayState === 'SUBSCRIPTION_STATE_ON_HOLD' || googlePlayState === 'SUBSCRIPTION_STATE_PAUSED') {
        storeState = 'suspended';
      } else if (googlePlayState === 'SUBSCRIPTION_STATE_EXPIRED') {
        storeState = 'expired';
      }
      // 그 외(CANCELED/PENDING 등) 는 storeState = 'unknown' 유지(안전 스킵)
    }
  }

  if (storeState === 'unknown') {
    return;
  }

  const now = dayjs();

  // 스토어 조회 이후 웹훅이 상태를 바꿨을 수 있으므로, 트랜잭션 안에서 행을 잠그고 재조회한 뒤 판정한다.
  await db.transaction(async (tx) => {
    await lockUserSubscriptionState(tx, subscription.userId);

    // 조회 이후 바인딩이 다른 구매로 교체됐다면(재구독 등) 이 스토어 결과는 stale 이므로 중단한다.
    const freshBinding = await tx
      .select({ store: UserInAppPurchases.store, identifier: UserInAppPurchases.identifier })
      .from(UserInAppPurchases)
      .where(eq(UserInAppPurchases.userId, subscription.userId))
      .for('no key update')
      .then(first);

    if (!freshBinding || freshBinding.store !== binding.store || freshBinding.identifier !== binding.identifier) {
      return;
    }

    const fresh = await tx
      .select({ state: Subscriptions.state, expiresAt: Subscriptions.expiresAt })
      .from(Subscriptions)
      .where(eq(Subscriptions.id, subscription.id))
      .for('no key update', { of: Subscriptions })
      .then(firstOrThrow);

    // 스토어 조회 시점 이후 웹훅이 상태/만료일을 바꿨다면(예: 갱신이 사이에 도착) 스토어 응답이 이미 stale 이다.
    // 이 경우 판정하지 않고 중단한다 — 다음 재조정이 갱신된 상태에 맞춰 스토어를 다시 조회한다.
    if (fresh.state !== subscription.state || !dayjs(fresh.expiresAt).isSame(dayjs(subscription.expiresAt))) {
      return;
    }

    const action = resolveReconcileAction({
      storeState,
      storeExpiresAt,
      currentState: fresh.state,
      currentExpiresAt: dayjs(fresh.expiresAt),
      now,
    });

    if (action.type === 'recover') {
      // 유실된 웹훅에 플랜 변경이 포함됐을 수 있으므로, 스토어가 알려준 플랜이 유효한 IAP 플랜이면 함께 동기화한다.
      const storePlan = storePlanId
        ? await tx
            .select({ id: Plans.id })
            .from(Plans)
            .where(and(eq(Plans.id, storePlanId), eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)))
            .then(first)
        : null;

      await tx
        .update(Subscriptions)
        .set({
          state: SubscriptionState.ACTIVE,
          renewedAt: fresh.expiresAt,
          expiresAt: action.expiresAt,
          ...(storePlan && { planId: storePlan.id }),
        })
        .where(eq(Subscriptions.id, subscription.id));
    } else if (action.type === 'grace') {
      await tx.update(Subscriptions).set({ state: SubscriptionState.IN_GRACE_PERIOD }).where(eq(Subscriptions.id, subscription.id));
    } else if (action.type === 'suspend') {
      await tx.update(Subscriptions).set({ state: SubscriptionState.WILL_EXPIRE }).where(eq(Subscriptions.id, subscription.id));
    } else if (action.type === 'expire') {
      await tx
        .update(Subscriptions)
        .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
        .where(eq(Subscriptions.id, subscription.id));
    }
  });
});
