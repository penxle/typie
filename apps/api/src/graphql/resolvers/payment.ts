import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import {
  db,
  first,
  firstOrThrow,
  PaymentInvoices,
  PaymentMethods,
  PaymentRecords,
  Plans,
  TableCode,
  UserPlans,
  Users,
  validateDbId,
} from '@/db';
import {
  PaymentInvoiceState,
  PaymentMethodState,
  PaymentRecordState,
  PlanAvailability,
  UserPlanBillingCycle,
  UserPlanState,
} from '@/enums';
import { TypieError } from '@/errors';
import * as portone from '@/external/portone';
import { calculatePaymentAmount, getNextPaymentDate } from '@/utils';
import { cardSchema } from '@/validation';
import { builder } from '../builder';
import { isTypeOf, PaymentMethod, Plan, UserPlan } from '../objects';

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

Plan.implement({
  isTypeOf: isTypeOf(TableCode.PLANS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    fee: t.exposeInt('fee'),
  }),
});

UserPlan.implement({
  isTypeOf: isTypeOf(TableCode.USER_PLANS),
  fields: (t) => ({
    id: t.exposeID('id'),
    fee: t.exposeInt('fee'),
    billingCycle: t.expose('billingCycle', { type: UserPlanBillingCycle }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),

    plan: t.expose('planId', { type: Plan }),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  updatePaymentMethod: t.withAuth({ session: true }).fieldWithInput({
    type: PaymentMethod,
    input: {
      cardNumber: t.input.string({ validate: { schema: cardSchema.cardNumber } }),
      expiryDate: t.input.string({ validate: { schema: cardSchema.expiryDate } }),
      birthOrBusinessRegistrationNumber: t.input.string({
        validate: { schema: cardSchema.birthOrBusinessRegistrationNumber },
      }),
      passwordTwoDigits: t.input.string({ validate: { schema: cardSchema.passwordTwoDigits } }),
    },
    resolve: async (_, { input }, ctx) => {
      const [, expiryMonth, expiryYear] = input.expiryDate.match(/^(\d{2})(\d{2})$/) || [];

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
            name: `${result.cardName} ${input.cardNumber.slice(-4)}`,
            billingKey: result.billingKey,
          })
          .returning()
          .then(firstOrThrow);
      });
    },
  }),

  enrollPlan: t.withAuth({ session: true }).fieldWithInput({
    type: UserPlan,
    input: {
      planId: t.input.id({ validate: validateDbId(TableCode.PLANS) }),
      billingCycle: t.input.field({ type: UserPlanBillingCycle }),
    },
    resolve: async (_, { input }, ctx) => {
      const userPlan = await db
        .select({
          id: UserPlans.id,
        })
        .from(UserPlans)
        .where(eq(UserPlans.userId, ctx.session.userId))
        .then(first);

      if (userPlan) {
        // TODO: 플랜 변경 & 정산 처리 추후 구현 (아예 다른 플랜? 결제주기만 변경?)
        throw new TypieError({ code: 'plan_already_enrolled' });
      }

      const user = await db
        .select({ name: Users.name, email: Users.email })
        .from(Users)
        .where(eq(Users.id, ctx.session.userId))
        .then(firstOrThrow);

      const plan = await db
        .select({ id: Plans.id, name: Plans.name, fee: Plans.fee })
        .from(Plans)
        .where(and(eq(Plans.id, input.planId), eq(Plans.availability, PlanAvailability.PUBLIC)))
        .then(firstOrThrow);

      const paymentMethod = await db
        .select({ id: PaymentMethods.id, billingKey: PaymentMethods.billingKey })
        .from(PaymentMethods)
        .where(and(eq(PaymentMethods.userId, ctx.session.userId), eq(PaymentMethods.state, PaymentMethodState.ACTIVE)))
        .then(firstOrThrow);

      const enrolledAt = dayjs.kst().startOf('day');
      const nextPaymentDate = getNextPaymentDate(input.billingCycle, enrolledAt);

      return await db.transaction(async (tx) => {
        const userPlan = await tx
          .insert(UserPlans)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            fee: calculatePaymentAmount(input.billingCycle, plan.fee),
            billingCycle: input.billingCycle,
            expiresAt: nextPaymentDate,
          })
          .returning()
          .then(firstOrThrow);

        const invoice = await tx
          .insert(PaymentInvoices)
          .values({
            userId: ctx.session.userId,
            amount: userPlan.fee,
            billingAt: enrolledAt,
            state: PaymentInvoiceState.PAID,
          })
          .returning({ id: PaymentInvoices.id })
          .then(firstOrThrow);

        await tx.insert(PaymentInvoices).values({
          userId: ctx.session.userId,
          amount: userPlan.fee,
          billingAt: nextPaymentDate,
          state: PaymentInvoiceState.UPCOMING,
        });

        const paymentResult = await portone.makePayment({
          paymentId: invoice.id,
          billingKey: paymentMethod.billingKey,
          customerName: user.name,
          customerEmail: user.email,
          orderName: '타이피 정기결제',
          amount: userPlan.fee,
        });

        if (paymentResult.status === 'failed') {
          throw new TypieError({ code: 'payment_failed', message: paymentResult.message });
        }

        await tx.insert(PaymentRecords).values({
          invoiceId: invoice.id,
          methodId: paymentMethod.id,
          state: PaymentRecordState.SUCCEEDED,
          amount: userPlan.fee,
          receiptUrl: paymentResult.receiptUrl,
        });

        return userPlan;
      });
    },
  }),

  cancelPlan: t.withAuth({ session: true }).field({
    type: UserPlan,
    resolve: async (_, __, ctx) => {
      await db
        .select()
        .from(UserPlans)
        .where(and(eq(UserPlans.userId, ctx.session.userId), eq(UserPlans.state, UserPlanState.ACTIVE)))
        .then(firstOrThrow);

      return await db.transaction(async (tx) => {
        await tx
          .update(PaymentInvoices)
          .set({ state: PaymentInvoiceState.CANCELED })
          .where(and(eq(PaymentInvoices.userId, ctx.session.userId), eq(PaymentInvoices.state, PaymentInvoiceState.UPCOMING)));

        return await tx
          .update(UserPlans)
          .set({ state: UserPlanState.CANCELED })
          .where(and(eq(UserPlans.userId, ctx.session.userId), eq(UserPlans.state, UserPlanState.ACTIVE)))
          .returning()
          .then(firstOrThrow);
      });
    },
  }),
}));
