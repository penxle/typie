import * as Sentry from '@sentry/node';
import dayjs from 'dayjs';
import { and, eq, lt } from 'drizzle-orm';
import { PLAN_PAYMENT_GRACE_DAYS } from '@/const';
import { db, PaymentInvoices, UserPlans } from '@/db';
import { PaymentInvoiceState, UserPlanState } from '@/enums';
import { payInvoice } from '@/utils';
import { defineCron } from '../types';

const PAYMENT_FAILED_ERROR = Symbol('payment_failed');

export const PaymentCron = defineCron('payment', '0 10 * * *', async () => {
  const now = dayjs();

  const invoices = await db
    .select({
      id: PaymentInvoices.id,
      billingAt: PaymentInvoices.billingAt,
      userId: PaymentInvoices.userId,
    })
    .from(PaymentInvoices)
    .where(and(eq(PaymentInvoices.state, PaymentInvoiceState.UPCOMING), lt(PaymentInvoices.billingAt, now)));

  for (const invoice of invoices) {
    const payResult = await db
      .transaction(async (tx) => {
        const payInvoiceResult = await payInvoice({
          tx,
          invoiceId: invoice.id,
          makeRecordWhenFail: true,
        });

        if (payInvoiceResult.status === 'failed') {
          throw PAYMENT_FAILED_ERROR;
        }

        return true;
      })
      .catch((err) => {
        if (err !== PAYMENT_FAILED_ERROR) {
          Sentry.captureException(err);
        }

        return false;
      });

    // 이미 여기 들어온 시점에서 billingAt < now 니까 부호 신경쓸 필요 X
    if (!payResult && invoice.billingAt.diff(now, 'day') > PLAN_PAYMENT_GRACE_DAYS) {
      await db.transaction(async (tx) => {
        await tx
          .update(PaymentInvoices)
          .set({
            state: PaymentInvoiceState.CANCELED,
          })
          .where(eq(PaymentInvoices.id, invoice.id));

        await tx.delete(UserPlans).where(eq(UserPlans.userId, invoice.userId));
      });
    }
  }

  await db.delete(UserPlans).where(and(eq(UserPlans.state, UserPlanState.CANCELED), lt(UserPlans.expiresAt, now)));
});
