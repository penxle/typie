import dayjs, { Dayjs } from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { db, first, firstOrThrow, PaymentInvoices, PaymentMethods, PaymentRecords, Plans, TableCode, UserPlans, Users } from '@/db';
import { PaymentInvoiceState, PaymentMethodState, PaymentRecordState, PlanAvailability, UserPlanBillingCycle } from '@/enums';
import { TypieError } from '@/errors';
import * as portone from '@/external/portone';
import { cardSchema } from '@/validation';
import { builder } from '../builder';
import { isTypeOf, PaymentMethod, Plan, User, UserPlan } from '../objects';

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
    fee: t.exposeInt('fee'),
  }),
});

UserPlan.implement({
  isTypeOf: isTypeOf(TableCode.USER_PLANS),
  fields: (t) => ({
    id: t.exposeID('id'),
    plan: t.field({ type: Plan, resolve: (userPlan) => userPlan.planId }),
    fee: t.exposeInt('fee'),
    billingCycle: t.expose('billingCycle', { type: UserPlanBillingCycle }),
    nextBillingAt: t.expose('nextBillingAt', { type: 'DateTime' }),
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

  enrollPlan: t.withAuth({ session: true }).fieldWithInput({
    type: UserPlan,
    input: {
      planId: t.input.id(),
      billingCycle: t.input.field({ type: UserPlanBillingCycle }),
    },
    resolve: async (_, { input }, ctx) => {
      const currentPlan = await db
        .select({
          id: UserPlans.id,
        })
        .from(UserPlans)
        .where(eq(UserPlans.userId, ctx.session.userId))
        .then(first);

      if (currentPlan) {
        // TODO: 플랜 변경 & 정산 처리 추후 구현 (아예 다른 플랜? 결제주기만 변경?)
        throw new TypieError({ code: 'plan_already_enrolled' });
      }

      const [paymentMethod, plan, me] = await Promise.all([
        db
          .select({
            id: PaymentMethods.id,
            billingKey: PaymentMethods.billingKey,
          })
          .from(PaymentMethods)
          .where(and(eq(PaymentMethods.userId, ctx.session.userId), eq(PaymentMethods.state, PaymentMethodState.ACTIVE)))
          .then(firstOrThrow),
        db
          .select({
            id: Plans.id,
            name: Plans.name,
            fee: Plans.fee,
          })
          .from(Plans)
          .where(and(eq(Plans.id, input.planId), eq(Plans.availability, PlanAvailability.PUBLIC)))
          .then(firstOrThrow),
        db.select({ name: Users.name, email: Users.email }).from(Users).where(eq(Users.id, ctx.session.userId)).then(firstOrThrow),
      ]);

      const amount = calcuratePaymentAmount({ fee: plan.fee, billingCycle: input.billingCycle });
      const today = dayjs().kst();
      const billingDate = today.date();
      const nextBillingAt = getNextBillingDate({ today, billingCycle: input.billingCycle, billingDate });

      return await db.transaction(async (tx) => {
        const invoice = await tx
          .insert(PaymentInvoices)
          .values({
            userId: ctx.session.userId,
            amount,
            state: PaymentInvoiceState.PAID,
          })
          .returning({ id: PaymentInvoices.id })
          .then(firstOrThrow);

        const userPlan = await tx
          .insert(UserPlans)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            fee: plan.fee,
            billingCycle: input.billingCycle,
            nextBillingAt,
            billingDate,
          })
          .returning()
          .then(firstOrThrow);

        const paymentResult = await portone.makePayment({
          paymentId: invoice.id,
          billingKey: paymentMethod.billingKey,
          customerName: me.name,
          customerEmail: me.email,
          orderName: '타이피 정기결제',
          amount,
        });

        if (paymentResult.status === 'failed') {
          throw new TypieError({ code: 'payment_failed', message: paymentResult.message });
        }

        await tx.insert(PaymentRecords).values({
          invoiceId: invoice.id,
          methodId: paymentMethod.id,
          state: PaymentRecordState.SUCCEEDED,
          amount,
          receiptUrl: paymentResult.receiptUrl,
        });

        return userPlan;
      });
    },
  }),
}));

/**
 * * Utils
 */

const billingCycleToUnit = (billingCycle: UserPlanBillingCycle) =>
  match(billingCycle)
    .with(UserPlanBillingCycle.MONTHLY, () => 'month' as const)
    .with(UserPlanBillingCycle.YEARLY, () => 'year' as const)
    .exhaustive();

type CalcuratePaymentAmountParams = {
  fee: number;
  billingCycle: UserPlanBillingCycle;
};

const calcuratePaymentAmount = ({ fee, billingCycle }: CalcuratePaymentAmountParams) => {
  return match(billingCycle)
    .with(UserPlanBillingCycle.MONTHLY, () => fee)
    .with(UserPlanBillingCycle.YEARLY, () => fee * 12)
    .exhaustive();
};

type GetNextBillingDateParams = {
  today: Dayjs;
  billingCycle: UserPlanBillingCycle;
  billingDate: number;
};

const getNextBillingDate = ({ today, billingCycle, billingDate }: GetNextBillingDateParams) => {
  const lastDayOfNextBillingMonth = today.add(1, billingCycleToUnit(billingCycle)).endOf('month').startOf('day');

  if (billingDate < lastDayOfNextBillingMonth.date()) {
    return lastDayOfNextBillingMonth.date(billingDate);
  }
  return lastDayOfNextBillingMonth;
};
