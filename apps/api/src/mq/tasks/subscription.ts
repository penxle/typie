import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { PaymentInvoiceState, PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, desc, eq, gt, inArray, isNull, lte, ne, or } from 'drizzle-orm';
import { db, first, firstOrThrow, PaymentInvoices, Plans, Subscriptions, UserBillingKeys, UserInAppPurchases } from '#/db/index.ts';
import * as portone from '#/external/portone.ts';
import { computeNextPeriodEnd } from '#/utils/billing-period.ts';
import { deriveGraceDeadline } from '#/utils/entitlement.ts';
import { ingestIapPayments } from '#/utils/iap-ingest.ts';
import { syncIapBinding } from '#/utils/iap-sync.ts';
import { attemptInvoicePayment, enrichPaymentRecordReceipt, hasBillableUsageDuring } from '#/utils/index.ts';
import { opsAlertOnce } from '#/utils/ops-alert.ts';
import { derivePaymentKey } from '#/utils/payment-key.ts';
import { hasFutureBillingObligation } from '#/utils/plan.ts';
import { lockUserSubscriptionState } from '#/utils/subscription-lock.ts';
import { retireReservation } from '#/utils/subscription-retire.ts';
import { enqueueJob } from '../index.ts';
import { defineCron, defineJob } from '../types.ts';

const log = logger.getChild('subscription');

// 트랜잭션 없이 조회한다 — 잡이 상태를 재검증하므로 스냅샷이 불필요하고, Redis enqueue 를 DB 트랜잭션
// 안에서 하면 지연 시 커넥션을 붙든다.
export const SubscriptionBillingScanCron = defineCron('subscription:billing-scan', '* 10-21 * * *', async () => {
  const now = dayjs();

  const targets = await db
    .select({ id: Subscriptions.id, currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(Subscriptions.state, SubscriptionState.ACTIVE),
        lte(Subscriptions.currentPeriodEndsAt, now),
        eq(Plans.availability, PlanAvailability.BILLING_KEY),
      ),
    );

  for (const subscription of targets) {
    // 결정적 jobId — 매분 스캔이 처리 지연 중인 같은 구독·같은 주기를 중복 적재하지 않는다.
    await enqueueJob('subscription:renewal:initial', subscription.id, {
      jobId: `renewal-initial-${subscription.id}-${subscription.currentPeriodEndsAt.valueOf()}`,
    });
  }
});

// 재시도 전용 크론이다. 페이스는 인보이스당 하루 1회로 유지하되(카드 거절 반복 재청구 금지), 시각은 고정 10시
// 일괄 대신 유저별 위상(마지막 처리 + 24시간)으로 주간 창 안에 분산한다 — 일괄은 버스트를 만들고, 유예 마감
// (주기 종료 시각 + 7일)과 위상이 어긋나 유저별 유효 시도 횟수가 들쭉했다. 페이싱 신호는 PaymentRecords 가
// 아니라 lastAttemptedAt 스탬프다 — 기록은 승인 증거라 PG 미호출·비확정 경로(빌링키 결손, AlreadyPaid 회수
// 비확정)에 남지 않아, 존재 검사로 페이스를 재면 그 경로들이 분 단위 재처리 루프가 된다. 스탬프는 청구
// 트랜잭션 안에서 커밋되므로(attemptInvoicePayment 진입부) 처리 직후 같은 인보이스는 즉시 부적격이 되고,
// jobId 디듀프가 잡이 큐·실행 중인 동안의 재적재를 막는다.
export const SubscriptionRenewalCron = defineCron('subscription:renewal', '* 10-21 * * *', async () => {
  const now = dayjs();

  // 백필 재정렬로 서비스 시작이 미래가 된 인보이스는 아직 재시도 대상이 아니다.
  const overdueInvoices = await db
    .select({ id: PaymentInvoices.id })
    .from(PaymentInvoices)
    .where(
      and(
        eq(PaymentInvoices.state, PaymentInvoiceState.OVERDUE),
        lte(PaymentInvoices.servicePeriodStartsAt, now),
        or(isNull(PaymentInvoices.lastAttemptedAt), lte(PaymentInvoices.lastAttemptedAt, now.subtract(24, 'hours'))),
      ),
    );

  for (const invoice of overdueInvoices) {
    await enqueueJob('subscription:renewal:retry', invoice.id, { jobId: `renewal-retry-${invoice.id}` });
  }
});

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
        lte(Subscriptions.currentPeriodEndsAt, now),
        ne(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
      ),
    );

  for (const subscription of cancelSubscriptions) {
    await enqueueJob('subscription:renewal:cancel', subscription.id, { jobId: `renewal-cancel-${subscription.id}` });
  }

  // 유예 소진 종결은 재청구 크론(일 1회)에서 분리해 다른 시각 전이와 같은 주기로 수렴시킨다 — 권한식은 마감
  // 즉시 꺼지는데 상태가 다음 재청구까지 남으면 그 창 동안 표시·불변식이 사실과 어긋난다(게이트는 liveness 가 막는다).
  // 마감 판정·전이는 재시도 잡이 락 안에서 다시 수행한다(마감 경과 시 PG 호출 없이 종결) — 여기서는 후보만 고른다.
  // 마감식은 deriveGraceDeadline 단일 소스라, 여기서 경과로 본 행을 잡이 재청구로 판정하는 역전은 없다.
  const graceInvoices = await db
    .select({
      id: PaymentInvoices.id,
      state: Subscriptions.state,
      planAvailability: Plans.availability,
      startsAt: Subscriptions.startsAt,
      currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
      currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
    })
    .from(PaymentInvoices)
    .innerJoin(Subscriptions, eq(PaymentInvoices.subscriptionId, Subscriptions.id))
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(PaymentInvoices.state, PaymentInvoiceState.OVERDUE),
        eq(Subscriptions.state, SubscriptionState.IN_GRACE_PERIOD),
        ne(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
      ),
    );

  for (const invoice of graceInvoices) {
    if (deriveGraceDeadline(invoice, now).isAfter(now)) {
      continue;
    }

    await enqueueJob('subscription:renewal:retry', invoice.id, { jobId: `grace-expire-${invoice.id}` });
  }
});

