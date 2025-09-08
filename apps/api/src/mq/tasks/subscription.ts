import dayjs from 'dayjs';
import { and, eq, lte } from 'drizzle-orm';
import { SUBSCRIPTION_GRACE_DAYS } from '@/const';
import { db, firstOrThrow, PaymentInvoices, Plans, Subscriptions } from '@/db';
import { PaymentInvoiceState, PlanAvailability, SubscriptionState } from '@/enums';
import { getSubscriptionExpiresAt, payInvoiceWithBillingKey } from '@/utils';
import { enqueueJob } from '../publisher';
import { defineCron, defineJob } from '../types';

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
        await enqueueJob('subscription:renewal:plan-change', subscription.id);
      }

      const cancelSubscriptions = await tx
        .select({ id: Subscriptions.id })
        .from(Subscriptions)
        .where(and(eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE), lte(Subscriptions.expiresAt, now)));

      for (const subscription of cancelSubscriptions) {
        await enqueueJob('subscription:renewal:cancel', subscription.id);
      }
    },
    { isolationLevel: 'repeatable read' },
  );
});

export const SubscriptionRenewalInitialJob = defineJob('subscription:renewal:initial', async (subscriptionId: string) => {
  await db.transaction(async (tx) => {
    const subscription = await tx
      .select({
        id: Subscriptions.id,
        userId: Subscriptions.userId,
        expiresAt: Subscriptions.expiresAt,
        plan: { fee: Plans.fee, interval: Plans.interval },
      })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(Subscriptions.id, subscriptionId))
      .then(firstOrThrow);

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
      await tx.update(Subscriptions).set({ expiresAt: newExpiresAt }).where(eq(Subscriptions.id, subscriptionId));
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
    const invoice = await tx
      .select({
        id: PaymentInvoices.id,
        subscription: { id: Subscriptions.id, userId: Subscriptions.userId, expiresAt: Subscriptions.expiresAt },
        plan: { interval: Plans.interval },
      })
      .from(PaymentInvoices)
      .innerJoin(Subscriptions, eq(PaymentInvoices.subscriptionId, Subscriptions.id))
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(PaymentInvoices.id, invoiceId))
      .then(firstOrThrow);

    const success = await payInvoiceWithBillingKey(tx, invoice.id);
    if (success) {
      const newExpiresAt = getSubscriptionExpiresAt(invoice.subscription.expiresAt, invoice.plan.interval);
      await tx
        .update(Subscriptions)
        .set({ expiresAt: newExpiresAt, state: SubscriptionState.ACTIVE })
        .where(eq(Subscriptions.id, invoice.subscription.id));
      await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.PAID }).where(eq(PaymentInvoices.id, invoice.id));
    } else {
      const gracePeriodEndsAt = invoice.subscription.expiresAt.add(SUBSCRIPTION_GRACE_DAYS, 'days').kst();

      if (gracePeriodEndsAt.subtract(1, 'day').isSame(dayjs.kst(), 'day')) {
        await enqueueJob('email:subscription-expiring', invoice.subscription.id, { delay: 5 * 60 * 1000 });
      }

      if (gracePeriodEndsAt.isBefore(dayjs())) {
        await tx.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(eq(Subscriptions.id, invoice.subscription.id));
        await tx.update(PaymentInvoices).set({ state: PaymentInvoiceState.CANCELED }).where(eq(PaymentInvoices.id, invoice.id));

        await enqueueJob('email:subscription-expired', invoice.subscription.id, { delay: 5 * 60 * 1000 });
      }
    }
  });
});

export const SubscriptionRenewalPlanChangeJob = defineJob('subscription:renewal:plan-change', async (subscriptionId: string) => {
  await db.transaction(async (tx) => {
    const subscription = await tx
      .select({ id: Subscriptions.id, userId: Subscriptions.userId, startsAt: Subscriptions.startsAt, plan: { fee: Plans.fee } })
      .from(Subscriptions)
      .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
      .where(eq(Subscriptions.id, subscriptionId))
      .then(firstOrThrow);

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

    const success = await payInvoiceWithBillingKey(tx, invoice.id);
    if (success) {
      await tx.update(Subscriptions).set({ state: SubscriptionState.ACTIVE }).where(eq(Subscriptions.id, subscriptionId));
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
  await db.transaction(async (tx) => {
    await tx.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(eq(Subscriptions.id, subscriptionId));
  });
});
