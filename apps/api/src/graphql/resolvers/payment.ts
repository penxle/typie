import dayjs from 'dayjs';
import { and, eq, gt } from 'drizzle-orm';
import {
  CreditCodes,
  db,
  first,
  firstOrThrow,
  firstOrThrowWith,
  PaymentBillingKeys,
  PaymentInvoices,
  PaymentRecords,
  Plans,
  TableCode,
  UserPaymentCredits,
  UserPlans,
  Users,
  validateDbId,
} from '@/db';
import { defaultPlanRules } from '@/db/schemas/json';
import {
  CreditCodeState,
  PaymentBillingKeyState,
  PaymentInvoiceState,
  PaymentMethodType,
  PaymentRecordState,
  PlanAvailability,
  UserPlanBillingCycle,
  UserPlanState,
} from '@/enums';
import { NotFoundError, TypieError } from '@/errors';
import * as portone from '@/external/portone';
import { calculatePaymentAmount, getNextPaymentDate } from '@/utils';
import { delay } from '@/utils/promise';
import { cardSchema, redeemCodeSchema } from '@/validation';
import { builder } from '../builder';
import { CreditCode, isTypeOf, PaymentBillingKey, Plan, PlanRule, UserPlan } from '../objects';

/**
 * * Types
 */

PaymentBillingKey.implement({
  isTypeOf: isTypeOf(TableCode.PAYMENT_BILLING_KEYS),
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

    rules: t.expose('rules', { type: PlanRule }),
  }),
});

PlanRule.implement({
  fields: (t) => ({
    maxTotalCharacterCount: t.int({ resolve: (self) => self.maxTotalCharacterCount ?? defaultPlanRules.maxTotalCharacterCount }),
    maxTotalBlobSize: t.int({ resolve: (self) => self.maxTotalBlobSize ?? defaultPlanRules.maxTotalBlobSize }),
  }),
});

UserPlan.implement({
  isTypeOf: isTypeOf(TableCode.USER_PLANS),
  fields: (t) => ({
    id: t.exposeID('id'),
    fee: t.exposeInt('fee'),
    billingCycle: t.expose('billingCycle', { type: UserPlanBillingCycle }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    expiresAt: t.expose('expiresAt', { type: 'DateTime' }),
    state: t.expose('state', { type: UserPlanState }),

    plan: t.expose('planId', { type: Plan }),
  }),
});

CreditCode.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    code: t.exposeString('code'),
    amount: t.exposeInt('amount'),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  creditCode: t.withAuth({ session: true }).field({
    type: CreditCode,
    args: {
      code: t.input.string({ validate: { schema: redeemCodeSchema } }),
    },
    resolve: async (_, { code }) => {
      await delay(Math.random() * 1000);

      return await db
        .select()
        .from(CreditCodes)
        .where(and(eq(CreditCodes.code, code), eq(CreditCodes.state, CreditCodeState.AVAILABLE), gt(CreditCodes.expiresAt, dayjs())))
        .then(firstOrThrowWith(new NotFoundError()));
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  updatePaymentBillingKey: t.withAuth({ session: true }).fieldWithInput({
    type: PaymentBillingKey,
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
        const billingKeys = await tx
          .update(PaymentBillingKeys)
          .set({ state: PaymentBillingKeyState.DEACTIVATED })
          .where(and(eq(PaymentBillingKeys.userId, ctx.session.userId), eq(PaymentBillingKeys.state, PaymentBillingKeyState.ACTIVE)))
          .returning({ billingKey: PaymentBillingKeys.billingKey });

        for (const billingKey of billingKeys) {
          await portone.deleteBillingKey({ billingKey: billingKey.billingKey });
        }

        return await tx
          .insert(PaymentBillingKeys)
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

      const paymentBillingKey = await db
        .select({ id: PaymentBillingKeys.id, billingKey: PaymentBillingKeys.billingKey })
        .from(PaymentBillingKeys)
        .where(and(eq(PaymentBillingKeys.userId, ctx.session.userId), eq(PaymentBillingKeys.state, PaymentBillingKeyState.ACTIVE)))
        .then(firstOrThrow);

      const enrolledAt = dayjs.kst().startOf('day');
      const nextPaymentDate = getNextPaymentDate(input.billingCycle, enrolledAt);
      const paymentAmount = calculatePaymentAmount(input.billingCycle, plan.fee);

      return await db.transaction(async (tx) => {
        const userPlan = await tx
          .insert(UserPlans)
          .values({
            userId: ctx.session.userId,
            planId: plan.id,
            fee: plan.fee,
            billingCycle: input.billingCycle,
            expiresAt: nextPaymentDate,
          })
          .returning()
          .then(firstOrThrow);

        const invoice = await tx
          .insert(PaymentInvoices)
          .values({
            userId: ctx.session.userId,
            amount: paymentAmount,
            billingAt: enrolledAt,
            state: PaymentInvoiceState.PAID,
          })
          .returning({ id: PaymentInvoices.id })
          .then(firstOrThrow);

        await tx.insert(PaymentInvoices).values({
          userId: ctx.session.userId,
          amount: paymentAmount,
          billingAt: nextPaymentDate,
          state: PaymentInvoiceState.UPCOMING,
        });

        const paymentCredit = await tx
          .select({ id: UserPaymentCredits.id, amount: UserPaymentCredits.amount })
          .from(UserPaymentCredits)
          .where(eq(UserPaymentCredits.userId, ctx.session.userId))
          .for('update')
          .then(first);

        const creditPaymentAmount = Math.min(paymentCredit?.amount ?? 0, paymentAmount);

        if (paymentCredit && creditPaymentAmount > 0) {
          await tx.insert(PaymentRecords).values({
            invoiceId: invoice.id,
            methodType: PaymentMethodType.CREDIT,
            methodId: paymentCredit.id,
            state: PaymentRecordState.SUCCEEDED,
            amount: creditPaymentAmount,
          });

          await tx
            .update(UserPaymentCredits)
            .set({ amount: paymentCredit.amount - creditPaymentAmount })
            .where(eq(UserPaymentCredits.id, paymentCredit.id));
        }

        const billingKeyPaymentAmount = paymentAmount - creditPaymentAmount;

        if (billingKeyPaymentAmount > 0) {
          const paymentResult = await portone.makePayment({
            paymentId: invoice.id,
            billingKey: paymentBillingKey.billingKey,
            customerName: user.name,
            customerEmail: user.email,
            orderName: '타이피 정기결제',
            amount: billingKeyPaymentAmount,
          });

          if (paymentResult.status === 'failed') {
            throw new TypieError({ code: 'payment_failed', message: paymentResult.message });
          }

          await tx.insert(PaymentRecords).values({
            invoiceId: invoice.id,
            methodType: PaymentMethodType.BILLING_KEY,
            methodId: paymentBillingKey.id,
            state: PaymentRecordState.SUCCEEDED,
            amount: billingKeyPaymentAmount,
            receiptUrl: paymentResult.receiptUrl,
          });
        }

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
