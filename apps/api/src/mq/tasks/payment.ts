import * as Sentry from '@sentry/bun';
import dayjs from 'dayjs';
import { and, eq, lt } from 'drizzle-orm';
import { db, PaymentInvoices, UserPlans, Users } from '@/db';
import { PaymentInvoiceState } from '@/enums';
import { payInvoice } from '@/utils';
import { defineCron } from '../types';

const PAYMENT_FAILED_ERROR = Symbol('payment_failed');

export const PaymentCron = defineCron('payment', '0 10 * * *', async () => {
  const invoices = await db
    .select({
      id: PaymentInvoices.id,
    })
    .from(PaymentInvoices)
    .innerJoin(Users, eq(PaymentInvoices.userId, Users.id))
    .innerJoin(UserPlans, eq(PaymentInvoices.userId, UserPlans.userId))
    .where(and(eq(PaymentInvoices.state, PaymentInvoiceState.UPCOMING), lt(PaymentInvoices.billingAt, dayjs())));

  for (const invoice of invoices) {
    await db
      .transaction(async (tx) => {
        const payInvoiceResult = await payInvoice({
          tx,
          invoiceId: invoice.id,
          makeRecordWhenFail: true,
        });

        if (payInvoiceResult.status === 'failed') {
          throw PAYMENT_FAILED_ERROR;
        }
      })
      .catch((err) => {
        if (err !== PAYMENT_FAILED_ERROR) {
          Sentry.captureException(err);
        }
      });
  }
});
