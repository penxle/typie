import { and, eq } from 'drizzle-orm';
import { db, first, firstOrThrow, PaymentMethods, TableCode } from '@/db';
import { PaymentMethodState } from '@/enums';
import { TypieError } from '@/errors';
import * as portone from '@/external/portone';
import { cardSchema } from '@/validation';
import { builder } from '../builder';
import { isTypeOf, PaymentMethod, User } from '../objects';

/**
 * * Types
 */

PaymentMethod.implement({
  isTypeOf: isTypeOf(TableCode.PAYMENT_METHODS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

builder.objectField(User, 'paymentMethod', (t) =>
  t.field({
    type: PaymentMethod,
    nullable: true,
    resolve: async (user) => {
      return await db
        .select()
        .from(PaymentMethods)
        .where(and(eq(PaymentMethods.userId, user.id), eq(PaymentMethods.state, PaymentMethodState.ACTIVE)))
        .then(first);
    },
  }),
);

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  updatePaymentMethod: t.withAuth({ session: true }).fieldWithInput({
    type: PaymentMethod,
    input: {
      cardNumber: t.input.string({ validate: { schema: cardSchema.number } }),
      expiry: t.input.string({ validate: { schema: cardSchema.expiry } }),
      birthOrBusinessRegistrationNumber: t.input.string({
        validate: { schema: cardSchema.birthOrBusinessRegistrationNumber },
      }),
      passwordTwoDigits: t.input.string({ validate: { schema: cardSchema.passwordTwoDigits } }),
    },
    resolve: async (_, { input }, ctx) => {
      const [, expiryMonth, expiryYear] = input.expiry.match(/^(\d{2})(\d{2})$/) || [];

      const result = await portone.issueBillingKey({
        customerId: ctx.session.userId,
        cardNumber: input.cardNumber,
        expiryYear,
        expiryMonth,
        birthOrBusinessRegistrationNumber: input.birthOrBusinessRegistrationNumber,
        passwordTwoDigits: input.passwordTwoDigits,
      });

      if (result.status === 'failed') {
        throw new TypieError({ code: 'billing_key_issue_failed' });
      }

      return await db.transaction(async (tx) => {
        const methods = await tx
          .update(PaymentMethods)
          .set({ state: PaymentMethodState.DEACTIVATED })
          .where(and(eq(PaymentMethods.userId, ctx.session.userId), eq(PaymentMethods.state, PaymentMethodState.ACTIVE)))
          .returning({ billingKey: PaymentMethods.billingKey });

        for (const method of methods) {
          await portone.deleteBillingKey({ billingKey: method.billingKey });
        }

        return await tx
          .insert(PaymentMethods)
          .values({
            userId: ctx.session.userId,
            name: `${result.card.name} ${input.cardNumber.slice(-4)}`,
            billingKey: result.billingKey,
          })
          .returning()
          .then(firstOrThrow);
      });
    },
  }),
}));
