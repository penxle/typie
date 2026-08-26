import type { PrismCreditEntryKind } from '@typie/lib/enums';

export const MILLI_PER_CREDIT = 1000;

export type CreditDelta = { paidDelta: number; freeDelta: number };

const assertInteger = (value: number, label: string) => {
  if (!Number.isSafeInteger(value)) {
    throw new TypeError(`prism credit ${label} must be an integer: ${value}`);
  }
};

export const splitCharge = ({ free, amount }: { free: number; amount: number }): CreditDelta => {
  assertInteger(free, 'free');
  assertInteger(amount, 'amount');
  if (amount < 0) {
    throw new Error(`prism credit charge amount must be non-negative: ${amount}`);
  }

  const freeUse = Math.min(Math.max(free, 0), amount);

  return { paidDelta: freeUse - amount, freeDelta: 0 - freeUse };
};

export const invertCharge = (delta: CreditDelta): CreditDelta => ({ paidDelta: 0 - delta.paidDelta, freeDelta: 0 - delta.freeDelta });

export const validateEntry = (kind: PrismCreditEntryKind, { paidDelta, freeDelta }: CreditDelta) => {
  assertInteger(paidDelta, 'paidDelta');
  assertInteger(freeDelta, 'freeDelta');

  const ok = (() => {
    switch (kind) {
      case 'GRANT':
      case 'TRIAL': {
        return freeDelta > 0 && paidDelta === 0;
      }
      case 'REVIEW_CHARGE':
      case 'CHAT_CHARGE': {
        return paidDelta <= 0 && freeDelta <= 0;
      }
      case 'REVIEW_REFUND': {
        return paidDelta >= 0 && freeDelta >= 0;
      }
      case 'PURCHASE': {
        return paidDelta > 0 && freeDelta === 0;
      }
      case 'BONUS': {
        return freeDelta > 0 && paidDelta === 0;
      }
      case 'REFUND_OUT': {
        return paidDelta <= 0 && freeDelta <= 0 && (paidDelta !== 0 || freeDelta !== 0);
      }
      case 'ADJUSTMENT': {
        return true;
      }
      default: {
        kind satisfies never;
        return false;
      }
    }
  })();

  if (!ok) {
    throw new Error(`prism credit entry violates ${kind} sign rule: paid=${paidDelta} free=${freeDelta}`);
  }
};

export const toDisplayCredits = (milli: number): number => {
  if (milli >= 0) {
    return Math.ceil(milli / MILLI_PER_CREDIT);
  }

  return -Math.ceil(-milli / MILLI_PER_CREDIT);
};

export const toMilli = (credits: number): number => {
  assertInteger(credits, 'credits');

  return credits * MILLI_PER_CREDIT;
};
