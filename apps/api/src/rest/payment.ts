import { eq } from 'drizzle-orm';
import { Hono } from 'hono';
import qs from 'query-string';
import { db, firstOrThrow, PreorderPayments, PreorderUsers } from '@/db';
import { sendEmail } from '@/email';
import { PreorderCompletedEmail } from '@/email/templates';
import { env } from '@/env';
import * as portone from '@/external/portone';
import type { Env } from '@/context';

export const payment = new Hono<Env>();

payment.get('/redirect', async (c) => {
  const paymentId = c.req.query('paymentId');

  if (!paymentId) {
    return c.redirect(env.WEBSITE_URL);
  }

  const paymentRequest = await db
    .select({ id: PreorderPayments.id, amount: PreorderPayments.amount })
    .from(PreorderPayments)
    .where(eq(PreorderPayments.id, paymentId))
    .limit(1)
    .then(firstOrThrow);

  const paymentResult = await portone.getPayment({ paymentId: paymentRequest.id });

  if (paymentResult.status !== 'succeeded') {
    return c.redirect(
      qs.stringifyUrl({
        url: env.WEBSITE_URL,
        query: {
          message: paymentResult.message,
        },
      }),
    );
  }

  if (paymentResult.amount !== paymentRequest.amount) {
    return c.redirect(
      qs.stringifyUrl({
        url: env.WEBSITE_URL,
        query: {
          message: '결제 금액이 일치하지 않아요',
        },
      }),
    );
  }

  const customData = JSON.parse(paymentResult.customData ?? '{}');

  const preorderUser = await db.transaction(async (tx) => {
    await tx
      .update(PreorderPayments)
      .set({
        state: 'COMPLETED',
      })
      .where(eq(PreorderPayments.id, paymentRequest.id));

    return await tx
      .insert(PreorderUsers)
      .values({
        email: customData.email.toLowerCase(),
        wish: customData.wish,
        preorderPaymentId: paymentRequest.id,
      })
      .returning()
      .then(firstOrThrow);
  });

  await sendEmail({
    recipient: preorderUser.email,
    subject: '[타이피] 사전 등록이 완료되었어요',
    body: PreorderCompletedEmail(),
  });

  return c.redirect(
    qs.stringifyUrl({
      url: env.WEBSITE_URL,
      query: {
        success: '1',
      },
    }),
  );
});