export const SubscriptionRenewalInitialJob = defineJob('subscription:renewal:initial', async (subscriptionId: string) => {
  const paidInvoiceId = await db.transaction(async (tx) => {
    // userId 는 불변 컬럼이라 무락 조회가 안전하다 — advisory 를 행 잠금보다 먼저 잡기 위한 사전 조회.
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
        id: Subscriptions.id,
        userId: Subscriptions.userId,
        state: Subscriptions.state,
        currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
        currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
        billingAnchorAt: Subscriptions.billingAnchorAt,
        plan: { fee: Plans.fee, interval: Plans.interval, availability: Plans.availability },
      })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(Subscriptions.id, subscriptionId))
      .for('no key update', { of: Subscriptions })
      .then(first);

    // availability 재검증은 크론 재시도·구버전 잡이 IAP·트라이얼 구독을 빌링키 청구로 끌고 오는 것을 막는다.
    if (
      !subscription ||
      subscription.state !== SubscriptionState.ACTIVE ||
      subscription.plan.availability !== PlanAvailability.BILLING_KEY ||
      subscription.currentPeriodEndsAt.isAfter(dayjs())
    ) {
      return null;
    }

    if (!subscription.billingAnchorAt) {
      // 빌링키 구독의 앵커 부재는 주기를 계산할 수 없다는 뜻이다 — 임의 앵커로 청구하느니 사람이 본다.
      await opsAlertOnce('invariant-violation', subscriptionId, {
        reason: 'billing key subscription without billing anchor',
        subscriptionId,
      });

      return null;
    }

    const servicePeriodStartsAt = subscription.currentPeriodEndsAt;
    const servicePeriodEndsAt = computeNextPeriodEnd({
      periodStartsAt: servicePeriodStartsAt,
      interval: subscription.plan.interval,
      billingAnchorAt: subscription.billingAnchorAt,
    });

    const hasUsage = await hasBillableUsageDuring(
      tx,
      subscription.userId,
      subscription.currentPeriodStartsAt,
      subscription.currentPeriodEndsAt,
    );

    if (!hasUsage) {
      // 미사용 면제. 무청구라 finalizePaymentSuccess 를 타지 않으므로 주기 전진은 여기가 유일한 지점이다.
      const waivedInvoice = await tx
        .insert(PaymentInvoices)
        .values({
          userId: subscription.userId,
          subscriptionId: subscription.id,
          amount: 0,
          state: PaymentInvoiceState.WAIVED,
          dueAt: servicePeriodStartsAt,
          paymentKey: derivePaymentKey(subscription.id, servicePeriodStartsAt),
          servicePeriodStartsAt,
          servicePeriodEndsAt,
        })
        .returning({ id: PaymentInvoices.id })
        .then(firstOrThrow);

      await tx
        .update(Subscriptions)
        .set({ currentPeriodStartsAt: servicePeriodStartsAt, currentPeriodEndsAt: servicePeriodEndsAt })
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

      return null;
    }

    const invoice = await tx
      .insert(PaymentInvoices)
      .values({
        userId: subscription.userId,
        subscriptionId: subscription.id,
        amount: subscription.plan.fee,
        state: PaymentInvoiceState.UPCOMING,
        dueAt: servicePeriodStartsAt,
        paymentKey: derivePaymentKey(subscription.id, servicePeriodStartsAt),
        servicePeriodStartsAt,
        servicePeriodEndsAt,
      })
      .returning({ id: PaymentInvoices.id })
      .then(firstOrThrow);

    const outcome = await attemptInvoicePayment(tx, invoice.id);

    // 성공 종결(인보이스 PAID·주기 설정·ACTIVE)은 finalizePaymentSuccess 가 이미 끝냈다.
    if (outcome.kind === 'paid') {
      return invoice.id;
    }

    // not-paid 에는 "PAID CAS 0행(다른 경로가 이미 종결)"과 진짜 실패가 접혀 있다. UPCOMING CAS 로 갈라야
    // 확정된 결제를 OVERDUE 로 되돌리지 않고, 구독의 유예 전이도 그 결과와 어긋나지 않는다.
    const overdue = await tx
      .update(PaymentInvoices)
      .set({ state: PaymentInvoiceState.OVERDUE })
      .where(and(eq(PaymentInvoices.id, invoice.id), eq(PaymentInvoices.state, PaymentInvoiceState.UPCOMING)))
      .returning({ id: PaymentInvoices.id })
      .then(first);

    if (!overdue) {
      return null;
    }

    await tx.update(Subscriptions).set({ state: SubscriptionState.IN_GRACE_PERIOD }).where(eq(Subscriptions.id, subscriptionId));

    await enqueueJob('email:subscription-grace-period', subscription.id, { delay: 5 * 60 * 1000 });

    return null;
  });

  // 영수증 보강은 커밋 이후다 — 조회 실패가 성공 확정을 뒤집지 않고, 승인~커밋 창을 늘리지도 않는다.
  if (paidInvoiceId) {
    await enrichPaymentRecordReceipt(paidInvoiceId);
  }
});

