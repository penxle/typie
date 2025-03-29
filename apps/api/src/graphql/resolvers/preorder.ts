import { eq } from 'drizzle-orm';
import { db, firstOrThrow, PreorderPayments, PreorderUsers } from '@/db';
import { TypieError } from '@/errors';
import * as portone from '@/external/portone';
import { builder } from '../builder';
import { PreorderPayment, PreorderUser } from '../objects';

const PAYMENT_AMOUNT = 4900;

PreorderPayment.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
  }),
});

PreorderUser.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    email: t.exposeString('email'),
  }),
});

builder.mutationFields((t) => ({
  createPreorderPayment: t.fieldWithInput({
    type: PreorderPayment,
    input: {
      email: t.input.string({ required: true, validate: { email: true } }),
    },
    resolve: async (_, { input }) => {
      const alreadyOrderedUser = await db
        .select({ id: PreorderUsers.id })
        .from(PreorderUsers)
        .where(eq(PreorderUsers.email, input.email.toLowerCase()))
        .limit(1);
      if (alreadyOrderedUser.length > 0) {
        throw new TypieError({ code: 'ALREADY_ORDERED_EMAIL', message: '이미 예약 신청한 이메일입니다.' });
      }

      return await db
        .insert(PreorderPayments)
        .values({
          email: input.email.toLowerCase(),
          amount: PAYMENT_AMOUNT,
        })
        .returning()
        .then(firstOrThrow);
    },
  }),

  finalizePreorderPayment: t.fieldWithInput({
    type: PreorderUser,
    input: {
      paymentId: t.input.string({ required: true }),
      email: t.input.string({ required: true, validate: { email: true } }),
      wish: t.input.string(),
    },
    resolve: async (_, { input }) => {
      const paymentRequest = await db
        .select({ id: PreorderPayments.id, amount: PreorderPayments.amount })
        .from(PreorderPayments)
        .where(eq(PreorderPayments.id, input.paymentId))
        .limit(1)
        .then(firstOrThrow);

      const paymentResult = await portone.getPayment({ paymentId: paymentRequest.id });

      if (paymentResult.status !== 'succeeded') {
        throw new TypieError({ code: 'PAYMENT_FAILED', message: paymentResult.message });
      }

      if (paymentResult.amount.total !== paymentRequest.amount) {
        throw new TypieError({ code: 'PAYMENT_AMOUNT_MISMATCH', message: '결제 금액이 일치하지 않아요' });
      }

      return await db.transaction(async (tx) => {
        await tx
          .update(PreorderPayments)
          .set({
            state: 'COMPLETED',
          })
          .where(eq(PreorderPayments.id, paymentRequest.id));

        return await tx
          .insert(PreorderUsers)
          .values({
            email: input.email.toLowerCase(),
            wish: input.wish,
            preorderPaymentId: paymentRequest.id,
          })
          .returning()
          .then(firstOrThrow);
      });
    },
  }),
}));
