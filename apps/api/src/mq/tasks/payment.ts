import dayjs from 'dayjs';
import { and, eq, lt } from 'drizzle-orm';
import { db, PaymentInvoices, PaymentRecords, UserPlans, Users } from '@/db';
import { PaymentInvoiceState } from '@/enums';
import { makePeriodPayment } from '@/utils';
import { defineCron } from '../types';

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
    const result = await db
      .transaction(async (tx) => {
        return await makePeriodPayment({
          tx,
          invoiceId: invoice.id,
        });
      })
      .catch((err) => console.error(err));

    if (result?.makePaymentResult) {
      await db
        .update(PaymentRecords)
        .set({
          receiptUrl: result.makePaymentResult.receiptUrl,
        })
        .where(eq(PaymentRecords.id, result.paymentRecordId));
    }
  }
});