export const SubscriptionRenewalRetryJob = defineJob('subscription:renewal:retry', async (invoiceId: string) => {
  const paidInvoiceId = await db.transaction(async (tx) => {
    // 교착 방지: 모든 갱신·환불 경로는 구독 → 인보이스 순으로 잠근다(환불은 구독을 잠근 채 인보이스를 갱신한다).
    // subscriptionId/userId 는 불변 컬럼이라 무락 조회가 안전하고, 상태는 아래 잠금 조회에서 재검증한다.
    const invoiceRef = await tx
      .select({ subscriptionId: PaymentInvoices.subscriptionId, userId: PaymentInvoices.userId })
      .from(PaymentInvoices)
      .where(eq(PaymentInvoices.id, invoiceId))
      .then(first);

    if (!invoiceRef) {
      return null;
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
      return null;
    }

    const invoice = await tx
      .select({
        id: PaymentInvoices.id,
        state: PaymentInvoices.state,
        subscription: {
          id: Subscriptions.id,
          userId: Subscriptions.userId,
          state: Subscriptions.state,
          planAvailability: Plans.availability,
          startsAt: Subscriptions.startsAt,
          currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
          currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
          createdAt: Subscriptions.createdAt,
        },
      })
      .from(PaymentInvoices)
      .innerJoin(Subscriptions, eq(PaymentInvoices.subscriptionId, Subscriptions.id))
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(PaymentInvoices.id, invoiceId))
      .for('no key update', { of: PaymentInvoices })
      .then(first);

    if (!invoice || invoice.state !== PaymentInvoiceState.OVERDUE || invoice.subscription.state !== SubscriptionState.IN_GRACE_PERIOD) {
      return null;
    }

    const now = dayjs();
    const graceDeadline = deriveGraceDeadline(invoice.subscription, now);

    // 등호 포함 — 권한식(마감 > now)의 여집합이다. 마감이 지났으면 PG 를 호출하지 않고 종결한다.
    if (!graceDeadline.isAfter(now)) {
      await tx.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(eq(Subscriptions.id, invoice.subscription.id));
      await tx
        .update(PaymentInvoices)
        .set({ state: PaymentInvoiceState.CANCELED })
        .where(
          and(
            eq(PaymentInvoices.id, invoice.id),
            inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE]),
          ),
        );

      await enqueueJob('email:subscription-expired', invoice.subscription.id, { delay: 5 * 60 * 1000 });

      return null;
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
            and(eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE), gt(Subscriptions.currentPeriodEndsAt, dayjs())),
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
      await tx.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(eq(Subscriptions.id, invoice.subscription.id));
      await tx
        .update(PaymentInvoices)
        .set({ state: PaymentInvoiceState.CANCELED })
        .where(
          and(
            eq(PaymentInvoices.id, invoice.id),
            inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE]),
          ),
        );

      return null;
    }

    // 슬롯 선점: ACTIVE 전이를 결제보다 먼저 둔다 — 동시 다른 채널의 ACTIVE 와의 유니크 충돌이
    // PG 호출 전에 발생해, 승인 후 롤백(재과금) 경로가 구조적으로 사라진다.
    await tx.update(Subscriptions).set({ state: SubscriptionState.ACTIVE }).where(eq(Subscriptions.id, invoice.subscription.id));

    // 저장된 paymentKey·서비스 주기로 재시도한다 — 재청구 판정(사용량·주기 계산)은 인보이스 생성 시 이미 끝났다.
    const outcome = await attemptInvoicePayment(tx, invoice.id);

    // 성공 종결(인보이스 PAID·주기 설정·ACTIVE)은 finalizePaymentSuccess 가 이미 끝냈다.
    if (outcome.kind === 'paid') {
      return invoice.id;
    }

    // 선점한 슬롯을 규칙대로 유예로 되돌린다 — 우리가 세운 ACTIVE 일 때만 전이해, 다른 경로가 소유한 상태를 덮지 않는다.
    await tx
      .update(Subscriptions)
      .set({ state: SubscriptionState.IN_GRACE_PERIOD })
      .where(and(eq(Subscriptions.id, invoice.subscription.id), eq(Subscriptions.state, SubscriptionState.ACTIVE)));

    if (graceDeadline.kst().subtract(1, 'day').isSame(dayjs.kst(), 'day')) {
      await enqueueJob('email:subscription-expiring', invoice.subscription.id, { delay: 5 * 60 * 1000 });
    }

    return null;
  });

  // 영수증 보강은 커밋 이후다 — 조회 실패가 성공 확정을 뒤집지 않고, 승인~커밋 창을 늘리지도 않는다.
  if (paidInvoiceId) {
    await enrichPaymentRecordReceipt(paidInvoiceId);
  }
});

