import * as Sentry from '@sentry/node';
import { defaultPlanRules, TRIAL_DURATION_DAYS } from '@typie/lib/const';
import {
  BillingKeyType,
  CreditCodeState,
  InAppPurchaseStore,
  PaymentInvoiceState,
  PaymentOutcome,
  PlanAvailability,
  PlanInterval,
  SubscriptionState,
  UserState,
} from '@typie/lib/enums';
import { NotFoundError, TypieError } from '@typie/lib/errors';
import { supportsPlanInterval } from '@typie/lib/plan';
import { cardSchema, redeemCodeSchema } from '@typie/lib/validation';
import dayjs from 'dayjs';
import { and, desc, eq, gt, inArray, ne, notInArray } from 'drizzle-orm';
import * as uuid from 'uuid';
import {
  CreditCodes,
  db,
  first,
  firstOrThrow,
  firstOrThrowWith,
  PaymentInvoices,
  PaymentRecords,
  Plans,
  Subscriptions,
  TableCode,
  UserBillingKeys,
  UserInAppPurchases,
  Users,
  UserTrials,
  validateDbId,
} from '#/db/index.ts';
import { env } from '#/env.ts';
import * as googleplay from '#/external/googleplay.ts';
import * as portone from '#/external/portone.ts';
import { enqueueJob } from '#/mq/index.ts';
import { verifyEasyPayBillingKey } from '#/utils/billing-key.ts';
import { computeNextPeriodEnd, floorToHourKst } from '#/utils/billing-period.ts';
import { deriveExpiresAtShim, isSubscriptionLive } from '#/utils/entitlement.ts';
import { fetchIapEnrollment, normalizeIapEnrollment, probeIapBoundContractTermination } from '#/utils/iap-enroll.ts';
import { precheckIapEnroll } from '#/utils/iap-normalize.ts';
import { applyNormalizedIapLocked } from '#/utils/iap-sync.ts';
import { attemptInvoicePayment, enrichPaymentRecordReceipt, hasBillableUsageDuring } from '#/utils/index.ts';
import { opsAlert, opsAlertOnce } from '#/utils/ops-alert.ts';
import { derivePaymentKey } from '#/utils/payment-key.ts';
import { createTrialSubscription, hasFutureBillingObligation } from '#/utils/plan.ts';
import { delay } from '#/utils/promise.ts';
import { hasLiveYearlyBillingKeySubscription, replaceUserBillingKey } from '#/utils/subscription-billing-key.ts';
import { resolveEnrollAction } from '#/utils/subscription-enroll.ts';
import { lockUserSubscriptionState } from '#/utils/subscription-lock.ts';
import { retireReservation } from '#/utils/subscription-retire.ts';
import { builder } from '../builder.ts';
import {
  CreditCode,
  isTypeOf,
  PaymentInvoice,
  PaymentRecord,
  Plan,
  PlanRule,
  Subscription,
  User,
  UserBillingKey,
  UserTrial,
} from '../objects.ts';
import type { Database, Transaction } from '#/db/index.ts';
import type { IapEnrollOwnership, IapEnrollRejection } from '#/utils/iap-enroll.ts';

/**
 * * Types
 */

CreditCode.implement({
  isTypeOf: isTypeOf(TableCode.CREDIT_CODES),
  fields: (t) => ({
    id: t.exposeID('id'),
    code: t.exposeString('code'),
    amount: t.exposeInt('amount'),
  }),
});

