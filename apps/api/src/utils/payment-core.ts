import type { LookupPaymentResult } from '#/external/portone.ts';

export type AlreadyPaidClassification =
  | { kind: 'finalize'; evidence: { billingAmount: number; creditAmount: number } }
  | { kind: 'defer' }
  | { kind: 'alert'; id: 'already-paid-without-evidence' | 'already-paid-amount-mismatch' };

export type ClassifyAlreadyPaidRecoveryParams = {
  lookup: LookupPaymentResult;
  attemptRecords: { billingAmount: number; creditAmount: number }[];
};

// attemptRecords는 최신순이다. 검증 기준은 인보이스 총액이 아니라 그 시도의 PG 청구액 — 재시도 사이 크레딧 잔액이
// 변하면 시도마다 청구액이 다르므로, 조회된 승인 금액과 일치하는 시도가 하나라도 있어야 승인 증거로 인정한다.
export const classifyAlreadyPaidRecovery = ({ lookup, attemptRecords }: ClassifyAlreadyPaidRecoveryParams): AlreadyPaidClassification => {
  if (lookup.kind !== 'paid') {
    return { kind: 'defer' };
  }

  if (attemptRecords.length === 0) {
    return { kind: 'alert', id: 'already-paid-without-evidence' };
  }

  const matched = attemptRecords.find((record) => record.billingAmount === lookup.amount);

  if (!matched) {
    return { kind: 'alert', id: 'already-paid-amount-mismatch' };
  }

  return { kind: 'finalize', evidence: { billingAmount: matched.billingAmount, creditAmount: matched.creditAmount } };
};

export type SplitBillingAmountParams = { invoiceAmount: number; creditBalance: number };

export const splitBillingAmount = ({ invoiceAmount, creditBalance }: SplitBillingAmountParams) => {
  const creditAmount = Math.min(Math.max(creditBalance, 0), invoiceAmount);

  return { billingAmount: invoiceAmount - creditAmount, creditAmount };
};
