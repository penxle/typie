import { logger } from '@typie/lib';
import { PaymentInvoiceState, PaymentOutcome, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, desc, eq, inArray, isNull, sql } from 'drizzle-orm';
import {
  db,
  first,
  firstOrThrow,
  PaymentInvoices,
  PaymentRecords,
  Referrals,
  Subscriptions,
  UserBillingKeys,
  UserPaymentCredits,
  Users,
} from '#/db/index.ts';
import * as portone from '#/external/portone.ts';
import { opsAlert } from './ops-alert.ts';
import { classifyAlreadyPaidRecovery, splitBillingAmount } from './payment-core.ts';
import type { Transaction } from '#/db/index.ts';

const log = logger.getChild('payment');

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
      .set({ amount: existingCredit.amount + 2900 })
      .where(eq(UserPaymentCredits.id, existingCredit.id));
  } else {
    await tx.insert(UserPaymentCredits).values({
      userId: referral.referrerId,
      amount: 2900,
    });
  }

  await tx.update(Referrals).set({ referrerCompensatedAt: dayjs() }).where(eq(Referrals.id, referral.id));
};

export type PaymentAttemptOutcome = { kind: 'paid' } | { kind: 'not-paid' };

type FinalizePaymentSuccessParams = {
  invoiceId: string;
  evidence: { billingAmount: number; creditAmount: number; data: Record<string, unknown> };
};

export const finalizePaymentSuccess = async (tx: Transaction, { invoiceId, evidence }: FinalizePaymentSuccessParams) => {
  // PAID CAS가 소유권 가드다 — 0행이면 다른 경로가 이미 종결했거나 인보이스가 취소된 것이라 부수효과 전체를 건너뛴다.
  const casResult = await tx
    .update(PaymentInvoices)
    .set({ state: PaymentInvoiceState.PAID })
    .where(
      and(eq(PaymentInvoices.id, invoiceId), inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE])),
    )
    .returning({
      subscriptionId: PaymentInvoices.subscriptionId,
      userId: PaymentInvoices.userId,
      servicePeriodStartsAt: PaymentInvoices.servicePeriodStartsAt,
      servicePeriodEndsAt: PaymentInvoices.servicePeriodEndsAt,
    })
    .then(first);

  if (!casResult) {
    return false;
  }

  await tx.insert(PaymentRecords).values({
    invoiceId,
    outcome: PaymentOutcome.SUCCESS,
    billingAmount: evidence.billingAmount,
    creditAmount: evidence.creditAmount,
    data: evidence.data,
  });

  if (evidence.creditAmount > 0) {
    const credit = await tx
      .select({ id: UserPaymentCredits.id, amount: UserPaymentCredits.amount })
      .from(UserPaymentCredits)
      .where(eq(UserPaymentCredits.userId, casResult.userId))
      .for('no key update')
      .then(first);

    const deductible = Math.min(credit?.amount ?? 0, evidence.creditAmount);

    if (credit && deductible > 0) {
      await tx
        .update(UserPaymentCredits)
        .set({ amount: credit.amount - deductible })
        .where(eq(UserPaymentCredits.id, credit.id));
    }

    if (deductible < evidence.creditAmount) {
      // 늦은 성공 확정 사이 크레딧이 소비됐으면 부족분은 차감을 포기한다(원장 미도입 — 알람만).
      await opsAlert('payment-credit-shortfall', { invoiceId, expected: evidence.creditAmount, deducted: deductible });
    }
  }

  await compensateReferrer(tx, casResult.userId);

  // 주기는 산술 전진이 아니라 인보이스의 서비스 주기로 멱등 설정한다 — 재실행·전환(선설정 주기)에 안전.
  await tx
    .update(Subscriptions)
    .set({
      state: SubscriptionState.ACTIVE,
      currentPeriodStartsAt: casResult.servicePeriodStartsAt,
      currentPeriodEndsAt: casResult.servicePeriodEndsAt,
    })
    .where(eq(Subscriptions.id, casResult.subscriptionId));

  return true;
};

const recoverAlreadyPaid = async (tx: Transaction, { invoiceId }: { invoiceId: string }): Promise<PaymentAttemptOutcome> => {
  const invoice = await tx
    .select({ paymentKey: PaymentInvoices.paymentKey })
    .from(PaymentInvoices)
    .where(eq(PaymentInvoices.id, invoiceId))
    .then(firstOrThrow);

  const lookup = await portone.lookupPayment({ paymentId: invoice.paymentKey });

  // 증거는 승인 당시 시도의 것이어야 한다 — AlreadyPaid 시도 자체는 PaymentRecords에 기록하지 않는다.
  const attemptRecords = await tx
    .select({ billingAmount: PaymentRecords.billingAmount, creditAmount: PaymentRecords.creditAmount })
    .from(PaymentRecords)
    .where(eq(PaymentRecords.invoiceId, invoiceId))
    .orderBy(desc(PaymentRecords.createdAt));

  const classification = classifyAlreadyPaidRecovery({ lookup, attemptRecords });

  switch (classification.kind) {
    case 'finalize': {
      const finalized = await finalizePaymentSuccess(tx, {
        invoiceId,
        evidence: { ...classification.evidence, data: { recoveredFromAlreadyPaid: true } },
      });

      return finalized ? { kind: 'paid' } : { kind: 'not-paid' };
    }
    case 'alert': {
      await opsAlert(classification.id, { invoiceId });

      return { kind: 'not-paid' };
    }
    case 'defer': {
      // 확인 조회 비확정 — 알람 없이 일반 규칙(다음 재시도 → 유예 마감 종결)로 수렴한다.
      return { kind: 'not-paid' };
    }
  }
};