PaymentInvoice.implement({
  isTypeOf: isTypeOf(TableCode.PAYMENT_INVOICES),
  fields: (t) => ({
    id: t.exposeID('id'),
    state: t.expose('state', { type: PaymentInvoiceState }),
    amount: t.exposeInt('amount'),
    dueAt: t.expose('dueAt', { type: 'DateTime' }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    subscription: t.field({
      type: Subscription,
      resolve: (self) => self.subscriptionId,
    }),
    records: t.field({
      type: [PaymentRecord],
      resolve: async (self) => {
        return await db.select().from(PaymentRecords).where(eq(PaymentRecords.invoiceId, self.id));
      },
    }),
  }),
});

PaymentRecord.implement({
  isTypeOf: isTypeOf(TableCode.PAYMENT_RECORDS),
  fields: (t) => ({
    id: t.exposeID('id'),
    outcome: t.expose('outcome', { type: PaymentOutcome }),
    billingAmount: t.exposeInt('billingAmount'),
    creditAmount: t.exposeInt('creditAmount'),
    data: t.expose('data', { type: 'JSON' }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

Plan.implement({
  isTypeOf: isTypeOf(TableCode.PLANS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    fee: t.exposeInt('fee'),
    interval: t.expose('interval', { type: PlanInterval }),
    availability: t.expose('availability', { type: PlanAvailability }),
    rule: t.expose('rule', { type: PlanRule }),
  }),
});

PlanRule.implement({
  fields: (t) => ({
    maxTotalCharacterCount: t.int({ resolve: (self) => self.maxTotalCharacterCount ?? defaultPlanRules.maxTotalCharacterCount }),
    maxTotalBlobSize: t.int({ resolve: (self) => self.maxTotalBlobSize ?? defaultPlanRules.maxTotalBlobSize }),
  }),
});

Subscription.implement({
  isTypeOf: isTypeOf(TableCode.SUBSCRIPTIONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    plan: t.expose('planId', { type: Plan }),
    startsAt: t.expose('startsAt', { type: 'DateTime' }),
    currentPeriodEndsAt: t.expose('currentPeriodEndsAt', { type: 'DateTime' }),

    // 구버전 앱 호환 전용 파생값이다 — 저장하면 상태를 바꾸는 모든 경로가 함께 갱신해야 한다.
    expiresAt: t.field({
      type: 'DateTime',
      resolve: async (self, _args, ctx) => {
        const plan = await Plan.getDataloader(ctx).load(self.planId);

        return deriveExpiresAtShim({ ...self, planAvailability: plan.availability }, (ctx.entitlementNow ??= dayjs()));
      },
    }),

    state: t.expose('state', { type: SubscriptionState }),
    user: t.expose('userId', { type: User }),
    hasBillableUsage: t.boolean({
      resolve: async (self) => {
        return await hasBillableUsageDuring(db, self.userId, self.currentPeriodStartsAt, self.currentPeriodEndsAt);
      },
    }),
  }),
});

UserTrial.implement({
  isTypeOf: isTypeOf(TableCode.USER_TRIALS),
  fields: (t) => ({
    id: t.exposeID('id'),
    startedAt: t.expose('startedAt', { type: 'DateTime' }),
    expiresAt: t.expose('expiresAt', { type: 'DateTime' }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  defaultPlanRule: t.field({
    type: PlanRule,
    resolve: async () => {
      return defaultPlanRules;
    },
  }),

  creditCode: t.withAuth({ session: true }).field({
    type: CreditCode,
    args: { code: t.input.string({ validate: { schema: redeemCodeSchema } }) },
    resolve: async (_, args) => {
      const code = args.code.toUpperCase().replaceAll('-', '').replaceAll('O', '0').replaceAll(/[IL]/g, '1');

      await delay(Math.random() * 1000);

      return await db
        .select()
        .from(CreditCodes)
        .where(and(eq(CreditCodes.code, code), eq(CreditCodes.state, CreditCodeState.AVAILABLE), gt(CreditCodes.expiresAt, dayjs())))
        .then(firstOrThrowWith(new NotFoundError()));
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  subscribePlanWithTrial: t.withAuth({ session: true }).field({
    type: Subscription,
    resolve: async (_, __, ctx) => {
      const startsAt = dayjs();
      const expiresAt = startsAt.add(TRIAL_DURATION_DAYS, 'days');

      return await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        const subscriptionHistory = await tx
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .where(eq(Subscriptions.userId, ctx.session.userId))
          .then(first);

        if (subscriptionHistory) {
          throw new TypieError({ code: 'subscription_history_exists' });
        }

        const existingTrial = await tx
          .select({ id: UserTrials.id })
          .from(UserTrials)
          .where(eq(UserTrials.userId, ctx.session.userId))
          .then(first);

        if (existingTrial) {
          throw new TypieError({ code: 'trial_already_used' });
        }

        return await createTrialSubscription(tx, { userId: ctx.session.userId, startsAt, expiresAt });
      });
    },
  }),

  updateBillingKey: t.withAuth({ session: true }).fieldWithInput({
    type: UserBillingKey,
    input: {
      cardNumber: t.input.string({ validate: { schema: cardSchema.cardNumber } }),
      expiryDate: t.input.string({ validate: { schema: cardSchema.expiryDate } }),
      birthOrBusinessRegistrationNumber: t.input.string({
        validate: { schema: cardSchema.birthOrBusinessRegistrationNumber },
      }),
      passwordTwoDigits: t.input.string({ validate: { schema: cardSchema.passwordTwoDigits } }),
    },
    resolve: async (_, { input }, ctx) => {
      const [, expiryMonth, expiryYear] = input.expiryDate.match(/^(\d{2})(\d{2})$/) || [];

      const result = await portone.issueBillingKey({
        customerId: ctx.session.userId,
        cardNumber: input.cardNumber,
        expiryYear,
        expiryMonth,
        birthOrBusinessRegistrationNumber: input.birthOrBusinessRegistrationNumber,
        passwordTwoDigits: input.passwordTwoDigits,
      });

      if (result.status === 'failed') {
        throw new TypieError({ code: 'billing_key_issue_failed' });
      }

      return await replaceUserBillingKey({
        userId: ctx.session.userId,
        name: `${result.cardName} ${input.cardNumber.slice(-4)}`,
        type: BillingKeyType.CARD,
        billingKey: result.billingKey,
      });
    },
  }),

  updateBillingKeyWithEasyPay: t.withAuth({ session: true }).fieldWithInput({
    type: UserBillingKey,
    input: { billingKey: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      const result = await portone.getBillingKeyInfo({ billingKey: input.billingKey });

      if (result.status === 'failed') {
        throw new TypieError({ code: 'billing_key_issue_failed' });
      }

      const verification = verifyEasyPayBillingKey(result.issuance, {
        userId: ctx.session.userId,
        channelKey: env.PORTONE_KAKAOPAY_CHANNEL_KEY,
      });

      if (!verification.ok) {
        // 저장되는 키에는 항상 customerId 가 있으므로 customer_missing 만 우리 DB 가 참조하지 않음이 보장된다 — 다른 사유는 타인의·참조 중인 키를 지울 수 있어 회수하지 않는다.
        const deletion =
          verification.reason === 'customer_missing' ? await portone.deleteBillingKey({ billingKey: input.billingKey }) : undefined;

        Sentry.captureMessage('easy pay billing key rejected', {
          level: 'warning',
          extra: {
            userId: ctx.session.userId,
            reason: verification.reason,
            billingKey: input.billingKey,
            channelKeys: result.issuance.channelKeys,
            deletion,
          },
        });

        throw new TypieError({ code: 'billing_key_issue_failed' });
      }

      return await replaceUserBillingKey({
        userId: ctx.session.userId,
        name: '카카오페이',
        type: BillingKeyType.KAKAOPAY,
        billingKey: input.billingKey,
        guard: async (tx) => {
          if (await hasLiveYearlyBillingKeySubscription(tx, ctx.session.userId)) {
            throw new TypieError({ code: 'plan_interval_not_supported' });
          }
        },
      });
    },
  }),

  deleteBillingKey: t.withAuth({ session: true }).field({
    type: 'Boolean',
    resolve: async (_, __, ctx) => {
      const billingKey = await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        // 가드와 삭제가 같은 트랜잭션이어야 예약 생성과의 경합(빌링키 없는 예약 잔존)을 막는다.
        if (await hasFutureBillingObligation(tx, ctx.session.userId)) {
          throw new TypieError({ code: 'active_subscription_exists' });
        }

        return await tx
          .delete(UserBillingKeys)
          .where(eq(UserBillingKeys.userId, ctx.session.userId))
          .returning({ billingKey: UserBillingKeys.billingKey })
          .then(first);
      });

      if (billingKey) {
        await portone.deleteBillingKey({ billingKey: billingKey.billingKey });
      }

      return true;
    },
  }),

  subscribePlanWithBillingKey: t.withAuth({ session: true }).fieldWithInput({
    type: Subscription,
    input: { planId: t.input.id({ validate: validateDbId(TableCode.PLANS) }) },
    resolve: async (_, { input }, ctx) => {
      const plan = await db
        .select({ id: Plans.id, name: Plans.name, fee: Plans.fee, interval: Plans.interval })
        .from(Plans)
        .where(and(eq(Plans.id, input.planId), eq(Plans.availability, PlanAvailability.BILLING_KEY)))
        .then(firstOrThrow);

      const { subscription, paidInvoiceId } = await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        const subscriptionRows = await tx
          .select({
            id: Subscriptions.id,
            state: Subscriptions.state,
            planAvailability: Plans.availability,
            startsAt: Subscriptions.startsAt,
            currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
            currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
          })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(and(eq(Subscriptions.userId, ctx.session.userId), ne(Subscriptions.state, SubscriptionState.EXPIRED)));

        const now = dayjs();
        const action = resolveEnrollAction(subscriptionRows, now);
        if (action.kind === 'reject') {
          throw new TypieError({ code: 'subscription_already_exists' });
        }

        // 유예 소진 행은 liveness 가 비활성으로 판정해 여기까지 통과시킨다 — 열린 인보이스를 남긴 채 진행하면
        // 늦은 성공 확정이 죽은 구독을 되살리므로(해지 전이와 같은 불변식) 같은 트랜잭션에서 상태와 함께 거둔다.
        // IAP 행은 건드리지 않는다 — IAP 만료는 스토어 웹훅·재조정 소관이다.
        const exhaustedGraceIds = subscriptionRows
          .filter(
            (row) =>
              row.state === SubscriptionState.IN_GRACE_PERIOD &&
              row.planAvailability !== PlanAvailability.IN_APP_PURCHASE &&
              !isSubscriptionLive(row, now),
          )
          .map((row) => row.id);

        if (exhaustedGraceIds.length > 0) {
          await tx.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(inArray(Subscriptions.id, exhaustedGraceIds));
          await tx
            .update(PaymentInvoices)
            .set({ state: PaymentInvoiceState.CANCELED })
            .where(
              and(
                inArray(PaymentInvoices.subscriptionId, exhaustedGraceIds),
                inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE]),
              ),
            );
        }

        const billingKey = await tx
          .select({ id: UserBillingKeys.id, type: UserBillingKeys.type })
          .from(UserBillingKeys)
          .where(eq(UserBillingKeys.userId, ctx.session.userId))
          .then(first);

        if (billingKey && !supportsPlanInterval(billingKey.type, plan.interval)) {
          throw new TypieError({ code: 'plan_interval_not_supported' });
        }

        if (action.kind === 'schedule') {
          // 전환 잡의 predecessor 판정이 이 값과 트라이얼 주기 종료의 등호로 성립한다 — 내리지 않는다.
          const startsAt = action.startsAt;
          const periodStartsAt = floorToHourKst(startsAt);
          const periodEndsAt = computeNextPeriodEnd({ periodStartsAt, interval: plan.interval, billingAnchorAt: periodStartsAt });
          const hadReservation = subscriptionRows.some((row) => row.state === SubscriptionState.WILL_ACTIVATE);

          if (!billingKey) {
            throw new TypieError({ code: 'billing_key_required' });
          }

          const replaced = await retireReservation(tx, { userId: ctx.session.userId });

          // 봤던 예약이 사라졌다면 전환 잡이 그 사이 결제·활성화한 것 — 새 예약을 얹으면 안 된다.
          if (hadReservation && replaced.length === 0) {
            throw new TypieError({ code: 'subscription_already_exists' });
          }

          const reservation = await tx
            .insert(Subscriptions)
            .values({
              userId: ctx.session.userId,
              planId: plan.id,
              startsAt,
              currentPeriodStartsAt: periodStartsAt,
              currentPeriodEndsAt: periodEndsAt,
              billingAnchorAt: periodStartsAt,
              state: SubscriptionState.WILL_ACTIVATE,
            })
            .returning()
            .then(firstOrThrow);

          return { subscription: reservation, paidInvoiceId: null };
        }

        const startsAt = dayjs();
        // 사용량 버킷이 시간 단위라 주기 하한도 시간 경계로 내린다(가입 직전 같은 시간대 사용이 포함되는 과다 방향은 수용).
        const periodStartsAt = floorToHourKst(startsAt);
        const periodEndsAt = computeNextPeriodEnd({ periodStartsAt, interval: plan.interval, billingAnchorAt: periodStartsAt });

        // 유령 예약이 새 ACTIVE 와 공존하면 전환 잡이 결제를 시도한다 — 신규 구독 의사가 예약을 대체한다.
        await retireReservation(tx, { userId: ctx.session.userId });

        const created = await tx
          .insert(Subscriptions)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            startsAt,
            currentPeriodStartsAt: periodStartsAt,
            currentPeriodEndsAt: periodEndsAt,
            billingAnchorAt: periodStartsAt,
            state: SubscriptionState.ACTIVE,
          })
          .returning()
          .then(firstOrThrow);

        // UPCOMING 으로 시작해야 성공 확정의 PAID CAS 가 소유권 가드로 작동한다.
        const invoice = await tx
          .insert(PaymentInvoices)
          .values({
            userId: ctx.session.userId,
            subscriptionId: created.id,
            amount: plan.fee,
            state: PaymentInvoiceState.UPCOMING,
            dueAt: periodStartsAt,
            paymentKey: derivePaymentKey(created.id, periodStartsAt),
            servicePeriodStartsAt: periodStartsAt,
            servicePeriodEndsAt: periodEndsAt,
          })
          .returning({ id: PaymentInvoices.id })
          .then(firstOrThrow);

        // 신규 즉시 결제 실패는 유예가 아니라 롤백이다 — 구독을 만들지 않는다.
        const outcome = await attemptInvoicePayment(tx, invoice.id);
        if (outcome.kind !== 'paid') {
          throw new TypieError({ code: 'payment_failed' });
        }

        return { subscription: created, paidInvoiceId: invoice.id };
      });

      // 영수증 보강은 커밋 이후다 — 조회 실패가 성공 확정을 뒤집지 않고, 승인~커밋 창을 늘리지도 않는다.
      if (paidInvoiceId) {
        await enrichPaymentRecordReceipt(paidInvoiceId);
      }

      return subscription;
    },
  }),

  schedulePlanChange: t.withAuth({ session: true }).fieldWithInput({
    type: Subscription,
    input: { planId: t.input.id({ validate: validateDbId(TableCode.PLANS) }) },
    resolve: async (_, { input }, ctx) => {
      const plan = await db
        .select({ id: Plans.id, fee: Plans.fee, interval: Plans.interval })
        .from(Plans)
        .where(and(eq(Plans.id, input.planId), eq(Plans.availability, PlanAvailability.BILLING_KEY)))
        .then(firstOrThrow);

      return await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        const activeSubscription = await tx
          .select({ id: Subscriptions.id, currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(
            and(
              eq(Subscriptions.userId, ctx.session.userId),
              eq(Subscriptions.state, SubscriptionState.ACTIVE),
              eq(Plans.availability, PlanAvailability.BILLING_KEY),
            ),
          )
          .then(firstOrThrow);

        const billingKey = await tx
          .select({ type: UserBillingKeys.type })
          .from(UserBillingKeys)
          .where(eq(UserBillingKeys.userId, ctx.session.userId))
          .then(firstOrThrow);

        if (!supportsPlanInterval(billingKey.type, plan.interval)) {
          throw new TypieError({ code: 'plan_interval_not_supported' });
        }

        // 전환 잡의 predecessor 판정이 이 값과 현 구독 주기 종료의 등호로 성립한다 — 내리지 않는다.
        const startsAt = activeSubscription.currentPeriodEndsAt;
        // 플랜 변경은 앵커 재설정 시점이다.
        const billingAnchorAt = floorToHourKst(startsAt);
        const periodEndsAt = computeNextPeriodEnd({ periodStartsAt: startsAt, interval: plan.interval, billingAnchorAt });

        await retireReservation(tx, { userId: ctx.session.userId });

        await tx.update(Subscriptions).set({ state: SubscriptionState.WILL_EXPIRE }).where(eq(Subscriptions.id, activeSubscription.id));

        return await tx
          .insert(Subscriptions)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            startsAt,
            currentPeriodStartsAt: startsAt,
            currentPeriodEndsAt: periodEndsAt,
            billingAnchorAt,
            state: SubscriptionState.WILL_ACTIVATE,
          })
          .returning()
          .then(firstOrThrow);
      });
    },
  }),

  cancelPlanChange: t.withAuth({ session: true }).field({
    type: Subscription,
    resolve: async (_, __, ctx) => {
      return await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        // 전환 잡이 이미 결제·활성화했거나 IAP 가 대체했으면 0건.
        const retired = await retireReservation(tx, { userId: ctx.session.userId });

        if (retired.length === 0) {
          throw new TypieError({ code: 'plan_change_already_processed', status: 409 });
        }

        const trialSubscription = await tx
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(
            and(
              eq(Subscriptions.userId, ctx.session.userId),
              eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE),
              eq(Plans.availability, PlanAvailability.TRIAL),
            ),
          )
          .then(first);

        if (trialSubscription) {
          return await tx.select().from(Subscriptions).where(eq(Subscriptions.id, trialSubscription.id)).then(firstOrThrow);
        }

        // 해지 확정 잡이 그 사이 EXPIRED 로 만든 행을 무조건 UPDATE 로 부활시키지 않도록 상태를 CAS 하고,
        // 전환 공존 창에서 후보가 둘이면 전부 ACTIVE 를 시도하다 유니크 충돌로 예약 삭제까지 롤백되므로
        // 만료 전 최신 한 건만 대상으로 한다.
        const restoreCandidate = await tx
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(
            and(
              eq(Subscriptions.userId, ctx.session.userId),
              eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE),
              gt(Subscriptions.currentPeriodEndsAt, dayjs()),
              eq(Plans.availability, PlanAvailability.BILLING_KEY),
            ),
          )
          .orderBy(desc(Subscriptions.createdAt))
          .limit(1)
          .then(first);

        const restored = restoreCandidate
          ? await tx
              .update(Subscriptions)
              .set({ state: SubscriptionState.ACTIVE })
              .where(and(eq(Subscriptions.id, restoreCandidate.id), eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE)))
              .returning()
              .then(first)
          : null;

        if (restored) {
          return restored;
        }

        // 이전 구독이 이미 만료 확정된 뒤의 취소 — 예약 삭제(취소 의사)는 유지하고 최신 구독 행을 반환한다.
        // 방금 tombstone 한 예약 행(EXPIRED)이 createdAt 최신이라 이 조회에 다시 걸리므로 id 로 명시 제외한다.
        return await tx
          .select()
          .from(Subscriptions)
          .where(and(eq(Subscriptions.userId, ctx.session.userId), notInArray(Subscriptions.id, retired)))
          .orderBy(desc(Subscriptions.createdAt))
          .limit(1)
          .then(firstOrThrow);
      });
    },
  }),

  subscribeOrChangePlanWithInAppPurchase: t.withAuth({ session: true }).fieldWithInput({
    type: Subscription,
    input: {
      store: t.input.field({ type: InAppPurchaseStore }),
      data: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const alertContext = {
        source: 'graphql/subscribeOrChangePlanWithInAppPurchase',
        userId: ctx.session.userId,
        store: input.store,
        identifier: input.data,
      };

      // account_mismatch 를 받은 신버전 클라이언트는 트랜잭션을 종료하지만, 구버전은 앱 실행마다 재시도한다 —
      // 같은 유저·같은 토큰의 알람은 디듀프로 접는다.
      const mismatchDedupeKey = `${ctx.session.userId}:${input.data}`;

      // 거절의 알람·오류 코드는 GraphQL 표면의 몫이다 — 관측(조회·소유 증거·정규화)은 utils 가 판정하고 여기서는 옮기기만 한다.
      const enrollmentError = async (lookup: IapEnrollRejection) => {
        if (lookup.kind === 'lookup-failed') {
          return new Error(`in-app purchase lookup failed: ${lookup.detail}`);
        }

        if (lookup.reason === 'ownership-mismatch') {
          await opsAlertOnce('iap-ownership-mismatch', mismatchDedupeKey, { ...alertContext, detail: lookup.detail });
          return new TypieError({ code: 'in_app_purchase_account_mismatch' });
        }

        if (lookup.reason === 'family-shared') {
          await opsAlertOnce('iap-unsupported-store-payload', mismatchDedupeKey, { ...alertContext, detail: lookup.detail });
          return new TypieError({ code: 'in_app_purchase_account_mismatch' });
        }

        // 만료된 구매의 재등록은 미완료 트랜잭션의 앱 실행마다 재전송이 만드는 정상 거절이다 — 일반 Error 로 두면
        // 무한 반복이 Sentry 에 쌓인다. 전용 코드를 받은 클라이언트는 트랜잭션을 종료해 루프를 끊는다.
        if (lookup.reason === 'expired') {
          return new TypieError({ code: 'in_app_purchase_expired' });
        }

        return new Error(`in-app purchase is not trackable: ${lookup.detail}`);
      };

      // 등록 대상은 스토어에 실린 소유 증거가 지목한다 — 결제한 계정과 로그인 계정이 달라도 결제는 소유자에게
      // 귀속된다. 증거가 없는 구매(레거시)만 호출 세션에 귀속된다.
      const resolveOwnerUserId = async (executor: Database | Transaction, ownership: IapEnrollOwnership) => {
        if (ownership.kind === 'legacy') {
          return ctx.session.userId;
        }

        // 식별자는 앱이 users.uuid 를 실어 보낸 값이지만 스토어가 형식을 강제하지 않는다 — uuid 컬럼 비교 전에 거른다.
        if (!uuid.validate(ownership.ownerUuid)) {
          return null;
        }

        return await executor
          .select({ id: Users.id })
          .from(Users)
          .where(and(eq(Users.uuid, ownership.ownerUuid), eq(Users.state, UserState.ACTIVE)))
          .then(first)
          .then((row) => row?.id ?? null);
      };

      const planIntervalsOf = (plans: { id: string; interval: PlanInterval }[]): Record<string, PlanInterval> =>
        Object.fromEntries(plans.map((plan) => [plan.id, plan.interval]));

      const findRelatedBindings = async (executor: Database | Transaction, tokens: string[]) =>
        await executor
          .select({
            id: UserInAppPurchases.id,
            userId: UserInAppPurchases.userId,
            identifier: UserInAppPurchases.identifier,
            subscriptionId: UserInAppPurchases.subscriptionId,
          })
          .from(UserInAppPurchases)
          .where(and(eq(UserInAppPurchases.store, input.store), inArray(UserInAppPurchases.identifier, tokens)));

      // 1차 조회는 소유 증거·수용 가능성·관련 유저 확정 전용이다 — 상태·주기의 확정은 락 안 재조회가 한다.
      const probeFetch = await fetchIapEnrollment({ store: input.store, data: input.data });
      if (probeFetch.kind !== 'fetched') {
        throw await enrollmentError(probeFetch);
      }

      const ownerUserId = await resolveOwnerUserId(db, probeFetch.source.ownership);
      if (!ownerUserId) {
        // 증거는 있는데 대응하는 활성 계정이 없다(탈퇴·환경 불일치) — 재시도로 풀리지 않으므로 알람 + 수동이다.
        await opsAlertOnce('iap-ownership-mismatch', mismatchDedupeKey, {
          ...alertContext,
          detail: 'owner-not-found',
          ownerUuid: probeFetch.source.ownership.kind === 'evidenced' ? probeFetch.source.ownership.ownerUuid : null,
        });

        throw new TypieError({ code: 'in_app_purchase_account_mismatch' });
      }

      const probePlans = await db
        .select({ id: Plans.id, interval: Plans.interval })
        .from(Plans)
        .where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE));

      // 선행 주기는 락 밖에서도 읽는다 — 없이 정규화하면 1차 판정이 락 안 판정과 갈라져(주기 역산 불가 등)
      // 락 안에서라면 수용될 등록이 락 전에 거절된다. 주기는 소유자의 것이다 — 소유자 확정이 정규화보다 앞서는 이유.
      const probePrior = await db
        .select({
          state: Subscriptions.state,
          currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
          currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
        })
        .from(UserInAppPurchases)
        .innerJoin(Subscriptions, eq(UserInAppPurchases.subscriptionId, Subscriptions.id))
        .where(and(eq(UserInAppPurchases.userId, ownerUserId), eq(UserInAppPurchases.store, input.store)))
        .then(first);

      const probe = await normalizeIapEnrollment({
        source: probeFetch.source,
        prior: probePrior ?? null,
        planIntervals: planIntervalsOf(probePlans),
        now: dayjs(),
      });

      if (probe.kind !== 'observed') {
        throw await enrollmentError(probe);
      }

      // 교착 방지: 관련 유저(소유자 + predecessor 소유 유저)를 락 획득 전에 확정하고 userId 사전순으로 잠근다.
      // lockUserSubscriptionState 는 호출 즉시 xact 락을 잡으므로, 잠근 뒤 predecessor 를 발견하면 역순 잠금이 성립한다.
      // 호출 세션은 잠그지 않는다 — 이 흐름은 세션 유저의 행을 쓰지 않는다(소유자와 같을 때만 겹친다).
      const probeTokens = [...new Set([input.data, ...probe.observation.predecessorTokens])];
      const capturedBindings = await findRelatedBindings(db, probeTokens);
      // 사전순은 바이트 비교다 — 로케일 비교는 런타임 로케일에 따라 순서가 갈려 락 순서 규약이 흔들린다.
      const compareUserId = (a: string, b: string) => (a < b ? -1 : 1);
      const lockUserIds = [...new Set([ownerUserId, ...capturedBindings.map((row) => row.userId)])].toSorted(compareUserId);

      const { subscription, acknowledge, bindingId } = await db.transaction(async (tx) => {
        for (const userId of lockUserIds) {
          await lockUserSubscriptionState(tx, userId);
        }

        // 탈퇴 경합: 스토어 결제가 끝났어도 소유 계정이 사라졌으면 등록하지 않는다.
        await tx
          .select({ id: Users.id })
          .from(Users)
          .where(and(eq(Users.id, ownerUserId), eq(Users.state, UserState.ACTIVE)))
          .then(firstOrThrow);

        // 락 안 재조회. 소유 증거가 락 밖 관측과 다른 계정을 지목하면 그 계정의 락이 없다 — 재시도가 새 소유자 집합으로 잠근다.
        const lockedFetch = await fetchIapEnrollment({ store: input.store, data: input.data });
        if (lockedFetch.kind !== 'fetched') {
          throw await enrollmentError(lockedFetch);
        }

        if ((await resolveOwnerUserId(tx, lockedFetch.source.ownership)) !== ownerUserId) {
          throw new TypieError({ code: 'in_app_purchase_registration_conflict', status: 409 });
        }

        const binding = await tx
          .select({
            id: UserInAppPurchases.id,
            store: UserInAppPurchases.store,
            identifier: UserInAppPurchases.identifier,
            subscriptionId: UserInAppPurchases.subscriptionId,
          })
          .from(UserInAppPurchases)
          .where(eq(UserInAppPurchases.userId, ownerUserId))
          .for('no key update')
          .then(first);

        // canonical FK 가 빈 바인딩(백필 전 레거시·마커 격리)은 등록을 진행시키지 않는다 — 새 행을 만들면 그 바인딩이
        // 가리켰어야 할 구 행과 이중화되고, 어느 쪽이 canonical 인지 아무도 판정할 수 없다. 사람이 잇는다.
        if (binding && !binding.subscriptionId) {
          await opsAlert('invariant-violation', {
            ...alertContext,
            check: 'enroll-binding-null-canonical',
            bindingId: binding.id,
            boundIdentifier: binding.identifier,
          });

          throw new TypieError({ code: 'in_app_purchase_registration_conflict', status: 409 });
        }

        const canonical = binding?.subscriptionId
          ? await tx
              .select({
                id: Subscriptions.id,
                state: Subscriptions.state,
                currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
                currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
              })
              .from(Subscriptions)
              .where(eq(Subscriptions.id, binding.subscriptionId))
              .for('no key update')
              .then(first)
          : null;

        // 복합 FK 가 강제하는 참조라 도달하지 않는다 — 도달했다면 사람이 고칠 불변식 위반이다.
        if (!canonical && binding?.subscriptionId) {
          await opsAlert('invariant-violation', {
            ...alertContext,
            reason: 'iap binding canonical subscription missing',
            bindingId: binding.id,
            subscriptionId: binding.subscriptionId,
          });

          throw new TypieError({ code: 'in_app_purchase_registration_conflict', status: 409 });
        }

        const plans = await tx
          .select({ id: Plans.id, interval: Plans.interval })
          .from(Plans)
          .where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE));

        const now = dayjs();

        // 락 안 재조회·재정규화의 결과만 적용한다 — 락 밖 관측을 적용하면 그 사이 커밋된 환불 웹훅·재조정 뒤에
        // stale 한 상태가 되살아난다.
        const lookup = await normalizeIapEnrollment({
          source: lockedFetch.source,
          // 선행 주기는 같은 스토어 계약의 사실이다 — 크로스 스토어 바인딩의 주기를 먹이면 남의 계약으로 주기를 계산한다
          // (그 등록 자체는 아래 precheck 이 거절한다).
          prior:
            canonical && binding?.store === input.store
              ? {
                  state: canonical.state,
                  currentPeriodStartsAt: canonical.currentPeriodStartsAt,
                  currentPeriodEndsAt: canonical.currentPeriodEndsAt,
                }
              : null,
          planIntervals: planIntervalsOf(plans),
          now,
        });

        if (lookup.kind !== 'observed') {
          throw await enrollmentError(lookup);
        }

        const observation = lookup.observation;

        // (store, identifier) 의 소유 이동은 유저 advisory 락으로 검출되지 않는다 — 락 안 재SELECT 가 유일한 검출점이다.
        const relatedTokens = [...new Set([...probeTokens, ...observation.predecessorTokens])];
        const relatedBindings = await findRelatedBindings(tx, relatedTokens);

        // 잠그지 않은 유저의 바인딩은 건드릴 수 없다 — 재시도가 새 집합으로 잠근다.
        if (relatedBindings.some((row) => !lockUserIds.includes(row.userId))) {
          throw new TypieError({ code: 'in_app_purchase_registration_conflict', status: 409 });
        }

        for (const captured of capturedBindings) {
          const current = relatedBindings.find((row) => row.id === captured.id);
          if (!current || current.userId !== captured.userId || current.identifier !== captured.identifier) {
            throw new TypieError({ code: 'in_app_purchase_registration_conflict', status: 409 });
          }
        }

        const subscriptionRows = await tx
          .select({
            id: Subscriptions.id,
            state: Subscriptions.state,
            planAvailability: Plans.availability,
            startsAt: Subscriptions.startsAt,
            currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
            currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
            createdAt: Subscriptions.createdAt,
          })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(and(eq(Subscriptions.userId, ownerUserId), ne(Subscriptions.state, SubscriptionState.EXPIRED)));

        // preflight 는 신뢰하지 않는다 — 서버가 같은 판정을 다시 수행한다.
        const precheck = precheckIapEnroll({
          rows: subscriptionRows,
          binding: binding ? { store: binding.store } : null,
          store: input.store,
          iapPlanAvailable: plans.length > 0,
          now,
        });

        if (!precheck.allowed) {
          if (precheck.reason === 'cross-store-binding') {
            // 거절은 알람을 남긴다 — CS 유입 신호다.
            await opsAlert('iap-cross-store-enroll-rejected', { ...alertContext, boundStore: binding?.store, bindingId: binding?.id });
            throw new TypieError({ code: 'subscription_already_exists' });
          }

          if (precheck.reason === 'non-iap-subscription') {
            throw new TypieError({ code: 'subscription_already_exists' });
          }

          throw new Error('no in-app purchase plan is available');
        }

        // 같은 스토어의 허용은 동일 토큰 재등록 또는 검증된 승계뿐이다 — 연결 없는 독립 토큰을 등록하면
        // 유저당 1행인 바인딩이 기존 계약의 추적 주소를 잃는다. 단 기존 계약의 확정 종료를 스토어가 확인해
        // 주면 잃을 추적이 없다 — 만료 후 재구독은 스토어가 승계 포인터를 싣지 않아 독립 토큰으로만 도착하므로,
        // 이 재확인 없이는 죽은 바인딩이 정당한 재구독을 영구 거절한다.
        if (binding && binding.identifier !== input.data && !observation.successionSources.includes(binding.identifier)) {
          const boundContract = await probeIapBoundContractTermination({ store: binding.store, identifier: binding.identifier, now });

          if (boundContract.kind === 'lookup-failed') {
            throw new Error(`in-app purchase bound contract lookup failed: ${boundContract.detail}`);
          }

          if (boundContract.kind === 'live') {
            await opsAlert('iap-independent-token-rejected', {
              ...alertContext,
              bindingId: binding.id,
              boundIdentifier: binding.identifier,
            });

            throw new TypieError({ code: 'subscription_already_exists' });
          }
        }

        // 타 유저 predecessor 회수 — 한 스토어 계약이 두 타이피 유저의 권한으로 남지 않게 한다.
        // 잠금은 계보 전체로 넓게 잡되(위), 쓰기는 락 안 관측이 확정 종료로 판정한 원천과 요청 토큰으로만 좁힌다.
        const observedTokens = new Set([input.data, ...observation.predecessorTokens]);
        const recoveryTokens = new Set([input.data, ...observation.successionSources]);
        const foreignBindings = relatedBindings.filter((row) => row.userId !== ownerUserId && observedTokens.has(row.identifier));

        // 소유 증거 없는 구매(레거시·out-of-app)의 토큰이 다른 계정 소유면 정당한 주인이 따로 있다는 뜻이다 —
        // 호출 세션에 임의 배정하지 않는다(증거가 없으면 소유자 = 호출 세션이다). 거절 + 알람 + 수동이다.
        const unverifiedRecovery = foreignBindings.find((row) => recoveryTokens.has(row.identifier));
        if (unverifiedRecovery && !observation.ownershipVerified) {
          await opsAlertOnce('iap-ownership-mismatch', mismatchDedupeKey, {
            ...alertContext,
            detail: 'legacy-registration-foreign-recovery-token',
            foreignBindingId: unverifiedRecovery.id,
            foreignUserId: unverifiedRecovery.userId,
          });

          throw new TypieError({ code: 'in_app_purchase_account_mismatch' });
        }

        const retirableStates = [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD];

        for (const foreign of foreignBindings) {
          // 확정 종료가 아닌 형제 계약은 관측만 남긴다 — 살아 있는 계약을 회수하면 남의 유료 권한을 끊는다.
          if (!recoveryTokens.has(foreign.identifier)) {
            await opsAlert('iap-foreign-predecessor-observed', {
              ...alertContext,
              predecessorBindingId: foreign.id,
              predecessorUserId: foreign.userId,
              predecessorIdentifier: foreign.identifier,
            });

            continue;
          }

          // 회수 대상은 그 바인딩의 canonical 한 행뿐이다 — 유저·플랜으로 쓸면 그 유저의 다른 IAP 계약까지 끊는다.
          // 회수는 상태가 표현한다 — 주기 컬럼은 서비스 기간의 사실이라 자르지 않는다.
          await tx
            .update(Subscriptions)
            .set({ state: SubscriptionState.EXPIRED })
            .where(
              foreign.subscriptionId
                ? and(eq(Subscriptions.id, foreign.subscriptionId), inArray(Subscriptions.state, retirableStates))
                : // 백필 전 레거시 바인딩에는 가리키는 행이 없다 — 현행 폴백(그 유저의 IAP 행)을 유지한다.
                  and(
                    eq(Subscriptions.userId, foreign.userId),
                    inArray(Subscriptions.state, retirableStates),
                    inArray(
                      Subscriptions.planId,
                      tx.select({ id: Plans.id }).from(Plans).where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)),
                    ),
                  ),
            );

          await tx.delete(UserInAppPurchases).where(eq(UserInAppPurchases.id, foreign.id));
        }

        // 스토어가 선언한 승계 포인터가 전역에도 없으면 신규로 취급하되 남긴다 — 계정 이전·재설치 변칙의 신호다.
        if (
          observation.declaredPredecessors.length > 0 &&
          relatedBindings.every((row) => !observation.declaredPredecessors.includes(row.identifier))
        ) {
          await opsAlert('iap-succession-token-unknown-globally', {
            ...alertContext,
            successorTokens: observation.declaredPredecessors,
          });
        }

        // 스토어 구매가 웹 예약을 대체한다(오너 결정). 예약이 남으면 전환 잡이 카드 결제까지 시도한다.
        await retireReservation(tx, { userId: ownerUserId });

        // canonical 은 정리 대상에서 뺀다 — 여기서 EXPIRED 로 만들면 적용 primitive 의 복권 게이트가 방금 등록한 계약을 되돌린다.
        const canonicalExclusion = canonical ? ne(Subscriptions.id, canonical.id) : undefined;

        // 트라이얼 행을 같은 락 안에서 종료하지 않으면 재조정이 매일 승격 충돌로 스킵한다.
        await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED })
          .where(
            and(
              eq(Subscriptions.userId, ownerUserId),
              ne(Subscriptions.state, SubscriptionState.EXPIRED),
              canonicalExclusion,
              inArray(Subscriptions.planId, tx.select({ id: Plans.id }).from(Plans).where(eq(Plans.availability, PlanAvailability.TRIAL))),
            ),
          );

        const retiredOpenSubscriptions = await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED })
          .where(
            and(
              eq(Subscriptions.userId, ownerUserId),
              inArray(Subscriptions.state, [SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD]),
              canonicalExclusion,
            ),
          )
          .returning({ id: Subscriptions.id });

        // 이 시점에 열린 인보이스가 없다는 보장은 이 함수 밖 상태 전이 불변식(원격 술어)에 의존한다 — 로컬로
        // 검증할 수 없으므로 형제 지점(scheduleSubscriptionCancellation·retireReservation)과 동형으로 CAS 방어를 심층화한다.
        await tx
          .update(PaymentInvoices)
          .set({ state: PaymentInvoiceState.CANCELED })
          .where(
            and(
              inArray(
                PaymentInvoices.subscriptionId,
                retiredOpenSubscriptions.map((row) => row.id),
              ),
              inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE]),
            ),
          );

        let lockedBinding: { id: string; userId: string; store: InAppPurchaseStore; identifier: string; subscriptionId: string };

        if (binding && canonical) {
          // 바인딩이 있으면 canonical 은 그 FK 가 가리키는 한 행이다 — upsert 는 목표 상태가 ACTIVE 가 아닐 때
          // 충돌 없이 새 행을 만들어 한 행 원칙을 깬다. 상태·주기는 아래 primitive 가 이 행 ID 로 갱신한다.
          // 토큰 교체와 재조정 비활성 마커 해제는 같은 UPDATE 다 — 갈라지면 승계된 토큰이 재조정에서 영구 제외된다.
          await tx
            .update(UserInAppPurchases)
            .set({
              identifier: input.data,
              subscriptionId: canonical.id,
              ...(binding.identifier !== input.data && { reconcileSuspendedAt: null }),
            })
            .where(eq(UserInAppPurchases.id, binding.id));

          lockedBinding = {
            id: binding.id,
            userId: ownerUserId,
            store: input.store,
            identifier: input.data,
            subscriptionId: canonical.id,
          };
        } else {
          const plan = await tx
            .select({ id: Plans.id })
            .from(Plans)
            .where(and(eq(Plans.id, observation.normalized.planKey), eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)))
            .then(firstOrThrow);

          const created = await tx
            .insert(Subscriptions)
            .values({
              userId: ownerUserId,
              planId: plan.id,
              startsAt: observation.startsAt,
              currentPeriodStartsAt: observation.normalized.periodStartsAt,
              currentPeriodEndsAt: observation.normalized.periodEndsAt,
              state: observation.normalized.state,
            })
            .onConflictDoUpdate({
              target: [Subscriptions.userId],
              targetWhere: eq(Subscriptions.state, SubscriptionState.ACTIVE),
              // 기존 행의 startsAt 은 보존한다(구독이 처음 부여된 시각).
              set: {
                planId: plan.id,
                currentPeriodStartsAt: observation.normalized.periodStartsAt,
                currentPeriodEndsAt: observation.normalized.periodEndsAt,
                state: observation.normalized.state,
              },
              // 위 검사 이후 동시에 커밋된 다른 채널(빌링키 등) ACTIVE 구독을 IAP 값으로 덮어쓰지 않는다.
              // 충돌 행이 IAP 가 아니면 no-op → returning 이 비어 firstOrThrow 로 트랜잭션 전체가 롤백된다(오염 대신 실패).
              setWhere: inArray(
                Subscriptions.planId,
                tx.select({ id: Plans.id }).from(Plans).where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)),
              ),
            })
            .returning({ id: Subscriptions.id })
            .then(firstOrThrow);

          // 크로스 스토어 덮어쓰기는 하지 않는다(precheck 이 이미 거절했다) — setWhere 가 없으면 경합이 그 문을 다시 연다.
          const bound = await tx
            .insert(UserInAppPurchases)
            .values({ userId: ownerUserId, store: input.store, identifier: input.data, subscriptionId: created.id })
            .onConflictDoUpdate({
              target: [UserInAppPurchases.userId],
              set: { identifier: input.data, subscriptionId: created.id, reconcileSuspendedAt: null },
              setWhere: eq(UserInAppPurchases.store, input.store),
            })
            .returning({ id: UserInAppPurchases.id })
            .then(firstOrThrow);

          lockedBinding = {
            id: bound.id,
            userId: ownerUserId,
            store: input.store,
            identifier: input.data,
            subscriptionId: created.id,
          };
        }

        // 상태·주기·플랜의 적용은 재조정·웹훅과 같은 primitive 를 공유한다(직접 UPDATE 금지).
        const { acknowledge } = await applyNormalizedIapLocked(tx, { binding: lockedBinding, normalized: observation.normalized });

        const subscription = await tx
          .select()
          .from(Subscriptions)
          .where(eq(Subscriptions.id, lockedBinding.subscriptionId))
          .then(firstOrThrow);

        return { subscription, acknowledge, bindingId: lockedBinding.id };
      });

      // 승인은 커밋 후 의무다 — 롤백된 트랜잭션의 토큰을 승인하지 않는다.
      if (acknowledge) {
        try {
          await googleplay.acknowledgeSubscription(acknowledge);
        } catch (err) {
          await opsAlert('google-acknowledge-failed', { ...acknowledge, error: err instanceof Error ? err.message : String(err) });
        }
      }

      await enqueueJob('iap:ingest', { bindingId });

      // 소유자에게 등록은 반영됐지만 호출 세션의 것이 아니다 — 반환형이 Subscription 이라 그대로 돌려주면 타 유저
      // 데이터가 샌다. 클라이언트는 이 코드에서 트랜잭션을 종료한다(등록이 반영됐으므로 결제는 유실되지 않는다).
      if (ownerUserId !== ctx.session.userId) {
        await opsAlertOnce('iap-ownership-mismatch', mismatchDedupeKey, { ...alertContext, detail: 'enrolled-to-owner', ownerUserId });
        throw new TypieError({ code: 'in_app_purchase_account_mismatch' });
      }

      return subscription;
    },
  }),

  scheduleSubscriptionCancellation: t.withAuth({ session: true }).field({
    type: Subscription,
    resolve: async (_, __, ctx) => {
      return await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        const activeSubscription = await tx
          .select({ id: Subscriptions.id, state: Subscriptions.state })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(
            and(
              eq(Subscriptions.userId, ctx.session.userId),
              inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.IN_GRACE_PERIOD]),
              eq(Plans.availability, PlanAvailability.BILLING_KEY),
            ),
          )
          .then(firstOrThrow);

        // 교착 방지 전역 락 순서: WILL_ACTIVATE 구독 → 주 구독 → 인보이스.
        // 환불·갱신 재시도가 구독을 잠근 채 인보이스를 갱신하므로, 여기서 인보이스를 구독보다 먼저 잠그면 역순 교착이 된다.
        await retireReservation(tx, { userId: ctx.session.userId });

        // 회수는 상태가 표현한다 — 주기 컬럼은 서비스 기간의 사실이라 자르지 않는다.
        const subscription = await tx
          .update(Subscriptions)
          .set({
            state: activeSubscription.state === SubscriptionState.ACTIVE ? SubscriptionState.WILL_EXPIRE : SubscriptionState.EXPIRED,
          })
          .where(eq(Subscriptions.id, activeSubscription.id))
          .returning()
          .then(firstOrThrow);

        // 청구 이탈 전이는 열린 인보이스를 같은 트랜잭션에서 거둔다 — 남기면 늦은 성공 확정이 해지한 구독을 되살린다.
        if (activeSubscription.state === SubscriptionState.IN_GRACE_PERIOD) {
          await tx
            .update(PaymentInvoices)
            .set({ state: PaymentInvoiceState.CANCELED })
            .where(
              and(
                eq(PaymentInvoices.subscriptionId, activeSubscription.id),
                inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE]),
              ),
            );
        }

        return subscription;
      });
    },
  }),

  cancelSubscriptionCancellation: t.withAuth({ session: true }).field({
    type: Subscription,
    resolve: async (_, __, ctx) => {
      return await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        // 전환 공존 창의 옛 행을 고르지 않도록 만료 전 행만, 최신 우선으로 선택한다.
        const willExpireSubscription = await tx
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(
            and(
              eq(Subscriptions.userId, ctx.session.userId),
              eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE),
              gt(Subscriptions.currentPeriodEndsAt, dayjs()),
              eq(Plans.availability, PlanAvailability.BILLING_KEY),
            ),
          )
          .orderBy(desc(Subscriptions.createdAt))
          .limit(1)
          .then(first);

        // 해지 확정 잡이 먼저 만료시켰으면 재개할 대상이 없다 — 일반 500 이 아니라 명시적 conflict 로 응답한다.
        if (!willExpireSubscription) {
          throw new TypieError({ code: 'subscription_already_expired', status: 409 });
        }

        const willActivateSubscription = await tx
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)))
          .then(first);

        if (willActivateSubscription) {
          throw new TypieError({ code: 'plan_change_scheduled' });
        }

        // 해지 확정 잡이 그 사이 EXPIRED 로 만든 행을 부활시키지 않는다.
        const restored = await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.ACTIVE })
          .where(and(eq(Subscriptions.id, willExpireSubscription.id), eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE)))
          .returning()
          .then(first);

        if (!restored) {
          throw new TypieError({ code: 'subscription_already_expired', status: 409 });
        }

        return restored;
      });
    },
  }),
}));
