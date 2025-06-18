import dayjs from 'dayjs';
import { and, desc, eq } from 'drizzle-orm';
import { SUBSCRIPTION_GRACE_DAYS } from '@/const';
import { db, first, firstOrThrow, PaymentInvoices, PaymentRecords, Plans, Subscriptions, Users } from '@/db';
import { sendEmail } from '@/email';
import { SubscriptionExpiredEmail, SubscriptionExpiringEmail, SubscriptionGracePeriodEmail } from '@/email/templates';
import { PaymentInvoiceState, PaymentOutcome } from '@/enums';
import { env } from '@/env';
import { defineJob } from '../types';

export const SendSubscriptionGracePeriodEmailJob = defineJob('email:subscription-grace-period', async (subscriptionId: string) => {
  const subscription = await db
    .select({
      userId: Subscriptions.userId,
      expiresAt: Subscriptions.expiresAt,
      plan: { name: Plans.name },
      user: { name: Users.name, email: Users.email },
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .innerJoin(Users, eq(Subscriptions.userId, Users.id))
    .where(eq(Subscriptions.id, subscriptionId))
    .then(firstOrThrow);

  const invoice = await db
    .select({ id: PaymentInvoices.id })
    .from(PaymentInvoices)
    .where(and(eq(PaymentInvoices.subscriptionId, subscriptionId), eq(PaymentInvoices.state, PaymentInvoiceState.OVERDUE)))
    .orderBy(desc(PaymentInvoices.createdAt))
    .limit(1)
    .then(first);

  let reason;
  if (invoice) {
    const paymentRecord = await db
      .select({ data: PaymentRecords.data })
      .from(PaymentRecords)
      .where(and(eq(PaymentRecords.invoiceId, invoice.id), eq(PaymentRecords.outcome, PaymentOutcome.FAILURE)))
      .orderBy(desc(PaymentRecords.createdAt))
      .limit(1)
      .then(first);

    reason = (paymentRecord?.data as { message?: string }).message;
  }

  const gracePeriodEndsAt = subscription.expiresAt.add(SUBSCRIPTION_GRACE_DAYS, 'days');

  await sendEmail({
    subject: '[타이피] 결제 정보 확인이 필요해요',
    recipient: subscription.user.email,
    body: SubscriptionGracePeriodEmail({
      userName: subscription.user.name,
      planName: subscription.plan.name,
      gracePeriodEndsAt: gracePeriodEndsAt.kst().format('YYYY년 M월 D일'),
      dashboardUrl: env.WEBSITE_URL,
      reason: reason || '결제 실패',
    }),
  });
});

export const SendSubscriptionExpiringEmailJob = defineJob('email:subscription-expiring', async (subscriptionId: string) => {
  const subscription = await db
    .select({
      plan: { name: Plans.name },
      user: { name: Users.name, email: Users.email },
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .innerJoin(Users, eq(Subscriptions.userId, Users.id))
    .where(eq(Subscriptions.id, subscriptionId))
    .then(firstOrThrow);

  const invoice = await db
    .select({ id: PaymentInvoices.id })
    .from(PaymentInvoices)
    .where(and(eq(PaymentInvoices.subscriptionId, subscriptionId), eq(PaymentInvoices.state, PaymentInvoiceState.OVERDUE)))
    .orderBy(desc(PaymentInvoices.createdAt))
    .limit(1)
    .then(first);

  let reason;
  if (invoice) {
    const paymentRecord = await db
      .select({ data: PaymentRecords.data })
      .from(PaymentRecords)
      .where(and(eq(PaymentRecords.invoiceId, invoice.id), eq(PaymentRecords.outcome, PaymentOutcome.FAILURE)))
      .orderBy(desc(PaymentRecords.createdAt))
      .limit(1)
      .then(first);

    reason = (paymentRecord?.data as { message?: string }).message;
  }

  await sendEmail({
    subject: '[타이피] 곧 구독이 중단돼요',
    recipient: subscription.user.email,
    body: SubscriptionExpiringEmail({
      userName: subscription.user.name,
      planName: subscription.plan.name,
      dashboardUrl: env.WEBSITE_URL,
      reason: reason || '결제 실패',
    }),
  });
});

export const SendSubscriptionExpiredEmailJob = defineJob('email:subscription-expired', async (subscriptionId: string) => {
  const subscription = await db
    .select({
      plan: { name: Plans.name },
      user: { name: Users.name, email: Users.email },
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .innerJoin(Users, eq(Subscriptions.userId, Users.id))
    .where(eq(Subscriptions.id, subscriptionId))
    .then(firstOrThrow);

  await sendEmail({
    subject: '[타이피] 구독이 중단되었어요',
    recipient: subscription.user.email,
    body: SubscriptionExpiredEmail({
      userName: subscription.user.name,
      planName: subscription.plan.name,
      expiredAt: dayjs.kst().format('YYYY년 M월 D일'),
    }),
  });
});