// 호출 전제: 유저 advisory 락 보유, 인보이스 UPCOMING|OVERDUE.
export const attemptInvoicePayment = async (tx: Transaction, invoiceId: string): Promise<PaymentAttemptOutcome> => {
  const invoice = await tx
    .select({
      id: PaymentInvoices.id,
      userId: PaymentInvoices.userId,
      amount: PaymentInvoices.amount,
      paymentKey: PaymentInvoices.paymentKey,
    })
    .from(PaymentInvoices)
    .where(eq(PaymentInvoices.id, invoiceId))
    .for('no key update')
    .then(firstOrThrow);

  // 성패·기록 유무와 무관한 처리 스탬프 — 재시도 크론의 페이싱 신호다. 아래 PG 미호출·비확정 경로들은
  // PaymentRecords 를 남기지 않으므로(승인 증거 오염 방지) 기록 존재 검사로는 페이스를 잴 수 없고,
  // 그 상태로 분 단위 스캔에 걸리면 같은 인보이스가 하루 종일 재처리된다.
  await tx.update(PaymentInvoices).set({ lastAttemptedAt: dayjs() }).where(eq(PaymentInvoices.id, invoiceId));

  const paymentCredit = await tx
    .select({ amount: UserPaymentCredits.amount })
    .from(UserPaymentCredits)
    .where(eq(UserPaymentCredits.userId, invoice.userId))
    .for('no key update')
    .then(first);

  const { billingAmount, creditAmount } = splitBillingAmount({ invoiceAmount: invoice.amount, creditBalance: paymentCredit?.amount ?? 0 });

  if (billingAmount === 0) {
    const finalized = await finalizePaymentSuccess(tx, { invoiceId, evidence: { billingAmount: 0, creditAmount, data: {} } });

    return finalized ? { kind: 'paid' } : { kind: 'not-paid' };
  }

  // 빌링키 결손·조회 예외도 비성공이다 — 선행 실패가 유예 분기를 우회하면 안 된다.
  const billingKey = await tx
    .select({ billingKey: UserBillingKeys.billingKey })
    .from(UserBillingKeys)
    .where(eq(UserBillingKeys.userId, invoice.userId))
    .for('no key update')
    .then(first)
    .catch(() => null);

  if (!billingKey) {
    // PG를 호출하지 않은 실패는 PaymentRecords에 남기지 않는다 — 남기면 그 금액이 우연히 PG 조회 금액과
    // 일치해 AlreadyPaid 회수의 가짜 승인 증거가 된다(증거는 승인 당시 시도의 것이어야 한다).
    return { kind: 'not-paid' };
  }

  const user = await tx.select({ name: Users.name, email: Users.email }).from(Users).where(eq(Users.id, invoice.userId)).then(firstOrThrow);

  const result = await portone.payWithBillingKey({
    paymentId: invoice.paymentKey,
    billingKey: billingKey.billingKey,
    customerName: user.name,
    customerEmail: user.email,
    orderName: '타이피 정기결제',
    amount: billingAmount,
  });

  if (result.status === 'succeeded') {
    const finalized = await finalizePaymentSuccess(tx, {
      invoiceId,
      evidence: { billingAmount, creditAmount, data: { pgTxId: result.pgTxId, paidAt: result.paidAt } },
    });

    return finalized ? { kind: 'paid' } : { kind: 'not-paid' };
  }

  if (result.portoneErrorType === 'ALREADY_PAID') {
    return await recoverAlreadyPaid(tx, { invoiceId });
  }

  await tx.insert(PaymentRecords).values({
    invoiceId,
    outcome: PaymentOutcome.FAILURE,
    billingAmount,
    creditAmount,
    data: { code: result.code, message: result.message },
  });

  return { kind: 'not-paid' };
};

// 트랜잭션 커밋 후 호출한다 — 영수증 조회 실패가 성공 확정을 뒤집지 않고, 승인~커밋 창을 늘리지도 않는다.
export const enrichPaymentRecordReceipt = async (invoiceId: string) => {
  try {
    const invoice = await db
      .select({ paymentKey: PaymentInvoices.paymentKey })
      .from(PaymentInvoices)
      .where(eq(PaymentInvoices.id, invoiceId))
      .then(first);

    if (!invoice) {
      return;
    }

    const receipt = await portone.getPaymentReceipt({ paymentId: invoice.paymentKey });

    if (!receipt) {
      return;
    }

    const patch = JSON.stringify({ approvalNumber: receipt.approvalNumber, receiptUrl: receipt.receiptUrl });

    await db
      .update(PaymentRecords)
      .set({ data: sql`${PaymentRecords.data} || ${patch}::jsonb` })
      .where(and(eq(PaymentRecords.invoiceId, invoiceId), eq(PaymentRecords.outcome, PaymentOutcome.SUCCESS)));
  } catch (err) {
    log.warn('failed to enrich payment receipt {*}', { invoiceId, error: err instanceof Error ? err.message : String(err) });
  }
};
