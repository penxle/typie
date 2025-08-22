import * as Sentry from '@sentry/bun';
import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { first, firstOrThrow, PaymentInvoices, PaymentRecords, UserBillingKeys, UserPaymentCredits, Users } from '@/db';
import { PaymentOutcome, PlanInterval } from '@/enums';
import * as portone from '@/external/portone';
import type { Transaction } from '@/db';

export const getSubscriptionExpiresAt = (startsAt: dayjs.Dayjs, interval: PlanInterval) => {
  if (interval === PlanInterval.LIFETIME) {
    return dayjs('9999-12-31T00:00:00.000Z');
  }

  const startOfMonth = startsAt.kst().startOf('month').startOf('day');
  const expiresAtMonth = match(interval)
    .with(PlanInterval.MONTHLY, () => startOfMonth.add(1, 'month'))
    .with(PlanInterval.YEARLY, () => startOfMonth.add(1, 'year'))
    .exhaustive();

  return expiresAtMonth.date(Math.min(startsAt.kst().date(), expiresAtMonth.daysInMonth()));
};

export const payInvoiceWithBillingKey = async (tx: Transaction, invoiceId: string) => {
  const invoice = await tx
    .select({ id: PaymentInvoices.id, userId: PaymentInvoices.userId, amount: PaymentInvoices.amount })
    .from(PaymentInvoices)
    .where(eq(PaymentInvoices.id, invoiceId))
    .for('no key update')
    .then(firstOrThrow);

  const user = await tx
    .select({ id: Users.id, name: Users.name, email: Users.email })
    .from(Users)
    .where(eq(Users.id, invoice.userId))
    .then(firstOrThrow);

  const billingKey = await tx
    .select({ billingKey: UserBillingKeys.billingKey })
    .from(UserBillingKeys)
    .where(eq(UserBillingKeys.userId, invoice.userId))
    .for('no key update')
    .then(firstOrThrow);

  const paymentCredit = await tx
    .select({ id: UserPaymentCredits.id, amount: UserPaymentCredits.amount })
    .from(UserPaymentCredits)
    .where(eq(UserPaymentCredits.userId, invoice.userId))
    .for('no key update')
    .then(first);

  const creditAmount = Math.min(paymentCredit?.amount ?? 0, invoice.amount);
  const billingAmount = invoice.amount - creditAmount;

  try {
    if (billingAmount > 0) {
      const result = await portone.payWithBillingKey({
        paymentId: invoice.id,
        billingKey: billingKey.billingKey,
        customerName: user.name,
        customerEmail: user.email,
        orderName: '타이피 정기결제',
        amount: billingAmount,
      });

      if (result.status === 'succeeded') {
        await tx.insert(PaymentRecords).values({
          invoiceId: invoice.id,
          outcome: PaymentOutcome.SUCCESS,
          billingAmount,
          creditAmount,
          data: { approvalNumber: result.approvalNumber, receiptUrl: result.receiptUrl },
        });

        if (paymentCredit && creditAmount > 0) {
          await tx
            .update(UserPaymentCredits)
            .set({ amount: paymentCredit.amount - creditAmount })
            .where(eq(UserPaymentCredits.id, paymentCredit.id));
        }

        return true;
      } else if (result.status === 'failed') {
        await tx.insert(PaymentRecords).values({
          invoiceId: invoice.id,
          outcome: PaymentOutcome.FAILURE,
          billingAmount,
          creditAmount,
          data: { code: result.code, message: result.message },
        });

        return false;
      }
    } else if (paymentCredit && creditAmount > 0) {
      await tx.insert(PaymentRecords).values({
        invoiceId: invoice.id,
        outcome: PaymentOutcome.SUCCESS,
        billingAmount,
        creditAmount,
        data: {},
      });

      await tx
        .update(UserPaymentCredits)
        .set({ amount: paymentCredit.amount - creditAmount })
        .where(eq(UserPaymentCredits.id, paymentCredit.id));

      return true;
    }

    throw new Error('Invalid billing amount');
  } catch (err) {
    await tx.insert(PaymentRecords).values({
      invoiceId: invoice.id,
      outcome: PaymentOutcome.FAILURE,
      billingAmount,
      creditAmount,
      data: { message: err instanceof Error ? err.message : String(err) },
    });

    Sentry.captureException(err);

    return false;
  }
};
