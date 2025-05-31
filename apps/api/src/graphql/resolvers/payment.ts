import dayjs, { Dayjs } from 'dayjs';
import { and, eq, gt, lt } from 'drizzle-orm';
import {
  CreditCodes,
  db,
  first,
  firstOrThrow,
  firstOrThrowWith,
  PaymentBillingKeys,
  PaymentInvoices,
  Plans,
  TableCode,
  UserIAPSubscriptions,
  UserPlans,
  validateDbId,
} from '@/db';
import { defaultPlanRules } from '@/db/schemas/json';
import {
  CreditCodeState,
  InAppPurchaseStore,
  PaymentBillingKeyState,
  PaymentInvoiceState,
  PlanAvailability,
  UserPlanBillingCycle,
  UserPlanBillingMethod,
  UserPlanState,
} from '@/enums';
import { production } from '@/env';
import { NotFoundError, TypieError } from '@/errors';
import * as appstore from '@/external/appstore';
import * as googleplay from '@/external/googleplay';
import * as portone from '@/external/portone';
import * as slack from '@/external/slack';
import { calculatePaymentAmount, payInvoice } from '@/utils';
import { delay } from '@/utils/promise';
import { cardSchema, redeemCodeSchema } from '@/validation';
import { builder } from '../builder';
import { CreditCode, isTypeOf, PaymentBillingKey, PaymentInvoice, Plan, PlanRule, UserPlan } from '../objects';

/**
 * * Types
 */

