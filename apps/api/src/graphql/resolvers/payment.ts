import { createHash } from 'node:crypto';
import * as Sentry from '@sentry/node';
import { defaultPlanRules, TRIAL_DURATION_DAYS } from '@typie/lib/const';
import {
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
import { cardSchema, redeemCodeSchema } from '@typie/lib/validation';
import dayjs from 'dayjs';
import { and, desc, eq, gt, inArray, ne, or, sql } from 'drizzle-orm';
import {
  CreditCodes,
  db,
  Entities,
  first,
  firstOrThrow,
  firstOrThrowWith,
  PaymentInvoices,
  PaymentRecords,
  Plans,
  PostPaywallPurchases,
  PostPaywalls,
  Posts,
  Subscriptions,
  TableCode,
  UserBillingKeys,
  UserInAppPurchases,
  UserRevenues,
  Users,
  UserTrials,
  validateDbId,
} from '#/db/index.ts';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';
import * as portone from '#/external/portone.ts';
import { getSubscriptionExpiresAt, hasBillableUsageDuring, payAmountWithBillingKey, payInvoiceWithBillingKey } from '#/utils/index.ts';
import { createTrialSubscription } from '#/utils/plan.ts';
import { delay } from '#/utils/promise.ts';
import { resolveEnrollAction } from '#/utils/subscription-enroll.ts';
import { lockUserSubscriptionState } from '#/utils/subscription-lock.ts';
import { getUserUuid } from '#/utils/user.ts';
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
    expiresAt: t.expose('expiresAt', { type: 'DateTime' }),
    state: t.expose('state', { type: SubscriptionState }),
    user: t.expose('userId', { type: User }),
    hasBillableUsage: t.boolean({
      resolve: async (self) => {
        return await hasBillableUsageDuring(db, self.userId, self.renewedAt, self.expiresAt);
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

      try {
        return await db.transaction(async (tx) => {
          await lockUserSubscriptionState(tx, ctx.session.userId);

          // 발급 대기 중 탈퇴가 완료됐으면 키를 재삽입하지 않는다.
          await tx
            .select({ id: Users.id })
            .from(Users)
            .where(and(eq(Users.id, ctx.session.userId), eq(Users.state, UserState.ACTIVE)))
            .then(firstOrThrow);

          const existingBillingKey = await tx
            .delete(UserBillingKeys)
            .where(eq(UserBillingKeys.userId, ctx.session.userId))
            .returning({ billingKey: UserBillingKeys.billingKey })
            .then(first);

          if (existingBillingKey) {
            await portone.deleteBillingKey({ billingKey: existingBillingKey.billingKey });
          }

          return await tx
            .insert(UserBillingKeys)
            .values({
              userId: ctx.session.userId,
              name: `${result.cardName} ${input.cardNumber.slice(-4)}`,
              billingKey: result.billingKey,
              cardNumberHash: createHash('sha256').update(input.cardNumber).digest('hex'),
            })
            .returning()
            .then(firstOrThrow);
        });
      } catch (err) {
        // 저장 실패(탈퇴 경합 등) 시 방금 발급한 외부 키가 로컬 참조 없는 고아로 남지 않게 회수한다.
        // deleteBillingKey 는 throw 하지 않고 상태를 반환하므로, 결과를 검사해야 회수 실패가 무음으로 사라지지 않는다.
        const deletion = await portone.deleteBillingKey({ billingKey: result.billingKey });
        if (deletion.status === 'failed') {
          Sentry.captureMessage('billing key compensation cleanup failed', {
            level: 'warning',
            extra: { userId: ctx.session.userId, billingKey: result.billingKey, code: deletion.code, message: deletion.message },
          });
        }
        throw err;
      }
    },
  }),

  deleteBillingKey: t.withAuth({ session: true }).field({
    type: 'Boolean',
    resolve: async (_, __, ctx) => {
      const billingKey = await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        // 가드와 삭제가 같은 트랜잭션이어야 예약 생성과의 경합(빌링키 없는 예약 잔존)을 막는다.
        const activeSubscription = await tx
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(
            and(
              eq(Subscriptions.userId, ctx.session.userId),
              inArray(Subscriptions.state, [
                SubscriptionState.ACTIVE,
                SubscriptionState.WILL_EXPIRE,
                SubscriptionState.IN_GRACE_PERIOD,
                SubscriptionState.WILL_ACTIVATE,
              ]),
              eq(Plans.availability, PlanAvailability.BILLING_KEY),
            ),
          )
          .then(first);

        if (activeSubscription) {
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

      return await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        const subscriptionRows = await tx
          .select({ state: Subscriptions.state, planAvailability: Plans.availability, expiresAt: Subscriptions.expiresAt })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(and(eq(Subscriptions.userId, ctx.session.userId), ne(Subscriptions.state, SubscriptionState.EXPIRED)));

        const action = resolveEnrollAction(subscriptionRows, dayjs());
        if (action.kind === 'reject') {
          throw new TypieError({ code: 'subscription_already_exists' });
        }

        if (action.kind === 'schedule') {
          const startsAt = action.startsAt;
          const expiresAt = getSubscriptionExpiresAt(startsAt, plan.interval);
          const hadReservation = subscriptionRows.some((row) => row.state === SubscriptionState.WILL_ACTIVATE);

          const billingKey = await tx
            .select({ id: UserBillingKeys.id })
            .from(UserBillingKeys)
            .where(eq(UserBillingKeys.userId, ctx.session.userId))
            .then(first);

          if (!billingKey) {
            throw new TypieError({ code: 'billing_key_required' });
          }

          const replaced = await tx
            .delete(Subscriptions)
            .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)))
            .returning({ id: Subscriptions.id });

          // 봤던 예약이 사라졌다면 전환 잡이 그 사이 결제·활성화한 것 — 새 예약을 얹으면 안 된다.
          if (hadReservation && replaced.length === 0) {
            throw new TypieError({ code: 'subscription_already_exists' });
          }

          return await tx
            .insert(Subscriptions)
            .values({
              userId: ctx.session.userId,
              planId: plan.id,
              startsAt,
              expiresAt,
              renewedAt: startsAt,
              state: SubscriptionState.WILL_ACTIVATE,
            })
            .returning()
            .then(firstOrThrow);
        }

        const startsAt = dayjs();
        const expiresAt = getSubscriptionExpiresAt(startsAt, plan.interval);

        // 유령 예약이 새 ACTIVE 와 공존하면 전환 잡이 결제를 시도한다 — 신규 구독 의사가 예약을 대체한다.
        await tx
          .delete(Subscriptions)
          .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)));

        const subscription = await tx
          .insert(Subscriptions)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            startsAt,
            expiresAt,
            renewedAt: startsAt,
            state: SubscriptionState.ACTIVE,
          })
          .returning()
          .then(firstOrThrow);

        const invoice = await tx
          .insert(PaymentInvoices)
          .values({
            userId: ctx.session.userId,
            subscriptionId: subscription.id,
            amount: plan.fee,
            dueAt: startsAt,
            state: PaymentInvoiceState.PAID,
          })
          .returning({ id: PaymentInvoices.id })
          .then(firstOrThrow);

        const success = await payInvoiceWithBillingKey(tx, invoice.id);
        if (!success) {
          throw new TypieError({ code: 'payment_failed' });
        }

        return subscription;
      });
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
          .select({ id: Subscriptions.id, expiresAt: Subscriptions.expiresAt })
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

        const startsAt = activeSubscription.expiresAt;
        const expiresAt = getSubscriptionExpiresAt(startsAt, plan.interval);

        await tx
          .delete(Subscriptions)
          .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)));

        await tx.update(Subscriptions).set({ state: SubscriptionState.WILL_EXPIRE }).where(eq(Subscriptions.id, activeSubscription.id));

        return await tx
          .insert(Subscriptions)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            startsAt,
            expiresAt,
            renewedAt: activeSubscription.expiresAt,
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

        // 전환 잡이 이미 결제·활성화했거나 IAP 가 대체했으면 0건 — id 로만 지우면 ACTIVE 행 삭제 시도가 인보이스 FK 에 걸린다.
        const deleted = await tx
          .delete(Subscriptions)
          .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)))
          .returning({ id: Subscriptions.id });

        if (deleted.length === 0) {
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
              gt(Subscriptions.expiresAt, dayjs()),
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
        return await tx
          .select()
          .from(Subscriptions)
          .where(eq(Subscriptions.userId, ctx.session.userId))
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
      let identifier: string;
      let planId: string;
      let startsAt: dayjs.Dayjs;
      let expiresAt: dayjs.Dayjs;
      // Google 은 purchaseToken 이 바뀔 때 이전 토큰을 알려준다: 비만료 재가입/플랜 변경은 linkedPurchaseToken 으로,
      // 만료 후 Play 구독 센터 재구독(out-of-app)은 outOfAppPurchaseContext.expiredPurchaseToken 으로(acknowledge 후 소멸).
      let previousPurchaseTokens: string[] = [];
      // 스토어 응답의 계정 식별자로 현재 세션 소유가 증명됐는지. 증명 없이 이전 토큰이 타 계정에 바인딩돼 있으면 배정을 거부한다.
      let ownershipVerified = false;

      if (input.store === InAppPurchaseStore.APP_STORE) {
        const subscription = await appstore.getSubscription(input.data);

        if (!subscription.productId || !subscription.originalTransactionId || !subscription.purchaseDate || !subscription.expiresDate) {
          throw new Error('required fields are missing');
        }

        // 소유권 검증: 다른 유저의 구매(예: 복구 경로가 잘못 흘려보낸 구매)가 현재 세션에 바인딩되는 것을 막는다.
        // appAccountToken 이 없는 레거시 구매는 통과시킨다.
        if (subscription.appAccountToken && subscription.appAccountToken !== getUserUuid(ctx.session.userId)) {
          throw new TypieError({ code: 'in_app_purchase_account_mismatch' });
        }

        identifier = subscription.originalTransactionId;
        planId = subscription.productId.toUpperCase();
        startsAt = dayjs(subscription.purchaseDate);
        expiresAt = dayjs(subscription.expiresDate);
      } else if (input.store === InAppPurchaseStore.GOOGLE_PLAY) {
        const subscription = await googleplay.getSubscription(input.data);

        if (subscription.subscriptionState !== 'SUBSCRIPTION_STATE_ACTIVE') {
          throw new Error('subscriptionState is not active');
        }

        const item = subscription.lineItems?.[0];
        if (!item || !item.offerDetails?.basePlanId || !subscription.startTime || !item.expiryTime) {
          throw new Error('required fields are missing');
        }

        // 소유권 검증: 다른 유저의 구매가 현재 세션에 바인딩되는 것을 막는다.
        // 만료 후 재구독(out-of-app)은 externalAccountIdentifiers 없이 이전 구매의 식별자를
        // outOfAppPurchaseContext 로만 노출하므로 그쪽도 함께 본다. 둘 다 없는 레거시 구매는 통과.
        const obfuscatedAccountId =
          subscription.externalAccountIdentifiers?.obfuscatedExternalAccountId ??
          subscription.outOfAppPurchaseContext?.expiredExternalAccountIdentifiers?.obfuscatedExternalAccountId;
        if (obfuscatedAccountId && obfuscatedAccountId !== getUserUuid(ctx.session.userId)) {
          throw new TypieError({ code: 'in_app_purchase_account_mismatch' });
        }

        identifier = input.data;
        planId = item.offerDetails.basePlanId.toUpperCase();
        startsAt = dayjs(subscription.startTime);
        expiresAt = dayjs(item.expiryTime);
        previousPurchaseTokens = [subscription.linkedPurchaseToken, subscription.outOfAppPurchaseContext?.expiredPurchaseToken].filter(
          (token): token is string => !!token,
        );
        ownershipVerified = !!obfuscatedAccountId;
      } else {
        throw new Error('Invalid store');
      }

      if (!expiresAt.isAfter(dayjs())) {
        throw new Error('expiresAt should be in the future');
      }

      await db
        .select({ id: Plans.id })
        .from(Plans)
        .where(and(eq(Plans.id, planId), eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)))
        .then(firstOrThrow);

      return await db.transaction(async (tx) => {
        await lockUserSubscriptionState(tx, ctx.session.userId);

        // 스토어 서명·소유권 검증은 락 밖 결과를 쓰되, 계정·구독 상태는 락 후 재판정한다(탈퇴·동시 구독 경합).
        await tx
          .select({ id: Users.id })
          .from(Users)
          .where(and(eq(Users.id, ctx.session.userId), eq(Users.state, UserState.ACTIVE)))
          .then(firstOrThrow);

        // 시간상 만료된 WILL_EXPIRE 는 차단하지 않는다 — resolveEnrollAction 과 동일한 liveness 기준.
        // 해지 확정 잡 지연(~1분)이 이미 스토어 결제를 마친 등록을 거부하면 안 된다.
        const existingSubscription = await tx
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(
            and(
              eq(Subscriptions.userId, ctx.session.userId),
              ne(Subscriptions.state, SubscriptionState.EXPIRED),
              ne(Subscriptions.state, SubscriptionState.WILL_ACTIVATE),
              or(ne(Subscriptions.state, SubscriptionState.WILL_EXPIRE), gt(Subscriptions.expiresAt, dayjs())),
              ne(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
              ne(Plans.availability, PlanAvailability.TRIAL),
            ),
          )
          .then(first);

        if (existingSubscription) {
          throw new TypieError({ code: 'subscription_already_exists' });
        }

        const trialSubscription = await tx
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
          .where(
            and(
              eq(Subscriptions.userId, ctx.session.userId),
              ne(Subscriptions.state, SubscriptionState.EXPIRED),
              eq(Plans.availability, PlanAvailability.TRIAL),
            ),
          )
          .then(first);

        // 스토어 구매가 웹 예약을 대체한다(오너 결정). 예약이 남으면 전환 잡이 카드 결제까지 시도한다.
        await tx
          .delete(Subscriptions)
          .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)));

        // 이전 purchaseToken 을 보유한 "다른" 타이피 계정이 있으면(같은 스토어 계정에서 플랜 변경/재구독으로 토큰이 바뀐 경우),
        // 그 구독을 만료시키고 바인딩을 제거한다. 하나의 스토어 구독이 여러 타이피 계정에 동시에 활성화되는 것을 막는다.
        // (같은 계정의 토큰 변경은 아래 userId 기준 upsert 가 바인딩을 이동시키므로 여기서 건드리지 않는다)
        for (const previousPurchaseToken of previousPurchaseTokens) {
          const previousBinding = await tx
            .select({ userId: UserInAppPurchases.userId })
            .from(UserInAppPurchases)
            .where(
              and(
                eq(UserInAppPurchases.store, InAppPurchaseStore.GOOGLE_PLAY),
                eq(UserInAppPurchases.identifier, previousPurchaseToken),
                ne(UserInAppPurchases.userId, ctx.session.userId),
              ),
            )
            .then(first);

          if (previousBinding) {
            // 계정 증빙 없는 구매(레거시·out-of-app)의 이전 토큰이 다른 계정 소유 — 정당한 주인이 따로 있다는 뜻이므로
            // 현재 세션에 임의 배정하지 않고 거부한다. 주인 계정으로 로그인하면 복구 경로가 다시 등록한다.
            if (!ownershipVerified) {
              throw new TypieError({ code: 'in_app_purchase_account_mismatch' });
            }

            await tx
              .update(Subscriptions)
              .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
              .where(
                and(
                  eq(Subscriptions.userId, previousBinding.userId),
                  inArray(Subscriptions.state, [
                    SubscriptionState.ACTIVE,
                    SubscriptionState.WILL_EXPIRE,
                    SubscriptionState.IN_GRACE_PERIOD,
                  ]),
                  inArray(
                    Subscriptions.planId,
                    tx.select({ id: Plans.id }).from(Plans).where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)),
                  ),
                ),
              );

            await tx
              .delete(UserInAppPurchases)
              .where(
                and(eq(UserInAppPurchases.store, InAppPurchaseStore.GOOGLE_PLAY), eq(UserInAppPurchases.identifier, previousPurchaseToken)),
              );
          }
        }

        if (trialSubscription) {
          await tx
            .update(Subscriptions)
            .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
            .where(eq(Subscriptions.id, trialSubscription.id));
        }

        await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
          .where(
            and(
              eq(Subscriptions.userId, ctx.session.userId),
              inArray(Subscriptions.state, [SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD]),
            ),
          );

        await tx
          .insert(UserInAppPurchases)
          .values({
            userId: ctx.session.userId,
            store: input.store,
            identifier,
          })
          .onConflictDoUpdate({
            target: [UserInAppPurchases.userId],
            set: { store: input.store, identifier },
          });

        return await tx
          .insert(Subscriptions)
          .values({
            userId: ctx.session.userId,
            planId,
            startsAt,
            expiresAt,
            renewedAt: startsAt,
            state: SubscriptionState.ACTIVE,
          })
          .onConflictDoUpdate({
            target: [Subscriptions.userId],
            targetWhere: eq(Subscriptions.state, SubscriptionState.ACTIVE),
            set: { planId, startsAt, expiresAt, renewedAt: startsAt },
            // 상단 eligibility 검사 이후 동시에 커밋된 다른 채널(빌링키 등) ACTIVE 구독을 IAP 값으로 덮어쓰지 않는다.
            // 충돌 행이 IAP 가 아니면 no-op → returning 이 비어 firstOrThrow 로 트랜잭션 전체가 롤백된다(오염 대신 실패).
            setWhere: inArray(
              Subscriptions.planId,
              tx.select({ id: Plans.id }).from(Plans).where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)),
            ),
          })
          .returning()
          .then(firstOrThrow);
      });
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
        await tx
          .delete(Subscriptions)
          .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)));

        const subscription = await tx
          .update(Subscriptions)
          .set(
            activeSubscription.state === SubscriptionState.ACTIVE
              ? { state: SubscriptionState.WILL_EXPIRE }
              : { state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` },
          )
          .where(eq(Subscriptions.id, activeSubscription.id))
          .returning()
          .then(firstOrThrow);

        if (activeSubscription.state === SubscriptionState.IN_GRACE_PERIOD) {
          await tx
            .update(PaymentInvoices)
            .set({ state: PaymentInvoiceState.CANCELED })
            .where(and(eq(PaymentInvoices.subscriptionId, activeSubscription.id), eq(PaymentInvoices.state, PaymentInvoiceState.OVERDUE)));
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
              gt(Subscriptions.expiresAt, dayjs()),
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

  purchasePaywall: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      postId: t.input.id({ validate: validateDbId(TableCode.POSTS) }),
      nodeId: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const paywall = await db
        .select({ id: PostPaywalls.id, price: PostPaywalls.price, authorId: Entities.userId })
        .from(PostPaywalls)
        .innerJoin(Posts, eq(PostPaywalls.postId, Posts.id))
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(and(eq(PostPaywalls.postId, input.postId), eq(PostPaywalls.nodeId, input.nodeId)))
        .then(firstOrThrowWith(new NotFoundError()));

      if (paywall.authorId === ctx.session.userId) {
        throw new TypieError({ code: 'cannot_purchase_own_post' });
      }

      return await db.transaction(async (tx) => {
        const existingPurchase = await tx
          .select({ id: PostPaywallPurchases.id })
          .from(PostPaywallPurchases)
          .where(and(eq(PostPaywallPurchases.paywallId, paywall.id), eq(PostPaywallPurchases.userId, ctx.session.userId)))
          .for('no key update')
          .then(first);

        if (existingPurchase) {
          throw new TypieError({ code: 'paywall_already_purchased' });
        }
        const result = await payAmountWithBillingKey(tx, {
          paymentId: `${ctx.session.userId}_${paywall.id}`,
          userId: ctx.session.userId,
          orderName: `타이피 ${paywall.price} P`,
          amount: paywall.price,
        });

        await tx.insert(PostPaywallPurchases).values({
          paywallId: paywall.id,
          userId: ctx.session.userId,
          billingAmount: result.billingAmount,
          creditAmount: result.creditAmount,
          data: result.data,
        });

        if (result.status !== 'succeeded') {
          throw new TypieError({ code: 'payment_failed' });
        }

        await tx
          .insert(UserRevenues)
          .values({ userId: paywall.authorId, amount: paywall.price })
          .onConflictDoUpdate({
            target: [UserRevenues.userId],
            set: { amount: sql`${UserRevenues.amount} + ${paywall.price}` },
          });

        return true;
      });
    },
  }),
}));
