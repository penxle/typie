import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { db, first, firstOrThrow, PaymentBillingKeys, PaymentInvoices, PaymentRecords, UserPaymentCredits, UserPlans, Users } from '@/db';
import { PaymentBillingKeyState, PaymentInvoiceState, PaymentMethodType, PaymentRecordState, UserPlanBillingCycle } from '@/enums';
import * as portone from '@/external/portone';
import type { Transaction } from '@/db';

export const calculatePaymentAmount = (billingCycle: UserPlanBillingCycle, fee: number) => {
  return match(billingCycle)
    .with(UserPlanBillingCycle.MONTHLY, () => fee)
    .with(UserPlanBillingCycle.YEARLY, () => fee * 10)
    .exhaustive();
};

export const getNextPaymentDate = (billingCycle: UserPlanBillingCycle, enrolledAt: dayjs.Dayjs, previousPaymentDate?: dayjs.Dayjs) => {
  const date = previousPaymentDate?.kst() ?? dayjs.kst();
  const startOfMonth = date.kst().startOf('month').startOf('day');

  const nextCycleMonth = match(billingCycle)
    .with(UserPlanBillingCycle.MONTHLY, () => startOfMonth.add(1, 'month'))
    .with(UserPlanBillingCycle.YEARLY, () => startOfMonth.add(1, 'year'))
    .exhaustive();

  return nextCycleMonth.date(Math.min(enrolledAt.kst().date(), nextCycleMonth.daysInMonth()));
};

type PayInvoiceParams = {
  invoiceId: string;
  tx: Transaction;
  makeRecordWhenFail: boolean;
};
export const payInvoice = async ({ invoiceId, tx, makeRecordWhenFail }: PayInvoiceParams) => {
  const { invoice, billingKey, userPlan } = await tx
    .select({
      invoice: {
        id: PaymentInvoices.id,
        userId: PaymentInvoices.userId,
        amount: PaymentInvoices.amount,
        billingAt: PaymentInvoices.billingAt,
      },
      billingKey: {
        id: PaymentBillingKeys.id,
        billingKey: PaymentBillingKeys.billingKey,
      },
      userPlan: {
        id: UserPlans.id,
        fee: UserPlans.fee,
        billingCycle: UserPlans.billingCycle,
        createdAt: UserPlans.createdAt,
      },
    })
    .from(PaymentInvoices)
    .innerJoin(Users, eq(PaymentInvoices.userId, Users.id))
    .innerJoin(
      PaymentBillingKeys,
      and(eq(Users.id, PaymentBillingKeys.userId), eq(PaymentBillingKeys.state, PaymentBillingKeyState.ACTIVE)),
    )
    .innerJoin(UserPlans, eq(Users.id, UserPlans.userId))
    .where(eq(PaymentInvoices.id, invoiceId))
    .for('no key update')
    .then(firstOrThrow);

  const user = await tx
    .select({ id: Users.id, name: Users.name, email: Users.email })
    .from(Users)
    .where(eq(Users.id, invoice.userId))
    .then(firstOrThrow);

  // 다음 invoice 미리 생성

  const nextPaymentDate = getNextPaymentDate(userPlan.billingCycle, userPlan.createdAt, invoice.billingAt);
  const nextPaymentAmount = calculatePaymentAmount(userPlan.billingCycle, userPlan.fee);

  await tx.insert(PaymentInvoices).values({
    userId: user.id,
    amount: nextPaymentAmount,
    billingAt: nextPaymentDate,
    state: PaymentInvoiceState.UPCOMING,
  });

  // 플랜 연장

  await tx
    .update(UserPlans)
    .set({
      expiresAt: nextPaymentDate,
    })
    .where(eq(UserPlans.id, userPlan.id));

  await tx
    .update(PaymentInvoices)
    .set({
      state: PaymentInvoiceState.PAID,
    })
    .where(eq(PaymentInvoices.id, invoice.id));

  // 크레딧으로 먼저 결제

  const paymentCredit = await tx
    .select({ id: UserPaymentCredits.id, amount: UserPaymentCredits.amount })
    .from(UserPaymentCredits)
    .where(eq(UserPaymentCredits.userId, user.id))
    .for('update')
    .then(first);

  let creditPaymentAmount = 0;

  if (paymentCredit) {
    creditPaymentAmount = Math.min(paymentCredit.amount, invoice.amount);
    if (creditPaymentAmount > 0) {
      await tx
        .update(UserPaymentCredits)
        .set({ amount: paymentCredit.amount - creditPaymentAmount })
        .where(eq(UserPaymentCredits.id, paymentCredit.id));

      await tx.insert(PaymentRecords).values({
        invoiceId: invoice.id,
        methodType: PaymentMethodType.CREDIT,
        methodId: paymentCredit.id,
        state: PaymentRecordState.SUCCEEDED,
        amount: creditPaymentAmount,
      });
    }
  }

  // 빌링키 결제

  const billingKeyPaymentAmount = invoice.amount - creditPaymentAmount;

  if (billingKeyPaymentAmount > 0) {
    const paymentResult = await portone.makePayment({
      paymentId: invoice.id,
      billingKey: billingKey.billingKey,
      customerName: user.name,
      customerEmail: user.email,
      orderName: '타이피 정기결제',
      amount: billingKeyPaymentAmount,
    });

    if (paymentResult.status === 'failed') {
      if (makeRecordWhenFail) {
        // 롤백되도 로그는 남아야하니 트랜잭션 밖에서
        await db.insert(PaymentRecords).values({
          invoiceId: invoice.id,
          methodType: PaymentMethodType.BILLING_KEY,
          methodId: billingKey.id,
          state: PaymentRecordState.FAILED,
          amount: billingKeyPaymentAmount,
        });
      }
      return {
        status: 'failed' as const,
        message: paymentResult.message,
      };
    }

    await tx.insert(PaymentRecords).values({
      invoiceId: invoice.id,
      methodType: PaymentMethodType.BILLING_KEY,
      methodId: billingKey.id,
      state: PaymentRecordState.SUCCEEDED,
      amount: billingKeyPaymentAmount,
      receiptUrl: paymentResult.receiptUrl,
    });
  }

  return {
    status: 'succeeded' as const,
  };
};