PaymentBillingKey.implement({
  isTypeOf: isTypeOf(TableCode.PAYMENT_BILLING_KEYS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

PaymentInvoice.implement({
  isTypeOf: isTypeOf(TableCode.PAYMENT_INVOICES),
  fields: (t) => ({
    id: t.exposeID('id'),
    state: t.expose('state', { type: PaymentInvoiceState }),
    amount: t.exposeInt('amount'),
    billingAt: t.expose('billingAt', { type: 'DateTime' }),
  }),
});

Plan.implement({
  isTypeOf: isTypeOf(TableCode.PLANS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    fee: t.exposeInt('fee'),

    rules: t.expose('rules', { type: PlanRule }),
  }),
});

PlanRule.implement({
  fields: (t) => ({
    maxTotalCharacterCount: t.int({ resolve: (self) => self.maxTotalCharacterCount ?? defaultPlanRules.maxTotalCharacterCount }),
    maxTotalBlobSize: t.int({ resolve: (self) => self.maxTotalBlobSize ?? defaultPlanRules.maxTotalBlobSize }),
  }),
});

UserPlan.implement({
  isTypeOf: isTypeOf(TableCode.USER_PLANS),
  fields: (t) => ({
    id: t.exposeID('id'),
    fee: t.exposeInt('fee'),
    billingCycle: t.expose('billingCycle', { type: UserPlanBillingCycle }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    expiresAt: t.expose('expiresAt', { type: 'DateTime' }),
    state: t.expose('state', { type: UserPlanState }),

    plan: t.expose('planId', { type: Plan }),

    nextInvoice: t.field({
      type: PaymentInvoice,
      nullable: true,
      resolve: async (self) => {
        return await db
          .select()
          .from(PaymentInvoices)
          .where(and(eq(PaymentInvoices.userId, self.userId), eq(PaymentInvoices.state, PaymentInvoiceState.UPCOMING)))
          .then(first);
      },
    }),
  }),
});

CreditCode.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    code: t.exposeString('code'),
    amount: t.exposeInt('amount'),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  creditCode: t.withAuth({ session: true }).field({
    type: CreditCode,
    args: {
      code: t.input.string({ validate: { schema: redeemCodeSchema } }),
    },
    resolve: async (_, args) => {
      const code = args.code.toUpperCase().replaceAll('-', '').replaceAll('O', '0').replaceAll('I', '1').replaceAll('L', '1');

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
  updatePaymentBillingKey: t.withAuth({ session: true }).fieldWithInput({
    type: PaymentBillingKey,
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

      return await db.transaction(async (tx) => {
        const billingKeys = await tx
          .update(PaymentBillingKeys)
          .set({ state: PaymentBillingKeyState.DEACTIVATED })
          .where(and(eq(PaymentBillingKeys.userId, ctx.session.userId), eq(PaymentBillingKeys.state, PaymentBillingKeyState.ACTIVE)))
          .returning({ billingKey: PaymentBillingKeys.billingKey });

        for (const billingKey of billingKeys) {
          await portone.deleteBillingKey({ billingKey: billingKey.billingKey });
        }

        return await tx
          .insert(PaymentBillingKeys)
          .values({
            userId: ctx.session.userId,
            name: `${result.cardName} ${input.cardNumber.slice(-4)}`,
            billingKey: result.billingKey,
          })
          .returning()
          .then(firstOrThrow);
      });
    },
  }),

  enrollPlan: t.withAuth({ session: true }).fieldWithInput({
    type: UserPlan,
    input: {
      planId: t.input.id({ validate: validateDbId(TableCode.PLANS) }),
      billingCycle: t.input.field({ type: UserPlanBillingCycle }),
    },
    resolve: async (_, { input }, ctx) => {
      const existingUserPlan = await db
        .select({
          planId: UserPlans.planId,
          expiresAt: UserPlans.expiresAt,
          billingMethod: UserPlans.billingMethod,
        })
        .from(UserPlans)
        .where(eq(UserPlans.userId, ctx.session.userId))
        .then(first);

      const plan = await db
        .select({ id: Plans.id, name: Plans.name, fee: Plans.fee })
        .from(Plans)
        .where(and(eq(Plans.id, input.planId), eq(Plans.availability, PlanAvailability.PUBLIC)))
        .then(firstOrThrow);

      const enrolledAt = dayjs.kst().startOf('day');
      const paymentAmount = calculatePaymentAmount(input.billingCycle, plan.fee);

      if (existingUserPlan) {
        if (existingUserPlan.planId !== input.planId || existingUserPlan.billingMethod !== UserPlanBillingMethod.BILLING_KEY_AND_CREDIT) {
          // TODO: 다른 플랜 자체가 없으니까... 나중에 다른 플랜이 생기면 생각해보기
          throw new TypieError({ code: 'plan_already_enrolled' });
        }

        if (existingUserPlan.expiresAt.isAfter(dayjs())) {
          return await db.transaction(async (tx) => {
            const userPlan = await tx
              .update(UserPlans)
              .set({
                state: UserPlanState.ACTIVE,
                fee: plan.fee,
                billingCycle: input.billingCycle,
              })
              .where(eq(UserPlans.userId, ctx.session.userId))
              .returning()
              .then(firstOrThrow);

            await tx
              .update(PaymentInvoices)
              .set({
                amount: paymentAmount,
              })
              .where(and(eq(PaymentInvoices.userId, ctx.session.userId), eq(PaymentInvoices.state, PaymentInvoiceState.UPCOMING)));

            return userPlan;
          });
        }
      }

      const unpaidInvoice = await db
        .select({ id: PaymentInvoices.id })
        .from(PaymentInvoices)
        .where(
          and(
            eq(PaymentInvoices.userId, ctx.session.userId),
            eq(PaymentInvoices.state, PaymentInvoiceState.UPCOMING),
            lt(PaymentInvoices.billingAt, dayjs()),
          ),
        )
        .then(first);

      if (unpaidInvoice) {
        throw new TypieError({ code: 'unpaid_invoice_exists' });
      }

      const { userPlan } = await db.transaction(async (tx) => {
        const userPlan = await tx
          .insert(UserPlans)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            fee: plan.fee,
            billingCycle: input.billingCycle,
            billingMethod: UserPlanBillingMethod.BILLING_KEY_AND_CREDIT,
            expiresAt: enrolledAt,
          })
          .onConflictDoUpdate({
            target: [UserPlans.userId],
            set: {
              planId: plan.id,
              fee: plan.fee,
              billingCycle: input.billingCycle,
              expiresAt: enrolledAt,
              state: UserPlanState.ACTIVE,
            },
          })
          .returning()
          .then(firstOrThrow);

        const invoice = await tx
          .insert(PaymentInvoices)
          .values({
            userId: ctx.session.userId,
            amount: paymentAmount,
            billingAt: enrolledAt,
            state: PaymentInvoiceState.UPCOMING,
          })
          .returning({ id: PaymentInvoices.id })
          .then(firstOrThrow);

        const payInvoiceResult = await payInvoice({
          tx,
          invoiceId: invoice.id,
          makeRecordWhenFail: false,
        });

        if (payInvoiceResult.status === 'failed') {
          // 에러 던져서 tx 롤백 일으키기 (여기서 실패하면 로그 쌓을 필요 X)
          throw new TypieError({ code: 'payment_failed', message: payInvoiceResult.message });
        }

        return { userPlan };
      });

      return userPlan;
    },
  }),

  cancelPlan: t.withAuth({ session: true }).field({
    type: UserPlan,
    resolve: async (_, __, ctx) => {
      const userPlan = await db
        .select({ expiresAt: UserPlans.expiresAt, billingMethod: UserPlans.billingMethod })
        .from(UserPlans)
        .where(and(eq(UserPlans.userId, ctx.session.userId), eq(UserPlans.state, UserPlanState.ACTIVE)))
        .then(firstOrThrow);

      if (userPlan.billingMethod === UserPlanBillingMethod.APP_STORE || userPlan.billingMethod === UserPlanBillingMethod.GOOGLE_PLAY) {
        throw new TypieError({ code: 'iap_subscription' });
      }

      return await db.transaction(async (tx) => {
        await tx
          .update(PaymentInvoices)
          .set({ state: PaymentInvoiceState.CANCELED })
          .where(and(eq(PaymentInvoices.userId, ctx.session.userId), eq(PaymentInvoices.state, PaymentInvoiceState.UPCOMING)));

        if (userPlan.expiresAt.isAfter(dayjs())) {
          return await tx
            .update(UserPlans)
            .set({ state: UserPlanState.CANCELED })
            .where(and(eq(UserPlans.userId, ctx.session.userId), eq(UserPlans.state, UserPlanState.ACTIVE)))
            .returning()
            .then(firstOrThrow);
        } else {
          return await tx.delete(UserPlans).where(eq(UserPlans.userId, ctx.session.userId)).returning().then(firstOrThrow);
        }
      });
    },
  }),

  enrollPlanWithInAppPurchase: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      store: t.input.field({ type: InAppPurchaseStore }),
      data: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      let subscriptionId: string;
      let planId: string;
      let expiresAt: Dayjs;
      let billingCycle: UserPlanBillingCycle;

      if (input.store === InAppPurchaseStore.APP_STORE) {
        const transaction = await appstore.getSubscription({
          environment: production ? 'production' : 'sandbox',
          transactionId: input.data,
        });

        await slack.sendMessage({ channel: 'iap', message: JSON.stringify({ source: 'mutation/appstore', transaction }, null, 2) });

        if (!transaction.originalTransactionId) {
          throw new TypieError({ code: 'subscription_not_active' });
        }

        subscriptionId = transaction.originalTransactionId;
        expiresAt = dayjs(transaction.expiresDate);
        ({ planId, billingCycle } = appstore.getPlanInfoByProductId(transaction.productId));
      } else if (input.store === InAppPurchaseStore.GOOGLE_PLAY) {
        const subscription = await googleplay.getSubscription({
          purchaseToken: input.data,
        });

        await slack.sendMessage({ channel: 'iap', message: JSON.stringify({ source: 'mutation/googleplay', subscription }, null, 2) });

        if (subscription.subscriptionState !== 'SUBSCRIPTION_STATE_ACTIVE') {
          throw new TypieError({ code: 'subscription_not_active' });
        }

        subscriptionId = input.data;
        expiresAt = dayjs(subscription.lineItems?.[0].expiryTime);
        planId = googleplay.getPlanIdByProductId(subscription.lineItems?.[0].productId);
        billingCycle = googleplay.getPlanBillingCycleByBasePlanId(subscription.lineItems?.[0].offerDetails?.basePlanId);
      } else {
        throw new Error('Should not reach here');
      }

      if (expiresAt.isBefore(dayjs())) {
        throw new TypieError({ code: 'subscription_expired' });
      }

      const plan = await db
        .select({
          id: Plans.id,
          fee: Plans.fee,
        })
        .from(Plans)
        .where(eq(Plans.id, planId))
        .then(firstOrThrow);

      const existingUserPlan = await db
        .select({
          planId: UserPlans.planId,
          billingMethod: UserPlans.billingMethod,
        })
        .from(UserPlans)
        .leftJoin(UserIAPSubscriptions, eq(UserPlans.userId, UserIAPSubscriptions.userId))
        .where(eq(UserPlans.userId, ctx.session.userId))
        .then(first);

      const iapSubscription = await db
        .select({
          userId: UserIAPSubscriptions.userId,
        })
        .from(UserIAPSubscriptions)
        .where(and(eq(UserIAPSubscriptions.store, input.store), eq(UserIAPSubscriptions.subscriptionId, subscriptionId)))
        .then(first);

      if (existingUserPlan && (existingUserPlan.planId !== plan.id || existingUserPlan.billingMethod !== input.store)) {
        throw new TypieError({ code: 'already_enrolled' });
      }

      if (iapSubscription && iapSubscription.userId !== ctx.session.userId) {
        throw new TypieError({ code: 'already_enrolled_by_other_account' });
      }

      return await db.transaction(async (tx) => {
        await tx
          .insert(UserPlans)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            fee: plan.fee,
            billingCycle,
            billingMethod: input.store,
            expiresAt,
          })
          .onConflictDoUpdate({
            target: [UserPlans.userId],
            set: {
              billingCycle,
            },
          });

        await tx
          .insert(UserIAPSubscriptions)
          .values({
            userId: ctx.session.userId,
            store: input.store,
            subscriptionId,
          })
          .onConflictDoUpdate({
            target: [UserIAPSubscriptions.userId],
            set: {
              store: input.store,
              subscriptionId,
            },
          });

        return true;
      });
    },
  }),
}));
