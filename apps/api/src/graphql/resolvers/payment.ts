import dayjs from 'dayjs';
import { and, eq, gt, ne } from 'drizzle-orm';
import { defaultPlanRules } from '@/const';
import {
  CreditCodes,
  db,
  first,
  firstOrThrow,
  firstOrThrowWith,
  PaymentInvoices,
  Plans,
  Subscriptions,
  TableCode,
  UserBillingKeys,
  UserInAppPurchases,
  validateDbId,
} from '@/db';
import { CreditCodeState, InAppPurchaseStore, PaymentInvoiceState, PlanAvailability, PlanInterval, SubscriptionState } from '@/enums';
import { NotFoundError, TypieError } from '@/errors';
import * as appstore from '@/external/appstore';
import * as googleplay from '@/external/googleplay';
import * as portone from '@/external/portone';
import { getSubscriptionExpiresAt, payInvoiceWithBillingKey } from '@/utils';
import { delay } from '@/utils/promise';
import { cardSchema, redeemCodeSchema } from '@/validation';
import { builder } from '../builder';
import { CreditCode, isTypeOf, PaymentInvoice, Plan, PlanRule, Subscription, UserBillingKey } from '../objects';

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
    dueAt: t.expose('dueAt', { type: 'DateTime' }),
  }),
});

Plan.implement({
  isTypeOf: isTypeOf(TableCode.PLANS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    fee: t.exposeInt('fee'),
    interval: t.expose('interval', { type: PlanInterval }),
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
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  creditCode: t.withAuth({ session: true }).field({
    type: CreditCode,
    args: { code: t.input.string({ validate: { schema: redeemCodeSchema } }) },
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

      return await db.transaction(async (tx) => {
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
          })
          .returning()
          .then(firstOrThrow);
      });
    },
  }),

  subscribePlanWithBillingKey: t.withAuth({ session: true }).fieldWithInput({
    type: Subscription,
    input: { planId: t.input.id({ validate: validateDbId(TableCode.PLANS) }) },
    resolve: async (_, { input }, ctx) => {
      const existingSubscription = await db
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .where(and(eq(Subscriptions.userId, ctx.session.userId), ne(Subscriptions.state, SubscriptionState.EXPIRED)))
        .then(first);

      if (existingSubscription) {
        throw new TypieError({ code: 'subscription_already_exists' });
      }

      const plan = await db
        .select({ id: Plans.id, name: Plans.name, fee: Plans.fee, interval: Plans.interval })
        .from(Plans)
        .where(and(eq(Plans.id, input.planId), eq(Plans.availability, PlanAvailability.BILLING_KEY)))
        .then(firstOrThrow);

      const startsAt = dayjs();
      const expiresAt = getSubscriptionExpiresAt(startsAt, plan.interval);

      return await db.transaction(async (tx) => {
        const subscription = await tx
          .insert(Subscriptions)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            startsAt,
            expiresAt,
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
      const activeSubscription = await db
        .select({ id: Subscriptions.id, expiresAt: Subscriptions.expiresAt })
        .from(Subscriptions)
        .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.ACTIVE)))
        .then(firstOrThrow);

      const plan = await db
        .select({ id: Plans.id, fee: Plans.fee, interval: Plans.interval })
        .from(Plans)
        .where(and(eq(Plans.id, input.planId), eq(Plans.availability, PlanAvailability.BILLING_KEY)))
        .then(firstOrThrow);

      const startsAt = activeSubscription.expiresAt;
      const expiresAt = getSubscriptionExpiresAt(startsAt, plan.interval);

      return await db.transaction(async (tx) => {
        await tx.update(Subscriptions).set({ state: SubscriptionState.WILL_EXPIRE }).where(eq(Subscriptions.id, activeSubscription.id));

        return await tx
          .insert(Subscriptions)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            startsAt,
            expiresAt,
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
      const willExpireSubscription = await db
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE)))
        .then(firstOrThrow);

      const willActivateSubscription = await db
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)))
        .then(firstOrThrow);

      return await db.transaction(async (tx) => {
        await tx.delete(Subscriptions).where(eq(Subscriptions.id, willActivateSubscription.id));

        return await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.ACTIVE })
          .where(eq(Subscriptions.id, willExpireSubscription.id))
          .returning()
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
      const existingSubscription = await db
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
        .where(
          and(
            eq(Subscriptions.userId, ctx.session.userId),
            ne(Subscriptions.state, SubscriptionState.EXPIRED),
            ne(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
          ),
        )
        .then(first);

      if (existingSubscription) {
        throw new TypieError({ code: 'subscription_already_exists' });
      }

      let identifier: string;
      let planId: string;
      let startsAt: dayjs.Dayjs;
      let expiresAt: dayjs.Dayjs;

      if (input.store === InAppPurchaseStore.APP_STORE) {
        const subscription = await appstore.getSubscription(input.data);

        if (!subscription.productId || !subscription.originalTransactionId || !subscription.purchaseDate || !subscription.expiresDate) {
          throw new Error('required fields are missing');
        }

        identifier = subscription.originalTransactionId;
        planId = subscription.productId.toUpperCase();
        startsAt = dayjs(subscription.purchaseDate);
        expiresAt = dayjs(subscription.expiresDate);
      } else if (input.store === InAppPurchaseStore.GOOGLE_PLAY) {
        const subscription = await googleplay.getSubscription({
          purchaseToken: input.data,
        });

        if (subscription.subscriptionState !== 'SUBSCRIPTION_STATE_ACTIVE') {
          throw new Error('subscriptionState is not active');
        }

        const item = subscription.lineItems?.[0];
        if (!item || !item.offerDetails?.basePlanId || !subscription.startTime || !item.expiryTime) {
          throw new Error('required fields are missing');
        }

        identifier = input.data;
        planId = item.offerDetails.basePlanId.toUpperCase();
        startsAt = dayjs(subscription.startTime);
        expiresAt = dayjs(item.expiryTime);
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
            state: SubscriptionState.ACTIVE,
          })
          .onConflictDoUpdate({
            target: [Subscriptions.userId],
            set: { planId, startsAt, expiresAt },
          })
          .returning()
          .then(firstOrThrow);
      });
    },
  }),

  scheduleSubscriptionCancellation: t.withAuth({ session: true }).field({
    type: Subscription,
    resolve: async (_, __, ctx) => {
      const activeSubscription = await db
        .select({ id: Subscriptions.id })
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

      const willActivateSubscription = await db
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)))
        .then(first);

      return await db.transaction(async (tx) => {
        if (willActivateSubscription) {
          await tx.delete(Subscriptions).where(eq(Subscriptions.id, willActivateSubscription.id));
        }

        return await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.WILL_EXPIRE })
          .where(eq(Subscriptions.id, activeSubscription.id))
          .returning()
          .then(firstOrThrow);
      });
    },
  }),

  cancelSubscriptionCancellation: t.withAuth({ session: true }).field({
    type: Subscription,
    resolve: async (_, __, ctx) => {
      const willExpireSubscription = await db
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .where(and(eq(Subscriptions.userId, ctx.session.userId), eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE)))
        .then(firstOrThrow);

      return await db.transaction(async (tx) => {
        return await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.ACTIVE })
          .where(eq(Subscriptions.id, willExpireSubscription.id))
          .returning()
          .then(firstOrThrow);
      });
    },
  }),
}));
