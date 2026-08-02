import { PaymentInvoiceState, PaymentOutcome } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, desc, eq, lt } from 'drizzle-orm';
import { db, first, firstOrThrow, PaymentInvoices, PaymentRecords, Plans, Subscriptions, Users } from '#/db/index.ts';
import { sendEmail } from '#/email/index.ts';
import {
  SubscriptionExpiredEmail,
  SubscriptionExpiringEmail,
  SubscriptionGracePeriodEmail,
  SubscriptionWaivedEmail,
} from '#/email/templates/index.ts';
import { env } from '#/env.ts';
import { defineJob } from '../types.ts';

export const SendSubscriptionGracePeriodEmailJob = defineJob('email:subscription-grace-period', async (subscriptionId: string) => {
  const subscription = await db
    .select({
      userId: Subscriptions.userId,
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

    reason = (paymentRecord?.data as { message?: string } | undefined)?.message;
  }

  await sendEmail({
    subject: '[타이피] 결제 정보 확인이 필요해요',
    recipient: subscription.user.email,
    body: SubscriptionGracePeriodEmail({
      userName: subscription.user.name,
      planName: subscription.plan.name,
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

    reason = (paymentRecord?.data as { message?: string } | undefined)?.message;
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

export const SendSubscriptionWaivedEmailJob = defineJob('email:subscription-waived', async (subscriptionId: string) => {
  const subscription = await db
    .select({
      currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
      plan: { name: Plans.name, interval: Plans.interval },
      user: { name: Users.name, email: Users.email },
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .innerJoin(Users, eq(Subscriptions.userId, Users.id))
    .where(eq(Subscriptions.id, subscriptionId))
    .then(firstOrThrow);

  const isYearly = subscription.plan.interval === 'YEARLY';

  // 메일이 말하는 기간은 미사용 판정 대상이었던 직전 주기다 — WAIVED 인보이스의 서비스 주기는 그 판정으로 면제된 다음 주기다.
  const waivedInvoice = await db
    .select({ servicePeriodStartsAt: PaymentInvoices.servicePeriodStartsAt })
    .from(PaymentInvoices)
    .where(and(eq(PaymentInvoices.subscriptionId, subscriptionId), eq(PaymentInvoices.state, PaymentInvoiceState.WAIVED)))
    .orderBy(desc(PaymentInvoices.createdAt))
    .limit(1)
    .then(first);

  // 판정 대상 주기의 기록은 상태를 가리지 않는다(PAID·WAIVED 모두 그 주기의 인보이스다).
  const evaluatedInvoice = waivedInvoice
    ? await db
        .select({ servicePeriodStartsAt: PaymentInvoices.servicePeriodStartsAt, servicePeriodEndsAt: PaymentInvoices.servicePeriodEndsAt })
        .from(PaymentInvoices)
        .where(
          and(
            eq(PaymentInvoices.subscriptionId, subscriptionId),
            lt(PaymentInvoices.servicePeriodStartsAt, waivedInvoice.servicePeriodStartsAt),
          ),
        )
        .orderBy(desc(PaymentInvoices.servicePeriodStartsAt))
        .limit(1)
        .then(first)
    : null;

  // 선행 인보이스가 없는 행(백필 이전 레거시·수기 생성)은 표시할 기록이 없어 주기 컬럼에서 역산한다 — 월말 앵커에서 하루씩 어긋날 수 있다.
  const waivedStart =
    evaluatedInvoice?.servicePeriodStartsAt ?? subscription.currentPeriodStartsAt.subtract(1, isYearly ? 'year' : 'month');
  const waivedEnd = evaluatedInvoice?.servicePeriodEndsAt ?? subscription.currentPeriodStartsAt;
  const period = isYearly ? '올해' : '이번 달';

  const startStr = waivedStart.kst().format('YYYY년 M월 D일');
  const endStr =
    waivedStart.kst().year() === waivedEnd.kst().year() ? waivedEnd.kst().format('M월 D일') : waivedEnd.kst().format('YYYY년 M월 D일');

  await sendEmail({
    subject: `[타이피] ${period}은 구독료 결제를 건너뛰었어요`,
    recipient: subscription.user.email,
    body: SubscriptionWaivedEmail({
      userName: subscription.user.name,
      interval: subscription.plan.interval as 'MONTHLY' | 'YEARLY',
      waivedStart: startStr,
      waivedEnd: endStr,
    }),
  });
});
