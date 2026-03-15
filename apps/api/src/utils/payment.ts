import * as Sentry from '@sentry/node';
import { PaymentOutcome, PlanInterval } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, eq, isNull } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { first, firstOrThrow, PaymentInvoices, PaymentRecords, Referrals, UserBillingKeys, UserPaymentCredits, Users } from '#/db/index.ts';
import * as portone from '#/external/portone.ts';
import type { Transaction } from '#/db/index.ts';

export const getSubscriptionExpiresAt = (startsAt: dayjs.Dayjs, interval: PlanInterval) => {
  if (interval === PlanInterval.LIFETIME) {
    return dayjs('9999-12-31T00:00:00.000Z');
  }

  if (interval === PlanInterval.TRIAL) {
    return startsAt.add(14, 'days');
  }

  const startOfMonth = startsAt.kst().startOf('month').startOf('day');
  const expiresAtMonth = match(interval)
    .with(PlanInterval.MONTHLY, () => startOfMonth.add(1, 'month'))
    .with(PlanInterval.YEARLY, () => startOfMonth.add(1, 'year'))
    .exhaustive();

  return expiresAtMonth.date(Math.min(startsAt.kst().date(), expiresAtMonth.daysInMonth()));
};

const compensateReferrer = async (tx: Transaction, refereeId: string) => {
  const referral = await tx
    .select({ id: Referrals.id, referrerId: Referrals.referrerId })
    .from(Referrals)
    .where(and(eq(Referrals.refereeId, refereeId), isNull(Referrals.referrerCompensatedAt)))
    .for('no key update')
    .then(first);

  if (!referral) {
    return;
  }

  const existingCredit = await tx
    .select({ id: UserPaymentCredits.id, amount: UserPaymentCredits.amount })
    .from(UserPaymentCredits)
    .where(eq(UserPaymentCredits.userId, referral.referrerId))
    .for('no key update')
    .then(first);

  if (existingCredit) {
    await tx
      .update(UserPaymentCredits)
      .set({ amount: existingCredit.amount + 4900 })
      .where(eq(UserPaymentCredits.id, existingCredit.id));
  } else {
    await tx.insert(UserPaymentCredits).values({
      userId: referral.referrerId,
      amount: 4900,
    });
  }

  await tx.update(Referrals).set({ referrerCompensatedAt: dayjs() }).where(eq(Referrals.id, referral.id));
};

type PayAmountWithBillingKeyParams = {
  paymentId: string;
  userId: string;
  orderName: string;
  amount: number;
};

type PayAmountWithBillingKeyResult =
  | { status: 'succeeded'; billingAmount: number; creditAmount: number; data: Record<string, unknown> }
  | { status: 'failed'; billingAmount: number; creditAmount: number; data: Record<string, unknown> };

export const payAmountWithBillingKey = async (
  tx: Transaction,
  params: PayAmountWithBillingKeyParams,
): Promise<PayAmountWithBillingKeyResult> => {
  const user = await tx
    .select({ id: Users.id, name: Users.name, email: Users.email })
    .from(Users)
    .where(eq(Users.id, params.userId))
    .then(firstOrThrow);

  const billingKey = await tx
    .select({ billingKey: UserBillingKeys.billingKey })
    .from(UserBillingKeys)
    .where(eq(UserBillingKeys.userId, params.userId))
    .for('no key update')
    .then(firstOrThrow);

  const paymentCredit = await tx
    .select({ id: UserPaymentCredits.id, amount: UserPaymentCredits.amount })
    .from(UserPaymentCredits)
    .where(eq(UserPaymentCredits.userId, params.userId))
    .for('no key update')
    .then(first);

  const creditAmount = Math.min(paymentCredit?.amount ?? 0, params.amount);
  const billingAmount = params.amount - creditAmount;

  try {
    if (billingAmount > 0) {
      const result = await portone.payWithBillingKey({
        paymentId: params.paymentId,
        billingKey: billingKey.billingKey,
        customerName: user.name,
        customerEmail: user.email,
        orderName: params.orderName,
        amount: billingAmount,
      });

      if (result.status === 'succeeded') {
        if (paymentCredit && creditAmount > 0) {
          await tx
            .update(UserPaymentCredits)
            .set({ amount: paymentCredit.amount - creditAmount })
            .where(eq(UserPaymentCredits.id, paymentCredit.id));
        }

        return {
          status: 'succeeded',
          billingAmount,
          creditAmount,
          data: { approvalNumber: result.approvalNumber, receiptUrl: result.receiptUrl },
        };
      } else if (result.status === 'failed') {
        return {
          status: 'failed',
          billingAmount,
          creditAmount,
          data: { code: result.code, message: result.message },
        };
      }
    } else if (paymentCredit && creditAmount > 0) {
      await tx
        .update(UserPaymentCredits)
        .set({ amount: paymentCredit.amount - creditAmount })
        .where(eq(UserPaymentCredits.id, paymentCredit.id));

      return {
        status: 'succeeded',
        billingAmount,
        creditAmount,
        data: {},
      };
    }

    throw new Error('Invalid billing amount');
  } catch (err) {
    Sentry.captureException(err);

    return {
      status: 'failed',
      billingAmount,
      creditAmount,
      data: { message: err instanceof Error ? err.message : String(err) },
    };
  }
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

        await compensateReferrer(tx, invoice.userId);

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