export const SubscriptionRenewalPlanChangeJob = defineJob('subscription:renewal:plan-change', async (subscriptionId: string) => {
  const paidInvoiceId = await db.transaction(async (tx) => {
    // userId 는 불변 컬럼이라 무락 조회가 안전하다 — advisory 를 행 잠금보다 먼저 잡기 위한 사전 조회.
    const subscriptionRef = await tx
      .select({ userId: Subscriptions.userId })
      .from(Subscriptions)
      .where(eq(Subscriptions.id, subscriptionId))
      .then(first);

    if (!subscriptionRef) {
      return null;
    }

    await lockUserSubscriptionState(tx, subscriptionRef.userId);

    // 락 대기 중 취소가 예약을 지웠으면 조용한 no-op — throw 는 불필요한 큐 재시도·Sentry 노이즈다.
    const subscription = await tx
      .select({
        id: Subscriptions.id,
        userId: Subscriptions.userId,
        state: Subscriptions.state,
        startsAt: Subscriptions.startsAt,
        currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
        currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
        plan: { fee: Plans.fee },
      })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(Subscriptions.id, subscriptionId))
      .for('no key update', { of: Subscriptions })
      .then(first);

    if (!subscription || subscription.state !== SubscriptionState.WILL_ACTIVATE || dayjs(subscription.startsAt).isAfter(dayjs())) {
      return null;
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
            and(eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE), gt(Subscriptions.currentPeriodEndsAt, dayjs())),
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
      await retireReservation(tx, { userId: subscription.userId, subscriptionId });
      return null;
    }

    const openInvoice = await tx
      .select({ id: PaymentInvoices.id })
      .from(PaymentInvoices)
      .where(
        and(
          eq(PaymentInvoices.subscriptionId, subscriptionId),
          inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE]),
        ),
      )
      .then(first);

    let invoiceId = openInvoice?.id;
    if (!invoiceId) {
      // predecessor 결정은 최초 실행 1회뿐이다. 재시도는 저장된 payment_key 를 쓴다 — 그 시점의 predecessor 는
      // EXPIRED 로 종결된 것이 정상이라 predicate 를 다시 태우면 일일 재시도가 한 번도 실행되지 못한다.
      // 같은 유저 ∧ 빌링키·트라이얼 채널 ∧ 주기 종료 = 예약 시작, 상태는 무관하다.
      const predecessors = await tx
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
        .where(
          and(
            eq(Subscriptions.userId, subscription.userId),
            ne(Subscriptions.id, subscriptionId),
            inArray(Plans.availability, [PlanAvailability.BILLING_KEY, PlanAvailability.TRIAL]),
            eq(Subscriptions.currentPeriodEndsAt, subscription.startsAt),
          ),
        );

      if (predecessors.length !== 1) {
        // 임의 선택은 같은 계약의 다른 키 재승인 위험이다 — PG 를 호출하지 않고 사람에게 넘긴다.
        await opsAlertOnce('plan-change-predecessor-ambiguous', subscriptionId, { subscriptionId, candidates: predecessors.length });

        return null;
      }

      // 예약(WILL_ACTIVATE)은 취소로 물리 삭제될 수 있어 키 lineage 는 tombstone 으로 보존되는 predecessor 의 ID 다.
      const invoice = await tx
        .insert(PaymentInvoices)
        .values({
          userId: subscription.userId,
          subscriptionId: subscription.id,
          amount: subscription.plan.fee,
          state: PaymentInvoiceState.UPCOMING,
          dueAt: subscription.startsAt,
          paymentKey: derivePaymentKey(predecessors[0].id, subscription.currentPeriodStartsAt),
          servicePeriodStartsAt: subscription.currentPeriodStartsAt,
          servicePeriodEndsAt: subscription.currentPeriodEndsAt,
        })
        .returning({ id: PaymentInvoices.id })
        .then(firstOrThrow);

      invoiceId = invoice.id;
    }

    // 슬롯 선점: ACTIVE 전이를 결제보다 먼저 둔다 — 동시 다른 채널의 ACTIVE 와의 유니크 충돌이
    // PG 호출 전에 발생해, 승인 후 롤백(재과금) 경로가 구조적으로 사라진다. 실패 시 아래에서 유예로 전이한다.
    await tx.update(Subscriptions).set({ state: SubscriptionState.ACTIVE }).where(eq(Subscriptions.id, subscriptionId));

    const outcome = await attemptInvoicePayment(tx, invoiceId);

    // 성공 종결(인보이스 PAID·주기 설정·ACTIVE)은 finalizePaymentSuccess 가 이미 끝냈다.
    if (outcome.kind === 'paid') {
      return invoiceId;
    }

    // not-paid 에는 "PAID CAS 0행(다른 경로가 이미 종결)"과 진짜 실패가 접혀 있다. UPCOMING CAS 로 갈라야
    // 확정된 결제를 OVERDUE 로 되돌리지 않고, 선점한 슬롯도 그 결과와 어긋나지 않는다.
    const overdue = await tx
      .update(PaymentInvoices)
      .set({ state: PaymentInvoiceState.OVERDUE })
      .where(and(eq(PaymentInvoices.id, invoiceId), eq(PaymentInvoices.state, PaymentInvoiceState.UPCOMING)))
      .returning({ id: PaymentInvoices.id })
      .then(first);

    if (!overdue) {
      return null;
    }

    // 주기는 선설정된 목표 주기 그대로 둔다 — 유예 마감이 미결제 주기 시작에서 파생되는 근거다.
    await tx.update(Subscriptions).set({ state: SubscriptionState.IN_GRACE_PERIOD }).where(eq(Subscriptions.id, subscriptionId));

    await enqueueJob('email:subscription-grace-period', subscription.id, { delay: 5 * 60 * 1000 });

    return null;
  });

  // 영수증 보강은 커밋 이후다 — 조회 실패가 성공 확정을 뒤집지 않고, 승인~커밋 창을 늘리지도 않는다.
  if (paidInvoiceId) {
    await enrichPaymentRecordReceipt(paidInvoiceId);
  }
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
        currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
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
      subscription.currentPeriodEndsAt.isAfter(dayjs())
    ) {
      return null;
    }

    // 청구 대상에서 이탈시키는 전이라 열린 인보이스를 같은 트랜잭션에서 먼저 거둔다 — 남기면 재시도 잡이 집지 못하는
    // 인보이스가 미래 청구 의무로 남아 빌링키를 영구히 붙잡는다.
    await tx
      .update(PaymentInvoices)
      .set({ state: PaymentInvoiceState.CANCELED })
      .where(
        and(
          eq(PaymentInvoices.subscriptionId, subscriptionId),
          inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE]),
        ),
      );

    // 빌링키 취소 확정 또는 트라이얼 만료 — 둘 다 EXPIRED 로 전이한다. 주기 컬럼은 자르지 않는다(회수는 상태가 표현한다).
    await tx.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(eq(Subscriptions.id, subscriptionId));

    // 빌링키 정리는 BILLING_KEY 플랜에만 해당한다(트라이얼은 빌링키가 없음).
    if (subscription.availability !== PlanAvailability.BILLING_KEY) {
      return null;
    }

    if (await hasFutureBillingObligation(tx, subscription.userId)) {
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

export const IapSyncJob = defineJob('iap:sync', async (payload: { bindingId: string; cycle?: string }) => {
  const outcome = await syncIapBinding({ bindingId: payload.bindingId });

  // 판정 보류는 재시도가 백스톱이다 — 조용히 완료하면 확정이 일일 재조정까지 밀린다.
  if (outcome.kind === 'deferred') {
    throw new Error(`iap sync deferred: ${payload.bindingId}`);
  }

  await enqueueJob('iap:ingest', { bindingId: payload.bindingId });
});

export const SubscriptionReconcileInAppPurchaseCron = defineCron('subscription:reconcile-iap', '0 4 * * *', async () => {
  // canonical FK 조인 — subscriptionId 가 없는 마커(gone) 행은 자동 제외된다. 상태 필터는 없다: EXPIRED 도
  // 포함해야 환불 철회 복권 백스톱이 성립한다. reconcileSuspendedAt 만 걸러 재조정 비활성 바인딩을 뺀다.
  const bindings = await db
    .select({ id: UserInAppPurchases.id })
    .from(UserInAppPurchases)
    .innerJoin(Subscriptions, eq(UserInAppPurchases.subscriptionId, Subscriptions.id))
    .where(isNull(UserInAppPurchases.reconcileSuspendedAt));

  const cycle = dayjs().toISOString();
  log.info('reconcile-iap cycle {*}', { cycle, targets: bindings.length });

  for (const binding of bindings) {
    await enqueueJob('iap:sync', { bindingId: binding.id, cycle });
  }
});

// 대기 중 잡 호환을 위해 잡 이름은 유지한다 — 본체는 iap:sync 로 옮겼고, 이 잡은 구독 ID 로부터 바인딩을
// 역조회해 위임하기만 한다.
export const SubscriptionReconcileInAppPurchaseJob = defineJob('subscription:reconcile-iap:sync', async (subscriptionId: string) => {
  const binding = await db
    .select({ id: UserInAppPurchases.id })
    .from(UserInAppPurchases)
    .where(eq(UserInAppPurchases.subscriptionId, subscriptionId))
    .then(first);

  if (!binding) {
    return;
  }

  const outcome = await syncIapBinding({ bindingId: binding.id });

  // 판정 보류는 재시도가 백스톱이다 — 조용히 완료하면 확정이 일일 재조정까지 밀린다.
  if (outcome.kind === 'deferred') {
    throw new Error(`iap sync deferred: ${binding.id}`);
  }

  await enqueueJob('iap:ingest', { bindingId: binding.id });
});

// 결제 기록 적재는 상태 동기화와 분리된 잡이다 — 스토어 이력 재수집이라 멱등이고, 실패해도
// 다음 sync 연쇄·일일 재조정이 재수집한다. 스토어 조회 실패는 collect 가 throw 해 BullMQ 재시도를 탄다.
export const IapIngestJob = defineJob('iap:ingest', async (payload: { bindingId: string }) => {
  await ingestIapPayments({ bindingId: payload.bindingId });
});
